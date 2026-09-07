use crate::http::ErrorResponse;
use crate::runtime::terminal_runtime::workflow_report_session_for_token;
use crate::session_application::{
    RecommendationAcceptError, RecommendationCompleteError, RecommendationDismissError,
    RecommendationQueryError, SessionApplication,
};
use crate::taskmaster::store::StoreError;
use crate::taskmaster::Recommendation;
use crate::AppState;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompleteRequest {
    #[serde(default)]
    summary: Option<String>,
}

const MAX_COMPLETION_SUMMARY_CHARS: usize = 2_000;

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
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

pub(crate) async fn complete_recommendation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(token) = bearer_token(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(session_id) = workflow_report_session_for_token(token) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let summary = if body.is_empty() {
        None
    } else {
        let Ok(request) = serde_json::from_slice::<CompleteRequest>(&body) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        request.summary
    };
    if summary
        .as_deref()
        .is_some_and(|value| value.chars().count() > MAX_COMPLETION_SUMMARY_CHARS)
    {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }

    match SessionApplication::new(state).complete_recommendation(&id, &session_id, summary) {
        Ok(Some(recommendation)) => Json(recommendation).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(RecommendationCompleteError::Conflict) => StatusCode::CONFLICT.into_response(),
        Err(RecommendationCompleteError::Store(error)) => store_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::terminal_runtime::{
        clear_workflow_report_token, set_workflow_report_token,
    };
    use crate::test_support::test_app_state_with_workspace;
    use axum::body::Bytes;
    use axum::http::header::AUTHORIZATION;

    fn accepted_recommendation(
        state: &std::sync::Arc<crate::AppState>,
        target_session_id: &str,
        token_session_id: &str,
        token: &str,
    ) -> String {
        {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            for key in ["complete-one", "complete-two"] {
                workspace
                    .workflow_observations
                    .record_observation(
                        target_session_id,
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
        let workspace = state.workspace.lock().unwrap();
        let store = &workspace.as_ref().unwrap().recommendation_store;
        store
            .begin_execution(
                &recommendation_id,
                target_session_id.into(),
                "2026-09-07T10:00:00Z".into(),
            )
            .unwrap();
        store
            .complete_execution(&recommendation_id, "2026-09-07T10:00:01Z".into())
            .unwrap();
        set_workflow_report_token(token_session_id, token.into());
        recommendation_id
    }

    fn authorization(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        headers
    }

    #[tokio::test]
    async fn complete_requires_a_valid_reporting_token() {
        let dir = tempfile::tempdir().unwrap();
        let state = State(test_app_state_with_workspace(dir.path()));

        assert_eq!(
            complete_recommendation(
                state.clone(),
                Path("missing".into()),
                HeaderMap::new(),
                Bytes::new(),
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            complete_recommendation(
                state,
                Path("missing".into()),
                authorization("wrong-token"),
                Bytes::new(),
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn complete_rejects_an_oversized_summary_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        set_workflow_report_token("summary-limit-session", "summary-limit-token".into());

        let response = complete_recommendation(
            State(state),
            Path("missing".into()),
            authorization("summary-limit-token"),
            Bytes::from(format!(
                "{{\"summary\":\"{}\"}}",
                "x".repeat(MAX_COMPLETION_SUMMARY_CHARS + 1)
            )),
        )
        .await;
        clear_workflow_report_token("summary-limit-session");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn complete_rejects_malformed_body_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let recommendation_id = accepted_recommendation(
            &state,
            "malformed-body-session",
            "malformed-body-session",
            "malformed-body-token",
        );

        let response = complete_recommendation(
            State(state.clone()),
            Path(recommendation_id.clone()),
            authorization("malformed-body-token"),
            Bytes::from_static(br#"{"unknown":true}"#),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .recommendation_store
                .get(&recommendation_id)
                .unwrap()
                .unwrap()
                .status,
            crate::taskmaster::RecommendationStatus::Accepted
        );
        clear_workflow_report_token("malformed-body-session");
    }

    #[tokio::test]
    async fn complete_uses_the_token_session_and_rejects_a_different_target() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let recommendation_id = accepted_recommendation(
            &state,
            "target-session",
            "different-session",
            "complete-token",
        );

        let response = complete_recommendation(
            State(state),
            Path(recommendation_id),
            authorization("complete-token"),
            Bytes::new(),
        )
        .await;
        clear_workflow_report_token("different-session");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn complete_returns_completed_recommendation_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let recommendation_id = accepted_recommendation(
            &state,
            "complete-session",
            "complete-session",
            "complete-token-unique",
        );

        let first = complete_recommendation(
            State(state.clone()),
            Path(recommendation_id.clone()),
            authorization("complete-token-unique"),
            Bytes::from_static(br#"{"summary":"Verified by the agent."}"#),
        )
        .await;
        assert_eq!(first.status(), StatusCode::OK);

        let second = complete_recommendation(
            State(state.clone()),
            Path(recommendation_id.clone()),
            authorization("complete-token-unique"),
            Bytes::new(),
        )
        .await;
        assert_eq!(second.status(), StatusCode::OK);

        let events = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events("complete-session");
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "taskmaster_fix_completed")
                .count(),
            1
        );
        clear_workflow_report_token("complete-session");
    }
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
            .begin_execution(
                &recommendation_id,
                "already-accepted-target".into(),
                "2026-09-06T00:00:00Z".into(),
            )
            .unwrap();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .complete_execution(&recommendation_id, "2026-09-06T00:00:01Z".into())
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
