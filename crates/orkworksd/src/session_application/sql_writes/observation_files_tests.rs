use super::*;
use crate::workflow_observations::{
    sql_origin::{ScanRequest, WriteError},
    FileClaim, Impact, ObservationCandidate, ObservationKind, ObservationOrigin, RecordError,
    SqlCandidate, WorkflowObservationStore,
};
use crate::workspace_runtime::WorkspaceLease;
use std::time::{Duration, Instant};
async fn fixture() -> (
    tempfile::TempDir,
    WorkspaceLease,
    WorkflowObservationStore,
    SessionStore,
    crate::session_store::ObservationSource,
) {
    let root = tempfile::tempdir().unwrap();
    let lease = WorkspaceLease::acquire(root.path()).unwrap();
    let workflow = WorkflowObservationStore::open(root.path().into()).unwrap();
    let store = SessionStore::initialize(root.path().into(), "/workspace".into())
        .await
        .unwrap();
    let meta = crate::test_support::test_session_metadata(
        "s",
        "Before",
        "/workspace",
        "running",
        "2026-01-01T00:00:00Z",
        "2026-01-02T00:00:00Z",
    );
    store
        .create(serde_json::to_value(meta).unwrap())
        .await
        .unwrap();
    let source = store
        .capture_observation_source("s".into(), "2026-01-01T00:00:00Z".into())
        .await
        .unwrap();
    (root, lease, workflow, store, source)
}
fn value() -> ObservationCandidate {
    ObservationCandidate {
        kind: ObservationKind::Obstacle,
        description: "blocked".into(),
        evidence: "error".into(),
        problem_area: None,
        reported_impact: Impact::Low,
        confidence: Some(0.8),
    }
}
#[tokio::test]
async fn read_admission_cannot_be_upgraded_to_writer_even_with_a_new_claim() {
    let (root, lease, workflow, store, source) = fixture().await;
    let mut candidate = workflow.sql_candidate(&lease).unwrap();
    candidate
        .admit(ObservationOpenAdmission { _private: () }, &FileClaim::new())
        .unwrap();
    let claim = FileClaim::new();
    assert!(matches!(
        candidate.admit_write(ObservationWriteAdmission { source }, &claim),
        Err(StoreError::Conflict)
    ));
    assert!(claim.pending().is_ok());
    assert!(!root.path().join("workflow-observations/sequence").exists());
    store.close().await.unwrap();
}
#[tokio::test]
async fn cancelled_write_claim_makes_no_transition_or_file_effect() {
    let (root, lease, workflow, store, source) = fixture().await;
    let candidate = workflow.sql_candidate(&lease).unwrap();
    let claim = FileClaim::new();
    assert!(claim.cancel());
    assert!(matches!(
        candidate.admit_write(ObservationWriteAdmission { source }, &claim),
        Err(StoreError::Cancelled)
    ));
    assert!(workflow.workspace_observations().is_ok());
    assert!(!root.path().join("workflow-observations/sequence").exists());
    store.close().await.unwrap();
}
#[tokio::test]
async fn admitted_write_grant_owns_exclusivity_and_retains_sql_floor_on_payload_failure() {
    let (root, lease, workflow, store, source) = fixture().await;
    let writer = workflow
        .sql_candidate(&lease)
        .unwrap()
        .admit_write(ObservationWriteAdmission { source }, &FileClaim::new())
        .unwrap();
    assert!(matches!(
        SqlCandidate::bind(root.path(), &lease),
        Err(StoreError::Busy)
    ));
    assert!(workflow.workspace_observations().is_err());
    assert!(matches!(
        writer.persist(
            ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap(),
            ObservationOrigin::Peon,
            "",
            value()
        ),
        Err(WriteError::Candidate(RecordError::EmptyIdempotencyKey))
    ));
    assert!(workflow.sql_candidate(&lease).is_ok());
    assert!(workflow.workspace_observations().is_err());
    assert!(!root.path().join("workflow-observations/sequence").exists());
    store.close().await.unwrap();
}
#[tokio::test]
async fn admitted_writer_cannot_extend_original_claim_deadline_with_a_new_scan_request() {
    let (root, lease, workflow, store, source) = fixture().await;
    let mut claim = FileClaim::new();
    claim.deadline = Instant::now() + Duration::from_millis(200);
    let writer = workflow
        .sql_candidate(&lease)
        .unwrap()
        .admit_write(ObservationWriteAdmission { source }, &claim)
        .unwrap();
    let scan = ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap();
    tokio::time::sleep(Duration::from_millis(230)).await;
    assert!(matches!(
        writer.persist(scan, ObservationOrigin::Peon, "key", value()),
        Err(WriteError::Storage(StoreError::Expired))
    ));
    assert!(!root.path().join("workflow-observations/sequence").exists());
    store.close().await.unwrap();
}

#[tokio::test]
async fn admitted_effect_keeps_exclusivity_past_expiry_and_returns_its_definitive_receipt() {
    let (root, lease, workflow, store, source) = fixture().await;
    let lease = Arc::new(lease);
    // A short original deadline, with a later scan, exercises the grant's cap.
    let mut short = FileClaim::new();
    short.deadline = Instant::now() + Duration::from_millis(500);
    let claim = Arc::new(short);
    let deadline = claim.deadline;
    let writer = workflow
        .sql_candidate(&lease)
        .unwrap()
        .admit_write(
            ObservationWriteAdmission {
                source: source.clone(),
            },
            &claim,
        )
        .unwrap();
    let root_path = root.path().to_owned();
    let held_lease = lease.clone();
    let held_store = workflow.clone();
    let held_claim = claim.clone();
    crate::workflow_observations::sql_origin::test_after_counter(move || {
        std::thread::sleep(
            deadline.saturating_duration_since(Instant::now()) + Duration::from_millis(10),
        );
        assert!(matches!(
            SqlCandidate::bind(&root_path, &held_lease),
            Err(StoreError::Busy)
        ));
        assert!(held_store.workspace_observations().is_err());
        assert!(!held_claim.expire());
        assert!(!held_claim.cancel());
        assert_eq!(
            std::fs::read(root_path.join("workflow-observations/sequence")).unwrap(),
            b"1"
        );
    });
    let result = writer
        .persist(
            ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap(),
            ObservationOrigin::Peon,
            "key",
            value(),
        )
        .unwrap();
    let crate::workflow_observations::RecordOutcome::Accepted(accepted) = result else {
        panic!("definitive committed receipt")
    };
    assert!(Instant::now() > deadline);
    assert_eq!(accepted.sequence, 1);
    let before = std::fs::read(root.path().join("workflow-observations/s.ndjson")).unwrap();
    let retry = workflow
        .sql_candidate(&lease)
        .unwrap()
        .admit_write(ObservationWriteAdmission { source }, &FileClaim::new())
        .unwrap()
        .persist(
            ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap(),
            ObservationOrigin::Peon,
            "key",
            value(),
        )
        .unwrap();
    assert!(
        matches!(retry,crate::workflow_observations::RecordOutcome::Duplicate{observation_id,sequence:1,..} if observation_id==accepted.id)
    );
    assert_eq!(
        std::fs::read(root.path().join("workflow-observations/s.ndjson")).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"1"
    );
    store.close().await.unwrap();
}
