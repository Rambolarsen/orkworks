use crate::session_projection::SessionProjection;
#[cfg(test)]
use crate::session_projection::enrich_sessions_with_git_context as project_git_context;
#[cfg(test)]
use crate::session_application::{resolve_session_launch, CreateSessionCommand};
use crate::session_application::{
    try_install_claimed_resume_handle, DebugAttentionSignal, SessionApplication, SessionError,
};
use crate::session_types::SessionInfo;
#[cfg(test)]
use crate::workspace_runtime::orkworks_global_dir;
use crate::{git, harness, metadata, peon, AppState, SessionHandle};
#[cfg(test)]
use crate::{watcher, WorkspaceState};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Deserialize)]
pub(crate) struct WorkspaceRequest {
    pub(crate) path: String,
}

#[derive(Deserialize)]
pub(crate) struct ActiveSessionRequest {
    #[serde(rename = "sessionId")]
    pub(crate) session_id: String,
}

#[derive(Deserialize)]
pub(crate) struct ActiveHarnessesRequest {
    #[serde(rename = "activeHarnessIds", default)]
    pub(crate) active_harness_ids: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct HarnessSessionReportRequest {
    #[serde(rename = "harnessSessionId")]
    pub(crate) harness_session_id: String,
    pub(crate) source: String,
    pub(crate) confidence: f64,
}

#[derive(Deserialize)]
pub(crate) struct AttentionReportRequest {
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) message: Option<String>,
    #[serde(rename = "planPath", default)]
    pub(crate) plan_path: metadata::PlanPathUpdate,
    #[serde(rename = "observedAt", default)]
    pub(crate) observed_at: Option<String>,
    /// The harness's own logical working directory, when it reports one
    /// (currently Claude Code only, forwarded from its hook JSON payload).
    /// Authoritative over the pid-probed/launch-time cwd fallbacks — see
    /// ADR 0032.
    #[serde(default)]
    pub(crate) cwd: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct PlanPathReportRequest {
    #[serde(rename = "planPath")]
    pub(crate) plan_path: String,
}

#[derive(Deserialize)]
pub(crate) struct TerminalPlanSelectionRequest {
    #[serde(rename = "printedPath")]
    pub(crate) printed_path: String,
}

#[derive(Deserialize)]
pub(crate) struct DebugAttentionRequest {
    pub(crate) attention: String,
    #[serde(default)]
    pub(crate) message: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct WorkspaceResponse {
    pub(crate) path: String,
    pub(crate) repo_root: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) dirty: Option<bool>,
    #[serde(rename = "lastActiveSessionId")]
    pub(crate) last_active_session_id: Option<String>,
    #[serde(rename = "activeHarnessIds", skip_serializing_if = "Vec::is_empty")]
    pub(crate) active_harness_ids: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct PlanContentResponse {
    pub(crate) content: String,
}

fn authorize_plan_request(headers: &HeaderMap) -> Result<(), axum::http::StatusCode> {
    let Ok(token) = std::env::var("ORKWORKS_OPEN_PLAN_TOKEN") else {
        return Err(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    };
    if token.is_empty() {
        return Err(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }
    (Some(token.as_str())
        == headers
            .get("x-orkworks-open-plan-token")
            .and_then(|value| value.to_str().ok()))
    .then_some(())
    .ok_or(axum::http::StatusCode::UNAUTHORIZED)
}

pub(crate) async fn get_session_plan_content(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(status) = authorize_plan_request(&headers) {
        return status.into_response();
    }
    match SessionApplication::new(state).read_plan_content(&id) {
        Ok(content) => Json(PlanContentResponse { content }).into_response(),
        Err(error) => application_error_response(error),
    }
}

pub(crate) async fn request_session_plan_review(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(status) = authorize_plan_request(&headers) {
        return status.into_response();
    }
    SessionApplication::new(state)
        .request_plan_review(&id)
        .await
        .map(|_| axum::http::StatusCode::NO_CONTENT.into_response())
        .unwrap_or_else(application_error_response)
}

pub(crate) async fn report_session_plan_path(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<PlanPathReportRequest>,
) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        SessionApplication::new(state).report_plan_path(&id, &req.plan_path)
    })
    .await;
    match result {
        Ok(Ok(())) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Ok(Err(error)) => application_error_response(error),
        Err(error) => {
            tracing::error!(error = %error, "plan path metadata task failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub(crate) async fn set_workspace(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WorkspaceRequest>,
) -> impl IntoResponse {
    let projection_state = state.clone();
    let _projection = projection_state.projection_lock.lock().unwrap();
    match SessionApplication::new(state).open_workspace(PathBuf::from(&req.path)) {
        Ok(snapshot) => Json(WorkspaceResponse {
            path: snapshot.path,
            repo_root: snapshot.repo_root,
            branch: snapshot.branch,
            dirty: snapshot.dirty,
            last_active_session_id: snapshot.last_active_session_id,
            active_harness_ids: snapshot.active_harness_ids,
        })
        .into_response(),
        Err(crate::session_application::SessionError::BadRequest(message)) => {
            (axum::http::StatusCode::BAD_REQUEST, message).into_response()
        }
        Err(crate::session_application::SessionError::Internal(message)) => {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub(crate) async fn set_active_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActiveSessionRequest>,
) -> impl IntoResponse {
    match SessionApplication::new(state).set_active_session(&req.session_id) {
        Ok(()) => axum::http::StatusCode::OK,
        Err(crate::session_application::SessionError::Conflict) => axum::http::StatusCode::CONFLICT,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub(crate) async fn set_active_harnesses(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActiveHarnessesRequest>,
) -> impl IntoResponse {
    match SessionApplication::new(state).set_active_harnesses(req.active_harness_ids) {
        Ok(()) => axum::http::StatusCode::OK,
        Err(crate::session_application::SessionError::Conflict) => axum::http::StatusCode::CONFLICT,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub(crate) async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> axum::response::Response {
    SessionApplication::new(state)
        .create_session(crate::session_application::CreateSessionCommand {
            harness_id: req.harness_id,
            model: req.model,
            initial_prompt: req.initial_prompt,
        })
        .await
        .map(|info| Json(info).into_response())
        .unwrap_or_else(application_error_response)
}

pub(crate) async fn resume_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::response::Response {
    SessionApplication::new(state)
        .resume_session(&id)
        .await
        .map(|info| Json(info).into_response())
        .unwrap_or_else(application_error_response)
}

pub(crate) async fn report_attention(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AttentionReportRequest>,
) -> axum::response::Response {
    SessionApplication::new(state)
        .report_attention(
            &id,
            crate::session_application::AttentionSignal {
                status: req.status,
                message: req.message,
                plan_path: req.plan_path,
                observed_at: req.observed_at,
                cwd: req.cwd,
            },
        )
        .await
        .map(|_| axum::http::StatusCode::OK.into_response())
        .unwrap_or_else(application_error_response)
}

pub(crate) async fn select_terminal_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<TerminalPlanSelectionRequest>,
) -> axum::response::Response {
    if let Err(status) = authorize_plan_request(&headers) {
        return status.into_response();
    }
    SessionApplication::new(state)
        .select_plan(
            &id,
            crate::session_application::PlanSelection {
                printed_path: req.printed_path,
            },
        )
        .await
        .map(|_| axum::http::StatusCode::NO_CONTENT.into_response())
        .unwrap_or_else(application_error_response)
}

pub(crate) async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::response::Response {
    SessionApplication::new(state)
        .delete_session(&id)
        .await
        .map(|_| axum::http::StatusCode::OK.into_response())
        .unwrap_or_else(application_error_response)
}

pub(crate) async fn forget_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::response::Response {
    SessionApplication::new(state)
        .forget_session(&id)
        .await
        .map(|_| axum::http::StatusCode::OK.into_response())
        .unwrap_or_else(application_error_response)
}

fn application_error_response(
    error: crate::session_application::SessionError,
) -> axum::response::Response {
    match error {
        crate::session_application::SessionError::BadRequest(message) => {
            (axum::http::StatusCode::BAD_REQUEST, message).into_response()
        }
        crate::session_application::SessionError::EmptyBadRequest => {
            axum::http::StatusCode::BAD_REQUEST.into_response()
        }
        crate::session_application::SessionError::Conflict => {
            axum::http::StatusCode::CONFLICT.into_response()
        }
        crate::session_application::SessionError::ConflictWithMessage(message) => {
            (axum::http::StatusCode::CONFLICT, message).into_response()
        }
        crate::session_application::SessionError::NotFound => {
            axum::http::StatusCode::NOT_FOUND.into_response()
        }
        crate::session_application::SessionError::Internal(_) => {
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub(crate) async fn report_harness_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<HarnessSessionReportRequest>,
) -> impl IntoResponse {
    let report = metadata::HarnessSessionReport {
        harness_session_id: req.harness_session_id,
        source: req.source,
        confidence: req.confidence,
    };

    let result = match SessionApplication::new(state).report_harness_session(&id, report) {
        Ok(result) => result,
        Err(crate::session_application::SessionError::Conflict) => {
            return axum::http::StatusCode::CONFLICT.into_response();
        }
        Err(_) => return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    match result {
        metadata::HarnessSessionMergeResult::Accepted
        | metadata::HarnessSessionMergeResult::IgnoredLowerConfidence => {
            axum::http::StatusCode::OK.into_response()
        }
        metadata::HarnessSessionMergeResult::NotFound => {
            axum::http::StatusCode::NOT_FOUND.into_response()
        }
        metadata::HarnessSessionMergeResult::Invalid => {
            axum::http::StatusCode::BAD_REQUEST.into_response()
        }
    }
}

/// Dev-only convenience for exercising UI/runtime convergence without a real
/// coding-agent session. Writes through the same merge path as `report_attention`
/// but tagged `source = "debug"`, `confidence = 0.0` — the lowest documented
/// priority tier, so any real signal (including the next peon inference pass)
/// reclaims the session immediately. That reclaim is the intended behavior, not
/// a bug: injecting a value and watching it get overwritten by a real signal is
/// itself the convergence test this endpoint exists to support.
pub(crate) async fn apply_debug_attention(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<DebugAttentionRequest>,
) -> impl IntoResponse {
    let result = match SessionApplication::new(state)
        .apply_debug_attention(
            &id,
            DebugAttentionSignal {
                attention: req.attention,
                message: req.message,
            },
        )
        .await
    {
        Ok(result) => result,
        Err(SessionError::EmptyBadRequest) => {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        }
        Err(SessionError::Conflict) => {
            return axum::http::StatusCode::CONFLICT.into_response();
        }
        Err(SessionError::NotFound) => {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        }
        Err(_) => return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    match result {
        metadata::AttentionMergeResult::Accepted => axum::http::StatusCode::OK.into_response(),
        metadata::AttentionMergeResult::Ignored => axum::http::StatusCode::OK.into_response(),
        metadata::AttentionMergeResult::NotFound => {
            axum::http::StatusCode::NOT_FOUND.into_response()
        }
        metadata::AttentionMergeResult::PersistFailed => {
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize, Default)]
pub(crate) struct CreateSessionRequest {
    #[serde(rename = "harnessId", default)]
    pub(crate) harness_id: Option<String>,
    #[serde(default)]
    pub(crate) model: Option<String>,
    #[serde(rename = "initialPrompt", default)]
    pub(crate) initial_prompt: Option<String>,
}

#[cfg(test)]
fn enrich_sessions_with_git_context<F>(
    infos: &mut [SessionInfo],
    effective_cwds: &HashMap<String, String>,
    detect_git: F,
) where
    F: FnMut(&std::path::Path) -> git::GitContext,
{
    project_git_context(infos, effective_cwds, detect_git);
}

pub(crate) async fn list_sessions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        #[cfg(test)]
        let before_write_back = || tests::run_list_sessions_before_write_back_hook(&state);
        let projection = SessionProjection::new(state.clone());
        #[cfg(test)]
        {
            projection.list_with_hook(before_write_back)
        }
        #[cfg(not(test))]
        {
            projection.list()
        }
    })
    .await;

    match result {
        Ok(infos) => Json(infos).into_response(),
        Err(error) => {
            tracing::error!(error = %error, "session projection task failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::terminal_runtime::set_session_status;
    use crate::test_support::*;

    static PLAN_TOKEN_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    static LIST_SESSIONS_BEFORE_WRITE_BACK_HOOK: std::sync::LazyLock<
        std::sync::Mutex<HashMap<usize, Box<dyn FnOnce() + Send>>>,
    > = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

    fn install_list_sessions_before_write_back_hook(
        state: &Arc<AppState>,
        hook: Box<dyn FnOnce() + Send>,
    ) {
        LIST_SESSIONS_BEFORE_WRITE_BACK_HOOK
            .lock()
            .unwrap()
            .insert(Arc::as_ptr(state) as usize, hook);
    }

    pub(super) fn run_list_sessions_before_write_back_hook(state: &Arc<AppState>) {
        if let Some(hook) = LIST_SESSIONS_BEFORE_WRITE_BACK_HOOK
            .lock()
            .unwrap()
            .remove(&(Arc::as_ptr(state) as usize))
        {
            hook();
        }
    }

    async fn listed_sessions(state: Arc<AppState>) -> Vec<serde_json::Value> {
        let response = list_sessions(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// `create_session` now returns before the detached spawn completes (see
    /// issue #302), so any test that needs the session past its `"creating"`
    /// interval — e.g. to call an endpoint that requires `lifecycle ==
    /// "alive"` — must wait for the spawn like a real client polling would.
    async fn wait_for_session_status(state: &Arc<AppState>, id: &str, status: &str) {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let matches = state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(id)
                    .is_some_and(|handle| handle.info.status == status);
                if matches {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("session {id} did not reach status {status:?} in time"));
    }

    #[test]
    fn session_git_context_is_resolved_once_per_unique_cwd() {
        let shared = "/workspace/shared";
        let separate = "/workspace/separate";
        let mut infos = vec![
            test_session_info("one", "One", shared, "running", "now"),
            test_session_info("two", "Two", shared, "running", "now"),
            test_session_info("three", "Three", separate, "ended", "now"),
        ];
        let mut calls: HashMap<String, usize> = HashMap::new();

        enrich_sessions_with_git_context(&mut infos, &HashMap::new(), |cwd| {
            *calls.entry(cwd.display().to_string()).or_default() += 1;
            git::GitContext {
                repo_root: Some(format!("{}/repo", cwd.display())),
                branch: Some("test-branch".into()),
                dirty: true,
                changed_files: 2,
                is_worktree: cwd == std::path::Path::new(separate),
            }
        });

        assert_eq!(calls.get(shared), Some(&1));
        assert_eq!(calls.get(separate), Some(&1));
        assert_eq!(calls.len(), 2);
        assert_eq!(
            infos[0].repo_root.as_deref(),
            Some("/workspace/shared/repo")
        );
        assert_eq!(infos[1].branch.as_deref(), Some("test-branch"));
        assert_eq!(infos[1].dirty, Some(true));
        assert_eq!(infos[1].changed_files, Some(2));
        assert_eq!(infos[2].is_worktree, Some(true));
        assert!(infos[0].recommendation.is_some());
    }

    #[test]
    fn session_git_context_uses_supplied_effective_cwd_over_launch_cwd() {
        let launch_cwd = "/workspace/launched-here";
        let mut infos = vec![
            test_session_info("moved", "Moved", launch_cwd, "running", "now"),
            test_session_info("stayed", "Stayed", launch_cwd, "running", "now"),
        ];
        // "moved" resolved to a live cwd distinct from its launch cwd (e.g. via
        // `resolve_effective_cwds`); "stayed" has no entry, so it should fall
        // back to its launch cwd.
        let effective_cwds: HashMap<String, String> =
            HashMap::from([("moved".to_string(), "/workspace/worktree".to_string())]);
        let mut detect_calls: Vec<String> = Vec::new();

        enrich_sessions_with_git_context(&mut infos, &effective_cwds, |cwd| {
            detect_calls.push(cwd.display().to_string());
            git::GitContext {
                repo_root: Some(format!("{}/repo", cwd.display())),
                branch: Some(format!("branch-for-{}", cwd.display())),
                dirty: false,
                changed_files: 0,
                is_worktree: false,
            }
        });

        assert_eq!(
            infos[0].branch.as_deref(),
            Some("branch-for-/workspace/worktree")
        );
        assert_eq!(
            infos[1].branch.as_deref(),
            Some(format!("branch-for-{launch_cwd}").as_str())
        );
        assert!(detect_calls.contains(&"/workspace/worktree".to_string()));
        assert!(detect_calls.contains(&launch_cwd.to_string()));
    }

    #[test]
    fn session_git_context_falls_back_to_launch_cwd_when_not_in_effective_cwds() {
        let launch_cwd = "/workspace/no-entry-tracked";
        let mut infos = vec![test_session_info(
            "untracked",
            "Untracked",
            launch_cwd,
            "running",
            "now",
        )];

        enrich_sessions_with_git_context(&mut infos, &HashMap::new(), |cwd| git::GitContext {
            repo_root: Some(cwd.display().to_string()),
            branch: None,
            dirty: false,
            changed_files: 0,
            is_worktree: false,
        });

        assert_eq!(infos[0].repo_root.as_deref(), Some(launch_cwd));
    }

    fn attention_test_handle(id: &str, cwd: &std::path::Path) -> SessionHandle {
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        SessionHandle {
            info: test_session_info(id, "Known", cwd.display().to_string(), "running", "now"),
            active_work_hook: false,
            kill_tx,
            output_buffer: peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
            ),
            terminal_attached: false,
            resume_in_progress: false,
            at_usage_limit_latched: false,
            capacity_check_pending: false,
            output_lines_seen: 0,
            scan_bytes_seen: 0,
            resume_scan_origin: None,
            pending_capacity_visible_once: false,
        }
    }

    #[tokio::test]
    async fn list_sessions_prefers_live_records_and_keeps_durable_metadata_in_live_then_remembered_order(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let duplicate_id = "duplicate-live";
        let mut duplicate_metadata = test_session_metadata(
            duplicate_id,
            "Remembered durable label",
            dir.path().display().to_string(),
            "ended",
            "before",
            "durable-activity",
        );
        duplicate_metadata.harness = "codex".into();
        duplicate_metadata.lifecycle_phase = "ended".into();
        duplicate_metadata.lifecycle = "dead".into();
        duplicate_metadata.summary = Some("durable summary".into());
        duplicate_metadata.metadata_source = "agent".into();
        duplicate_metadata.metadata_confidence = 0.9;
        let mut remembered_one = test_session_metadata(
            "remembered-one",
            "Remembered one",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        remembered_one.harness = "codex".into();
        let mut remembered_two = test_session_metadata(
            "remembered-two",
            "Remembered two",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        remembered_two.harness = "codex".into();
        {
            let workspace = state.workspace.lock().unwrap();
            let metadata = &workspace.as_ref().unwrap().metadata;
            metadata.write_session(&duplicate_metadata);
            metadata.write_session(&remembered_one);
            metadata.write_session(&remembered_two);
        }

        let mut duplicate_live = attention_test_handle(duplicate_id, dir.path());
        duplicate_live.info.label = "Live runtime label".into();
        duplicate_live.info.status = "running".into();
        duplicate_live.info.cwd = "/live/runtime/cwd".into();
        duplicate_live.info.lifecycle_phase = "active".into();
        duplicate_live.info.lifecycle = "alive".into();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(duplicate_id.into(), duplicate_live);
        state.sessions.lock().unwrap().insert(
            "live-only".into(),
            attention_test_handle("live-only", dir.path()),
        );

        let sessions = listed_sessions(state.clone()).await;
        let duplicate: Vec<_> = sessions
            .iter()
            .filter(|session| session["id"] == duplicate_id)
            .collect();
        assert_eq!(
            duplicate.len(),
            1,
            "live id must suppress its remembered duplicate"
        );
        let duplicate = duplicate[0];
        assert_eq!(duplicate["status"], "running");
        assert_eq!(duplicate["connectivity"], "online");
        assert_eq!(duplicate.get("terminalOutcome"), None);
        assert_eq!(duplicate["cwd"], "/live/runtime/cwd");
        assert_eq!(duplicate["label"], "Remembered durable label");
        assert_eq!(duplicate["harnessId"], "codex");
        assert_eq!(duplicate["harness"], "codex");
        assert_eq!(duplicate["lifecyclePhase"], "ended");
        assert_eq!(duplicate["lifecycle"], "dead");
        assert_eq!(duplicate["memoryState"], "remembered");
        assert_eq!(duplicate["summary"], "durable summary");
        assert_eq!(duplicate["metadataSource"], "agent");
        assert_eq!(duplicate["metadataConfidence"], 0.9);

        let live_ids = [duplicate_id, "live-only"];
        let first_remembered = sessions
            .iter()
            .position(|session| !live_ids.contains(&session["id"].as_str().unwrap()))
            .unwrap();
        assert!(
            sessions[..first_remembered]
                .iter()
                .all(|session| live_ids.contains(&session["id"].as_str().unwrap())),
            "live HashMap records precede remembered metadata records"
        );
        assert!(
            sessions[first_remembered..]
                .iter()
                .all(|session| !live_ids.contains(&session["id"].as_str().unwrap())),
            "remembered records follow all live records"
        );
    }

    #[tokio::test]
    async fn list_sessions_omits_missing_or_corrupt_metadata_without_hiding_live_records() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        state.sessions.lock().unwrap().insert(
            "live-without-metadata".into(),
            attention_test_handle("live-without-metadata", dir.path()),
        );
        let sessions = listed_sessions(state.clone()).await;
        assert_eq!(
            sessions.len(),
            1,
            "a missing sessions directory is empty metadata"
        );

        let metadata_dir = dir.path().join(".orkworks-test/sessions");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        std::fs::write(
            metadata_dir.join("remembered-corrupt.json"),
            b"not valid json",
        )
        .unwrap();
        std::fs::write(
            metadata_dir.join("live-without-metadata.json"),
            b"not valid json",
        )
        .unwrap();

        let sessions = listed_sessions(state).await;
        assert_eq!(sessions.len(), 1, "corrupt remembered records are omitted");
        assert_eq!(sessions[0]["id"], "live-without-metadata");
    }

    #[tokio::test]
    async fn list_sessions_without_workspace_keeps_live_sessions_and_propagates_live_capacity() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        state
            .providers
            .apply_settings(crate::providers::ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: crate::providers::default_ollama_base_url(),
                providers: vec![crate::providers::ProviderSettingsEntry {
                    id: "codex".into(),
                    enabled: true,
                    fallback_order: 0,
                    model: None,
                    default_state: crate::providers::ProviderCapacityState::Unknown,
                    override_state: None,
                }],
            });
        {
            let workspace = state.workspace.lock().unwrap();
            workspace
                .as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "remembered-only",
                    "Remembered only",
                    dir.path().display().to_string(),
                    "ended",
                    "before",
                    "before",
                ));
        }
        let mut live = attention_test_handle("live-capped", dir.path());
        live.info.harness_id = Some("codex".into());
        live.info.harness = Some("codex".into());
        live.at_usage_limit_latched = true;
        state
            .sessions
            .lock()
            .unwrap()
            .insert("live-capped".into(), live);
        *state.workspace.lock().unwrap() = None;

        let sessions = listed_sessions(state.clone()).await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["id"], "live-capped");
        assert_eq!(sessions[0]["atUsageLimit"], true);
        let codex = state
            .providers
            .get_providers_response()
            .providers
            .into_iter()
            .find(|provider| provider.id == "codex")
            .unwrap();
        assert_eq!(codex.effective_state, "capped");
    }

    #[tokio::test]
    async fn session_projection_owns_live_capacity_and_provider_publication() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        state
            .providers
            .apply_settings(crate::providers::ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: crate::providers::default_ollama_base_url(),
                providers: vec![crate::providers::ProviderSettingsEntry {
                    id: "codex".into(),
                    enabled: true,
                    fallback_order: 0,
                    model: None,
                    default_state: crate::providers::ProviderCapacityState::Unknown,
                    override_state: None,
                }],
            });
        let mut live = attention_test_handle("projected-capacity", dir.path());
        live.info.harness_id = Some("codex".into());
        live.info.harness = Some("codex".into());
        live.at_usage_limit_latched = true;
        state
            .sessions
            .lock()
            .unwrap()
            .insert("projected-capacity".into(), live);
        *state.workspace.lock().unwrap() = None;

        let infos = SessionProjection::new(state.clone()).list();

        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].at_usage_limit, Some(true));
        let codex = state
            .providers
            .get_providers_response()
            .providers
            .into_iter()
            .find(|provider| provider.id == "codex")
            .unwrap();
        assert_eq!(codex.effective_state, "capped");
    }

    #[tokio::test]
    async fn list_sessions_write_back_hook_is_scoped_to_its_registered_state() {
        let hook_state_dir = tempfile::tempdir().unwrap();
        let hook_state = test_app_state_with_workspace(hook_state_dir.path());
        let other_state_dir = tempfile::tempdir().unwrap();
        let other_state = test_app_state_with_workspace(other_state_dir.path());
        let hook_ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let hook_ran_for_callback = hook_ran.clone();

        install_list_sessions_before_write_back_hook(&hook_state, Box::new(move || {
            hook_ran_for_callback.store(true, std::sync::atomic::Ordering::SeqCst);
        }));

        listed_sessions(other_state).await;
        assert!(
            !hook_ran.load(std::sync::atomic::Ordering::SeqCst),
            "a list_sessions call for another AppState must not consume this state’s hook"
        );

        listed_sessions(hook_state).await;
        assert!(hook_ran.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_sessions_rejects_stale_capacity_write_back() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "stale-capacity-write-back".to_string();
        let mut handle = attention_test_handle(&session_id, dir.path());
        handle.info.harness_id = Some("codex".into());
        handle.info.harness = Some("codex".into());
        handle.capacity_check_pending = true;
        handle.info.capacity_check_pending = Some(true);
        handle.output_buffer.push("You've hit your usage limit".into());
        handle.output_lines_seen = 1;
        handle.scan_bytes_seen = 0;
        handle.resume_scan_origin = Some((0, 0));
        {
            let workspace = state.workspace.lock().unwrap();
            let mut metadata = test_session_metadata(
                &session_id,
                "Stale capacity write-back",
                dir.path().display().to_string(),
                "running",
                "before",
                "before",
            );
            metadata.harness = "codex".into();
            workspace.as_ref().unwrap().metadata.write_session(&metadata);
        }
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), handle);

        let stale_state = state.clone();
        let stale_id = session_id.clone();
        install_list_sessions_before_write_back_hook(&state, Box::new(move || {
            let mut sessions = stale_state.sessions.lock().unwrap();
            sessions.get_mut(&stale_id).unwrap().output_lines_seen += 1;
        }));

        let sessions = listed_sessions(state.clone()).await;
        assert_eq!(sessions.len(), 1);
        let handle = state.sessions.lock().unwrap();
        let handle = &handle[&session_id];
        assert!(!handle.at_usage_limit_latched);
        assert!(handle.capacity_check_pending);
        assert!(!handle.pending_capacity_visible_once);
        assert_eq!(handle.info.capacity_check_pending, Some(true));
        assert_eq!(handle.output_lines_seen, 2);
        assert_eq!(handle.scan_bytes_seen, 0);
        assert_eq!(handle.resume_scan_origin, Some((0, 0)));
    }

    #[tokio::test]
    async fn list_sessions_maps_a_poisoned_projection_lock_to_an_empty_500_response() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let poisoned_state = state.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoned_state.projection_lock.lock().unwrap();
            panic!("poison projection lock for join-error coverage");
        })
        .join();

        let response = list_sessions(State(state)).await.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(body.is_empty());
    }
    fn orphan_test_metadata(id: &str, workspace: &std::path::Path) -> metadata::SessionMetadata {
        let workspace = workspace.display().to_string();
        metadata::SessionMetadata {
            id: id.into(),
            label: "Test".into(),
            workspace: workspace.clone(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: workspace,
            status: "running".into(),
            work_phase: "unknown".into(),
            lifecycle_phase: "active".into(),
            lifecycle: "alive".into(),
            attention: None,
            plan_path: None,
            connectivity: "online".into(),
            terminal_outcome: None,
            pending_terminal_status: None,
            observed_status: None,
            ending_observed_status_snapshot: None,
            final_observed_status_snapshot: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "now".into(),
            last_activity: "now".into(),
            last_output_at: None,
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            resume: None,
            resume_options: vec![],
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
            resumed_from: None,
            last_user_input: None,
        }
    }

    /// A daemon restart truly does empty `state.sessions`, so a session
    /// found "running" in persisted metadata with no matching in-memory
    /// handle is genuinely orphaned and must be reconciled to "ended".
    #[tokio::test]
    async fn set_workspace_reconciles_running_session_with_no_live_handle() {
        let home_dir = tempfile::tempdir().unwrap();
        let workspace_dir = tempfile::tempdir().unwrap();
        let _home = FakeHome::set(home_dir.path());

        let state = test_app_state_with_workspace(workspace_dir.path());
        let global_dir = orkworks_global_dir(workspace_dir.path()).unwrap();
        std::fs::create_dir_all(global_dir.join("sessions")).unwrap();
        metadata::MetadataStore::new(&global_dir)
            .write_session(&orphan_test_metadata("orphaned", workspace_dir.path()));

        let response = set_workspace(
            State(state.clone()),
            Json(WorkspaceRequest {
                path: workspace_dir.path().display().to_string(),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let ws_guard = state.workspace.lock().unwrap();
        let reloaded = ws_guard
            .as_ref()
            .unwrap()
            .metadata
            .read_session("orphaned")
            .unwrap();
        assert_eq!(reloaded.status, "ended");
    }

    /// Only the sidecar (`orkworksd`) itself restarting empties
    /// `state.sessions`. When just the Electron app restarts and
    /// reconnects to an already-running sidecar, `set_workspace` runs
    /// again against a sidecar whose in-memory session handles are still
    /// alive — a session with a live handle here must not be reconciled
    /// to "ended" out from under its still-running process.
    #[tokio::test]
    async fn set_workspace_does_not_reconcile_session_with_live_in_memory_handle() {
        let home_dir = tempfile::tempdir().unwrap();
        let workspace_dir = tempfile::tempdir().unwrap();
        let _home = FakeHome::set(home_dir.path());

        let state = test_app_state_with_workspace(workspace_dir.path());
        let global_dir = orkworks_global_dir(workspace_dir.path()).unwrap();
        std::fs::create_dir_all(global_dir.join("sessions")).unwrap();
        metadata::MetadataStore::new(&global_dir)
            .write_session(&orphan_test_metadata("alive", workspace_dir.path()));

        state.sessions.lock().unwrap().insert(
            "alive".into(),
            attention_test_handle("alive", workspace_dir.path()),
        );

        let response = set_workspace(
            State(state.clone()),
            Json(WorkspaceRequest {
                path: workspace_dir.path().display().to_string(),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let ws_guard = state.workspace.lock().unwrap();
        let reloaded = ws_guard
            .as_ref()
            .unwrap()
            .metadata
            .read_session("alive")
            .unwrap();
        assert_eq!(
            reloaded.status, "running",
            "a session with a live in-memory handle must not be reconciled as orphaned"
        );
    }

    #[tokio::test]
    async fn harness_session_report_rejects_invalid_native_id() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_harness_session(
            State(state),
            Path("missing".into()),
            Json(HarnessSessionReportRequest {
                harness_session_id: "bad id".into(),
                source: "test".into(),
                confidence: 0.9,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn harness_session_report_returns_not_found_for_unknown_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_harness_session(
            State(state),
            Path("missing".into()),
            Json(HarnessSessionReportRequest {
                harness_session_id: "native-123".into(),
                source: "test".into(),
                confidence: 0.9,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn harness_session_report_writes_metadata_for_known_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: "known".into(),
                    label: "Known".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "opencode".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
                    attention: None,
                    plan_path: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: None,
                    observed_status: None,
                    ending_observed_status_snapshot: None,
                    final_observed_status_snapshot: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    peon_last_inference: None,
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
                    last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
        }

        let response = report_harness_session(
            State(state.clone()),
            Path("known".into()),
            Json(HarnessSessionReportRequest {
                harness_session_id: "native-123".into(),
                source: "opencode_env".into(),
                confidence: 0.98,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws.as_ref().unwrap().metadata.read_session("known").unwrap();
        assert_eq!(
            updated
                .resume
                .as_ref()
                .and_then(|r| r.harness_session_id.as_deref()),
            Some("native-123"),
        );
    }

    #[tokio::test]
    async fn harness_session_report_keeps_resume_memory_in_sync_for_later_status_updates() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "live-known".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("opencode".into()),
                    harness: Some("opencode".into()),
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    resume_strategy: harness::ResumeStrategy::LatestCwd,
                    resume: Some(resume.clone()),
                    ..test_session_info(
                        session_id.clone(),
                        "Known",
                        dir.path().display().to_string(),
                        "running",
                        "before",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Known".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "opencode".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
                    attention: None,
                    plan_path: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: None,
                    observed_status: None,
                    ending_observed_status_snapshot: None,
                    final_observed_status_snapshot: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    peon_last_inference: None,
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "before".into(),
                    last_activity: "before".into(),
                    last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: Some(resume),
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
        }

        let response = report_harness_session(
            State(state.clone()),
            Path(session_id.clone()),
            Json(HarnessSessionReportRequest {
                harness_session_id: "native-123".into(),
                source: "opencode_env".into(),
                confidence: 0.98,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        set_session_status(&state, &session_id, "ended").await;

        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&session_id)
            .unwrap();
        let updated_resume = updated.resume.unwrap();
        assert_eq!(
            updated_resume.harness_session_id.as_deref(),
            Some("native-123")
        );
        assert_ne!(updated_resume.last_seen_at.as_deref(), Some("before"));
    }

    #[tokio::test]
    async fn resume_session_replaces_unattached_ended_stale_handle() {
        #[cfg(unix)]
        use crate::test_support::make_test_executable;
        use crate::test_support::FakePath;

        let dir = tempfile::tempdir().unwrap();
        let fake_bin_dir = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        let _opencode = {
            let path = fake_bin_dir.path().join("opencode");
            std::fs::write(&path, "#!/bin/sh\nexec sleep 30\n").unwrap();
            make_test_executable(&path);
            path
        };
        #[cfg(windows)]
        let _opencode = {
            let path = fake_bin_dir.path().join("opencode.cmd");
            std::fs::write(
                &path,
                "@echo off\r\n%ComSpec% /c timeout /T 30 /NOBREAK >nul\r\n",
            )
            .unwrap();
            path
        };
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-stale-ended".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        let mut info = test_session_info(
            session_id.clone(),
            "Resume Stale Ended",
            dir.path().display().to_string(),
            "running",
            "before",
        );
        info.harness_id = Some("opencode".into());
        info.harness = Some("opencode".into());
        info.resume_strategy = harness::ResumeStrategy::LatestCwd;
        info.resume = Some(resume.clone());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info,
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let mut metadata = test_session_metadata(
            session_id.clone(),
            "Resume Stale Ended",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.harness = "opencode".into();
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "ended".into();
        metadata.resume = Some(resume);
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.write_session(&metadata);
        }

        let response = resume_session(State(state.clone()), Path(session_id.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(state.sessions.lock().unwrap()[&session_id].resume_in_progress);

        crate::runtime::session_runtime::send_runtime_command(
            &state,
            &session_id,
            crate::runtime::session_runtime::RuntimeCommand::Kill,
        )
        .await
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if !state.sessions.lock().unwrap()[&session_id].resume_in_progress {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("terminal finalization clears the runtime claim");
    }

    #[tokio::test]
    async fn cancelled_resume_after_spawn_keeps_live_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (program, args) = if cfg!(windows) {
            (
                "cmd.exe".to_string(),
                vec!["/C".to_string(), "timeout /T 5 /NOBREAK".to_string()],
            )
        } else {
            (
                "sh".to_string(),
                vec!["-c".to_string(), "exec sleep 5".to_string()],
            )
        };
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                let override_patch = document.overrides.entry("opencode".into()).or_default();
                override_patch.resume = Some(Some(harness::definition::ResumePatch {
                    latest_cwd: Some(Some(harness::CommandTemplate {
                        command: program,
                        args,
                    })),
                    ..Default::default()
                }));
                Ok(())
            })
            .unwrap();

        let session_id = "resume-cancelled-after-spawn".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let mut metadata = test_session_metadata(
            session_id.clone(),
            "Cancelled Resume",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.harness = "opencode".into();
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        metadata.resume = Some(resume);
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&metadata);
            ws.metadata.write_terminal_size(&session_id, 120, 40);
        }

        let (startup_checked, resume_startup) =
            crate::runtime::session_runtime::pause_startup_after_ending_check(session_id.clone());
        let task = tokio::spawn(resume_session(
            State(state.clone()),
            Path(session_id.clone()),
        ));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if state.session_pids.lock().unwrap().contains_key(&session_id) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("resumed child reaches the PTY spawn boundary");
        startup_checked.await.unwrap();

        task.abort();
        assert!(matches!(task.await, Err(error) if error.is_cancelled()));
        let _ = resume_startup.send(());

        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let active =
                    state
                        .sessions
                        .lock()
                        .unwrap()
                        .get(&session_id)
                        .is_some_and(|handle| {
                            handle.info.status == "running"
                                && handle.info.lifecycle_phase == "active"
                        });
                if active {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("detached startup continues after request cancellation");

        let replacement_generation = state.sessions.lock().unwrap()[&session_id]
            .runtime
            .run_generation();
        assert!(state.sessions.lock().unwrap()[&session_id].resume_in_progress);
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        assert_ne!(
            ws.metadata.read_session(&session_id).unwrap().status,
            "ended"
        );
        assert_eq!(ws.metadata.read_terminal_size(&session_id), None);
        drop(ws_guard);

        crate::runtime::session_runtime::send_runtime_command(
            &state,
            &session_id,
            crate::runtime::session_runtime::RuntimeCommand::Kill,
        )
        .await
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&session_id)
                    .is_some_and(|handle| !handle.resume_in_progress)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("terminal finalization releases the replacement claim");
        assert_eq!(
            state.sessions.lock().unwrap()[&session_id]
                .runtime
                .run_generation(),
            replacement_generation
        );
    }

    #[tokio::test]
    async fn delete_during_startup_finalizes_same_generation() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (program, args) = if cfg!(windows) {
            (
                "cmd.exe".to_string(),
                vec!["/C".to_string(), "timeout /T 30 /NOBREAK".to_string()],
            )
        } else {
            (
                "sh".to_string(),
                vec!["-c".to_string(), "exec sleep 30".to_string()],
            )
        };
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                let override_patch = document.overrides.entry("opencode".into()).or_default();
                override_patch.resume = Some(Some(harness::definition::ResumePatch {
                    latest_cwd: Some(Some(harness::CommandTemplate {
                        command: program,
                        args,
                    })),
                    ..Default::default()
                }));
                Ok(())
            })
            .unwrap();

        let session_id = "delete-during-startup".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let mut metadata = test_session_metadata(
            session_id.clone(),
            "Delete During Startup",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.harness = "opencode".into();
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        metadata.resume = Some(resume);
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let (checked_rx, resume_tx) =
            crate::runtime::session_runtime::pause_startup_after_ending_check(session_id.clone());
        let resume_task = tokio::spawn(resume_session(
            State(state.clone()),
            Path(session_id.clone()),
        ));
        tokio::time::timeout(std::time::Duration::from_secs(5), checked_rx)
            .await
            .expect("startup reaches the post-check transition gap")
            .expect("startup test hook remains installed");

        let response = delete_session(State(state.clone()), Path(session_id.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        resume_tx
            .send(())
            .expect("startup is waiting to attempt the running transition");

        let response = tokio::time::timeout(std::time::Duration::from_secs(5), resume_task)
            .await
            .expect("startup request returns after its generation is finalized")
            .expect("startup task does not panic");
        assert_eq!(
            response.into_response().status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );

        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let finalized =
                    state
                        .sessions
                        .lock()
                        .unwrap()
                        .get(&session_id)
                        .is_some_and(|handle| {
                            handle.info.status == "killed"
                                && handle.info.lifecycle_phase == "ended"
                                && !handle.resume_in_progress
                        });
                if finalized {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("deleted startup generation is finalized");

        let metadata = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&session_id)
            .unwrap();
        assert_eq!(metadata.status, "killed");
        assert_eq!(metadata.lifecycle_phase, "ended");
        assert!(!state.session_pids.lock().unwrap().contains_key(&session_id));
        assert!(!state
            .peon
            .last_output
            .read()
            .unwrap()
            .contains_key(&session_id));
    }

    #[tokio::test]
    async fn resume_session_startup_failure_eventually_clears_runtime_claim() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                let override_patch = document.overrides.entry("opencode".into()).or_default();
                override_patch.resume = Some(Some(harness::definition::ResumePatch {
                    latest_cwd: Some(Some(harness::CommandTemplate {
                        command: "orkworks-resume-command-that-does-not-exist".into(),
                        args: vec![],
                    })),
                    ..Default::default()
                }));
                Ok(())
            })
            .unwrap();
        let session_id = "resume-startup-failure".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let mut metadata = test_session_metadata(
            session_id.clone(),
            "Resume Startup Failure",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.harness = "opencode".into();
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        metadata.resume = Some(resume);
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let response = resume_session(State(state.clone()), Path(session_id.clone()))
            .await
            .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let claim_cleared = state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&session_id)
                    .is_some_and(|handle| !handle.resume_in_progress);
                if claim_cleared {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("startup-failure finalization clears the runtime claim");
        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions[&session_id].info.status, "error");
        assert_eq!(sessions[&session_id].info.lifecycle_phase, "ended");
        assert!(!sessions[&session_id].resume_in_progress);
    }

    #[test]
    fn resume_admission_installs_one_claimed_runtime_after_both_callers_observe_ended_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-no-handle-concurrent".to_string();
        let mut metadata = test_session_metadata(
            &session_id,
            "Concurrent Resume",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let mut callers = Vec::new();

        for _ in 0..2 {
            let state = state.clone();
            let session_id = session_id.clone();
            let cwd = dir.path().to_path_buf();
            let barrier = barrier.clone();
            callers.push(std::thread::spawn(move || {
                // Both request paths have already read the same ended metadata
                // before either is allowed to enter atomic admission.
                let metadata_ended = true;
                let replacement = attention_test_handle(&session_id, &cwd);
                barrier.wait();
                try_install_claimed_resume_handle(
                    &state,
                    &session_id,
                    replacement,
                    metadata_ended,
                    None,
                )
            }));
        }

        barrier.wait();
        let results = callers
            .into_iter()
            .map(|caller| caller.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains_key(&session_id));
        assert!(sessions[&session_id].resume_in_progress);
    }

    #[test]
    fn resume_admission_rejects_a_stale_observation_after_the_first_claim_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-stale-observation";

        let mut metadata = test_session_metadata(
            session_id,
            "Stale Resume",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let mut old_handle = attention_test_handle(session_id, dir.path());
        old_handle.info.status = "ended".into();
        old_handle.info.lifecycle_phase = "ended".into();
        old_handle.info.lifecycle = "dead".into();
        let observed_generation = old_handle.runtime.run_generation();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), old_handle);

        let first = try_install_claimed_resume_handle(
            &state,
            session_id,
            attention_test_handle(session_id, dir.path()),
            true,
            Some(observed_generation),
        )
        .unwrap();
        first.commit();

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.resume_in_progress = false;
            handle.info.status = "ended".into();
            handle.info.lifecycle_phase = "ended".into();
            handle.info.lifecycle = "dead".into();
        }

        let second = try_install_claimed_resume_handle(
            &state,
            session_id,
            attention_test_handle(session_id, dir.path()),
            true,
            Some(observed_generation),
        );

        assert!(second.is_err());
    }

    #[tokio::test]
    async fn late_old_runtime_exit_is_a_noop_after_resume_replaces_its_generation() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-generation-replacement";
        let mut metadata = test_session_metadata(
            session_id,
            "Replacement",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let old_handle = attention_test_handle(session_id, dir.path());
        let old_generation = old_handle.runtime.run_generation();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), old_handle);

        let replacement = attention_test_handle(session_id, dir.path());
        let admission = try_install_claimed_resume_handle(
            &state,
            session_id,
            replacement,
            true,
            Some(old_generation),
        )
        .unwrap();
        let replacement_generation = admission.generation();
        admission.commit();
        assert!(replacement_generation > old_generation);

        let mut metadata = test_session_metadata(
            session_id,
            "Replacement",
            dir.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        state
            .session_pids
            .lock()
            .unwrap()
            .insert(session_id.to_string(), 4242);
        state
            .peon
            .last_output
            .write()
            .unwrap()
            .insert(session_id.to_string(), tokio::time::Instant::now());
        state
            .peon
            .last_inference
            .write()
            .unwrap()
            .insert(session_id.to_string(), "replacement inference".into());
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert(session_id.to_string(), "replacement input".into());

        assert!(
            !crate::runtime::session_runtime::handle_runtime_exit(
                &state,
                session_id,
                old_generation,
                "ended",
            )
            .await
        );
        tokio::task::yield_now().await;

        let sessions = state.sessions.lock().unwrap();
        let replacement = &sessions[session_id];
        assert_eq!(replacement.runtime.run_generation(), replacement_generation);
        assert_eq!(replacement.info.status, "running");
        assert_eq!(replacement.info.lifecycle_phase, "active");
        assert!(replacement.resume_in_progress);
        drop(sessions);
        assert_eq!(state.session_pids.lock().unwrap()[session_id], 4242);
        assert!(state
            .peon
            .last_output
            .read()
            .unwrap()
            .contains_key(session_id));
        assert_eq!(
            state.peon.last_inference.read().unwrap()[session_id],
            "replacement inference"
        );
        assert_eq!(
            state.peon.input_buf.read().unwrap()[session_id],
            "replacement input"
        );
        let persisted = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(session_id)
            .unwrap();
        assert_eq!(persisted.status, "running");
        assert_eq!(persisted.lifecycle_phase, "active");

        // A finalizer that was already scheduled by the old generation must
        // also be harmless if the replacement independently enters ending.
        {
            let mut sessions = state.sessions.lock().unwrap();
            let replacement = sessions.get_mut(session_id).unwrap();
            replacement.info.lifecycle_phase = "ending".into();
            replacement.info.lifecycle = "stopping".into();
        }
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            let mut ending = ws.metadata.read_session(session_id).unwrap();
            ending.lifecycle_phase = "ending".into();
            ending.lifecycle = "stopping".into();
            ending.pending_terminal_status = Some("killed".into());
            ws.metadata.write_session(&ending);
        }
        crate::runtime::terminal_runtime::finalize_session_ending(
            state.clone(),
            session_id.to_string(),
            old_generation,
            "ended".into(),
        )
        .await;
        let sessions = state.sessions.lock().unwrap();
        let replacement = &sessions[session_id];
        assert_eq!(replacement.runtime.run_generation(), replacement_generation);
        assert_eq!(replacement.info.lifecycle_phase, "ending");
        assert!(replacement.resume_in_progress);
        drop(sessions);
        let persisted = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(session_id)
            .unwrap();
        assert_eq!(persisted.lifecycle_phase, "ending");
        assert_eq!(persisted.pending_terminal_status.as_deref(), Some("killed"));
    }

    #[test]
    fn cancelled_resume_admission_restores_prior_runtime_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-cancelled-admission";
        let mut old_handle = attention_test_handle(session_id, dir.path());
        old_handle.info.status = "ended".into();
        old_handle.info.lifecycle_phase = "ended".into();
        old_handle.info.lifecycle = "dead".into();
        let old_generation = old_handle.runtime.run_generation();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), old_handle);

        let mut metadata = test_session_metadata(
            session_id,
            "Prior Runtime",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        ws.metadata.write_session(&metadata);
        ws.metadata.write_terminal_size(session_id, 120, 40);
        drop(ws_guard);

        let replacement = attention_test_handle(session_id, dir.path());
        let mut admission = try_install_claimed_resume_handle(
            &state,
            session_id,
            replacement,
            true,
            Some(old_generation),
        )
        .unwrap();
        admission.arm_rollback(dir.path().to_path_buf(), metadata.clone(), Some((120, 40)));
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            let mut creating = metadata.clone();
            creating.status = "creating".into();
            creating.lifecycle_phase = "creating".into();
            ws.metadata.write_session(&creating);
            ws.metadata.clear_terminal_size(session_id);
        }

        // Dropping the request future drops its admission guard.
        drop(admission);

        let sessions = state.sessions.lock().unwrap();
        let restored = &sessions[session_id];
        assert_eq!(restored.runtime.run_generation(), old_generation);
        assert_eq!(restored.info.lifecycle_phase, "ended");
        assert!(!restored.resume_in_progress);
        drop(sessions);
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        let restored_metadata = ws.metadata.read_session(session_id).unwrap();
        assert_eq!(restored_metadata.status, "ended");
        assert_eq!(restored_metadata.lifecycle_phase, "ended");
        assert_eq!(ws.metadata.read_terminal_size(session_id), Some((120, 40)));
    }

    #[test]
    fn cancelled_resume_does_not_overwrite_newer_generation_metadata_between_rollback_stages() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-cancelled-interleaving";
        let mut old_handle = attention_test_handle(session_id, dir.path());
        old_handle.info.status = "ended".into();
        old_handle.info.lifecycle_phase = "ended".into();
        old_handle.info.lifecycle = "dead".into();
        let old_generation = old_handle.runtime.run_generation();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), old_handle);

        let mut old_metadata = test_session_metadata(
            session_id,
            "Prior Runtime",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        old_metadata.lifecycle_phase = "ended".into();
        old_metadata.lifecycle = "dead".into();
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&old_metadata);
            ws.metadata.write_terminal_size(session_id, 120, 40);
        }

        let replacement = attention_test_handle(session_id, dir.path());
        let mut cancelled_admission = try_install_claimed_resume_handle(
            &state,
            session_id,
            replacement,
            true,
            Some(old_generation),
        )
        .unwrap();
        cancelled_admission.arm_rollback(
            dir.path().to_path_buf(),
            old_metadata.clone(),
            Some((120, 40)),
        );
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            let mut creating = old_metadata.clone();
            creating.status = "creating".into();
            creating.lifecycle_phase = "creating".into();
            creating.lifecycle = "creating".into();
            ws.metadata.write_session(&creating);
            ws.metadata.clear_terminal_size(session_id);
        }

