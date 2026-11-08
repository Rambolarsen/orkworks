use super::sql_reservation_tests::fixture;
use super::*;
use crate::runtime::terminal_runtime::ending_persistence::{EndingCompletion, EndingPersistence};
use crate::session_application::ending_completion::WriteResult;
use crate::session_application::sql_writes::lifecycle::{LifecycleWrite, MetadataResult};
use crate::session_application::sql_writes::Outcome;
use crate::session_store::StoreError;
const ID: &str = "sql-runtime";
fn snapshot() -> crate::metadata::ObservedStatusSnapshotMetadata {
    crate::metadata::ObservedStatusSnapshotMetadata {
        value: Some("blocked".into()),
        source: "peon".into(),
        confidence: Some(0.8),
        observed_at: None,
        attention_origin: None,
    }
}
#[tokio::test]
async fn runtime_ending_sql_capture_cannot_adopt_replacement_control_after_scan_wait() {
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
    let before = store.get(ID.into()).await.unwrap().unwrap();
    // The runtime captures its original identity before awaiting the final scan.
    let persistence =
        EndingPersistence::sql(state.clone(), writes, ID.into(), runtime.run_generation)
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
    let result = persistence.complete(snapshot(), "ended".into()).await;
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap(),
        before,
        "a completed old final scan adopted replacement control: {result:?}"
    );
    assert!(matches!(
        result,
        EndingCompletion::Sql(Err(StoreError::Conflict))
    ));
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    assert!(!root
        .join("events")
        .join(format!("{ID}.terminal-size"))
        .exists());
    store.close().await.unwrap();
}

