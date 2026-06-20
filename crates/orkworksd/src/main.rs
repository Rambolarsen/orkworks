use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use portable_pty::{CommandBuilder, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock as StdRwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(unix)]
use portable_pty::unix::UnixPtySystem;
#[cfg(windows)]
use portable_pty::win::conpty::ConPtySystem;

mod metadata;
mod watcher;
mod git;
mod harness;
mod peon;

#[derive(Clone, Debug, Serialize)]
struct SessionInfo {
    id: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    harness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    status: String,
    cwd: String,
    created_at: String,
    #[serde(rename = "observedStatus")]
    observed_status: Option<String>,
    pub summary: Option<String>,
    #[serde(rename = "nextAction")]
    next_action: Option<String>,
    #[serde(rename = "needsUserInput")]
    needs_user_input: Option<bool>,
    #[serde(rename = "detectedQuestion")]
    detected_question: Option<String>,
    #[serde(rename = "suggestedOptions")]
    suggested_options: Option<Vec<String>>,
    #[serde(rename = "blockerDescription")]
    blocker_description: Option<String>,
    #[serde(rename = "failedCommand")]
    failed_command: Option<String>,
    #[serde(rename = "failedTest")]
    failed_test: Option<String>,
    #[serde(rename = "capacityHints")]
    capacity_hints: Option<Vec<String>>,
    #[serde(rename = "metadataSource")]
    metadata_source: Option<String>,
    #[serde(rename = "metadataConfidence")]
    metadata_confidence: Option<f64>,
    #[serde(rename = "repoRoot")]
    repo_root: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
    #[serde(rename = "changedFiles")]
    changed_files: Option<usize>,
    #[serde(rename = "isWorktree")]
    is_worktree: Option<bool>,
    #[serde(rename = "conflictWarning")]
    conflict_warning: Option<String>,
    recommendation: Option<String>,
    #[serde(rename = "peonLastInference")]
    peon_last_inference: Option<String>,
    #[serde(rename = "memoryState")]
    memory_state: MemoryState,
    #[serde(rename = "resumeStrategy")]
    resume_strategy: harness::ResumeStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    resume: Option<harness::ResumeMemory>,
    #[serde(rename = "resumedFrom", skip_serializing_if = "Option::is_none")]
    resumed_from: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MemoryState {
    Live,
    Remembered,
    Resumable,
    Unsupported,
}

struct SessionHandle {
    info: SessionInfo,
    kill_tx: tokio::sync::watch::Sender<bool>,
    output_buffer: peon::RingBuffer,
    command: harness::CommandSpec,
}

struct WorkspaceState {
    path: PathBuf,
    metadata: metadata::MetadataStore,
    #[allow(dead_code)]
    watcher: watcher::MetadataWatcher,
}

struct PeonState {
    last_output: StdRwLock<HashMap<String, tokio::time::Instant>>,
    last_inference: StdRwLock<HashMap<String, String>>,
    in_flight: StdRwLock<HashSet<String>>,
    config: peon::PeonConfig,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct RetentionConfig {
    #[serde(rename = "maxSessions", default)]
    max_sessions: usize,
    #[serde(rename = "maxAgeDays", default)]
    max_age_days: u32,
}

struct AppState {
    sessions: Mutex<HashMap<String, SessionHandle>>,
    workspace: Mutex<Option<WorkspaceState>>,
    peon: PeonState,
    adapters: HashMap<String, harness::HarnessAdapter>,
    retention_config: tokio::sync::RwLock<RetentionConfig>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "orkworksd=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = Arc::new(AppState {
        sessions: Mutex::new(HashMap::new()),
        workspace: Mutex::new(None),
        peon: PeonState {
            last_output: StdRwLock::new(HashMap::new()),
            last_inference: StdRwLock::new(HashMap::new()),
            in_flight: StdRwLock::new(HashSet::new()),
            config: peon::PeonConfig::from_env(),
        },
        adapters: builtin_adapters(),
        retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
    });

    // Start Peon background task
    if state.peon.config.enabled {
        let peon_state = state.clone();
        tokio::spawn(async move {
            peon_loop(peon_state).await;
        });
    }

    // Start retention cleanup background task
    {
        let retention_state = state.clone();
        tokio::spawn(async move {
            retention_cleanup_task(retention_state).await;
        });
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/workspace", post(set_workspace))
        .route("/workspace/active-session", post(set_active_session))
        .route("/sessions", post(create_session))
        .route("/sessions", get(list_sessions))
        .route("/sessions/:id", delete(delete_session))
        .route("/sessions/:id/forget", delete(forget_session))
        .route("/sessions/:id/resume", post(resume_session))
        .route("/settings/retention", post(set_retention))
        .route("/sessions/:id/terminal", get(session_terminal_handler))
        .route("/sessions/:id/terminal-output", get(get_terminal_output))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let bound_addr = listener.local_addr().unwrap();

    println!("ORKWORKSD_PORT={}", bound_addr.port());

    tracing::info!("orkworksd listening on {}", bound_addr);

    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct WorkspaceRequest {
    path: String,
}

#[derive(Deserialize)]
struct ActiveSessionRequest {
    #[serde(rename = "sessionId")]
    session_id: String,
}

#[derive(Serialize)]
struct WorkspaceResponse {
    path: String,
    repo_root: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
    #[serde(rename = "lastActiveSessionId")]
    last_active_session_id: Option<String>,
}

async fn set_workspace(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WorkspaceRequest>,
) -> impl IntoResponse {
    let ws_path = PathBuf::from(&req.path);
    if !ws_path.is_dir() {
        return (axum::http::StatusCode::BAD_REQUEST, "not a directory").into_response();
    }

    let orkworks_dir = ws_path.join(".orkworks");
    for dir in &["sessions", "events", "capacity", "skills"] {
        if let Err(e) = std::fs::create_dir_all(orkworks_dir.join(dir)) {
            tracing::warn!("failed to create .orkworks/{}: {e}", dir);
        }
    }

    let store = metadata::MetadataStore::new(&orkworks_dir);
    let last_active_session_id = store
        .read_workspace_memory()
        .and_then(|memory| memory.last_active_session_id);
    let watch_dir = orkworks_dir.join("sessions");
    let watcher = watcher::MetadataWatcher::start(&watch_dir);

    let mut ws = state.workspace.lock().unwrap();
    *ws = Some(WorkspaceState {
        path: ws_path.clone(),
        metadata: store,
        watcher,
    });

    let git_ctx = git::detect(&ws_path);

    Json(WorkspaceResponse {
        path: req.path,
        repo_root: git_ctx.repo_root,
        branch: git_ctx.branch,
        dirty: Some(git_ctx.dirty),
        last_active_session_id,
    })
    .into_response()
}

async fn health_check() -> &'static str {
    "ok"
}

async fn set_active_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActiveSessionRequest>,
) -> impl IntoResponse {
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        ws.metadata.write_workspace_memory(&metadata::WorkspaceMemory {
            last_active_session_id: Some(req.session_id),
            last_active_at: Some(now),
        });
        return axum::http::StatusCode::OK;
    }
    axum::http::StatusCode::CONFLICT
}

