//! Read-only workspace synchronization. Dirty revisions are hints; the shared
//! complete session query remains authoritative. No terminal ownership here.
use crate::{http::session_handlers::query_sessions, AppState};
use axum::{
    extract::Request,
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use notify::{EventKind, RecursiveMode, Watcher};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::watch;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct Revisions {
    sessions: u64,
    recommendations: u64,
    workspace: u64,
    watcher: u64,
}

#[derive(Clone)]
pub(crate) struct UpdateHub {
    revisions: watch::Sender<Revisions>,
    instance: String,
}
impl Default for UpdateHub {
    fn default() -> Self {
        Self {
            revisions: watch::channel(Revisions::default()).0,
            instance: uuid::Uuid::new_v4().to_string(),
        }
    }
}
impl UpdateHub {
    fn subscribe(&self) -> watch::Receiver<Revisions> {
        self.revisions.subscribe()
    }
    pub(crate) fn sessions_changed(&self) {
        self.revisions
            .send_modify(|r| r.sessions = r.sessions.wrapping_add(1));
    }
    pub(crate) fn recommendations_changed(&self) {
        self.revisions
            .send_modify(|r| r.recommendations = r.recommendations.wrapping_add(1));
    }
    fn all_changed(&self) {
        self.sessions_changed();
        self.recommendations_changed();
    }
    fn rearm(&self) {
        self.revisions
            .send_modify(|r| r.watcher = r.watcher.wrapping_add(1));
        self.all_changed();
    }
}

pub(crate) async fn invalidate_mutations(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let mutation = !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    );
    let workspace = request.uri().path() == "/workspace";
    let response = next.run(request).await;
    // Failed mutations can have completed partial work too; reconcile, never infer success.
    if mutation {
        if workspace && response.status().is_success() {
            state
                .updates
                .revisions
                .send_modify(|r| r.workspace = r.workspace.wrapping_add(1));
        }
        state.updates.all_changed();
    }
    response
}

fn origin_allowed(origin: Option<&str>) -> bool {
    let Some(origin) = origin else {
        return true;
    }; // Native clients have no browser Origin.
    if origin == "null" {
        return true;
    } // Packaged file renderer.
    url::Url::parse(origin).is_ok_and(|url| {
        url.scheme() == "http"
            && url.username().is_empty()
            && url.password().is_none()
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none()
    })
}

#[derive(Deserialize)]
pub(crate) struct Subscription {
    workspace: String,
}

pub(crate) async fn subscribe(
    State(state): State<Arc<AppState>>,
    Query(query): Query<Subscription>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if headers.get("origin").is_some_and(|h| {
        h.to_str()
            .map_or(true, |value| !origin_allowed(Some(value)))
    }) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let identity = workspace_identity(&state);
    let Some((workspace, root)) = identity.filter(|(path, _)| path == &query.workspace) else {
        return StatusCode::CONFLICT.into_response();
    };
    ws.max_message_size(4096)
        .on_upgrade(move |socket| run(socket, state, workspace, root))
        .into_response()
}

fn workspace_identity(state: &AppState) -> Option<(String, PathBuf)> {
    state.workspace.lock().ok()?.as_ref().map(|ws| {
        (
            ws.path.to_string_lossy().into_owned(),
            ws.metadata.root_path(),
        )
    })
}

fn session_delta(old: &[Value], next: &[Value]) -> Option<Value> {
    let previous: HashMap<&str, &Value> = old
        .iter()
        .filter_map(|row| Some((row["id"].as_str()?, row)))
        .collect();
    let current: HashMap<&str, &Value> = next
        .iter()
        .filter_map(|row| Some((row["id"].as_str()?, row)))
        .collect();
    let changed: Vec<&Value> = next
        .iter()
        .filter(|row| previous.get(row["id"].as_str().unwrap_or("")).copied() != Some(*row))
        .collect();
    let removed: Vec<&Value> = old
        .iter()
        .filter(|row| !current.contains_key(row["id"].as_str().unwrap_or("")))
        .map(|row| &row["id"])
        .collect();
    let order: Vec<&Value> = next.iter().map(|row| &row["id"]).collect();
    let reordered = old.iter().map(|row| &row["id"]).ne(order.iter().copied());
    if changed.is_empty() && removed.is_empty() && !reordered {
        return None;
    }
    let mut delta = json!({"changed": changed, "removed": removed});
    if reordered {
        delta["order"] = json!(order);
    }
    Some(delta)
}

