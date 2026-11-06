use super::organization_tests::{root, CREATED};
use super::*;
const IMAGE_ID: &str = "11111111-1111-4111-8111-111111111111";
const OTHER_ID: &str = "22222222-2222-4222-8222-222222222222";
fn try_publish(
    path: &std::path::Path,
    id: &str,
) -> std::io::Result<crate::image_context::ImageBundle> {
    use base64::Engine;
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(1, 1)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let inputs = vec![crate::image_context::ImageInput {
        name: "fixture.png".into(),
        mime_type: "image/png".into(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes.into_inner()),
    }];
    crate::image_context::ImageBundle::publish(
        path,
        id,
        crate::image_context::validate(&inputs, "inspect").unwrap(),
    )
}
fn publish(path: &std::path::Path, id: &str) -> crate::image_context::ImageBundle {
    try_publish(path, id).unwrap()
}
async fn fixture() -> (
    tempfile::TempDir,
    Arc<AppState>,
    Arc<SessionStore>,
    Arc<SessionWrites>,
) {
    let (dir, state, store, writes, _killed, _commands) =
        super::organization_tests::fixture().await;
    let mut metadata = crate::test_support::test_session_metadata(
        IMAGE_ID,
        "Images",
        "/workspace",
        "ended",
        CREATED,
        CREATED,
    );
    metadata.lifecycle = "dead".into();
    metadata.lifecycle_phase = "ended".into();
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .try_write_session(&metadata)
        .unwrap();
    store
        .create(serde_json::to_value(metadata).unwrap())
        .await
        .unwrap();
    writes
        .capture_organization(IMAGE_ID.into())
        .await
        .unwrap()
        .begin_delete()
        .await
        .unwrap();
    let mut bundle = publish(&root(&state), IMAGE_ID);
    bundle.accept();
    drop(bundle);
    (dir, state, store, writes)
}
fn path(state: &Arc<AppState>) -> std::path::PathBuf {
    root(state).join("session-images").join(IMAGE_ID)
}
#[tokio::test]
async fn sql_image_cleanup_removes_exact_bundle_and_keeps_durable_job() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = fixture().await;
    let mut other = publish(&root(&state), OTHER_ID);
    let other_path = other.paths()[0].clone();
    other.accept();
    drop(other);
    let legacy = std::fs::read(root(&state).join(format!("sessions/{IMAGE_ID}.json"))).unwrap();
    let receipt = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(4))));
    assert!(!receipt.detached);
    assert!(!path(&state).exists());
    assert!(other_path.exists());
    assert_eq!(
        std::fs::read(root(&state).join(format!("sessions/{IMAGE_ID}.json"))).unwrap(),
        legacy
    );
    assert!(store.get(IMAGE_ID.into()).await.unwrap().is_none());
    let (db, key) = store.location();
    let (db, key) = (db.to_owned(), key.to_owned());
    store.close().await.unwrap();
    let reopened = Arc::new(SessionStore::open(db, key).await.unwrap());
    assert_eq!(
        reopened
            .deletion_record(IMAGE_ID.into(), CREATED.into())
            .await
            .unwrap()
            .unwrap()
            .job
            .progress,
        4
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
    let writes = SessionWrites::bind(state, reopened.clone()).unwrap();
    let receipt = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(4))));
    reopened.close().await.unwrap();
}

