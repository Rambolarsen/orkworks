//! Authenticated versioned metadata HTTP adapter.

use crate::metadata::agent_patch::{AgentPatchEnvelope, AgentPatchError};
use crate::runtime::terminal_runtime::verify_workflow_report_token;
use crate::session_types::MemoryState;
use crate::AppState;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{path::Path as FilePath, sync::Arc};

const MAX_PATCH_BODY_BYTES: usize = 8 * 1024;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/sessions/:id/metadata",
            get(get_metadata).patch(patch_metadata),
        )
        .layer(DefaultBodyLimit::max(MAX_PATCH_BODY_BYTES))
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PatchQuery {
    #[serde(default)]
    validate_only: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidatedResponse {
    validated: bool,
    metadata_revision: crate::metadata::session_transactions::MetadataRevision,
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let header = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;
    if token.is_empty() || token.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return None;
    }
    Some(token.to_owned())
}

enum MetadataOperation {
    Read,
    Patch { body: Bytes, validate_only: bool },
}

fn perform(
    state: &AppState,
    id: &str,
    token: &str,
    operation: MetadataOperation,
) -> Result<serde_json::Value, StatusCode> {
    // Keep workspace and session identity stable through the store transaction.
    // The order matches existing application code: workspace, then sessions.
    let workspace_guard = state
        .workspace
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let workspace = workspace_guard.as_ref().ok_or(StatusCode::UNAUTHORIZED)?;
    let sessions_guard = state
        .sessions
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let live = sessions_guard.get(id).is_some_and(|handle| {
        handle.info.id == id
            && handle.info.memory_state == MemoryState::Live
            && handle.info.lifecycle == "alive"
            && handle.info.lifecycle_phase == "active"
    });
    if !live || !verify_workflow_report_token(id, token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let operation = match operation {
        MetadataOperation::Patch {
            body,
            validate_only,
        } => {
            let envelope = serde_json::from_slice::<AgentPatchEnvelope>(&body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            ParsedOperation::Patch {
                envelope,
                validate_only,
            }
        }
        MetadataOperation::Read => ParsedOperation::Read,
    };
    let record = workspace
        .metadata
        .try_read_revisioned_session(id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    if FilePath::new(&record.metadata.workspace) != workspace.path {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if record.metadata.lifecycle != "alive"
        || record.metadata.lifecycle_phase != "active"
        || record.metadata.status != "running"
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let value = match operation {
        ParsedOperation::Read => serde_json::to_value(record),
        ParsedOperation::Patch {
            envelope,
            validate_only,
        } => {
            if validate_only {
                let revision = workspace
                    .metadata
                    .validate_agent_patch(id, &envelope.metadata_revision, &envelope.patch)
                    .map_err(status_for_patch_error)?;
                serde_json::to_value(ValidatedResponse {
                    validated: true,
                    metadata_revision: revision,
                })
            } else {
                let updated = workspace
                    .metadata
                    .apply_agent_patch(id, &envelope.metadata_revision, &envelope.patch)
                    .map_err(status_for_patch_error)?;
                serde_json::to_value(updated)
            }
        }
    };
    value.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

enum ParsedOperation {
    Read,
    Patch {
        envelope: AgentPatchEnvelope,
        validate_only: bool,
    },
}

fn status_for_patch_error(error: AgentPatchError) -> StatusCode {
    match error {
        AgentPatchError::Conflict => StatusCode::CONFLICT,
        AgentPatchError::BadRequest => StatusCode::BAD_REQUEST,
        AgentPatchError::Unauthorized => StatusCode::UNAUTHORIZED,
        AgentPatchError::Io(error) => {
            tracing::error!(error = %error, "session metadata transaction failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn get_metadata(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(token) = bearer_token(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match tokio::task::spawn_blocking(move || perform(&state, &id, &token, MetadataOperation::Read))
        .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn patch_metadata(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<PatchQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(token) = bearer_token(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let operation = MetadataOperation::Patch {
        body,
        validate_only: query.validate_only,
    };
    match tokio::task::spawn_blocking(move || perform(&state, &id, &token, operation)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::session_runtime::SessionRuntime;
    use crate::runtime::terminal_runtime::{
        clear_workflow_report_token, set_workflow_report_token,
    };
    use crate::test_support::{
        test_app_state_with_workspace, test_session_info, test_session_metadata,
    };
    use axum::http::StatusCode;
    use serde_json::{json, Value};

    async fn fixture() -> (Arc<AppState>, String, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = format!("metadata-route-{}", uuid::Uuid::new_v4());
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: test_session_info(
                    &id,
                    "test",
                    dir.path().to_str().unwrap(),
                    "running",
                    "now",
                ),
                active_work_hook: false,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: SessionRuntime::detached_test(),
                terminal_attached: false,
                resume_in_progress: false,
                capacity: crate::capacity_state::CapacityState::default(),
            },
        );
        let ws = state.workspace.lock().unwrap();
        let mut meta = test_session_metadata(
            &id,
            "test",
            dir.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        meta.connectivity = "online".into();
        meta.terminal_outcome = None;
        meta.final_observed_status_snapshot = None;
        meta.cwd = dir.path().display().to_string();
        ws.as_ref().unwrap().metadata.write_session(&meta);
        drop(ws);
        set_workflow_report_token(&id, "secret".into());
        (state, id, dir)
    }

    async fn request(
        state: Arc<AppState>,
        id: &str,
        method: &str,
        body: Option<Value>,
        token: &str,
    ) -> (StatusCode, Value) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, routes().with_state(state))
                .await
                .unwrap();
        });
        let client = reqwest::Client::new();
        let (session_id, query) = id.split_once('?').unwrap_or((id, ""));
        let suffix = if query.is_empty() {
            String::new()
        } else {
            format!("?{query}")
        };
        let mut builder = client.request(
            method.parse().unwrap(),
            format!("http://{address}/sessions/{session_id}/metadata{suffix}"),
        );
        if !token.is_empty() {
            builder = builder.bearer_auth(token);
        }
        if let Some(body) = body {
            builder = builder.json(&body);
        }
        let response = builder.send().await.unwrap();
        let status = response.status();
        let bytes = response.bytes().await.unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        server.abort();
        (status, json)
    }

    #[tokio::test]
    async fn authenticated_patch_advances_revision_and_rejects_stale_write() {
        let (state, id, _dir) = fixture().await;
        let (status, initial) = request(state.clone(), &id, "GET", None, "secret").await;
        assert_eq!(status, StatusCode::OK);
        let revision = initial["metadataRevision"].as_str().unwrap();
        let body = json!({"metadataRevision":revision,"patch":{"attentionState":{
            "observedStatus":"blocked","needsUserInput":true,"detectedQuestion":null,"suggestedOptions":[]}}});
        let (status, updated) =
            request(state.clone(), &id, "PATCH", Some(body.clone()), "secret").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated["attention"], "blocked");
        assert_eq!(updated["attentionOrigin"], "direct_agent");
        assert_ne!(updated["metadataRevision"], initial["metadataRevision"]);
        let (status, _) = request(state.clone(), &id, "PATCH", Some(body), "secret").await;
        assert_eq!(status, StatusCode::CONFLICT);
        clear_workflow_report_token(&id);
    }

    #[tokio::test]
    async fn wrong_token_cannot_read_or_patch() {
        let (state, id, _dir) = fixture().await;
        assert_eq!(
            request(state.clone(), &id, "GET", None, "wrong").await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(state.clone(), &id, "PATCH", Some(json!({})), "wrong")
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
        clear_workflow_report_token(&id);
    }

    #[tokio::test]
    async fn validation_only_preserves_revision_and_attention() {
        let (state, id, _dir) = fixture().await;
        let (_, initial) = request(state.clone(), &id, "GET", None, "secret").await;
        let body = json!({"metadataRevision":initial["metadataRevision"],"patch":{"attentionState":{
            "observedStatus":"waiting_for_input","needsUserInput":true,
            "detectedQuestion":"Continue?","suggestedOptions":["Yes"]}}});
        let (status, validated) = request(
            state.clone(),
            &format!("{id}?validateOnly=true"),
            "PATCH",
            Some(body),
            "secret",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(validated["validated"], true);
        assert_eq!(validated["metadataRevision"], initial["metadataRevision"]);
        let (_, later) = request(state.clone(), &id, "GET", None, "secret").await;
        assert_eq!(later["metadataRevision"], initial["metadataRevision"]);
        assert_eq!(later["attention"], initial["attention"]);
        assert_eq!(later["attentionUpdatedAt"], initial["attentionUpdatedAt"]);
        clear_workflow_report_token(&id);
    }

    #[tokio::test]
    async fn dead_session_and_foreign_token_are_unauthorized() {
        let (state, id, _dir) = fixture().await;
        let foreign_id = format!("foreign-{}", uuid::Uuid::new_v4());
        set_workflow_report_token(&foreign_id, "foreign-secret".into());
        assert_eq!(
            request(state.clone(), &id, "GET", None, "foreign-secret")
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut(&id)
            .unwrap()
            .info
            .memory_state = MemoryState::Remembered;
        assert_eq!(
            request(state.clone(), &id, "GET", None, "secret").await.0,
            StatusCode::UNAUTHORIZED
        );
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut(&id)
            .unwrap()
            .info
            .memory_state = MemoryState::Live;
        {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            let mut meta = store.read_session(&id).unwrap();
            meta.lifecycle = "dead".into();
            meta.lifecycle_phase = "ended".into();
            meta.status = "ended".into();
            store.write_session(&meta);
        }
        assert_eq!(
            request(state.clone(), &id, "GET", None, "secret").await.0,
            StatusCode::UNAUTHORIZED
        );
        clear_workflow_report_token(&id);
        clear_workflow_report_token(&foreign_id);
    }

    #[tokio::test]
    async fn validation_only_skips_source_priority_without_allowing_the_write() {
        let (state, id, _dir) = fixture().await;
        {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            let mut meta = store.read_session(&id).unwrap();
            meta.provenance.attention_source = Some("user".into());
            store.write_session(&meta);
        }
        let (_, current) = request(state.clone(), &id, "GET", None, "secret").await;
        let body = json!({"metadataRevision":current["metadataRevision"],"patch":{"attentionState":{
            "observedStatus":"working","needsUserInput":false,"detectedQuestion":null,"suggestedOptions":[]}}});
        let (status, _) = request(
            state.clone(),
            &format!("{id}?validateOnly=true"),
            "PATCH",
            Some(body.clone()),
            "secret",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = request(state.clone(), &id, "PATCH", Some(body), "secret").await;
        assert_eq!(status, StatusCode::CONFLICT);
        let (_, later) = request(state.clone(), &id, "GET", None, "secret").await;
        assert_eq!(later["metadataRevision"], current["metadataRevision"]);
        clear_workflow_report_token(&id);
    }

    #[tokio::test]
    async fn oversized_patch_is_rejected_by_route_body_limit() {
        let (state, id, _dir) = fixture().await;
        let (_, current) = request(state.clone(), &id, "GET", None, "secret").await;
        let body = json!({"metadataRevision":current["metadataRevision"],
            "patch":{"agentMetadata":{"summary":"x".repeat(9_000)}}});
        let (status, _) = request(state.clone(), &id, "PATCH", Some(body), "secret").await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        clear_workflow_report_token(&id);
    }
}
