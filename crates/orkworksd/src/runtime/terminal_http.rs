use crate::runtime::terminal_runtime::handle_session_terminal;
use crate::{metadata, AppState};
use axum::{
    extract::{
        ws::{Message, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub(crate) struct TerminalOutputResponse {
    pub(crate) lines: Vec<String>,
}

pub(crate) async fn get_terminal_output(
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

pub(crate) async fn session_terminal_handler(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use axum::{extract::State, response::IntoResponse};

    #[tokio::test]
    async fn get_terminal_output_reads_persisted_terminal_history_for_dead_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "dead-session".to_string();

        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.append_terminal_output_lines(
                &session_id,
                &["first line".into(), "second line".into()],
            );
        }

        let response = get_terminal_output(State(state), Path(session_id))
            .await
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload["lines"],
            serde_json::json!(["first line", "second line"])
        );
    }
}
