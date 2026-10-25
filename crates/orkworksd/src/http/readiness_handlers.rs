//! Electron-authorized, selected-workspace-only readiness endpoints.
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
pub(crate) async fn assess(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(status) = super::taskmaster_settings_handlers::authorize_taskmaster_request(&headers)
    {
        return status.into_response();
    }
    if !body.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let runtime = super::taskmaster_settings_handlers::runtime_for(&state);
    Json(crate::taskmaster::readiness::schedule(
        state,
        runtime.root_path().to_path_buf(),
    ))
    .into_response()
}
pub(crate) async fn get_status(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(status) = super::taskmaster_settings_handlers::authorize_taskmaster_request(&headers)
    {
        return status.into_response();
    }
    let runtime = super::taskmaster_settings_handlers::runtime_for(&state);
    Json(crate::taskmaster::readiness::status(
        &state,
        runtime.root_path().to_path_buf(),
    ))
    .into_response()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn readiness_routes_require_electron_authorization_and_empty_admission_body() {
        let _token = crate::test_support::open_plan_token::set("readiness-test-token").await;
        let directory = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(directory.path());
        assert_eq!(
            assess(State(state.clone()), HeaderMap::new(), Bytes::new())
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get_status(State(state.clone()), HeaderMap::new())
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-orkworks-open-plan-token",
            "readiness-test-token".parse().unwrap(),
        );
        assert_eq!(
            assess(
                State(state.clone()),
                headers.clone(),
                Bytes::from_static(b"{}")
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        let response = get_status(State(state), headers).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
