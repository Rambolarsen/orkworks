use crate::http::ErrorResponse;
use crate::runtime::terminal_runtime::{record_report_attempt, workflow_report_session_for_token};
use crate::session_application::{
    RecommendationAcceptError, RecommendationCompleteError, RecommendationDismissError,
    RecommendationPacketError, RecommendationQueryError, SessionApplication,
};
use crate::taskmaster::completion::CompletionMutationRequest;
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
use std::collections::BTreeSet;
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
    #[serde(flatten)]
    packet_mutation: Option<CompletionMutationRequest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompleteRequest {
    #[serde(default)]
    summary: Option<String>,
    #[serde(flatten)]
    packet_mutation: Option<CompletionMutationRequest>,
}

const MAX_COMPLETION_SUMMARY_CHARS: usize = 2_000;

fn actionable_recommendations(recommendations: Vec<Recommendation>) -> Vec<Recommendation> {
    let active_member_ids = recommendations
        .iter()
        .filter(|recommendation| {
            !recommendation.rollup_member_ids.is_empty()
                && matches!(
                    recommendation.status,
                    crate::taskmaster::RecommendationStatus::Proposed
                        | crate::taskmaster::RecommendationStatus::Executing
                )
        })
        .flat_map(|recommendation| recommendation.rollup_member_ids.iter().cloned())
        .collect::<BTreeSet<_>>();

    recommendations
        .into_iter()
        .filter(|recommendation| {
            if !recommendation.rollup_member_ids.is_empty() {
                return matches!(
                    recommendation.status,
                    crate::taskmaster::RecommendationStatus::Proposed
                        | crate::taskmaster::RecommendationStatus::Executing
                );
            }
            recommendation.status == crate::taskmaster::RecommendationStatus::Proposed
                && !active_member_ids.contains(&recommendation.id)
        })
        .collect()
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
}

