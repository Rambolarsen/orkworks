use super::organization_tests::{finalize, fixture, root, CREATED, ID};
use super::*;

pub(super) async fn deleted() -> (
    tempfile::TempDir,
    Arc<AppState>,
    Arc<SessionStore>,
    Arc<SessionWrites>,
) {
    let (dir, state, store, writes, _killed, commands) = fixture().await;
    std::fs::create_dir_all(root(&state).join("events")).unwrap();
    for suffix in ["ndjson", "terminal", "terminal-size"] {
        std::fs::write(
            root(&state).join(format!("events/{ID}.{suffix}")),
            b"payload",
        )
        .unwrap();
    }
    finalize(&state, &writes).await;
    drop(commands);
    writes
        .capture_organization(ID.into())
        .await
        .unwrap()
        .begin_delete()
        .await
        .unwrap();
    (dir, state, store, writes)
}

#[tokio::test]
async fn sql_replay_cleanup_removes_only_exact_payloads_and_keeps_job_across_reopen() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let legacy = std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap();
    let other = root(&state).join("events/other.terminal");
    std::fs::write(&other, b"other").unwrap();
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let receipt = view.remove_replay().await.unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(1))));
    assert!(!receipt.detached);
    for suffix in ["ndjson", "terminal", "terminal-size"] {
        assert!(!root(&state).join(format!("events/{ID}.{suffix}")).exists());
    }
    assert_eq!(std::fs::read(&other).unwrap(), b"other");
    assert_eq!(
        std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap(),
        legacy
    );
    assert_eq!(
        store
            .deletion_record(ID.into(), CREATED.into())
            .await
            .unwrap()
            .unwrap()
            .job
            .progress,
        1
    );
    assert!(store.get(ID.into()).await.unwrap().is_none());
    let (path, workspace) = store.location();
    let (path, workspace) = (path.to_owned(), workspace.to_owned());
    store.close().await.unwrap();
    let reopened = Arc::new(SessionStore::open(path, workspace).await.unwrap());
    let writes = SessionWrites::bind(state, reopened.clone()).unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_replay()
        .await
        .unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(1))));
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
async fn sql_replay_cleanup_rejects_ordinary_rows_and_wrong_creation_before_files() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, _commands) = fixture().await;
    assert!(writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .is_err());
    store.close().await.unwrap();
    drop(state);
    let (_dir, state, store, writes) = deleted().await;
    assert!(writes
        .capture_deletion(ID.into(), "replacement".into())
        .await
        .is_err());
    assert!(root(&state).join(format!("events/{ID}.terminal")).exists());
    store.close().await.unwrap();
}

