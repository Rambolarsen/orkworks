use super::organization_tests::{finalize, fixture, organization, pending, root, CREATED, ID};
use super::*;
use crate::session_application::session_actions::SessionAction;
use std::time::Duration;

#[tokio::test]
async fn sql_delete_waits_for_original_close_and_leaves_recoverable_job_and_payloads() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, mut killed, commands) = fixture().await;
    let legacy = std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap();
    let payload = root(&state).join("events").join(format!("{ID}.terminal"));
    std::fs::create_dir_all(payload.parent().unwrap()).unwrap();
    std::fs::write(&payload, b"retained replay").unwrap();
    state.session_pids.lock().unwrap().insert(ID.into(), 123);
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let mut action = tokio::spawn(view.begin_delete());
    tokio::time::timeout(Duration::from_secs(3), killed.changed())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), finalize(&state, &writes))
        .await
        .unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(75), &mut action)
        .await
        .is_err());
    assert!(store
        .pending_deletions(None, 100)
        .await
        .unwrap()
        .jobs
        .is_empty());
    state.session_pids.lock().unwrap().remove(ID);
    assert!(tokio::time::timeout(Duration::from_millis(75), &mut action)
        .await
        .is_err());
    drop(commands);
    let receipt = action.await.unwrap().unwrap();
    assert_eq!(receipt.id, ID);
    assert_eq!(receipt.created_at, CREATED);
    assert!(matches!(receipt.outcome, Outcome::Committed));
    assert!(store.get(ID.into()).await.unwrap().is_none());
    let pending = store
        .deletion_record(ID.into(), CREATED.into())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.job.progress, 0);
    assert_eq!(pending.document["metadataRevision"], "3");
    assert_eq!(std::fs::read(&payload).unwrap(), b"retained replay");
    assert_eq!(
        std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap(),
        legacy
    );
    let (path, workspace) = store.location();
    let (path, workspace) = (path.to_owned(), workspace.to_owned());
    store.close().await.unwrap();
    let reopened = SessionStore::open(path, workspace).await.unwrap();
    assert_eq!(
        reopened
            .pending_deletions(None, 100)
            .await
            .unwrap()
            .jobs
            .len(),
        1
    );
    reopened.close().await.unwrap();
}

