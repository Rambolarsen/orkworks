use super::*;
fn busy<T>(result: Result<T, StoreError>) {
    assert!(matches!(result, Err(StoreError::Busy)));
}
#[test]
fn recommendation_owner_clones_fence_reads_mutations_and_release_for_retry() {
    let dir = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().into()).unwrap();
    let record = super::tests::recommendation("r", "s");
    store.put(&record).unwrap();
    let cloned = store.clone();
    let operation = cloned.try_operation().unwrap();
    busy(store.get("r"));
    busy(store.list());
    busy(store.list_with_hashes());
    busy(store.put(&record));
    busy(store.delete_referencing_session("s"));
    drop(operation);
    assert_eq!(store.get("r").unwrap(), Some(record));
    store.delete_referencing_session("s").unwrap();
    assert!(cloned.list().unwrap().is_empty());
}
#[test]
fn recommendation_owner_independent_open_rejects_before_recovery_or_directory_creation() {
    let dir = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().into()).unwrap();
    let operation = store.try_operation().unwrap();
    fs::remove_dir(&store.dir).unwrap();
    busy(RecommendationStore::open(dir.path().into()));
    assert!(!store.dir.exists());
    drop(operation);
    assert!(RecommendationStore::open(dir.path().into())
        .unwrap()
        .list()
        .unwrap()
        .is_empty());
}