async fn ending(
    writes: &Arc<crate::session_application::sql_writes::SessionWrites>,
    state: &Arc<AppState>,
) -> u64 {
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
    runtime.run_generation
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
async fn runtime_ending_sql_completion_preserves_typed_stages_without_json_reads_or_writes() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let generation = ending(&writes, &state).await;
    let files = files(&state);
    let legacy = files.sessions_dir().join(format!("{ID}.json"));
    std::fs::write(&legacy, b"invalid legacy JSON must stay untouched").unwrap();
    let persistence = EndingPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
    let EndingCompletion::Sql(result) = persistence.complete(snapshot(), "ended".into()).await
    else {
        panic!("SQL completion changed backend")
    };
    let result = result.unwrap();
    assert!(matches!(result.replay, WriteResult::Written));
    assert!(matches!(
        result.metadata,
        MetadataResult::Completed(Outcome::Committed)
    ));
    assert!(matches!(result.event, WriteResult::Written));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["status"],
        "killed"
    );
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ended"
    );
    assert_eq!(files.read_events(ID).len(), 1);
    assert_eq!(
        std::fs::read(legacy).unwrap(),
        b"invalid legacy JSON must stay untouched"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn runtime_ending_json_completion_retains_legacy_backend_and_leaves_sql_unchanged() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, _writes, _, _, _, _commands) = fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    assert!(
        crate::runtime::terminal_runtime::set_session_status_for_generation(
            &state, ID, generation, "killed"
        )
        .await
    );
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let result = EndingPersistence::json(state.clone(), ID.into(), generation)
        .complete(snapshot(), "ended".into())
        .await;
    let EndingCompletion::Json(result) = result else {
        panic!("JSON completion changed backend")
    };
    let result = result.unwrap();
    assert!(matches!(result.metadata, WriteResult::Written));
    assert!(matches!(result.event, WriteResult::Written));
    assert!(result.published);
    assert_eq!(
        files(&state).read_session(ID).unwrap().lifecycle_phase,
        "ended"
    );
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
    store.close().await.unwrap();
}
#[tokio::test]
async fn runtime_ending_sql_completion_keeps_file_failures_as_partial_stages() {
    let _guard = diagnostic_test_guard();
    for fault in ["grid", "event"] {
        let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
        let generation = ending(&writes, &state).await;
        let files = files(&state);
        let before = store.get(ID.into()).await.unwrap().unwrap();
        let persistence = EndingPersistence::sql(state.clone(), writes, ID.into(), generation)
            .await
            .unwrap();
        let blocked = files.root_path().join("events").join(format!(
            "{ID}.{}",
            if fault == "grid" {
                "terminal-size"
            } else {
                "ndjson"
            }
        ));
        std::fs::create_dir_all(blocked).unwrap();
        let EndingCompletion::Sql(result) = persistence.complete(snapshot(), "ended".into()).await
        else {
            panic!("SQL completion changed backend")
        };
        let result = result.unwrap();
        if fault == "grid" {
            assert!(matches!(result.replay, WriteResult::Failed(_)));
            assert!(matches!(result.metadata, MetadataResult::NotAttempted));
            assert!(matches!(result.event, WriteResult::NotAttempted));
            assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
        } else {
            assert!(matches!(result.replay, WriteResult::Written));
            assert!(matches!(
                result.metadata,
                MetadataResult::Completed(Outcome::Committed)
            ));
            assert!(matches!(result.event, WriteResult::Failed(_)));
            assert_eq!(
                state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
                "ended"
            );
        }
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn runtime_ending_sql_completion_keeps_unavailable_store_error_typed() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let generation = ending(&writes, &state).await;
    let persistence = EndingPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
    store.close().await.unwrap();
    assert!(matches!(
        persistence.complete(snapshot(), "ended".into()).await,
        EndingCompletion::Sql(Err(StoreError::Unavailable))
    ));
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ending"
    );
    assert!(files(&state).read_events(ID).is_empty());
}
#[tokio::test]
async fn runtime_ending_sql_capture_rejects_wrong_application_generation_and_phase() {
    let _guard = diagnostic_test_guard();
    let (dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    // Active is not finalization authority.
    assert!(matches!(
        EndingPersistence::sql(
            state.clone(),
            writes.clone(),
            ID.into(),
            runtime.run_generation
        )
        .await,
        Err(StoreError::Conflict)
    ));
    ending(&writes, &state).await;
    assert!(matches!(
        EndingPersistence::sql(
            state.clone(),
            writes.clone(),
            ID.into(),
            runtime.run_generation.wrapping_add(1)
        )
        .await,
        Err(StoreError::Conflict)
    ));
    // Only the AppState identity is consumed from `other`; the checked owner
    // still uses its original workspace. Do not legacy-open its SQL root.
    let other = crate::test_support::test_app_state_with_workspace(&dir.path().join("foreign-app"));
    assert!(matches!(
        writes
            .capture_ending_history(&other, ID.into(), runtime)
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["lifecyclePhase"],
        "ending"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn runtime_ending_sql_original_capture_rejects_replacements_across_wait() {
    let _guard = diagnostic_test_guard();
    for change in [
        "workspace",
        "reader",
        "lease",
        "runtime",
        "creation",
        "source",
        "attention",
        "phase",
    ] {
        let (dir, state, store, writes, _, _, _, _commands) = fixture().await;
        let generation = ending(&writes, &state).await;
        let old_files = files(&state);
        let before = store.get(ID.into()).await.unwrap().unwrap();
        let persistence = EndingPersistence::sql(state.clone(), writes, ID.into(), generation)
            .await
            .unwrap();
        match change {
            "workspace" => crate::test_support::replace_sql_workspace_context(&state, dir.path()),
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
                persistence.complete(snapshot(), "ended".into()).await,
                EndingCompletion::Sql(Err(StoreError::Conflict))
            ),
            "{change}"
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap(),
            before,
            "{change}"
        );
        assert!(old_files.read_terminal_size(ID).is_none());
        assert!(old_files.read_events(ID).is_empty());
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn runtime_ending_sql_dropped_capture_retains_then_releases_old_lease_without_effects() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let generation = ending(&writes, &state).await;
    let files = files(&state);
    let before = store.get(ID.into()).await.unwrap().unwrap();
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
    let persistence = EndingPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
    state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
    assert_eq!(
        lease.strong_count(),
        1,
        "capture lost original lease during a scan wait"
    );
    drop(persistence);
    assert_eq!(lease.strong_count(), 0);
    assert!(files.read_terminal_size(ID).is_none());
    assert!(files.read_events(ID).is_empty());
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
    store.close().await.unwrap();
}

#[tokio::test]
async fn runtime_ending_sql_metadata_completion_does_not_prove_physical_cleanup() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, mut commands) = fixture().await;
    let generation = ending(&writes, &state).await;
    state.session_pids.lock().unwrap().insert(ID.into(), 12345);
    let persistence = EndingPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
    let result = persistence.complete(snapshot(), "ended".into()).await;
    assert!(matches!(result, EndingCompletion::Sql(Ok(_))));
    assert_eq!(state.session_pids.lock().unwrap().get(ID), Some(&12345));
    assert!(matches!(
        commands.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
    {
        let sessions = state.sessions.lock().unwrap();
        let h = &sessions[ID];
        assert!(
            !*h.kill_tx.borrow(),
            "metadata completion requested process exit"
        );
        assert!(!crate::session_application::session_actions::runtime_closed(Some(h), true));
        assert!(
            !crate::session_application::session_actions::runtime_closed(Some(h), false),
            "live command channel was treated as physically closed"
        );
    }
    // Only the physical driver can supply the missing cleanup facts.
    state.session_pids.lock().unwrap().remove(ID);
    drop(commands);
    let sessions = state.sessions.lock().unwrap();
    assert!(crate::session_application::session_actions::runtime_closed(
        sessions.get(ID),
        false
    ));
    drop(sessions);
    store.close().await.unwrap();
}
#[tokio::test]
async fn runtime_ending_sql_cancelled_waiter_retains_definitive_event_and_publication() {
    use crate::session_application::ending_completion::testing::{hold, Stage};
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let generation = ending(&writes, &state).await;
    let persistence = EndingPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
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
    let waiter =
        tokio::spawn(async move { persistence.complete(snapshot(), "ended".into()).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    assert!(lane.clone().try_lock_owned().is_err());
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    drop(pause);
    let _released = tokio::time::timeout(std::time::Duration::from_secs(5), lane.lock_owned())
        .await
        .unwrap();
    assert_eq!(files(&state).read_events(ID).len(), 1);
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ended"
    );
    store.close().await.unwrap();
}
#[tokio::test(flavor = "current_thread")]
async fn runtime_ending_sql_capture_does_not_wait_on_sessions_from_tokio_worker() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let generation = ending(&writes, &state).await;
    let mut notice =
        crate::session_application::ending_completion::testing::notice_dispatch(&state);
    let locked_state = state.clone();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
    let blocker = std::thread::spawn(move || {
        let _sessions = locked_state.sessions.lock().unwrap();
        let _ = entered_tx.send(());
        progress_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .is_ok()
    });
    entered_rx.await.unwrap();
    let waiter =
        tokio::spawn(
            async move { EndingPersistence::sql(state, writes, ID.into(), generation).await },
        );
    notice.entered().await;
    let _ = progress_tx.send(());
    let persistence = waiter.await.unwrap().unwrap();
    assert!(
        blocker.join().unwrap(),
        "capture blocked the only Tokio worker until the OS watchdog released sessions"
    );
    drop(persistence);
    store.close().await.unwrap();
}
