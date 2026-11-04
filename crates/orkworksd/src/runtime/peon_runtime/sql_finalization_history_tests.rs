use super::sql_reservation_tests::fixture;
use super::*;
use crate::session_application::ending_completion::{
    testing::{hold, Stage},
    WriteResult,
};
use crate::session_application::sql_writes::{
    lifecycle::{LifecycleWrite, MetadataResult},
    Outcome,
};
use crate::session_store::StoreError;
const ID: &str = "sql-runtime";
fn identity(state: &Arc<AppState>) -> crate::runtime::session_runtime::RuntimeIdentity {
    state.sessions.lock().unwrap()[ID].runtime.identity()
}
fn stop() -> LifecycleWrite {
    LifecycleWrite::Transition {
        status: "killed".into(),
    }
}
fn snapshot() -> crate::metadata::ObservedStatusSnapshotMetadata {
    crate::metadata::ObservedStatusSnapshotMetadata {
        value: Some("blocked".into()),
        source: "peon".into(),
        confidence: Some(0.8),
        observed_at: None,
        attention_origin: None,
    }
}
fn files(state: &Arc<AppState>) -> crate::metadata::MetadataStore {
    crate::metadata::MetadataStore::new(
        &state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path(),
    )
}
async fn ending(
    writes: &Arc<crate::session_application::sql_writes::SessionWrites>,
    state: &Arc<AppState>,
) {
    writes
        .lifecycle(ID.into(), identity(state), stop())
        .await
        .unwrap();
}

#[tokio::test]
async fn sql_history_finalization_requires_confirmed_grid_before_commit() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let runtime = identity(&state);
    writes
        .lifecycle(ID.into(), runtime.clone(), stop())
        .await
        .unwrap();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let grid = root.join("events").join(format!("{ID}.terminal-size"));
    std::fs::create_dir_all(&grid).unwrap();
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let result = writes
        .finalize_with_history(
            ID.into(),
            runtime,
            crate::metadata::ObservedStatusSnapshotMetadata {
                value: None,
                source: "process".into(),
                confidence: None,
                observed_at: None,
                attention_origin: None,
            },
            "ended".into(),
        )
        .await;
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap(),
        before,
        "failed final grid must leave SQL ending metadata unchanged: {result:?}"
    );
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ending"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_history_repair_preserves_order_and_exact_committed_event() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    ending(&writes, &state).await;
    let files = files(&state);
    let legacy = files.sessions_dir().join(format!("{ID}.json"));
    let before = std::fs::read(&legacy).unwrap();
    let grid = files
        .root_path()
        .join("events")
        .join(format!("{ID}.terminal-size"));
    std::fs::create_dir_all(&grid).unwrap();
    let failed = writes
        .finalize_with_history(ID.into(), identity(&state), snapshot(), "ended".into())
        .await
        .unwrap();
    assert!(matches!(failed.replay, WriteResult::Failed(_)));
    assert!(matches!(failed.metadata, MetadataResult::NotAttempted));
    assert!(matches!(failed.event, WriteResult::NotAttempted));
    std::fs::remove_dir(&grid).unwrap();
    let mut pause = hold(&state, Stage::BeforeEvent);
    let w = writes.clone();
    let runtime = identity(&state);
    let writer = tokio::spawn(async move {
        w.finalize_with_history(ID.into(), runtime, snapshot(), "ended".into())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    let ended = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(ended["lifecyclePhase"], "ended");
    assert_eq!(files.read_terminal_size(ID), Some((80, 24)));
    assert!(files.read_events(ID).is_empty());
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ending"
    );
    drop(pause);
    let done = writer.await.unwrap().unwrap();
    assert!(matches!(done.replay, WriteResult::Written));
    assert!(matches!(
        done.metadata,
        MetadataResult::Completed(Outcome::Committed)
    ));
    assert!(matches!(done.event, WriteResult::Written));
    let events = files.read_events(ID);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, "killed");
    assert_eq!(events[0].timestamp, ended["lastActivity"].as_str().unwrap());
    assert_eq!(events[0].observed_status.as_deref(), Some("blocked"));
    assert_eq!(events[0].confidence, Some(0.8));
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ended"
    );
    assert_eq!(std::fs::read(&legacy).unwrap(), before);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_history_event_failure_preserves_commit_without_blind_retry() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    ending(&writes, &state).await;
    let files = files(&state);
    let event = files
        .root_path()
        .join("events")
        .join(format!("{ID}.ndjson"));
    std::fs::create_dir_all(&event).unwrap();
    let done = writes
        .finalize_with_history(ID.into(), identity(&state), snapshot(), "ended".into())
        .await
        .unwrap();
    assert!(matches!(done.replay, WriteResult::Written));
    assert!(matches!(
        done.metadata,
        MetadataResult::Completed(Outcome::Committed)
    ));
    assert!(matches!(done.event, WriteResult::Failed(_)));
    let ended = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(ended["lifecyclePhase"], "ended");
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ended"
    );
    std::fs::remove_dir(&event).unwrap();
    assert!(matches!(
        writes
            .finalize_with_history(ID.into(), identity(&state), snapshot(), "ended".into())
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(!event.exists());
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), ended);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_history_sql_failure_reports_confirmed_grid_only() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    ending(&writes, &state).await;
    let files = files(&state);
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let mut pause = hold(&state, Stage::BeforeMetadata);
    let runtime = identity(&state);
    let writer = tokio::spawn(async move {
        writes
            .finalize_with_history(ID.into(), runtime, snapshot(), "ended".into())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    assert_eq!(files.read_terminal_size(ID), Some((80, 24)));
    store.close().await.unwrap();
    drop(pause);
    let done = writer.await.unwrap().unwrap();
    assert!(matches!(
        done.metadata,
        MetadataResult::Failed(StoreError::Unavailable)
    ));
    assert!(matches!(done.replay, WriteResult::Written));
    assert!(matches!(done.event, WriteResult::NotAttempted));
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ending"
    );
    assert!(files.read_events(ID).is_empty());
    let reopened = crate::session_store::SessionStore::open(
        files.root_path(),
        before["workspace"].as_str().unwrap().into(),
    )
    .await
    .unwrap();
    assert_eq!(reopened.get(ID.into()).await.unwrap().unwrap(), before);
    reopened.close().await.unwrap();
}

