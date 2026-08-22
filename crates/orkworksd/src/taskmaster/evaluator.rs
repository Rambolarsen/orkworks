//! Deterministic workflow-improvement evaluation.
//!
//! The implementation lives beside the canonical contract in `mod.rs` so the
//! evaluator and serialized model cannot drift. This module is the stable
//! Taskmaster-facing seam for the next coordinator increment.

pub(crate) use super::evaluate_workflow_improvements;

use crate::AppState;
use std::sync::Arc;

pub(crate) fn refresh_now(state: &Arc<AppState>) {
    let workspace = state.workspace.lock().unwrap();
    let Some(workspace) = workspace.as_ref() else {
        return;
    };
    let Ok(observations) = workspace.workflow_observations.workspace_observations() else {
        return;
    };
    let Ok(existing) = workspace.recommendation_store.list() else {
        return;
    };
    let now = chrono::Utc::now().to_rfc3339();
    let proposals = evaluate_workflow_improvements(
        &observations,
        &existing,
        &workspace.path.display().to_string(),
        &now,
    );
    for proposal in proposals {
        if let Err(error) = workspace.recommendation_store.put(&proposal) {
            tracing::warn!(recommendation_id = %proposal.id, %error, "failed to persist Taskmaster recommendation");
        }
    }
}

pub(crate) fn schedule_evaluation(state: Arc<AppState>) {
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
