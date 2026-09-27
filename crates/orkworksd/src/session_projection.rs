use crate::git;
use crate::metadata;
use crate::plan_handoff::resolve_openable_plan_reference;
use crate::session_types::SessionInfo;
use crate::session_view::{
    detect_conflicts, merge_live_session_info, resolve_effective_cwds,
    resolve_harness_or_generic_shell, session_recommendation,
};
use crate::AppState;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Shared by the live and remembered session-info paths (issue #399): both
/// resolve the same openable-plan reference from `meta.plan_path` against
/// the current workspace, and previously reimplemented this expression
/// independently.
fn resolve_has_openable_plan(
    plan_path: Option<&metadata::PlanReference>,
    workspace: Option<&WorkspaceSnapshot>,
) -> Option<bool> {
    plan_path.and_then(|reference| {
        workspace.map(|snapshot| {
            resolve_openable_plan_reference(&snapshot.workspace_path, reference).is_ok()
        })
    })
}

/// Stateful coordinator for the session-listing projection.
///
/// This borrows the existing application state; it does not own a second
/// session registry or metadata store.
pub(crate) struct SessionProjection {
    state: Arc<AppState>,
}

#[derive(Clone)]
struct WorkspaceSnapshot {
    metadata_root: PathBuf,
    workspace_path: PathBuf,
    // Retained for the projection commit validation introduced in Task 5.
    identity: PathBuf,
}

struct ProjectionSnapshot {
    infos: Vec<SessionInfo>,
    workspace_identity: Option<PathBuf>,
    metadata_root: Option<PathBuf>,
}

impl SessionProjection {
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        let registry = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned")
            .clone();
        let live_sessions: Vec<SessionInfo> = self
            .state
            .sessions
            .lock()
            .unwrap()
            .values()
            .map(|handle| handle.info.clone())
            .collect();
        let workspace = self.state.workspace.lock().unwrap().as_ref().map(|ws| {
            let metadata_root = ws.metadata.root_path();
            WorkspaceSnapshot {
                identity: metadata_root.clone(),
                metadata_root,
                workspace_path: ws.path.clone(),
            }
        });

        // State locks are released before constructing the reader or reading
        // metadata from disk.
        let metadata = workspace
            .as_ref()
            .map(|snapshot| metadata::MetadataStore::new(&snapshot.metadata_root));
        let metadata_map = metadata
            .as_ref()
            .map(|store| {
                live_sessions
                    .iter()
                    .filter_map(|info| {
                        store
                            .read_session(&info.id)
                            .map(|meta| (info.id.clone(), meta))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        let remembered_sessions = metadata
            .as_ref()
            .map(metadata::MetadataStore::read_all_sessions)
            .unwrap_or_default();
        let mut codex_native_session_ids = live_sessions
            .iter()
            .filter_map(|info| {
                let meta = metadata_map.get(&info.id);
                let harness_id = meta
                    .and_then(|meta| (!meta.harness.is_empty()).then_some(meta.harness.as_str()))
                    .or(info.harness_id.as_deref());
                (harness_id == Some("codex"))
                    .then(|| {
                        meta.and_then(|meta| meta.resume.as_ref())
                            .or(info.resume.as_ref())
                    })
                    .flatten()
                    .and_then(|resume| resume.harness_session_id.clone())
            })
            .collect::<Vec<_>>();
        codex_native_session_ids.extend(
            remembered_sessions
                .iter()
                .filter(|meta| meta.harness == "codex")
                .filter_map(|meta| {
                    meta.resume
                        .as_ref()
                        .and_then(|resume| resume.harness_session_id.clone())
                }),
        );
        codex_native_session_ids.sort_unstable();
        codex_native_session_ids.dedup();
        let saved_codex_session_ids =
            crate::codex_session_store::saved_sessions(&codex_native_session_ids);
        let live_ids: HashSet<String> = live_sessions.iter().map(|info| info.id.clone()).collect();
        let peon_last_inference = self.state.peon.last_inference.read().unwrap();

        let mut infos = live_sessions
            .into_iter()
            .map(|info| {
                let id = info.id.clone();
                let meta = metadata_map.get(&id);
                let harness_id = meta
                    .and_then(|meta| (!meta.harness.is_empty()).then_some(meta.harness.as_str()))
                    .or(info.harness_id.as_deref());
                let resolved_harness = resolve_harness_or_generic_shell(&registry, harness_id);
                let mut info = merge_live_session_info(
                    info,
                    meta,
                    peon_last_inference.get(&id),
                    resolved_harness,
                    &saved_codex_session_ids,
                );
                info.has_openable_plan = resolve_has_openable_plan(
                    meta.and_then(|meta| meta.plan_path.as_ref()),
                    workspace.as_ref(),
                );
                info
            })
            .collect::<Vec<_>>();

        for meta in remembered_sessions {
            if live_ids.contains(&meta.id) {
                continue;
            }
            infos.push(remembered_session_info(
                &meta,
                &registry,
                workspace.as_ref(),
                &saved_codex_session_ids,
            ));
        }

        ProjectionSnapshot {
            infos,
            workspace_identity: workspace.as_ref().map(|snapshot| snapshot.identity.clone()),
            metadata_root: workspace
                .as_ref()
                .map(|snapshot| snapshot.metadata_root.clone()),
        }
    }

    pub(crate) fn list(&self) -> Vec<SessionInfo> {
        let lock_state = self.state.clone();
        let _projection_lock = lock_state.projection_lock.lock().unwrap();
        let infos = self.project_capacity(self.snapshot());
        self.enrich_workspace(infos)
    }

    pub(crate) fn enrich_workspace(&self, mut infos: Vec<SessionInfo>) -> Vec<SessionInfo> {
        let session_pids = self.state.session_pids.lock().unwrap().clone();
        let reported_cwds = self.state.peon.reported_cwd.read().unwrap().clone();
        let effective_cwds = resolve_effective_cwds(
            &infos,
            &reported_cwds,
            &session_pids,
            crate::procfs::live_cwds,
        );
        enrich_sessions_with_git_context(&mut infos, &effective_cwds, git::detect);

        let conflict_warnings = detect_conflicts(&infos, &effective_cwds);
        for info in &mut infos {
            info.conflict_warning = conflict_warnings
                .iter()
                .find(|(id, _)| id == &info.id)
                .map(|(_, warning)| warning.clone());
        }
        infos
    }

    fn project_capacity(&self, snapshot: ProjectionSnapshot) -> Vec<SessionInfo> {
        let registry = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned")
            .clone();
        let workspace_identity = snapshot.workspace_identity;
        let metadata_root = snapshot.metadata_root;
        let projected_infos = snapshot.infos;
        let live_sessions: Vec<_> = {
            let sessions = self.state.sessions.lock().unwrap();
            sessions
                .values()
                .map(|h| {
                    (
                        h.info.clone(),
                        h.runtime.run_generation(),
                        h.output_buffer.snapshot(),
                        h.scan_buf.clone(),
                        h.capacity.clone(),
                    )
                })
                .collect()
        };
        let capacity_snapshots: HashMap<String, (u64, crate::capacity_state::CapacityState)> =
            live_sessions
                .iter()
                .map(|(info, generation, _, _, capacity)| {
                    (info.id.clone(), (*generation, capacity.clone()))
                })
                .collect();
        let capacity_metadata = metadata_root
            .as_ref()
            .map(|root| metadata::MetadataStore::new(root));
        let durable_harnesses: HashMap<String, String> = capacity_metadata
            .as_ref()
            .map(|metadata| {
                live_sessions
                    .iter()
                    .filter_map(|(info, _, _, _, _)| {
                        metadata
                            .read_session(&info.id)
                            .filter(|session| !session.harness.is_empty())
                            .map(|session| (info.id.clone(), session.harness))
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Per-session scoped-recheck decisions from `CapacityState::observe`,
        // kept for the write-back stage: the *latch* write-back uses the
        // harness-aggregated `at_usage_limit` computed below (a shared
        // provider limit shows as capped across every session of that
        // harness), but the recheck-window `origin_update`/`clear_latch`
        // decision is always this session's own.
        let mut observations: HashMap<String, crate::capacity_state::CapacityObservation> =
            HashMap::new();
        let mut pending_transitions: Vec<(String, bool)> = Vec::new();
        let capacity_infos: Vec<SessionInfo> = live_sessions
            .into_iter()
            .map(|(info, _, snapshot, scan_buf, capacity)| {
                let id = info.id.clone();
                let live_harness_id = info.harness_id.clone();
                let mut merged = projected_infos
                    .iter()
                    .find(|candidate| candidate.id == id)
                    .cloned()
                    .unwrap_or(info);
                let harness_id = durable_harnesses
                    .get(&id)
                    .map(String::as_str)
                    .or(live_harness_id.as_deref());
                let resolved_harness = resolve_harness_or_generic_shell(&registry, harness_id);

                let observation = resolved_harness.map(|harness| {
                    capacity.observe(harness.capacity_patterns(), &snapshot, &scan_buf)
                });
                merged.at_usage_limit = observation.as_ref().map(|o| o.at_usage_limit);
                if merged.lifecycle == "alive" && merged.at_usage_limit == Some(true) {
                    merged.attention = Some("capped".into());
                }
                let preserve_debug_hint = merged.metadata_source.as_deref() == Some("debug")
                    && merged.lifecycle == "alive"
                    && merged.attention.as_deref() == Some("capped");
                let detected_reset_hint = observation.as_ref().and_then(|o| o.reset_hint.clone());
                if !preserve_debug_hint || detected_reset_hint.is_some() {
                    merged.usage_limit_reset_hint = detected_reset_hint;
                }
                merged.capacity_check_pending = capacity.rendered_check_pending();

                let has_fresh_resume_output = capacity.capacity_check_pending
                    && !capacity.pending_capacity_visible_once
                    && capacity.fresh_output_since_origin();
                pending_transitions.push((id.clone(), has_fresh_resume_output));
                if let Some(observation) = observation {
                    observations.insert(id, observation);
                }
                merged
            })
            .collect();

        let mut infos = projected_infos;
        for info in &mut infos {
            let Some(capacity_info) = capacity_infos
                .iter()
                .find(|candidate| candidate.id == info.id)
            else {
                continue;
            };
            info.at_usage_limit = capacity_info.at_usage_limit;
            info.capacity_check_pending = capacity_info.capacity_check_pending;
            info.usage_limit_reset_hint = capacity_info.usage_limit_reset_hint.clone();
            if capacity_info.at_usage_limit == Some(true) && info.lifecycle == "alive" {
                info.attention = capacity_info.attention.clone();
            }
        }

        let mut harness_capped: HashMap<String, bool> = HashMap::new();
        let mut harness_reset_hint: HashMap<String, String> = HashMap::new();
        let mut provider_checking: HashSet<String> = HashSet::new();
        for info in &infos {
            if let (Some(hid), Some(capped)) = (&info.harness_id, info.at_usage_limit) {
                *harness_capped.entry(hid.clone()).or_insert(false) |= capped;
            }
            if let (Some(hid), Some(hint)) = (&info.harness_id, &info.usage_limit_reset_hint) {
                harness_reset_hint
                    .entry(hid.clone())
                    .or_insert_with(|| hint.clone());
            }
            if info.capacity_check_pending == Some(true) {
                if let Some(hid) = &info.harness_id {
                    provider_checking.insert(hid.clone());
                }
            }
        }
        for info in &mut infos {
            if info.memory_state != crate::session_types::MemoryState::Live {
                continue;
            }
            if let Some(hid) = &info.harness_id {
                if let Some(&capped) = harness_capped.get(hid) {
                    info.at_usage_limit = Some(capped);
                    if capped && info.lifecycle == "alive" {
                        info.attention = Some("capped".into());
                    }
                }
                if info.usage_limit_reset_hint.is_none() {
                    if let Some(hint) = harness_reset_hint.get(hid) {
                        info.usage_limit_reset_hint = Some(hint.clone());
                    }
                }
            }
        }
        let current_workspace_identity = self
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .map(|workspace| workspace.metadata.root_path());
        if current_workspace_identity != workspace_identity {
            return Vec::new();
        }
        let mut sessions = self.state.sessions.lock().unwrap();
        let mut write_back_snapshot_ids = HashSet::new();
        for info in &infos {
            if let Some(handle) = sessions.get_mut(&info.id) {
                let Some((generation, capacity_snapshot)) = capacity_snapshots.get(&info.id) else {
                    continue;
                };
                if handle.runtime.run_generation() != *generation
                    || handle.capacity != *capacity_snapshot
                {
                    continue;
                }
                write_back_snapshot_ids.insert(info.id.clone());
                let scoped_observation = observations.get(&info.id);
                let final_observation = crate::capacity_state::CapacityObservation {
                    at_usage_limit: info.at_usage_limit == Some(true),
                    reset_hint: None,
                    origin_update: scoped_observation.and_then(|o| o.origin_update),
                    clear_latch: scoped_observation.is_some_and(|o| o.clear_latch),
                };
                let newly_latched = handle.capacity.apply_observation(&final_observation);
                if newly_latched {
                    handle.runtime.usage_limit_latched_at = handle
                        .info
                        .last_output_at
                        .as_deref()
                        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
                        .map(|timestamp| timestamp.with_timezone(&chrono::Utc));
                }
            }
        }
        harness_capped.clear();
        harness_reset_hint.clear();
        provider_checking.clear();
        for info in &infos {
            if !write_back_snapshot_ids.contains(&info.id) {
                continue;
            }
            if let (Some(hid), Some(capped)) = (&info.harness_id, info.at_usage_limit) {
                *harness_capped.entry(hid.clone()).or_insert(false) |= capped;
            }
            if let (Some(hid), Some(hint)) = (&info.harness_id, &info.usage_limit_reset_hint) {
                harness_reset_hint
                    .entry(hid.clone())
                    .or_insert_with(|| hint.clone());
            }
            if info.capacity_check_pending == Some(true) {
                if let Some(hid) = &info.harness_id {
                    provider_checking.insert(hid.clone());
                }
            }
        }
        self.state.providers.update_session_capping(
            harness_capped,
            harness_reset_hint,
            provider_checking,
        );
        for (id, has_fresh_resume_output) in &pending_transitions {
            if !write_back_snapshot_ids.contains(id) {
                continue;
            }
            let Some(handle) = sessions.get_mut(id) else {
                continue;
            };
            if let Some(new_value) = handle
                .capacity
                .advance_pending_visibility(*has_fresh_resume_output)
            {
                handle.info.capacity_check_pending = new_value;
            }
        }
        infos
    }
}

pub(crate) fn enrich_sessions_with_git_context<F>(
    infos: &mut [SessionInfo],
    effective_cwds: &HashMap<String, String>,
    mut detect_git: F,
) where
    F: FnMut(&std::path::Path) -> git::GitContext,
{
    let cwd_for = |info: &SessionInfo| {
        effective_cwds
            .get(&info.id)
            .cloned()
            .unwrap_or_else(|| info.cwd.clone())
    };
    let mut cwd_counts: HashMap<String, usize> = HashMap::new();
    for info in infos.iter() {
        if info.status == "running" || info.status == "creating" {
            *cwd_counts.entry(cwd_for(info)).or_default() += 1;
        }
    }
    let mut contexts: HashMap<String, git::GitContext> = HashMap::new();
    for info in infos.iter_mut() {
        let cwd = cwd_for(info);
        let ctx = contexts
            .entry(cwd.clone())
            .or_insert_with(|| detect_git(std::path::Path::new(&cwd)));
        let count = cwd_counts.get(&cwd).copied().unwrap_or(1);
        info.recommendation = session_recommendation(ctx, count);
        info.repo_root = ctx.repo_root.clone();
        info.branch = ctx.branch.clone();
        info.dirty = Some(ctx.dirty);
        info.changed_files = Some(ctx.changed_files);
        info.is_worktree = Some(ctx.is_worktree);
    }
}

/// Projects a remembered (no live handle) session's persisted metadata into
/// a `SessionInfo`. Delegates all field precedence to
/// `session_view::project_session_info` — the same function the live path
/// uses — via a baseline built purely from `meta`
/// ([`SessionInfo::baseline_from_metadata`]) and `is_live: false` (a
/// remembered session's live handle is gone, regardless of what its stale
/// persisted `lifecycle`/`status` claim; issue #399).
fn remembered_session_info(
    meta: &metadata::SessionMetadata,
    registry: &crate::harness::registry::ResolvedHarnessRegistry,
    workspace: Option<&WorkspaceSnapshot>,
    saved_codex_session_ids: &std::collections::HashSet<String>,
) -> SessionInfo {
    let harness_id = (!meta.harness.is_empty()).then_some(meta.harness.as_str());
    let resolved_harness = resolve_harness_or_generic_shell(registry, harness_id);
    let baseline = SessionInfo::baseline_from_metadata(meta);
    let mut info = crate::session_view::project_session_info(
        baseline,
        Some(meta),
        false,
        None,
        resolved_harness,
        saved_codex_session_ids,
    );
    info.has_openable_plan = resolve_has_openable_plan(meta.plan_path.as_ref(), workspace);
    info
}

#[cfg(test)]
mod tests {
    use super::{remembered_session_info, SessionProjection};
    use crate::harness::definition::{BuiltinDocument, HarnessUserDocument, EMBEDDED_BUILTINS};
    use crate::harness::registry::resolve_document;
    use crate::session_types::MemoryState;
    use crate::test_support::test_session_metadata;
    use crate::AppState;
    use std::sync::Arc;

    #[test]
    fn exposes_a_constructor_for_shared_app_state() {
        let _constructor: fn(Arc<AppState>) -> SessionProjection = SessionProjection::new;
    }

    fn registry() -> crate::harness::registry::ResolvedHarnessRegistry {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        resolve_document(&builtins, &HarnessUserDocument::default()).unwrap()
    }

    /// A remembered session's `meta.last_activity` is a required, always-set
    /// field, so `remembered_session_info` must surface it directly rather
    /// than falling back to `created_at` — pins the deliberate decision
    /// documented on `session_view::project_session_info` (issue #399).
    #[test]
    fn remembered_session_info_uses_metas_last_activity_without_falling_back_to_created_at() {
        let meta = test_session_metadata(
            "remembered-1",
            "Remembered",
            "/tmp/project",
            "ended",
            "2026-06-28T09:00:00Z",
            "2026-06-28T09:05:00Z",
        );

        let info = remembered_session_info(&meta, &registry(), None, &Default::default());

        assert_eq!(info.created_at, "2026-06-28T09:00:00Z");
        assert_eq!(
            info.last_activity_at.as_deref(),
            Some("2026-06-28T09:05:00Z")
        );
    }

    /// A remembered session has no live handle to fall back to; a field
    /// `meta` doesn't carry must come out `None`, not silently substituted
    /// from a stray baseline value (issue #399).
    #[test]
    fn remembered_session_info_leaves_unset_meta_fields_absent() {
        let meta = test_session_metadata(
            "remembered-2",
            "Remembered",
            "/tmp/project",
            "ended",
            "2026-06-28T09:00:00Z",
            "2026-06-28T09:05:00Z",
        );
        assert!(meta.summary.is_none());
        assert!(meta.repo_root.is_none());

        let info = remembered_session_info(&meta, &registry(), None, &Default::default());

        assert_eq!(info.summary, None);
        assert_eq!(info.repo_root, None);
        assert_eq!(info.memory_state, MemoryState::Remembered);
    }

    /// `meta.attention` is normalized to `None` whenever `lifecycle != "alive"`
    /// (`metadata::reconcile`), which every remembered session satisfies —
    /// so routing through the shared alive-only attention rule in
    /// `project_session_info` is a no-op for this path, not a behavior
    /// change (issue #399).
    #[test]
    fn remembered_session_info_never_reports_attention_for_a_dead_session() {
        let mut meta = test_session_metadata(
            "remembered-3",
            "Remembered",
            "/tmp/project",
            "ended",
            "2026-06-28T09:00:00Z",
            "2026-06-28T09:05:00Z",
        );
        meta.lifecycle = "dead".into();

        let info = remembered_session_info(&meta, &registry(), None, &Default::default());

        assert_eq!(info.attention, None);
    }
}
