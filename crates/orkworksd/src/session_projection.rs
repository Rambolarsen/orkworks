use crate::harness::registry::ResolvedHarness;
use crate::metadata;
use crate::plan_handoff::resolve_openable_plan_reference;
use crate::session_types::SessionInfo;
use crate::session_view::{
    connectivity_for_status, derive_memory_state, merge_live_session_info,
    terminal_outcome_for_status,
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

impl SessionProjection {
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub(crate) fn list(&self) -> Vec<SessionInfo> {
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
                        store.read_session(&info.id).map(|meta| (info.id.clone(), meta))
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
                    .and_then(|id| registry.get(id))
                    .or_else(|| registry.get("generic-shell"));
                let mut info = merge_live_session_info(
                    info,
                    meta,
                    peon_last_inference.get(&id),
                    resolved_harness,
                );
                info.has_openable_plan = meta
                    .and_then(|meta| meta.plan_path.as_ref())
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

        // Task 5 validates this snapshot identity while holding the projection
        // lock before committing write-backs. Capturing it here keeps this read
        // stage independent from the workspace lock.
        let _workspace_identity = workspace.map(|snapshot| snapshot.identity);
        infos
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