#[tokio::test]
async fn sql_delete_admits_closed_archived_and_off_page_history_without_stopping() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for case in ["closed", "archived", "history-only"] {
        let (_dir, state, store, writes, killed, commands) = fixture().await;
        finalize(&state, &writes).await;
        drop(commands);
        if case == "archived" {
            writes
                .capture_organization(ID.into())
                .await
                .unwrap()
                .apply(SessionAction::Archive)
                .await
                .unwrap();
        }
        if case == "history-only" {
            state.sessions.lock().unwrap().remove(ID);
        }
        let receipt = writes
            .capture_organization(ID.into())
            .await
            .unwrap()
            .begin_delete()
            .await
            .unwrap();
        assert!(matches!(receipt.outcome, Outcome::Committed));
        assert!(!*killed.borrow());
        assert_eq!(
            store.pending_deletions(None, 100).await.unwrap().jobs[0].id,
            ID
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_delete_stale_owner_or_choice_never_stops_or_creates_a_job() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in ["control", "runtime", "creation", "lease", "organization"] {
        let (_dir, state, store, writes, killed, _commands) = fixture().await;
        let view = writes.capture_organization(ID.into()).await.unwrap();
        match change {
            "control" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .runtime
                    .control_tx = tokio::sync::mpsc::channel(2).0
            }
            "runtime" => {
                state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
                    crate::runtime::session_runtime::SessionRuntime::live(24, 80).0
            }
            "creation" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .info
                    .created_at = "replacement".into()
            }
            "lease" => state.workspace.lock().unwrap().as_mut().unwrap().lease = None,
            "organization" => {
                let original_root = root(&state);
                state
                    .workspace
                    .lock()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .metadata
                    .organization = Arc::new(crate::session_archive::OrganizationStore::new(
                    &original_root,
                ));
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(view.begin_delete().await, Err(StoreError::Conflict)),
            "{change}"
        );
        assert!(!*killed.borrow());
        assert!(store
            .pending_deletions(None, 100)
            .await
            .unwrap()
            .jobs
            .is_empty());
        store.close().await.unwrap();
    }
    let (_dir, state, store, writes, killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let view = writes.capture_organization(ID.into()).await.unwrap();
    store
        .organize(ID.into(), CREATED.into(), "0".into(), "archived".into())
        .await
        .unwrap();
    assert!(matches!(
        view.begin_delete().await,
        Err(StoreError::Conflict)
    ));
    assert!(!*killed.borrow());
    assert!(store
        .pending_deletions(None, 100)
        .await
        .unwrap()
        .jobs
        .is_empty());
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_delete_timeout_or_cancelled_close_never_admits_after_finalization() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (_dir, state, store, writes, mut killed, commands) = fixture().await;
        let original = organization(&state);
        let view = writes.capture_organization(ID.into()).await.unwrap();
        if cancel {
            let action = tokio::spawn(view.begin_delete());
            killed.changed().await.unwrap();
            action.abort();
            assert!(action.await.unwrap_err().is_cancelled());
        } else {
            assert!(matches!(
                view.begin_delete_with_timeout(Duration::ZERO).await,
                Err(StoreError::Expired)
            ));
        }
        pending(&original, false).await;
        finalize(&state, &writes).await;
        drop(commands);
        assert!(store
            .pending_deletions(None, 100)
            .await
            .unwrap()
            .jobs
            .is_empty());
        assert!(store.get(ID.into()).await.unwrap().is_some());
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_delete_rechecks_canonical_revision_and_physical_owner_at_grant() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "revision",
        "organization",
        "pid",
        "control",
        "creation",
        "canonical",
    ] {
        let (_dir, state, store, writes, _killed, commands) = fixture().await;
        finalize(&state, &writes).await;
        drop(commands);
        let lane = store.runtime_lane(ID).unwrap().lock_owned().await;
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let (ready, waiting) = oneshot::channel();
        *writes.before_organization_lane.lock().unwrap() = Some(ready);
        let action = tokio::spawn(view.begin_delete());
        waiting.await.unwrap();
        match change {
            "revision" => {
                store
                    .mutate_internal(
                        ID.into(),
                        CREATED.into(),
                        None,
                        crate::metadata::internal_mutations::InternalMutation::UserTitle {
                            title: Some("Later title".into()),
                        },
                    )
                    .await
                    .unwrap();
            }
            "canonical" => {
                let db = rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite"))
                    .unwrap();
                db.execute("UPDATE sessions SET metadata=json_set(metadata,'$.lifecycle','alive','$.lifecyclePhase','active','$.status','running') WHERE id=?1", [ID]).unwrap();
            }
            "creation" => {
                let db = rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite"))
                    .unwrap();
                db.execute("UPDATE sessions SET created_at='replacement',metadata=json_set(metadata,'$.createdAt','replacement') WHERE id=?1", [ID]).unwrap();
            }
            "organization" => {
                store
                    .organize(ID.into(), CREATED.into(), "0".into(), "archived".into())
                    .await
                    .unwrap();
            }
            "pid" => {
                state.session_pids.lock().unwrap().insert(ID.into(), 123);
            }
            "control" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .runtime
                    .control_tx = tokio::sync::mpsc::channel(2).0;
            }
            _ => unreachable!(),
        }
        drop(lane);
        assert!(
            matches!(action.await.unwrap(), Err(StoreError::Conflict)),
            "{change}"
        );
        assert!(store
            .pending_deletions(None, 100)
            .await
            .unwrap()
            .jobs
            .is_empty());
        assert!(store.get(ID.into()).await.unwrap().is_some());
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_delete_queued_expiry_releases_resources_without_late_admission() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let original = organization(&state);
    let lane = store.runtime_lane(ID).unwrap().lock_owned().await;
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let (ready, waiting) = oneshot::channel();
    *writes.before_organization_lane.lock().unwrap() = Some(ready);
    let action = tokio::spawn(view.begin_delete());
    waiting.await.unwrap();
    let stalled = crate::session_store::test_worker::pause(&store);
    drop(lane);
    crate::session_store::test_worker::wait_for_organization(&store, ID).await;
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), action)
            .await
            .unwrap()
            .unwrap(),
        Err(StoreError::Expired)
    ));
    pending(&original, false).await;
    assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_ok());
    drop(stalled);
    assert!(store
        .pending_deletions(None, 100)
        .await
        .unwrap()
        .jobs
        .is_empty());
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_delete_admitted_cancellation_retains_lease_lane_and_pending_job() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let original = organization(&state);
    let weak = Arc::downgrade(
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
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let mut held = crate::session_store::test_worker::hold_next_organization(&store, ID);
    let action = tokio::spawn(view.begin_delete());
    held.entered().await;
    action.abort();
    assert!(action.await.unwrap_err().is_cancelled());
    state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
    assert!(weak.upgrade().is_some());
    assert!(original.pending(ID));
    assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_err());
    drop(held);
    assert_eq!(
        store.pending_deletions(None, 100).await.unwrap().jobs.len(),
        1
    );
    pending(&original, false).await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while weak.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_ok());
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_delete_admitted_receipt_detaches_from_a_replacement_runtime() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let mut held = crate::session_store::test_worker::hold_next_organization(&store, ID);
    let action = tokio::spawn(view.begin_delete());
    held.entered().await;
    state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
        crate::runtime::session_runtime::SessionRuntime::live(24, 80).0;
    let replacement = state.sessions.lock().unwrap()[ID].runtime.identity();
    drop(held);
    let receipt = action.await.unwrap().unwrap();
    assert!(matches!(receipt.outcome, Outcome::CommittedDetached));
    assert_eq!(receipt.created_at, CREATED);
    assert_eq!(
        state.sessions.lock().unwrap()[ID].runtime.identity(),
        replacement
    );
    assert!(store
        .deletion_record(ID.into(), CREATED.into())
        .await
        .unwrap()
        .is_some());
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_delete_successful_grant_survives_cancellation_before_delivery() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (_dir, state, store, writes, _killed, commands) = fixture().await;
        finalize(&state, &writes).await;
        drop(commands);
        let original = organization(&state);
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let (ready, entered) = oneshot::channel();
        let (release, delivery) = oneshot::channel();
        *writes.after_organization_grant.lock().unwrap() = Some((ready, delivery));
        let mut action = tokio::spawn(view.begin_delete());
        tokio::time::timeout(Duration::from_secs(5), entered)
            .await
            .unwrap()
            .unwrap();
        if cancel {
            action.abort();
            assert!((&mut action).await.unwrap_err().is_cancelled());
        } else {
            assert!(
                tokio::time::timeout(Duration::from_millis(2100), &mut action)
                    .await
                    .is_err(),
                "successful grant expired before delivery"
            );
        }
        assert!(original.pending(ID));
        release.send(()).expect("admitted grant was discarded");
        if !cancel {
            action.await.unwrap().unwrap();
        }
        pending(&original, false).await;
        assert_eq!(
            store.pending_deletions(None, 100).await.unwrap().jobs.len(),
            1
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_delete_revokes_before_stop_and_sql_failure_never_falls_back() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, mut killed, _commands) = fixture().await;
    let authority = crate::taskmaster::publication_lifecycle::testing::Fixture::new();
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut(ID)
        .unwrap()
        .runtime
        .evaluation_revocation = Some(authority.runtime.clone());
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let action = tokio::spawn(view.begin_delete_with_timeout(Duration::ZERO));
    tokio::time::timeout(Duration::from_secs(3), killed.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(!authority.can_respond());
    assert!(matches!(action.await.unwrap(), Err(StoreError::Expired)));
    assert!(store
        .pending_deletions(None, 100)
        .await
        .unwrap()
        .jobs
        .is_empty());
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let legacy = std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap();
    store.close().await.unwrap();
    assert!(matches!(
        view.begin_delete().await,
        Err(StoreError::Unavailable)
    ));
    assert_eq!(
        std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap(),
        legacy
    );
}

