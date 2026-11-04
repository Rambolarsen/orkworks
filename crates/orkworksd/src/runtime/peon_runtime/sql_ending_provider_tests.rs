use super::sql_reservation_tests::fixture;
use super::*;
use crate::metadata::internal_mutations::InternalMutation;
use crate::metadata::lifecycle_mutations::LifecycleMutation;
use crate::session_application::sql_writes::lifecycle::{
    CapturedEnding, LifecycleWrite, ProviderWrite,
};
const ID: &str = "sql-runtime";
const CREATED: &str = "2026-01-01T00:00:00Z";
fn provider() -> crate::providers::ProviderObservation {
    crate::providers::ProviderObservation {
        provider_id: "final-provider".into(),
        provider_label: "Final provider".into(),
        provider_model: Some("final-model".into()),
        provider_state: "healthy".into(),
    }
}
fn snapshot() -> crate::metadata::ObservedStatusSnapshotMetadata {
    crate::metadata::ObservedStatusSnapshotMetadata {
        value: None,
        source: "process".into(),
        confidence: None,
        observed_at: None,
        attention_origin: None,
    }
}
#[tokio::test]
async fn ending_provider_does_not_mutate_durable_ended_record_with_stale_live_phase() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    writes
        .lifecycle(
            ID.into(),
            runtime.clone(),
            LifecycleWrite::Transition {
                status: "killed".into(),
            },
        )
        .await
        .unwrap();
    let captured = writes
        .capture_ending_history(&state, ID.into(), runtime)
        .await
        .unwrap();
    // Simulate durable finalization whose live publication could not attach.
    store
        .mutate_internal(
            ID.into(),
            CREATED.into(),
            None,
            InternalMutation::Lifecycle(LifecycleMutation::Finalize {
                snapshot: snapshot(),
                fallback_status: "ended".into(),
                timestamp: "2026-11-04T00:00:00Z".into(),
            }),
        )
        .await
        .unwrap();
    let before = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(
        captured.persist_provider_context(provider()).await.unwrap(),
        ProviderWrite::Unchanged
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap(),
        before,
        "a late final provider observation changed an ended record"
    );
    store.close().await.unwrap();
}

