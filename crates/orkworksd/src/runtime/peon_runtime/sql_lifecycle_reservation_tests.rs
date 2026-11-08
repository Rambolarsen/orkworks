use super::sql_reservation_tests::fixture;
use super::*;
use crate::session_application::sql_writes::{lifecycle::LifecycleWrite, Outcome};
const ID: &str = "sql-runtime";

#[tokio::test]
async fn sql_lifecycle_transition_and_finalization_share_committed_fields() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _context, _capture, _, _commands) = fixture().await;
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    let legacy_path = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .sessions_dir()
        .join(format!("{ID}.json"));
    let legacy_before = std::fs::read(&legacy_path).unwrap();
    assert_eq!(
        writes
            .lifecycle(
                ID.into(),
                runtime.clone(),
                LifecycleWrite::Transition {
                    status: "killed".into(),
                }
            )
            .await
            .unwrap(),
        Outcome::Committed
    );
    let ending = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(ending["lifecyclePhase"], "ending");
    assert_eq!(ending["pendingTerminalStatus"], "killed");
    assert_eq!(
        writes
            .lifecycle(ID.into(), runtime.clone(), stop())
            .await
            .unwrap(),
        Outcome::Noop {
            permanent_hold: false
        }
    );
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), ending);
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle,
        "stopping"
    );
    assert_eq!(
        writes
            .lifecycle(
                ID.into(),
                runtime,
                LifecycleWrite::Finalize {
                    snapshot: crate::metadata::ObservedStatusSnapshotMetadata {
                        value: Some("blocked".into()),
                        source: "peon".into(),
                        confidence: Some(0.8),
                        observed_at: None,
                        attention_origin: None,
                    },
                    fallback_status: "ended".into(),
                }
            )
            .await
            .unwrap(),
        Outcome::Committed
    );
    let ended = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(ended["status"], "killed");
    assert_eq!(ended["lifecyclePhase"], "ended");
    let sessions = state.sessions.lock().unwrap();
    assert_eq!(sessions[ID].info.status, "killed");
    assert_eq!(sessions[ID].info.lifecycle, "dead");
    assert_eq!(
        sessions[ID].info.final_observed_status.as_deref(),
        Some("blocked")
    );
    drop(sessions);
    assert_eq!(std::fs::read(legacy_path).unwrap(), legacy_before);
    store.close().await.unwrap();
}

fn stop() -> LifecycleWrite {
    LifecycleWrite::Transition {
        status: "killed".into(),
    }
}
fn identity(state: &Arc<AppState>) -> crate::runtime::session_runtime::RuntimeIdentity {
    state.sessions.lock().unwrap()[ID].runtime.identity()
}