#[tokio::test(flavor = "current_thread")]
async fn sql_delete_stall_keeps_guards_and_another_runtime_input_receiver_available() {
    super::organization_tests::stalled_organization_keeps_input_available(true).await;
}

#[tokio::test]
async fn sql_delete_busy_or_full_queue_leaves_visible_session_and_files() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for fault in ["queue", "busy"] {
        let (_dir, state, store, writes, _killed, commands) = fixture().await;
        finalize(&state, &writes).await;
        drop(commands);
        let legacy = std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap();
        let view = writes.capture_organization(ID.into()).await.unwrap();
        if fault == "queue" {
            let held: Vec<_> = (0..32)
                .map(|_| {
                    store
                        .prepare_organization(
                            ID.into(),
                            CREATED.into(),
                            "0".into(),
                            "archived".into(),
                        )
                        .unwrap()
                })
                .collect();
            assert!(matches!(
                view.begin_delete().await,
                Err(StoreError::QueueFull)
            ));
            drop(held);
        } else {
            let lane = store.runtime_lane(ID).unwrap().lock_owned().await;
            let (ready, waiting) = oneshot::channel();
            *writes.before_organization_lane.lock().unwrap() = Some(ready);
            let action = tokio::spawn(view.begin_delete());
            waiting.await.unwrap();
            let db =
                rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
            db.execute_batch("BEGIN EXCLUSIVE").unwrap();
            drop(lane);
            assert!(matches!(action.await.unwrap(), Err(StoreError::Busy)));
            db.execute_batch("ROLLBACK").unwrap();
        }
        pending(&organization(&state), false).await;
        assert!(store.get(ID.into()).await.unwrap().is_some());
        assert!(store
            .pending_deletions(None, 100)
            .await
            .unwrap()
            .jobs
            .is_empty());
        assert_eq!(
            std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap(),
            legacy
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_delete_unreadable_or_missing_sql_never_signals_stop() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for fault in ["missing", "corrupt"] {
        let (_dir, state, store, writes, killed, _commands) = fixture().await;
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let db = rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
        if fault == "missing" {
            db.execute("DELETE FROM sessions WHERE id=?1", [ID])
                .unwrap();
        } else {
            db.execute("UPDATE sessions SET metadata='[]' WHERE id=?1", [ID])
                .unwrap();
        }
        assert!(view.begin_delete().await.is_err());
        assert!(!*killed.borrow());
        assert!(store
            .pending_deletions(None, 100)
            .await
            .unwrap()
            .jobs
            .is_empty());
        store.close().await.unwrap();
    }
}