async fn resume_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let now = iso_now();
    let (meta, command, strategy) = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ref ws) = *ws_guard else {
            return axum::http::StatusCode::CONFLICT.into_response();
        };
        let Some(meta) = ws.metadata.read_session(&id) else {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        };
        let Some(resume) = meta.resume.as_ref() else {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        };
        let harness_id = (!meta.harness.is_empty()).then(|| meta.harness.as_str());
        let capabilities = capabilities_for_harness(&state.adapters, harness_id);
        let strategy = harness::select_resume_strategy(resume, &capabilities);
        if strategy == harness::ResumeStrategy::None {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        }
        let adapter = adapter_for_harness(&state.adapters, harness_id);
        let request = harness::ResumeRequest {
            strategy: strategy.clone(),
            cwd: meta.cwd.clone(),
            repo_root: meta.repo_root.clone(),
            harness_session_id: resume.harness_session_id.clone(),
            model: (!meta.model.is_empty()).then(|| meta.model.clone()),
        };
        let Some(command) = adapter.build_resume_command(&request) else {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        };
        (meta, command, strategy)
    };

    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);
    let info = SessionInfo {
        id: id.clone(),
        label: meta.label.clone(),
        harness: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
        model: (!meta.model.is_empty()).then(|| meta.model.clone()),
        status: "creating".into(),
        cwd: command.cwd.clone(),
        created_at: meta.created_at.clone(),
        observed_status: None,
        summary: meta.summary.clone(),
        next_action: meta.next_action.clone(),
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        metadata_source: Some("process".into()),
        metadata_confidence: Some(1.0),
        repo_root: meta.repo_root.clone(),
        branch: meta.branch.clone(),
        dirty: meta.dirty,
        changed_files: meta.changed_files,
        is_worktree: meta.is_worktree,
        conflict_warning: None,
        recommendation: None,
        peon_last_inference: None,
        memory_state: MemoryState::Live,
        resume_strategy: strategy,
        resume: meta.resume.clone(),
        resumed_from: meta.resumed_from.clone(),
    };

    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(&id) {
            handle.info = info.clone();
            handle.kill_tx = kill_tx;
            handle.output_buffer = peon::RingBuffer::new(state.peon.config.max_lines);
            handle.command = command;
        } else {
            sessions.insert(
                id.clone(),
                SessionHandle {
                    info: info.clone(),
                    kill_tx,
                    output_buffer: peon::RingBuffer::new(state.peon.config.max_lines),
                    command,
                },
            );
        }
    }

    {
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(mut stored_meta) = ws.metadata.read_session(&id) {
                stored_meta.status = "creating".to_string();
                stored_meta.resume = meta.resume.clone();
                stored_meta.resumed_from = meta.resumed_from.clone();
                ws.metadata.write_session(&stored_meta);
            }
            ws.metadata.append_event(&id, &metadata::Event {
                event_type: "session.resumed".into(),
                timestamp: now,
                status: "creating".into(),
                observed_status: None,
                confidence: None,
            });
        }
    }

    Json(info).into_response()
}

async fn create_session(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".into());

    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);

    let git_ctx = git::detect(&PathBuf::from(&cwd));
    let now = iso_now();
    let info = SessionInfo {
        id: id.clone(),
        label: format!("Session {}", &id[..8]),
        harness: None,
        model: None,
        status: "creating".into(),
        cwd,
        created_at: now.clone(),
        observed_status: None,
        summary: None,
        next_action: None,
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        metadata_source: None,
        metadata_confidence: None,
        repo_root: git_ctx.repo_root.clone(),
        branch: git_ctx.branch.clone(),
        dirty: Some(git_ctx.dirty),
        changed_files: Some(git_ctx.changed_files),
        is_worktree: Some(git_ctx.is_worktree),
        conflict_warning: None,
        recommendation: None,
        peon_last_inference: None,
        memory_state: MemoryState::Live,
        resume_strategy: harness::ResumeStrategy::None,
        resume: Some(harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some(now.clone()),
        }),
        resumed_from: None,
    };

    let handle = SessionHandle { info: info.clone(), kill_tx, output_buffer: peon::RingBuffer::new(state.peon.config.max_lines), command: default_shell_command(info.cwd.clone()) };

    state.sessions.lock().unwrap().insert(id.clone(), handle);

    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        let meta_git_ctx = git::detect(&ws.path);
        ws.metadata.write_session(&metadata::SessionMetadata {
            id: id.clone(),
            label: info.label.clone(),
            workspace: ws.path.display().to_string(),
            task: String::new(),
            harness: String::new(),
            model: String::new(),
            cwd: info.cwd.clone(),
            status: "creating".into(),
            phase: String::new(),
            observed_status: None,
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
            created_at: now.clone(),
            last_activity: now.clone(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: meta_git_ctx.repo_root.clone(),
            branch: meta_git_ctx.branch.clone(),
            dirty: Some(meta_git_ctx.dirty),
            changed_files: Some(meta_git_ctx.changed_files),
            is_worktree: Some(meta_git_ctx.is_worktree),
            resume: info.resume.clone(),
            resumed_from: info.resumed_from.clone(),
        });
        ws.metadata.append_event(&id, &metadata::Event {
            event_type: "session.created".into(),
            timestamp: now,
            status: "creating".into(),
            observed_status: None,
            confidence: None,
        });
    }
    drop(ws_guard);

    Json(info)
}

fn detect_conflicts(sessions: &[SessionInfo]) -> Vec<(String, String)> {
    let mut cwd_groups: HashMap<&str, Vec<&SessionInfo>> = HashMap::new();
    for s in sessions {
        if s.status == "running" || s.status == "creating" {
            cwd_groups.entry(&s.cwd).or_default().push(s);
        }
    }
    let mut warnings = Vec::new();
    for (_cwd, group) in &cwd_groups {
        if group.len() >= 2 {
            if let Some(s) = group.first() {
                if s.dirty.unwrap_or(false) {
                    let warning = format!("{} sessions in this dirty workspace", group.len());
                    for session in group {
                        warnings.push((session.id.clone(), warning.clone()));
                    }
                }
            }
        }
    }
    warnings
}

fn session_recommendation(ctx: &git::GitContext, session_count_in_cwd: usize) -> Option<String> {
    if ctx.is_worktree {
        return Some("Running in a separate worktree. Good isolation.".into());
    }
    if session_count_in_cwd >= 2 && ctx.dirty {
        return Some("Multiple sessions in the same dirty workspace. Consider separate worktrees.".into());
    }
    if !ctx.is_worktree && ctx.dirty && ctx.branch.as_deref() != Some("main") {
        return Some("Working outside main in a dirty workspace. A worktree may be safer.".into());
    }
    None
}

