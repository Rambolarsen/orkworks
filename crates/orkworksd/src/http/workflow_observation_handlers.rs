//! Capability-authenticated explicit workflow-observation reporting.
//!
//! This is the HTTP adapter described by ADR 0042 and
//! `docs/superpowers/specs/2026-08-14-workflow-observation-feedback-loop-design.md`'s
//! "Explicit agent reporting" section: it lets a coding agent inside a live
//! session call `POST /sessions/:id/workflow-observations` and have the
//! report land in Task 3's `workflow_observations::WorkflowObservationStore`
//! as an `ObservationOrigin::Agent` record.
//!
//! This module is deliberately thin. It owns exactly:
//! - capability authentication (`Authorization: Bearer <ORKWORKS_REPORT_TOKEN>`),
//!   via `runtime::terminal_runtime`'s process-local capability registry;
//! - the HTTP-layer, pre-persistence 30-attempts/60-seconds rate limit,
//!   which is distinct from `workflow_observations`' own post-persistence
//!   60-accepted/minute cap;
//! - `Idempotency-Key` header validation and the fixed request vocabulary
//!   (`kind`, `description`, `evidence`, `reportedImpact` only, via
//!   `#[serde(deny_unknown_fields)]`); and
//! - mapping the validated request onto an Agent observation candidate and
//!   handing it to `SessionApplication` for workspace-scoped persistence.
//!
//! Everything else -- confidence policy, fingerprinting, deduplication,
//! bounded persistence -- stays inside `workflow_observations`, which this
//! module calls but never modifies.

use crate::runtime::terminal_runtime::{record_report_attempt, verify_workflow_report_token};
use crate::session_application::{SessionApplication, WorkflowObservationPersistenceError};
use crate::taskmaster::evaluator::schedule_evaluation;
use crate::session_types::MemoryState;
use crate::workflow_observations::{
    self, ObservationCandidate, RecordError, RecordOutcome,
};
use crate::AppState;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// `Idempotency-Key` must be 1-128 visible ASCII characters (design doc,
/// "Explicit agent reporting").
const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;

/// The only fields a caller may set. Everything else the persisted
/// `WorkflowObservation` carries (workspace, source, confidence,
/// fingerprint, sequence, id, observed time) is server-derived --
/// `deny_unknown_fields` rejects any attempt to smuggle one of those in.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowObservationReport {
    kind: workflow_observations::ObservationKind,
    description: String,
    evidence: String,
    #[serde(rename = "reportedImpact")]
    reported_impact: workflow_observations::Impact,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowObservationReportResponse {
    observation_id: String,
    sequence: u64,
    accepted_at: String,
    duplicate: bool,
}