        // Hold the workspace lock so cancellation must pause after restoring
        // the prior handle and before attempting its persisted rollback.
        let ws_guard = state.workspace.lock().unwrap();
        let drop_thread = std::thread::spawn(move || drop(cancelled_admission));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let restored = state
                .sessions
                .lock()
                .unwrap()
                .get(session_id)
                .is_some_and(|handle| handle.runtime.run_generation() == old_generation);
            if restored {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "cancelled admission did not restore its prior handle"
            );
            std::thread::yield_now();
        }

        let newer_handle = attention_test_handle(session_id, dir.path());
        let newer_admission = try_install_claimed_resume_handle(
            &state,
            session_id,
            newer_handle,
            true,
            Some(old_generation),
        )
        .unwrap();
        let newer_generation = newer_admission.generation();
        let ws = ws_guard.as_ref().unwrap();
        let mut newer_metadata = old_metadata.clone();
        newer_metadata.label = "Newer Runtime".into();
        newer_metadata.status = "running".into();
        newer_metadata.lifecycle_phase = "active".into();
        newer_metadata.lifecycle = "alive".into();
        ws.metadata.write_session(&newer_metadata);
        ws.metadata.write_terminal_size(session_id, 150, 50);
        newer_admission.commit();
        drop(ws_guard);
        drop_thread.join().unwrap();

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[session_id].runtime.run_generation(),
            newer_generation
        );
        drop(sessions);
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        let persisted = ws.metadata.read_session(session_id).unwrap();
        assert_eq!(persisted.label, "Newer Runtime");
        assert_eq!(persisted.status, "running");
        assert_eq!(persisted.lifecycle_phase, "active");
        assert_eq!(ws.metadata.read_terminal_size(session_id), Some((150, 50)));
    }

    #[test]
    fn resume_admission_rejects_active_metadata_without_a_session_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-no-handle-active-metadata";
        let replacement = attention_test_handle(session_id, dir.path());

        let result =
            try_install_claimed_resume_handle(&state, session_id, replacement, false, None);

        assert!(result.is_err());
        assert!(!state.sessions.lock().unwrap().contains_key(session_id));
    }

    #[test]
    fn resume_admission_rejects_tracked_pid_without_a_session_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-no-handle-tracked-pid";
        state
            .session_pids
            .lock()
            .unwrap()
            .insert(session_id.to_string(), 42);
        let replacement = attention_test_handle(session_id, dir.path());

        let result = try_install_claimed_resume_handle(&state, session_id, replacement, true, None);

        assert!(result.is_err());
        assert!(!state.sessions.lock().unwrap().contains_key(session_id));
    }

    #[tokio::test]
    async fn resume_admission_waits_for_ending_handle_finalization_before_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-ending-finalization";
        let mut ending_handle = attention_test_handle(session_id, dir.path());
        ending_handle.info.lifecycle_phase = "ending".into();
        ending_handle.info.lifecycle = "stopping".into();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), ending_handle);
        let mut metadata = test_session_metadata(
            session_id,
            "Resume Ending Finalization",
            dir.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ending".into();
        metadata.lifecycle = "stopping".into();
        metadata.pending_terminal_status = Some("ended".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        // The request-side metadata snapshot may already say `ended` while
        // the old finalizer is waiting to update the in-memory handle.
        let generation = state.sessions.lock().unwrap()[session_id]
            .runtime
            .run_generation();
        let premature_replacement = attention_test_handle(session_id, dir.path());
        let premature_result = try_install_claimed_resume_handle(
            &state,
            session_id,
            premature_replacement,
            true,
            Some(generation),
        );

        assert!(premature_result.is_err());
        assert_eq!(
            state.sessions.lock().unwrap()[session_id]
                .info
                .lifecycle_phase,
            "ending"
        );

        crate::runtime::terminal_runtime::finalize_session_ending(
            state.clone(),
            session_id.to_string(),
            generation,
            "ended".into(),
        )
        .await;
        assert_eq!(
            state.sessions.lock().unwrap()[session_id]
                .info
                .lifecycle_phase,
            "ended"
        );

        let replacement = attention_test_handle(session_id, dir.path());
        let result = try_install_claimed_resume_handle(
            &state,
            session_id,
            replacement,
            true,
            Some(generation),
        );

        assert!(result.is_ok());
        let sessions = state.sessions.lock().unwrap();
        assert!(sessions[session_id].resume_in_progress);
        assert_ne!(sessions[session_id].info.lifecycle_phase, "ended");
    }

    #[tokio::test]
    async fn resume_session_rejects_attached_live_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-attached".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("opencode".into()),
                    harness: Some("opencode".into()),
                    resume_strategy: harness::ResumeStrategy::LatestCwd,
                    resume: Some(resume.clone()),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Attached",
                        dir.path().display().to_string(),
                        "running",
                        "before",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: true,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Resume Attached".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "opencode".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
                    attention: None,
                    plan_path: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: None,
                    observed_status: None,
                    ending_observed_status_snapshot: None,
                    final_observed_status_snapshot: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    peon_last_inference: None,
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "before".into(),
                    last_activity: "before".into(),
                    last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: Some(resume),
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
        }

        let response = resume_session(State(state), Path(session_id))
            .await
            .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn resume_session_rejects_detached_live_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-detached-live".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("opencode".into()),
                    harness: Some("opencode".into()),
                    resume_strategy: harness::ResumeStrategy::LatestCwd,
                    resume: Some(resume.clone()),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Detached Live",
                        dir.path().display().to_string(),
                        "running",
                        "before",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Resume Detached Live".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "opencode".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
                    attention: None,
                    plan_path: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: None,
                    observed_status: None,
                    ending_observed_status_snapshot: None,
                    final_observed_status_snapshot: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    peon_last_inference: None,
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "before".into(),
                    last_activity: "before".into(),
                    last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: Some(resume),
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
        }

        let response = resume_session(State(state), Path(session_id))
            .await
            .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn report_attention_rejects_invalid_status() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_attention(
            State(state),
            Path("missing".into()),
            Json(AttentionReportRequest {
                status: "not_a_real_status".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        assert!(axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn report_attention_rejects_malformed_observed_at() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_attention(
            State(state),
            Path("missing".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: Some("not-a-timestamp".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn report_attention_returns_not_found_for_unknown_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_attention(
            State(state),
            Path("missing".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn report_attention_writes_metadata_for_known_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: "attention-known".into(),
                    label: "Known".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "claude-code".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
                    attention: None,
                    plan_path: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: None,
                    observed_status: None,
                    ending_observed_status_snapshot: None,
                    final_observed_status_snapshot: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    peon_last_inference: None,
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
                    last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
        }

        let response = report_attention(
            State(state.clone()),
            Path("attention-known".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("attention-known")
            .unwrap();
        assert_eq!(
            updated.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(updated.metadata_source, "agent");
    }

    #[tokio::test]
    async fn report_attention_stores_reported_cwd_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.write_session(
                &crate::test_support::test_session_metadata(
                    "cwd-report",
                    "CwdReport",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ),
            );
        }

        let response = report_attention(
            State(state.clone()),
            Path("cwd-report".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: Some("/harness-reported/worktree".into()),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            state
                .peon
                .reported_cwd
                .read()
                .unwrap()
                .get("cwd-report")
                .map(String::as_str),
            Some("/harness-reported/worktree")
        );
    }

    #[tokio::test]
    async fn report_attention_does_not_clobber_reported_cwd_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.write_session(
                &crate::test_support::test_session_metadata(
                    "cwd-sticky",
                    "CwdSticky",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ),
            );
        }
        state
            .peon
            .reported_cwd
            .write()
            .unwrap()
            .insert("cwd-sticky".into(), "/previously-reported".into());

        let response = report_attention(
            State(state.clone()),
            Path("cwd-sticky".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            state
                .peon
                .reported_cwd
                .read()
                .unwrap()
                .get("cwd-sticky")
                .map(String::as_str),
            Some("/previously-reported")
        );
    }

    #[tokio::test]
    async fn report_attention_ignores_stale_observed_at_before_side_effects() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "attention-stale-observed-at";
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                id,
                "Known",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            meta.observed_status = Some("working".into());
            meta.attention = Some("working".into());
            meta.metadata_source = "process".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        let mut handle = attention_test_handle(id, dir.path());
        handle.info.observed_status = Some("working".into());
        handle.info.attention = Some("working".into());
        handle.info.metadata_source = Some("process".into());
        handle.runtime.accepted_input_at = Some(
            crate::workspace_runtime::parse_hook_observed_at("2026-07-21T08:00:01.000000Z")
                .unwrap(),
        );
        state.sessions.lock().unwrap().insert(id.into(), handle);
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert(id.into(), "y".into());

        let response = report_attention(
            State(state.clone()),
            Path(id.into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: Some("old prompt".into()),
                plan_path: Default::default(),
                observed_at: Some("2026-07-21T08:00:00.000000Z".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let meta = ws.as_ref().unwrap().metadata.read_session(id).unwrap();
        assert_eq!(meta.observed_status.as_deref(), Some("working"));
        drop(ws);
        assert_eq!(
            state.sessions.lock().unwrap()[id].info.attention.as_deref(),
            Some("working")
        );
        assert_eq!(
            state
                .peon
                .input_buf
                .read()
                .unwrap()
                .get(id)
                .map(String::as_str),
            Some("y")
        );
    }

    #[tokio::test]
    async fn report_attention_ignores_an_out_of_order_hook_event() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "attention-hook-order";
        let mut handle = attention_test_handle(id, dir.path());
        handle.active_work_hook = true;
        state.sessions.lock().unwrap().insert(id.into(), handle);
        let ws = state.workspace.lock().unwrap();
        let mut meta = test_session_metadata(
            id,
            "Known",
            dir.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        meta.lifecycle = "alive".into();
        ws.as_ref().unwrap().metadata.write_session(&meta);
        drop(ws);
        for (status, observed_at) in [
            ("waiting_for_input", "2026-08-01T08:00:02.000000Z"),
            ("working", "2026-08-01T08:00:01.000000Z"),
        ] {
            let response = report_attention(
                State(state.clone()),
                Path(id.into()),
                Json(AttentionReportRequest {
                    status: status.into(),
                    message: None,
                    plan_path: Default::default(),
                    observed_at: Some(observed_at.into()),
                    cwd: None,
                }),
            )
            .await
            .into_response();
            assert_eq!(response.status(), axum::http::StatusCode::OK);
        }
        assert_eq!(
            state.sessions.lock().unwrap()[id]
                .info
                .observed_status
                .as_deref(),
            Some("waiting_for_input")
        );
    }

    #[tokio::test]
    async fn claude_working_hook_clears_shared_stale_capacity_latches() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let source_id = "claude-working-source";
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                source_id,
                "Claude source",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.harness = "claude-code".into();
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        for (id, lines, bytes) in [(source_id, 7, 11), ("claude-working-peer", 3, 5)] {
            let mut handle = attention_test_handle(id, dir.path());
            handle.active_work_hook = true;
            handle.info.harness_id = Some("claude-code".into());
            handle.info.harness = Some("claude-code".into());
            handle.info.last_output_at = Some("2026-08-01T08:00:00.000000Z".into());
            handle.at_usage_limit_latched = true;
            handle.output_lines_seen = lines;
            handle.scan_bytes_seen = bytes;
            state.sessions.lock().unwrap().insert(id.into(), handle);
        }
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut("claude-working-peer")
            .unwrap()
            .info
            .last_output_at = Some("2026-08-01T08:00:02.000000Z".into());

        let response = report_attention(
            State(state.clone()),
            Path(source_id.into()),
            Json(AttentionReportRequest {
                status: "working".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: Some("2026-08-01T08:00:01.000000Z".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let sessions = state.sessions.lock().unwrap();
        assert!(!sessions[source_id].at_usage_limit_latched);
        assert_eq!(sessions[source_id].resume_scan_origin, Some((7, 11)));
        assert!(!sessions["claude-working-peer"].at_usage_limit_latched);
        assert_eq!(
            sessions["claude-working-peer"].resume_scan_origin,
            Some((3, 5))
        );
        drop(sessions);

        {
            let mut sessions = state.sessions.lock().unwrap();
            sessions.get_mut(source_id).unwrap().at_usage_limit_latched = true;
            let peer = sessions.get_mut("claude-working-peer").unwrap();
            peer.at_usage_limit_latched = false;
            peer.runtime.usage_limit_latched_at = Some(
                crate::workspace_runtime::parse_hook_observed_at("2026-08-01T08:00:05.000000Z")
                    .unwrap(),
            );
        }
        let response = report_attention(
            State(state.clone()),
            Path(source_id.into()),
            Json(AttentionReportRequest {
                status: "working".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: Some("2026-08-01T08:00:04.000000Z".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(!state.sessions.lock().unwrap()[source_id].at_usage_limit_latched);
    }

    #[tokio::test]
    async fn claude_working_hook_does_not_clear_a_newer_capacity_signal() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "claude-working-after-cap";
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                id,
                "Claude source",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.harness = "claude-code".into();
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        let mut handle = attention_test_handle(id, dir.path());
        handle.active_work_hook = true;
        handle.info.harness_id = Some("claude-code".into());
        handle.info.harness = Some("claude-code".into());
        handle.at_usage_limit_latched = true;
        handle.runtime.usage_limit_latched_at = Some(
            crate::workspace_runtime::parse_hook_observed_at("2026-08-01T08:00:02.000000Z")
                .unwrap(),
        );
        state.sessions.lock().unwrap().insert(id.into(), handle);

        let response = report_attention(
            State(state.clone()),
            Path(id.into()),
            Json(AttentionReportRequest {
                status: "working".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: Some("2026-08-01T08:00:01.000000Z".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(state.sessions.lock().unwrap()[id].at_usage_limit_latched);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn report_attention_metadata_io_does_not_block_tokio_worker() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "attention-nonblocking",
                "Known",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let (locked_tx, locked_rx) = std::sync::mpsc::sync_channel(1);
        let locked_state = state.clone();
        let locker = std::thread::spawn(move || {
            let _guard = locked_state.workspace.lock().unwrap();
            locked_tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(250));
        });
        locked_rx.recv().unwrap();

        let request_state = state.clone();
        let response = tokio::spawn(async move {
            report_attention(
                State(request_state),
                Path("attention-nonblocking".into()),
                Json(AttentionReportRequest {
                    status: "blocked".into(),
                    message: Some("Waiting".into()),
                    plan_path: Default::default(),
                    observed_at: None,
                    cwd: None,
                }),
            )
            .await
            .into_response()
        });
        let started = std::time::Instant::now();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(
            started.elapsed() < std::time::Duration::from_millis(100),
            "Tokio worker was blocked for {:?}",
            started.elapsed()
        );

        assert_eq!(response.await.unwrap().status(), axum::http::StatusCode::OK);
        locker.join().unwrap();
    }

    // The "concurrent calls keep live/persisted in agreement" guarantee for
    // both report_attention and apply_debug_attention now lives in a single
    // test on the shared apply_attention_signal module they both call
    // through: runtime::observed_status::tests::
    // apply_attention_signal_keeps_stores_in_agreement_under_concurrent_calls
    // (see ADR 0027).

    #[tokio::test]
    async fn apply_debug_attention_rejects_invalid_attention() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = apply_debug_attention(
            State(state),
            Path("missing".into()),
            Json(DebugAttentionRequest {
                attention: "not_a_real_value".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn apply_debug_attention_returns_not_found_for_unknown_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = apply_debug_attention(
            State(state),
            Path("missing".into()),
            Json(DebugAttentionRequest {
                attention: "working".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn apply_debug_attention_rejects_when_lifecycle_not_alive() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            // test_session_metadata defaults to a dead/ended session.
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "debug-dead",
                    "Dead",
                    dir.path().display().to_string(),
                    "ended",
                    "now",
                    "now",
                ));
        }

        let response = apply_debug_attention(
            State(state),
            Path("debug-dead".into()),
            Json(DebugAttentionRequest {
                attention: "working".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn apply_debug_attention_writes_debug_source_and_confidence() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "debug-alive",
                "Alive",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let response = apply_debug_attention(
            State(state.clone()),
            Path("debug-alive".into()),
            Json(DebugAttentionRequest {
                attention: "blocked".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("debug-alive")
            .unwrap();
        assert_eq!(updated.observed_status.as_deref(), Some("blocked"));
        assert_eq!(updated.attention.as_deref(), Some("blocked"));
        assert_eq!(updated.metadata_source, "debug");
        assert_eq!(updated.metadata_confidence, 0.0);
    }

    #[tokio::test]
    async fn apply_debug_attention_cannot_clobber_live_agent_signal() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "debug-vs-agent",
                "Alive",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            meta.metadata_source = "agent".into();
            meta.observed_status = Some("working".into());
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let response = apply_debug_attention(
            State(state.clone()),
            Path("debug-vs-agent".into()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        // Ignored (not rejected) mirrors report_attention's own handling of an
        // unwritable target: the request is well-formed, it just didn't land.
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("debug-vs-agent")
            .unwrap();
        assert_eq!(
            updated.observed_status.as_deref(),
            Some("working"),
            "a live agent signal must survive a debug injection"
        );
        assert_eq!(updated.metadata_source, "agent");
    }

    #[tokio::test]
    async fn apply_debug_attention_maps_needs_you_to_waiting_for_input_observed_status() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "debug-needs-you",
                "Alive",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let response = apply_debug_attention(
            State(state.clone()),
            Path("debug-needs-you".into()),
            Json(DebugAttentionRequest {
                attention: "needs_you".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("debug-needs-you")
            .unwrap();
        assert_eq!(
            updated.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(updated.attention.as_deref(), Some("needs_you"));
    }

    #[tokio::test]
    async fn apply_debug_attention_injected_value_is_reclaimed_by_next_real_signal() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "debug-reclaim",
                "Alive",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let _ = apply_debug_attention(
            State(state.clone()),
            Path("debug-reclaim".into()),
            Json(DebugAttentionRequest {
                attention: "failed".into(),
                message: None,
            }),
        )
        .await
        .into_response();
        {
            let ws = state.workspace.lock().unwrap();
            let injected = ws
                .as_ref()
                .unwrap()
                .metadata
                .read_session("debug-reclaim")
                .unwrap();
            assert_eq!(injected.metadata_source, "debug");
        }

        // Any real attention source is unconditionally accepted over "debug",
        // since debug is the lowest documented priority tier. "blocked" is
        // accepted regardless of active_work_hook capability, unlike "working".
        let _ = report_attention(
            State(state.clone()),
            Path("debug-reclaim".into()),
            Json(AttentionReportRequest {
                status: "blocked".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        let ws = state.workspace.lock().unwrap();
        let reclaimed = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("debug-reclaim")
            .unwrap();
        assert_eq!(reclaimed.metadata_source, "agent");
        assert_eq!(reclaimed.observed_status.as_deref(), Some("blocked"));
    }

    #[tokio::test]
    async fn apply_debug_attention_capped_message_lands_in_usage_limit_reset_hint_not_summary() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: Some("resets in 2h".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&created_id)
            .unwrap();
        assert_eq!(
            updated.summary, None,
            "capped hint must not land in the generic summary field"
        );
        drop(ws);

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[&created_id].info.usage_limit_reset_hint.as_deref(),
            Some("resets in 2h"),
        );
        drop(sessions);

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    /// Regression test for a race apply_debug_attention used to have: the
    /// usage_limit_reset_hint write was a separate, later critical section
    /// from the attention-field write, so a concurrent call could leave the
    /// two fields disagreeing (see ADR 0027 / PR #208 review). Both calls
    /// race through spawn_blocking; whichever lands last must leave
    /// usage_limit_reset_hint consistent with its own attention, not a mix
    /// of the two calls' state.
    #[tokio::test]
    async fn apply_debug_attention_keeps_reset_hint_consistent_with_attention_under_concurrent_calls(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "debug-concurrent-reset-hint";
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                id,
                "Known",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, dir.path()));

        let capped = apply_debug_attention(
            State(state.clone()),
            Path(id.into()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: Some("resets in 2h".into()),
            }),
        );
        let working = apply_debug_attention(
            State(state.clone()),
            Path(id.into()),
            Json(DebugAttentionRequest {
                attention: "working".into(),
                message: None,
            }),
        );
        let (capped_response, working_response) = tokio::join!(capped, working);
        assert_eq!(
            capped_response.into_response().status(),
            axum::http::StatusCode::OK
        );
        assert_eq!(
            working_response.into_response().status(),
            axum::http::StatusCode::OK
        );

        let sessions = state.sessions.lock().unwrap();
        let info = &sessions.get(id).unwrap().info;
        if info.attention.as_deref() == Some("capped") {
            assert!(
                info.usage_limit_reset_hint.is_some(),
                "attention is capped but reset hint is missing"
            );
        } else {
            assert!(
                info.usage_limit_reset_hint.is_none(),
                "attention is {:?} but a stale capped reset hint survived",
                info.attention
            );
        }
    }

    #[tokio::test]
    async fn debug_injected_capped_hint_survives_list_sessions_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: Some("resets in 2h".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // The bug this guards against: list_sessions used to unconditionally
        // recompute usage_limit_reset_hint from live terminal-output
        // scanning, discarding the debug-injected value on the very next
        // poll since a generic-shell session has no real usage-limit text
        // in its terminal output to detect.
        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(created_id.as_str()))
            .expect("created session should be listed");
        assert_eq!(
            session.get("usageLimitResetHint").and_then(|v| v.as_str()),
            Some("resets in 2h"),
            "debug-injected capped hint must survive a list_sessions refresh"
        );

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn debug_injection_off_capped_clears_stale_reset_hint() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: Some("resets in 2h".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // The bug this guards against: apply_debug_attention only ever SET
        // usage_limit_reset_hint on a capped injection; it never cleared it
        // on a later non-capped injection, so the stale hint survived
        // indefinitely -- and would keep feeding the cross-session harness
        // reset-hint propagation in list_sessions even after this session
        // moved off "capped".
        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "working".into(),
                message: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[&created_id].info.usage_limit_reset_hint, None,
            "moving debug attention off capped must clear the stale reset hint",
        );
        drop(sessions);

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(created_id.as_str()))
            .expect("created session should be listed");
        assert_eq!(
            session.get("usageLimitResetHint"),
            None,
            "list_sessions must not surface a stale hint once attention is off capped"
        );

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn apply_debug_attention_capped_without_message_does_not_clear_existing_hint() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        // A real capacity hint is already present, as if genuine harness capacity
        // detection had set it before the debug picker was ever touched.
        {
            let mut sessions = state.sessions.lock().unwrap();
            sessions
                .get_mut(&created_id)
                .unwrap()
                .info
                .usage_limit_reset_hint = Some("resets in 45m".into());
        }

        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[&created_id].info.usage_limit_reset_hint.as_deref(),
            Some("resets in 45m"),
            "a message-less capped injection must not wipe an existing real hint",
        );
        drop(sessions);

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn created_generic_session_does_not_advertise_active_work_hook() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(!state.sessions.lock().unwrap()[&created_id].active_work_hook);

        let response = report_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(AttentionReportRequest {
                status: "thinking".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn creation_with_descriptive_initial_prompt_queues_topic_inference() {
        // The initial prompt bypasses terminal_runtime's keystroke-based label
        // seeding entirely (it's written straight to the PTY), so session
        // creation must seed the synchronous fallback and queue Peon's
        // InputLabel pass (ADR 0029) rather than leaving the label stuck on
        // its placeholder.
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: Some("fix the login redirect bug".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let created_id = body["id"].as_str().unwrap().to_owned();
        assert_eq!(body["label"], "fix the login redirect bug");

        assert_eq!(
            state.peon.label_hint.read().unwrap().get(&created_id),
            Some(&crate::LabelHint {
                text: "fix the login redirect bug".into(),
                epoch: 0,
            })
        );
        assert!(state
            .peon
            .label_pending
            .read()
            .unwrap()
            .contains(&created_id));

        let meta = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&created_id)
            .unwrap();
        assert_eq!(meta.label, "fix the login redirect bug");
    }

    #[tokio::test]
    async fn creation_keeps_full_initial_hint_when_pr_is_after_display_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let prompt = format!("review {} PR #249", "important changes ".repeat(6));
        let display: String = prompt.chars().take(100).collect();
        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: Some(prompt.clone()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        let created_id = body["id"].as_str().unwrap();

        assert_eq!(body["label"], display);
        assert_eq!(
            state.peon.label_hint.read().unwrap().get(created_id),
            Some(&crate::LabelHint {
                text: prompt.clone(),
                epoch: 0,
            })
        );
    }

    #[tokio::test]
    async fn creation_with_non_descriptive_initial_prompt_does_not_queue_topic_inference() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: Some("y".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        assert!(state
            .peon
            .label_hint
            .read()
            .unwrap()
            .get(&created_id)
            .is_none());
        assert!(!state
            .peon
            .label_pending
            .read()
            .unwrap()
            .contains(&created_id));
    }

    #[tokio::test]
    async fn unsupported_hook_rejects_thinking_without_changing_attention() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "attention-unsupported-thinking",
                    "Known",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ));
        }

        let response = report_attention(
            State(state.clone()),
            Path("attention-unsupported-thinking".into()),
            Json(AttentionReportRequest {
                status: "thinking".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("attention-unsupported-thinking")
            .unwrap();
        assert_eq!(updated.observed_status, None);
    }

    #[tokio::test]
    async fn session_without_active_work_hook_rejects_thinking_after_registry_changes() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "attention-session-scoped",
                "Known",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.harness = "claude-code".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        state.sessions.lock().unwrap().insert(
            "attention-session-scoped".into(),
            attention_test_handle("attention-session-scoped", dir.path()),
        );

        let response = report_attention(
            State(state.clone()),
            Path("attention-session-scoped".into()),
            Json(AttentionReportRequest {
                status: "thinking".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("attention-session-scoped")
            .unwrap();
        assert_eq!(updated.observed_status, None);
    }

    #[tokio::test]
    async fn report_attention_clears_pending_input_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "attention-clears-buf",
                    "Known",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ));
        }
        // A single-key "accept" hotkey press leaves an unterminated keystroke
        // sitting in the pending input-line buffer from an earlier prompt.
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert("attention-clears-buf".into(), "a".into());

        let response = report_attention(
            State(state.clone()),
            Path("attention-clears-buf".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(state
            .peon
            .input_buf
            .read()
            .unwrap()
            .get("attention-clears-buf")
            .is_none());
    }

    #[tokio::test]
    async fn report_attention_preserves_in_progress_descriptive_input() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "attention-preserves-buf",
                    "Known",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ));
        }
        // The user already started typing a real, unterminated response before
        // this (possibly delayed) hook POST landed; it must not be discarded.
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert("attention-preserves-buf".into(), "please also".into());

        let response = report_attention(
            State(state.clone()),
            Path("attention-preserves-buf".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            state
                .peon
                .input_buf
                .read()
                .unwrap()
                .get("attention-preserves-buf")
                .cloned(),
            Some("please also".to_string())
        );
    }

    #[tokio::test]
    async fn report_attention_returns_500_when_persist_fails() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let store = &ws.as_ref().unwrap().metadata;
            store.write_session(&test_session_metadata(
                "attention-persist-fail",
                "Known",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            ));
            // A directory squatting on the atomic-write temp path makes the
            // persist fail while the session stays readable.
            std::fs::create_dir_all(store.sessions_dir().join("attention-persist-fail.json.tmp"))
                .unwrap();
        }

        let response = report_attention(
            State(state),
            Path("attention-persist-fail".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "a lost attention signal must not be acknowledged with 200"
        );
    }

    #[tokio::test]
    async fn forget_session_deletes_session_with_unparseable_metadata_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (json_path, corrupt_path) = {
            let ws = state.workspace.lock().unwrap();
            let store = &ws.as_ref().unwrap().metadata;
            std::fs::create_dir_all(store.sessions_dir()).unwrap();
            let json_path = store.sessions_dir().join("corrupt-forget.json");
            std::fs::write(&json_path, "{\"id\": \"corrupt-forget\",").unwrap();
            (
                json_path,
                store.sessions_dir().join("corrupt-forget.json.corrupt"),
            )
        };

        let response = forget_session(State(state), Path("corrupt-forget".into()))
            .await
            .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::OK,
            "a corrupt-but-present session file must be forgettable, not 404"
        );
        assert!(!json_path.exists());
        assert!(!corrupt_path.exists());
    }

    #[tokio::test]
    async fn forget_session_deletes_terminal_output() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "forget-terminal-output".to_string();
        {
            let ws = state.workspace.lock().unwrap();
            let store = &ws.as_ref().unwrap().metadata;
            store.write_session(&test_session_metadata(
                session_id.clone(),
                "Forget Terminal Output",
                dir.path().display().to_string(),
                "ended",
                "2024-01-01T00:00:00Z",
                "2024-01-01T00:00:00Z",
            ));
            store.append_terminal_output_lines(&session_id, &["hello".to_string()]);
            assert_eq!(
                store.read_terminal_output(&session_id, 10),
                vec!["hello".to_string()]
            );
        }
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(session_id.clone(), 2);

        let response = forget_session(State(state.clone()), Path(session_id.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let ws = state.workspace.lock().unwrap();
        let store = &ws.as_ref().unwrap().metadata;
        assert!(
            store.read_terminal_output(&session_id, 10).is_empty(),
            "forgetting a session must delete its terminal output file, not just its metadata"
        );
        assert!(!state
            .peon
            .label_epochs
            .read()
            .unwrap()
            .contains_key(&session_id));
    }

    #[tokio::test]
    async fn forget_session_rejects_live_session_with_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "live-session".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: test_session_info(
                    session_id.clone(),
                    "Live Session",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = forget_session(State(state), Path(session_id))
            .await
            .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn list_sessions_does_not_duplicate_killed_sessions_with_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            projection_lock: std::sync::Mutex::new(()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations:
                    crate::workflow_observations::WorkflowObservationStore::open(orkworks.clone())
                        .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "killed-with-metadata".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Killed",
                        dir.path().display().to_string(),
                        "killed",
                        "2026-06-25T10:00:00Z",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            ws.metadata.write_session(&metadata::SessionMetadata {
                id: session_id.clone(),
                label: "Killed".into(),
                workspace: dir.path().display().to_string(),
                task: "".into(),
                harness: "".into(),
                model: "".into(),
                cwd: dir.path().display().to_string(),
                status: "killed".into(),
                work_phase: "unknown".into(),
                lifecycle_phase: "ended".into(),
                lifecycle: "dead".into(),
                attention: None,
                plan_path: None,
                connectivity: "offline".into(),
                terminal_outcome: Some("killed".into()),
                pending_terminal_status: None,
                observed_status: None,
                ending_observed_status_snapshot: None,
                final_observed_status_snapshot: Some(metadata::ObservedStatusSnapshotMetadata {
                    value: None,
                    source: "recovery".into(),
                    confidence: None,
                    observed_at: None,
                }),
                summary: None,
                next_action: None,
                needs_user_input: None,
                detected_question: None,
                suggested_options: None,
                blocker_description: None,
                failed_command: None,
                failed_test: None,
                capacity_hints: None,
                peon_last_inference: None,
                provider_id: None,
                provider_label: None,
                provider_model: None,
                provider_state: None,
                created_at: "2026-06-25T10:00:00Z".into(),
                last_activity: "2026-06-25T10:00:00Z".into(),
                last_output_at: None,
                metadata_source: "process".into(),
                metadata_confidence: 1.0,
                repo_root: None,
                branch: None,
                dirty: None,
                changed_files: None,
                is_worktree: None,
                resume: None,
                resume_options: vec![],
                harness_session_id_source: None,
                harness_session_id_confidence: None,
                harness_session_id_captured_at: None,
                resumed_from: None,
                last_user_input: None,
            });
        }

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let matching = sessions
            .iter()
            .filter(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .count();

        assert_eq!(matching, 1);
    }

    #[tokio::test]
    async fn delete_session_enters_ending_lifecycle_instead_of_marking_terminal_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "delete-ending".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: test_session_info(
                    session_id.clone(),
                    "Delete Ending",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Delete Ending".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
                    attention: None,
                    plan_path: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: None,
                    observed_status: Some("blocked".into()),
                    ending_observed_status_snapshot: None,
                    final_observed_status_snapshot: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    peon_last_inference: None,
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
                    last_output_at: None,
                    metadata_source: "peon".into(),
                    metadata_confidence: 0.8,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
        }

        let response = delete_session(State(state.clone()), Path(session_id.clone())).await;
        assert_eq!(
            response.into_response().status(),
            axum::http::StatusCode::OK
        );

        let info = state
            .sessions
            .lock()
            .unwrap()
            .get(&session_id)
            .unwrap()
            .info
            .clone();
        assert_eq!(info.status, "running");
        assert_eq!(info.lifecycle_phase, "ending");

        let ws = state.workspace.lock().unwrap();
        let meta = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&session_id)
            .unwrap();
        assert_eq!(meta.status, "running");
        assert_eq!(meta.lifecycle_phase, "ending");
        assert_eq!(meta.pending_terminal_status.as_deref(), Some("killed"));
        assert_eq!(
            meta.ending_observed_status_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.value.as_deref()),
            Some("blocked")
        );
    }

    #[tokio::test]
    async fn delete_session_clears_pending_input_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "delete-clears-input-buf".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: test_session_info(
                    session_id.clone(),
                    "Delete Clears Input Buf",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );
        // A stale, unterminated keystroke left over from an earlier prompt.
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert(session_id.clone(), "a".into());

        let response = delete_session(State(state.clone()), Path(session_id.clone())).await;
        assert_eq!(
            response.into_response().status(),
            axum::http::StatusCode::OK
        );

        assert!(state
            .peon
            .input_buf
            .read()
            .unwrap()
            .get(&session_id)
            .is_none());
    }

    #[tokio::test]
    async fn list_sessions_uses_live_session_contract_fields_without_metadata() {
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            projection_lock: std::sync::Mutex::new(()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "offline-live-only".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    connectivity: Some("offline".into()),
                    terminal_outcome: Some("ended".into()),
                    last_activity_at: Some("2026-06-28T09:05:00Z".into()),
                    ..test_session_info(
                        session_id.clone(),
                        "Offline Live Only",
                        "/tmp/project",
                        "ended",
                        "2026-06-28T09:00:00Z",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session.get("connectivity").and_then(|value| value.as_str()),
            Some("offline")
        );
        assert_eq!(
            session
                .get("terminalOutcome")
                .and_then(|value| value.as_str()),
            Some("ended")
        );
        assert_eq!(
            session
                .get("lastActivityAt")
                .and_then(|value| value.as_str()),
            Some("2026-06-28T09:05:00Z"),
        );
    }

    #[tokio::test]
    async fn list_sessions_keeps_pending_without_fresh_resume_output() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            projection_lock: std::sync::Mutex::new(()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "resume-pending-empty".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    capacity_check_pending: Some(true),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Pending Empty",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: true,
                output_lines_seen: 1,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((0, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("capacityCheckPending")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[tokio::test]
    async fn list_sessions_keys_checking_state_by_harness_like_capped_state() {
        let dir = tempfile::tempdir().unwrap();
        let settings = crate::providers::ProviderSettingsPayload {
            version: 1,
            revision: 1,
            peon_model: None,
            ollama_base_url: crate::providers::default_ollama_base_url(),
            providers: vec![crate::providers::ProviderSettingsEntry {
                id: "opencode".into(),
                enabled: true,
                fallback_order: 0,
                model: None,
                default_state: crate::providers::ProviderCapacityState::Healthy,
                override_state: None,
            }],
        };
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            projection_lock: std::sync::Mutex::new(()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::for_tests(settings, vec![]),
        });

        // Session on the opencode harness whose model provider is ollama:
        // capped state is keyed by harness, so checking must be too, or the
        // pending badge lands on a different provider row than the capped one.
        let session_id = "resume-pending-provider-mismatch".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("opencode".into()),
                    model_provider_id: Some("ollama".into()),
                    capacity_check_pending: Some(true),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Pending Provider Mismatch",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: true,
                output_lines_seen: 1,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((0, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        list_sessions(State(state.clone())).await.into_response();

        let response = state.providers.get_providers_response();
        let opencode = response
            .providers
            .iter()
            .find(|provider| provider.id == "opencode")
            .unwrap();
        assert_eq!(opencode.effective_state, "checking_capacity");
    }

    #[tokio::test]
    async fn list_sessions_requires_one_visible_fresh_output_cycle_before_clearing_pending() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            projection_lock: std::sync::Mutex::new(()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "resume-pending-fresh".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(200);
        output_buffer.push("Welcome back".into());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    capacity_check_pending: Some(true),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Pending Fresh",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: true,
                output_lines_seen: 1,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((0, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();
        assert_eq!(
            session
                .get("capacityCheckPending")
                .and_then(|value| value.as_bool()),
            Some(true)
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();
        assert_eq!(session.get("capacityCheckPending"), None);
    }

    #[tokio::test]
    async fn list_sessions_does_not_mark_remembered_sessions_capped_from_other_live_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        state
            .providers
            .apply_settings(crate::providers::ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: crate::providers::default_ollama_base_url(),
                providers: vec![crate::providers::ProviderSettingsEntry {
                    id: "codex".into(),
                    enabled: true,
                    fallback_order: 0,
                    model: None,
                    default_state: crate::providers::ProviderCapacityState::Unknown,
                    override_state: None,
                }],
            });
        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            let mut remembered = test_session_metadata(
                "remembered-codex",
                "Remembered Codex",
                dir.path().display().to_string(),
                "ended",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            remembered.harness = "codex".into();
            remembered.cwd = dir.path().display().to_string();
            ws.metadata.write_session(&remembered);

            let mut live_meta = test_session_metadata(
                "live-capped-codex",
                "Live Capped Codex",
                dir.path().display().to_string(),
                "running",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            live_meta.harness = "codex".into();
            live_meta.cwd = dir.path().display().to_string();
            live_meta.status = "running".into();
            live_meta.lifecycle_phase = "active".into();
            live_meta.connectivity = "online".into();
            live_meta.terminal_outcome = None;
            live_meta.final_observed_status_snapshot = None;
            ws.metadata.write_session(&live_meta);
        }

        let live_id = "live-capped-codex".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(200);
        output_buffer.push("You've hit your usage limit".into());
        state.sessions.lock().unwrap().insert(
            live_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    harness: Some("codex".into()),
                    ..test_session_info(
                        live_id.clone(),
                        "Live Capped Codex",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 1,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let live = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(live_id.as_str()))
            .unwrap();
        let remembered = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some("remembered-codex")
            })
            .unwrap();

        assert_eq!(
            live.get("atUsageLimit").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            remembered
                .get("memoryState")
                .and_then(|value| value.as_str()),
            Some("remembered")
        );
        assert_eq!(remembered.get("atUsageLimit"), None);

        let providers = state.providers.get_providers_response();
        let codex = providers
            .providers
            .iter()
            .find(|provider| provider.id == "codex")
            .unwrap();
        assert_eq!(codex.effective_state, "capped");
    }

    #[tokio::test]
    async fn list_sessions_clears_live_capped_after_fresh_post_input_output_without_new_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let session_id = "codex-cap-clear".to_string();
        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            let mut meta = test_session_metadata(
                session_id.clone(),
                "Codex Cap Clear",
                dir.path().display().to_string(),
                "running",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            meta.harness = "codex".into();
            meta.cwd = dir.path().display().to_string();
            meta.status = "running".into();
            meta.lifecycle_phase = "active".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            meta.final_observed_status_snapshot = None;
            ws.metadata.write_session(&meta);
        }
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(200);
        output_buffer.push("You've hit your usage limit".into());
        output_buffer.push("Back in the thread and working again".into());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    harness: Some("codex".into()),
                    ..test_session_info(
                        session_id.clone(),
                        "Codex Cap Clear",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: true,
                capacity_check_pending: false,
                output_lines_seen: 2,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((1, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("atUsageLimit")
                .and_then(|value| value.as_bool()),
            Some(false)
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("atUsageLimit")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
    }

    #[tokio::test]
    async fn list_sessions_keeps_live_capped_when_fresh_post_input_output_still_contains_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let session_id = "codex-cap-still-capped".to_string();
        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            let mut meta = test_session_metadata(
                session_id.clone(),
                "Codex Cap Still Capped",
                dir.path().display().to_string(),
                "running",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            meta.harness = "codex".into();
            meta.cwd = dir.path().display().to_string();
            meta.status = "running".into();
            meta.lifecycle_phase = "active".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            meta.final_observed_status_snapshot = None;
            ws.metadata.write_session(&meta);
        }
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(200);
        output_buffer.push("You've hit your usage limit".into());
        output_buffer.push("You've hit your usage limit".into());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    harness: Some("codex".into()),
                    ..test_session_info(
                        session_id.clone(),
                        "Codex Cap Still Capped",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: true,
                capacity_check_pending: false,
                output_lines_seen: 2,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((1, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state)).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("atUsageLimit")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[tokio::test]
    async fn list_sessions_clears_live_capped_even_when_ring_buffer_length_stays_flat() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let session_id = "codex-cap-clear-saturated".to_string();
        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            let mut meta = test_session_metadata(
                session_id.clone(),
                "Codex Cap Clear Saturated",
                dir.path().display().to_string(),
                "running",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            meta.harness = "codex".into();
            meta.cwd = dir.path().display().to_string();
            meta.status = "running".into();
            meta.lifecycle_phase = "active".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            meta.final_observed_status_snapshot = None;
            ws.metadata.write_session(&meta);
        }
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(1);
        output_buffer.push("Back in the thread and working again".into());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    harness: Some("codex".into()),
                    ..test_session_info(
                        session_id.clone(),
                        "Codex Cap Clear Saturated",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: true,
                capacity_check_pending: false,
                output_lines_seen: 2,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((1, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("atUsageLimit")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
    }

    #[tokio::test]
    async fn list_sessions_derives_resume_options_for_remembered_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            projection_lock: std::sync::Mutex::new(()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations:
                    crate::workflow_observations::WorkflowObservationStore::open(orkworks.clone())
                        .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            ws.metadata.write_session(&metadata::SessionMetadata {
                id: "remembered-derived".into(),
                label: "Remembered Derived".into(),
                workspace: dir.path().display().to_string(),
                task: "".into(),
                harness: "opencode".into(),
                model: "".into(),
                cwd: dir.path().display().to_string(),
                status: "ended".into(),
                work_phase: "unknown".into(),
                lifecycle_phase: "ended".into(),
                lifecycle: "dead".into(),
                attention: None,
                plan_path: None,
                connectivity: "offline".into(),
                terminal_outcome: Some("ended".into()),
                pending_terminal_status: None,
                observed_status: None,
                ending_observed_status_snapshot: None,
                final_observed_status_snapshot: Some(metadata::ObservedStatusSnapshotMetadata {
                    value: None,
                    source: "recovery".into(),
                    confidence: None,
                    observed_at: None,
                }),
                summary: None,
                next_action: None,
                needs_user_input: None,
                detected_question: None,
                suggested_options: None,
                blocker_description: None,
                failed_command: None,
                failed_test: None,
                capacity_hints: None,
                peon_last_inference: None,
                provider_id: None,
                provider_label: None,
                provider_model: None,
                provider_state: None,
                created_at: "2026-06-28T09:00:00Z".into(),
                last_activity: "2026-06-28T09:05:00Z".into(),
                last_output_at: None,
                metadata_source: "process".into(),
                metadata_confidence: 1.0,
                repo_root: Some(dir.path().display().to_string()),
                branch: Some("main".into()),
                dirty: Some(false),
                changed_files: Some(0),
                is_worktree: Some(false),
                resume: Some(harness::ResumeMemory {
                    state: harness::ResumeState::Available,
                    preferred_strategy: harness::ResumeStrategy::Exact,
                    harness_session_id: None,
                    latest_fallback: true,
                    last_seen_at: Some("2026-06-28T09:05:00Z".into()),
                }),
                resume_options: vec![metadata::ResumeOption {
                    strategy: harness::ResumeStrategy::Exact,
                    label: "Resume exact session".into(),
                    available: true,
                    preferred: true,
                    reason: None,
                }],
                harness_session_id_source: None,
                harness_session_id_confidence: None,
                harness_session_id_captured_at: None,
                resumed_from: None,
                last_user_input: None,
            });
        }

        let response = list_sessions(State(state)).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some("remembered-derived")
            })
            .unwrap();
        let options = session
            .get("resumeOptions")
            .and_then(|value| value.as_array())
            .unwrap();

        assert_eq!(options.len(), 3);
        assert_eq!(options[0]["strategy"], "exact");
        assert_eq!(options[0]["available"], false);
        assert_eq!(options[1]["strategy"], "latest_cwd");
        assert_eq!(options[1]["available"], true);
        assert_eq!(options[1]["preferred"], true);
        assert_eq!(options[2]["strategy"], "latest_repo");
        assert_eq!(options[2]["available"], true);
    }

    #[test]
    fn workspace_request_deserializes_path() {
        let json = r#"{"path": "/home/user/project"}"#;
        let req: WorkspaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.path, "/home/user/project");
    }

    #[test]
    fn workspace_response_serializes_all_fields() {
        let resp = WorkspaceResponse {
            path: "/tmp".into(),
            repo_root: Some("/tmp".into()),
            branch: Some("main".into()),
            dirty: Some(false),
            last_active_session_id: Some("session-1".into()),
            active_harness_ids: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"path\":\"/tmp\""));
        assert!(json.contains("\"repo_root\":\"/tmp\""));
        assert!(json.contains("\"branch\":\"main\""));
        assert!(json.contains("\"dirty\":false"));
        assert!(json.contains("\"lastActiveSessionId\":\"session-1\""));
    }

    #[test]
    fn workspace_response_without_git() {
        let resp = WorkspaceResponse {
            path: "/tmp".into(),
            repo_root: None,
            branch: None,
            dirty: None,
            last_active_session_id: None,
            active_harness_ids: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"path\":\"/tmp\""));
        assert!(json.contains("\"repo_root\":null"));
        assert!(json.contains("\"branch\":null"));
        assert!(json.contains("\"dirty\":null"));
        assert!(json.contains("\"lastActiveSessionId\":null"));
    }

    fn test_resolved_registry() -> crate::harness::registry::ResolvedHarnessRegistry {
        let builtins = crate::harness::definition::BuiltinDocument::parse(
            crate::harness::definition::EMBEDDED_BUILTINS,
        )
        .unwrap();
        crate::harness::registry::resolve_document(
            &builtins,
            &crate::harness::definition::HarnessUserDocument::default(),
        )
        .unwrap()
    }

    #[test]
    fn resolve_session_launch_codex_wires_to_codex_definition() {
        let registry = test_resolved_registry();
        let launch = resolve_session_launch(
            &registry,
            &CreateSessionCommand {
                harness_id: Some("codex".into()),
                model: None,
                initial_prompt: None,
            },
            "/repo".into(),
        );

        assert_eq!(launch.session_harness_id.as_deref(), Some("codex"));
        assert_eq!(launch.command.program, "codex");
    }

    #[tokio::test]
    async fn create_session_rejects_a_retired_harness() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = create_session(
            State(state),
            Json(CreateSessionRequest {
                harness_id: Some("gemini".into()),
                model: None,
                initial_prompt: Some("do not send this to a shell".into()),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn resume_invalid_request_keeps_the_empty_bad_request_body() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "resume-without-metadata";
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&test_session_metadata(
                id,
                "Resume without metadata",
                dir.path().display().to_string(),
                "ended",
                "before",
                "before",
            ));

        let response = resume_session(State(state), Path(id.into())).await;
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(body.is_empty());
    }

    /// Regression test for issue #302: `create_session` used to await the PTY
    /// spawn before responding, so the client's response body — and thus the
    /// first render of the new session in `sessions` state — never actually
    /// observed `status: "creating"`. The response must reflect the
    /// pre-spawn record; the transition to `"running"` happens afterward,
    /// asynchronously.
    #[tokio::test]
    async fn create_session_returns_creating_status_before_spawn_completes() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body["status"], "creating",
            "the create response must reflect the pre-spawn record, not wait for spawn to finish"
        );
        let created_id = body["id"].as_str().unwrap().to_owned();

        wait_for_session_status(&state, &created_id, "running").await;
    }

    /// Regression test for issue #302: a spawn failure must no longer surface
    /// as a synchronous 500 from `POST /sessions` (that response has already
    /// gone out reporting `"creating"`), but as an async transition to
    /// `status: "error"` observed via the existing poll/metadata mechanism —
    /// the same signal resume and daemon-restart reconciliation already
    /// produce for this failure mode.
    #[tokio::test]
    async fn create_session_spawn_failure_surfaces_as_async_error_status() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                let override_patch = document.overrides.entry("opencode".into()).or_default();
                override_patch.launch = Some(harness::definition::LaunchPatch {
                    command: Some("orkworks-create-command-that-does-not-exist".into()),
                    ..Default::default()
                });
                Ok(())
            })
            .unwrap();

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("opencode".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::OK,
            "spawn failures are no longer synchronous 500s"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["status"], "creating");
        let created_id = body["id"].as_str().unwrap().to_owned();

        wait_for_session_status(&state, &created_id, "error").await;
    }

    #[test]
    fn resolve_session_launch_opencode_no_model_omits_model_args() {
        let registry = test_resolved_registry();
        let launch = resolve_session_launch(
            &registry,
            &CreateSessionCommand {
                harness_id: Some("opencode".into()),
                model: None,
                initial_prompt: None,
            },
            "/repo".into(),
        );
        assert!(
            !launch.command.args.contains(&"--model".into()),
            "bare --model should be dropped"
        );
        assert!(
            !launch.command.args.iter().any(|a| a.starts_with("ollama/")),
            "bare prefix should not appear"
        );
    }

    #[test]
    fn resolve_session_launch_opencode_with_model_uses_prefix() {
        let registry = test_resolved_registry();
        let launch = resolve_session_launch(
            &registry,
            &CreateSessionCommand {
                harness_id: Some("opencode".into()),
                model: Some("qwen2.5-coder:latest".into()),
                initial_prompt: None,
            },
            "/repo".into(),
        );
        assert!(launch
            .command
            .args
            .contains(&"ollama/qwen2.5-coder:latest".into()));
    }

    #[test]
    fn resolve_session_launch_does_not_infer_model_provider_from_harness() {
        let registry = test_resolved_registry();
        let launch = resolve_session_launch(
            &registry,
            &CreateSessionCommand {
                harness_id: Some("codex".into()),
                model: Some("gpt-5".into()),
                initial_prompt: None,
            },
            "/repo".into(),
        );

        assert_eq!(launch.session_harness_id.as_deref(), Some("codex"));
        assert_eq!(launch.model.as_deref(), Some("gpt-5"));
        assert_eq!(launch.provider_id, None);
        assert_eq!(launch.provider_label, None);
    }

    #[test]
    fn attention_report_plan_path_distinguishes_set_clear_and_omission() {
        let set: AttentionReportRequest =
            serde_json::from_str(r#"{"status":"waiting_for_input","planPath":"docs/plan.md"}"#)
                .unwrap();
        assert_eq!(
            set.plan_path,
            metadata::PlanPathUpdate::Set("docs/plan.md".into())
        );

        let clear: AttentionReportRequest =
            serde_json::from_str(r#"{"status":"waiting_for_input","planPath":null}"#).unwrap();
        assert_eq!(clear.plan_path, metadata::PlanPathUpdate::Clear);

        let unchanged: AttentionReportRequest =
            serde_json::from_str(r#"{"status":"waiting_for_input"}"#).unwrap();
        assert_eq!(unchanged.plan_path, metadata::PlanPathUpdate::Unchanged);
    }

    #[tokio::test]
    async fn plan_content_requires_a_valid_plan_and_sidecar_token() {
        let _token_lock = PLAN_TOKEN_TEST_LOCK.lock().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        std::fs::create_dir(workspace.path().join("docs")).unwrap();
        let plan = workspace.path().join("docs/plan.md");
        std::fs::write(&plan, "# plan").unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        let mut metadata = test_session_metadata(
            "plan-session",
            "Plan session",
            workspace.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.plan_path = Some("docs/plan.md".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "test-token");
        let mut headers = HeaderMap::new();
        headers.insert("x-orkworks-open-plan-token", "test-token".parse().unwrap());
        let response = get_session_plan_content(State(state), Path("plan-session".into()), headers)
            .await
            .into_response();
        std::env::remove_var("ORKWORKS_OPEN_PLAN_TOKEN");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn report_session_plan_path_normalizes_the_path_without_changing_attention() {
        let workspace = tempfile::tempdir().unwrap();
        let plan_dir = workspace.path().join("docs/superpowers/plans");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("plan.md");
        std::fs::write(&plan, "# plan").unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        let mut metadata = test_session_metadata(
            "plan-session",
            "Plan session",
            workspace.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        metadata.attention = Some("working".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let response = report_session_plan_path(
            State(state.clone()),
            Path("plan-session".into()),
            Json(PlanPathReportRequest {
                plan_path: plan.display().to_string(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
        let metadata = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session("plan-session")
            .unwrap();
        assert_eq!(
            metadata.plan_path.as_deref(),
            Some("docs/superpowers/plans/plan.md")
        );
        assert_eq!(metadata.attention.as_deref(), Some("working"));
    }

    #[tokio::test]
    async fn report_session_plan_path_returns_internal_error_and_skips_event_when_session_write_fails(
    ) {
        let workspace = tempfile::tempdir().unwrap();
        let plan_dir = workspace.path().join("docs/superpowers/plans");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("plan.md");
        std::fs::write(&plan, "# plan").unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        let mut metadata = test_session_metadata(
            "plan-session",
            "Plan session",
            workspace.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        metadata.attention = Some("working".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        // Squat a directory on the per-session temp path so the atomic write
        // fails (write_session returns Err) while the session JSON remains
        // readable — mirrors the established failure-mode test pattern.
        let sessions_path = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .sessions_dir();
        std::fs::create_dir_all(sessions_path.join("plan-session.json.tmp")).unwrap();

        let response = report_session_plan_path(
            State(state.clone()),
            Path("plan-session".into()),
            Json(PlanPathReportRequest {
                plan_path: plan.display().to_string(),
            }),
        )
        .await
        .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
        // The session JSON is the source of truth; the handler must not
        // append a `session.plan_path_hooked` event when the write never
        // landed.
        let events = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events("plan-session");
        assert!(
            events
                .iter()
                .all(|event| event.event_type != "session.plan_path_hooked"),
            "no plan-path-hooked event should be appended on a write failure, got: {events:?}"
        );
    }

    #[tokio::test]
    async fn plan_endpoints_reject_a_missing_sidecar_token() {
        let _token_lock = PLAN_TOKEN_TEST_LOCK.lock().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "test-token");

        let content = get_session_plan_content(
            State(state.clone()),
            Path("missing".into()),
            HeaderMap::new(),
        )
        .await
        .into_response();
        let review =
            request_session_plan_review(State(state), Path("missing".into()), HeaderMap::new())
                .await
                .into_response();
        std::env::remove_var("ORKWORKS_OPEN_PLAN_TOKEN");

        assert_eq!(content.status(), axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(review.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn plan_review_submits_the_fixed_prompt_before_recording_the_event() {
        let _token_lock = PLAN_TOKEN_TEST_LOCK.lock().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        std::fs::create_dir(workspace.path().join("specs")).unwrap();
        std::fs::write(workspace.path().join("specs/plan.md"), "# plan").unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        let mut metadata = test_session_metadata(
            "plan-session",
            "Plan session",
            workspace.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        metadata.plan_path = Some("specs/plan.md".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let mut handle = attention_test_handle("plan-session", workspace.path());
        let (runtime, mut control_rx) =
            crate::runtime::session_runtime::SessionRuntime::live(24, 80);
        handle.runtime = runtime;
        state
            .sessions
            .lock()
            .unwrap()
            .insert("plan-session".into(), handle);

        std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "test-token");
        let mut headers = HeaderMap::new();
        headers.insert("x-orkworks-open-plan-token", "test-token".parse().unwrap());
        let mut request = tokio::spawn(request_session_plan_review(
            State(state.clone()),
            Path("plan-session".into()),
            headers,
        ));
        let crate::runtime::session_runtime::RuntimeCommand::Input { data, accepted } = (tokio::select! {
            command = control_rx.recv() => command.unwrap(),
            response = &mut request => panic!("review request returned {} before reaching the PTY", response.unwrap().into_response().status()),
        }) else {
            panic!("expected terminal input")
        };
        assert_eq!(data, "Please review the plan or specification at specs/plan.md. If your tooling can spawn a separate review subagent, delegate the review to it instead of reviewing your own work; otherwise review it yourself. Check for missing requirements, risky assumptions, and unclear steps, then report the findings.\r");
        accepted.unwrap().send(Ok(())).unwrap();
        let response = request.await.unwrap().into_response();
        std::env::remove_var("ORKWORKS_OPEN_PLAN_TOKEN");

        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
        let events = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events("plan-session");
        assert!(events
            .iter()
            .any(|event| event.event_type == "plan_review_requested"));
    }
}