async fn list_sessions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let session_data: Vec<(String, String, String, String, String)> = {
        let sessions = state.sessions.lock().unwrap();
        sessions.values().map(|h| {
            (h.info.id.clone(), h.info.label.clone(), h.info.status.clone(), h.info.cwd.clone(), h.info.created_at.clone())
        }).collect()
    };

    let ws_guard = state.workspace.lock().unwrap();
    let metadata_map = ws_guard.as_ref().map(|ws| {
        let mut metadata = HashMap::new();
        for (id, _, _, _, _) in &session_data {
            if let Some(meta) = ws.metadata.read_session(id) {
                metadata.insert(id.clone(), meta);
            }
        }
        metadata
    }).unwrap_or_default();

    let all_metadata_sessions = ws_guard.as_ref().map(|ws| ws.metadata.read_all_sessions()).unwrap_or_default();
    drop(ws_guard);

    let live_ids: HashSet<String> = session_data.iter().map(|(id, _, _, _, _)| id.clone()).collect();

    let peon_times = state.peon.last_inference.read().unwrap();
    let mut infos: Vec<SessionInfo> = session_data.into_iter().map(|(id, label, status, cwd, created_at)| {
        let meta = metadata_map.get(&id);
        let harness_id = meta.and_then(|m| (!m.harness.is_empty()).then(|| m.harness.as_str()));
        let caps = capabilities_for_harness(&state.adapters, harness_id);
        let (memory_state, resume_strategy) =
            derive_memory_state(true, meta.and_then(|m| m.resume.as_ref()), &caps);
        SessionInfo {
            id: id.clone(),
            label: meta.map(|m| m.label.clone()).unwrap_or(label),
            harness: meta.and_then(|m| (!m.harness.is_empty()).then(|| m.harness.clone())),
            model: meta.and_then(|m| (!m.model.is_empty()).then(|| m.model.clone())),
            status,
            cwd,
            created_at,
            observed_status: meta.and_then(|m| m.observed_status.clone()),
            summary: meta.and_then(|m| m.summary.clone()),
            next_action: meta.and_then(|m| m.next_action.clone()),
            needs_user_input: meta.and_then(|m| m.needs_user_input),
            detected_question: meta.and_then(|m| m.detected_question.clone()),
            suggested_options: meta.and_then(|m| m.suggested_options.clone()),
            blocker_description: meta.and_then(|m| m.blocker_description.clone()),
            failed_command: meta.and_then(|m| m.failed_command.clone()),
            failed_test: meta.and_then(|m| m.failed_test.clone()),
            capacity_hints: meta.and_then(|m| m.capacity_hints.clone()),
            metadata_source: meta.map(|m| m.metadata_source.clone()),
            metadata_confidence: meta.map(|m| m.metadata_confidence),
            peon_last_inference: meta
                .and_then(|m| m.peon_last_inference.clone())
                .or_else(|| peon_times.get(&id).cloned()),
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            memory_state,
            resume_strategy,
            resume: meta.and_then(|m| m.resume.clone()),
            resumed_from: meta.and_then(|m| m.resumed_from.clone()),
        }
    }).collect();

    // Append remembered (non-live) sessions from metadata
    for meta in &all_metadata_sessions {
        if live_ids.contains(&meta.id) {
            continue;
        }
        let harness_id = (!meta.harness.is_empty()).then(|| meta.harness.as_str());
        let caps = capabilities_for_harness(&state.adapters, harness_id);
        let (memory_state, resume_strategy) =
            derive_memory_state(false, meta.resume.as_ref(), &caps);
        infos.push(SessionInfo {
            id: meta.id.clone(),
            label: meta.label.clone(),
            harness: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
            model: (!meta.model.is_empty()).then(|| meta.model.clone()),
            status: "ended".into(),
            cwd: meta.cwd.clone(),
            created_at: meta.created_at.clone(),
            observed_status: meta.observed_status.clone(),
            summary: meta.summary.clone(),
            next_action: meta.next_action.clone(),
            needs_user_input: meta.needs_user_input,
            detected_question: meta.detected_question.clone(),
            suggested_options: meta.suggested_options.clone(),
            blocker_description: meta.blocker_description.clone(),
            failed_command: meta.failed_command.clone(),
            failed_test: meta.failed_test.clone(),
            capacity_hints: meta.capacity_hints.clone(),
            metadata_source: Some(meta.metadata_source.clone()),
            metadata_confidence: Some(meta.metadata_confidence),
            peon_last_inference: meta.peon_last_inference.clone(),
            repo_root: meta.repo_root.clone(),
            branch: meta.branch.clone(),
            dirty: meta.dirty,
            changed_files: meta.changed_files,
            is_worktree: meta.is_worktree,
            conflict_warning: None,
            recommendation: None,
            memory_state,
            resume_strategy,
            resume: meta.resume.clone(),
            resumed_from: meta.resumed_from.clone(),
        });
    }

    let mut cwd_counts: HashMap<String, usize> = HashMap::new();
    for info in &infos {
        if info.status == "running" || info.status == "creating" {
            *cwd_counts.entry(info.cwd.clone()).or_default() += 1;
        }
    }
    for info in &mut infos {
        let ctx = git::detect(&PathBuf::from(&info.cwd));
        let count = cwd_counts.get(&info.cwd).copied().unwrap_or(1);
        info.recommendation = session_recommendation(&ctx, count);
        info.repo_root = ctx.repo_root;
        info.branch = ctx.branch;
        info.dirty = Some(ctx.dirty);
        info.changed_files = Some(ctx.changed_files);
        info.is_worktree = Some(ctx.is_worktree);
    }

    let conflict_warnings = detect_conflicts(&infos);
    for info in &mut infos {
        info.conflict_warning = conflict_warnings.iter()
            .find(|(id, _)| id == &info.id)
            .map(|(_, w)| w.clone());
    }
    Json(infos)
}

async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let handle = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&id).map(|h| h.kill_tx.clone())
    };
    match handle {
        Some(kill_tx) => {
            let _ = kill_tx.send(true);
        }
        None => return axum::http::StatusCode::NOT_FOUND,
    }
    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(h) = sessions.get_mut(&id) {
            h.info.status = "killed".to_string();
        }
    }
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(&id) {
            meta.status = "killed".to_string();
            meta.last_activity = now.clone();
            ws.metadata.write_session(&meta);
        }
        ws.metadata.append_event(&id, &metadata::Event {
            event_type: "session.killed".into(),
            timestamp: now,
            status: "killed".into(),
            observed_status: None,
            confidence: None,
        });
        // Preserve persisted terminal output: the session metadata file remains
        // (status=killed), so the scrollback should remain accessible for the
        // resulting remembered session.
    }
    drop(ws_guard);
    state.peon.last_output.write().unwrap().remove(&id);
    state.peon.last_inference.write().unwrap().remove(&id);
    axum::http::StatusCode::OK
}

