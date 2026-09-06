use crate::http::ErrorResponse;
use crate::session_application::{
    RecommendationAcceptError, RecommendationDismissError, RecommendationQueryError,
    SessionApplication,
};
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

#[derive(Deserialize)]
pub(crate) struct AcceptRequest {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(default)]
    prompt: Option<String>,
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
    let (recommendations, diagnostics) = match SessionApplication::new(state).list_recommendations()
    {
        Ok(result) => result,
        Err(RecommendationQueryError::Conflict) => return StatusCode::CONFLICT.into_response(),
        Err(RecommendationQueryError::Store(error)) => return store_error(error),
    };
    Json(RecommendationListResponse {
        recommendations,
        diagnostics,
    })
    .into_response()
}

pub(crate) async fn get_recommendation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    match SessionApplication::new(state).get_recommendation(&id) {
        Ok(Some(recommendation)) => Json(recommendation).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(RecommendationQueryError::Conflict) => StatusCode::CONFLICT.into_response(),
        Err(RecommendationQueryError::Store(error)) => store_error(error),
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

pub(crate) async fn accept_recommendation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<AcceptRequest>,
) -> Response {
    match SessionApplication::new(state)
        .accept_recommendation(&id, &request.session_id, request.prompt)
        .await
    {
        Ok(Some(recommendation)) => Json(recommendation).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(RecommendationAcceptError::Conflict) => StatusCode::CONFLICT.into_response(),
        Err(RecommendationAcceptError::SessionNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(RecommendationAcceptError::Store(error)) => store_error(error),
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

    #[tokio::test]
    async fn accept_returns_not_found_for_unknown_recommendation() {
        let dir = tempfile::tempdir().unwrap();
        let state = State(test_app_state_with_workspace(dir.path()));
        let response = accept_recommendation(
            state,
            Path("missing".into()),
            Json(AcceptRequest {
                session_id: "no-session".into(),
                prompt: None,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn accept_returns_not_found_when_target_session_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            for key in ["one", "two"] {
                workspace
                    .workflow_observations
                    .record_observation(
                        "http-accept-session",
                        crate::workflow_observations::ObservationOrigin::Peon,
                        key,
                        crate::workflow_observations::ObservationCandidate {
                            kind: crate::workflow_observations::ObservationKind::Obstacle,
                            description: "The setup blocks progress".into(),
                            evidence: "The same command failed twice".into(),
                            reported_impact: crate::workflow_observations::Impact::Medium,
                            confidence: Some(0.8),
                        },
                    )
                    .unwrap();
            }
        }
        crate::session_application::SessionApplication::new(state.clone())
            .refresh_workflow_recommendations();
        let recommendation_id = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .list()
            .unwrap()
            .pop()
            .unwrap()
            .id;

        let response = accept_recommendation(
            State(state),
            Path(recommendation_id),
            Json(AcceptRequest {
                session_id: "no-such-session".into(),
                prompt: None,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn accept_returns_conflict_for_already_accepted_recommendation() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            for key in ["one", "two"] {
                workspace
                    .workflow_observations
                    .record_observation(
                        "http-accept-conflict-session",
                        crate::workflow_observations::ObservationOrigin::Peon,
                        key,
                        crate::workflow_observations::ObservationCandidate {
                            kind: crate::workflow_observations::ObservationKind::Obstacle,
                            description: "The setup blocks progress".into(),
                            evidence: "The same command failed twice".into(),
                            reported_impact: crate::workflow_observations::Impact::Medium,
                            confidence: Some(0.8),
                        },
                    )
                    .unwrap();
            }
        }
        crate::session_application::SessionApplication::new(state.clone())
            .refresh_workflow_recommendations();
        let recommendation_id = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .list()
            .unwrap()
            .pop()
            .unwrap()
            .id;
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .accept(
                &recommendation_id,
                "already-accepted-target".into(),
                "2026-09-06T00:00:00Z".into(),
            )
            .unwrap();

        let response = accept_recommendation(
            State(state),
            Path(recommendation_id),
            Json(AcceptRequest {
                session_id: "some-other-session".into(),
                prompt: None,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