use std::time::Duration;
pub(super) struct Held {
    entered: Option<oneshot::Receiver<()>>,
    release: Option<std::sync::mpsc::Sender<()>>,
}
impl Held {
    pub(super) async fn entered(&mut self) {
        tokio::time::timeout(Duration::from_secs(5), self.entered.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }
}
impl Drop for Held {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}
pub(super) fn hold(gate: &ReplayGate) -> Held {
    let (ready, entered) = oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    *gate.lock().unwrap() = Some((ready, wait));
    Held {
        entered: Some(entered),
        release: Some(release),
    }
}
fn replay_exists(state: &Arc<AppState>) -> bool {
    root(state).join(format!("events/{ID}.terminal")).exists()
}
pub(super) async fn progress(store: &Arc<SessionStore>) -> u8 {
    store
        .deletion_record(ID.into(), CREATED.into())
        .await
        .unwrap()
        .unwrap()
        .job
        .progress
}

#[tokio::test]
async fn sql_replay_cleanup_stale_views_and_physical_reopening_never_remove_files() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "lease",
        "reader",
        "organization",
        "runtime",
        "control",
        "pid",
        "terminal",
    ] {
        let (_dir, state, store, writes) = deleted().await;
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        match change {
            "lease" => state.workspace.lock().unwrap().as_mut().unwrap().lease = None,
            "reader" => {
                let path = root(&state);
                state.workspace.lock().unwrap().as_mut().unwrap().metadata =
                    metadata::MetadataStore::new(&path);
            }
            "organization" => {
                let path = root(&state);
                state
                    .workspace
                    .lock()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .metadata
                    .organization = Arc::new(crate::session_archive::OrganizationStore::new(&path));
            }
            "runtime" => {
                state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
                    crate::runtime::session_runtime::SessionRuntime::live(24, 80).0
            }
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
            "pid" => {
                state.session_pids.lock().unwrap().insert(ID.into(), 123);
            }
            "terminal" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .terminal_attached = true
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(view.remove_replay().await, Err(StoreError::Conflict)),
            "{change}"
        );
        assert!(replay_exists(&state));
        assert_eq!(progress(&store).await, 0);
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_replay_cleanup_rechecks_job_after_lane_wait_and_never_adopts_reused_identity() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let lane = store.runtime_lane(ID).unwrap().lock_owned().await;
    let action = tokio::spawn(view.remove_replay());
    store
        .deletion_progress(ID.into(), CREATED.into(), 0, true)
        .await
        .unwrap();
    let mut replacement = crate::test_support::test_session_metadata(
        ID,
        "Replacement",
        "/workspace",
        "ended",
        "replacement",
        "replacement",
    );
    replacement.lifecycle = "dead".into();
    replacement.lifecycle_phase = "ended".into();
    store
        .create(serde_json::to_value(replacement).unwrap())
        .await
        .unwrap();
    drop(lane);
    assert!(action.await.unwrap().is_err());
    assert!(replay_exists(&state));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["createdAt"],
        "replacement"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_replay_cleanup_cancellation_expiry_and_reservation_conflict_have_no_effect() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for mode in ["cancel", "expire", "reservation"] {
        let (_dir, state, store, writes) = deleted().await;
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        if mode == "reservation" {
            let owner = super::organization_tests::organization(&state);
            let _held = owner.reserve(ID).unwrap();
            assert!(matches!(
                view.remove_replay().await,
                Err(StoreError::Conflict)
            ));
        } else {
            let mut gate = hold(&writes.replay_before);
            let action = tokio::spawn(view.remove_replay());
            gate.entered().await;
            if mode == "cancel" {
                action.abort();
                assert!(action.await.unwrap_err().is_cancelled());
            } else {
                assert!(matches!(
                    tokio::time::timeout(Duration::from_secs(3), action)
                        .await
                        .unwrap()
                        .unwrap(),
                    Err(StoreError::Expired)
                ));
            }
            drop(gate);
        }
        // A subsequent lane acquisition is a barrier for retained rejected work.
        let _lane = tokio::time::timeout(
            Duration::from_secs(3),
            store.runtime_lane(ID).unwrap().lock_owned(),
        )
        .await
        .unwrap();
        assert!(replay_exists(&state));
        assert_eq!(progress(&store).await, 0);
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_replay_cleanup_partial_file_error_never_records_progress_and_retry_preserves_other_bits(
) {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let terminal = root(&state).join(format!("events/{ID}.terminal"));
    std::fs::remove_file(&terminal).unwrap();
    std::fs::create_dir(&terminal).unwrap();
    let result = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_replay()
        .await
        .unwrap();
    assert!(result.files.is_err());
    assert!(result.progress.is_none());
    assert_eq!(progress(&store).await, 0);
    assert!(!root(&state).join(format!("events/{ID}.ndjson")).exists());
    std::fs::remove_dir(&terminal).unwrap();
    store
        .deletion_progress(ID.into(), CREATED.into(), 4, false)
        .await
        .unwrap();
    let result = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_replay()
        .await
        .unwrap();
    assert!(result.files.is_ok());
    assert!(matches!(result.progress, Some(Ok(5))));
    assert_eq!(
        store.pending_deletions(None, 100).await.unwrap().jobs.len(),
        1
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_replay_cleanup_progress_failure_reports_durable_files_and_retries_pending_job() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.remove_replay());
    gate.entered().await;
    assert!(!replay_exists(&state));
    let db = rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    drop(gate);
    let result = action.await.unwrap().unwrap();
    assert!(result.files.is_ok());
    assert!(matches!(result.progress, Some(Err(StoreError::Busy))));
    db.execute_batch("ROLLBACK").unwrap();
    assert_eq!(progress(&store).await, 0);
    let result = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_replay()
        .await
        .unwrap();
    assert!(matches!(result.progress, Some(Ok(1))));
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_replay_cleanup_admitted_cancel_retains_original_resources_and_detached_completion() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (_dir, state, store, writes) = deleted().await;
        let original = super::organization_tests::organization(&state);
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
        let mut gate = hold(&writes.replay_admitted);
        let mut action = tokio::spawn(view.remove_replay());
        gate.entered().await;
        state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
            crate::runtime::session_runtime::SessionRuntime::live(24, 80).0;
        state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
        if cancel {
            action.abort();
            assert!((&mut action).await.unwrap_err().is_cancelled());
        } else {
            assert!(
                tokio::time::timeout(Duration::from_millis(2100), &mut action)
                    .await
                    .is_err(),
                "admitted cleanup returned expiry before definitive completion"
            );
        }
        assert!(weak.upgrade().is_some());
        assert!(original.pending(ID));
        assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_err());
        drop(gate);
        if !cancel {
            let result = action.await.unwrap().unwrap();
            assert!(result.detached);
            assert!(matches!(result.progress, Some(Ok(1))));
        }
        super::organization_tests::pending(&original, false).await;
        tokio::time::timeout(Duration::from_secs(3), async {
            while weak.upgrade().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(progress(&store).await, 1);
        assert!(!replay_exists(&state));
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_replay_cleanup_corrupt_finalization_and_unavailable_sql_preserve_files() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for fault in ["lifecycle", "shape", "closed"] {
        let (_dir, state, store, writes) = deleted().await;
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        if fault == "closed" {
            store.close().await.unwrap();
        } else {
            let db =
                rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
            if fault == "shape" {
                db.execute("UPDATE sessions SET metadata='[]' WHERE id=?1", [ID])
                    .unwrap();
            } else {
                db.execute("UPDATE sessions SET metadata=json_set(metadata,'$.lifecycle','alive') WHERE id=?1",[ID]).unwrap();
            }
        }
        assert!(view.remove_replay().await.is_err());
        assert!(replay_exists(&state));
        store.close().await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn sql_replay_cleanup_stall_leaves_guards_and_independent_input_available() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes) = deleted().await;
    let (input_id, mut commands) = super::organization_tests::input_runtime(&state, dir.path());
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_admitted);
    let (ready, entered) = oneshot::channel();
    let (progress, done) = oneshot::channel();
    let watchdog = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(async {
            gate.entered().await;
            let _ = ready.send(());
            tokio::time::timeout(Duration::from_secs(2), done)
                .await
                .is_ok_and(|r| r.is_ok())
        });
        drop(gate);
        result
    });
    let action = tokio::spawn(view.remove_replay());
    entered.await.unwrap();
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
                assert_eq!(data, "cleanup-input\r");
                let _ = accepted.send(Ok(()));
            }
            _ => panic!("expected acknowledged input"),
        }
    });
    let delivered = tokio::time::timeout(
        Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            input_id,
            "cleanup-input\r".into(),
        ),
    )
    .await;
    let driven = tokio::time::timeout(Duration::from_secs(1), driver).await;
    let _ = progress.send(());
    assert!(
        watchdog.join().unwrap(),
        "cleanup blocked sole Tokio worker until OS watchdog release"
    );
    driven.unwrap().unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    let result = action.await.unwrap().unwrap();
    assert!(matches!(result.progress, Some(Ok(1))));
    store.close().await.unwrap();
}