/// Watch immediate metadata children, not terminal logs or arbitrary repositories.
/// Parent watches and periodic rearming cover atomic directory replacement.
fn watch_metadata(
    root: &Path,
    global: &Path,
    hub: UpdateHub,
) -> notify::Result<notify::RecommendedWatcher> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    let watched_root = root.clone();
    let global = global.canonicalize().unwrap_or_else(|_| global.to_owned());
    let watched_global = global.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        match event {
            Ok(event) if !matches!(event.kind, EventKind::Access(_)) => {
                if event.need_rescan() {
                    hub.rearm();
                    return;
                }
                for path in event.paths {
                    let relative = path.strip_prefix(&watched_root).ok();
                    let first = relative
                        .and_then(|p| p.components().next())
                        .and_then(|c| c.as_os_str().to_str());
                    if path.parent() == Some(watched_root.as_path())
                        && matches!(
                            first,
                            Some(
                                "sessions"
                                    | "session-state"
                                    | "capacity"
                                    | "workflow-observations"
                                    | "recommendations"
                                    | "taskmaster"
                            )
                        )
                    {
                        hub.rearm();
                    }
                    match first {
                        Some(
                            "sessions" | "session-state" | "capacity" | "workflow-observations",
                        ) => hub.sessions_changed(),
                        Some("recommendations" | "taskmaster" | "assessment.json") => {
                            hub.recommendations_changed()
                        }
                        _ if path.starts_with(&watched_global) => hub.recommendations_changed(),
                        _ => {}
                    }
                }
            }
            Err(error) => {
                tracing::warn!(%error, "workspace metadata watcher failed; reconciliation remains active");
                hub.rearm();
            }
            _ => {}
        }
    })?;
    for dir in [
        root.clone(),
        root.join("sessions"),
        root.join("session-state"),
        root.join("capacity"),
        root.join("workflow-observations"),
        root.join("recommendations"),
        root.join("taskmaster"),
        global,
    ] {
        if dir.is_dir() {
            watcher.watch(&dir, RecursiveMode::NonRecursive)?;
        }
    }
    Ok(watcher)
}

async fn send(socket: &mut WebSocket, message: Value) -> bool {
    // No unbounded output queue: one pending message; a slow consumer resynchronizes.
    matches!(
        tokio::time::timeout(
            Duration::from_secs(5),
            socket.send(Message::Text(message.to_string()))
        )
        .await,
        Ok(Ok(()))
    )
}