use super::delete_replay_tests::hold;
async fn progress(store: &Arc<SessionStore>) -> u8 {
    store
        .deletion_record(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .unwrap()
        .job
        .progress
}
#[tokio::test]
async fn sql_image_cleanup_busy_publisher_is_retryable_without_losing_files() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = fixture().await;
    let publisher = publish(&root(&state), OTHER_ID);
    let receipt = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert_eq!(
        receipt.files.err().unwrap().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(receipt.progress.is_none());
    assert_eq!(progress(&store).await, 0);
    assert!(path(&state).join("0.png").exists());
    drop(publisher);
    let receipt = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(4))));
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_image_cleanup_retains_lock_through_progress_failure_then_retries() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = fixture().await;
    let view = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.remove_images());
    gate.entered().await;
    assert!(!path(&state).exists());
    assert_eq!(
        try_publish(&root(&state), OTHER_ID).err().unwrap().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let db = rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    drop(gate);
    let receipt = action.await.unwrap().unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Err(StoreError::Busy))));
    db.execute_batch("ROLLBACK").unwrap();
    assert_eq!(progress(&store).await, 0);
    drop(try_publish(&root(&state), OTHER_ID).unwrap());
    let receipt = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(4))));
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_image_cleanup_partial_error_preserves_marker_and_other_progress_bits() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = fixture().await;
    store
        .deletion_progress(IMAGE_ID.into(), CREATED.into(), 3, false)
        .await
        .unwrap();
    std::fs::create_dir(path(&state).join("obstruction")).unwrap();
    let receipt = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert!(receipt.files.is_err());
    assert!(receipt.progress.is_none());
    assert_eq!(progress(&store).await, 3);
    assert!(path(&state).join("owner.json").exists());
    std::fs::remove_dir(path(&state).join("obstruction")).unwrap();
    let receipt = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(7))));
    assert_eq!(
        store.pending_deletions(None, 100).await.unwrap().jobs.len(),
        1
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_image_cleanup_cancellation_before_admission_never_creates_lock_or_removes_payloads() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (_dir, state, store, writes) = fixture().await;
        let lock = root(&state).join("session-images/.lock");
        std::fs::remove_file(&lock).unwrap();
        let view = writes
            .capture_deletion(IMAGE_ID.into(), CREATED.into())
            .await
            .unwrap();
        assert!(!lock.exists());
        let mut gate = hold(&writes.replay_before);
        let action = tokio::spawn(view.remove_images());
        gate.entered().await;
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
            store.runtime_lane(IMAGE_ID).unwrap().lock_owned(),
        )
        .await
        .unwrap();
        assert!(!lock.exists());
        assert!(path(&state).join("0.png").exists());
        assert_eq!(progress(&store).await, 0);
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn sql_image_cleanup_admitted_caller_loss_retains_original_lease_and_lock() {
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
            .capture_deletion(IMAGE_ID.into(), CREATED.into())
            .await
            .unwrap();
        let mut gate = hold(&writes.replay_removed);
        let action = tokio::spawn(view.remove_images());
        gate.entered().await;
        state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
        assert!(weak.upgrade().is_some());
        assert!(owner.pending(IMAGE_ID));
        assert_eq!(
            try_publish(&root(&state), OTHER_ID).err().unwrap().kind(),
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
            assert!(matches!(receipt.progress, Some(Ok(4))));
        }
        super::organization_tests::pending(&owner, false).await;
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while weak.upgrade().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(progress(&store).await, 4);
        assert!(!path(&state).exists());
        drop(try_publish(&root(&state), OTHER_ID).unwrap());
        store.close().await.unwrap();
    }
}
#[tokio::test(flavor = "current_thread")]
async fn sql_image_cleanup_stall_leaves_guards_and_acknowledged_input_free() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes) = fixture().await;
    let (input_id, mut commands) = super::organization_tests::input_runtime(&state, dir.path());
    let view = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.remove_images());
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
                assert_eq!(data, "image-input\r");
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
            "image-input\r".into(),
        ),
    )
    .await;
    drop(gate);
    driver.await.unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(matches!(
        action.await.unwrap().unwrap().progress,
        Some(Ok(4))
    ));
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_image_cleanup_non_uuid_identity_never_touches_unowned_paths() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = super::delete_replay_tests::deleted().await;
    let id = super::organization_tests::ID;
    let unowned = root(&state).join("session-images").join(id);
    std::fs::create_dir_all(&unowned).unwrap();
    std::fs::write(unowned.join("keep"), b"keep").unwrap();
    let receipt = writes
        .capture_deletion(id.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(4))));
    assert_eq!(std::fs::read(unowned.join("keep")).unwrap(), b"keep");
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_image_cleanup_binding_failure_does_not_block_other_stages() {
    use std::os::unix::fs::PermissionsExt;
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = fixture().await;
    let parent = root(&state).join("session-images");
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    let receipt = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert!(receipt.files.is_err());
    assert!(receipt.progress.is_none());
    assert_eq!(progress(&store).await, 0);
    let replay = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_replay()
        .await
        .unwrap();
    assert!(matches!(replay.progress, Some(Ok(1))));
    let workflow = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_workflow_evidence()
        .await
        .unwrap();
    assert!(matches!(workflow.progress, Some(Ok(3))));
    assert!(path(&state).join("0.png").exists());
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    let images = writes
        .capture_deletion(IMAGE_ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_images()
        .await
        .unwrap();
    assert!(matches!(images.progress, Some(Ok(7))));
    store.close().await.unwrap();
}
#[test]
fn sql_image_cleanup_finishes_with_one_blocking_worker() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
        let (_dir, _state, store, writes) = fixture().await;
        let view = writes
            .capture_deletion(IMAGE_ID.into(), CREATED.into())
            .await
            .unwrap();
        let receipt =
            tokio::time::timeout(std::time::Duration::from_secs(5), view.remove_images()).await;
        store.close().await.unwrap();
        receipt
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
    assert!(result.is_ok_and(|r| r.is_ok_and(|receipt| matches!(receipt.progress, Some(Ok(4))))));
}
