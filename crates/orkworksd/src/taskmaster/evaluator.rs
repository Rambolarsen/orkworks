//! Deterministic workflow-improvement evaluation.
//!
//! The implementation lives beside the canonical contract in `mod.rs` so the
//! evaluator and serialized model cannot drift. This module is the stable
//! Taskmaster-facing seam for the next coordinator increment.

use crate::{session_application::SessionApplication, AppState};
use std::sync::Arc;

pub(crate) fn refresh_now(state: &Arc<AppState>) {
    SessionApplication::new(state.clone()).refresh_workflow_recommendations();
}

pub(crate) fn schedule_evaluation(state: Arc<AppState>) {
    // Workspace opening is also exposed through a synchronous application seam
    // used by tests and non-HTTP callers. There is no async scheduler to use
    // in that context; runtime-backed callers still take the normal path.
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let (generation, workspace_id) = {
        let workspace = state.workspace.lock().unwrap();
        let Some(workspace) = workspace.as_ref() else {
            return;
        };
        (
            workspace.workflow_observations.next_evaluation_generation(),
            workspace.path.display().to_string(),
        )
    };
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let current = state.workspace.lock().unwrap();
        let still_current = current.as_ref().is_some_and(|workspace| {
            workspace.path.display().to_string() == workspace_id
                && workspace.workflow_observations.evaluation_generation() == generation
        });
        drop(current);
        if still_current {
            refresh_now(&state);
        }
    });
}
