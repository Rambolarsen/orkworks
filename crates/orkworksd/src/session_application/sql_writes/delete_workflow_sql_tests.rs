use super::delete_replay_tests::{deleted, progress};
use super::organization_tests::{root, CREATED, ID};
use super::*;
use crate::workflow_observations::{
    FileClaim, Impact, ObservationCandidate, ObservationKind, ObservationOrigin,
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
fn seed(files: &WorkflowObservationStore) {
    files
        .record_observation(ID, ObservationOrigin::Peon, "first", candidate())
        .unwrap();
    files
        .record_observation("other", ObservationOrigin::Agent, "other", candidate())
        .unwrap();
}
#[tokio::test]
async fn workflow_cleanup_uses_sql_owner_and_preserves_non_target_evidence() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let directory = root(&state).join("workflow-observations");
    let other = std::fs::read(directory.join("other.ndjson")).unwrap();
    let sequence = std::fs::read(directory.join("sequence")).unwrap();
    let lease = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .lease
        .clone()
        .unwrap();
    let mut opening = files.sql_candidate(&lease).unwrap();
    opening
        .admit(ObservationOpenAdmission { _private: () }, &FileClaim::new())
        .unwrap();
    drop(opening);
    let result = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_workflow_evidence()
        .await
        .unwrap();
    assert!(result.files.is_ok(), "{:?}", result.files);
    assert!(matches!(result.progress, Some(Ok(2))));
    assert!(!directory.join(format!("{ID}.ndjson")).exists());
    assert_eq!(
        std::fs::read(directory.join("other.ndjson")).unwrap(),
        other
    );
    assert_eq!(std::fs::read(directory.join("sequence")).unwrap(), sequence);
    assert!(files.workspace_observations().is_err());
    assert_eq!(progress(&store).await, 2);
    store.close().await.unwrap();
}
#[tokio::test]
async fn captured_workflow_cleanup_refuses_replaced_epoch_before_any_file_effect() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let writer = files.capture_writer(ID.into(), CREATED.into());
    let directory = root(&state).join("workflow-observations");
    let before = std::fs::read(directory.join(format!("{ID}.ndjson"))).unwrap();
    let sequence = std::fs::read(directory.join("sequence")).unwrap();
    let captured = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let conn =
        rusqlite::Connection::open(store.location().0.join("session-store/v1.sqlite")).unwrap();
    conn.execute(
        "UPDATE store_header SET epoch=?1",
        [uuid::Uuid::new_v4().to_string()],
    )
    .unwrap();
    assert!(matches!(
        captured.remove_workflow_evidence().await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        std::fs::read(directory.join(format!("{ID}.ndjson"))).unwrap(),
        before
    );
    assert_eq!(std::fs::read(directory.join("sequence")).unwrap(), sequence);
    assert!(writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok());
    assert_eq!(progress(&store).await, 0);
    store.close().await.unwrap();
}

#[tokio::test]
async fn workflow_progress_cannot_acknowledge_an_epoch_replaced_after_file_removal() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let captured = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut pause = super::delete_replay_tests::hold(&writes.replay_removed);
    let job = tokio::spawn(captured.remove_workflow_evidence());
    pause.entered().await;
    let conn =
        rusqlite::Connection::open(store.location().0.join("session-store/v1.sqlite")).unwrap();
    conn.execute(
        "UPDATE store_header SET epoch=?1",
        [uuid::Uuid::new_v4().to_string()],
    )
    .unwrap();
    drop(pause);
    let receipt = job.await.unwrap().unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Err(StoreError::Conflict))));
    assert!(!root(&state)
        .join(format!("workflow-observations/{ID}.ndjson"))
        .exists());
    assert!(root(&state)
        .join("workflow-observations/other.ndjson")
        .exists());
    let remaining: u8 = conn
        .query_row(
            "SELECT progress FROM deletion_jobs WHERE id=?1",
            [ID],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 0);
    store.close().await.unwrap();
}