#[test]
fn sql_replay_cleanup_finishes_with_one_blocking_worker() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
        let (_dir, _state, store, writes) = deleted().await;
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), view.remove_replay()).await;
        store.close().await.unwrap();
        result
    });
    runtime.shutdown_timeout(Duration::from_secs(2));
    assert!(result.is_ok_and(|r| r.is_ok_and(|receipt| matches!(receipt.progress, Some(Ok(1))))));
}

#[tokio::test]
async fn sql_replay_cleanup_maximum_identity_records_progress_without_optional_payloads() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let id = "x".repeat(256);
    let mut metadata = crate::test_support::test_session_metadata(
        &id,
        "Long identity",
        "/workspace",
        "ended",
        CREATED,
        CREATED,
    );
    metadata.lifecycle = "dead".into();
    metadata.lifecycle_phase = "ended".into();
    store
        .create(serde_json::to_value(metadata).unwrap())
        .await
        .unwrap();
    store
        .begin_delete(id.clone(), CREATED.into(), "1".into(), "0".into())
        .await
        .unwrap();
    let result = writes
        .capture_deletion(id.clone(), CREATED.into())
        .await
        .unwrap()
        .remove_replay()
        .await
        .unwrap();
    assert!(result.files.is_ok());
    assert!(matches!(result.progress, Some(Ok(1))));
    assert_eq!(
        store
            .deletion_record(id, CREATED.into())
            .await
            .unwrap()
            .unwrap()
            .job
            .progress,
        1
    );
    assert!(replay_exists(&state));
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_replay_directory_replacement_never_removes_replacement_payloads() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for target in ["events", "root"] {
        for admitted in [false, true] {
            let (dir, state, store, writes) = deleted().await;
            let path = root(&state);
            let mut view = Some(
                writes
                    .capture_deletion(ID.into(), CREATED.into())
                    .await
                    .unwrap(),
            );
            let mut gate = hold(&writes.replay_admitted);
            let action = if admitted {
                let action = tokio::spawn(view.take().unwrap().remove_replay());
                gate.entered().await;
                Some(action)
            } else {
                None
            };
            // The not-yet-started case keeps its view separately below.
            let original = dir.path().join("original-replay-root");
            if target == "events" {
                std::fs::rename(path.join("events"), &original).unwrap();
            } else {
                std::fs::rename(&path, &original).unwrap();
            }
            std::fs::create_dir_all(path.join("events")).unwrap();
            for suffix in ["ndjson", "terminal", "terminal-size"] {
                std::fs::write(path.join(format!("events/{ID}.{suffix}")), b"replacement").unwrap();
            }
            drop(gate);
            let receipt = if let Some(action) = action {
                action.await.unwrap().unwrap()
            } else {
                view.take().unwrap().remove_replay().await.unwrap()
            };
            assert!(receipt.files.is_ok());
            for suffix in ["ndjson", "terminal", "terminal-size"] {
                assert_eq!(
                    std::fs::read(path.join(format!("events/{ID}.{suffix}"))).unwrap(),
                    b"replacement"
                );
                let old = if target == "events" {
                    original.join(format!("{ID}.{suffix}"))
                } else {
                    original.join(format!("events/{ID}.{suffix}"))
                };
                assert!(!old.exists());
            }
            store.close().await.unwrap();
        }
    }
}