#[test]
fn recommendation_owner_fences_every_composite_transition_without_modifying_records() {
    let dir = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().into()).unwrap();
    let record = super::tests::recommendation("r", "s");
    store.put(&record).unwrap();
    let before = fs::read(store.path_for("r")).unwrap();
    let operation = store.try_operation().unwrap();
    let request = super::super::completion::CompletionMutationRequest {
        packet_revision: 1,
        evidence_fingerprint: "a".repeat(64),
        idempotency_key: "operation-owner-test".into(),
    };
    busy(store.apply_recommendation_graph_transaction(&BTreeMap::new(), &[]));
    busy(store.dismiss("r", "later".into(), None));
    busy(store.begin_execution("r", "target".into(), "later".into()));
    busy(store.begin_execution_checked(
        "r",
        "target".into(),
        "later".into(),
        &request,
        "user".into(),
    ));
    busy(store.complete_execution("r", "later".into()));
    busy(store.complete_accepted("r", "later".into()));
    busy(store.complete_accepted_checked("r", "later".into(), &request));
    busy(store.cancel_execution("r", "later".into()));
    busy(store.replace_completion_packet(
        "r",
        1,
        crate::taskmaster::completion_tests::test_packet(),
        "later".into(),
    ));
    busy(store.recover_orphaned_executions(&HashSet::new(), &HashSet::new(), "later".into()));
    busy(store.scrub_orphans(&HashSet::new()));
    assert_eq!(fs::read(store.path_for("r")).unwrap(), before);
    drop(operation);
    assert_eq!(
        store
            .begin_execution("r", "target".into(), "later".into())
            .unwrap()
            .unwrap()
            .status,
        RecommendationStatus::Executing
    );
    assert_eq!(
        store
            .complete_execution("r", "later".into())
            .unwrap()
            .unwrap()
            .status,
        RecommendationStatus::Accepted
    );
    assert_eq!(
        store
            .complete_accepted("r", "later".into())
            .unwrap()
            .unwrap()
            .status,
        RecommendationStatus::Completed
    );
}
#[test]
fn recommendation_owner_canonical_opens_share_admission_and_other_roots_are_independent() {
    let dir = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().into()).unwrap();
    let alias = RecommendationStore::open(dir.path().join(".")).unwrap();
    let operation = alias.try_operation().unwrap();
    busy(store.list());
    busy(RecommendationStore::open(dir.path().join(".")));
    assert!(RecommendationStore::open(other.path().into())
        .unwrap()
        .list()
        .unwrap()
        .is_empty());
    drop(operation);
    assert!(store.list().unwrap().is_empty());
}
#[cfg(unix)]
#[test]
fn recommendation_owner_symlink_root_open_cannot_bypass_an_active_operation() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let alias = dir.path().join("alias");
    let store = RecommendationStore::open(root).unwrap();
    std::os::unix::fs::symlink(store.dir.parent().unwrap(), &alias).unwrap();
    let operation = store.try_operation().unwrap();
    busy(RecommendationStore::open(alias.clone()));
    drop(operation);
    let alias = RecommendationStore::open(alias).unwrap();
    let operation = alias.try_operation().unwrap();
    busy(store.list());
    drop(operation);
}
#[test]
fn recommendation_owner_error_unwind_and_original_store_drop_release_the_permit() {
    let dir = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().into()).unwrap();
    let mut bad = super::tests::recommendation("r", "s");
    bad.completion_packet = Some(crate::taskmaster::completion_tests::test_packet());
    bad.completion_packet.as_mut().unwrap().evidence_fingerprint = "forged".into();
    assert!(store.put(&bad).is_err());
    assert!(store.list().unwrap().is_empty());
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _operation = store.try_operation().unwrap();
        panic!("controlled operation unwind");
    }));
    assert!(caught.is_err());
    assert!(store.list().unwrap().is_empty());
    let operation = store.try_operation().unwrap();
    drop(store);
    busy(RecommendationStore::open(dir.path().into()));
    drop(operation);
    assert!(RecommendationStore::open(dir.path().into())
        .unwrap()
        .list()
        .unwrap()
        .is_empty());
}
#[test]
fn recommendation_owner_busy_reads_and_open_preserve_pending_recovery_until_release() {
    let dir = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().into()).unwrap();
    let a = super::tests::recommendation("a", "s-a");
    let b = super::tests::recommendation("b", "s-b");
    store.put(&a).unwrap();
    store.put(&b).unwrap();
    let (_, hashes) = store.list_with_hashes().unwrap();
    let expected = hashes
        .into_iter()
        .map(|(id, hash)| (id, Some(hash)))
        .collect();
    let mut next_a = a.clone();
    next_a.title = "after-a".into();
    let mut next_b = b.clone();
    next_b.title = "after-b".into();
    set_fault_point(Some(FaultPoint::Publication(1)));
    assert!(store
        .apply_recommendation_graph_transaction(&expected, &[next_a.clone(), next_b.clone()])
        .is_err());
    set_fault_point(None);
    let operation = store.try_operation().unwrap();
    let before_a = fs::read(store.path_for("a")).unwrap();
    let before_b = fs::read(store.path_for("b")).unwrap();
    let transactions = store.dir.join(ROLLUP_TRANSACTION_DIR);
    assert!(fs::read_dir(&transactions).unwrap().next().is_some());
    busy(store.list());
    busy(store.list_with_hashes());
    busy(store.get("a"));
    busy(RecommendationStore::open(dir.path().into()));
    assert_eq!(fs::read(store.path_for("a")).unwrap(), before_a);
    assert_eq!(fs::read(store.path_for("b")).unwrap(), before_b);
    assert!(fs::read_dir(&transactions).unwrap().next().is_some());
    drop(operation);
    let reopened = RecommendationStore::open(dir.path().into()).unwrap();
    assert_eq!(reopened.get("a").unwrap(), Some(next_a));
    assert_eq!(reopened.get("b").unwrap(), Some(next_b));
    assert!(!transactions.exists() || fs::read_dir(transactions).unwrap().next().is_none());
}
#[test]
fn recommendation_owner_redaction_open_observes_the_existing_owner() {
    let dir = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().into()).unwrap();
    let record = super::tests::recommendation("r", "s");
    store.put(&record).unwrap();
    let before = fs::read(store.path_for("r")).unwrap();
    let operation = store.try_operation().unwrap();
    assert!(super::super::assessment_access::reconcile_metadata(
        dir.path(),
        &super::super::runtime::TaskmasterSettings::default()
    )
    .is_err());
    assert_eq!(fs::read(store.path_for("r")).unwrap(), before);
    drop(operation);
    super::super::assessment_access::reconcile_metadata(
        dir.path(),
        &super::super::runtime::TaskmasterSettings::default(),
    )
    .unwrap();
    assert_eq!(store.get("r").unwrap(), Some(record));
}
#[tokio::test(flavor = "current_thread")]
async fn recommendation_owner_busy_application_read_leaves_acknowledged_input_free() {
    use crate::runtime::session_runtime::{RuntimeCommand, SessionRuntime};
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let captured = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .recommendation_store
        .clone();
    let operation = captured.try_operation().unwrap();
    let app = crate::session_application::SessionApplication::new(state.clone());
    assert!(app.list_recommendations().is_err());
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(state.session_pids.try_lock().is_ok());
    let (runtime, mut commands) = SessionRuntime::live(24, 80);
    let mut info = crate::test_support::test_session_info(
        "recommendation-input",
        "Input",
        dir.path().display().to_string(),
        "running",
        "2026-01-01T00:00:00Z",
    );
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    state.sessions.lock().unwrap().insert(
        "recommendation-input".into(),
        crate::SessionHandle {
            info,
            runtime,
            kill_tx: tokio::sync::watch::channel(false).0,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
            active_work_hook: false,
        },
    );
    let driver = tokio::spawn(async move {
        match commands.recv().await.unwrap() {
            RuntimeCommand::Input {
                data,
                accepted: Some(accepted),
            } => {
                assert_eq!(data, "recommendation-input\r");
                let _ = accepted.send(Ok(()));
            }
            _ => panic!("expected acknowledged input"),
        }
    });
    let delivered = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            "recommendation-input",
            "recommendation-input\r".into(),
        ),
    )
    .await;
    driver.await.unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    drop(operation);
    assert!(app.list_recommendations().is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn recommendation_owner_delivery_settlement_waits_for_contention_without_resending() {
    use crate::runtime::session_runtime::{RuntimeCommand, SessionRuntime};
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for (delivered, cancel_caller) in [(true, false), (false, false), (true, true), (false, true)] {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let id = "settlement-target";
        let rec_id = "settlement-recommendation";
        let store = {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            let mut metadata = crate::test_support::test_session_metadata(
                id,
                "Delivery",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            metadata.lifecycle = "alive".into();
            metadata.lifecycle_phase = "active".into();
            ws.metadata.try_write_session(&metadata).unwrap();
            ws.recommendation_store
                .put(&super::tests::recommendation(rec_id, "source"))
                .unwrap();
            ws.recommendation_store.clone()
        };
        let (runtime, mut commands) = SessionRuntime::live(24, 80);
        let mut info = crate::test_support::test_session_info(
            id,
            "Delivery",
            dir.path().display().to_string(),
            "running",
            "now",
        );
        info.lifecycle = "alive".into();
        info.lifecycle_phase = "active".into();
        state.sessions.lock().unwrap().insert(
            id.into(),
            crate::SessionHandle {
                info,
                runtime,
                kill_tx: tokio::sync::watch::channel(false).0,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                terminal_attached: false,
                resume_in_progress: false,
                capacity: Default::default(),
                active_work_hook: false,
            },
        );
        let request_state = state.clone();
        let mut request = Some(tokio::spawn(async move {
            crate::session_application::SessionApplication::new(request_state)
                .accept_recommendation_with_packet(rec_id, id, None, None)
                .await
        }));
        let RuntimeCommand::Input {
            accepted: Some(accepted),
            ..
        } = commands.recv().await.unwrap()
        else {
            panic!("expected input")
        };
        let operation = store.try_operation().unwrap();
        if cancel_caller {
            let request = request.take().unwrap();
            request.abort();
            assert!(request.await.unwrap_err().is_cancelled());
        }
        accepted
            .send(if delivered { Ok(()) } else { Err(()) })
            .unwrap();
        let waiting = if let Some(request) = request.as_mut() {
            tokio::time::timeout(std::time::Duration::from_millis(30), request)
                .await
                .is_err()
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            true
        };
        let retained = crate::session_application::recommendation_delivery_in_flight_id(rec_id);
        assert!(state.workspace.try_lock().is_ok());
        assert!(state.projection_lock.try_lock().is_ok());
        assert!(state.sessions.try_lock().is_ok());
        drop(operation);
        assert!(
            waiting && retained,
            "contention discarded a delivered/failed input's settlement guard"
        );
        if let Some(request) = request.take() {
            let result = tokio::time::timeout(std::time::Duration::from_secs(1), request)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(result.is_ok(), delivered);
        } else {
            tokio::time::timeout(std::time::Duration::from_secs(1), async {
                while crate::session_application::recommendation_delivery_in_flight_id(rec_id) {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        }
        assert_eq!(
            store.get(rec_id).unwrap().unwrap().status,
            if delivered {
                RecommendationStatus::Accepted
            } else {
                RecommendationStatus::Proposed
            }
        );
        assert!(
            commands.try_recv().is_err(),
            "settlement must never resend terminal input"
        );
        let ws = state.workspace.lock().unwrap();
        let events = ws.as_ref().unwrap().metadata.read_events(id);
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "taskmaster_fix_requested")
                .count(),
            usize::from(delivered)
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn recommendation_owner_release_notification_handles_early_release_and_reclaim() {
    use std::{future::Future, task::Poll};
    let dir = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().into()).unwrap();
    let operation = store.try_operation().unwrap();
    let wait = store.wait_until_available();
    drop(operation);
    tokio::time::timeout(std::time::Duration::from_secs(1), wait)
        .await
        .unwrap();
    let operation = store.try_operation().unwrap();
    let mut wait = Box::pin(store.wait_until_available());
    std::future::poll_fn(|cx| {
        assert!(wait.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(operation);
    let reclaimed = store.try_operation().unwrap();
    std::future::poll_fn(|cx| {
        assert!(wait.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(reclaimed);
    tokio::time::timeout(std::time::Duration::from_secs(1), wait)
        .await
        .unwrap();
}
fn delivery_fixture(
    id: &str,
    rec_id: &str,
) -> (
    tempfile::TempDir,
    std::sync::Arc<crate::AppState>,
    RecommendationStore,
    tokio::sync::mpsc::Receiver<crate::runtime::session_runtime::RuntimeCommand>,
) {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let store = {
        let ws = state.workspace.lock().unwrap();
        let ws = ws.as_ref().unwrap();
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Delivery",
            dir.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.lifecycle = "alive".into();
        metadata.lifecycle_phase = "active".into();
        ws.metadata.try_write_session(&metadata).unwrap();
        ws.recommendation_store
            .put(&super::tests::recommendation(rec_id, "source"))
            .unwrap();
        ws.recommendation_store.clone()
    };
    let (runtime, commands) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let mut info = crate::test_support::test_session_info(
        id,
        "Delivery",
        dir.path().display().to_string(),
        "running",
        "now",
    );
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    state.sessions.lock().unwrap().insert(
        id.into(),
        crate::SessionHandle {
            info,
            runtime,
            kill_tx: tokio::sync::watch::channel(false).0,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
            active_work_hook: false,
        },
    );
    (dir, state, store, commands)
}
#[tokio::test(flavor = "current_thread")]
async fn recommendation_owner_delivery_retry_does_not_finalize_a_replacement_workspace() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, mut commands) =
        delivery_fixture("replace-target", "replace-recommendation");
    let sent_state = state.clone();
    let mut request = tokio::spawn(async move {
        crate::session_application::SessionApplication::new(sent_state)
            .accept_recommendation_with_packet(
                "replace-recommendation",
                "replace-target",
                None,
                None,
            )
            .await
    });
    let crate::runtime::session_runtime::RuntimeCommand::Input {
        accepted: Some(accepted),
        ..
    } = tokio::time::timeout(std::time::Duration::from_secs(1), commands.recv())
        .await
        .unwrap()
        .unwrap()
    else {
        panic!("expected input")
    };
    let operation = store.try_operation().unwrap();
    accepted.send(Ok(())).unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(30), &mut request)
            .await
            .is_err()
    );
    let (_other, replacement, replacement_store, _commands) =
        delivery_fixture("replace-target", "replace-recommendation");
    *state.workspace.lock().unwrap() = replacement.workspace.lock().unwrap().take();
    drop(operation);
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(1), request)
            .await
            .unwrap()
            .unwrap(),
        Err(crate::session_application::RecommendationAcceptError::Conflict)
    ));
    assert_eq!(
        store.get("replace-recommendation").unwrap().unwrap().status,
        RecommendationStatus::Executing
    );
    assert_eq!(
        replacement_store
            .get("replace-recommendation")
            .unwrap()
            .unwrap()
            .status,
        RecommendationStatus::Proposed
    );
    assert!(
        !crate::session_application::recommendation_delivery_in_flight_id("replace-recommendation")
    );
}
#[tokio::test(flavor = "current_thread")]
async fn recommendation_owner_delivery_persistence_failure_is_not_retried_as_contention() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, mut commands) = delivery_fixture("io-target", "io-recommendation");
    let sent_state = state.clone();
    let request = tokio::spawn(async move {
        crate::session_application::SessionApplication::new(sent_state)
            .accept_recommendation_with_packet("io-recommendation", "io-target", None, None)
            .await
    });
    let crate::runtime::session_runtime::RuntimeCommand::Input {
        accepted: Some(accepted),
        ..
    } = tokio::time::timeout(std::time::Duration::from_secs(1), commands.recv())
        .await
        .unwrap()
        .unwrap()
    else {
        panic!("expected input")
    };
    let obstruction = store
        .path_for("io-recommendation")
        .with_extension("json.tmp");
    fs::create_dir(&obstruction).unwrap();
    accepted.send(Ok(())).unwrap();
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(1), request)
            .await
            .unwrap()
            .unwrap(),
        Err(crate::session_application::RecommendationAcceptError::Store(StoreError::Io(_)))
    ));
    assert_eq!(
        store.get("io-recommendation").unwrap().unwrap().status,
        RecommendationStatus::Executing
    );
    assert!(!crate::session_application::recommendation_delivery_in_flight_id("io-recommendation"));
    assert!(commands.try_recv().is_err());
    assert_eq!(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events("io-target")
            .iter()
            .filter(|e| e.event_type == "taskmaster_fix_requested")
            .count(),
        0
    );
}
