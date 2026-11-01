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
    let mut sessions_guard = state
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
    match operation {
        ParsedOperation::Read => {
            serde_json::to_value(record).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
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
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            } else {
                let scope = envelope.patch.scope();
                let updates_summary = envelope.patch.updates_summary();
                let updated = workspace
                    .metadata
                    .apply_agent_patch(id, &envelope.metadata_revision, &envelope.patch)
                    .map_err(status_for_patch_error)?;
                let value = serde_json::to_value(&updated)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let handle = sessions_guard
                    .get_mut(id)
                    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
                handle.info.last_activity_at = Some(updated.metadata.last_activity.clone());
                handle.info.worktree_binding = updated.metadata.worktree_binding.clone();
                if scope.attention {
                    handle.runtime.attention_owner.accepted_write();
                    handle.info.observed_status = updated.metadata.observed_status.clone();
                    handle.info.attention = updated.metadata.attention.clone();
                    handle.info.needs_user_input = updated.metadata.needs_user_input;
                    handle.info.detected_question = updated.metadata.detected_question.clone();
                    handle.info.suggested_options = updated.metadata.suggested_options.clone();
                    handle.info.provenance.attention_source =
                        updated.metadata.provenance.attention_source.clone();
                    handle.info.provenance.attention_confidence =
                        updated.metadata.provenance.attention_confidence;
                    handle.info.provenance.attention_origin =
                        updated.metadata.provenance.attention_origin.clone();
                    handle.info.provenance.attention_updated_at =
                        updated.metadata.provenance.attention_updated_at.clone();
                    if let Some(tracker) = handle.runtime.native_approval.as_mut() {
                        tracker.invalidate();
                    }
                    handle.pending_work_signal = None;
                    handle.runtime.active_prompt_kind = None;
                }
                if scope.work {
                    handle.info.work_phase = updated.metadata.work_phase.clone();
                    handle.info.next_action = updated.metadata.next_action.clone();
                    handle.info.blocker_description = updated.metadata.blocker_description.clone();
                    handle.info.failed_command = updated.metadata.failed_command.clone();
                    handle.info.failed_test = updated.metadata.failed_test.clone();
                    handle.info.metadata_source = Some(updated.metadata.metadata_source.clone());
                    handle.info.metadata_confidence = Some(updated.metadata.metadata_confidence);
                    handle.info.provenance.work_metadata_updated_at =
                        updated.metadata.provenance.work_metadata_updated_at.clone();
                }
                if updates_summary {
                    handle.info.summary = updated.metadata.summary.clone();
                    handle.info.provenance.summary_source =
                        updated.metadata.provenance.summary_source.clone();
                    handle.info.provenance.summary_confidence =
                        updated.metadata.provenance.summary_confidence;
                    handle.info.provenance.summary_observed_at =
                        updated.metadata.provenance.summary_observed_at.clone();
                }
                Ok(value)
            }
        }
    }
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

    #[tokio::test]
    #[cfg(unix)]
    async fn real_worktree_helper_drives_git_context_captures_switches_and_history() {
        let (state, id, launch) = fixture().await;
        let repo = git2::Repository::init(launch.path()).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let sig = git2::Signature::now("test", "test@example.com").unwrap();
        let tree = repo
            .find_tree(repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
            .unwrap();
        let tasks = tempfile::tempdir().unwrap();
        let b = tasks.path().join("feature");
        let c = tasks.path().join("other");
        repo.worktree("feature", &b, None).unwrap();
        repo.worktree("other", &c, None).unwrap();
        let feature_repo = git2::Repository::open(&b).unwrap();
        std::fs::write(b.join("committed.txt"), "feature commit\n").unwrap();
        let mut index = feature_repo.index().unwrap();
        index.add_path(FilePath::new("committed.txt")).unwrap();
        index.write().unwrap();
        let tree = feature_repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = feature_repo.head().unwrap().peel_to_commit().unwrap();
        feature_repo
            .commit(Some("HEAD"), &sig, &sig, "feature", &tree, &[&parent])
            .unwrap();
        std::fs::write(b.join("new.txt"), "feature working edit\n").unwrap();
        let _token = crate::test_support::open_plan_token::set("worktree-reader").await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let router = crate::build_router(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        struct Server(tokio::task::JoinHandle<()>);
        impl Drop for Server {
            fn drop(&mut self) {
                self.0.abort();
            }
        }
        let _server = Server(server);
        let client = reqwest::Client::new();
        let base = format!("http://127.0.0.1:{port}");
        async fn report(port: u16, id: &str, path: &str) {
            let result = tokio::process::Command::new("python3")
                .args([
                    concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/session-metadata.py"),
                    "worktree",
                    path,
                ])
                .env_remove("ORKWORKS_SESSION_METADATA_API_VERSION")
                .env("ORKWORKS_SESSION_ID", id)
                .env("ORKWORKS_PORT", port.to_string())
                .env("ORKWORKS_REPORT_TOKEN", "secret")
                .output()
                .await
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        async fn capture(
            client: &reqwest::Client,
            base: &str,
            id: &str,
            comparison: &str,
        ) -> Value {
            let response = client
                .get(format!(
                    "{base}/sessions/{id}/git-changes?comparison={comparison}"
                ))
                .header("x-orkworks-open-plan-token", "worktree-reader")
                .send()
                .await
                .unwrap();
            assert!(response.status().is_success(), "{}", response.status());
            response.json().await.unwrap()
        }
        report(port, &id, b.to_str().unwrap()).await;
        state
            .peon
            .reported_cwd
            .write()
            .unwrap()
            .insert(id.clone(), launch.path().display().to_string());
        let infos: Value = client
            .get(format!("{base}/sessions"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let info = infos
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap();
        assert_eq!(info["branch"], "feature");
        assert_eq!(info["gitContextSource"], "agent");
        let working = capture(&client, &base, &id, "uncommitted").await;
        assert_eq!(working["branch"], "feature");
        assert_eq!(working["files"][0]["path"], "new.txt");
        let branch = capture(&client, &base, &id, "branch&baseRef=refs%2Fheads%2Fmain").await;
        assert_eq!(branch["files"][0]["path"], "committed.txt");
        let patch_url = format!(
            "{base}/sessions/{id}/git-changes/{}/files/{}",
            working["snapshotId"].as_str().unwrap(),
            working["files"][0]["id"].as_str().unwrap()
        );
        let patch: Value = client
            .get(&patch_url)
            .header("x-orkworks-open-plan-token", "worktree-reader")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(patch["patch"]
            .as_str()
            .unwrap()
            .contains("+feature working edit"));
        assert_eq!(repo.head().unwrap().shorthand(), Some("main"));
        report(port, &id, c.to_str().unwrap()).await;
        assert_eq!(
            client
                .get(&patch_url)
                .header("x-orkworks-open-plan-token", "worktree-reader")
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            capture(&client, &base, &id, "uncommitted").await["branch"],
            "other"
        );
        std::fs::remove_dir_all(&c).unwrap();
        assert_eq!(
            client
                .get(format!(
                    "{base}/sessions/{id}/git-changes?comparison=uncommitted"
                ))
                .header("x-orkworks-open-plan-token", "worktree-reader")
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        report(port, &id, "--clear").await;
        assert_eq!(
            capture(&client, &base, &id, "uncommitted").await["branch"],
            "main"
        );
        report(port, &id, b.to_str().unwrap()).await;
        state.sessions.lock().unwrap().remove(&id);
        {
            let ws = state.workspace.lock().unwrap();
            let store = &ws.as_ref().unwrap().metadata;
            store
                .mutate_session(
                    &id,
                    crate::metadata::session_transactions::WriteScope::NONE,
                    |m| {
                        m.lifecycle = "dead".into();
                        m.lifecycle_phase = "ended".into();
                        m.status = "ended".into();
                        Ok(())
                    },
                )
                .unwrap();
        }
        assert_eq!(
            capture(&client, &base, &id, "uncommitted").await["branch"],
            "feature"
        );
    }

    fn install_hook_owned_wait(state: &Arc<AppState>, id: &str) {
        {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            let mut meta = store.read_session(id).unwrap();
            meta.observed_status = Some("waiting_for_input".into());
            meta.attention = Some("needs_you".into());
            meta.needs_user_input = Some(true);
            meta.provenance.attention_source = Some("agent".into());
            meta.provenance.attention_origin = Some("harness_hook".into());
            store.write_session(&meta);
        }
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(id).unwrap();
        handle.info.observed_status = Some("waiting_for_input".into());
        handle.info.attention = Some("needs_you".into());
        handle.info.needs_user_input = Some(true);
        handle.info.provenance.attention_source = Some("agent".into());
        handle.info.provenance.attention_origin = Some("harness_hook".into());
        handle.runtime.attention_owner.accepted_write();
        let mut tracker = crate::runtime::codex_approval_application::NativeApprovalState::new(1);
        tracker.record_validated_permission(&handle.runtime.attention_owner);
        handle.runtime.native_approval = Some(tracker);
        assert!(handle
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .protects_permission_wait(&handle.runtime.attention_owner, &handle.info));
    }

    #[cfg(unix)]
    async fn run_metadata_producer(
        state: Arc<AppState>,
        id: &str,
        command: &str,
        body: Option<Value>,
    ) -> std::process::Output {
        run_metadata_producer_with_protocol(state, id, command, body, false).await
    }

    #[cfg(unix)]
    async fn run_metadata_producer_with_protocol(
        state: Arc<AppState>,
        id: &str,
        command: &str,
        body: Option<Value>,
        ordinary: bool,
    ) -> std::process::Output {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let router = super::routes().with_state(state);
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let mut child = tokio::process::Command::new("python3")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/scripts/session-metadata.py"
            ))
            .arg(command)
            .env_remove("ORKWORKS_SESSION_METADATA_API_VERSION")
            .env_remove("ORKWORKS_SESSION_STORAGE_PROTOCOL")
            .env(
                if ordinary {
                    "ORKWORKS_SESSION_STORAGE_PROTOCOL"
                } else {
                    "ORKWORKS_SESSION_METADATA_API_VERSION"
                },
                if ordinary { "sidecar-v1" } else { "1" },
            )
            .env("ORKWORKS_SESSION_ID", id)
            .env("ORKWORKS_PORT", port.to_string())
            .env("ORKWORKS_REPORT_TOKEN", "secret")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        if let Some(body) = body {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(body.to_string().as_bytes())
                .await
                .unwrap();
        } else {
            drop(child.stdin.take());
        }
        let output =
            tokio::time::timeout(std::time::Duration::from_secs(20), child.wait_with_output())
                .await
                .unwrap()
                .unwrap();
        server.abort();
        let _ = server.await;
        output
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn producer_bootstrap_is_read_only_and_identical_attention_patch_revokes_owner() {
        for ordinary in [false, true] {
            let (state, id, _dir) = fixture().await;
            install_hook_owned_wait(&state, &id);
            let path = state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .sessions_dir()
                .join(format!("{id}.json"));
            let before = std::fs::read(&path).unwrap();
            let owner = state.sessions.lock().unwrap()[&id]
                .runtime
                .attention_owner
                .snapshot()
                .unwrap();
            let output = run_metadata_producer_with_protocol(
                state.clone(),
                &id,
                "bootstrap",
                None,
                ordinary,
            )
            .await;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(std::fs::read(&path).unwrap(), before);
            assert!(state.sessions.lock().unwrap()[&id]
                .runtime
                .attention_owner
                .owns(&owner));
            let validated: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(validated["validated"], true);
            let patch = json!({"metadataRevision": validated["metadataRevision"], "patch": {"attentionState": {
                "observedStatus": "waiting_for_input", "needsUserInput": true,
                "detectedQuestion": null, "suggestedOptions": null
            }}});
            let output = run_metadata_producer_with_protocol(
                state.clone(),
                &id,
                "patch",
                Some(patch),
                ordinary,
            )
            .await;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let sessions = state.sessions.lock().unwrap();
            let handle = &sessions[&id];
            assert!(!handle.runtime.attention_owner.owns(&owner));
            assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
            assert_eq!(
                handle.info.provenance.attention_origin.as_deref(),
                Some("direct_agent")
            );
            assert!(!handle
                .runtime
                .native_approval
                .as_ref()
                .unwrap()
                .protects_permission_wait(&handle.runtime.attention_owner, &handle.info));
            crate::runtime::terminal_runtime::clear_workflow_report_token(&id);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn producer_keeps_stale_patch_conflicting_after_fresh_bootstrap() {
        let (state, id, _dir) = fixture().await;
        let stale_revision = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .try_read_revisioned_session(&id)
            .unwrap()
            .unwrap()
            .revision;
        let stale_revision: String = stale_revision.into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .mutate_session(
                &id,
                crate::metadata::session_transactions::WriteScope::ALL,
                |meta| {
                    meta.summary = Some("competing writer".into());
                    Ok(())
                },
            )
            .unwrap();
        let path = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .sessions_dir()
            .join(format!("{id}.json"));
        let before = std::fs::read(&path).unwrap();
        let patch = json!({"metadataRevision": stale_revision, "patch": {"agentMetadata": {"summary": "stale intent"}}});
        let output = run_metadata_producer(state.clone(), &id, "patch", Some(patch)).await;
        assert_eq!(output.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&output.stderr).contains("reread"));
        assert_eq!(std::fs::read(path).unwrap(), before);
        crate::runtime::terminal_runtime::clear_workflow_report_token(&id);
    }

    async fn request(
        state: Arc<AppState>,
        id: &str,
        method: &str,
        body: Option<Value>,
        token: &str,
    ) -> (StatusCode, Value) {
        request_notifying(state, id, method, body, token, None).await
    }

    async fn request_notifying(
        state: Arc<AppState>,
        id: &str,
        method: &str,
        body: Option<Value>,
        token: &str,
        entered: Option<std::sync::mpsc::Sender<()>>,
    ) -> (StatusCode, Value) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let router = routes().with_state(state).layer(axum::middleware::from_fn(
                move |request: axum::extract::Request, next: axum::middleware::Next| {
                    let entered = entered.clone();
                    async move {
                        if let Some(entered) = entered {
                            entered.send(()).unwrap();
                        }
                        next.run(request).await
                    }
                },
            ));
            axum::serve(listener, router).await.unwrap();
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

    fn install_native_owned_wait(
        state: &Arc<AppState>,
        id: &str,
    ) -> (
        crate::metadata::SessionMetadata,
        crate::metadata::SessionWriteToken,
        crate::runtime::observed_status::AttentionWriteToken,
    ) {
        let workspace = state.workspace.lock().unwrap();
        let store = &workspace.as_ref().unwrap().metadata;
        let mut meta = store.read_session(id).unwrap();
        meta.observed_status = Some("waiting_for_input".into());
        meta.attention = Some("needs_you".into());
        meta.needs_user_input = Some(true);
        meta.detected_question = None;
        meta.suggested_options = None;
        meta.provenance.attention_source = Some("agent".into());
        meta.provenance.attention_origin = Some("native_codex".into());
        meta.provenance.attention_confidence = Some(1.0);
        // A complete API-valid tuple lets the identical-write case prove
        // revocation using the real retained-store native ownership token.
        // This fixture exercises the clear transaction, not reducer activation.
        let durable_owner = store.try_write_session_owned(&meta).unwrap().unwrap();
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(id).unwrap();
        publish_attention(handle, &meta);
        let live_owner = handle.runtime.attention_owner.snapshot().unwrap();
        (meta, durable_owner, live_owner)
    }

    fn publish_attention(
        handle: &mut crate::SessionHandle,
        meta: &crate::metadata::SessionMetadata,
    ) {
        handle.runtime.attention_owner.accepted_write();
        handle.info.observed_status = meta.observed_status.clone();
        handle.info.attention = meta.attention.clone();
        handle.info.needs_user_input = meta.needs_user_input;
        handle.info.detected_question = meta.detected_question.clone();
        handle.info.suggested_options = meta.suggested_options.clone();
        handle.info.provenance.attention_source = meta.provenance.attention_source.clone();
        handle.info.provenance.attention_confidence = meta.provenance.attention_confidence;
        handle.info.provenance.attention_origin = meta.provenance.attention_origin.clone();
        handle.info.provenance.attention_updated_at = meta.provenance.attention_updated_at.clone();
    }

    // Exercise the real native clear transaction with the application's actual
    // workspace → sessions lock order and committed live publication. Native
    // production activation remains disabled; no reducer/gate is bypassed there.
    fn clear_native_wait(
        state: &Arc<AppState>,
        mut native: crate::metadata::SessionMetadata,
        owner: &crate::metadata::SessionWriteToken,
        ready: impl FnOnce() -> bool,
    ) -> bool {
        let workspace = state.workspace.lock().unwrap();
        let store = &workspace.as_ref().unwrap().metadata;
        let mut sessions = state.sessions.lock().unwrap();
        native.observed_status = Some("working".into());
        native.attention = Some("working".into());
        native.needs_user_input = None;
        let committed = store
            .try_write_session_if_owned_when(&native, owner, ready)
            .unwrap();
        if committed {
            let saved = store
                .try_read_revisioned_session(&native.id)
                .unwrap()
                .unwrap();
            publish_attention(sessions.get_mut(&native.id).unwrap(), &saved.metadata);
        }
        committed
    }

    fn attention_patch(revision: &Value, identical: bool) -> Value {
        json!({"metadataRevision":revision,"patch":{"attentionState":{
            "observedStatus": if identical { "waiting_for_input" } else { "blocked" },
            "needsUserInput":true,
            "detectedQuestion":null,"suggestedOptions":null}}})
    }

    fn assert_current_attention(state: &Arc<AppState>, id: &str, status: &str, origin: &str) {
        let workspace = state.workspace.lock().unwrap();
        let saved = workspace
            .as_ref()
            .unwrap()
            .metadata
            .try_read_revisioned_session(id)
            .unwrap()
            .unwrap();
        let sessions = state.sessions.lock().unwrap();
        let info = &sessions.get(id).unwrap().info;
        assert_eq!(saved.metadata.observed_status.as_deref(), Some(status));
        assert_eq!(
            saved.metadata.provenance.attention_source.as_deref(),
            Some("agent")
        );
        assert_eq!(
            saved.metadata.provenance.attention_origin.as_deref(),
            Some(origin)
        );
        assert_eq!(info.observed_status, saved.metadata.observed_status);
        assert_eq!(info.attention, saved.metadata.attention);
        assert_eq!(info.needs_user_input, saved.metadata.needs_user_input);
        assert_eq!(info.detected_question, saved.metadata.detected_question);
        assert_eq!(info.suggested_options, saved.metadata.suggested_options);
        assert_eq!(
            info.provenance.attention_source,
            saved.metadata.provenance.attention_source
        );
        assert_eq!(
            info.provenance.attention_origin,
            saved.metadata.provenance.attention_origin
        );
        assert_eq!(
            info.provenance.attention_confidence,
            saved.metadata.provenance.attention_confidence
        );
        assert_eq!(
            info.provenance.attention_updated_at,
            saved.metadata.provenance.attention_updated_at
        );
        drop(sessions);
        drop(workspace);
        let projected = crate::session_projection::SessionProjection::new(state.clone())
            .list()
            .unwrap();
        let projected = projected.iter().find(|info| info.id == id).unwrap();
        assert_eq!(projected.observed_status, saved.metadata.observed_status);
        assert_eq!(projected.attention, saved.metadata.attention);
        assert_eq!(projected.needs_user_input, saved.metadata.needs_user_input);
        assert_eq!(
            projected.detected_question,
            saved.metadata.detected_question
        );
        assert_eq!(
            projected.suggested_options,
            saved.metadata.suggested_options
        );
        assert_eq!(
            projected.provenance.attention_source,
            saved.metadata.provenance.attention_source
        );
        assert_eq!(
            projected.provenance.attention_origin,
            saved.metadata.provenance.attention_origin
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn authenticated_patch_waits_for_native_clear_then_conflicts_and_retries() {
        use std::sync::mpsc;
        use std::time::Duration;
        let (state, id, _dir) = fixture().await;
        let (native, owner, _) = install_native_owned_wait(&state, &id);
        let (status, before) = request(state.clone(), &id, "GET", None, "secret").await;
        assert_eq!(status, StatusCode::OK);
        let body = attention_patch(&before["metadataRevision"], false);
        let (at_final_tx, at_final_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let clear_state = state.clone();
        let clear = std::thread::spawn(move || {
            clear_native_wait(&clear_state, native, &owner, || {
                at_final_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                true
            })
        });
        at_final_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let writer_state = state.clone();
        let writer_id = id.clone();
        let writer = tokio::spawn(async move {
            request_notifying(
                writer_state,
                &writer_id,
                "PATCH",
                Some(body),
                "secret",
                Some(entered_tx),
            )
            .await
        });
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        // Request has entered the registered router, while native clear holds
        // the application locks and store lane after its final check. The PATCH
        // cannot reach authentication/commit or finish before clear is released.
        assert!(state.workspace.try_lock().is_err());
        assert!(state.sessions.try_lock().is_err());
        assert!(!writer.is_finished());
        release_tx.send(()).unwrap();
        assert!(clear.join().unwrap());
        assert_eq!(writer.await.unwrap().0, StatusCode::CONFLICT);
        assert_current_attention(&state, &id, "working", "native_codex");
        let (status, cleared) = request(state.clone(), &id, "GET", None, "secret").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cleared["observedStatus"], "working");
        assert_ne!(cleared["metadataRevision"], before["metadataRevision"]);
        let (status, updated) = request(
            state.clone(),
            &id,
            "PATCH",
            Some(attention_patch(&cleared["metadataRevision"], false)),
            "secret",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_ne!(updated["metadataRevision"], cleared["metadataRevision"]);
        assert_current_attention(&state, &id, "blocked", "direct_agent");
        clear_workflow_report_token(&id);
    }

    #[tokio::test]
    async fn authenticated_attention_patch_first_revokes_old_native_clear_even_when_identical() {
        for identical in [false, true] {
            let (state, id, _dir) = fixture().await;
            let (native, owner, live_owner) = install_native_owned_wait(&state, &id);
            let (status, before) = request(state.clone(), &id, "GET", None, "secret").await;
            assert_eq!(status, StatusCode::OK);
            let (status, updated) = request(
                state.clone(),
                &id,
                "PATCH",
                Some(attention_patch(&before["metadataRevision"], identical)),
                "secret",
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            {
                let sessions = state.sessions.lock().unwrap();
                let handle = sessions.get(&id).unwrap();
                assert!(!handle.runtime.attention_owner.owns(&live_owner));
            }
            assert!(!clear_native_wait(&state, native, &owner, || panic!(
                "revoked owner must fail before the final check"
            )));
            let (status, after) = request(state.clone(), &id, "GET", None, "secret").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(after["metadataRevision"], updated["metadataRevision"]);
            assert_current_attention(
                &state,
                &id,
                if identical {
                    "waiting_for_input"
                } else {
                    "blocked"
                },
                "direct_agent",
            );
            clear_workflow_report_token(&id);
        }
    }

    #[tokio::test]
    async fn authenticated_work_patch_preserves_native_clear_owner_and_latest_work() {
        let (state, id, _dir) = fixture().await;
        let (native, owner, live_owner) = install_native_owned_wait(&state, &id);
        let (status, before) = request(state.clone(), &id, "GET", None, "secret").await;
        assert_eq!(status, StatusCode::OK);
        let body = json!({"metadataRevision":before["metadataRevision"],"patch":{"agentMetadata":{"workFields":{
            "task":"Latest API work","nextAction":"Review patch","workPhase":"implementation",
            "blockerDescription":null,"failedCommand":null,"failedTest":null}}}});
        assert_eq!(
            request(state.clone(), &id, "PATCH", Some(body), "secret")
                .await
                .0,
            StatusCode::OK
        );
        {
            let sessions = state.sessions.lock().unwrap();
            let handle = sessions.get(&id).unwrap();
            assert!(handle.runtime.attention_owner.owns(&live_owner));
        }
        assert!(clear_native_wait(&state, native, &owner, || true));
        assert_current_attention(&state, &id, "working", "native_codex");
        let (status, after) = request(state.clone(), &id, "GET", None, "secret").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(after["task"], "Latest API work");
        assert_eq!(after["nextAction"], "Review patch");
        assert_eq!(after["metadataSource"], "agent");
        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get(&id)
                .unwrap()
                .info
                .next_action
                .as_deref(),
            Some("Review patch")
        );
        clear_workflow_report_token(&id);
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

    #[tokio::test]
    async fn accepted_attention_patch_revokes_hook_wait_and_publishes_live_tuple() {
        let (state, id, _dir) = fixture().await;
        install_hook_owned_wait(&state, &id);
        let previous_owner = state
            .sessions
            .lock()
            .unwrap()
            .get(&id)
            .unwrap()
            .runtime
            .attention_owner
            .snapshot()
            .unwrap();
        let (_, before) = request(state.clone(), &id, "GET", None, "secret").await;
        let body = json!({"metadataRevision":before["metadataRevision"],"patch":{"attentionState":{
            "observedStatus":"working","needsUserInput":false,"detectedQuestion":null,"suggestedOptions":[]}}});
        let (status, after) = request(state.clone(), &id, "PATCH", Some(body), "secret").await;
        assert_eq!(status, StatusCode::OK);
        let sessions = state.sessions.lock().unwrap();
        let handle = sessions.get(&id).unwrap();
        assert_eq!(handle.info.observed_status.as_deref(), Some("working"));
        assert_eq!(handle.info.attention.as_deref(), Some("working"));
        assert_eq!(
            handle.info.provenance.attention_origin.as_deref(),
            Some("direct_agent")
        );
        assert_eq!(
            handle.info.provenance.attention_updated_at.as_deref(),
            after["attentionUpdatedAt"].as_str()
        );
        assert!(!handle.runtime.attention_owner.owns(&previous_owner));
        assert!(!handle
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .protects_permission_wait(&handle.runtime.attention_owner, &handle.info));
        drop(sessions);
        clear_workflow_report_token(&id);
    }

    #[tokio::test]
    async fn work_only_patch_preserves_hook_wait_owner() {
        let (state, id, _dir) = fixture().await;
        install_hook_owned_wait(&state, &id);
        let previous_owner = state
            .sessions
            .lock()
            .unwrap()
            .get(&id)
            .unwrap()
            .runtime
            .attention_owner
            .snapshot()
            .unwrap();
        let (_, before) = request(state.clone(), &id, "GET", None, "secret").await;
        let body = json!({"metadataRevision":before["metadataRevision"],"patch":{"agentMetadata":{"workFields":{
            "task":"Implement route","nextAction":"Run tests","workPhase":"implementation",
            "blockerDescription":null,"failedCommand":null,"failedTest":null}}}});
        let (status, _) = request(state.clone(), &id, "PATCH", Some(body), "secret").await;
        assert_eq!(status, StatusCode::OK);
        let sessions = state.sessions.lock().unwrap();
        let handle = sessions.get(&id).unwrap();
        assert!(handle.runtime.attention_owner.owns(&previous_owner));
        assert!(handle
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .protects_permission_wait(&handle.runtime.attention_owner, &handle.info));
        assert_eq!(
            handle.info.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        drop(sessions);
        clear_workflow_report_token(&id);
    }

    #[tokio::test]
    async fn failed_attention_patch_leaves_live_wait_unchanged() {
        let (state, id, _dir) = fixture().await;
        install_hook_owned_wait(&state, &id);
        let previous_owner = state
            .sessions
            .lock()
            .unwrap()
            .get(&id)
            .unwrap()
            .runtime
            .attention_owner
            .snapshot()
            .unwrap();
        let body = json!({"metadataRevision":"0","patch":{"attentionState":{
            "observedStatus":"working","needsUserInput":false,"detectedQuestion":null,"suggestedOptions":[]}}});
        let (status, _) = request(state.clone(), &id, "PATCH", Some(body), "secret").await;
        assert_eq!(status, StatusCode::CONFLICT);
        let sessions = state.sessions.lock().unwrap();
        let handle = sessions.get(&id).unwrap();
        assert!(handle.runtime.attention_owner.owns(&previous_owner));
        assert!(handle
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .protects_permission_wait(&handle.runtime.attention_owner, &handle.info));
        assert_eq!(
            handle.info.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        drop(sessions);
        clear_workflow_report_token(&id);
    }
}
