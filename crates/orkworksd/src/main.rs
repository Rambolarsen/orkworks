use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use portable_pty::{CommandBuilder, PtySize, PtySystem};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(unix)]
use portable_pty::unix::UnixPtySystem;
#[cfg(windows)]
use portable_pty::win::conpty::ConPtySystem;

mod metadata;
mod watcher;

#[derive(Clone, Debug, Serialize)]
struct SessionInfo {
    id: String,
    label: String,
    status: String,
    cwd: String,
    created_at: String,
}

struct SessionHandle {
    info: SessionInfo,
    kill_tx: tokio::sync::watch::Sender<bool>,
}

struct AppState {
    sessions: Mutex<HashMap<String, SessionHandle>>,
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
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
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

async fn health_check() -> &'static str {
    "ok"
}

async fn create_session(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".into());

    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);

    let info = SessionInfo {
        id: id.clone(),
        label: format!("Session {}", &id[..8]),
        status: "creating".into(),
        cwd,
        created_at: chrono_now(),
    };

    let handle = SessionHandle { info: info.clone(), kill_tx };

    state.sessions.lock().unwrap().insert(id, handle);

    Json(info)
}

async fn list_sessions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let sessions = state.sessions.lock().unwrap();
    let infos: Vec<SessionInfo> = sessions.values().map(|h| h.info.clone()).collect();
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
    axum::http::StatusCode::OK
}

async fn session_terminal_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let exists = {
        let sessions = state.sessions.lock().unwrap();
        sessions.contains_key(&id)
    };

    if exists {
        ws.on_upgrade(move |ws| handle_session_terminal(ws, id, state))
    } else {
        ws.on_upgrade(|mut ws| async move {
            let _ = ws
                .send(Message::Text("session not found".into()))
                .await;
            let _ = ws.close().await;
        })
    }
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now();
    let since_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = since_epoch.as_secs();
    // ponytail: gmtime fallback; add chrono crate if full ISO-8601 needed
    format!("epoch-{}", secs)
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
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(handle) = sessions.get_mut(id) {
        handle.info.status = status.to_string();
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
        };
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id, SessionHandle { info: info.clone(), kill_tx });

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
                },
                kill_tx,
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
}