async fn run(mut socket: WebSocket, state: Arc<AppState>, workspace: String, root: PathBuf) {
    let mut changes = state.updates.subscribe(); // Register before initial query.
    let generation = changes.borrow().workspace;
    let global = crate::taskmaster::assessment_access::runtime_for(&state)
        .root_path()
        .to_owned();
    let mut watcher = watch_metadata(&root, &global, state.updates.clone())
        .map_err(|error| tracing::warn!(%error, "workspace watcher unavailable"))
        .ok();
    let mut baseline: Option<Vec<Value>> = None;
    let mut sequence = 0_u64;
    let mut consumed = Revisions::default();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(10));
    heartbeat.tick().await;
    let mut reconcile = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_secs(30),
        Duration::from_secs(30),
    );
    let mut next_projection = tokio::time::Instant::now();
    let mut full = true;
    loop {
        let captured = *changes.borrow_and_update();
        if captured.workspace != generation
            || workspace_identity(&state) != Some((workspace.clone(), root.clone()))
        {
            break;
        }
        if captured.watcher != consumed.watcher {
            watcher = watch_metadata(&root, &global, state.updates.clone()).ok();
        }
        let sessions_dirty = baseline.is_none() || full || captured.sessions != consumed.sessions;
        let recommendations_dirty =
            baseline.is_none() || full || captured.recommendations != consumed.recommendations;
        if sessions_dirty {
            // Changes during the await remain visible through the captured revision.
            tokio::time::sleep_until(next_projection).await;
            next_projection = tokio::time::Instant::now() + Duration::from_secs(2);
            let query_state = state.clone();
            let result = tokio::task::spawn_blocking(move || query_sessions(&query_state)).await;
            if workspace_identity(&state) != Some((workspace.clone(), root.clone()))
                || changes.borrow().workspace != generation
            {
                break;
            }
            let Ok(Ok(rows)) = result else {
                let _ = send(&mut socket, json!({"type":"unavailable"})).await;
                break; // Preserve baseline; reconnect must start from complete authoritative query.
            };
            let Ok(rows) = rows
                .into_iter()
                .map(serde_json::to_value)
                .collect::<Result<Vec<_>, _>>()
            else {
                break;
            };
            let mut payload = if let Some(old) = &baseline {
                session_delta(old, &rows).map(|mut message| {
                    message["type"] = json!("sessions-delta");
                    message
                })
            } else {
                Some(json!({"type":"snapshot", "sessions":rows}))
            };
            if let Some(ref mut message) = payload {
                message["version"] = json!(1);
                message["instance"] = json!(state.updates.instance);
                message["workspace"] = json!(workspace);
                message["generation"] = json!(generation);
                if baseline.is_some() {
                    message["base"] = json!(sequence);
                    sequence += 1;
                }
                message["sequence"] = json!(sequence);
                if !send(&mut socket, message.take()).await {
                    break;
                }
            }
            baseline = Some(rows);
        }
        if recommendations_dirty {
            let base = sequence;
            sequence += 1;
            if !send(
                &mut socket,
                json!({"type":"recommendations-invalidated", "version":1,
                "instance":state.updates.instance, "workspace":workspace, "generation":generation,
                "base":base, "sequence":sequence}),
            )
            .await
            {
                break;
            }
        }
        consumed = captured;
        full = false;
        tokio::select! {
            changed = changes.changed() => { if changed.is_err() { break; } }
            _ = reconcile.tick() => {
                full = true;
                // Recreate directory watches after errors or directory replacement.
                watcher = watch_metadata(&root, &global, state.updates.clone()).ok();
            }
            _ = heartbeat.tick() => {
                if !send(&mut socket, json!({"type":"heartbeat"})).await { break; }
                // Check workspace identity even when all producers are idle.
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) if text == "refresh" => { full = true; }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
                    _ => break,
                }
            }
        }
    }
    drop(watcher);
    let _ = socket.close().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn delta_sends_only_changed_rows_and_explicit_removals() {
        let old = vec![json!({"id":"a","label":"old"}), json!({"id":"b"})];
        let next = vec![json!({"id":"a","label":"new"}), json!({"id":"c"})];
        let delta = session_delta(&old, &next).unwrap();
        assert_eq!(
            delta["changed"],
            json!([{"id":"a","label":"new"},{"id":"c"}])
        );
        assert_eq!(delta["removed"], json!(["b"]));
        assert_eq!(delta["order"], json!(["a", "c"]));
        assert!(session_delta(&next, &next).is_none());
    }

    #[test]
    fn reorder_is_published_without_resending_unchanged_rows() {
        let old = vec![json!({"id":"a"}), json!({"id":"b"})];
        let next = vec![json!({"id":"b"}), json!({"id":"a"})];
        let delta = session_delta(&old, &next).unwrap();
        assert_eq!(delta["changed"], json!([]));
        assert_eq!(delta["order"], json!(["b", "a"]));
    }

    #[test]
    fn rejects_remote_browser_origins_without_changing_legacy_routes() {
        assert!(origin_allowed(None));
        assert!(origin_allowed(Some("null")));
        assert!(origin_allowed(Some("http://localhost:5173")));
        assert!(origin_allowed(Some("http://127.0.0.1:5173")));
        assert!(!origin_allowed(Some("https://example.com")));
        assert!(!origin_allowed(Some("http://localhost.example.com")));
        assert!(!origin_allowed(Some("file:///tmp/page.html")));
    }

    #[test]
    fn dirty_signal_retains_changes_received_during_capture() {
        let hub = UpdateHub::default();
        let mut receiver = hub.subscribe();
        let captured = *receiver.borrow_and_update();
        hub.sessions_changed();
        assert_ne!(receiver.borrow().sessions, captured.sessions);
        hub.recommendations_changed();
        assert_ne!(receiver.borrow().recommendations, captured.recommendations);
    }
    async fn next_json(
        socket: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> Value {
        use futures_util::StreamExt;
        let message = tokio::time::timeout(Duration::from_secs(8), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        serde_json::from_str(message.to_text().unwrap()).unwrap()
    }

    async fn next_session_update(
        socket: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        cursor: &mut u64,
    ) -> Value {
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let message = next_json(socket).await;
                if message["type"] == "heartbeat" {
                    continue;
                }
                if message["type"] == "unavailable" {
                    return message;
                }
                assert_eq!(message["base"], json!(*cursor), "{message}");
                assert_eq!(message["sequence"], json!(*cursor + 1), "{message}");
                *cursor += 1;
                if message["type"] != "recommendations-invalidated" {
                    return message;
                }
            }
        })
        .await
        .expect("session update deadline")
    }

    #[tokio::test]
    async fn real_socket_publishes_changed_row_then_removal_without_full_retransmission() {
        let directory = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(directory.path());
        let root = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path();
        let metadata = crate::metadata::MetadataStore::new(&root);
        for index in 0..32 {
            metadata.write_session(&crate::test_support::test_session_metadata(
                format!("session-{index}"),
                format!("History {index}"),
                directory.path().display().to_string(),
                "ended",
                "2026-10-24T10:00:00Z",
                "2026-10-24T10:00:00Z",
            ));
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = crate::build_router(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let mut url = url::Url::parse(&format!("ws://{address}/workspace/updates")).unwrap();
        url.query_pairs_mut()
            .append_pair("workspace", &directory.path().display().to_string());
        let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .unwrap();
        let snapshot = next_json(&mut socket).await;
        assert_eq!(snapshot["type"], "snapshot");
        assert_eq!(snapshot["sessions"].as_array().unwrap().len(), 32);
        let invalidation = next_json(&mut socket).await;
        assert_eq!(invalidation["base"], snapshot["sequence"]);
        let mut changed = metadata
            .try_read_revisioned_session("session-7")
            .unwrap()
            .unwrap()
            .metadata;
        changed.label = "Updated one row".into();
        metadata.write_session(&changed); // Exercise the real native watcher, no manual dirty signal.
        let mut cursor = invalidation["sequence"].as_u64().unwrap();
        let delta = next_session_update(&mut socket, &mut cursor).await;
        assert_eq!(delta["type"], "sessions-delta");
        assert_eq!(delta["changed"].as_array().unwrap().len(), 1);
        assert_eq!(delta["changed"][0]["id"], "session-7");
        assert!(delta.to_string().len() < snapshot.to_string().len() / 4);
        {
            // Exercise removal publication without intentionally racing the strict
            // directory scan; incomplete reads are covered by the corruption case.
            let _projection = state.projection_lock.lock().unwrap();
            metadata.delete_session("session-7").unwrap();
        }
        let removal = next_session_update(&mut socket, &mut cursor).await;
        assert_eq!(removal["type"], "sessions-delta", "{removal}");
        assert_eq!(removal["removed"], json!(["session-7"]));
        assert_eq!(removal["changed"], json!([]));
        assert_eq!(removal["order"].as_array().unwrap().len(), 31);
        // A corrupt remembered record must close for recovery, never remove it.
        std::fs::write(root.join("sessions/session-8.json"), b"invalid json").unwrap();
        let unavailable = next_session_update(&mut socket, &mut cursor).await;
        assert_eq!(unavailable["type"], "unavailable");
        server.abort();
    }

    #[tokio::test]
    async fn socket_rejects_wrong_workspace_and_remote_origin_and_closes_after_replacement() {
        use futures_util::StreamExt;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let directory = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(directory.path());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = crate::build_router(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let wrong = format!("ws://{address}/workspace/updates?workspace=wrong");
        assert!(tokio_tungstenite::connect_async(&wrong).await.is_err());
        let mut url = url::Url::parse(&format!("ws://{address}/workspace/updates")).unwrap();
        url.query_pairs_mut()
            .append_pair("workspace", &directory.path().display().to_string());
        let mut request = url.as_str().into_client_request().unwrap();
        request
            .headers_mut()
            .insert("origin", "https://example.com".parse().unwrap());
        let error = tokio_tungstenite::connect_async(request).await.unwrap_err();
        assert!(
            matches!(error, tokio_tungstenite::tungstenite::Error::Http(response) if response.status() == 403)
        );
        let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .unwrap();
        assert_eq!(next_json(&mut socket).await["type"], "snapshot");
        next_json(&mut socket).await;
        *state.workspace.lock().unwrap() = None;
        state.updates.sessions_changed();
        let closed = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .unwrap();
        assert!(matches!(
            closed,
            None | Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_)))
        ));
        server.abort();
    }
    #[tokio::test]
    async fn established_directory_disappearance_is_unavailable_then_recreation_resynchronizes() {
        let directory = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(directory.path());
        let root = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path();
        let metadata = crate::metadata::MetadataStore::new(&root);
        metadata.write_session(&crate::test_support::test_session_metadata(
            "retained",
            "Retained history",
            directory.path().display().to_string(),
            "ended",
            "2026-10-24T10:00:00Z",
            "2026-10-24T10:00:00Z",
        ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = crate::build_router(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let mut url = url::Url::parse(&format!("ws://{address}/workspace/updates")).unwrap();
        url.query_pairs_mut()
            .append_pair("workspace", &directory.path().display().to_string());
        let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .unwrap();
        assert_eq!(
            next_json(&mut socket).await["sessions"][0]["id"],
            "retained"
        );
        next_json(&mut socket).await;
        std::fs::rename(root.join("sessions"), root.join("sessions-replacing")).unwrap();
        // Both transports must reject this incomplete directory, not publish an empty list.
        assert!(query_sessions(&state).is_err());
        assert_eq!(next_json(&mut socket).await["type"], "unavailable");
        std::fs::rename(root.join("sessions-replacing"), root.join("sessions")).unwrap();
        let (mut recovered, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .unwrap();
        let snapshot = next_json(&mut recovered).await;
        assert_eq!(snapshot["type"], "snapshot");
        assert_eq!(snapshot["sessions"][0]["id"], "retained");
        server.abort();
    }
}
