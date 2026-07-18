use crate::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct RetentionRequest {
    #[serde(rename = "maxSessions", default)]
    pub(crate) max_sessions: usize,
    #[serde(rename = "maxAgeDays", default)]
    pub(crate) max_age_days: u32,
}

pub(crate) async fn set_retention(
    State(state): State<Arc<AppState>>,
    axum::Json(req): axum::Json<RetentionRequest>,
) -> impl IntoResponse {
    let mut config = state.retention_config.write().await;
    config.max_sessions = req.max_sessions;
    config.max_age_days = req.max_age_days;
    tracing::info!(
        max_sessions = config.max_sessions,
        max_age_days = config.max_age_days,
        "retention config updated"
    );
    axum::http::StatusCode::OK
}
