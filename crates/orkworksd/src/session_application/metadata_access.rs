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
                crate::runtime::observed_status::mirror_agent_metadata_patch(
                    handle,
                    &updated.metadata,
                    scope,
                    updates_summary,
                );
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
