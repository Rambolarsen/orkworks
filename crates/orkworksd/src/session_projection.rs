use crate::git;
use crate::metadata;
use crate::plan_handoff::resolve_openable_plan_reference;
use crate::session_types::SessionInfo;
use crate::session_view::{
    detect_conflicts, merge_live_session_info, resolve_harness_or_generic_shell,
    session_recommendation,
};
use crate::AppState;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const GIT_CONTEXT_TTL: Duration = Duration::from_secs(5);
const MAX_GIT_CONTEXT_ENTRIES: usize = 512;

/// A fresh workspace may have no session directory. Once observed, its
/// disappearance is an incomplete read, including across stream reconnects.
#[derive(Default)]
pub(crate) struct SessionDirectoryState {
    directory: Option<PathBuf>,
    established: bool,
}

impl SessionDirectoryState {
    fn observe(&mut self, directory: &Path) -> io::Result<()> {
        if self.directory.as_deref() != Some(directory) {
            self.directory = Some(directory.to_owned());
            self.established = false;
        }
        match std::fs::metadata(directory) {
            Ok(metadata) if metadata.is_dir() => {
                self.established = true;
                Ok(())
            }
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "session directory is not a directory",
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound && !self.established => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn read(
        &mut self,
        store: &metadata::MetadataStore,
    ) -> io::Result<Vec<metadata::SessionMetadata>> {
        let directory = store.sessions_dir();
        self.observe(&directory)?;
        let sessions = read_remembered_sessions(store)?;
        self.observe(&directory)?;
        Ok(sessions)
    }
}

struct CachedGitContext {
    scanned_at: Instant,
    context: git::GitContext,
}

/// Process-local results shared across projection requests. Callers resolve
/// worktree roots before lookup; workspace adoption clears all entries.
#[derive(Default)]
pub(crate) struct GitContextCache {
    entries: HashMap<PathBuf, CachedGitContext>,
    pub(crate) changes: Arc<std::sync::Mutex<crate::git_changes::CaptureStore>>,
    pub(crate) changes_busy: Arc<std::sync::atomic::AtomicBool>,
}

impl GitContextCache {
    fn get_or_detect(
        &mut self,
        cwd: &Path,
        now: Instant,
        detect: impl FnOnce(&Path) -> git::GitContext,
    ) -> git::GitContext {
        if let Some(entry) = self.entries.get(cwd) {
            if now.saturating_duration_since(entry.scanned_at) < GIT_CONTEXT_TTL {
                return entry.context.clone();
            }
        }
        // Expiry is measured from scan start, not last access: frequent polls
        // must never extend the freshness window. Prune even unused roots.
        self.entries
            .retain(|_, entry| now.saturating_duration_since(entry.scanned_at) < GIT_CONTEXT_TTL);
        let context = detect(cwd);
        if self.entries.len() >= MAX_GIT_CONTEXT_ENTRIES {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.scanned_at)
                .map(|(path, _)| path.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(
            cwd.to_path_buf(),
            CachedGitContext {
                scanned_at: now,
                context: context.clone(),
            },
        );
        context
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.changes = Default::default();
        self.changes_busy = Default::default();
    }
}

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

    fn snapshot(&self) -> io::Result<ProjectionSnapshot> {
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
        let mut metadata_map = HashMap::new();
        if let Some(store) = metadata.as_ref() {
            for info in &live_sessions {
                if let Some(record) = store.try_read_revisioned_session(&info.id)? {
                    metadata_map.insert(info.id.clone(), record.metadata);
                }
            }
        }
        let remembered_sessions = metadata
            .as_ref()
            .map(|store| {
                self.state
                    .session_directory_state
                    .lock()
                    .map_err(|_| io::Error::other("session directory state lock poisoned"))?
                    .read(store)
            })
            .transpose()?
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

        Ok(ProjectionSnapshot {
            infos,
            workspace_identity: workspace.as_ref().map(|snapshot| snapshot.identity.clone()),
            metadata_root: workspace
                .as_ref()
                .map(|snapshot| snapshot.metadata_root.clone()),
        })
    }

    pub(crate) fn list(&self) -> io::Result<Vec<SessionInfo>> {
        let lock_state = self.state.clone();
        let _projection_lock = lock_state.projection_lock.lock().unwrap();
        let infos = self.project_capacity(self.snapshot()?)?;
        let mut infos = infos;
        self.project_organization(&mut infos);
        let infos = self.enrich_workspace(infos);
        Ok(infos)
    }

    pub(crate) fn project_organization(&self, infos: &mut [SessionInfo]) {
        self.project_organization_with(infos, |store, info, closed, active, key| {
            store.project(&info.id, &info.created_at, closed, active, key)
        });
    }

    fn project_organization_with(
        &self,
        infos: &mut [SessionInfo],
        mut read: impl FnMut(
            &crate::session_archive::OrganizationStore,
            &SessionInfo,
            bool,
            bool,
            &str,
        ) -> crate::session_archive::OrganizationProjection,
    ) {
        // Snapshot runtime authority under the workspace, sessions and PID
        // mutexes; release all three before filesystem reads. List projection
        // retains its existing outer projection gate for write-back ordering.
        let inputs = {
            let workspace = self.state.workspace.lock().unwrap();
            let sessions = self.state.sessions.lock().unwrap();
            let pids = self.state.session_pids.lock().unwrap();
            workspace.as_ref().map(|ws| {
                let inputs = infos
                    .iter()
                    .map(|info| {
                        let handle = sessions.get(&info.id);
                        let closed = info.lifecycle_phase == "ended"
                            && info.lifecycle == "dead"
                            && crate::session_application::session_actions::runtime_closed(
                                handle,
                                pids.contains_key(&info.id),
                            );
                        let key = crate::session_application::session_actions::runtime_key(handle);
                        let active = !crate::session_application::session_actions::runtime_closed(
                            handle,
                            pids.contains_key(&info.id),
                        );
                        (closed, active, key)
                    })
                    .collect::<Vec<_>>();
                (ws.metadata.organization.clone(), inputs)
            })
        };
        if let Some((store, inputs)) = inputs {
            for (info, (closed, active, key)) in infos.iter_mut().zip(inputs) {
                info.organization = read(&store, info, closed, active, &key);
            }
        }
    }

    pub(crate) fn enrich_workspace(&self, mut infos: Vec<SessionInfo>) -> Vec<SessionInfo> {
        let session_pids = self.state.session_pids.lock().unwrap().clone();
        let reported_cwds = self.state.peon.reported_cwd.read().unwrap().clone();
        let locations = crate::session_view::resolve_effective_locations(
            &infos,
            &reported_cwds,
            &session_pids,
            crate::procfs::live_cwds,
        );
        let effective_cwds = locations
            .iter()
            .map(|(id, (path, _))| (id.clone(), path.clone()))
            .collect();
        for info in &mut infos {
            info.git_context_source = locations.get(&info.id).map(|(_, source)| *source);
        }
        {
            let mut cache = self
                .state
                .git_context_cache
                .lock()
                .expect("Git context cache lock poisoned");
            enrich_sessions_with_git_context(&mut infos, &effective_cwds, |cwd| {
                cache.get_or_detect(cwd, Instant::now(), git::detect)
            });
        }

        let conflict_warnings = detect_conflicts(&infos, &effective_cwds);
        for info in &mut infos {
            info.conflict_warning = conflict_warnings
                .iter()
                .find(|(id, _)| id == &info.id)
                .map(|(_, warning)| warning.clone());
        }
        infos
    }

    fn project_capacity(&self, snapshot: ProjectionSnapshot) -> io::Result<Vec<SessionInfo>> {
        let registry = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned")
            .clone();
        let workspace_identity = snapshot.workspace_identity;
        let _metadata_root = snapshot.metadata_root;
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
        let durable_harnesses: HashMap<String, String> = projected_infos
            .iter()
            .filter_map(|info| {
                info.harness_id
                    .as_ref()
                    .map(|harness| (info.id.clone(), harness.clone()))
            })
            .collect();

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
                let preserve_debug_hint = merged.attention_source() == Some("debug")
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
            return Ok(Vec::new());
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
        Ok(infos)
    }
}

/// Scan remembered records without the legacy read path's quarantine and
/// skip-on-error behavior. A missing directory is a fresh workspace; a file
/// listed by the directory must remain readable and valid for this response.
fn read_remembered_sessions(
    store: &metadata::MetadataStore,
) -> io::Result<Vec<metadata::SessionMetadata>> {
    let entries = match std::fs::read_dir(store.sessions_dir()) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut sessions = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid session filename")
            })?;
        let record = store.try_read_revisioned_session(id)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "listed session record disappeared")
        })?;
        sessions.push(record.metadata);
    }
    sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(sessions)
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
        if info.organization.session_state != Some(crate::session_archive::SessionState::Archived)
            && (info.status == "running" || info.status == "creating")
        {
            *cwd_counts.entry(cwd_for(info)).or_default() += 1;
        }
    }
    let mut contexts: HashMap<String, git::GitContext> = HashMap::new();
    for info in infos.iter_mut() {
        if info
            .worktree_binding
            .as_ref()
            .is_some_and(|binding| binding.validate().is_err())
        {
            info.repo_root = None;
            info.branch = None;
            info.dirty = None;
            info.changed_files = None;
            info.line_changes = None;
            info.is_worktree = None;
            info.recommendation = Some("Reported task worktree unavailable.".into());
            continue;
        }
        let cwd = cwd_for(info);
        let git_cwd = git::worktree_root(std::path::Path::new(&cwd))
            .map(|root| root.display().to_string())
            .unwrap_or_else(|| cwd.clone());
        let ctx = contexts
            .entry(git_cwd.clone())
            .or_insert_with(|| detect_git(std::path::Path::new(&git_cwd)));
        let count = cwd_counts.get(&cwd).copied().unwrap_or(1);
        info.recommendation = if info.organization.session_state
            == Some(crate::session_archive::SessionState::Archived)
        {
            None
        } else {
            session_recommendation(ctx, count)
        };
        info.repo_root = ctx.repo_root.clone();
        info.branch = ctx.branch.clone();
        info.dirty = Some(ctx.dirty);
        info.changed_files = Some(ctx.changed_files);
        info.line_changes = ctx.line_changes;
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
    fn legacy_closed_session_exposes_organization_and_action_precondition() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        {
            let workspace = state.workspace.lock().unwrap();
            workspace
                .as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "old",
                    "Old",
                    dir.path().display().to_string(),
                    "ended",
                    "2024-01-01T00:00:00Z",
                    "2024-01-01T00:00:00Z",
                ));
        }
        let sessions = SessionProjection::new(state).list().unwrap();
        let json = serde_json::to_value(&sessions[0]).unwrap();
        assert_eq!(json["sessionState"], "closed");
        assert!(json["sessionActionToken"]
            .as_str()
            .is_some_and(|token| !token.is_empty()));
    }

    #[test]
    fn organization_reads_release_application_state_locks() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let mut infos = vec![crate::test_support::test_session_info(
            "one",
            "One",
            dir.path().display().to_string(),
            "ended",
            "before",
        )];
        SessionProjection::new(state.clone()).project_organization_with(
            &mut infos,
            |store, info, closed, active, key| {
                assert!(
                    state.workspace.try_lock().is_ok(),
                    "workspace locked during archive read"
                );
                assert!(
                    state.sessions.try_lock().is_ok(),
                    "sessions locked during archive read"
                );
                assert!(
                    state.session_pids.try_lock().is_ok(),
                    "PIDs locked during archive read"
                );
                store.project(&info.id, &info.created_at, closed, active, key)
            },
        );
        assert!(infos[0].organization.session_action_token.is_some());
    }

    #[test]
    fn archived_history_does_not_receive_live_workspace_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        let commit_id = repo
            .commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
            .unwrap();
        repo.branch("feature", &repo.find_commit(commit_id).unwrap(), false)
            .unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        std::fs::write(dir.path().join("dirty.txt"), "uncommitted").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        {
            let workspace = state.workspace.lock().unwrap();
            let ws = workspace.as_ref().unwrap();
            for id in ["archived", "closed"] {
                let mut meta = test_session_metadata(
                    id,
                    id,
                    dir.path().display().to_string(),
                    "ended",
                    "before",
                    "before",
                );
                meta.cwd = dir.path().display().to_string();
                ws.metadata.write_session(&meta);
            }
            ws.metadata
                .organization
                .write(
                    "archived",
                    "before",
                    crate::session_archive::SessionState::Archived,
                )
                .unwrap();
        }
        {
            let workspace = state.workspace.lock().unwrap();
            let ws = workspace.as_ref().unwrap();
            let mut stale = ws.metadata.read_session("archived").unwrap();
            stale.status = "running".into();
            stale.lifecycle = "alive".into();
            stale.lifecycle_phase = "alive".into();
            ws.metadata.write_session(&stale);
            stale.id = "live".into();
            ws.metadata.write_session(&stale);
        }
        let infos = SessionProjection::new(state).list().unwrap();
        let live = infos.iter().find(|info| info.id == "live").unwrap();
        assert!(
            !live
                .recommendation
                .as_deref()
                .unwrap_or("")
                .contains("Multiple sessions"),
            "archived stale metadata must not count as a second current session"
        );
        assert!(live.conflict_warning.is_none());
        let archived = infos.iter().find(|info| info.id == "archived").unwrap();
        let closed = infos.iter().find(|info| info.id == "closed").unwrap();
        assert!(
            closed.recommendation.is_some(),
            "fixture must derive current guidance"
        );
        assert!(archived.recommendation.is_none());
        assert!(archived.conflict_warning.is_none());
        assert_eq!(
            archived.dirty,
            Some(true),
            "read-only Git context remains available"
        );
    }

    #[test]
    fn exposes_a_constructor_for_shared_app_state() {
        let _constructor: fn(Arc<AppState>) -> SessionProjection = SessionProjection::new;
    }

    #[test]
    fn git_context_cache_survives_separate_projection_requests() {
        let workspace = tempfile::tempdir().unwrap();
        let repo_dir = tempfile::tempdir().unwrap();
        git2::Repository::init(repo_dir.path()).unwrap();
        let file = repo_dir.path().join("new.txt");
        std::fs::write(&file, "first\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(workspace.path());
        let infos = vec![crate::test_support::test_session_info(
            "one",
            "One",
            repo_dir.path().display().to_string(),
            "running",
            "now",
        )];
        let first = SessionProjection::new(state.clone()).enrich_workspace(infos.clone());
        assert_eq!(first[0].line_changes.unwrap().additions, 1);

        std::fs::write(&file, "first\nsecond\n").unwrap();
        let second = SessionProjection::new(state).enrich_workspace(infos);
        assert_eq!(
            second[0].line_changes.unwrap().additions,
            1,
            "a separate poll must reuse the Git result within its TTL"
        );
    }

    #[test]
    fn git_context_cache_reuses_scans_but_refreshes_at_expiry() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let file = dir.path().join("new.txt");
        std::fs::write(&file, "first\n").unwrap();
        let mut cache = super::GitContextCache::default();
        let now = std::time::Instant::now();
        let mut calls = 0;
        let mut detect = |cwd: &std::path::Path| {
            calls += 1;
            crate::git::detect(cwd)
        };
        let first = cache.get_or_detect(dir.path(), now, &mut detect);
        assert_eq!(first.line_changes.unwrap().additions, 1);
        std::fs::write(&file, "first\nsecond\n").unwrap();
        let reused = cache.get_or_detect(
            dir.path(),
            now + std::time::Duration::from_secs(4),
            &mut detect,
        );
        assert_eq!(reused.line_changes.unwrap().additions, 1);
        let refreshed = cache.get_or_detect(
            dir.path(),
            now + std::time::Duration::from_secs(5),
            &mut detect,
        );
        assert_eq!(refreshed.line_changes.unwrap().additions, 2);
        assert_eq!(refreshed.changed_files, 1);
        assert!(refreshed.dirty);
        assert_eq!(calls, 2, "a cache hit must skip the expensive detector");
    }

    #[test]
    fn git_context_cache_keeps_distinct_roots_and_evicts_oldest_when_full() {
        let mut cache = super::GitContextCache::default();
        let now = std::time::Instant::now();
        let context = crate::git::GitContext {
            repo_root: None,
            branch: None,
            dirty: false,
            changed_files: 0,
            line_changes: None,
            is_worktree: false,
        };
        for i in 0..513 {
            let path = std::path::PathBuf::from(format!("repo-{i}"));
            cache.get_or_detect(&path, now + std::time::Duration::from_millis(i), |_| {
                context.clone()
            });
        }
        assert_eq!(cache.entries.len(), 512);
        assert!(!cache.entries.contains_key(std::path::Path::new("repo-0")));
        assert!(cache.entries.contains_key(std::path::Path::new("repo-1")));
        assert!(cache.entries.contains_key(std::path::Path::new("repo-512")));
        cache.get_or_detect(
            std::path::Path::new("fresh"),
            now + std::time::Duration::from_secs(6),
            |_| context,
        );
        assert_eq!(
            cache.entries.len(),
            1,
            "unused expired roots must be removed"
        );
    }

    #[test]
    fn git_context_cache_shares_subdirectories_without_caching_session_recommendations() {
        let workspace = tempfile::tempdir().unwrap();
        let repo_dir = tempfile::tempdir().unwrap();
        git2::Repository::init(repo_dir.path()).unwrap();
        let child = repo_dir.path().join("child");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("new.txt"), "first\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(workspace.path());
        let mut infos = vec![crate::test_support::test_session_info(
            "one",
            "One",
            child.display().to_string(),
            "running",
            "now",
        )];
        let first = SessionProjection::new(state.clone()).enrich_workspace(infos.clone());
        assert!(first[0].recommendation.is_some());
        infos.push(crate::test_support::test_session_info(
            "two",
            "Two",
            child.display().to_string(),
            "running",
            "now",
        ));
        infos.push(crate::test_support::test_session_info(
            "three",
            "Three",
            repo_dir.path().display().to_string(),
            "running",
            "now",
        ));
        let projected = SessionProjection::new(state.clone()).enrich_workspace(infos);
        assert_eq!(state.git_context_cache.lock().unwrap().entries.len(), 1);
        assert_eq!(projected[0].line_changes.unwrap().additions, 1);
        assert_eq!(projected[2].line_changes.unwrap().additions, 1);
        assert_ne!(projected[0].recommendation, first[0].recommendation);
        assert!(projected[0].conflict_warning.is_some());
        assert!(projected[2].conflict_warning.is_none());
    }

    #[test]
    fn git_context_cache_is_cleared_when_a_workspace_is_adopted() {
        let home = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let next_workspace = tempfile::tempdir().unwrap();
        let repo_dir = tempfile::tempdir().unwrap();
        git2::Repository::init(repo_dir.path()).unwrap();
        let file = repo_dir.path().join("new.txt");
        std::fs::write(&file, "first\n").unwrap();
        crate::test_support::with_fake_home(home.path(), || {
            let state = crate::test_support::test_app_state_with_workspace(workspace.path());
            let infos = vec![crate::test_support::test_session_info(
                "one",
                "One",
                repo_dir.path().display().to_string(),
                "running",
                "now",
            )];
            SessionProjection::new(state.clone()).enrich_workspace(infos.clone());
            std::fs::write(&file, "first\nsecond\n").unwrap();
            crate::session_application::SessionApplication::new(state.clone())
                .open_workspace(next_workspace.path().to_path_buf())
                .unwrap();
            let projected = SessionProjection::new(state).enrich_workspace(infos);
            assert_eq!(projected[0].line_changes.unwrap().additions, 2);
        });
    }

    #[test]
    fn git_context_cache_keeps_linked_worktrees_separate() {
        let workspace = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let main = root.path().join("main");
        let linked = root.path().join("linked");
        let repo = git2::Repository::init(&main).unwrap();
        let mut index = repo.index().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
            .unwrap();
        repo.worktree("linked", &linked, None).unwrap();
        std::fs::write(main.join("new.txt"), "one\n").unwrap();
        std::fs::write(linked.join("new.txt"), "one\ntwo\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(workspace.path());
        let infos = vec![
            crate::test_support::test_session_info(
                "one",
                "One",
                main.display().to_string(),
                "running",
                "now",
            ),
            crate::test_support::test_session_info(
                "two",
                "Two",
                linked.display().to_string(),
                "running",
                "now",
            ),
        ];
        let first = SessionProjection::new(state.clone()).enrich_workspace(infos.clone());
        assert_eq!(first[0].line_changes.unwrap().additions, 1);
        assert_eq!(first[1].line_changes.unwrap().additions, 2);
        assert_eq!(first[0].is_worktree, Some(false));
        assert_eq!(first[1].is_worktree, Some(true));
        std::fs::write(main.join("new.txt"), "three\nfour\nfive\n").unwrap();
        let cached = SessionProjection::new(state.clone()).enrich_workspace(infos);
        assert_eq!(cached[0].line_changes.unwrap().additions, 1);
        assert_eq!(cached[1].line_changes.unwrap().additions, 2);
        assert_eq!(state.git_context_cache.lock().unwrap().entries.len(), 2);
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