async fn capture(
    writes: &Arc<crate::session_application::sql_writes::SessionWrites>,
    state: &Arc<AppState>,
) -> CapturedEnding {
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    writes
        .lifecycle(
            ID.into(),
            runtime.clone(),
            LifecycleWrite::Transition {
                status: "killed".into(),
            },
        )
        .await
        .unwrap();
    writes
        .capture_ending_history(state, ID.into(), runtime)
        .await
        .unwrap()
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
#[tokio::test]
async fn ending_provider_preserves_clocks_and_can_precede_history_without_recapture() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let captured = capture(&writes, &state).await;
    let files = files(&state);
    let legacy = files.sessions_dir().join(format!("{ID}.json"));
    let legacy_before = std::fs::read(&legacy).unwrap();
    let before = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(
        captured.persist_provider_context(provider()).await.unwrap(),
        ProviderWrite::Committed
    );
    let after = store.get(ID.into()).await.unwrap().unwrap();
    let parsed: crate::metadata::SessionMetadata = serde_json::from_value(after.clone()).unwrap();
    assert_eq!(parsed.provider_id.as_deref(), Some("final-provider"));
    assert_eq!(parsed.provider_label.as_deref(), Some("Final provider"));
    assert_eq!(parsed.provider_model.as_deref(), Some("final-model"));
    assert_eq!(parsed.provider_state.as_deref(), Some("healthy"));
    for key in [
        "attentionSource",
        "attentionUpdatedAt",
        "metadataSource",
        "lastActivity",
        "status",
        "lifecyclePhase",
        "label",
        "summary",
    ] {
        assert_eq!(after[key], before[key], "{key}");
    }
    assert_eq!(
        after["metadataRevision"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        before["metadataRevision"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            + 1
    );
    // Repeated accepted provider writes retain the legacy one-revision-per-write behavior.
    assert_eq!(
        captured.persist_provider_context(provider()).await.unwrap(),
        ProviderWrite::Committed
    );
    let repeated = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(
        repeated["metadataRevision"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        after["metadataRevision"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            + 1
    );
    assert!(files.read_events(ID).is_empty());
    assert!(files.read_terminal_size(ID).is_none());
    let finished = captured.complete(snapshot(), "ended".into()).await.unwrap();
    assert!(matches!(
        finished.metadata,
        crate::session_application::sql_writes::lifecycle::MetadataResult::Completed(
            crate::session_application::sql_writes::Outcome::Committed
        )
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["lifecyclePhase"],
        "ended"
    );
    assert_eq!(std::fs::read(legacy).unwrap(), legacy_before);
    store.close().await.unwrap();
}
#[tokio::test]
async fn ending_provider_original_capture_rejects_changed_owners_before_claim() {
    let _guard = diagnostic_test_guard();
    for change in [
        "workspace",
        "reader",
        "lease",
        "runtime",
        "control",
        "creation",
        "source",
        "attention",
        "phase",
    ] {
        let (dir, state, store, writes, _, _, _, _commands) = fixture().await;
        let captured = capture(&writes, &state).await;
        let before = store.get(ID.into()).await.unwrap().unwrap();
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
                    "source" => h.info.metadata_source = Some("user".into()),
                    "attention" => {
                        h.runtime.attention_owner.accepted_write();
                    }
                    "phase" => h.info.lifecycle_phase = "active".into(),
                    _ => unreachable!(),
                }
            }
        }
        assert!(
            matches!(
                captured.persist_provider_context(provider()).await,
                Err(crate::session_store::StoreError::Conflict)
            ),
            "{change}"
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap(),
            before,
            "{change}"
        );
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn ending_provider_replacement_after_execution_is_committed_detached() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let captured = capture(&writes, &state).await;
    let mut pause = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move { captured.persist_provider_context(provider()).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut(ID)
        .unwrap()
        .runtime
        .control_tx = tokio::sync::mpsc::channel(2).0;
    drop(pause);
    assert_eq!(
        writer.await.unwrap().unwrap(),
        ProviderWrite::CommittedDetached
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["modelProviderId"],
        "final-provider"
    );
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ending"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn ending_provider_cancelled_queue_cannot_write_later() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let captured = capture(&writes, &state).await;
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let pause = crate::session_store::test_worker::pause(&store);
    let writer = tokio::spawn(async move { captured.persist_provider_context(provider()).await });
    crate::session_store::test_worker::wait_for_internal(&store, ID).await;
    writer.abort();
    assert!(writer.await.unwrap_err().is_cancelled());
    drop(pause);
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
    store.close().await.unwrap();
}
#[tokio::test]
async fn ending_provider_cancelled_execution_retains_lane_and_original_lease() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let captured = capture(&writes, &state).await;
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
            CREATED.into(),
            InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = prepared.lane().unwrap();
    drop(prepared);
    let mut pause = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move { captured.persist_provider_context(provider()).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    writer.abort();
    assert!(writer.await.unwrap_err().is_cancelled());
    state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
    assert!(lane.clone().try_lock_owned().is_err());
    assert!(lease.strong_count() >= 1);
    drop(pause);
    let _released = tokio::time::timeout(std::time::Duration::from_secs(5), lane.lock_owned())
        .await
        .unwrap();
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["modelProviderId"],
        "final-provider"
    );
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while lease.strong_count() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    store.close().await.unwrap();
}
#[tokio::test]
async fn ending_provider_lane_deadline_and_storage_errors_remain_typed() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let captured = capture(&writes, &state).await;
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let prepared = store
        .prepare_internal(
            ID.into(),
            CREATED.into(),
            InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let held = prepared.lane().unwrap().lock_owned().await;
    drop(prepared);
    assert!(matches!(
        captured.persist_provider_context(provider()).await,
        Err(crate::session_store::StoreError::Expired)
    ));
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    drop(held);
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
    store.close().await.unwrap();
    assert!(matches!(
        captured.persist_provider_context(provider()).await,
        Err(crate::session_store::StoreError::Unavailable)
    ));
}
#[tokio::test]
async fn ending_provider_creation_conflict_and_oversized_intent_do_not_write() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let captured = capture(&writes, &state).await;
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let mut too_large = provider();
    too_large.provider_label = "x".repeat(1024 * 1024);
    assert!(matches!(
        captured.persist_provider_context(too_large).await,
        Err(crate::session_store::StoreError::TooLarge)
    ));
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
    // A coherent but replaced durable creation must fail the worker's own check.
    let root = files(&state).root_path();
    let conn = rusqlite::Connection::open(root.join("session-store/v1.sqlite")).unwrap();
    let mut replaced = before;
    replaced["createdAt"] = serde_json::json!("replacement");
    conn.execute(
        "UPDATE sessions SET created_at=?1,metadata=?2 WHERE id=?3",
        rusqlite::params!["replacement", replaced.to_string(), ID],
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        captured.persist_provider_context(provider()).await,
        Err(crate::session_store::StoreError::Conflict)
    ));
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), replaced);
    store.close().await.unwrap();
}

