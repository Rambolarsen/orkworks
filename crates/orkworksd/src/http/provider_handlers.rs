use crate::http::ErrorResponse;
use crate::providers;
use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub(crate) struct ProviderModelsResponse {
    pub(crate) models: Vec<String>,
}

pub(crate) async fn get_providers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    axum::Json(state.providers.get_providers_response())
}

pub(crate) async fn set_provider_settings(
    State(state): State<Arc<AppState>>,
    axum::Json(payload): axum::Json<providers::ProviderSettingsPayload>,
) -> impl IntoResponse {
    let status = state.providers.apply_settings(payload);
    axum::Json(status)
}

pub(crate) async fn verify_ollama_settings(
    State(state): State<Arc<AppState>>,
    axum::Json(payload): axum::Json<providers::OllamaVerifyRequest>,
) -> impl IntoResponse {
    let normalized = match providers::normalize_ollama_base_url(&payload.base_url) {
        Ok(value) => value,
        Err(error) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(ErrorResponse { error }),
            )
                .into_response();
        }
    };

    let providers = state.providers.clone();
    match tokio::task::spawn_blocking(move || providers.verify_ollama(&normalized)).await {
        Ok(result) => axum::Json(result).into_response(),
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(ErrorResponse {
                error: "internal error".into(),
            }),
        )
            .into_response(),
    }
}

pub(crate) async fn get_provider_models(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> impl IntoResponse {
    let providers = state.providers.clone();
    match tokio::task::spawn_blocking(move || providers.list_models(&provider_id)).await {
        Ok(Ok(models)) => axum::Json(ProviderModelsResponse { models }).into_response(),
        Ok(Err(msg)) => {
            let status = if msg.starts_with("unknown provider") {
                axum::http::StatusCode::NOT_FOUND
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, axum::Json(ErrorResponse { error: msg })).into_response()
        }
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(ErrorResponse {
                error: "internal error".into(),
            }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn get_provider_models_returns_not_found_for_unknown_provider() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = get_provider_models(State(state), Path("unknown-provider".into()))
            .await
            .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn verify_ollama_returns_bad_request_for_invalid_url() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = verify_ollama_settings(
            State(state),
            axum::Json(providers::OllamaVerifyRequest {
                base_url: "http://127.0.0.1:11434/api".into(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }
}