async fn forget_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    {
        let sessions = state.sessions.lock().unwrap();
        if let Some(h) = sessions.get(&id) {
            if h.info.status == "live" || h.info.status == "creating" || h.info.status == "running" {
                return (axum::http::StatusCode::CONFLICT, "Cannot forget a live session. Kill it first.").into_response();
            }
        }
    }

    let ws_guard = state.workspace.lock().unwrap();
    let ws = match &*ws_guard {
        Some(ws) => ws,
        None => return axum::http::StatusCode::CONFLICT.into_response(),
    };

    if ws.metadata.read_session(&id).is_none() {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }

    if let Err(e) = ws.metadata.delete_session(&id) {
        tracing::error!("failed to delete session {id}: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if let Err(e) = ws.metadata.delete_events(&id) {
        tracing::error!("failed to delete events for {id}: {e}");
    }
    let _ = ws.metadata.clear_last_active_session_if_matches(&id);
    drop(ws_guard);

    state.peon.last_output.write().unwrap().remove(&id);
    state.peon.last_inference.write().unwrap().remove(&id);

    axum::http::StatusCode::OK.into_response()
}

#[derive(Deserialize)]
struct RetentionRequest {
    #[serde(rename = "maxSessions", default)]
    max_sessions: usize,
    #[serde(rename = "maxAgeDays", default)]
    max_age_days: u32,
}

async fn set_retention(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RetentionRequest>,
) -> impl IntoResponse {
    let mut config = state.retention_config.write().await;
    config.max_sessions = req.max_sessions;
    config.max_age_days = req.max_age_days;
    tracing::info!(
        "retention config updated: max_sessions={} max_age_days={}",
        config.max_sessions,
        config.max_age_days
    );
    axum::http::StatusCode::OK
}

async fn retention_cleanup_task(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;

        let config = state.retention_config.read().await.clone();
        if config.max_sessions == 0 && config.max_age_days == 0 {
            continue;
        }

        let all_sessions = {
            let ws_guard = state.workspace.lock().unwrap();
            match &*ws_guard {
                Some(ws) => ws.metadata.read_all_sessions(),
                None => continue,
            }
        };

        let live_ids: std::collections::HashSet<String> = {
            let sessions = state.sessions.lock().unwrap();
            sessions
                .iter()
                .filter(|(_, h)| {
                    h.info.status == "live"
                        || h.info.status == "creating"
                        || h.info.status == "running"
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        let mut candidates: Vec<_> = all_sessions
            .into_iter()
            .filter(|s| !live_ids.contains(&s.id))
            .collect();

        if candidates.is_empty() {
            continue;
        }

        candidates.sort_by(|a, b| a.last_activity.cmp(&b.last_activity));

        if config.max_age_days > 0 {
            let cutoff = chrono::Utc::now()
                - chrono::Duration::days(config.max_age_days as i64);
            let mut expired: Vec<String> = Vec::new();
            for s in &candidates {
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&s.last_activity) {
                    if parsed < cutoff {
                        expired.push(s.id.clone());
                    }
                }
            }
            if !expired.is_empty() {
                let ws_guard = state.workspace.lock().unwrap();
                if let Some(ref ws) = *ws_guard {
                    for id in &expired {
                        tracing::info!("retention: deleting expired session {id}");
                        let _ = ws.metadata.delete_session(id);
                        let _ = ws.metadata.delete_events(id);
                        let _ = ws.metadata.clear_last_active_session_if_matches(id);
                    }
                }
                candidates.retain(|s| !expired.contains(&s.id));
            }
        }

        if config.max_sessions > 0 && candidates.len() > config.max_sessions {
            let to_delete = candidates.len() - config.max_sessions;
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                for s in candidates.iter().take(to_delete) {
                    tracing::info!(
                        "retention: deleting session {} (exceeds max {})",
                        s.id,
                        config.max_sessions
                    );
                    let _ = ws.metadata.delete_session(&s.id);
                    let _ = ws.metadata.delete_events(&s.id);
                    let _ = ws.metadata.clear_last_active_session_if_matches(&s.id);
                }
            }
        }
    }
}

#[derive(Serialize)]
struct TerminalOutputResponse {
    lines: Vec<String>,
}

async fn get_terminal_output(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let state_clone = state.clone();
    let id_clone = id.clone();
    let lines = tokio::task::spawn_blocking(move || {
        let ws_guard = state_clone.workspace.lock().unwrap();
        match &*ws_guard {
            Some(ws) => ws.metadata.read_terminal_output(&id_clone, metadata::TERMINAL_OUTPUT_MAX_LINES),
            None => Vec::new(),
        }
    })
    .await
    .unwrap_or_default();
    Json(TerminalOutputResponse { lines })
}

async fn session_terminal_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let session_status = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&id).map(|h| h.info.status.clone())
    };

    match session_status {
        None => {
            ws.on_upgrade(|mut ws| async move {
                let _ = ws
                    .send(Message::Text("session not found".into()))
                    .await;
                let _ = ws.close().await;
            })
        }
        Some(ref status) if status == "killed" || status == "ended" || status == "error" => {
            let msg = format!("session {status}");
            ws.on_upgrade(move |mut ws| async move {
                let _ = ws.send(Message::Text(msg.into())).await;
                let _ = ws.close().await;
            })
        }
        Some(_) => {
            ws.on_upgrade(move |ws| handle_session_terminal(ws, id, state))
        }
    }
}

fn iso_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

async fn peon_loop(state: Arc<AppState>) {
    let interval = state.peon.config.interval_secs;
    tracing::info!("Peon started (interval={interval}s, harness={})", state.peon.config.harness);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let now = tokio::time::Instant::now();
        let deadline = now - std::time::Duration::from_secs(interval);

        // Find sessions with new output that has gone silent
        let candidates: Vec<String> = {
            let last_output = state.peon.last_output.read().unwrap();
            let last_inference = state.peon.last_inference.read().unwrap();
            let in_flight = state.peon.in_flight.read().unwrap();

            last_output.iter()
                .filter(|(id, &t)| {
                    if t > deadline { return false; }
                    !last_inference.contains_key(*id) && !in_flight.contains(*id)
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        for session_id in candidates {
            {
                let mut in_flight = state.peon.in_flight.write().unwrap();
                if !in_flight.insert(session_id.clone()) {
                    continue;
                }
            }

            let output_snapshot = {
                let sessions = state.sessions.lock().unwrap();
                match sessions.get(&session_id) {
                    Some(handle) => handle.output_buffer.snapshot(),
                    None => {
                        state.peon.in_flight.write().unwrap().remove(&session_id);
                        continue;
                    }
                }
            };

            if output_snapshot.is_empty() {
                state.peon.in_flight.write().unwrap().remove(&session_id);
                continue;
            }

            let config = state.peon.config.clone();
            let state_clone = state.clone();
            let id = session_id.clone();

            tokio::task::spawn_blocking(move || {
                let inference = peon::run_inference(&config, &output_snapshot);
                let now_iso = iso_now();

                if let Some(inf) = inference {
                    let ws_guard = state_clone.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        let should_write = ws.metadata.read_session(&id)
                            .map(|m| {
                                let age = ws.metadata.session_modified_secs_ago(&id);
                                peon::should_overwrite(&m.metadata_source, age)
                            })
                            .unwrap_or(true);

                        if should_write {
                            ws.metadata.merge_peon_inference(&id, &inf, &now_iso);
                        } else {
                            tracing::debug!("Peon: skipping {id}, higher-priority source exists");
                        }
                    }

                }

                let mut last_inf = state_clone.peon.last_inference.write().unwrap();
                last_inf.insert(id.clone(), now_iso);
                drop(last_inf);
                state_clone.peon.in_flight.write().unwrap().remove(&id);
            });
        }
    }
}

fn shell_cmd() -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        (
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()),
            vec![],
        )
    } else {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        (shell, vec!["-i".into(), "-l".into()])
    }
}