fn store_error(error: StoreError) -> Response {
    if matches!(
        error,
        StoreError::InvalidTransition | StoreError::StalePacket { .. }
    ) {
        return StatusCode::CONFLICT.into_response();
    }
    if matches!(error, StoreError::GraphInvariant(_)) {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}

pub(crate) async fn report_completion_packet(
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
    if !record_report_attempt(&session_id) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let Ok(packet) =
        serde_json::from_slice::<crate::taskmaster::completion::CompletionPacket>(&body)
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match SessionApplication::new(state).report_completion_packet(&id, &session_id, packet) {
        Ok(Some(recommendation)) => Json(recommendation).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(RecommendationPacketError::Conflict) => StatusCode::CONFLICT.into_response(),
        Err(RecommendationPacketError::Store(error)) => store_error(error),
    }
}

pub(crate) async fn list_recommendations(State(state): State<Arc<AppState>>) -> Response {
    let (recommendations, diagnostics) = match SessionApplication::new(state).list_recommendations()
    {
        Ok(result) => result,
        Err(RecommendationQueryError::Conflict) => return StatusCode::CONFLICT.into_response(),
        Err(RecommendationQueryError::Store(error)) => return store_error(error),
    };
    Json(RecommendationListResponse {
        recommendations: actionable_recommendations(recommendations),
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
    if request
        .packet_mutation
        .as_ref()
        .is_some_and(|mutation| mutation.validate().is_err())
    {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }
    match SessionApplication::new(state)
        .accept_recommendation_with_packet(
            &id,
            &request.session_id,
            request.prompt,
            request.packet_mutation,
        )
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
    if !record_report_attempt(&session_id) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let (summary, packet_mutation) = if body.is_empty() {
        (None, None)
    } else {
        let Ok(request) = serde_json::from_slice::<CompleteRequest>(&body) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        (request.summary, request.packet_mutation)
    };
    if packet_mutation
        .as_ref()
        .is_some_and(|mutation| mutation.validate().is_err())
    {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }
    if summary
        .as_deref()
        .is_some_and(|value| value.chars().count() > MAX_COMPLETION_SUMMARY_CHARS)
    {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }

    match SessionApplication::new(state).complete_recommendation_with_packet(
        &id,
        &session_id,
        summary,
        packet_mutation,
    ) {
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
        clear_workflow_report_token, set_workflow_report_token, WORKFLOW_REPORT_RATE_LIMIT,
    };
    use crate::taskmaster::{
        RecommendationConfidence, RecommendationStatus, RecommendationType, TargetSurface,
        WorkflowImprovement, WorkflowObservationEvidence,
    };
    use crate::test_support::test_app_state_with_workspace;
    use crate::workflow_observations::{Impact, ObservationKind, ObservationSource};
    use axum::body::Bytes;
    use axum::http::header::AUTHORIZATION;

    fn recommendation_fixture(
        id: &str,
        status: RecommendationStatus,
        session_id: &str,
    ) -> Recommendation {
        Recommendation {
            id: id.into(),
            workspace_id: "workspace-1".into(),
            chain_id: id.into(),
            chain_depth: 0,
            recommendation_type: RecommendationType::ImproveWorkflow,
            status,
            priority: Impact::Medium,
            title: format!("Recommendation {id}"),
            summary: "A bounded workflow improvement".into(),
            reason: vec!["The evidence supports this change.".into()],
            evidence: vec![WorkflowObservationEvidence {
                observation_id: format!("observation-{id}"),
                sequence: 1,
                session_id: session_id.into(),
                kind: ObservationKind::Obstacle,
                description: format!("Evidence for {id}"),
                evidence: "The workflow failed at this point.".into(),
                problem_area: Some("model detection".into()),
                reported_impact: Impact::Medium,
                source: ObservationSource::Agent,
                confidence: 0.8,
                observed_at: "2026-09-13T10:00:00Z".into(),
            }],
            repository_evidence: Vec::new(),
            knowledge_evidence: Vec::new(),
            source_session_ids: vec![session_id.into()],
            target_session_id: None,
            suggested_harness_id: None,
            suggested_model: None,
            suggested_working_directory: None,
            suggested_prompt: None,
            confidence: RecommendationConfidence::Medium,
            requires_approval: false,
            dedupe_key: format!("dedupe-{id}"),
            created_at: "2026-09-13T10:00:00Z".into(),
            updated_at: "2026-09-13T10:00:00Z".into(),
            expires_at: None,
            workflow_improvement: WorkflowImprovement {
                proposed_improvement: "Improve the workflow".into(),
                target_surface: TargetSurface::Tooling,
                observation_ids: vec![format!("observation-{id}")],
                recurrence_count: 1,
                affected_session_ids: vec![session_id.into()],
                impact: Impact::Medium,
                expected_benefit: "Fewer repeated failures".into(),
                supersedes_recommendation_id: None,
                dismissal_watermark: None,
            },
            completion_packet: None,
            rollup_member_ids: Vec::new(),
            rollup_member_dedupe_keys: Vec::new(),
            rollup_generation: None,
            rolled_up_by: None,
        }
    }

    fn persist_rollup_fixture(state: &std::sync::Arc<crate::AppState>) -> (String, String) {
        let member_id = "rollup-member".to_string();
        let parent_id = crate::taskmaster::rollup::stable_rollup_id(&[member_id.clone()]);
        let mut parent =
            recommendation_fixture(&parent_id, RecommendationStatus::Proposed, "session-parent");
        parent.rollup_member_ids = vec![member_id.clone()];
        parent.rollup_member_dedupe_keys = vec!["dedupe-rollup-member".into()];
        parent.rollup_generation = Some(7);
        let mut member =
            recommendation_fixture(&member_id, RecommendationStatus::RolledUp, "session-member");
        member.rolled_up_by = Some(parent_id.clone());

        let workspace = state.workspace.lock().unwrap();
        let store = &workspace.as_ref().unwrap().recommendation_store;
        store.put(&parent).unwrap();
        store.put(&member).unwrap();
        store
            .put(&recommendation_fixture(
                "standalone",
                RecommendationStatus::Proposed,
                "session-standalone",
            ))
            .unwrap();
        store
            .put(&recommendation_fixture(
                "dismissed",
                RecommendationStatus::Dismissed,
                "session-dismissed",
            ))
            .unwrap();
        (parent_id, member_id)
    }

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
                            problem_area: None,
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
    async fn completion_packet_report_requires_a_valid_token_and_payload() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        assert_eq!(
            report_completion_packet(
                State(state.clone()),
                Path("missing".into()),
                HeaderMap::new(),
                Bytes::new(),
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );

        set_workflow_report_token("packet-source", "packet-token".into());
        let response = report_completion_packet(
            State(state),
            Path("missing".into()),
            authorization("packet-token"),
            Bytes::from_static(br#"{"notACompletionPacket":true}"#),
        )
        .await;
        clear_workflow_report_token("packet-source");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn completion_packet_report_binds_packet_to_the_source_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let recommendation = recommendation_fixture(
            "packet-report",
            RecommendationStatus::Proposed,
            "packet-source",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .put(&recommendation)
            .unwrap();

        set_workflow_report_token("other-source", "other-token".into());
        let mut packet = crate::taskmaster::completion_tests::test_packet();
        packet.source_session_id = "packet-source".into();
        packet.provenance.source_session_id = "packet-source".into();
        packet.evidence_fingerprint = packet.computed_evidence_fingerprint();
        let response = report_completion_packet(
            State(state.clone()),
            Path("packet-report".into()),
            authorization("other-token"),
            Bytes::from(serde_json::to_vec(&packet).unwrap()),
        )
        .await;
        clear_workflow_report_token("other-source");
        assert_eq!(response.status(), StatusCode::CONFLICT);

        set_workflow_report_token("packet-source", "packet-token".into());
        let response = report_completion_packet(
            State(state.clone()),
            Path("packet-report".into()),
            authorization("packet-token"),
            Bytes::from(serde_json::to_vec(&packet).unwrap()),
        )
        .await;
        clear_workflow_report_token("packet-source");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .get("packet-report")
            .unwrap()
            .unwrap()
            .completion_packet
            .is_some());
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
    async fn complete_applies_the_authenticated_report_rate_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let recommendation_id = accepted_recommendation(
            &state,
            "rate-limited-session",
            "rate-limited-session",
            "rate-limit-token",
        );

        for _ in 0..WORKFLOW_REPORT_RATE_LIMIT {
            let response = complete_recommendation(
                State(state.clone()),
                Path(recommendation_id.clone()),
                authorization("rate-limit-token"),
                Bytes::new(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
        }

        let rejected = complete_recommendation(
            State(state),
            Path(recommendation_id),
            authorization("rate-limit-token"),
            Bytes::new(),
        )
        .await;
        clear_workflow_report_token("rate-limited-session");
        assert_eq!(rejected.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn list_returns_empty_recommendations_for_a_new_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let response = list_recommendations(State(test_app_state_with_workspace(dir.path()))).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_returns_active_parents_and_unparented_proposals_only() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (parent_id, member_id) = persist_rollup_fixture(&state);

        let response = list_recommendations(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let ids = body["recommendations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(ids.contains(&parent_id.as_str()));
        assert!(ids.contains(&"standalone"));
        assert!(!ids.contains(&member_id.as_str()));
        assert!(!ids.contains(&"dismissed"));
    }

    #[test]
    fn list_projection_keeps_executing_rollup_parents_actionable_for_audit() {
        let mut parent = recommendation_fixture(
            "executing-parent",
            RecommendationStatus::Executing,
            "session-parent",
        );
        parent.rollup_member_ids = vec!["executing-member".into()];
        parent.rollup_member_dedupe_keys = vec!["dedupe-executing-member".into()];
        let mut member = recommendation_fixture(
            "executing-member",
            RecommendationStatus::RolledUp,
            "session-member",
        );
        member.rolled_up_by = Some(parent.id.clone());

        let projected = actionable_recommendations(vec![member, parent]);
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].id, "executing-parent");
    }

    #[tokio::test]
    async fn detail_exposes_rollup_relationship_and_problem_area_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (parent_id, member_id) = persist_rollup_fixture(&state);

        let parent = get_recommendation(State(state.clone()), Path(parent_id.clone())).await;
        assert_eq!(parent.status(), StatusCode::OK);
        let body = axum::body::to_bytes(parent.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["rollupMemberIds"], serde_json::json!([member_id]));
        assert_eq!(
            body["rollupMemberDedupeKeys"],
            serde_json::json!(["dedupe-rollup-member"])
        );
        assert_eq!(body["rollupGeneration"], 7);
        assert!(body["rolledUpBy"].is_null());
        assert_eq!(body["evidence"][0]["problemArea"], "model detection");

        let member = get_recommendation(State(state), Path("rollup-member".into())).await;
        assert_eq!(member.status(), StatusCode::OK);
        let body = axum::body::to_bytes(member.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["status"], "rolled_up");
        assert_eq!(body["rolledUpBy"], parent_id);
    }

    #[tokio::test]
    async fn actions_against_rolled_up_members_conflict_without_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (_, member_id) = persist_rollup_fixture(&state);
        let before = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .get(&member_id)
            .unwrap()
            .unwrap();

        let dismissed =
            dismiss_recommendation(State(state.clone()), Path(member_id.clone()), None).await;
        assert_eq!(dismissed.status(), StatusCode::CONFLICT);

        let accepted = accept_recommendation(
            State(state.clone()),
            Path(member_id.clone()),
            Json(AcceptRequest {
                session_id: "unrelated-session".into(),
                prompt: Some("attempted mutation".into()),
                packet_mutation: None,
            }),
        )
        .await;
        assert_eq!(accepted.status(), StatusCode::CONFLICT);

        let after = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .get(&member_id)
            .unwrap()
            .unwrap();
        assert_eq!(after, before);
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
                packet_mutation: None,
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
                            problem_area: None,
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
                packet_mutation: None,
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
                            problem_area: None,
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
                packet_mutation: None,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
