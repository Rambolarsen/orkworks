use super::*;

#[tokio::test]
async fn sql_cleanup_revokes_stale_legacy_writers_and_independent_opens() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = seed(&state);
    let stale = files.get("a").unwrap().unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(16))));
    assert!(
        files.put(&stale).is_err(),
        "stale publication must not recreate removed recommendation"
    );
    assert!(!root(&state).join("recommendations/a.json").exists());
    assert!(
        files.list().is_err(),
        "legacy reads can recover journals and require authority too"
    );
    assert!(RecommendationStore::open(root(&state)).is_err());
    let retry = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(retry.files.is_ok());
    assert!(matches!(retry.progress, Some(Ok(16))));
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_cleanup_refuses_inflight_delivery_without_revoking_or_removing_files() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = seed(&state);
    let workspace = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path
        .clone();
    let pending = crate::session_application::RecommendationDeliveryGuard::new(
        &workspace,
        "unrelated-delivery",
    );
    let before = std::fs::read(root(&state).join("recommendations/a.json")).unwrap();
    let result = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await;
    assert!(matches!(result, Err(StoreError::Conflict)), "{result:?}");
    assert_eq!(
        std::fs::read(root(&state).join("recommendations/a.json")).unwrap(),
        before
    );
    assert_eq!(files.list().unwrap().len(), 3);
    assert_eq!(progress(&store).await, 0);
    drop(pending);
    let result = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(result.files.is_ok());
    assert!(matches!(result.progress, Some(Ok(16))));
    store.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn sql_cleanup_waits_for_successful_and_failed_delivery_settlement_without_resend() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for delivered in [false, true] {
        let (dir, state, store, writes) = deleted().await;
        let files = seed(&state);
        files.put(&recommendation("delivery", "unrelated")).unwrap();
        let (target, mut commands) = delivery_runtime(&state, dir.path());
        let sent_state = state.clone();
        let mut delivery = tokio::spawn(async move {
            crate::session_application::SessionApplication::new(sent_state)
                .accept_recommendation_with_packet("delivery", target, None, None)
                .await
        });
        let crate::runtime::session_runtime::RuntimeCommand::Input {
            accepted: Some(accepted),
            ..
        } = tokio::time::timeout(std::time::Duration::from_secs(2), commands.recv())
            .await
            .unwrap()
            .unwrap()
        else {
            panic!("expected acknowledged input");
        };
        let lease = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .lease
            .clone()
            .unwrap();
        let inventory = files
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .inspect()
            .unwrap();
        accepted
            .send(if delivered { Ok(()) } else { Err(()) })
            .unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), &mut delivery)
                .await
                .is_err()
        );
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        assert!(matches!(
            view.remove_recommendations().await,
            Err(StoreError::Conflict)
        ));
        assert!(root(&state).join("recommendations/a.json").exists());
        assert_eq!(progress(&store).await, 0);
        drop(inventory);
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), delivery)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.is_ok(), delivered, "{result:?}");
        assert_eq!(
            files.get("delivery").unwrap().unwrap().status,
            if delivered {
                RecommendationStatus::Accepted
            } else {
                RecommendationStatus::Proposed
            }
        );
        assert!(
            commands.try_recv().is_err(),
            "settlement must not resend input"
        );
        let receipt = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap()
            .remove_recommendations()
            .await
            .unwrap();
        assert!(receipt.files.is_ok());
        assert!(matches!(receipt.progress, Some(Ok(16))));
        assert!(commands.try_recv().is_err());
        store.close().await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn sql_cleanup_admission_wins_before_new_delivery_and_keeps_input_guards_free() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes) = deleted().await;
    let files = seed(&state);
    files.put(&recommendation("delivery", "unrelated")).unwrap();
    let (target, mut commands) = delivery_runtime(&state, dir.path());
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_admitted);
    let cleanup = tokio::spawn(view.remove_recommendations());
    gate.entered().await;
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(matches!(files.get("a"), Err(RecommendationError::Busy)));
    let application = crate::session_application::SessionApplication::new(state.clone());
    assert!(matches!(
        application
            .accept_recommendation_with_packet("delivery", target, None, None)
            .await,
        Err(
            crate::session_application::RecommendationAcceptError::Store(RecommendationError::Busy)
        )
    ));
    assert!(commands.try_recv().is_err());
    drop(gate);
    assert!(matches!(
        cleanup.await.unwrap().unwrap().progress,
        Some(Ok(16))
    ));
    assert!(matches!(
        application
            .accept_recommendation_with_packet("delivery", target, None, None)
            .await,
        Err(
            crate::session_application::RecommendationAcceptError::Store(
                RecommendationError::SqlAuthorityRequired
            )
        )
    ));
    assert!(commands.try_recv().is_err());
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_cleanup_cancelled_or_expired_admission_preserves_legacy_authority() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [false, true] {
        let (_dir, state, store, writes) = deleted().await;
        let files = seed(&state);
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let mut gate = hold(&writes.replay_before);
        let action = tokio::spawn(view.remove_recommendations());
        gate.entered().await;
        if cancel {
            action.abort();
            assert!(action.await.unwrap_err().is_cancelled());
        } else {
            assert!(matches!(action.await.unwrap(), Err(StoreError::Expired)));
        }
        drop(gate);
        let lane = store.runtime_lane(ID).unwrap();
        let _released = tokio::time::timeout(std::time::Duration::from_secs(2), lane.lock_owned())
            .await
            .unwrap();
        assert_eq!(files.list().unwrap().len(), 3);
        assert_eq!(progress(&store).await, 0);
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_cleanup_progress_failure_then_last_owner_loss_is_explicitly_not_fenced() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = seed(&state);
    let stale = files.get("a").unwrap().unwrap();
    let path = root(&state);
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.remove_recommendations());
    gate.entered().await;
    let blocker = rusqlite::Connection::open(path.join("session-store/v1.sqlite")).unwrap();
    blocker.execute_batch("BEGIN EXCLUSIVE").unwrap();
    drop(gate);
    let receipt = action.await.unwrap().unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Err(StoreError::Busy))));
    blocker.execute_batch("ROLLBACK").unwrap();
    assert_eq!(progress(&store).await, 0);
    assert!(matches!(
        files.put(&stale),
        Err(RecommendationError::SqlAuthorityRequired)
    ));
    state.workspace.lock().unwrap().take();
    drop(files);
    drop(writes);
    let reopened = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            match RecommendationStore::open(path.clone()) {
                Ok(files) => break files,
                Err(RecommendationError::SqlAuthorityRequired | RecommendationError::Busy) => {
                    tokio::task::yield_now().await
                }
                Err(error) => panic!("unexpected reopen error: {error}"),
            }
        }
    })
    .await
    .unwrap();
    // Negative qualification case: fresh Legacy ownership can recreate bytes.
    // Live SQL startup/reopen must close this gap before activation.
    reopened.put(&stale).unwrap();
    assert!(path.join("recommendations/a.json").exists());
    assert_eq!(progress(&store).await, 0);
    store.close().await.unwrap();
}

fn delivery_runtime(
    state: &Arc<AppState>,
    directory: &std::path::Path,
) -> (
    &'static str,
    tokio::sync::mpsc::Receiver<crate::runtime::session_runtime::RuntimeCommand>,
) {
    let result = super::super::organization_tests::input_runtime(state, directory);
    let workspace = state.workspace.lock().unwrap();
    let metadata = &workspace.as_ref().unwrap().metadata;
    let mut target = metadata.read_session(result.0).unwrap();
    target.lifecycle = "alive".into();
    target.lifecycle_phase = "active".into();
    metadata.try_write_session(&target).unwrap();
    result
}
