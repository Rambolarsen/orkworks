use crate::http::ErrorResponse;
use crate::providers;
use crate::AppState;
use axum::extract::{rejection::JsonRejection, Path, State};
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

fn provider_error_response(error: providers::ProviderOperationError) -> axum::response::Response {
    let status = match error.code {
        providers::ProviderOperationErrorCode::Malformed
        | providers::ProviderOperationErrorCode::UnknownProvider => {
            axum::http::StatusCode::BAD_REQUEST
        }
        providers::ProviderOperationErrorCode::Unauthorized => axum::http::StatusCode::UNAUTHORIZED,
        providers::ProviderOperationErrorCode::Timeout => axum::http::StatusCode::GATEWAY_TIMEOUT,
        providers::ProviderOperationErrorCode::StaleGeneration => axum::http::StatusCode::CONFLICT,
        providers::ProviderOperationErrorCode::UnsupportedCapability => {
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        }
        providers::ProviderOperationErrorCode::ProviderFailure
        | providers::ProviderOperationErrorCode::ModelFailure => {
            axum::http::StatusCode::BAD_GATEWAY
        }
    };
    (status, axum::Json(providers::ProviderOperationErrorResponse { error })).into_response()
}

pub(crate) async fn verify_peon_provider(
    State(state): State<Arc<AppState>>,
    payload: Result<axum::Json<providers::PeonProviderVerifyRequest>, JsonRejection>,
) -> axum::response::Response {
    let payload = match payload {
        Ok(axum::Json(payload)) => payload,
        Err(rejection) => {
            return provider_error_response(providers::ProviderOperationError {
                code: providers::ProviderOperationErrorCode::Malformed,
                message: rejection.body_text(),
            });
        }
    };
    let provider_manager = state.providers.clone();
    match tokio::task::spawn_blocking(move || provider_manager.verify_provider(payload)).await {
        Ok(Ok(response)) => axum::Json(response).into_response(),
        Ok(Err(error)) => provider_error_response(error),
        Err(_) => provider_error_response(providers::ProviderOperationError {
            code: providers::ProviderOperationErrorCode::ProviderFailure,
            message: "provider verification task failed".into(),
        }),
    }
}

pub(crate) async fn test_and_apply_peon_provider(
    State(state): State<Arc<AppState>>,
    payload: Result<axum::Json<providers::PeonTestAndApplyRequest>, JsonRejection>,
) -> axum::response::Response {
    let payload = match payload {
        Ok(axum::Json(payload)) => payload,
        Err(rejection) => {
            return provider_error_response(providers::ProviderOperationError {
                code: providers::ProviderOperationErrorCode::Malformed,
                message: rejection.body_text(),
            });
        }
    };
    let provider_manager = state.providers.clone();
    match tokio::task::spawn_blocking(move || provider_manager.test_and_apply(payload)).await {
        Ok(Ok(response)) => axum::Json(response).into_response(),
        Ok(Err(error)) => provider_error_response(error),
        Err(_) => provider_error_response(providers::ProviderOperationError {
            code: providers::ProviderOperationErrorCode::ProviderFailure,
            message: "provider Apply task failed".into(),
        }),
    }
}

pub(crate) async fn get_applied_peon_provider(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    axum::Json(state.providers.get_applied())
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

    #[test]
    fn staged_provider_errors_have_structured_code_and_message() {
        let response = provider_error_response(providers::ProviderOperationError {
            code: providers::ProviderOperationErrorCode::Timeout,
            message: "provider request timed out".into(),
        });

        assert_eq!(response.status(), axum::http::StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .unwrap(),
            "application/json"
        );
    }
}