#[tokio::test]
async fn ending_provider_queue_rejection_and_missing_sql_never_fall_back_to_json() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let captured = capture(&writes, &state).await;
    let files = files(&state);
    let legacy = files.sessions_dir().join(format!("{ID}.json"));
    let legacy_before = std::fs::read(&legacy).unwrap();
    let mut held = Vec::new();
    loop {
        match store.prepare_internal(
            ID.into(),
            CREATED.into(),
            InternalMutation::UserTitle { title: None },
        ) {
            Ok(prepared) => held.push(prepared),
            Err(crate::session_store::StoreError::QueueFull) => break,
            Err(error) => panic!("unexpected admission error: {error:?}"),
        }
        assert!(held.len() <= 32);
    }
    assert!(matches!(
        captured.persist_provider_context(provider()).await,
        Err(crate::session_store::StoreError::QueueFull)
    ));
    drop(held);
    let conn =
        rusqlite::Connection::open(files.root_path().join("session-store/v1.sqlite")).unwrap();
    conn.execute("DELETE FROM sessions WHERE id=?1", [ID])
        .unwrap();
    drop(conn);
    assert!(matches!(
        captured.persist_provider_context(provider()).await,
        Err(crate::session_store::StoreError::Missing)
    ));
    assert_eq!(std::fs::read(legacy).unwrap(), legacy_before);
    assert!(files.read_events(ID).is_empty());
    store.close().await.unwrap();
}
#[tokio::test(flavor = "current_thread")]
async fn ending_provider_stall_leaves_executor_guards_and_other_runtime_input_available() {
    use crate::runtime::session_runtime::RuntimeCommand;
    use std::time::Duration;
    let _guard = diagnostic_test_guard();
    let (dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let captured = capture(&writes, &state).await;
    let input_id = "provider-input";
    let (runtime, mut input_commands) =
        crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let mut info = crate::test_support::test_session_info(
        input_id,
        "Input",
        dir.path().display().to_string(),
        "running",
        CREATED,
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
        CREATED,
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
    let mut pause = crate::session_store::test_worker::hold_next_internal(&store, ID);
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
    let writer = tokio::spawn(async move { captured.persist_provider_context(provider()).await });
    entered_rx.await.unwrap();
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    let delivery = tokio::spawn(async move {
        match input_commands.recv().await.unwrap() {
            RuntimeCommand::Input {
                data,
                accepted: Some(accepted),
            } => {
                assert_eq!(data, "independent-input\r");
                let _ = accepted.send(Ok(()));
            }
            _ => panic!("expected ordinary input delivery"),
        }
    });
    let delivered = tokio::time::timeout(
        Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            input_id,
            "independent-input\r".into(),
        ),
    )
    .await;
    let delivery_result = tokio::time::timeout(Duration::from_secs(1), delivery).await;
    let _ = progress_tx.send(());
    assert!(
        watchdog.join().unwrap(),
        "provider persistence blocked the sole Tokio worker until OS watchdog release"
    );
    delivery_result.unwrap().unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    assert_eq!(writer.await.unwrap().unwrap(), ProviderWrite::Committed);
    store.close().await.unwrap();
}
