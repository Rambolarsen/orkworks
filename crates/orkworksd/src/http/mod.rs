use serde::Serialize;

pub(crate) mod harness_handlers;
pub(crate) mod provider_handlers;
pub(crate) mod retention_handlers;

#[derive(Serialize)]
pub(crate) struct ErrorResponse {
    pub(crate) error: String,
}
