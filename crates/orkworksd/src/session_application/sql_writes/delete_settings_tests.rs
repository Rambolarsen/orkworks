use super::organization_tests::{root, CREATED, ID};
use super::*;

#[tokio::test]
async fn sql_settings_cleanup_clears_exact_selection_and_keeps_job_across_reopen() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = super::delete_replay_tests::deleted().await;
    let path = root(&state).join("workspace.json");
    let before = format!(
        r#"{{"lastActiveSessionId":"{ID}","lastActiveAt":"before","activeHarnessIds":["codex"],"activeHarnessRevision":7,"extension":{{"number":123456789012345678901234567890,"raw":"\u0061"}}}}"#
    );
    std::fs::write(&path, &before).unwrap();
    let legacy = std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .clear_settings()
        .await
        .unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(8))));
    assert!(!receipt.detached);
    let after = std::fs::read_to_string(&path).unwrap();
    let saved: crate::metadata::WorkspaceMemory = serde_json::from_str(&after).unwrap();
    assert_eq!(saved.last_active_session_id, None);
    assert_eq!(saved.last_active_at, None);
    assert_eq!(saved.active_harness_ids, vec!["codex"]);
    assert_eq!(saved.active_harness_revision, 7);
    assert!(after.contains(r#"{"number":123456789012345678901234567890,"raw":"\u0061"}"#));
    assert_eq!(
        std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap(),
        legacy
    );
    assert!(root(&state).join(format!("events/{ID}.terminal")).exists());
    assert!(store.get(ID.into()).await.unwrap().is_none());
    let (db, key) = store.location();
    let (db, key) = (db.to_owned(), key.to_owned());
    store.close().await.unwrap();
    let reopened = Arc::new(SessionStore::open(db, key).await.unwrap());
    assert_eq!(
        reopened
            .deletion_record(ID.into(), CREATED.into())
            .await
            .unwrap()
            .unwrap()
            .job
            .progress,
        8
    );
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

use super::delete_replay_tests::hold;
fn path(state: &Arc<AppState>) -> std::path::PathBuf {
    root(state).join("workspace.json")
}
async fn fixture() -> (
    tempfile::TempDir,
    Arc<AppState>,
    Arc<SessionStore>,
    Arc<SessionWrites>,
) {
    let result = super::delete_replay_tests::deleted().await;
    std::fs::write(path(&result.1),format!(r#"{{"lastActiveSessionId":"{ID}","lastActiveAt":"before","activeHarnessIds":["codex"],"activeHarnessRevision":7}}"#)).unwrap();
    result
}
async fn progress(store: &Arc<SessionStore>) -> u8 {
    store
        .deletion_record(ID.into(), CREATED.into())
        .await
        .unwrap()
        .unwrap()
        .job
        .progress
}
#[tokio::test]
async fn sql_settings_cleanup_busy_owner_does_not_block_capture_or_erase_selection() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = fixture().await;
    let before = std::fs::read(path(&state)).unwrap();
    let edit = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .edit_workspace_memory()
        .unwrap();
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let receipt = view.clear_settings().await.unwrap();
    assert_eq!(
        receipt.files.err().unwrap().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(receipt.progress.is_none());
    assert_eq!(progress(&store).await, 0);
    assert_eq!(std::fs::read(path(&state)).unwrap(), before);
    drop(edit);
    assert!(matches!(
        writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap()
            .clear_settings()
            .await
            .unwrap()
            .progress,
        Some(Ok(8))
    ));
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_settings_cleanup_retains_permit_through_sql_failure_then_preserves_new_selection() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = fixture().await;
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.clear_settings());
    gate.entered().await;
    let app = crate::session_application::SessionApplication::new(state.clone());
    assert!(matches!(
        app.set_active_session("other"),
        Err(crate::session_application::SessionError::Conflict)
    ));
    assert!(matches!(
        app.set_active_harnesses_at(vec![], 7),
        Err(crate::session_application::SessionError::Conflict)
    ));
    let db = rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    drop(gate);
    let receipt = action.await.unwrap().unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Err(StoreError::Busy))));
    db.execute_batch("ROLLBACK").unwrap();
    assert_eq!(progress(&store).await, 0);
    app.set_active_session("other").unwrap();
    let before = std::fs::read(path(&state)).unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .clear_settings()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(8))));
    assert_eq!(std::fs::read(path(&state)).unwrap(), before);
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_settings_cleanup_invalid_memory_preserves_other_stages_and_existing_progress() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = fixture().await;
    std::fs::write(path(&state), b"not json").unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .clear_settings()
        .await
        .unwrap();
    assert!(receipt.files.is_err());
    assert!(receipt.progress.is_none());
    assert_eq!(progress(&store).await, 0);
    assert!(matches!(
        writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap()
            .remove_replay()
            .await
            .unwrap()
            .progress,
        Some(Ok(1))
    ));
    assert!(matches!(
        writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap()
            .remove_workflow_evidence()
            .await
            .unwrap()
            .progress,
        Some(Ok(3))
    ));
    assert!(matches!(
        writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap()
            .remove_images()
            .await
            .unwrap()
            .progress,
        Some(Ok(7))
    ));
    std::fs::remove_file(path(&state)).unwrap();
    assert!(matches!(
        writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap()
            .clear_settings()
            .await
            .unwrap()
            .progress,
        Some(Ok(15))
    ));
    assert!(!path(&state).exists());
    assert_eq!(
        store.pending_deletions(None, 100).await.unwrap().jobs.len(),
        1
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_settings_cleanup_before_admission_cancellation_and_expiry_leave_settings_untouched() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (_dir, state, store, writes) = fixture().await;
        let before = std::fs::read(path(&state)).unwrap();
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let mut gate = hold(&writes.replay_before);
        let action = tokio::spawn(view.clear_settings());
        gate.entered().await;
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .edit_workspace_memory()
            .is_ok());
        if cancel {
            action.abort();
            assert!(action.await.unwrap_err().is_cancelled());
        } else {
            assert!(matches!(
                tokio::time::timeout(std::time::Duration::from_secs(3), action)
                    .await
                    .unwrap()
                    .unwrap(),
                Err(StoreError::Expired)
            ));
        }
        drop(gate);
        let _lane = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            store.runtime_lane(ID).unwrap().lock_owned(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(path(&state)).unwrap(), before);
        assert_eq!(progress(&store).await, 0);
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn sql_settings_cleanup_admitted_caller_loss_retains_original_lease_and_edit_until_receipt() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (_dir, state, store, writes) = fixture().await;
        let owner = super::organization_tests::organization(&state);
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
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let mut gate = hold(&writes.replay_removed);
        let action = tokio::spawn(view.clear_settings());
        gate.entered().await;
        state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
        assert!(weak.upgrade().is_some());
        assert!(owner.pending(ID));
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .edit_workspace_memory()
                .err()
                .unwrap()
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
        if cancel {
            action.abort();
            assert!(action.await.unwrap_err().is_cancelled());
            drop(gate);
        } else {
            drop(gate);
            let receipt = action.await.unwrap().unwrap();
            assert!(receipt.detached);
            assert!(matches!(receipt.progress, Some(Ok(8))));
        }
        super::organization_tests::pending(&owner, false).await;
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while weak.upgrade().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(progress(&store).await, 8);
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_workspace_memory()
                .unwrap()
                .last_active_session_id,
            None
        );
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .edit_workspace_memory()
            .is_ok());
        store.close().await.unwrap();
    }
}
#[tokio::test(flavor = "current_thread")]
async fn sql_settings_cleanup_stall_keeps_guards_and_acknowledged_input_free() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes) = fixture().await;
    let (input_id, mut commands) = super::organization_tests::input_runtime(&state, dir.path());
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.clear_settings());
    gate.entered().await;
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(state.session_pids.try_lock().is_ok());
    let driver = tokio::spawn(async move {
        match commands.recv().await.unwrap() {
            crate::runtime::session_runtime::RuntimeCommand::Input {
                data,
                accepted: Some(accepted),
            } => {
                assert_eq!(data, "settings-input\r");
                let _ = accepted.send(Ok(()));
            }
            _ => panic!("expected acknowledged input"),
        }
    });
    let delivered = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            input_id,
            "settings-input\r".into(),
        ),
    )
    .await;
    drop(gate);
    driver.await.unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(matches!(
        action.await.unwrap().unwrap().progress,
        Some(Ok(8))
    ));
    store.close().await.unwrap();
}
#[test]
fn sql_settings_cleanup_completes_with_one_blocking_worker() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
        let (_dir, _state, store, writes) = fixture().await;
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let receipt =
            tokio::time::timeout(std::time::Duration::from_secs(5), view.clear_settings()).await;
        store.close().await.unwrap();
        receipt
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
    assert!(result.is_ok_and(|r| r.is_ok_and(|receipt| matches!(receipt.progress, Some(Ok(8))))));
}
