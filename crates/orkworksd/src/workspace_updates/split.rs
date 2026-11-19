//! Negotiated display updates. The legacy stream and authoritative REST path
//! remain independent. A watch value bounds backlog to one current display.
use super::*;
use crate::session_projection::{LocationInputs, StreamProjection};
use tokio::task::JoinHandle;

pub(super) const CAPABILITY: &str = "status-enrichment-v1";
const FIELDS: &[&str] = &[
    "cwd",
    "worktreeBinding",
    "gitContextSource",
    "repoRoot",
    "branch",
    "dirty",
    "changedFiles",
    "lineChanges",
    "isWorktree",
    "conflictWarning",
    "recommendation",
    "hasOpenablePlan",
    "planDocuments",
];
const ENRICHMENT_INTERVAL: Duration = Duration::from_secs(5);
const STATUS_INTERVAL: Duration = Duration::from_secs(2);
type Job = JoinHandle<std::io::Result<StreamProjection>>;

#[derive(Clone)]
struct Owner {
    workspace: String,
    root: PathBuf,
    generation: u64,
    documents: Arc<crate::branch_documents_store::Store>,
}
impl Owner {
    fn current(&self, state: &AppState) -> bool {
        let generation = state.updates.revisions.borrow().workspace;
        generation == self.generation
            && state.workspace.lock().unwrap().as_ref().is_some_and(|ws| {
                ws.path.to_string_lossy() == self.workspace
                    && ws.metadata.root_path() == self.root
                    && Arc::ptr_eq(&self.documents, &ws.metadata.branch_documents)
            })
    }
}

pub(super) struct Shared {
    owner: Owner,
    sender: watch::Sender<Option<Arc<Display>>>,
}