#[tokio::test]
async fn sql_lifecycle_revalidates_original_owner_before_claim() {
    use crate::session_store::{test_worker, StoreError};
    let _guard = diagnostic_test_guard();
    for change in [
        "runtime",
        "control",
        "creation",
        "record_id",
        "harness",
        "harness_id",
        "status",
        "lifecycle",
        "resumed_from",
        "attention",
        "metadata_source",
        "metadata_confidence",
        "provenance",
        "phase",
        "workspace",
        "reader",
        "lease",
        "removed",
    ] {
        let (dir, state, store, writes, _, _, _, _commands) = fixture().await;
        let runtime = identity(&state);
        let pause = test_worker::pause(&store);
        let writer =
            tokio::spawn(async move { writes.lifecycle(ID.into(), runtime, stop()).await });
        test_worker::wait_for_internal(&store, ID).await;
        match change {
            "workspace" => crate::test_support::replace_sql_workspace_context(&state, dir.path()),
            "reader" => {
                let mut ws = state.workspace.lock().unwrap();
                let ws = ws.as_mut().unwrap();
                ws.metadata = crate::metadata::MetadataStore::new(&ws.metadata.root_path());
            }
            "lease" => {
                state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
            }
            "removed" => {
                state.sessions.lock().unwrap().remove(ID);
            }
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
                    "record_id" => h.info.id = "replacement".into(),
                    "harness" => h.info.harness = Some("changed".into()),
                    "harness_id" => h.info.harness_id = Some("changed".into()),
                    "status" => h.info.status = "creating".into(),
                    "lifecycle" => h.info.lifecycle = "stopping".into(),
                    "resumed_from" => h.info.resumed_from = Some("other".into()),
                    "attention" => {
                        h.runtime.attention_owner.accepted_write();
                    }
                    "metadata_source" => h.info.metadata_source = Some("user".into()),
                    "metadata_confidence" => h.info.metadata_confidence = Some(0.91),
                    "provenance" => h.info.provenance.attention_source = Some("user".into()),
                    "phase" => h.info.lifecycle_phase = "ending".into(),
                    _ => unreachable!(),
                }
            }
        }
        drop(pause);
        assert!(
            matches!(writer.await.unwrap(), Err(StoreError::Conflict)),
            "stale {change} executed"
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "1",
            "{change}"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_lifecycle_cancelled_queue_cannot_write_later() {
    use crate::session_store::test_worker;
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let runtime = identity(&state);
    let pause = test_worker::pause(&store);
    let writer = tokio::spawn(async move { writes.lifecycle(ID.into(), runtime, stop()).await });
    test_worker::wait_for_internal(&store, ID).await;
    writer.abort();
    assert!(writer.await.unwrap_err().is_cancelled());
    drop(pause);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_lifecycle_replacement_during_execution_detaches_publication() {
    use crate::session_store::test_worker;
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let runtime = identity(&state);
    let mut pause = test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move { writes.lifecycle(ID.into(), runtime, stop()).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
        crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
    drop(pause);
    assert_eq!(writer.await.unwrap().unwrap(), Outcome::CommittedDetached);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["lifecyclePhase"],
        "ending"
    );
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "active"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_lifecycle_cancelled_execution_retains_lane_lease_and_publishes() {
    use crate::session_store::test_worker;
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let runtime = identity(&state);
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
            crate::metadata::internal_mutations::InternalMutation::UserTitle {
                title: Some("other".into()),
            },
        )
        .unwrap();
    let lane = prepared.lane().unwrap();
    drop(prepared);
    let mut pause = test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move { writes.lifecycle(ID.into(), runtime, stop()).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(lane.clone().try_lock_owned().is_err());
    assert!(
        lease.strong_count() >= 2,
        "executing owner lost workspace lease"
    );
    writer.abort();
    assert!(writer.await.unwrap_err().is_cancelled());
    assert!(lane.clone().try_lock_owned().is_err());
    drop(pause);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if state.sessions.lock().unwrap()[ID].info.lifecycle_phase == "ending" {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let _released = tokio::time::timeout(std::time::Duration::from_secs(2), lane.lock_owned())
        .await
        .unwrap();
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["lifecyclePhase"],
        "ending"
    );
    assert_eq!(
        lease.strong_count(),
        1,
        "definitive publication leaked its lease"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_lifecycle_metadata_lane_wait_expires_without_shared_guards_or_write() {
    use crate::session_store::StoreError;
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let runtime = identity(&state);
    let prepared = store
        .prepare_internal(
            ID.into(),
            "2026-01-01T00:00:00Z".into(),
            crate::metadata::internal_mutations::InternalMutation::UserTitle {
                title: Some("other".into()),
            },
        )
        .unwrap();
    let lane = prepared.lane().unwrap();
    let held = lane.lock_owned().await;
    drop(prepared);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(4),
        writes.lifecycle(ID.into(), runtime, stop()),
    )
    .await
    .unwrap();
    assert!(matches!(result, Err(StoreError::Expired)));
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    drop(held);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_lifecycle_different_session_does_not_wait_for_held_metadata_lane() {
    let _guard = diagnostic_test_guard();
    let (dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let prepared = store
        .prepare_internal(
            ID.into(),
            "2026-01-01T00:00:00Z".into(),
            crate::metadata::internal_mutations::InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let held = prepared.lane().unwrap().lock_owned().await;
    drop(prepared);
    let other = "sql-other";
    let meta = crate::test_support::test_session_metadata(
        other,
        "Other",
        dir.path().display().to_string(),
        "running",
        "2026-01-01T00:00:00Z",
        "2026-01-02T00:00:00Z",
    );
    store
        .create(serde_json::to_value(meta).unwrap())
        .await
        .unwrap();
    let (runtime, _commands2) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let rid = runtime.identity();
    state.sessions.lock().unwrap().insert(
        other.into(),
        crate::SessionHandle {
            info: crate::test_support::test_session_info(
                other,
                "Other",
                dir.path().display().to_string(),
                "running",
                "2026-01-01T00:00:00Z",
            ),
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
    assert_eq!(
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            writes.lifecycle(other.into(), rid, stop())
        )
        .await
        .unwrap()
        .unwrap(),
        Outcome::Committed
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    drop(held);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_lifecycle_failure_and_invalid_intent_do_not_publish() {
    use crate::session_store::StoreError;
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let runtime = identity(&state);
    assert!(matches!(
        writes
            .lifecycle(
                ID.into(),
                runtime.clone(),
                LifecycleWrite::Transition {
                    status: "not-a-status".into()
                }
            )
            .await,
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        writes
            .lifecycle(
                ID.into(),
                runtime.clone(),
                LifecycleWrite::Finalize {
                    snapshot: crate::metadata::ObservedStatusSnapshotMetadata {
                        value: None,
                        source: "unknown".into(),
                        confidence: None,
                        observed_at: None,
                        attention_origin: None
                    },
                    fallback_status: "ended".into(),
                }
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
    assert!(matches!(
        writes.lifecycle(ID.into(), runtime, stop()).await,
        Err(StoreError::Unavailable)
    ));
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "active"
    );
}