#[tokio::test]
async fn sql_replay_capture_never_adopts_root_replaced_before_binding() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for lock_entry in ["missing", "copied", "symlink"] {
        let (dir, state, store, writes) = deleted().await;
        let path = root(&state);
        let original = dir.path().join("original-root");
        std::fs::rename(&path, &original).unwrap();
        std::fs::create_dir_all(path.join("events")).unwrap();
        match lock_entry {
            "copied" => std::fs::write(path.join(".sidecar.lock"), b"replacement lock").unwrap(),
            "symlink" => std::os::unix::fs::symlink(
                original.join(".sidecar.lock"),
                path.join(".sidecar.lock"),
            )
            .unwrap(),
            _ => {}
        }
        let replacement = path.join(format!("events/{ID}.terminal"));
        std::fs::write(&replacement, b"replacement").unwrap();
        let receipt = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap()
            .remove_replay()
            .await
            .unwrap();
        assert!(receipt.files.is_err());
        assert!(receipt.progress.is_none());
        assert_eq!(std::fs::read(replacement).unwrap(), b"replacement");
        assert!(original.join(format!("events/{ID}.terminal")).exists());
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_replay_capture_absence_does_not_adopt_later_events_directory() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    std::fs::remove_dir_all(root(&state).join("events")).unwrap();
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    std::fs::create_dir(root(&state).join("events")).unwrap();
    let payload = root(&state).join(format!("events/{ID}.terminal"));
    std::fs::write(&payload, b"later").unwrap();
    let receipt = view.remove_replay().await.unwrap();
    assert_eq!(
        receipt.files.unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert!(receipt.progress.is_none());
    assert_eq!(progress(&store).await, 0);
    assert_eq!(std::fs::read(&payload).unwrap(), b"later");
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_replay()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(1))));
    assert!(!payload.exists());
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_replay_capture_capacity_is_bounded_and_transfers_without_double_reservation() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, _state, store, writes) = deleted().await;
    let mut views = Vec::new();
    for _ in 0..32 {
        views.push(
            writes
                .capture_deletion(ID.into(), CREATED.into())
                .await
                .unwrap(),
        );
    }
    assert!(matches!(
        writes.capture_deletion(ID.into(), CREATED.into()).await,
        Err(StoreError::QueueFull)
    ));
    let receipt = views.pop().unwrap().remove_replay().await.unwrap();
    assert!(matches!(receipt.progress, Some(Ok(1))));
    views.push(
        writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap(),
    );
    assert!(matches!(
        writes.capture_deletion(ID.into(), CREATED.into()).await,
        Err(StoreError::QueueFull)
    ));
    drop(views);
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    drop(view);
    store.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn sql_replay_capture_stall_cancel_and_expiry_retain_budget_until_drain_without_guards() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (dir, state, store, writes) = deleted().await;
        let (input_id, mut commands) = super::organization_tests::input_runtime(&state, dir.path());
        let mut gate = hold(&writes.replay_binding);
        let captured = writes.clone();
        let action =
            tokio::spawn(async move { captured.capture_deletion(ID.into(), CREATED.into()).await });
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
                    assert_eq!(data, "binding-input\r");
                    let _ = accepted.send(Ok(()));
                }
                _ => panic!("expected acknowledged input"),
            }
        });
        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                crate::runtime::terminal_runtime::submit_approved_input(
                    &state,
                    input_id,
                    "binding-input\r".into()
                )
            )
            .await
            .unwrap(),
            Ok(())
        );
        driver.await.unwrap();
        if cancel {
            action.abort();
            assert!(matches!(action.await,Err(e) if e.is_cancelled()));
        } else {
            assert!(matches!(
                tokio::time::timeout(Duration::from_secs(3), action)
                    .await
                    .unwrap()
                    .unwrap(),
                Err(StoreError::Expired)
            ));
        }
        let mut held = Vec::new();
        for _ in 0..31 {
            held.push(
                writes
                    .capture_deletion(ID.into(), CREATED.into())
                    .await
                    .unwrap(),
            );
        }
        assert!(matches!(
            writes.capture_deletion(ID.into(), CREATED.into()).await,
            Err(StoreError::QueueFull)
        ));
        assert!(replay_exists(&state));
        assert_eq!(progress(&store).await, 0);
        drop(gate);
        let drained = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match writes.capture_deletion(ID.into(), CREATED.into()).await {
                    Ok(view) => break view,
                    Err(StoreError::QueueFull) => tokio::task::yield_now().await,
                    Err(e) => panic!("unexpected capture failure: {e:?}"),
                }
            }
        })
        .await
        .unwrap();
        drop(drained);
        drop(held);
        store.close().await.unwrap();
    }
}