#[tokio::test]
async fn captured_workflow_job_changes_refuse_without_revocation_or_byte_changes() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for sql in [
        "DELETE FROM deletion_jobs",
        "UPDATE deletion_jobs SET created_at='replacement'",
        "UPDATE sessions SET deleted=0",
        "DELETE FROM sessions",
        "UPDATE store_header SET workspace='/foreign'",
        "UPDATE store_header SET epoch='invalid'",
    ] {
        let (_dir, state, store, writes) = deleted().await;
        let files = workflow(&state);
        seed(&files);
        let writer = files.capture_writer(ID.into(), CREATED.into());
        let directory = root(&state).join("workflow-observations");
        let before = std::fs::read(directory.join(format!("{ID}.ndjson"))).unwrap();
        let other = std::fs::read(directory.join("other.ndjson")).unwrap();
        let sequence = std::fs::read(directory.join("sequence")).unwrap();
        let captured = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let conn =
            rusqlite::Connection::open(store.location().0.join("session-store/v1.sqlite")).unwrap(); // Deliberately inject broken job/row relationships to exercise refusal.
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(sql).unwrap();
        assert!(captured.remove_workflow_evidence().await.is_err(), "{sql}");
        assert_eq!(
            std::fs::read(directory.join(format!("{ID}.ndjson"))).unwrap(),
            before,
            "{sql}"
        );
        assert_eq!(
            std::fs::read(directory.join("other.ndjson")).unwrap(),
            other
        );
        assert_eq!(std::fs::read(directory.join("sequence")).unwrap(), sequence);
        assert!(
            writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok(),
            "revoked before admission: {sql}"
        );
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn workflow_capture_cannot_adopt_a_later_epoch_from_the_retained_worker() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let path = root(&state).join(format!("workflow-observations/{ID}.ndjson"));
    let before = std::fs::read(&path).unwrap();
    let conn =
        rusqlite::Connection::open(store.location().0.join("session-store/v1.sqlite")).unwrap();
    conn.execute(
        "UPDATE store_header SET epoch=?1",
        [uuid::Uuid::new_v4().to_string()],
    )
    .unwrap();
    let captured = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    assert!(matches!(
        captured.remove_workflow_evidence().await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert!(files.workspace_observations().is_ok());
    store.close().await.unwrap();
}
#[tokio::test]
async fn workflow_progress_bits_can_advance_without_replacing_the_original_job() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for bits in [0, 5, 31] {
        let (_dir, state, store, writes) = deleted().await;
        let files = workflow(&state);
        seed(&files);
        let captured = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        store
            .deletion_progress(ID.into(), CREATED.into(), bits, false)
            .await
            .unwrap();
        let receipt = captured.remove_workflow_evidence().await.unwrap();
        assert!(receipt.files.is_ok());
        let expected = match bits {
            0 => 2,
            5 => 7,
            _ => 31,
        };
        assert!(matches!(receipt.progress,Some(Ok(value))if value==expected));
        assert_eq!(
            store.pending_deletions(None, 100).await.unwrap().jobs.len(),
            1
        );
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn workflow_progress_merges_same_job_bits_advanced_after_file_removal() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let captured = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut pause = super::delete_replay_tests::hold(&writes.replay_removed);
    let job = tokio::spawn(captured.remove_workflow_evidence());
    pause.entered().await;
    store
        .deletion_progress(ID.into(), CREATED.into(), 5, false)
        .await
        .unwrap();
    drop(pause);
    let receipt = job.await.unwrap().unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(7))));
    assert_eq!(progress(&store).await, 7);
    store.close().await.unwrap();
}