pub(crate) async fn report_workflow_observation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // 1. Capability authentication, before anything else. A missing or
    // malformed Authorization header, an unknown session id, and a known
    // session id with the wrong token all take this exact same branch and
    // return the exact same response, so nothing here can be used to learn
    // whether `id` names a live session.
    let Some(token) = bearer_token(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let session_is_live = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&id)
            .is_some_and(|handle| handle.info.memory_state == MemoryState::Live)
    };
    if !session_is_live || !verify_workflow_report_token(&id, token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // 2. HTTP-layer, pre-persistence rate limit: at most 30 authenticated
    // attempts per session in a rolling 60-second window. This is separate
    // from workflow_observations' own 60-accepted-per-minute cap enforced
    // after persistence in step 5 below.
    if !record_report_attempt(&id) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    // 3. Idempotency-Key: required, 1-128 visible ASCII characters.
    let Some(idempotency_key) = idempotency_key_header(&headers) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    // 4. Fixed request vocabulary only. `body` was already bounded to 8 KiB
    // by this route's `DefaultBodyLimit` layer before we get here.
    let Ok(report) = serde_json::from_slice::<WorkflowObservationReport>(&body) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let candidate = ObservationCandidate {
        kind: report.kind,
        description: report.description,
        evidence: report.evidence,
        reported_impact: report.reported_impact,
        // Ignored for ObservationOrigin::Agent: the module enforces the
        // fixed 0.9 confidence policy regardless of what is set here.
        confidence: None,
    };

    // 5. Persist through Task 3's module. The workspace mutex is locked and
    // dropped entirely inside this blocking closure -- it is never held
    // across the `.await` on the JoinHandle below.
    let blocking_state = state.clone();
    let blocking_id = id.clone();
    let join_result = tokio::task::spawn_blocking(move || {
        SessionApplication::new(blocking_state).record_agent_workflow_observation(
            &blocking_id,
            &idempotency_key,
            candidate,
        )
    })
    .await;

    let outcome = match join_result {
        Ok(outcome) => outcome,
        Err(_join_error) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if matches!(&outcome, Ok(RecordOutcome::Accepted(_))) {
        schedule_evaluation(state.clone());
    }

    match outcome {
        Ok(RecordOutcome::Accepted(observation)) => Json(WorkflowObservationReportResponse {
            observation_id: observation.id,
            sequence: observation.sequence,
            accepted_at: observation.observed_at,
            duplicate: false,
        })
        .into_response(),
        Ok(RecordOutcome::Duplicate {
            observation_id,
            sequence,
            accepted_at,
        }) => Json(WorkflowObservationReportResponse {
            observation_id,
            sequence,
            accepted_at,
            duplicate: true,
        })
        .into_response(),
        Err(WorkflowObservationPersistenceError::NoWorkspace) => {
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
        Err(WorkflowObservationPersistenceError::SessionNotInWorkspace) => {
            StatusCode::UNAUTHORIZED.into_response()
        }
        Err(WorkflowObservationPersistenceError::Record(RecordError::IdempotencyConflict)) => {
            StatusCode::CONFLICT.into_response()
        }
        Err(WorkflowObservationPersistenceError::Record(RecordError::RateLimited)) => {
            StatusCode::TOO_MANY_REQUESTS.into_response()
        }
        Err(WorkflowObservationPersistenceError::Record(
            RecordError::EmptySessionId
            | RecordError::EmptyIdempotencyKey
            | RecordError::EmptyDescription
            | RecordError::DescriptionTooLong
            | RecordError::EmptyEvidence
            | RecordError::EvidenceTooLong,
        )) => StatusCode::BAD_REQUEST.into_response(),
        // Unreachable via this adapter: ObservationOrigin::Agent never
        // consults the candidate's confidence, so the module can never
        // reject it as missing/out-of-range here. Matched exhaustively
        // rather than a wildcard so a future RecordError variant fails to
        // compile instead of silently falling through.
        Err(WorkflowObservationPersistenceError::Record(
            RecordError::MissingConfidence | RecordError::ConfidenceOutOfRange,
        )) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        Err(WorkflowObservationPersistenceError::Record(
            RecordError::Degraded | RecordError::PersistFailed,
        )) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    value.strip_prefix("Bearer ").filter(|t| !t.is_empty())
}

/// Validates and returns the `Idempotency-Key` header value: required,
/// 1-128 visible ASCII characters (`0x21..=0x7E`, i.e. printable and
/// non-whitespace).
fn idempotency_key_header(headers: &HeaderMap) -> Option<String> {
    let value = headers.get("Idempotency-Key")?.to_str().ok()?;
    if value.is_empty() || value.len() > MAX_IDEMPOTENCY_KEY_LEN {
        return None;
    }
    if !value.bytes().all(|b| (0x21..=0x7E).contains(&b)) {
        return None;
    }
    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::session_runtime::SessionRuntime;
    use crate::runtime::terminal_runtime::{set_workflow_report_token, WORKFLOW_REPORT_RATE_LIMIT};
    use crate::session_types::MemoryState;
    use crate::test_support::{test_app_state_with_workspace, test_session_info, test_session_metadata};
    use axum::http::header::AUTHORIZATION;
    use axum::http::HeaderValue;
    use serde_json::json;

    /// Every capability lives in `terminal_runtime`'s process-wide registry
    /// (see its module doc), so a session id must be unique per test even
    /// though each test otherwise gets its own isolated `AppState`.
    fn unique_session_id() -> String {
        format!("workflow-report-{}", uuid::Uuid::new_v4())
    }

    fn insert_live_session(state: &Arc<AppState>, id: &str) {
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            id.to_string(),
            crate::SessionHandle {
                info: test_session_info(id, "Test Session", "/tmp", "running", "now"),
                active_work_hook: false,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: SessionRuntime::detached_test(),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            },
        );
    }

    fn insert_remembered_session(state: &Arc<AppState>, id: &str) {
        insert_live_session(state, id);
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut(id)
            .unwrap()
            .info
            .memory_state = MemoryState::Remembered;
    }

    fn insert_live_session_with_workspace_metadata(state: &Arc<AppState>, id: &str) {
        insert_live_session(state, id);
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        ws.metadata.write_session(&test_session_metadata(
            id,
            id,
            ws.path.display().to_string(),
            "running",
            "before",
            "before",
        ));
    }

    fn headers_with(auth: Option<&str>, idempotency_key: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(token) = auth {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
            );
        }
        if let Some(key) = idempotency_key {
            headers.insert("idempotency-key", HeaderValue::from_str(key).unwrap());
        }
        headers
    }

    fn valid_body() -> Bytes {
        Bytes::from(
            json!({
                "kind": "repetition",
                "description": "Re-ran the same failing command three times",
                "evidence": "cargo test foo failed identically at 10:01, 10:03, 10:05",
                "reportedImpact": "medium",
            })
            .to_string(),
        )
    }

    async fn status_and_json(
        response: axum::response::Response,
    ) -> (StatusCode, serde_json::Value) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
        };
        (status, json)
    }

    #[tokio::test]
    async fn unknown_session_id_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = report_workflow_observation(
            State(state),
            Path("no-such-session".to_string()),
            headers_with(Some("anything"), Some("key-1")),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn known_but_not_live_session_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_remembered_session(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), Some("key-1")),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn missing_authorization_header_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session_with_workspace_metadata(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(None, Some("key-1")),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn wrong_bearer_token_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session(&state, &id);
        set_workflow_report_token(&id, "the-real-token".to_string());

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("wrong-token"), Some("key-1")),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn no_workspace_returns_service_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session_with_workspace_metadata(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());
        *state.workspace.lock().unwrap() = None;

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), Some("key-1")),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn correct_bearer_token_accepts_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session_with_workspace_metadata(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let response = report_workflow_observation(
            State(state.clone()),
            Path(id.clone()),
            headers_with(Some("the-token"), Some("key-1")),
            valid_body(),
        )
        .await
        .into_response();

        let (status, body) = status_and_json(response).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["duplicate"], false);
        assert!(body["observationId"].as_str().is_some());
        assert!(body["acceptedAt"].as_str().is_some());

        let workspace = state.workspace.lock().unwrap();
        let observations = workspace
            .as_ref()
            .unwrap()
            .workflow_observations
            .workspace_observations()
            .unwrap();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].session_id, id);
    }

    #[tokio::test]
    async fn duplicate_report_with_same_key_and_payload_returns_duplicate_true() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session_with_workspace_metadata(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let first = report_workflow_observation(
            State(state.clone()),
            Path(id.clone()),
            headers_with(Some("the-token"), Some("same-key")),
            valid_body(),
        )
        .await
        .into_response();
        let (first_status, first_body) = status_and_json(first).await;
        assert_eq!(first_status, StatusCode::OK);

        let second = report_workflow_observation(
            State(state.clone()),
            Path(id),
            headers_with(Some("the-token"), Some("same-key")),
            valid_body(),
        )
        .await
        .into_response();
        let (second_status, second_body) = status_and_json(second).await;

        assert_eq!(second_status, StatusCode::OK);
        assert_eq!(second_body["duplicate"], true);
        assert_eq!(second_body["observationId"], first_body["observationId"]);
    }

    #[tokio::test]
    async fn same_key_different_payload_returns_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session_with_workspace_metadata(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let first = report_workflow_observation(
            State(state.clone()),
            Path(id.clone()),
            headers_with(Some("the-token"), Some("same-key")),
            valid_body(),
        )
        .await
        .into_response();
        assert_eq!(first.status(), StatusCode::OK);

        let different_body = Bytes::from(
            json!({
                "kind": "obstacle",
                "description": "A totally different description of a different problem",
                "evidence": "Different evidence entirely",
                "reportedImpact": "high",
            })
            .to_string(),
        );
        let second = report_workflow_observation(
            State(state.clone()),
            Path(id),
            headers_with(Some("the-token"), Some("same-key")),
            different_body,
        )
        .await
        .into_response();

        assert_eq!(second.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn missing_idempotency_key_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), None),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn empty_idempotency_key_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), Some("")),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn idempotency_key_over_128_bytes_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());
        let too_long = "a".repeat(129);

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), Some(&too_long)),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn idempotency_key_exactly_128_bytes_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session_with_workspace_metadata(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());
        let max_len = "a".repeat(128);

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), Some(&max_len)),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn idempotency_key_with_embedded_space_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), Some("has a space")),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn unknown_field_in_body_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let body = Bytes::from(
            json!({
                "kind": "repetition",
                "description": "valid description text",
                "evidence": "valid evidence text",
                "reportedImpact": "low",
                "confidence": 0.99,
            })
            .to_string(),
        );

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), Some("key-1")),
            body,
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn malformed_json_body_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), Some("key-1")),
            Bytes::from_static(b"{ not json"),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn response_body_never_contains_the_bearer_token() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session(&state, &id);
        set_workflow_report_token(&id, "super-secret-token-value".to_string());

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("super-secret-token-value"), Some("key-1")),
            valid_body(),
        )
        .await
        .into_response();

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("super-secret-token-value"));
    }

    #[tokio::test]
    async fn rate_limit_allows_configured_attempts_then_rejects_the_next() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = unique_session_id();
        insert_live_session(&state, &id);
        set_workflow_report_token(&id, "the-token".to_string());

        // Each attempt omits the Idempotency-Key header, so every call is
        // cheap (400, resolved before persistence) while still consuming one
        // of the route's WORKFLOW_REPORT_RATE_LIMIT pre-persistence attempts
        // for this rolling window.
        for attempt in 0..WORKFLOW_REPORT_RATE_LIMIT {
            let response = report_workflow_observation(
                State(state.clone()),
                Path(id.clone()),
                headers_with(Some("the-token"), None),
                valid_body(),
            )
            .await
            .into_response();
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "attempt {attempt} should reach the idempotency-key check, not the rate limit"
            );
        }

        let response = report_workflow_observation(
            State(state),
            Path(id),
            headers_with(Some("the-token"), None),
            valid_body(),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
