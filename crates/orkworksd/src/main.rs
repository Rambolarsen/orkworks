use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use portable_pty::{CommandBuilder, PtySize, PtySystem};

#[cfg(unix)]
use portable_pty::unix::UnixPtySystem;
#[cfg(windows)]
use portable_pty::win::conpty::ConPtySystem;
use std::io::{Read, Write};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "orkworksd=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/terminal", get(terminal_handler))
        .layer(cors);

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

async fn terminal_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_terminal)
}

fn shell_cmd() -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        (std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()), vec![])
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
}

#[cfg(unix)]
fn make_pty_system() -> UnixPtySystem { UnixPtySystem {} }
#[cfg(windows)]
fn make_pty_system() -> ConPtySystem { ConPtySystem {} }

async fn handle_terminal(mut ws: WebSocket) {
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
            return;
        }
    };

    let (shell_bin, shell_args) = shell_cmd();
    let mut cmd = CommandBuilder::new(&shell_bin);
    cmd.args(&shell_args);
    cmd.cwd(std::env::current_dir().unwrap_or_else(|_| "/".into()));
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
            return;
        }
    };

    let mut reader = match pair.master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("failed to clone PTY reader: {e}");
            return;
        }
    };

    let mut writer = match pair.master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("failed to take PTY writer: {e}");
            return;
        }
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

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
                    tracing::warn!("PTY read error: {e}");
                    break;
                }
            }
        }
    });

    loop {
        tokio::select! {
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
                                tracing::info!("kill received, terminating child");
                                let _ = child.kill();
                                break;
                            }
                            TerminalAction::Noop => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        let _ = child.kill();
                        break;
                    }
                    _ => {
                        let _ = child.kill();
                        break;
                    }
                }
            }
        }
    }

    tracing::info!("terminal session ended");
}