async fn capture_ending(
    state: &Arc<AppState>,
    writes: &Arc<SessionWrites>,
) -> lifecycle::CapturedSqlObservation {
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    writes
        .lifecycle(
            ID.into(),
            runtime.clone(),
            lifecycle::LifecycleWrite::Transition {
                status: "killed".into(),
            },
        )
        .await
        .unwrap();
    writes
        .capture_ending_history(state, ID.into(), runtime)
        .await
        .unwrap()
        .capture_sql_observation_origin()
        .await
        .unwrap()
}
fn final_snapshot() -> metadata::ObservedStatusSnapshotMetadata {
    metadata::ObservedStatusSnapshotMetadata {
        value: None,
        source: "process".into(),
        confidence: None,
        observed_at: None,
        attention_origin: None,
    }
}
fn inferred() -> crate::peon::PeonWorkflowObservation {
    serde_json::from_value(serde_json::json!({"kind":"obstacle","description":"blocked","evidence":"failure","reportedImpact":"low","confidence":0.8})).unwrap()
}
#[tokio::test]
async fn admitted_original_batch_settles_before_close_first_workflow_deletion() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) =
        super::organization_tests::fixture().await;
    let files = workflow(&state);
    files
        .record_observation("other", ObservationOrigin::Agent, "other", candidate())
        .unwrap();
    let directory = root(&state).join("workflow-observations");
    let other = std::fs::read(directory.join("other.ndjson")).unwrap();
    let captured = Arc::new(capture_ending(&state, &writes).await);
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    let finalizer = writes
        .capture_ending_history(&state, ID.into(), runtime)
        .await
        .unwrap();
    let (at_counter, release) =
        lifecycle::hold_observation(&writes, lifecycle::ObservationStage::AfterCounter);
    let original = captured.clone();
    let generation = original.run_generation();
    let batch = tokio::spawn(async move {
        original
            .persist_workflow_observations(generation, vec![inferred(); 5], vec!["failure".into()])
            .await
    });
    at_counter.await.unwrap();
    let original_writes = writes.clone();
    let deletion = tokio::spawn(async move {
        finalizer
            .complete(final_snapshot(), "killed".into())
            .await
            .unwrap();
        drop(commands);
        original_writes
            .capture_organization(ID.into())
            .await
            .unwrap()
            .begin_delete()
            .await
            .unwrap();
        original_writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap()
            .remove_workflow_evidence()
            .await
    });
    tokio::task::yield_now().await;
    assert!(!deletion.is_finished());
    assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_err());
    assert_eq!(std::fs::read(directory.join("sequence")).unwrap(), b"2");
    release.send(()).unwrap();
    let receipt = batch.await.unwrap().unwrap();
    assert_eq!(receipt.candidates.len(), 5);
    assert!(receipt.candidates.iter().all(|c| matches!(
        c,
        lifecycle::SqlCandidateResult::Recorded(Ok(
            crate::workflow_observations::RecordOutcome::Accepted(_)
        ))
    )));
    let removed = deletion.await.unwrap().unwrap();
    assert!(removed.files.is_ok());
    assert!(matches!(removed.progress, Some(Ok(2))));
    assert_eq!(std::fs::read(directory.join("sequence")).unwrap(), b"6");
    assert!(!directory.join(format!("{ID}.ndjson")).exists());
    assert_eq!(
        std::fs::read(directory.join("other.ndjson")).unwrap(),
        other
    );
    assert!(captured
        .persist_workflow_observations(generation, vec![inferred()], vec!["failure".into()])
        .await
        .is_err());
    assert_eq!(std::fs::read(directory.join("sequence")).unwrap(), b"6");
    store.close().await.unwrap();
}
#[tokio::test]
async fn deletion_revokes_original_sql_permit_and_delayed_batch_cannot_recreate_evidence() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) =
        super::organization_tests::fixture().await;
    let files = workflow(&state);
    files
        .record_observation("other", ObservationOrigin::Agent, "other", candidate())
        .unwrap();
    let captured = capture_ending(&state, &writes).await;
    let source = captured.source().clone();
    let revocation = files.capture_sql_revocation(ID.into(), CREATED.into());
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    writes
        .capture_ending_history(&state, ID.into(), runtime)
        .await
        .unwrap()
        .complete(final_snapshot(), "killed".into())
        .await
        .unwrap();
    drop(commands);
    writes
        .capture_organization(ID.into())
        .await
        .unwrap()
        .begin_delete()
        .await
        .unwrap();
    let removed = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_workflow_evidence()
        .await
        .unwrap();
    assert!(removed.files.is_ok());
    assert!(matches!(removed.progress, Some(Ok(2))));
    let _settled = store.runtime_lane(ID).unwrap().lock_owned().await;
    let lease = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .lease
        .clone()
        .unwrap();
    let claim = FileClaim::new();
    // A trusted test write token isolates revocation from the row/phase fences.
    assert!(matches!(
        files
            .sql_candidate(&lease)
            .unwrap()
            .admit_write(ObservationWriteAdmission { source, revocation }, &claim),
        Err(StoreError::Conflict)
    ));
    assert!(claim.pending().is_ok());
    drop(_settled);
    assert!(captured
        .persist_workflow_observations(
            captured.run_generation(),
            vec![inferred()],
            vec!["failure".into()]
        )
        .await
        .is_err());
    let directory = root(&state).join("workflow-observations");
    assert!(!directory.join(format!("{ID}.ndjson")).exists());
    assert_eq!(std::fs::read(directory.join("sequence")).unwrap(), b"1");
    // Recreate only this controlled SQL fixture; Workflow cleanup itself never
    // finalizes the job or grants fresh runtime/session authority.
    let conn =
        rusqlite::Connection::open(store.location().0.join("session-store/v1.sqlite")).unwrap();
    conn.execute("DELETE FROM deletion_jobs WHERE id=?1", [ID])
        .unwrap();
    conn.execute("DELETE FROM sessions WHERE id=?1", [ID])
        .unwrap();
    let mut replacement = crate::test_support::test_session_metadata(
        ID,
        "Replacement",
        store.location().1,
        "running",
        "2026-02-01T00:00:00Z",
        "2026-02-01T00:00:00Z",
    );
    replacement.lifecycle = "alive".into();
    replacement.lifecycle_phase = "active".into();
    store
        .create(serde_json::to_value(replacement).unwrap())
        .await
        .unwrap();
    assert!(captured
        .persist_workflow_observations(
            captured.run_generation(),
            vec![inferred()],
            vec!["failure".into()]
        )
        .await
        .is_err());
    assert!(!directory.join(format!("{ID}.ndjson")).exists());
    assert_eq!(std::fs::read(directory.join("sequence")).unwrap(), b"1");
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["createdAt"],
        "2026-02-01T00:00:00Z"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn workflow_application_guard_expiry_releases_candidate_and_lane_while_guard_stays_held() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let writer = files.capture_writer(ID.into(), CREATED.into());
    let captured = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut pause = super::delete_replay_tests::hold(&writes.replay_before);
    let job = tokio::spawn(captured.remove_workflow_evidence());
    pause.entered().await;
    let lease = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .lease
        .clone()
        .unwrap();
    let guard = state.projection_lock.lock().unwrap();
    drop(pause);
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(3), job)
            .await
            .unwrap()
            .unwrap(),
        Err(StoreError::Expired)
    ));
    let _settled = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        store.runtime_lane(ID).unwrap().lock_owned(),
    )
    .await
    .unwrap();
    assert!(
        files.sql_candidate(&lease).is_ok(),
        "expired job retained canonical exclusivity"
    );
    assert!(writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok());
    assert!(root(&state)
        .join(format!("workflow-observations/{ID}.ndjson"))
        .exists());
    assert_eq!(progress(&store).await, 0);
    drop(guard);
    store.close().await.unwrap();
}
#[tokio::test]
async fn workflow_admitted_caller_loss_retains_last_slot_lane_and_lease_until_settlement() {
    const CHILD: &str = "ORKWORKS_WORKFLOW_DELETE_SLOT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output=tokio::task::spawn_blocking(||std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact","session_application::sql_writes::delete_workflow_sql_tests::workflow_admitted_caller_loss_retains_last_slot_lane_and_lease_until_settlement","--nocapture"])
            .env(CHILD,"1").output().unwrap()).await.unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let captured = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let held: Vec<_> = (0..31).map(|_| file_budget::reserve(0).unwrap()).collect();
    let mut pause = super::delete_replay_tests::hold(&writes.replay_admitted);
    let job = tokio::spawn(captured.remove_workflow_evidence());
    pause.entered().await;
    let lease = {
        let mut ws = state.workspace.lock().unwrap();
        Arc::downgrade(&ws.as_mut().unwrap().lease.take().unwrap())
    };
    let organization = super::organization_tests::organization(&state);
    job.abort();
    let _ = job.await;
    tokio::time::sleep(std::time::Duration::from_millis(2050)).await;
    assert!(lease.upgrade().is_some());
    assert!(organization.pending(ID));
    assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_err());
    assert!(matches!(
        file_budget::reserve(0),
        Err(StoreError::QueueFull)
    ));
    drop(pause);
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if let Ok(slot) = file_budget::reserve(0) {
                drop(slot);
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(lease.upgrade().is_none());
    assert!(!organization.pending(ID));
    assert_eq!(progress(&store).await, 2);
    assert!(!root(&state)
        .join(format!("workflow-observations/{ID}.ndjson"))
        .exists());
    assert!(root(&state)
        .join("workflow-observations/other.ndjson")
        .exists());
    assert_eq!(
        std::fs::read(root(&state).join("workflow-observations/sequence")).unwrap(),
        b"2"
    );
    drop(held);
    store.close().await.unwrap();
}