#[derive(Clone, Default)]
struct Display {
    status: Vec<Value>,
    enrichment: HashMap<String, Value>,
    recommendations: u64,
    epoch: u64,
    validated_epoch: Option<u64>,
}
fn key(prepared: &StreamProjection, id: &str, epoch: u64) -> String {
    use sha2::{Digest, Sha256};
    let input = format!(
        "{epoch}:{:?}:{:?}",
        prepared.locations.get(id),
        prepared.plans.get(id)
    );
    hex::encode(Sha256::digest(input.as_bytes()))
}
fn status_rows(prepared: &StreamProjection, epoch: u64) -> Vec<Value> {
    prepared
        .infos
        .iter()
        .map(|info| {
            let mut row = serde_json::to_value(info).expect("session serializes");
            for field in FIELDS {
                row.as_object_mut().unwrap().remove(*field);
            }
            row["enrichmentRevision"] = json!(key(prepared, &info.id, epoch));
            row
        })
        .collect()
}
fn pending(row: &Value) -> Value {
    json!({"id":row["id"], "revision":row["enrichmentRevision"],
        "data":{"cwd":""}, "state":"pending", "checkedAt":null})
}
impl Display {
    // Membership comes only from the current status capture, never a late scan.
    fn status(&mut self, prepared: &StreamProjection, epoch: u64) -> bool {
        let next = status_rows(prepared, epoch);
        let mut invalidated = false;
        let mut enrichment = HashMap::new();
        for row in &next {
            let id = row["id"].as_str().unwrap();
            let value = self
                .enrichment
                .get(id)
                .filter(|value| value["revision"] == row["enrichmentRevision"])
                .cloned()
                .unwrap_or_else(|| {
                    invalidated = true;
                    pending(row)
                });
            enrichment.insert(id.to_owned(), value);
        }
        if invalidated || self.epoch != epoch {
            self.validated_epoch = None;
        }
        self.epoch = epoch;
        self.status = next;
        self.enrichment = enrichment;
        invalidated
    }
    fn inspect(&mut self, prepared: &StreamProjection, epoch: u64, checked_at: u64) {
        let mut matched = 0;
        for info in &prepared.infos {
            let Some(current) = self.enrichment.get_mut(&info.id) else {
                continue;
            };
            if current["revision"] != key(prepared, &info.id, epoch) {
                continue;
            }
            matched += 1;
            let mut data = serde_json::to_value(info).expect("session serializes");
            data.as_object_mut()
                .unwrap()
                .retain(|field, _| FIELDS.contains(&field.as_str()));
            let availability = if (info.worktree_binding.is_some() && info.repo_root.is_none())
                || info
                    .plan_documents
                    .as_ref()
                    .is_some_and(|documents| documents.notice.is_some())
            {
                "unavailable"
            } else {
                "ready"
            };
            *current = json!({"id": info.id, "revision": current["revision"],
                "data":data, "state":availability, "checkedAt":checked_at});
        }
        if epoch == self.epoch && matched == self.status.len() {
            self.validated_epoch = Some(epoch);
        }
    }
    fn rows(&self) -> Vec<Value> {
        self.status
            .iter()
            .map(|status| {
                let mut row = status.clone();
                let enrichment = &self.enrichment[status["id"].as_str().unwrap()];
                row.as_object_mut()
                    .unwrap()
                    .extend(enrichment["data"].as_object().unwrap().clone());
                row["enrichment"] =
                    json!({"state":enrichment["state"], "checkedAt":enrichment["checkedAt"]});
                row
            })
            .collect()
    }
}
fn launch(state: &Arc<AppState>, enrich: bool) -> Job {
    #[cfg(test)]
    if enrich {
        state
            .updates
            .inspections
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    let state = state.clone();
    tokio::task::spawn_blocking(move || {
        crate::http::session_handlers::query_stream_sessions(&state, enrich)
    })
}
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn subscribe_shared(
    state: Arc<AppState>,
    workspace: String,
    root: PathBuf,
) -> Option<(u64, u64, watch::Receiver<Option<Arc<Display>>>)> {
    let mut shared = state.updates.shared.lock().unwrap();
    if let Some(existing) = shared.as_ref().filter(|entry| {
        entry.owner.workspace == workspace
            && entry.owner.root == root
            && entry.owner.current(&state)
    }) {
        state.updates.sessions_changed();
        let epoch = state.updates.revisions.borrow().enrichment;
        return Some((
            existing.owner.generation,
            epoch,
            existing.sender.subscribe(),
        ));
    }
    let documents = {
        let current = state.workspace.lock().unwrap();
        let current = current.as_ref()?;
        if current.path.to_string_lossy() != workspace || current.metadata.root_path() != root {
            return None;
        }
        current.metadata.branch_documents.clone()
    };
    let owner = Owner {
        workspace,
        root,
        generation: state.updates.revisions.borrow().workspace,
        documents,
    };
    let (sender, receiver) = watch::channel(None);
    *shared = Some(Shared {
        owner: owner.clone(),
        sender: sender.clone(),
    });
    let generation = owner.generation;
    state.updates.sessions_changed();
    let epoch = state.updates.revisions.borrow().enrichment;
    tokio::spawn(publish(state.clone(), owner, sender));
    Some((generation, epoch, receiver))
}

async fn publish(state: Arc<AppState>, owner: Owner, sender: watch::Sender<Option<Arc<Display>>>) {
    let mut revisions = state.updates.subscribe();
    let global = crate::taskmaster::assessment_access::runtime_for(&state)
        .root_path()
        .to_owned();
    let mut watcher = watch_metadata(&owner.root, &global, state.updates.clone()).ok();
    let mut consumed = *revisions.borrow_and_update();
    let mut display = Display::default();
    let mut initialized = false;
    let mut status: Option<Job> = None;
    let mut status_epoch = consumed.enrichment;
    let mut inspection: Option<Job> = Some(launch(&state, true));
    let mut inspection_epoch = consumed.enrichment;
    let mut inspection_started = now_millis();
    let mut next_status = tokio::time::Instant::now();
    let mut next_inspection = tokio::time::Instant::now() + ENRICHMENT_INTERVAL;
    let mut reconcile = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut status_dirty = false;
    let mut inspection_dirty = false;
    let mut publish_dirty = false;
    loop {
        if !owner.current(&state) {
            break;
        }
        let current = *revisions.borrow_and_update();
        if current.enrichment != consumed.enrichment {
            // Known mutation/unknown scope invalidates all until a new status
            // capture establishes new revision keys. Do not publish old paths.
            display.validated_epoch = None;
            for value in display.enrichment.values_mut() {
                value["data"] = json!({"cwd":""});
                value["state"] = json!("pending");
            }
            inspection_dirty = true;
            status_dirty = true;
            publish_dirty = true;
        }
        if current.sessions != consumed.sessions {
            status_dirty = true;
        }
        if current.watcher != consumed.watcher {
            watcher = watch_metadata(&owner.root, &global, state.updates.clone()).ok();
        }
        if display.recommendations != current.recommendations {
            display.recommendations = current.recommendations;
            publish_dirty = true;
        }
        consumed = current;
        if initialized && publish_dirty {
            sender.send_replace(Some(Arc::new(display.clone())));
            publish_dirty = false;
        }
        let now = tokio::time::Instant::now();
        if now >= reconcile {
            status_dirty = true;
            inspection_dirty = true;
            watcher = watch_metadata(&owner.root, &global, state.updates.clone()).ok();
            reconcile = now + Duration::from_secs(30);
        }
        if initialized && status.is_none() && status_dirty && now >= next_status {
            status_epoch = current.enrichment;
            status = Some(launch(&state, false));
            status_dirty = false;
            next_status = now + STATUS_INTERVAL;
        }
        if inspection.is_none() && (inspection_dirty || now >= next_inspection) {
            inspection = Some(launch(&state, true));
            inspection_dirty = false;
            inspection_epoch = current.enrichment;
            inspection_started = now_millis();
            next_inspection = now + ENRICHMENT_INTERVAL;
        }
        // Only armed deadlines participate; a running task cannot cause a busy loop.
        let status_deadline = if initialized && status.is_none() && status_dirty {
            next_status
        } else {
            reconcile
        };
        let inspect_deadline = if inspection.is_none() {
            next_inspection
        } else {
            reconcile
        };
        let deadline = reconcile.min(status_deadline).min(inspect_deadline);
        tokio::select! {
            _ = sender.closed() => break,
            changed = revisions.changed() => { if changed.is_err() { break; } },
            _ = tokio::time::sleep_until(deadline) => {},
            result = async { status.as_mut().unwrap().await }, if status.is_some() => {
                status = None;
                let Ok(Ok(prepared)) = result else { break; };
                if !owner.current(&state) { break; }
                // The capture happened before this await. Dirty inputs are checked
                // again under the same admission fence before becoming a baseline.
                let _fence = state.projection_lock.lock().unwrap();
                if status_epoch != state.updates.revisions.borrow().enrichment || prepared.locations.iter().any(|(id, value)| LocationInputs::capture(&state, id) != *value) {
                    status_dirty = true; continue;
                }
                inspection_dirty |= display.status(&prepared, status_epoch);
                publish_dirty = true;
            },
            result = async { inspection.as_mut().unwrap().await }, if inspection.is_some() => {
                inspection = None;
                if !owner.current(&state) { break; }
                let _fence = state.projection_lock.lock().unwrap();
                let publication_epoch = state.updates.revisions.borrow().enrichment;
                match result {
                    Ok(Ok(prepared)) if publication_epoch == inspection_epoch
                        && prepared.locations.iter().all(|(id, value)| LocationInputs::capture(&state, id) == *value) => {
                        if !initialized {
                            display.status(&prepared, inspection_epoch);
                            initialized = true; status_dirty = true;
                        }
                        display.inspect(&prepared, inspection_epoch, inspection_started);
                        publish_dirty = true;
                    }
                    Ok(Ok(_)) => { inspection_dirty = true; status_dirty = true; },
                    _ => break,
                }
            },
        }
    }
    // spawn_blocking cannot be interrupted; dropping joins detaches computation,
    // but its captured owner can no longer publish to any subscriber.
    drop(watcher);
    let mut shared = state.updates.shared.lock().unwrap();
    if shared
        .as_ref()
        .is_some_and(|entry| entry.sender.same_channel(&sender))
    {
        *shared = None;
    }
}

pub(super) async fn run(
    mut socket: WebSocket,
    state: Arc<AppState>,
    workspace: String,
    root: PathBuf,
) {
    let Some((generation, required_epoch, mut updates)) =
        subscribe_shared(state.clone(), workspace.clone(), root)
    else {
        return;
    };
    let mut baseline: Option<Arc<Display>> = None;
    let mut sequence = 0_u64;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(10));
    heartbeat.tick().await;
    loop {
        let next = updates.borrow_and_update().clone();
        if let Some(next) = next.filter(|next| {
            baseline.is_some()
                || next
                    .validated_epoch
                    .is_some_and(|epoch| epoch >= required_epoch)
        }) {
            let mut messages = Vec::new();
            if let Some(old) = &baseline {
                if let Some(mut delta) = session_delta(&old.status, &next.status) {
                    delta["type"] = json!("session-status-delta");
                    messages.push(delta);
                }
                let changed: Vec<_> = next
                    .status
                    .iter()
                    .filter_map(|row| {
                        let id = row["id"].as_str().unwrap();
                        let value = &next.enrichment[id];
                        (old.enrichment.get(id) != Some(value)).then_some(value)
                    })
                    .collect();
                if !changed.is_empty() {
                    messages.push(json!({"type":"session-enrichment-delta", "changed":changed}));
                }
                if old.recommendations != next.recommendations {
                    messages.push(json!({"type":"recommendations-invalidated"}));
                }
            } else {
                messages.push(
                    json!({"type":"snapshot", "capability":CAPABILITY, "sessions":next.rows()}),
                );
            }
            for mut message in messages {
                if baseline.is_some() {
                    message["base"] = json!(sequence);
                    sequence += 1;
                }
                message["version"] = json!(1);
                message["instance"] = json!(state.updates.instance);
                message["workspace"] = json!(workspace);
                message["generation"] = json!(generation);
                message["sequence"] = json!(sequence);
                if !send(&mut socket, message).await {
                    return;
                }
            }
            baseline = Some(next);
        }
        tokio::select! {
            changed = updates.changed() => { if changed.is_err() { let _ = send(&mut socket, json!({"type":"unavailable"})).await; break; } },
            _ = heartbeat.tick() => { if !send(&mut socket, json!({"type":"heartbeat"})).await { break; } },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) if text == "refresh" => state.updates.sessions_changed(),
                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {},
                _ => break,
            },
        }
    }
    let _ = socket.close().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    fn fixture() -> (
        tempfile::TempDir,
        Arc<AppState>,
        crate::metadata::MetadataStore,
    ) {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let root = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path();
        let metadata = crate::metadata::MetadataStore::new(&root);
        for n in 0..32 {
            let mut record = crate::test_support::test_session_metadata(
                format!("row-{n}"),
                format!("Row {n}"),
                dir.path().display().to_string(),
                "ended",
                "2026-10-24T10:00:00Z",
                "2026-10-24T10:00:00Z",
            );
            record.cwd = dir.path().display().to_string();
            metadata.write_session(&record);
        }
        (dir, state, metadata)
    }
    async fn display_where(
        rx: &mut watch::Receiver<Option<Arc<Display>>>,
        predicate: impl Fn(&Display) -> bool,
    ) -> Arc<Display> {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if let Some(display) = rx.borrow_and_update().clone() {
                    if predicate(&display) {
                        return display;
                    }
                }
                rx.changed().await.unwrap();
            }
        })
        .await
        .expect("publisher made progress")
    }
    #[tokio::test]
    async fn upgrade_cannot_reuse_another_workspaces_publisher() {
        let (dir, state, metadata) = fixture();
        let _current = subscribe_shared(
            state.clone(),
            dir.path().display().to_string(),
            metadata.root_path(),
        )
        .unwrap();
        // Simulate an upgrade whose requested workspace was replaced after the
        // HTTP handler's initial check, before attaching to the publisher.
        assert!(subscribe_shared(
            state.clone(),
            "/previous-workspace".into(),
            metadata.root_path()
        )
        .is_none());
    }

    #[tokio::test]
    async fn reconnect_requests_fresh_inspection_instead_of_adopting_cached_snapshot() {
        let (dir, state, metadata) = fixture();
        let (_, _, mut first) = subscribe_shared(
            state.clone(),
            dir.path().display().to_string(),
            metadata.root_path(),
        )
        .unwrap();
        display_where(&mut first, |_| true).await;
        let before = state.updates.revisions.borrow().enrichment;
        let (_, required, mut second) = subscribe_shared(
            state.clone(),
            dir.path().display().to_string(),
            metadata.root_path(),
        )
        .unwrap();
        assert!(second.borrow().as_ref().unwrap().validated_epoch.unwrap() < required);
        let refreshed = display_where(&mut second, |d| {
            d.validated_epoch.is_some_and(|epoch| epoch >= required)
        })
        .await;
        assert!(refreshed
            .enrichment
            .values()
            .all(|e| e["state"] != "pending"));
        assert_ne!(
            state.updates.revisions.borrow().enrichment,
            before,
            "a new snapshot must request fresh validation"
        );
    }

    #[tokio::test]
    async fn subscribers_share_inspection_and_persisted_output_only_updates_status() {
        let (dir, state, metadata) = fixture();
        let root = metadata.root_path();
        let (_, _, mut first) = subscribe_shared(
            state.clone(),
            dir.path().display().to_string(),
            root.clone(),
        )
        .unwrap();
        let (_, _, second) =
            subscribe_shared(state.clone(), dir.path().display().to_string(), root).unwrap();
        assert!(first.same_channel(&second));
        display_where(&mut first, |_| true).await;
        let before = state.updates.inspections.load(Ordering::SeqCst);
        let mut record = metadata
            .try_read_revisioned_session("row-0")
            .unwrap()
            .unwrap()
            .metadata;
        record.last_output_at = Some("2026-11-19T10:00:00Z".into());
        metadata.write_session(&record);
        for _ in 0..100 {
            state.updates.output_changed();
        }
        let result = display_where(&mut first, |d| {
            d.status
                .iter()
                .any(|r| r["lastOutputAt"] == "2026-11-19T10:00:00Z")
        })
        .await;
        assert_eq!(result.status.len(), 32);
        assert_eq!(
            state.updates.inspections.load(Ordering::SeqCst),
            before,
            "output/persistence must not trigger repository inspection"
        );
        drop(first);
        drop(second);
        tokio::time::timeout(Duration::from_secs(2), async {
            while state.updates.shared.lock().unwrap().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn blocked_inspection_does_not_block_status_or_input_admission() {
        let (dir, state, metadata) = fixture();
        let (_, _, mut rx) = subscribe_shared(
            state.clone(),
            dir.path().display().to_string(),
            metadata.root_path(),
        )
        .unwrap();
        display_where(&mut rx, |_| true).await;
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let store = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .branch_documents
            .clone();
        *store.before_scan.lock().unwrap() = Some(Box::new(move || {
            let _ = entered_tx.send(());
            release_rx.recv_timeout(Duration::from_secs(15)).unwrap();
        }));
        state.updates.sessions_changed();
        tokio::time::timeout(Duration::from_secs(10), entered_rx)
            .await
            .unwrap()
            .unwrap();
        let mut record = metadata
            .try_read_revisioned_session("row-0")
            .unwrap()
            .unwrap()
            .metadata;
        record.label = "Updated while blocked".into();
        metadata.write_session(&record);
        state.updates.output_changed();
        let result = display_where(&mut rx, |d| {
            d.status
                .iter()
                .any(|r| r["label"] == "Updated while blocked")
        })
        .await;
        let input_available = state.projection_lock.try_lock().is_ok();
        release_tx.send(()).unwrap();
        assert!(input_available);
        assert_eq!(result.status.len(), 32);
    }
    #[tokio::test]
    #[ignore = "manual 35-second socket workload; use an external process CPU timer"]
    async fn benchmark_stream_workload() {
        use futures_util::StreamExt;
        let mode = std::env::var("ORKWORKS_REFRESH_BENCH_STREAM")
            .expect("select legacy-idle, split-idle, legacy-busy or split-busy");
        assert!(["legacy-idle", "split-idle", "legacy-busy", "split-busy"].contains(&mode.as_str()));
        let split = mode.starts_with("split-");
        let busy = mode.ends_with("-busy");
        let (dir, state, metadata) = fixture();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = crate::build_router(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let mut url = url::Url::parse(&format!("ws://{address}/workspace/updates")).unwrap();
        url.query_pairs_mut()
            .append_pair("workspace", &dir.path().display().to_string());
        if split {
            url.query_pairs_mut().append_pair("capability", CAPABILITY);
        }
        let (mut first, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .unwrap();
        let (mut second, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .unwrap();
        for socket in [&mut first, &mut second] {
            let message = tokio::time::timeout(Duration::from_secs(20), socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            let initial: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
            assert_eq!(initial["type"], "snapshot");
        }
        let mut record = metadata
            .try_read_revisioned_session("row-0")
            .unwrap()
            .unwrap()
            .metadata;
        let mut output = tokio::time::interval(Duration::from_millis(100));
        output.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(35);
        let mut messages = 0;
        let mut writes = 0;
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                _ = output.tick(), if busy => {
                    record.last_output_at = Some(crate::workspace_runtime::iso_now());
                    metadata.write_session(&record);
                    state.updates.output_changed();
                    writes += 1;
                },
                message = first.next() => { assert!(message.unwrap().is_ok()); messages += 1; },
                message = second.next() => { assert!(message.unwrap().is_ok()); messages += 1; },
            }
        }
        eprintln!("STREAM_BENCH mode={mode} rows=32 subscribers=2 duration_seconds=35 writes={writes} messages={messages} shared_inspections={}", state.updates.inspections.load(Ordering::SeqCst));
        drop(first);
        drop(second);
        server.abort();
    }

    #[test]
    #[ignore = "manual controlled projection timing; no wall-time threshold"]
    fn benchmark_status_and_enrichment_projection() {
        use std::time::Instant;
        let (_dir, state, metadata) = fixture();
        let distinct = std::env::var("ORKWORKS_REFRESH_BENCH_ROOTS").as_deref() == Ok("distinct");
        let roots: Vec<_> = if distinct {
            (0..4).map(|_| tempfile::tempdir().unwrap()).collect()
        } else {
            vec![]
        };
        if distinct {
            for root in &roots {
                git2::Repository::init(root.path()).unwrap();
            }
            for n in 0..32 {
                let mut record = metadata
                    .try_read_revisioned_session(&format!("row-{n}"))
                    .unwrap()
                    .unwrap()
                    .metadata;
                record.cwd = roots[n % roots.len()].path().display().to_string();
                metadata.write_session(&record);
            }
        }
        let modes = match std::env::var("ORKWORKS_REFRESH_BENCH").as_deref() {
            Ok("status") => vec![false],
            Ok("full") => vec![true],
            Err(_) => vec![true, false],
            _ => panic!("benchmark mode must be status or full"),
        };
        crate::http::session_handlers::query_stream_sessions(&state, true).unwrap();
        for round in 0..5 {
            for &enrich in &modes {
                let start = Instant::now();
                for _ in 0..20 {
                    assert_eq!(
                        crate::http::session_handlers::query_stream_sessions(&state, enrich)
                            .unwrap()
                            .infos
                            .len(),
                        32
                    );
                }
                eprintln!("REFRESH_BENCH round={round} enrich={enrich} distinct={distinct} rows=32 calls=20 elapsed_us={}", start.elapsed().as_micros());
            }
        }
    }

    #[test]
    fn late_inspection_cannot_resurrect_deleted_rows_or_overwrite_status() {
        let (_dir, state, _) = fixture();
        let prepared = crate::http::session_handlers::query_stream_sessions(&state, true).unwrap();
        let mut display = Display::default();
        display.status(&prepared, 0);
        display.inspect(&prepared, 0, 100);
        let mut status =
            crate::http::session_handlers::query_stream_sessions(&state, false).unwrap();
        status.infos.remove(0);
        status.infos[0].label = "Newer".into();
        display.status(&status, 1);
        display.inspect(&prepared, 0, 200);
        assert_eq!(display.status.len(), 31);
        assert_eq!(display.status[0]["label"], "Newer");
        assert!(display.enrichment.values().all(|e| e["state"] == "pending"));
    }
}