fn terminal_env_overrides() -> [(&'static str, &'static str); 5] {
    [
        ("TERM", "xterm-256color"),
        ("COLORTERM", "truecolor"),
        ("FORCE_COLOR", "1"),
        ("CLICOLOR", "1"),
        ("TERM_PROGRAM", "OrkWorks"),
    ]
}

fn default_shell_command(cwd: String) -> harness::CommandSpec {
    let (program, args) = shell_cmd();
    harness::CommandSpec { program, args, cwd }
}

fn default_capabilities() -> harness::HarnessCapabilities {
    harness::HarnessCapabilities {
        launch: true,
        resume_exact: false,
        resume_latest_in_cwd: false,
        resume_latest_in_repo: false,
        detect_session_id: false,
        detect_model: false,
        detect_context_usage: false,
        detect_capacity: false,
        native_voice: false,
    }
}

fn builtin_adapters() -> HashMap<String, harness::HarnessAdapter> {
    let (program, args) = shell_cmd();
    let mut map = HashMap::new();

    let generic = harness::HarnessAdapter::template(
        "generic-shell",
        "Generic Shell",
        default_capabilities(),
        harness::CommandTemplate {
            command: program.clone(),
            args: args.clone(),
        },
        None,
        None,
    );
    map.insert("generic-shell".into(), generic);

    let opencode_caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: true,
        resume_latest_in_cwd: true,
        resume_latest_in_repo: true,
        detect_session_id: true,
        detect_model: true,
        detect_context_usage: true,
        detect_capacity: true,
        native_voice: false,
    };
    let opencode = harness::HarnessAdapter::template(
        "opencode",
        "OpenCode",
        opencode_caps.clone(),
        harness::CommandTemplate {
            command: "opencode".into(),
            args: vec![],
        },
        Some(harness::CommandTemplate {
            command: "opencode".into(),
            args: vec!["--session".into(), "{harnessSessionId}".into()],
        }),
        Some(harness::CommandTemplate {
            command: "opencode".into(),
            args: vec!["--continue".into()],
        }),
    );
    map.insert("opencode".into(), opencode);

    let claude_caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: true,
        resume_latest_in_cwd: true,
        resume_latest_in_repo: true,
        detect_session_id: true,
        detect_model: true,
        detect_context_usage: true,
        detect_capacity: true,
        native_voice: false,
    };
    let claude = harness::HarnessAdapter::template(
        "claude-code",
        "Claude Code",
        claude_caps.clone(),
        harness::CommandTemplate {
            command: "claude".into(),
            args: vec![],
        },
        Some(harness::CommandTemplate {
            command: "claude".into(),
            args: vec!["--resume".into(), "{harnessSessionId}".into()],
        }),
        Some(harness::CommandTemplate {
            command: "claude".into(),
            args: vec!["--continue".into()],
        }),
    );
    map.insert("claude-code".into(), claude);

    map
}

fn capabilities_for_harness(
    adapters: &HashMap<String, harness::HarnessAdapter>,
    harness_id: Option<&str>,
) -> harness::HarnessCapabilities {
    match harness_id {
        Some(h) if !h.is_empty() => adapters
            .get(h)
            .map(|a| a.capabilities.clone())
            .unwrap_or_else(default_capabilities),
        _ => default_capabilities(),
    }
}

fn adapter_for_harness<'a>(
    adapters: &'a HashMap<String, harness::HarnessAdapter>,
    harness_id: Option<&str>,
) -> &'a harness::HarnessAdapter {
    match harness_id {
        Some(h) if !h.is_empty() => adapters.get(h),
        _ => None,
    }
    .unwrap_or_else(|| adapters.get("generic-shell").unwrap())
}

fn derive_memory_state(
    is_live: bool,
    resume: Option<&harness::ResumeMemory>,
    capabilities: &harness::HarnessCapabilities,
) -> (MemoryState, harness::ResumeStrategy) {
    if is_live {
        return (MemoryState::Live, harness::ResumeStrategy::None);
    }
    let Some(resume) = resume else {
        return (MemoryState::Remembered, harness::ResumeStrategy::None);
    };
    let strategy = harness::select_resume_strategy(resume, capabilities);
    if strategy == harness::ResumeStrategy::None {
        (MemoryState::Unsupported, strategy)
    } else {
        (MemoryState::Resumable, strategy)
    }
}

#[derive(Debug, PartialEq)]
enum TerminalAction {
    Input(String),
    Resize { rows: u16, cols: u16 },
    Kill,
    Noop,
}

fn dispatch_terminal_message(msg: &serde_json::Value) -> TerminalAction {
    match msg["type"].as_str() {
        Some("input") => {
            let data = msg["data"].as_str().unwrap_or("").to_string();
            TerminalAction::Input(data)
        }
        Some("resize") => {
            let rows = msg["rows"].as_u64().unwrap_or(24) as u16;
            let cols = msg["cols"].as_u64().unwrap_or(80) as u16;
            TerminalAction::Resize { rows, cols }
        }
        Some("kill") => TerminalAction::Kill,
        _ => TerminalAction::Noop,
    }
}

fn should_forward_terminal_env(key: &str) -> bool {
    key != "NODE_OPTIONS"
        && key != "VSCODE_INSPECTOR_OPTIONS"
        && !key.starts_with("VSCODE_")
        && !key.starts_with("ELECTRON_")
}

#[cfg(unix)]
fn make_pty_system() -> UnixPtySystem {
    UnixPtySystem {}
}
#[cfg(windows)]
fn make_pty_system() -> ConPtySystem {
    ConPtySystem {}
}

fn set_session_status(state: &Arc<AppState>, id: &str, status: &str) {
    let session_resume = {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(id) {
            handle.info.status = status.to_string();
            (handle.info.resume.clone(), handle.info.resumed_from.clone())
        } else {
            (None, None)
        }
    };
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(id) {
            meta.status = status.to_string();
            meta.last_activity = now.clone();
            if session_resume.0.is_some() {
                meta.resume = session_resume.0;
            }
            if session_resume.1.is_some() {
                meta.resumed_from = session_resume.1;
            }
            ws.metadata.write_session(&meta);
        }
        ws.metadata.append_event(id, &metadata::Event {
            event_type: "session.status".into(),
            timestamp: now,
            status: status.to_string(),
            observed_status: None,
            confidence: None,
        });
    }
}

