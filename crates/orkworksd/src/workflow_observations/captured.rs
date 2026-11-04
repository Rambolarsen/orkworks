//! Revocable file admission; this module never calls application or SQL owners.
use super::*;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::time::{Duration, Instant};
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AdmissionError {
    Conflict,
    Cancelled,
    Expired,
    TooLarge,
}
impl From<AdmissionError> for crate::session_store::StoreError {
    fn from(error: AdmissionError) -> Self {
        match error {
            AdmissionError::Conflict => Self::Conflict,
            AdmissionError::Cancelled => Self::Cancelled,
            AdmissionError::Expired => Self::Expired,
            AdmissionError::TooLarge => Self::TooLarge,
        }
    }
}

const PENDING: u8 = 0;
const ADMITTED: u8 = 1;
const CANCELLED: u8 = 2;
const EXPIRED: u8 = 3;

pub(crate) struct FileClaim {
    state: AtomicU8,
    pub(crate) deadline: Instant,
}
impl FileClaim {
    pub(crate) fn new() -> Self {
        Self {
            state: AtomicU8::new(PENDING),
            deadline: Instant::now() + Duration::from_secs(2),
        }
    }
    pub(crate) fn cancel(&self) -> bool {
        self.stop(CANCELLED)
    }
    pub(crate) fn expire(&self) -> bool {
        self.stop(EXPIRED)
    }
    fn stop(&self, state: u8) -> bool {
        self.state
            .compare_exchange(PENDING, state, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
    fn admit(&self) -> Result<(), AdmissionError> {
        if Instant::now() >= self.deadline {
            self.expire();
        }
        match self
            .state
            .compare_exchange(PENDING, ADMITTED, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => Ok(()),
            Err(EXPIRED) => Err(AdmissionError::Expired),
            Err(_) => Err(AdmissionError::Cancelled),
        }
    }
}
#[derive(Clone)]
pub(crate) struct CapturedWriter {
    store: WorkflowObservationStore,
    id: String,
    valid: Arc<AtomicBool>,
}
impl WorkflowObservationStore {
    // Blocking-only. The caller validates authoritative creation after registration.
    pub(crate) fn capture_writer(&self, id: String, created: String) -> CapturedWriter {
        let mut inner = self.inner.lock().unwrap();
        inner.permits.retain(|_, permit| permit.strong_count() > 0);
        let permit = inner.permits.entry((id.clone(), created)).or_default();
        let valid = permit.upgrade().unwrap_or_else(|| {
            let valid = Arc::new(AtomicBool::new(true));
            *permit = Arc::downgrade(&valid);
            valid
        });
        CapturedWriter {
            store: self.clone(),
            id,
            valid,
        }
    }
}
impl CapturedWriter {
    pub(crate) fn instance_id(&self) -> u64 {
        self.store.instance_id()
    }
    pub(crate) fn record_batch(
        &self,
        claim: &FileClaim,
        generation: u64,
        candidates: Vec<(usize, ObservationCandidate)>,
    ) -> Result<Vec<(usize, Result<RecordOutcome, RecordError>)>, AdmissionError> {
        if candidates.len() > 5 {
            return Err(AdmissionError::TooLarge);
        }
        let mut inner = self.store.inner.lock().unwrap();
        if !self.valid.load(Ordering::SeqCst) {
            return Err(AdmissionError::Conflict);
        }
        claim.admit()?;
        #[cfg(test)]
        if let Some(gate) = inner.capture_gate.take() {
            gate.enter();
        }
        Ok(candidates
            .into_iter()
            .map(|(index, candidate)| {
                let key = format!("final-scan:{generation}:{index}");
                let result = self.store.record_locked(
                    &mut inner,
                    &self.id,
                    ObservationOrigin::Peon,
                    &key,
                    candidate,
                );
                (index, result)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    #[test]
    fn shared_owner_preserves_sequence_duplicate_conflict_and_reopen() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkflowObservationStore::open(root.path().into()).unwrap();
        let writer = store.capture_writer("s".into(), "creation".into());
        let cloned = store.clone();
        assert_eq!(cloned.instance_id(), store.instance_id());
        assert_eq!(
            cloned.next_evaluation_generation(),
            store.evaluation_generation()
        );
        let first = writer
            .record_batch(&FileClaim::new(), 1, vec![(2, candidate())])
            .unwrap()
            .remove(0)
            .1
            .unwrap();
        let RecordOutcome::Accepted(first) = first else {
            panic!("expected accepted")
        };
        let duplicate = cloned
            .record_observation("s", ObservationOrigin::Peon, "final-scan:1:2", candidate())
            .unwrap();
        assert!(
            matches!(duplicate, RecordOutcome::Duplicate { sequence, observation_id, .. } if sequence == first.sequence && observation_id == first.id)
        );
        let mut changed = candidate();
        changed.description = "changed".into();
        assert_eq!(
            cloned.record_observation("s", ObservationOrigin::Peon, "final-scan:1:2", changed),
            Err(RecordError::IdempotencyConflict)
        );
        let joins: Vec<_> = (0..12)
            .map(|i| {
                let store = store.clone();
                std::thread::spawn(move || {
                    store
                        .record_observation(
                            "s",
                            ObservationOrigin::Peon,
                            &format!("parallel-{i}"),
                            candidate(),
                        )
                        .unwrap()
                })
            })
            .collect();
        for join in joins {
            join.join().unwrap();
        }
        let observations = store.session_observations("s").unwrap();
        assert_eq!(observations.len(), 13);
        assert_eq!(
            observations.iter().map(|o| o.sequence).collect::<Vec<_>>(),
            (1..=13).collect::<Vec<_>>()
        );
        drop(writer);
        drop(cloned);
        drop(store);
        let reopened = WorkflowObservationStore::open(root.path().into()).unwrap();
        assert_eq!(reopened.session_observations("s").unwrap(), observations);
        assert!(matches!(
            reopened
                .record_observation("s", ObservationOrigin::Peon, "final-scan:1:2", candidate())
                .unwrap(),
            RecordOutcome::Duplicate { sequence: 1, .. }
        ));
    }
    #[test]
    fn deletion_revokes_only_target_even_on_failure_and_releases_registry() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkflowObservationStore::open(root.path().into()).unwrap();
        let writer = store.capture_writer("s".into(), "creation".into());
        let other = store.capture_writer("other".into(), "creation".into());
        writer
            .record_batch(&FileClaim::new(), 1, vec![(0, candidate())])
            .unwrap();
        let segment = store.segment_path("s");
        fs::remove_file(&segment).unwrap();
        fs::create_dir(&segment).unwrap();
        assert!(store.delete_session_observations("s").is_err());
        assert!(matches!(
            writer.record_batch(&FileClaim::new(), 2, vec![(0, candidate())]),
            Err(AdmissionError::Conflict)
        ));
        assert_eq!(store.session_observation_count("s").unwrap(), 1);
        assert!(other
            .record_batch(&FileClaim::new(), 1, vec![(0, candidate())])
            .is_ok());
        fs::remove_dir(&segment).unwrap();
        store.delete_session_observations("s").unwrap();
        assert_eq!(store.session_observation_count("s").unwrap(), 0);
        assert!(!segment.exists());
        for i in 0..100 {
            drop(store.capture_writer(format!("expired-{i}"), "created".into()));
        }
        let _live = store.capture_writer("live".into(), "created".into());
        assert!(store.inner.lock().unwrap().permits.len() <= 2);
    }
    #[test]
    fn cancellation_and_expiry_cannot_claim_or_change_sequence() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkflowObservationStore::open(root.path().into()).unwrap();
        let writer = store.capture_writer("s".into(), "creation".into());
        for expired in [false, true] {
            let claim = FileClaim::new();
            if expired {
                claim.expire();
            } else {
                claim.cancel();
            }
            assert!(writer
                .record_batch(&claim, 1, vec![(0, candidate())])
                .is_err());
        }
        assert!(!store.dir.join("sequence").exists());
        let admitted = FileClaim::new();
        writer
            .record_batch(&admitted, 1, vec![(0, candidate())])
            .unwrap();
        assert!(!admitted.cancel());
        assert!(!admitted.expire());
        store.delete_session_observations("s").unwrap();
        assert_eq!(
            fs::read_to_string(store.dir.join("sequence"))
                .unwrap()
                .trim(),
            "1"
        );
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    pub(crate) struct Gate {
        entered: tokio::sync::Notify,
        released: Mutex<std::sync::mpsc::Receiver<()>>,
    }
    pub(crate) fn hold(
        store: &WorkflowObservationStore,
    ) -> (Arc<Gate>, std::sync::mpsc::Sender<()>) {
        let (released, rx) = std::sync::mpsc::channel();
        let gate = Arc::new(Gate {
            entered: tokio::sync::Notify::new(),
            released: Mutex::new(rx),
        });
        store.inner.lock().unwrap().capture_gate = Some(gate.clone());
        (gate, released)
    }
    pub(crate) fn hold_registration(
        store: &WorkflowObservationStore,
    ) -> (
        Arc<Gate>,
        std::sync::mpsc::Sender<()>,
        std::thread::JoinHandle<()>,
    ) {
        let (release, rx) = std::sync::mpsc::channel();
        let gate = Arc::new(Gate {
            entered: tokio::sync::Notify::new(),
            released: Mutex::new(rx),
        });
        let store = store.clone();
        let held = gate.clone();
        let thread = std::thread::spawn(move || {
            let _guard = store.inner.lock().unwrap();
            held.enter();
        });
        (gate, release, thread)
    }
    impl Gate {
        pub(super) fn enter(&self) {
            self.entered.notify_one();
            let _ = self
                .released
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(15));
        }
        pub(crate) async fn entered(&self) {
            self.entered.notified().await;
        }
    }
}

#[cfg(test)]
mod ordering_tests {
    use super::*;
    #[tokio::test]
    async fn admitted_batch_finishes_before_delete_which_forgets_all_duplicates() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkflowObservationStore::open(root.path().into()).unwrap();
        let writer = store.capture_writer("s".into(), "created".into());
        let (gate, release) = testing::hold(&store);
        let batch = std::thread::spawn(move || {
            let candidate = || ObservationCandidate {
                kind: ObservationKind::Obstacle,
                description: "blocked".into(),
                evidence: "failure".into(),
                problem_area: None,
                reported_impact: Impact::Low,
                confidence: Some(0.8),
            };
            writer
                .record_batch(
                    &FileClaim::new(),
                    1,
                    vec![(0, candidate()), (1, candidate())],
                )
                .unwrap()
        });
        tokio::time::timeout(std::time::Duration::from_secs(3), gate.entered())
            .await
            .unwrap();
        let clone = store.clone();
        let (started, start) = std::sync::mpsc::channel();
        let (done, finished) = std::sync::mpsc::channel();
        let deletion = std::thread::spawn(move || {
            started.send(()).unwrap();
            clone.delete_session_observations("s").unwrap();
            done.send(()).unwrap();
        });
        start
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        assert!(finished.try_recv().is_err());
        release.send(()).unwrap();
        assert_eq!(batch.join().unwrap().len(), 2);
        deletion.join().unwrap();
        assert_eq!(store.session_observation_count("s").unwrap(), 0);
        assert!(!store.segment_path("s").exists());
        let inner = store.inner.lock().unwrap();
        assert!(inner.idempotency.is_empty());
        assert_eq!(inner.last_issued_sequence, 2);
    }
}
