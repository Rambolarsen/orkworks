use serde::Serialize;

pub(crate) mod harness_handlers;
pub(crate) mod inference_trust_handlers;
pub(crate) mod integration_handlers;
pub(crate) mod provider_handlers;
pub(crate) mod retention_handlers;
pub(crate) mod session_handlers;
pub(crate) mod taskmaster_handlers;
pub(crate) mod taskmaster_settings_handlers;
pub(crate) mod workflow_observation_handlers;

#[derive(Serialize)]
pub(crate) struct ErrorResponse {
    pub(crate) error: String,
}

pub(crate) mod session_metadata_handlers;

pub(crate) mod git_changes_handlers;

pub(crate) mod readiness_handlers;
