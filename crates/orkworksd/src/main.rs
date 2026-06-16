use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use portable_pty::{CommandBuilder, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
mod peon;

#[derive(Clone, Debug, Serialize)]
struct SessionInfo {
    id: String,
    label: String,
    status: String,
    cwd: String,
    created_at: String,
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
}

struct SessionHandle {
    info: SessionInfo,
    kill_tx: tokio::sync::watch::Sender<bool>,
    output_buffer: peon::RingBuffer,
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
    config: peon::PeonConfig,
}

struct AppState {
    sessions: Mutex<HashMap<String, SessionHandle>>,
    workspace: Mutex<Option<WorkspaceState>>,
    peon: PeonState,
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
            config: peon::PeonConfig::from_env(),
        },
    });

    // Start Peon background task
    if state.peon.config.enabled {
        let peon_state = state.clone();
        tokio::spawn(async move {
            peon_loop(peon_state).await;
        });
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/workspace", post(set_workspace))
        .route("/sessions", post(create_session))
        .route("/sessions", get(list_sessions))
        .route("/sessions/:id", delete(delete_session))
        .route("/sessions/:id/terminal", get(session_terminal_handler))
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

#[derive(Serialize)]
struct WorkspaceResponse {
    path: String,
    repo_root: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
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
    })
    .into_response()
}

async fn health_check() -> &'static str {
    "ok"
}

