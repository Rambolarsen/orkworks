use crate::git;
use crate::harness::registry::ResolvedHarness;
use crate::metadata;
use crate::peon;
use crate::plan_handoff::resolve_openable_plan_reference;
use crate::session_types::SessionInfo;
use crate::session_view::{
    connectivity_for_status, derive_memory_state, detect_conflicts, merge_live_session_info,
    resolve_effective_cwds, session_recommendation, terminal_outcome_for_status,
};
use crate::AppState;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

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
        let live_ids: HashSet<String> = live_sessions.iter().map(|info| info.id.clone()).collect();
        let peon_last_inference = self.state.peon.last_inference.read().unwrap();

        let mut infos = live_sessions
            .into_iter()
            .map(|info| {
                let id = info.id.clone();
                let meta = metadata_map.get(&id);
                let resolved_harness = meta
                    .and_then(|meta| (!meta.harness.is_empty()).then_some(meta.harness.as_str()))
                    .or(info.harness_id.as_deref())
                    .and_then(|id| registry.get(id))
                    .or_else(|| registry.get("generic-shell"));
                let mut info = merge_live_session_info(
                    info,
                    meta,
                    peon_last_inference.get(&id),
                    resolved_harness,
                );
                info.has_openable_plan =
                    meta.and_then(|meta| meta.plan_path.as_ref())
                        .and_then(|reference| {
                            workspace.as_ref().map(|snapshot| {
                                resolve_openable_plan_reference(&snapshot.workspace_path, reference)
                                    .is_ok()
                            })
                        });
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
        self.list_with_hook(|| {})
    }

    pub(crate) fn list_with_hook(&self, before_write_back: impl FnOnce()) -> Vec<SessionInfo> {
        let lock_state = self.state.clone();
        let _projection_lock = lock_state.projection_lock.lock().unwrap();
        let infos = self.project_capacity(self.snapshot(), before_write_back);
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

    fn project_capacity(
        &self,
        snapshot: ProjectionSnapshot,
        before_write_back: impl FnOnce(),
    ) -> Vec<SessionInfo> {
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
                        h.at_usage_limit_latched,
                        h.capacity_check_pending,
                        h.output_lines_seen,
                        h.scan_bytes_seen,
                        h.resume_scan_origin,
                        h.pending_capacity_visible_once,
                    )
                })
                .collect()
        };
        let capacity_snapshots: HashMap<
            String,
            (u64, bool, bool, u64, u64, Option<(u64, u64)>, bool),
        > = live_sessions
            .iter()
            .map(
                |(info, generation, _, _, latched, pending, lines, bytes, origin, visible)| {
                    (
                        info.id.clone(),
                        (
                            *generation,
                            *latched,
                            *pending,
                            *lines,
                            *bytes,
                            *origin,
                            *visible,
                        ),
                    )
                },
            )
            .collect();
        let capacity_metadata = metadata_root
            .as_ref()
            .map(|root| metadata::MetadataStore::new(root));
        let durable_harnesses: HashMap<String, String> = capacity_metadata
            .as_ref()
            .map(|metadata| {
                live_sessions
                    .iter()
                    .filter_map(|(info, _, _, _, _, _, _, _, _, _)| {
                        metadata
                            .read_session(&info.id)
                            .filter(|session| !session.harness.is_empty())
                            .map(|session| (info.id.clone(), session.harness))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut pending_transitions: Vec<(String, bool, bool)> = Vec::new();
        let mut capped_recheck_resets: HashSet<String> = HashSet::new();
        let mut capped_clear_baselines: HashMap<String, (u64, u64)> = HashMap::new();
        let capacity_infos: Vec<SessionInfo> = live_sessions
            .into_iter()
            .map(
                |(
                    info,
                    _,
                    snapshot,
                    scan_buf,
                    prev_latch,
                    pending,
                    output_lines_seen,
                    scan_bytes_seen,
                    origin,
                    pending_visible_once,
                )| {
                    let id = info.id.clone();
                    let live_harness_id = info.harness_id.clone();
                    let mut merged = projected_infos
                        .iter()
                        .find(|candidate| candidate.id == id)
                        .cloned()
                        .unwrap_or(info);
                    let resolved_harness = durable_harnesses
                        .get(&id)
                        .map(String::as_str)
                        .or(live_harness_id.as_deref())
                        .and_then(|id| registry.get(id))
                        .or_else(|| registry.get("generic-shell"));
                    let fresh_output_since_origin = origin
                        .map(|(line_count, scan_len)| {
                            output_lines_seen > line_count || scan_bytes_seen > scan_len
                        })
                        .unwrap_or(false);
                    let has_fresh_resume_output =
                        pending && !pending_visible_once && fresh_output_since_origin;
                    let limit_patterns = resolved_harness
                        .map(|harness| harness.capacity_patterns())
                        .unwrap_or(&[]);
                    let stale_cap_recheck = prev_latch && !pending && origin.is_some();
                    let baseline_scoped_detection = !prev_latch && !pending && origin.is_some();
                    merged.at_usage_limit = resolved_harness.map(|_| {
                        let detected_full = peon::detect_usage_limit(limit_patterns, &snapshot)
                            || peon::detect_usage_limit_raw(limit_patterns, &scan_buf);
                        if stale_cap_recheck && fresh_output_since_origin {
                            let (line_count, scan_len) = origin.unwrap();
                            let line_window_start =
                                output_lines_seen.saturating_sub(snapshot.len() as u64);
                            let scan_window_start =
                                scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                            let fresh_line_start =
                                line_count.saturating_sub(line_window_start) as usize;
                            let fresh_scan_start =
                                scan_len.saturating_sub(scan_window_start) as usize;
                            let fresh_lines = snapshot
                                .get(fresh_line_start.min(snapshot.len())..)
                                .unwrap_or(&[]);
                            let fresh_scan = scan_buf
                                .get(fresh_scan_start.min(scan_buf.len())..)
                                .unwrap_or("");
                            let detected_scoped =
                                peon::detect_usage_limit(limit_patterns, fresh_lines)
                                    || peon::detect_usage_limit_raw(limit_patterns, fresh_scan);
                            capped_recheck_resets.insert(id.clone());
                            if !detected_scoped {
                                capped_clear_baselines
                                    .insert(id.clone(), (output_lines_seen, scan_bytes_seen));
                            }
                            detected_scoped
                        } else if baseline_scoped_detection {
                            let (line_count, scan_len) = origin.unwrap();
                            let line_window_start =
                                output_lines_seen.saturating_sub(snapshot.len() as u64);
                            let scan_window_start =
                                scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                            let fresh_line_start =
                                line_count.saturating_sub(line_window_start) as usize;
                            let fresh_scan_start =
                                scan_len.saturating_sub(scan_window_start) as usize;
                            let fresh_lines = snapshot
                                .get(fresh_line_start.min(snapshot.len())..)
                                .unwrap_or(&[]);
                            let fresh_scan = scan_buf
                                .get(fresh_scan_start.min(scan_buf.len())..)
                                .unwrap_or("");
                            let detected_scoped =
                                peon::detect_usage_limit(limit_patterns, fresh_lines)
                                    || peon::detect_usage_limit_raw(limit_patterns, fresh_scan);
                            if detected_scoped {
                                capped_recheck_resets.insert(id.clone());
                            }
                            detected_scoped
                        } else {
                            prev_latch || detected_full
                        }
                    });
                    if merged.lifecycle == "alive" && merged.at_usage_limit == Some(true) {
                        merged.attention = Some("capped".into());
                    }
                    let detected_reset_hint = resolved_harness.and_then(|_| {
                        if stale_cap_recheck && fresh_output_since_origin {
                            let (line_count, scan_len) = origin.unwrap();
                            let line_window_start =
                                output_lines_seen.saturating_sub(snapshot.len() as u64);
                            let scan_window_start =
                                scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                            let fresh_line_start =
                                line_count.saturating_sub(line_window_start) as usize;
                            let fresh_scan_start =
                                scan_len.saturating_sub(scan_window_start) as usize;
                            let fresh_lines = snapshot
                                .get(fresh_line_start.min(snapshot.len())..)
                                .unwrap_or(&[]);
                            let fresh_scan = scan_buf
                                .get(fresh_scan_start.min(scan_buf.len())..)
                                .unwrap_or("");
                            peon::detect_usage_limit_hint(limit_patterns, fresh_lines).or_else(
                                || peon::detect_usage_limit_hint_raw(limit_patterns, fresh_scan),
                            )
                        } else if baseline_scoped_detection {
                            let (line_count, scan_len) = origin.unwrap();
                            let line_window_start =
                                output_lines_seen.saturating_sub(snapshot.len() as u64);
                            let scan_window_start =
                                scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                            let fresh_line_start =
                                line_count.saturating_sub(line_window_start) as usize;
                            let fresh_scan_start =
                                scan_len.saturating_sub(scan_window_start) as usize;
                            let fresh_lines = snapshot
                                .get(fresh_line_start.min(snapshot.len())..)
                                .unwrap_or(&[]);
                            let fresh_scan = scan_buf
                                .get(fresh_scan_start.min(scan_buf.len())..)
                                .unwrap_or("");
                            peon::detect_usage_limit_hint(limit_patterns, fresh_lines).or_else(
                                || peon::detect_usage_limit_hint_raw(limit_patterns, fresh_scan),
                            )
                        } else {
                            peon::detect_usage_limit_hint(limit_patterns, &snapshot).or_else(|| {
                                peon::detect_usage_limit_hint_raw(limit_patterns, &scan_buf)
                            })
                        }
                    });
                    let preserve_debug_hint = merged.metadata_source.as_deref() == Some("debug")
                        && merged.lifecycle == "alive"
                        && merged.attention.as_deref() == Some("capped");
                    if !preserve_debug_hint || detected_reset_hint.is_some() {
                        merged.usage_limit_reset_hint = detected_reset_hint;
                    }
                    merged.capacity_check_pending = if pending && !pending_visible_once {
                        Some(true)
                    } else {
                        None
                    };
                    pending_transitions.push((id, has_fresh_resume_output, pending_visible_once));
                    merged
                },
            )
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
        before_write_back();
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
                let Some((generation, latched, pending, lines, bytes, origin, visible)) =
                    capacity_snapshots.get(&info.id)
                else {
                    continue;
                };
                if handle.runtime.run_generation() != *generation
                    || handle.at_usage_limit_latched != *latched
                    || handle.capacity_check_pending != *pending
                    || handle.pending_capacity_visible_once != *visible
                    || handle.resume_scan_origin != *origin
                    || handle.output_lines_seen != *lines
                    || handle.scan_bytes_seen != *bytes
                {
                    continue;
                }
                write_back_snapshot_ids.insert(info.id.clone());
                if info.at_usage_limit == Some(true) {
                    if !handle.at_usage_limit_latched {
                        handle.runtime.usage_limit_latched_at = handle
                            .info
                            .last_output_at
                            .as_deref()
                            .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
                            .map(|timestamp| timestamp.with_timezone(&chrono::Utc));
                    }
                    handle.at_usage_limit_latched = true;
                }
                if let Some(origin) = capped_clear_baselines.get(&info.id) {
                    handle.resume_scan_origin = Some(*origin);
                    handle.at_usage_limit_latched = false;
                } else if capped_recheck_resets.contains(&info.id) {
                    handle.resume_scan_origin = None;
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
        for (id, has_fresh_resume_output, pending_visible_once) in &pending_transitions {
            if !write_back_snapshot_ids.contains(id) {
                continue;
            }
            let Some(handle) = sessions.get_mut(id) else {
                continue;
            };
            if !handle.capacity_check_pending {
                continue;
            }
            if *pending_visible_once {
                handle.capacity_check_pending = false;
                handle.resume_scan_origin = None;
                handle.pending_capacity_visible_once = false;
                handle.info.capacity_check_pending = None;
            } else if *has_fresh_resume_output {
                handle.pending_capacity_visible_once = true;
                handle.resume_scan_origin = None;
                handle.info.capacity_check_pending = Some(true);
            } else {
                handle.info.capacity_check_pending = Some(true);
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

fn remembered_session_info(
    meta: &metadata::SessionMetadata,
    registry: &crate::harness::registry::ResolvedHarnessRegistry,
    workspace: Option<&WorkspaceSnapshot>,
) -> SessionInfo {
    let resolved_harness = (!meta.harness.is_empty())
        .then_some(meta.harness.as_str())
        .and_then(|id| registry.get(id))
        .or_else(|| registry.get("generic-shell"));
    let (memory_state, resume_strategy) =
        derive_memory_state(false, meta.resume.as_ref(), resolved_harness);
    let (resume_exact, resume_latest_cwd, resume_latest_repo) = resolved_harness
        .map(ResolvedHarness::resume_flags)
        .unwrap_or_default();
    SessionInfo {
        id: meta.id.clone(),
        label: meta.label.clone(),
        harness_id: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
        model_provider_id: meta.provider_id.clone(),
        model_id: (!meta.model.is_empty()).then(|| meta.model.clone()),
        harness: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
        model: (!meta.model.is_empty()).then(|| meta.model.clone()),
        work_phase: meta.work_phase.clone(),
        lifecycle_phase: meta.lifecycle_phase.clone(),
        lifecycle: meta.lifecycle.clone(),
        attention: meta.attention.clone(),
        status: meta.status.clone(),
        connectivity: Some(connectivity_for_status(&meta.status).into()),
        terminal_outcome: terminal_outcome_for_status(&meta.status),
        cwd: meta.cwd.clone(),
        created_at: meta.created_at.clone(),
        last_activity_at: Some(meta.last_activity.clone()),
        last_output_at: meta.last_output_at.clone(),
        final_observed_status: meta
            .final_observed_status_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.value.clone()),
        observed_status: meta.observed_status.clone(),
        summary: meta.summary.clone(),
        next_action: meta.next_action.clone(),
        needs_user_input: meta.needs_user_input,
        detected_question: meta.detected_question.clone(),
        suggested_options: meta.suggested_options.clone(),
        blocker_description: meta.blocker_description.clone(),
        failed_command: meta.failed_command.clone(),
        failed_test: meta.failed_test.clone(),
        capacity_hints: meta.capacity_hints.clone(),
        at_usage_limit: None,
        capacity_check_pending: None,
        usage_limit_reset_hint: None,
        metadata_source: Some(meta.metadata_source.clone()),
        metadata_confidence: Some(meta.metadata_confidence),
        peon_last_inference: meta.peon_last_inference.clone(),
        repo_root: meta.repo_root.clone(),
        branch: meta.branch.clone(),
        dirty: meta.dirty,
        changed_files: meta.changed_files,
        is_worktree: meta.is_worktree,
        conflict_warning: None,
        recommendation: None,
        memory_state,
        resume_strategy: resume_strategy.clone(),
        resume: meta.resume.clone(),
        resume_options: metadata::derive_resume_options(
            &resume_strategy,
            meta.resume.as_ref(),
            resume_exact,
            resume_latest_cwd,
            resume_latest_repo,
        ),
        resumed_from: meta.resumed_from.clone(),
        has_openable_plan: meta.plan_path.as_ref().and_then(|reference| {
            workspace.map(|snapshot| {
                resolve_openable_plan_reference(&snapshot.workspace_path, reference).is_ok()
            })
        }),
        provider: meta.provider_label.clone(),
        provider_model: meta.provider_model.clone(),
        provider_state: meta.provider_state.clone(),
    }
}

mod tests {
    use super::SessionProjection;
    use crate::AppState;
    use std::sync::Arc;

    #[test]
    fn exposes_a_constructor_for_shared_app_state() {
        let _constructor: fn(Arc<AppState>) -> SessionProjection = SessionProjection::new;
    }
}