#[tokio::test]
async fn mismatched_tombstone_payload_id_refuses_before_file_or_revocation_effects() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = workflow(&state);
    seed(&files);
    let writer = files.capture_writer(ID.into(), CREATED.into());
    let directory = root(&state).join("workflow-observations");
    let before = std::fs::read(directory.join(format!("{ID}.ndjson"))).unwrap();
    let other = std::fs::read(directory.join("other.ndjson")).unwrap();
    let sequence = std::fs::read(directory.join("sequence")).unwrap();
    let captured = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let conn =
        rusqlite::Connection::open(store.location().0.join("session-store/v1.sqlite")).unwrap();
    conn.execute(
        "UPDATE sessions SET metadata=json_set(metadata,'$.id','foreign-session') WHERE id=?1",
        [ID],
    )
    .unwrap();
    let result = captured.remove_workflow_evidence().await;
    assert!(matches!(result, Err(StoreError::Corrupt)), "{result:?}");
    assert_eq!(
        std::fs::read(directory.join(format!("{ID}.ndjson"))).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read(directory.join("other.ndjson")).unwrap(),
        other
    );
    assert_eq!(std::fs::read(directory.join("sequence")).unwrap(), sequence);
    assert!(writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok());
    let bits: u8 = conn
        .query_row(
            "SELECT progress FROM deletion_jobs WHERE id=?1",
            [ID],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(bits, 0);
    store.close().await.unwrap();
}