async fn create_session(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".into());

    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);

    let git_ctx = git::detect(&PathBuf::from(&cwd));
    let info = SessionInfo {
        id: id.clone(),
        label: format!("Session {}", &id[..8]),
        status: "creating".into(),
        cwd,
        created_at: iso_now(),
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
    };

    let handle = SessionHandle { info: info.clone(), kill_tx, output_buffer: peon::RingBuffer::new(state.peon.config.max_lines) };

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
            created_at: now.clone(),
            last_activity: now.clone(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: meta_git_ctx.repo_root.clone(),
            branch: meta_git_ctx.branch.clone(),
            dirty: Some(meta_git_ctx.dirty),
            changed_files: Some(meta_git_ctx.changed_files),
            is_worktree: Some(meta_git_ctx.is_worktree),
        });
        ws.metadata.append_event(&id, &metadata::Event {
            event_type: "session.created".into(),
            timestamp: now,
            status: "creating".into(),
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
    let (source_map, confidence_map) = ws_guard.as_ref().map(|ws| {
        let mut sources = HashMap::new();
        let mut confidences = HashMap::new();
        for (id, _, _, _, _) in &session_data {
            if let Some(meta) = ws.metadata.read_session(id) {
                sources.insert(id.clone(), meta.metadata_source);
                confidences.insert(id.clone(), meta.metadata_confidence);
            }
        }
        (sources, confidences)
    }).unwrap_or_default();
    drop(ws_guard);

    let peon_times = state.peon.last_inference.read().unwrap();
    let mut infos: Vec<SessionInfo> = session_data.into_iter().map(|(id, label, status, cwd, created_at)| {
        SessionInfo {
            id: id.clone(),
            label,
            status,
            cwd,
            created_at,
            metadata_source: source_map.get(&id).cloned(),
            metadata_confidence: confidence_map.get(&id).copied(),
            peon_last_inference: peon_times.get(&id).cloned(),
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
        }
    }).collect();

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
        });
    }
    drop(ws_guard);
    state.peon.last_output.write().unwrap().remove(&id);
    state.peon.last_inference.write().unwrap().remove(&id);
    axum::http::StatusCode::OK
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

            last_output.iter()
                .filter(|(id, &t)| {
                    if t > deadline { return false; }
                    !last_inference.contains_key(*id)
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        for session_id in candidates {
            let output_snapshot = {
                let sessions = state.sessions.lock().unwrap();
                match sessions.get(&session_id) {
                    Some(handle) => handle.output_buffer.snapshot(),
                    None => continue,
                }
            };

            if output_snapshot.is_empty() { continue; }

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
                            .map(|m| peon::should_overwrite(&m.metadata_source, None))
                            .unwrap_or(true);

                        if should_write {
                            // TODO: merge_peon_inference — Task 7
                            tracing::debug!("Peon: would write inference for {id}: status={:?}", inf.status);
                        } else {
                            tracing::debug!("Peon: skipping {id}, higher-priority source exists");
                        }
                    }
                }

                let mut last_inf = state_clone.peon.last_inference.write().unwrap();
                last_inf.insert(id, now_iso);
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
    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(id) {
            handle.info.status = status.to_string();
        }
    }
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(id) {
            meta.status = status.to_string();
            meta.last_activity = now.clone();
            ws.metadata.write_session(&meta);
        }
        ws.metadata.append_event(id, &metadata::Event {
            event_type: "session.status".into(),
            timestamp: now,
            status: status.to_string(),
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

    let (shell_bin, shell_args) = shell_cmd();
    let mut cmd = CommandBuilder::new(&shell_bin);
    cmd.args(&shell_args);
    cmd.cwd(&cwd);
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
                // Feed ring buffer for Peon
                if state.peon.config.enabled {
                    let text = String::from_utf8_lossy(&data);
                    let mut sessions = state.sessions.lock().unwrap();
                    if let Some(handle) = sessions.get_mut(&id) {
                        for line in text.lines() {
                            let trimmed = line.trim();
                            if !trimmed.is_empty() {
                                handle.output_buffer.push(trimmed.to_string());
                            }
                        }
                    }
                    drop(sessions);
                    let mut last_output = state.peon.last_output.write().unwrap();
                    last_output.insert(id.clone(), tokio::time::Instant::now());
                    drop(last_output);
                    // Clear last_inference so Peon will re-observe after new output
                    state.peon.last_inference.write().unwrap().remove(&id);
                }
                if ws.send(Message::Binary(data.into())).await.is_err() {
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
                config: peon::PeonConfig::from_env(),
            },
        });

        assert!(state.sessions.lock().unwrap().is_empty());

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "test-1".to_string();
        let info = SessionInfo {
            id: id.clone(),
            label: "Test".into(),
            status: "creating".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
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
        };
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id, SessionHandle { info: info.clone(), kill_tx, output_buffer: peon::RingBuffer::new(200) });

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
                config: peon::PeonConfig::from_env(),
            },
        });

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "test-2".to_string();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            SessionHandle {
                info: SessionInfo {
                    id: id.clone(),
                    label: "Test".into(),
                    status: "creating".into(),
                    cwd: "/tmp".into(),
                    created_at: "now".into(),
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
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
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
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"path\":\"/tmp\""));
        assert!(json.contains("\"repo_root\":\"/tmp\""));
        assert!(json.contains("\"branch\":\"main\""));
        assert!(json.contains("\"dirty\":false"));
    }

    #[test]
    fn workspace_response_without_git() {
        let resp = WorkspaceResponse {
            path: "/tmp".into(),
            repo_root: None,
            branch: None,
            dirty: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"path\":\"/tmp\""));
        assert!(json.contains("\"repo_root\":null"));
        assert!(json.contains("\"branch\":null"));
        assert!(json.contains("\"dirty\":null"));
    }

    #[test]
    fn session_info_includes_metadata_fields() {
        let info = SessionInfo {
            id: "test".into(),
            label: "Test".into(),
            status: "running".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
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
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"metadataSource\":\"process\""));
        assert!(json.contains("\"metadataConfidence\":1.0"));
    }

    #[test]
    fn session_info_without_metadata_is_valid() {
        let info = SessionInfo {
            id: "test".into(),
            label: "Test".into(),
            status: "creating".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
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
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"metadataSource\":null"));
        assert!(json.contains("\"metadataConfidence\":null"));
    }

    #[test]
    fn detect_conflicts_warns_on_multiple_dirty_sessions() {
        let sessions = vec![
            SessionInfo {
                id: "a".into(), label: "A".into(), status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(true),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
                peon_last_inference: None,
            },
            SessionInfo {
                id: "b".into(), label: "B".into(), status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(true),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
                peon_last_inference: None,
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
                id: "a".into(), label: "A".into(), status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(false),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
                peon_last_inference: None,
            },
            SessionInfo {
                id: "b".into(), label: "B".into(), status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(false),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
                peon_last_inference: None,
            },
        ];
        let warnings = detect_conflicts(&sessions);
        assert!(warnings.is_empty());
    }

    #[test]
    fn detect_conflicts_no_warning_when_dirty_is_none() {
        let sessions = vec![
            SessionInfo {
                id: "a".into(), label: "A".into(), status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: None,
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
                peon_last_inference: None,
            },
            SessionInfo {
                id: "b".into(), label: "B".into(), status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: None,
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
                peon_last_inference: None,
            },
        ];
        let warnings = detect_conflicts(&sessions);
        assert!(warnings.is_empty());
    }
}
