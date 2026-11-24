//! Authenticated live-session revisioned metadata read/patch operations.

use crate::metadata::agent_patch::{AgentPatchEnvelope, AgentPatchError};
use crate::runtime::terminal_runtime::verify_workflow_report_token;
use crate::session_types::MemoryState;
use crate::AppState;
use serde::Serialize;
use std::path::Path as FilePath;

pub(crate) enum MetadataOperation {
    Read,
    Patch { body: Vec<u8>, validate_only: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MetadataAccessError {
    Unauthorized,
    BadRequest,
    Conflict,
    Internal,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidatedResponse {
    validated: bool,
    metadata_revision: crate::metadata::session_transactions::MetadataRevision,
}

enum ParsedOperation {
    Read,
    Patch {
        envelope: AgentPatchEnvelope,
        validate_only: bool,
    },
}

pub(crate) fn perform(
    state: &AppState,
    id: &str,
    token: &str,
    operation: MetadataOperation,
) -> Result<serde_json::Value, MetadataAccessError> {
    // Keep workspace and session identity stable through the store transaction.
    // The order matches existing application code: workspace, then sessions.
    let workspace_guard = state
        .workspace
        .lock()
        .map_err(|_| MetadataAccessError::Internal)?;
    let workspace = workspace_guard
        .as_ref()
        .ok_or(MetadataAccessError::Unauthorized)?;
    let mut sessions_guard = state
        .sessions
        .lock()
        .map_err(|_| MetadataAccessError::Internal)?;
    let live = sessions_guard.get(id).is_some_and(|handle| {
        handle.info.id == id
            && handle.info.memory_state == MemoryState::Live
            && handle.info.lifecycle == "alive"
            && handle.info.lifecycle_phase == "active"
    });
    if !live || !verify_workflow_report_token(id, token) {
        return Err(MetadataAccessError::Unauthorized);
    }
    let operation = match operation {
        MetadataOperation::Patch {
            body,
            validate_only,
        } => {
            let envelope = serde_json::from_slice::<AgentPatchEnvelope>(&body)
                .map_err(|_| MetadataAccessError::BadRequest)?;
            ParsedOperation::Patch {
                envelope,
                validate_only,
            }
        }
        MetadataOperation::Read => ParsedOperation::Read,
    };
    let record = workspace
        .metadata
        .try_read_revisioned_session(id)
        .map_err(|_| MetadataAccessError::Internal)?
        .ok_or(MetadataAccessError::Internal)?;
    if FilePath::new(&record.metadata.workspace) != workspace.path {
        return Err(MetadataAccessError::Unauthorized);
    }
    if record.metadata.lifecycle != "alive"
        || record.metadata.lifecycle_phase != "active"
        || record.metadata.status != "running"
    {
        return Err(MetadataAccessError::Unauthorized);
    }
    match operation {
        ParsedOperation::Read => {
            serde_json::to_value(record).map_err(|_| MetadataAccessError::Internal)
        }
        ParsedOperation::Patch {
            envelope,
            validate_only,
        } => {
            if validate_only {
                let revision = workspace
                    .metadata
                    .validate_agent_patch(id, &envelope.metadata_revision, &envelope.patch)
                    .map_err(map_patch_error)?;
                serde_json::to_value(ValidatedResponse {
                    validated: true,
                    metadata_revision: revision,
                })
                .map_err(|_| MetadataAccessError::Internal)
            } else {
                let scope = envelope.patch.scope();
                let updates_summary = envelope.patch.updates_summary();
                let updated = workspace
                    .metadata
                    .apply_agent_patch(id, &envelope.metadata_revision, &envelope.patch)
                    .map_err(map_patch_error)?;
                let value =
                    serde_json::to_value(&updated).map_err(|_| MetadataAccessError::Internal)?;
                let handle = sessions_guard
                    .get_mut(id)
                    .ok_or(MetadataAccessError::Internal)?;
                handle.info.last_activity_at = Some(updated.metadata.last_activity.clone());
                handle.info.worktree_binding = updated.metadata.worktree_binding.clone();
                if scope.attention {
                    handle.runtime.attention_owner.accepted_write();
                    handle.info.observed_status = updated.metadata.observed_status.clone();
                    handle.info.attention = updated.metadata.attention.clone();
                    handle.info.needs_user_input = updated.metadata.needs_user_input;
                    handle.info.detected_question = updated.metadata.detected_question.clone();
                    handle.info.suggested_options = updated.metadata.suggested_options.clone();
                    handle.info.provenance.attention_source =
                        updated.metadata.provenance.attention_source.clone();
                    handle.info.provenance.attention_confidence =
                        updated.metadata.provenance.attention_confidence;
                    handle.info.provenance.attention_origin =
                        updated.metadata.provenance.attention_origin.clone();
                    handle.info.provenance.attention_updated_at =
                        updated.metadata.provenance.attention_updated_at.clone();
                    if let Some(tracker) = handle.runtime.native_approval.as_mut() {
                        tracker.invalidate();
                    }
                    handle.pending_work_signal = None;
                    handle.runtime.active_prompt_kind = None;
                }
                if scope.work {
                    handle.info.work_phase = updated.metadata.work_phase.clone();
                    handle.info.next_action = updated.metadata.next_action.clone();
                    handle.info.blocker_description = updated.metadata.blocker_description.clone();
                    handle.info.failed_command = updated.metadata.failed_command.clone();
                    handle.info.failed_test = updated.metadata.failed_test.clone();
                    handle.info.metadata_source = Some(updated.metadata.metadata_source.clone());
                    handle.info.metadata_confidence = Some(updated.metadata.metadata_confidence);
                    handle.info.provenance.work_metadata_updated_at =
                        updated.metadata.provenance.work_metadata_updated_at.clone();
                }
                if updates_summary {
                    handle.info.summary = updated.metadata.summary.clone();
                    handle.info.provenance.summary_source =
                        updated.metadata.provenance.summary_source.clone();
                    handle.info.provenance.summary_confidence =
                        updated.metadata.provenance.summary_confidence;
                    handle.info.provenance.summary_observed_at =
                        updated.metadata.provenance.summary_observed_at.clone();
                }
                Ok(value)
            }
        }
    }
}

fn map_patch_error(error: AgentPatchError) -> MetadataAccessError {
    match error {
        AgentPatchError::Conflict => MetadataAccessError::Conflict,
        AgentPatchError::BadRequest => MetadataAccessError::BadRequest,
        AgentPatchError::Unauthorized => MetadataAccessError::Unauthorized,
        AgentPatchError::Io(error) => {
            tracing::error!(error = %error, "session metadata transaction failed");
            MetadataAccessError::Internal
        }
    }
}