async fn handle_session_terminal(mut ws: WebSocket, id: String, state: Arc<AppState>) {
    let kill_result = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&id).map(|h| h.kill_tx.subscribe())
    };

    let mut kill_rx = match kill_result {
        Some(rx) => rx,
        None => {
            let _ = ws.close().await;
            return;
        }
    };

    if *kill_rx.borrow() {
        set_session_status(&state, &id, "killed");
        let _ = ws.close().await;
        return;
    }

    {
        let should_reject = {
            let sessions = state.sessions.lock().unwrap();
            sessions
                .get(&id)
                .map(|h| {
                    let s = &h.info.status;
                    s == "killed" || s == "ended" || s == "error"
                })
                .unwrap_or(false)
        };
        if should_reject {
            tracing::warn!("rejected terminal WebSocket for {id}: session in terminal state");
            let _ = ws.close().await;
            return;
        }
    }

    let pty_sys = make_pty_system();
    let pty_size = PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = match pty_sys.openpty(pty_size) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("failed to open PTY: {e}");
            set_session_status(&state, &id, "error");
            let _ = ws.close().await;
            return;
        }
    };

    let cwd = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&id)
            .map(|h| h.info.cwd.clone())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "/".into())
            })
    };

    let command = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&id)
            .map(|h| h.command.clone())
            .unwrap_or_else(|| default_shell_command(cwd.clone()))
    };

    let mut cmd = CommandBuilder::new(&command.program);
    cmd.args(&command.args);
    cmd.cwd(&command.cwd);
    for (key, value) in std::env::vars() {
        if should_forward_terminal_env(&key) {
            cmd.env(&key, &value);
        } else {
            cmd.env_remove(&key);
        }
    }
    for (key, value) in terminal_env_overrides() {
        cmd.env(key, value);
    }

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to spawn shell: {e}");
            set_session_status(&state, &id, "error");
            let _ = ws.close().await;
            return;
        }
    };

    let mut reader = match pair.master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("failed to clone PTY reader: {e}");
            set_session_status(&state, &id, "error");
            let _ = ws.close().await;
            return;
        }
    };

    let mut writer = match pair.master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("failed to take PTY writer: {e}");
            set_session_status(&state, &id, "error");
            let _ = ws.close().await;
            return;
        }
    };

    set_session_status(&state, &id, "running");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let id_for_reader = id.clone();

    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];
    loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!("PTY read error for session {}: {}", id_for_reader, e);
                    break;
                }
            }
        }
    });

    // Serial persistence writer: drains lines from an unbounded channel so that
    // append + trim never race and chunks are persisted in arrival order.
    let (persist_tx, mut persist_rx) =
        tokio::sync::mpsc::unbounded_channel::<Vec<String>>();
    let persist_state = state.clone();
    let persist_id = id.clone();
    let persist_writer = tokio::spawn(async move {
        while let Some(lines) = persist_rx.recv().await {
            let st = persist_state.clone();
            let i = persist_id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let ws_guard = st.workspace.lock().unwrap();
                if let Some(ref ws) = *ws_guard {
                    ws.metadata.append_terminal_output_lines(&i, &lines);
                }
            })
            .await;
        }
    });

    // Byte-level buffer of unflushed terminal output. We split on raw '\n' so a
    // chunk that splits a multi-byte UTF-8 sequence or breaks a line in the middle
    // doesn't corrupt persistence: only complete lines are written, the rest
    // stays here until more bytes arrive.
    let mut persist_buffer: Vec<u8> = Vec::new();

    loop {
        tokio::select! {
            _ = kill_rx.changed() => {
                if *kill_rx.borrow() {
                    tracing::info!("kill signal received for session {}", id);
                    let _ = child.kill();
                    set_session_status(&state, &id, "killed");
                    break;
                }
            }
            Some(data) = rx.recv() => {
                persist_buffer.extend_from_slice(&data);

                let mut raw_persist_lines: Vec<String> = Vec::new();
                while let Some(nl) = persist_buffer.iter().position(|&b| b == b'\n') {
                    let line: Vec<u8> = persist_buffer.drain(..=nl).collect();
                    // Strip the trailing \n (and a preceding \r if present) so persisted
                    // lines are bare content; replay re-adds line terminators.
                    let end = if line.ends_with(b"\r\n") {
                        line.len() - 2
                    } else {
                        line.len() - 1
                    };
                    raw_persist_lines.push(String::from_utf8_lossy(&line[..end]).into_owned());
                }

                if state.peon.config.enabled {
                    let mut sessions = state.sessions.lock().unwrap();
                    if let Some(handle) = sessions.get_mut(&id) {
                        for raw in &raw_persist_lines {
                            let trimmed = raw.trim();
                            if !trimmed.is_empty() {
                                handle.output_buffer.push(trimmed.to_string());
                            }
                        }
                    }
                }

                // Any PTY traffic at all counts as activity for Peon debounce —
                // including chunks that haven't yet completed a line.
                if state.peon.config.enabled {
                    state.peon.last_output.write().unwrap()
                        .insert(id.clone(), tokio::time::Instant::now());
                    state.peon.last_inference.write().unwrap().remove(&id);
                }

                if !raw_persist_lines.is_empty() {
                    let _ = persist_tx.send(raw_persist_lines);
                }

                if ws.send(Message::Binary(data)).await.is_err() {
                    break;
                }
            }
            msg = ws.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let val: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        match dispatch_terminal_message(&val) {
                            TerminalAction::Input(data) => {
                                let _ = writer.write_all(data.as_bytes());
                                let _ = writer.flush();

                                if state.peon.config.enabled && !data.is_empty() {
                                    state.peon.last_output.write().unwrap()
                                        .insert(id.clone(), tokio::time::Instant::now());
                                    state.peon.last_inference.write().unwrap().remove(&id);
                                }
                            }
                            TerminalAction::Resize { rows, cols } => {
                                if let Err(e) = pair.master.resize(PtySize {
                                    rows,
                                    cols,
                                    pixel_width: 0,
                                    pixel_height: 0,
                                }) {
                                    tracing::warn!("PTY resize error: {e}");
                                }
                            }
                            TerminalAction::Kill => {
                                tracing::info!("kill message received for session {}", id);
                                let _ = child.kill();
                                set_session_status(&state, &id, "killed");
                                break;
                            }
                            TerminalAction::Noop => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        let _ = child.kill();
                        if *kill_rx.borrow() {
                            set_session_status(&state, &id, "killed");
                        } else {
                            set_session_status(&state, &id, "ended");
                        }
                        break;
                    }
                    _ => {
                        let _ = child.kill();
                        set_session_status(&state, &id, "error");
                        break;
                    }
                }
            }
        }
    }

    {
        // Flush any unterminated tail so the user's last visible line survives.
        if !persist_buffer.is_empty() {
            let tail = String::from_utf8_lossy(&persist_buffer).into_owned();
            let _ = persist_tx.send(vec![tail]);
        }
        // Close the channel and let the serial writer drain all pending appends
        // before the trim runs, so trimming never races with a write.
        drop(persist_tx);
        let _ = persist_writer.await;

        let state_clone = state.clone();
        let id_clone = id.clone();
        tokio::task::spawn_blocking(move || {
            let ws_guard = state_clone.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.trim_terminal_output(&id_clone, metadata::TERMINAL_OUTPUT_MAX_LINES);
            }
        });
    }

    tracing::info!("session {} terminal ended", id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_env_overrides_force_color_capability() {
        let overrides = terminal_env_overrides();

        assert!(overrides.contains(&("TERM", "xterm-256color")));
        assert!(overrides.contains(&("COLORTERM", "truecolor")));
        assert!(overrides.contains(&("FORCE_COLOR", "1")));
        assert!(overrides.contains(&("CLICOLOR", "1")));
        assert!(overrides.contains(&("TERM_PROGRAM", "OrkWorks")));
    }

    #[test]
    fn terminal_env_filter_removes_launcher_debug_variables() {
        assert!(!should_forward_terminal_env("NODE_OPTIONS"));
        assert!(!should_forward_terminal_env("VSCODE_INSPECTOR_OPTIONS"));
        assert!(!should_forward_terminal_env("VSCODE_PID"));
        assert!(!should_forward_terminal_env("ELECTRON_RUN_AS_NODE"));
    }

    #[test]
    fn terminal_env_filter_keeps_normal_shell_variables() {
        assert!(should_forward_terminal_env("PATH"));
        assert!(should_forward_terminal_env("HOME"));
        assert!(should_forward_terminal_env("SHELL"));
        assert!(should_forward_terminal_env("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn terminal_message_dispatches_kill() {
        let msg = serde_json::json!({"type": "kill"});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Kill);
    }

    #[test]
    fn terminal_message_dispatches_input() {
        let msg = serde_json::json!({"type": "input", "data": "hello"});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Input("hello".into()));
    }

    #[test]
    fn terminal_message_dispatches_resize() {
        let msg = serde_json::json!({"type": "resize", "rows": 40, "cols": 120});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Resize { rows: 40, cols: 120 });
    }

    #[test]
    fn terminal_message_dispatches_unknown_as_noop() {
        let msg = serde_json::json!({"type": "unknown"});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Noop);
    }

    #[test]
    fn session_registry_create_and_list() {
        let state = Arc::new(AppState {
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
        });

        assert!(state.sessions.lock().unwrap().is_empty());

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "test-1".to_string();
        let info = SessionInfo {
            id: id.clone(),
            label: "Test".into(),
            harness: None,
            model: None,
            status: "creating".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
            observed_status: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            metadata_source: None,
            metadata_confidence: None,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        };

        state
            .sessions
            .lock()
            .unwrap()
            .insert(id, SessionHandle { info: info.clone(), kill_tx, output_buffer: peon::RingBuffer::new(200), command: default_shell_command("/tmp".into()) });

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        let stored = sessions.get("test-1").unwrap();
        assert_eq!(stored.info.label, "Test");
        assert_eq!(stored.info.status, "creating");
    }

    #[test]
    fn set_session_status_updates_registry() {
        let state = Arc::new(AppState {
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
        });

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "test-2".to_string();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            SessionHandle {
                info: SessionInfo {
                    id: id.clone(),
                    label: "Test".into(),
                    harness: None,
                    model: None,
                    status: "creating".into(),
                    cwd: "/tmp".into(),
                    created_at: "now".into(),
                    observed_status: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    metadata_source: None,
                    metadata_confidence: None,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
            },
        );

        set_session_status(&state, "test-2", "running");
        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get("test-2")
                .unwrap()
                .info
                .status,
            "running"
        );

        set_session_status(&state, "test-2", "ended");
        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get("test-2")
                .unwrap()
                .info
                .status,
            "ended"
        );
    }

    #[test]
    fn kill_signal_detected_by_subscriber() {
        let (kill_tx, _rx) = tokio::sync::watch::channel(false);

        let _ = kill_tx.send(true);

        // subscribe after send — should see current value as true
        let rx = kill_tx.subscribe();
        assert!(*rx.borrow());
    }

    #[test]
    fn kill_signal_not_seen_when_false() {
        let (kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        drop(kill_rx);

        let rx = kill_tx.subscribe();
        assert!(!*rx.borrow());
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
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"path\":\"/tmp\""));
        assert!(json.contains("\"repo_root\":null"));
        assert!(json.contains("\"branch\":null"));
        assert!(json.contains("\"dirty\":null"));
        assert!(json.contains("\"lastActiveSessionId\":null"));
    }

    #[test]
    fn session_info_includes_metadata_fields() {
        let info = SessionInfo {
            id: "test".into(),
            label: "Test".into(),
            harness: None,
            model: None,
            status: "running".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
            observed_status: Some("waiting_for_input".into()),
            summary: Some("Needs approval".into()),
            next_action: Some("Choose an option".into()),
            needs_user_input: Some(true),
            detected_question: Some("Proceed?".into()),
            suggested_options: Some(vec!["yes".into(), "no".into()]),
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            metadata_source: Some("process".into()),
            metadata_confidence: Some(1.0),
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"metadataSource\":\"process\""));
        assert!(json.contains("\"metadataConfidence\":1.0"));
        assert!(json.contains("\"observedStatus\":\"waiting_for_input\""));
        assert!(json.contains("\"needsUserInput\":true"));
    }

    #[test]
    fn session_info_without_metadata_is_valid() {
        let info = SessionInfo {
            id: "test".into(),
            label: "Test".into(),
            harness: None,
            model: None,
            status: "creating".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
            observed_status: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            metadata_source: None,
            metadata_confidence: None,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"metadataSource\":null"));
        assert!(json.contains("\"metadataConfidence\":null"));
    }

    #[test]
    fn detect_conflicts_warns_on_multiple_dirty_sessions() {
        let sessions = vec![
            SessionInfo {
                id: "a".into(), label: "A".into(), harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(true),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
            SessionInfo {
                id: "b".into(), label: "B".into(), harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(true),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
        ];
        let warnings = detect_conflicts(&sessions);
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|(id, _)| id == "a"));
        assert!(warnings.iter().any(|(id, _)| id == "b"));
        for (_, w) in &warnings {
            assert!(w.contains("2 sessions"));
        }
    }

    #[test]
    fn detect_conflicts_no_warning_on_clean_sessions() {
        let sessions = vec![
            SessionInfo {
                id: "a".into(), label: "A".into(), harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(false),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
            SessionInfo {
                id: "b".into(), label: "B".into(), harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(false),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
        ];
        let warnings = detect_conflicts(&sessions);
        assert!(warnings.is_empty());
    }

    #[test]
    fn detect_conflicts_no_warning_when_dirty_is_none() {
        let sessions = vec![
            SessionInfo {
                id: "a".into(), label: "A".into(), harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: None,
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
            SessionInfo {
                id: "b".into(), label: "B".into(), harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: None,
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
        ];
        let warnings = detect_conflicts(&sessions);
        assert!(warnings.is_empty());
    }

    #[tokio::test]
    async fn test_peon_inference_writes_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        // Create a mock harness script that echoes known JSON
        let harness_path = dir.path().join("mock-harness.sh");
        std::fs::write(&harness_path, "#!/bin/bash\necho '{\"status\":\"working\",\"summary\":\"test\",\"confidence\":0.85}'\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&harness_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let state = Arc::new(AppState {
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig {
                    harness: harness_path.display().to_string(),
                    harness_args: vec!["--print".into(), "-p".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    enabled: true,
                },
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
        });

        // Create a session with some output in the ring buffer
        let session_id = "peon-test-1".to_string();
        {
            let mut sessions = state.sessions.lock().unwrap();
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = SessionHandle {
                info: SessionInfo {
                    id: session_id.clone(),
                    label: "Test".into(),
                    harness: None,
                    model: None,
                    status: "running".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "now".into(),
                    observed_status: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
            };
            handle.output_buffer.push("running cargo test...".into());
            handle.output_buffer.push("test result: ok. 5 passed; 0 failed;".into());
            sessions.insert(session_id.clone(), handle);
        }

        // Write initial metadata
        {
            let ws = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    phase: "".into(),
                    observed_status: None,
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
                    created_at: "now".into(),
                    last_activity: "now".into(),
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resumed_from: None,
                });
            }
        }

        // Set last_output to trigger inference (5s ago = past debounce interval)
        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        // Run peon_loop in background
        tokio::spawn(peon_loop(state.clone()));

        // Wait for metadata to be updated (poll up to 10 seconds)
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let ws = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws {
                if let Some(meta) = ws.metadata.read_session("peon-test-1") {
                    if meta.metadata_source == "peon" {
                        // Verify metadata was updated correctly
                        assert_eq!(meta.status, "running");
                        assert_eq!(meta.observed_status, Some("working".into()));
                        assert_eq!(meta.summary, Some("test".into()));
                        assert_eq!(meta.peon_last_inference.is_some(), true);
                        assert_eq!(meta.metadata_source, "peon");
                        assert!((meta.metadata_confidence - 0.85).abs() < 0.001);
                        return; // test passes
                    }
                }
            }
        }

        panic!("Peon did not update metadata within 10 seconds");
    }

    #[tokio::test]
    async fn peon_loop_does_not_start_duplicate_inference_while_in_flight() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let count_path = dir.path().join("count.txt");
        let harness_path = dir.path().join("slow-harness.sh");
        std::fs::write(
            &harness_path,
            format!(
                "#!/bin/bash\ncount_file='{}'\ncount=$(cat \"$count_file\" 2>/dev/null || echo 0)\ncount=$((count + 1))\necho \"$count\" > \"$count_file\"\nsleep 3\necho '{{\"observedStatus\":\"working\",\"confidence\":0.85}}'\n",
                count_path.display()
            ),
        ).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&harness_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let state = Arc::new(AppState {
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig {
                    harness: harness_path.display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    enabled: true,
                },
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
        });

        let session_id = "peon-duplicate-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = SessionHandle {
                info: SessionInfo {
                    id: session_id.clone(),
                    label: "Test".into(),
                    harness: None,
                    model: None,
                    status: "running".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "now".into(),
                    observed_status: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
            };
            handle.output_buffer.push("quiet output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        state.peon.last_output.write().unwrap().insert(
            session_id,
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        task.abort();

        let count = std::fs::read_to_string(count_path)
            .unwrap_or_else(|_| "0".into())
            .trim()
            .parse::<usize>()
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn peon_loop_records_failed_inference_attempt() {
        let dir = tempfile::tempdir().unwrap();

        let state = Arc::new(AppState {
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    enabled: true,
                },
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
        });

        let session_id = "peon-failed-attempt-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = SessionHandle {
                info: SessionInfo {
                    id: session_id.clone(),
                    label: "Test".into(),
                    harness: None,
                    model: None,
                    status: "running".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "now".into(),
                    observed_status: None,
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: default_shell_command(dir.path().display().to_string()),
            };
            handle.output_buffer.push("quiet output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

        assert!(
            state.peon.last_inference.read().unwrap().contains_key(&session_id),
            "failed Peon attempts should still be recorded to avoid tight retry loops"
        );
    }

    #[test]
    fn memory_state_marks_absent_session_as_resumable_when_strategy_exists() {
        let caps = harness::HarnessCapabilities {
            launch: true,
            resume_exact: true,
            resume_latest_in_cwd: true,
            resume_latest_in_repo: false,
            detect_session_id: true,
            detect_model: true,
            detect_context_usage: false,
            detect_capacity: false,
            native_voice: false,
        };
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::Exact,
            harness_session_id: Some("sess-1".into()),
            latest_fallback: true,
            last_seen_at: None,
        };

        let (memory_state, strategy) = derive_memory_state(false, Some(&resume), &caps);

        assert_eq!(memory_state, MemoryState::Resumable);
        assert_eq!(strategy, harness::ResumeStrategy::Exact);
    }

    #[test]
    fn memory_state_marks_active_session_as_live() {
        let caps = harness::HarnessCapabilities {
            launch: true,
            resume_exact: false,
            resume_latest_in_cwd: false,
            resume_latest_in_repo: false,
            detect_session_id: false,
            detect_model: false,
            detect_context_usage: false,
            detect_capacity: false,
            native_voice: false,
        };

        let (memory_state, strategy) = derive_memory_state(true, None, &caps);

        assert_eq!(memory_state, MemoryState::Live);
        assert_eq!(strategy, harness::ResumeStrategy::None);
    }

    #[test]
    fn generic_shell_memory_state_is_not_resumable() {
        let capabilities = default_capabilities();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::Exact,
            harness_session_id: Some("sess-1".into()),
            latest_fallback: true,
            last_seen_at: None,
        };

        let (memory_state, strategy) = derive_memory_state(false, Some(&resume), &capabilities);
        let command = builtin_adapters().get("generic-shell").unwrap().build_resume_command(&harness::ResumeRequest {
            strategy: harness::ResumeStrategy::Exact,
            cwd: "/tmp".into(),
            repo_root: Some("/tmp".into()),
            harness_session_id: Some("sess-1".into()),
            model: None,
        });

        assert_eq!(memory_state, MemoryState::Unsupported);
        assert_eq!(strategy, harness::ResumeStrategy::None);
        assert!(command.is_none());
    }
}
