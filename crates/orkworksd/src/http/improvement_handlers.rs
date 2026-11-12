//! Electron-authorized, selected-workspace improvement transport.
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CancelRequest {
    run_id: String,
}

fn authorize(headers: &HeaderMap) -> Result<(), StatusCode> {
    super::taskmaster_settings_handlers::authorize_taskmaster_request(headers)
}

pub(crate) async fn status(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(status) = authorize(&headers) {
        return status.into_response();
    }
    let runtime = super::taskmaster_settings_handlers::runtime_for(&state);
    Json(crate::taskmaster::improvement::runtime::status(
        &state,
        runtime.root_path().to_path_buf(),
    ))
    .into_response()
}

pub(crate) async fn start(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(status) = authorize(&headers) {
        return status.into_response();
    }
    if !body.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let runtime = super::taskmaster_settings_handlers::runtime_for(&state);
    Json(crate::taskmaster::improvement::runtime::schedule(
        state,
        runtime.root_path().to_path_buf(),
    ))
    .into_response()
}

pub(crate) async fn cancel(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(status) = authorize(&headers) {
        return status.into_response();
    }
    let request: CancelRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if request.run_id.is_empty()
        || request.run_id.len() > 128
        || !request
            .run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    Json(crate::taskmaster::improvement::runtime::cancel(
        &state,
        &request.run_id,
    ))
    .into_response()
}

#[cfg(test)]
#[path = "improvement_handlers_tests.rs"]
mod tests;
