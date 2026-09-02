use crate::http::ErrorResponse;
use crate::providers;
use crate::AppState;
use axum::extract::{rejection::JsonRejection, State};
use axum::response::IntoResponse;
use std::sync::Arc;

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

pub(crate) async fn discover_provider_models(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(provider_id): axum::extract::Path<String>,
    payload: Result<axum::Json<providers::OllamaVerifyRequest>, JsonRejection>,
) -> axum::response::Response {
    let base_url = match payload {
        Ok(axum::Json(payload)) => Some(payload.base_url),
        Err(_) => None,
    };
    let provider_manager = state.providers.clone();
    match tokio::task::spawn_blocking(move || {
        provider_manager.discover_provider_models(&provider_id, base_url.as_deref())
    })
    .await
    {
        Ok(Ok(models)) => axum::Json(serde_json::json!({ "models": models })).into_response(),
        Ok(Err(error)) => provider_error_response(error),
        Err(_) => provider_error_response(providers::ProviderOperationError {
            code: providers::ProviderOperationErrorCode::ProviderFailure,
            message: "model discovery task failed".into(),
        }),
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
        providers::ProviderOperationErrorCode::StaleGeneration
        | providers::ProviderOperationErrorCode::VerificationRequired => {
            axum::http::StatusCode::CONFLICT
        }
        providers::ProviderOperationErrorCode::UnsupportedCapability => {
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        }
        providers::ProviderOperationErrorCode::ProviderFailure
        | providers::ProviderOperationErrorCode::ModelFailure => {
            axum::http::StatusCode::BAD_GATEWAY
        }
    };
    (
        status,
        axum::Json(providers::ProviderOperationErrorResponse { error }),
    )
        .into_response()
}

/// Stale-generation rejections tell the desktop client which generation the
/// sidecar is actually on so it can resync its local counter and retry
/// instead of staying permanently locked out.
fn stale_generation_response(
    error: providers::ProviderOperationError,
    current_generation: u64,
) -> axum::response::Response {
    let body = serde_json::json!({
        "error": error,
        "currentGeneration": current_generation,
    });
    (axum::http::StatusCode::CONFLICT, axum::Json(body)).into_response()
}

fn provider_operation_error_response(
    error: providers::ProviderOperationError,
    current_generation: u64,
) -> axum::response::Response {
    if error.code == providers::ProviderOperationErrorCode::StaleGeneration {
        stale_generation_response(error, current_generation)
    } else {
        provider_error_response(error)
    }
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
        Ok(Err(error)) => {
            provider_operation_error_response(error, state.providers.latest_generation())
        }
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
        Ok(Err(error)) => {
            provider_operation_error_response(error, state.providers.latest_generation())
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use axum::response::IntoResponse;

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

    #[tokio::test]
    async fn stale_verify_response_includes_current_generation() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let request = |generation: u64| {
            axum::Json(providers::PeonProviderVerifyRequest {
                provider: "opencode".into(),
                generation,
                ollama_base_url: None,
            })
        };
        verify_peon_provider(State(state.clone()), Ok(request(5)))
            .await
            .into_response();
        let stale = verify_peon_provider(State(state.clone()), Ok(request(4)))
            .await
            .into_response();

        assert_eq!(stale.status(), axum::http::StatusCode::CONFLICT);
        let bytes = axum::body::to_bytes(stale.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["code"], serde_json::json!("stale_generation"));
        assert_eq!(body["currentGeneration"], serde_json::json!(5));
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