#[tokio::test]
async fn sql_history_rejects_replacement_before_file_effects() {
    let _guard = diagnostic_test_guard();
    for change in [
        "runtime",
        "control",
        "creation",
        "workspace",
        "reader",
        "lease",
        "phase",
    ] {
        let (dir, state, store, writes, _, _, _, _commands) = fixture().await;
        ending(&writes, &state).await;
        let files = files(&state);
        let before = store.get(ID.into()).await.unwrap().unwrap();
        let mut pause = hold(&state, Stage::BeforeHistory);
        let runtime = identity(&state);
        let writer = tokio::spawn(async move {
            writes
                .finalize_with_history(ID.into(), runtime, snapshot(), "ended".into())
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
            .await
            .unwrap();
        match change {
            "workspace" => crate::test_support::swap_workspace(&state, dir.path()),
            "reader" => {
                let mut ws = state.workspace.lock().unwrap();
                let ws = ws.as_mut().unwrap();
                ws.metadata = crate::metadata::MetadataStore::new(&ws.metadata.root_path());
            }
            "lease" => state.workspace.lock().unwrap().as_mut().unwrap().lease = None,
            _ => {
                let mut sessions = state.sessions.lock().unwrap();
                let h = sessions.get_mut(ID).unwrap();
                match change {
                    "runtime" => {
                        h.runtime =
                            crate::runtime::session_runtime::SessionRuntime::detached(24, 80)
                    }
                    "control" => h.runtime.control_tx = tokio::sync::mpsc::channel(2).0,
                    "creation" => h.info.created_at = "replacement".into(),
                    "phase" => h.info.lifecycle_phase = "active".into(),
                    _ => unreachable!(),
                }
            }
        }
        drop(pause);
        assert!(
            matches!(writer.await.unwrap(), Err(StoreError::Conflict)),
            "{change}"
        );
        assert!(files.read_terminal_size(ID).is_none(), "{change}");
        assert!(files.read_events(ID).is_empty(), "{change}");
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap(),
            before,
            "{change}"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_history_replacement_after_commit_retains_old_event_and_detaches() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    ending(&writes, &state).await;
    let files = files(&state);
    let mut pause = hold(&state, Stage::BeforeEvent);
    let runtime = identity(&state);
    let writer = tokio::spawn(async move {
        writes
            .finalize_with_history(ID.into(), runtime, snapshot(), "ended".into())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
        crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
    drop(pause);
    let done = writer.await.unwrap().unwrap();
    assert!(matches!(
        done.metadata,
        MetadataResult::Completed(Outcome::CommittedDetached)
    ));
    assert!(matches!(done.event, WriteResult::Written));
    assert_eq!(files.read_events(ID).len(), 1);
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ending"
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["lifecyclePhase"],
        "ended"
    );
    store.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn sql_history_cancelled_after_commit_retains_lane_lease_and_publication() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    ending(&writes, &state).await;
    let files = files(&state);
    let lease = Arc::downgrade(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .lease
            .as_ref()
            .unwrap(),
    );
    let prepared = store
        .prepare_internal(
            ID.into(),
            "2026-01-01T00:00:00Z".into(),
            crate::metadata::internal_mutations::InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = prepared.lane().unwrap();
    drop(prepared);
    let mut pause = hold(&state, Stage::BeforeEvent);
    let runtime = identity(&state);
    let writer = tokio::spawn(async move {
        writes
            .finalize_with_history(ID.into(), runtime, snapshot(), "ended".into())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(lane.clone().try_lock_owned().is_err());
    assert!(lease.strong_count() >= 2);
    writer.abort();
    assert!(writer.await.unwrap_err().is_cancelled());
    assert!(lane.clone().try_lock_owned().is_err());
    drop(pause);
    let _released = tokio::time::timeout(std::time::Duration::from_secs(5), lane.lock_owned())
        .await
        .unwrap();
    assert_eq!(files.read_events(ID).len(), 1);
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ended"
    );
    assert_eq!(lease.strong_count(), 1);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_history_cancel_before_sql_claim_keeps_grid_only() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    ending(&writes, &state).await;
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let files = files(&state);
    let mut pause = hold(&state, Stage::BeforeMetadata);
    let runtime = identity(&state);
    let writer = tokio::spawn(async move {
        writes
            .finalize_with_history(ID.into(), runtime, snapshot(), "ended".into())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    writer.abort();
    assert!(writer.await.unwrap_err().is_cancelled());
    drop(pause);
    // Drain behind the retained application lane, not a timing guess.
    let prepared = store
        .prepare_internal(
            ID.into(),
            "2026-01-01T00:00:00Z".into(),
            crate::metadata::internal_mutations::InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = prepared.lane().unwrap();
    drop(prepared);
    let _released = tokio::time::timeout(std::time::Duration::from_secs(5), lane.lock_owned())
        .await
        .unwrap();
    assert_eq!(files.read_terminal_size(ID), Some((80, 24)));
    assert!(files.read_events(ID).is_empty());
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ending"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_history_validates_sql_identity_without_json_fallback() {
    let _guard = diagnostic_test_guard();
    for change in ["missing", "creation", "harness", "phase"] {
        let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
        ending(&writes, &state).await;
        let files = files(&state);
        // Qualify exact SQL authority; leave the valid legacy JSON untouched.
        let db = files.root_path().join("session-store/v1.sqlite");
        let connection = rusqlite::Connection::open(db).unwrap();
        let record = store.get(ID.into()).await.unwrap().unwrap();
        if change == "missing" {
            connection
                .execute("DELETE FROM sessions WHERE id=?1", [ID])
                .unwrap();
        } else {
            // Use the actual schema's canonical column; this deliberately bypasses
            // application lanes to simulate damaged/replaced test storage.
            let mut record = record;
            match change {
                "creation" => record["createdAt"] = serde_json::json!("replacement"),
                "harness" => record["harnessId"] = serde_json::json!("replacement"),
                "phase" => record["lifecyclePhase"] = serde_json::json!("active"),
                _ => unreachable!(),
            }
            if change == "harness" {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .info
                    .harness = Some("codex".into());
            }
            connection
                .execute(
                    "UPDATE sessions SET metadata=?1 WHERE id=?2",
                    rusqlite::params![record.to_string(), ID],
                )
                .unwrap();
        }
        drop(connection);
        let result = writes
            .finalize_with_history(ID.into(), identity(&state), snapshot(), "ended".into())
            .await;
        assert!(
            matches!(
                result,
                Err(StoreError::Missing | StoreError::Conflict | StoreError::Corrupt)
            ),
            "{change}: {result:?}"
        );
        assert!(files.read_terminal_size(ID).is_none());
        assert!(files.read_events(ID).is_empty());
        assert_eq!(
            state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
            "ending"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn sql_history_file_stall_leaves_tokio_sql_guards_and_real_pty_input_available() {
    use crate::runtime::session_runtime::RuntimeCommand;
    use std::{
        io::{Read, Write},
        time::Duration,
    };
    let _guard = diagnostic_test_guard();
    let (dir, state, store, writes, _, _, _, _) = fixture().await;
    ending(&writes, &state).await;
    let input_id = "sql-input";
    let (runtime, mut commands) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let mut info = crate::test_support::test_session_info(
        input_id,
        "Input",
        dir.path().display().to_string(),
        "running",
        "2026-01-01T00:00:00Z",
    );
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    state.sessions.lock().unwrap().insert(
        input_id.into(),
        crate::SessionHandle {
            info,
            runtime,
            kill_tx: tokio::sync::watch::channel(false).0,
            output_buffer: peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
            active_work_hook: false,
        },
    );
    let meta = crate::test_support::test_session_metadata(
        input_id,
        "Input",
        dir.path().display().to_string(),
        "running",
        "2026-01-01T00:00:00Z",
        "2026-01-02T00:00:00Z",
    );
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .try_write_session(&meta)
        .unwrap();
    let mut pause = hold(&state, Stage::BeforeMetadata);
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (progress_tx, progress_rx) = tokio::sync::oneshot::channel();
    let watchdog = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let progressed = rt.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), pause.entered())
                .await
                .unwrap();
            let _ = entered_tx.send(());
            tokio::time::timeout(Duration::from_secs(2), progress_rx)
                .await
                .is_ok_and(|r| r.is_ok())
        });
        drop(pause);
        progressed
    });
    let runtime = identity(&state);
    let writer = tokio::spawn(async move {
        writes
            .finalize_with_history(ID.into(), runtime, snapshot(), "ended".into())
            .await
    });
    entered_rx.await.unwrap();
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), store.get(ID.into()))
            .await
            .unwrap()
            .unwrap()
            .unwrap()["lifecyclePhase"],
        "ending"
    );
    let pair = portable_pty::native_pty_system()
        .openpty(portable_pty::PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut child = pair
        .slave
        .spawn_command(portable_pty::CommandBuilder::new("/bin/cat"))
        .unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut terminal = pair.master.take_writer().unwrap();
    let (seen, observed) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = [0; 4096];
        let mut captured = Vec::new();
        let result = loop {
            match reader.read(&mut bytes) {
                Ok(0) => break Ok(captured),
                Ok(n) => {
                    captured.extend_from_slice(&bytes[..n]);
                    if captured.windows(16).any(|s| s == b"responsive-input")
                        || captured.len() > 16_384
                    {
                        break Ok(captured);
                    }
                }
                Err(error) => break Err(error),
            }
        };
        let _ = seen.send(result);
    });
    let delivery = tokio::task::spawn_blocking(move || match commands.blocking_recv().unwrap() {
        RuntimeCommand::Input {
            data,
            accepted: Some(accepted),
        } => {
            let result = terminal
                .write_all(data.as_bytes())
                .and_then(|()| terminal.flush())
                .map_err(|_| ());
            let _ = accepted.send(result);
        }
        _ => panic!("expected ordinary input command"),
    });
    let delivered = tokio::time::timeout(
        Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            input_id,
            "responsive-input\r".into(),
        ),
    )
    .await;
    let echoed = observed.recv_timeout(Duration::from_secs(1));
    let _ = child.kill();
    let _ = child.wait();
    drop(pair.master);
    reader.join().unwrap();
    delivery.await.unwrap();
    let _ = progress_tx.send(());
    let progressed = watchdog.join().unwrap();
    let done = writer.await.unwrap().unwrap();
    assert!(
        progressed,
        "history persistence blocked the sole Tokio worker until the OS watchdog released it"
    );
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(echoed
        .unwrap()
        .unwrap()
        .windows(16)
        .any(|s| s == b"responsive-input"));
    assert!(matches!(
        done.metadata,
        MetadataResult::Completed(Outcome::Committed)
    ));
    store.close().await.unwrap();
}

#[test]
fn sql_history_one_blocking_thread_can_finish_without_recursive_pool_wait() {
    let _guard = diagnostic_test_guard();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
        ending(&writes, &state).await;
        let done = tokio::time::timeout(
            std::time::Duration::from_secs(4),
            writes.finalize_with_history(ID.into(), identity(&state), snapshot(), "ended".into()),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            matches!(done.metadata, MetadataResult::Completed(Outcome::Committed)),
            "recursive blocking-pool wait: {done:?}"
        );
        store.close().await.unwrap();
    });
}
