use super::delete_replay_tests::{deleted, hold, progress};
use super::organization_tests::{root, CREATED, ID};
use super::*;
use crate::workflow_observations::{
    CapturedWriter, FileClaim, Impact, ObservationCandidate, ObservationKind, ObservationOrigin,
    WorkflowObservationStore,
};

fn workflow(state: &Arc<AppState>) -> WorkflowObservationStore {
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .workflow_observations
        .clone()
}
// Inspect copied fixture bytes as data; SQL admission intentionally disables
// the live legacy cache API. Legacy module tests still assert cache semantics.
fn segment_count(state: &Arc<AppState>, id: &str) -> usize {
    let path = root(state).join(format!("workflow-observations/{id}.ndjson"));
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return 0,
        Err(error) => panic!("{error}"),
    };
    bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| crate::workflow_observations::sql_origin::decode_line(line, id).unwrap())
        .filter(|record| record.observation().is_some())
        .count()
}
fn candidate() -> ObservationCandidate {
    ObservationCandidate {
        kind: ObservationKind::Obstacle,
        description: "blocked".into(),
        evidence: "failure".into(),
        problem_area: None,
        reported_impact: Impact::Low,
        confidence: Some(0.8),
    }
}
fn seed(store: &WorkflowObservationStore) -> CapturedWriter {
    let writer = store.capture_writer(ID.into(), CREATED.into());
    writer
        .record_batch(&FileClaim::new(), 1, vec![(0, candidate())])
        .unwrap();
    store
        .record_observation("other", ObservationOrigin::Agent, "other", candidate())
        .unwrap();
    writer
}
#[tokio::test]
async fn sql_evidence_cleanup_revokes_original_writer_preserves_replay_and_reopens_progress() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    let writer = seed(&files);
    let sequence = std::fs::read(root(&state).join("workflow-observations/sequence")).unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_workflow_evidence()
        .await
        .unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(2))));
    assert!(!receipt.detached);
    assert!(writer
        .record_batch(&FileClaim::new(), 2, vec![(0, candidate())])
        .is_err());
    assert_eq!(segment_count(&state, ID), 0);
    assert!(files.session_observation_count(ID).is_err());
    assert_eq!(segment_count(&state, "other"), 1);
    assert_eq!(
        std::fs::read(root(&state).join("workflow-observations/sequence")).unwrap(),
        sequence
    );
    assert!(root(&state).join(format!("events/{ID}.terminal")).exists());
    assert!(root(&state).join(format!("sessions/{ID}.json")).exists());
    assert!(store.get(ID.into()).await.unwrap().is_none());
    let (path, key) = store.location();
    let (path, key) = (path.to_owned(), key.to_owned());
    store.close().await.unwrap();
    let reopened = Arc::new(SessionStore::open(path, key).await.unwrap());
    let writes = SessionWrites::bind(state, reopened.clone()).unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_workflow_evidence()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(2))));
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
async fn sql_evidence_cleanup_rejects_replaced_owner_before_revocation() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    let writer = seed(&files);
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    state
        .workspace
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .workflow_observations =
        WorkflowObservationStore::open(_dir.path().join("replacement-workflow")).unwrap();
    assert!(matches!(
        view.remove_workflow_evidence().await,
        Err(StoreError::Conflict)
    ));
    assert!(writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok());
    assert_eq!(files.session_observation_count(ID).unwrap(), 1);
    assert_eq!(progress(&store).await, 0);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_evidence_cleanup_file_failure_revokes_without_progress_and_retry_preserves_bits() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    let writer = seed(&files);
    let path = root(&state).join(format!("workflow-observations/{ID}.ndjson"));
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_workflow_evidence()
        .await
        .unwrap();
    assert!(receipt.files.is_err());
    assert!(receipt.progress.is_none());
    assert_eq!(progress(&store).await, 0);
    assert!(writer.record_batch(&FileClaim::new(), 2, vec![]).is_err());
    assert!(path.is_dir());
    assert!(files.session_observation_count(ID).is_err());
    std::fs::remove_dir(&path).unwrap();
    store
        .deletion_progress(ID.into(), CREATED.into(), 5, false)
        .await
        .unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_workflow_evidence()
        .await
        .unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(7))));
    assert_eq!(segment_count(&state, ID), 0);
    assert!(files.session_observation_count(ID).is_err());
    assert_eq!(
        store.pending_deletions(None, 100).await.unwrap().jobs.len(),
        1
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_evidence_cleanup_pre_admission_cancellation_and_expiry_preserve_writer() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (_dir, state, store, writes) = deleted().await;
        let files = workflow(&state);
        let writer = seed(&files);
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let mut gate = hold(&writes.replay_before);
        let action = tokio::spawn(view.remove_workflow_evidence());
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
            store.runtime_lane(ID).unwrap().lock_owned(),
        )
        .await
        .unwrap();
        assert!(writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok());
        assert_eq!(files.session_observation_count(ID).unwrap(), 1);
        assert_eq!(progress(&store).await, 0);
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_evidence_cleanup_progress_failure_reports_removed_files_and_retries() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.remove_workflow_evidence());
    gate.entered().await;
    assert_eq!(segment_count(&state, ID), 0);
    assert!(files.session_observation_count(ID).is_err());
    let db = rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    drop(gate);
    let receipt = action.await.unwrap().unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Err(StoreError::Busy))));
    db.execute_batch("ROLLBACK").unwrap();
    assert_eq!(progress(&store).await, 0);
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_workflow_evidence()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(2))));
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_evidence_cleanup_admitted_caller_loss_finishes_on_original_owner() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (_dir, state, store, writes) = deleted().await;
        let files = workflow(&state);
        let writer = seed(&files);
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let mut gate = hold(&writes.replay_admitted);
        let action = tokio::spawn(view.remove_workflow_evidence());
        gate.entered().await;
        // Snapshot this ordinary legacy fixture into an independent root. SQL
        // artifact roots can no longer be reopened through the legacy constructor.
        let replacement_root = _dir.path().join("replacement-workflow");
        let replacement_dir = replacement_root.join("workflow-observations");
        std::fs::create_dir_all(&replacement_dir).unwrap();
        for entry in std::fs::read_dir(root(&state).join("workflow-observations")).unwrap() {
            let entry = entry.unwrap();
            std::fs::copy(entry.path(), replacement_dir.join(entry.file_name())).unwrap();
        }
        let replacement = WorkflowObservationStore::open(replacement_root).unwrap();
        state
            .workspace
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .workflow_observations = replacement.clone();
        let owner = super::organization_tests::organization(&state);
        assert!(owner.pending(ID));
        assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_err());
        if cancel {
            action.abort();
            assert!(action.await.unwrap_err().is_cancelled());
            drop(gate);
        } else {
            drop(gate);
            let receipt = action.await.unwrap().unwrap();
            assert!(receipt.detached);
            assert!(matches!(receipt.progress, Some(Ok(2))));
        }
        super::organization_tests::pending(&owner, false).await;
        assert_eq!(progress(&store).await, 2);
        assert_eq!(segment_count(&state, ID), 0);
        assert!(files.session_observation_count(ID).is_err());
        assert_eq!(replacement.session_observation_count(ID).unwrap(), 1);
        assert!(writer.record_batch(&FileClaim::new(), 2, vec![]).is_err());
        store.close().await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn sql_evidence_cleanup_mutex_stall_keeps_guards_and_independent_input_free() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let (input_id, mut commands) = super::organization_tests::input_runtime(&state, dir.path());
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let (held, release, thread, _) =
        crate::workflow_observations::captured_testing::hold_registration(&files);
    held.entered().await;
    let mut gate = hold(&writes.replay_admitted);
    let (proven, proof) = std::sync::mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        let progressed = proof
            .recv_timeout(std::time::Duration::from_secs(4))
            .is_ok();
        let _ = release.send(());
        thread.join().unwrap();
        progressed
    });
    let action = tokio::spawn(view.remove_workflow_evidence());
    gate.entered().await;
    drop(gate);
    // Let the blocking file job reach the held store mutex.
    tokio::task::yield_now().await;
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
                assert_eq!(data, "evidence-input\r");
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
            "evidence-input\r".into(),
        ),
    )
    .await;
    let driven = tokio::time::timeout(std::time::Duration::from_secs(1), driver).await;
    let _ = proven.send(());
    assert!(
        watchdog.join().unwrap(),
        "sole Tokio worker blocked behind evidence mutex"
    );
    driven.unwrap().unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    let receipt = action.await.unwrap().unwrap();
    assert!(matches!(receipt.progress, Some(Ok(2))));
    store.close().await.unwrap();
}
