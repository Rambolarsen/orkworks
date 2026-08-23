use crate::http::ErrorResponse;
use crate::session_application::{RecommendationDismissError, SessionApplication};
use crate::taskmaster::store::StoreError;
use crate::taskmaster::Recommendation;
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecommendationListResponse {
    recommendations: Vec<Recommendation>,
    diagnostics: Vec<crate::workflow_observations::ObservationDiagnostic>,
}

#[derive(Deserialize, Default)]
pub(crate) struct DismissRequest {
    reason: Option<String>,
}

fn store_error(error: StoreError) -> Response {
    if matches!(error, StoreError::InvalidTransition) {
        return StatusCode::CONFLICT.into_response();
    }
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}

pub(crate) async fn list_recommendations(State(state): State<Arc<AppState>>) -> Response {
    let workspace = state.workspace.lock().unwrap();
    let Some(workspace) = workspace.as_ref() else {
        return StatusCode::CONFLICT.into_response();
    };
    let recommendations = match workspace.recommendation_store.list() {
        Ok(recommendations) => recommendations,
        Err(error) => return store_error(error),
    };
    let diagnostics = workspace.workflow_observations.diagnostics();
    Json(RecommendationListResponse { recommendations, diagnostics }).into_response()
}

pub(crate) async fn get_recommendation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let workspace = state.workspace.lock().unwrap();
    let Some(workspace) = workspace.as_ref() else {
        return StatusCode::CONFLICT.into_response();
    };
    match workspace.recommendation_store.get(&id) {
        Ok(Some(recommendation)) => Json(recommendation).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => store_error(error),
    }
}

pub(crate) async fn dismiss_recommendation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Option<Json<DismissRequest>>,
) -> Response {
    if body
        .and_then(|Json(request)| request.reason)
        .is_some_and(|reason| reason.chars().count() > 500)
    {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }
    match SessionApplication::new(state).dismiss_recommendation(&id) {
        Ok(Some(recommendation)) => Json(recommendation).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(RecommendationDismissError::Conflict) => StatusCode::CONFLICT.into_response(),
        Err(RecommendationDismissError::Store(error)) => store_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_app_state_with_workspace;
    #[tokio::test]
    async fn list_returns_empty_recommendations_for_a_new_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let response = list_recommendations(State(test_app_state_with_workspace(dir.path()))).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_and_dismiss_return_not_found_for_unknown_recommendations() {
        let dir = tempfile::tempdir().unwrap();
        let state = State(test_app_state_with_workspace(dir.path()));
        assert_eq!(
            get_recommendation(state.clone(), Path("missing".into()))
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            dismiss_recommendation(state, Path("missing".into()), None)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
}
