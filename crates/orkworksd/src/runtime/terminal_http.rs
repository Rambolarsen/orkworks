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

#[derive(Serialize)]
pub(crate) struct SummaryLogEntry {
    pub(crate) timestamp: String,
    pub(crate) summary: String,
    pub(crate) source: String,
    pub(crate) confidence: Option<f64>,
}

#[derive(Serialize)]
pub(crate) struct SummaryLogResponse {
    pub(crate) entries: Vec<SummaryLogEntry>,
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
    .unwrap_or_else(|error| {
        tracing::error!(%error, "terminal-output metadata task failed");
        Vec::new()
    });
    Json(TerminalOutputResponse { lines })
}

pub(crate) async fn get_summary_log(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let entries = tokio::task::spawn_blocking(move || {
        let ws_guard = state.workspace.lock().unwrap();
        match &*ws_guard {
            Some(ws) if ws.metadata.read_session(&id).is_some() => ws
                .metadata
                .read_events(&id)
                .into_iter()
                .filter_map(|event| {
                    let metadata::Event {
                        timestamp,
                        confidence,
                        summary: Some(summary),
                        source: Some(source),
                        ..
                    } = event
                    else {
                        return None;
                    };
                    Some(SummaryLogEntry {
                        timestamp,
                        summary,
                        source,
                        confidence,
                    })
                })
                .collect(),
            Some(_) => Vec::new(),
            None => Vec::new(),
        }
    })
    .await
    .unwrap_or_else(|error| {
        tracing::error!(%error, "summary-log metadata task failed");
        Vec::new()
    });
    Json(SummaryLogResponse { entries })
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

    async fn response_json(response: impl IntoResponse) -> serde_json::Value {
        let response = response.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

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

    #[tokio::test]
    async fn get_summary_log_filters_incomplete_events_and_preserves_public_shape_and_order() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "summary-session".to_string();

        {
            let ws_guard = state.workspace.lock().unwrap();
            let store = &ws_guard.as_ref().unwrap().metadata;
            store.write_session(&test_session_metadata(
                &session_id,
                "Persisted",
                dir.path().display().to_string(),
                "ended",
                "t0",
                "t0",
            ));
            for event in [
                metadata::Event {
                    event_type: "session.status".into(),
                    timestamp: "t0".into(),
                    status: "working".into(),
                    observed_status: Some("working".into()),
                    confidence: Some(0.5),
                    summary: None,
                    source: None,
                },
                metadata::Event {
                    event_type: "peon.checkpoint".into(),
                    timestamp: "t1".into(),
                    status: "working".into(),
                    observed_status: Some("working".into()),
                    confidence: Some(0.91),
                    summary: Some("First checkpoint".into()),
                    source: Some("peon".into()),
                },
                metadata::Event {
                    event_type: "peon.checkpoint".into(),
                    timestamp: "t2".into(),
                    status: "waiting_for_input".into(),
                    observed_status: Some("waiting_for_input".into()),
                    confidence: None,
                    summary: Some("Missing provenance".into()),
                    source: None,
                },
                metadata::Event {
                    event_type: "peon.checkpoint".into(),
                    timestamp: "t3".into(),
                    status: "done".into(),
                    observed_status: Some("done".into()),
                    confidence: None,
                    summary: Some("Second checkpoint".into()),
                    source: Some("agent".into()),
                },
            ] {
                store.append_event(&session_id, &event);
            }
        }

        let payload = response_json(get_summary_log(State(state), Path(session_id)).await).await;

        assert_eq!(
            payload,
            serde_json::json!({
                "entries": [
                    {
                        "timestamp": "t1",
                        "summary": "First checkpoint",
                        "source": "peon",
                        "confidence": 0.91
                    },
                    {
                        "timestamp": "t3",
                        "summary": "Second checkpoint",
                        "source": "agent",
                        "confidence": null
                    }
                ]
            })
        );
    }

    #[tokio::test]
    async fn get_summary_log_returns_empty_entries_for_absent_data() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let missing_file = response_json(
            get_summary_log(
                State(state.clone()),
                Path("unknown-session".into()),
            )
            .await,
        )
        .await;
        assert_eq!(missing_file, serde_json::json!({ "entries": [] }));

        {
            let ws_guard = state.workspace.lock().unwrap();
            ws_guard.as_ref().unwrap().metadata.append_event(
                "legacy-only",
                &metadata::Event {
                    event_type: "session.status".into(),
                    timestamp: "t0".into(),
                    status: "working".into(),
                    observed_status: None,
                    confidence: None,
                    summary: None,
                    source: None,
                },
            );
        }
        let no_checkpoints = response_json(
            get_summary_log(State(state.clone()), Path("legacy-only".into())).await,
        )
        .await;
        assert_eq!(no_checkpoints, serde_json::json!({ "entries": [] }));

        *state.workspace.lock().unwrap() = None;
        let missing_workspace = response_json(
            get_summary_log(State(state), Path("any-session".into())).await,
        )
        .await;
        assert_eq!(missing_workspace, serde_json::json!({ "entries": [] }));
    }

    #[tokio::test]
    async fn get_summary_log_returns_empty_entries_for_orphan_event_log() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "orphan-summary".to_string();
        {
            let ws_guard = state.workspace.lock().unwrap();
            ws_guard.as_ref().unwrap().metadata.append_event(
                &session_id,
                &metadata::Event {
                    event_type: "peon.inference".into(),
                    timestamp: "t1".into(),
                    status: "done".into(),
                    observed_status: Some("done".into()),
                    confidence: Some(0.9),
                    summary: Some("Orphaned checkpoint".into()),
                    source: Some("peon".into()),
                },
            );
        }

        let payload = response_json(get_summary_log(State(state), Path(session_id)).await).await;

        assert_eq!(payload, serde_json::json!({ "entries": [] }));
    }
}
