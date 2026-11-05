//! Captured workflow files stay outside the SQL worker and application guards.
use super::*;
use crate::session_application::sql_writes::file_budget as budget;
use crate::workflow_observations::{CapturedWriter, FileClaim, RecordError, RecordOutcome};
#[derive(Debug)]
pub(crate) enum CandidateResult {
    Ungrounded,
    Recorded(Result<RecordOutcome, RecordError>),
}
#[derive(Debug)]
#[must_use]
pub(crate) struct EvidenceWrite {
    pub(crate) detached: bool,
    pub(crate) candidates: Vec<CandidateResult>,
    pub(crate) evaluation_scheduled: bool,
}
#[must_use]
pub(crate) struct CapturedEndingEvidence {
    ending: CapturedEnding,
    writer: CapturedWriter,
}
struct CancelOnDrop(Arc<FileClaim>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
impl CapturedEnding {
    /// Compose before inference. Registration never refreshes the original snapshot.
    pub(crate) async fn with_workflow_evidence(self) -> Result<CapturedEndingEvidence, StoreError> {
        let owner = self.owner.clone();
        let store = self.workflow.clone();
        let instance = store.instance_id();
        let current = owner
            .lifecycle_access(
                self.id.clone(),
                self.runtime.clone(),
                Some(self.snapshot.clone()),
                None,
                Access::Async,
                move |ws, _| ws.workflow_observations.instance_id() == instance,
            )
            .await?
            .unwrap_or(false);
        if !current {
            return Err(StoreError::Conflict);
        }
        let id = self.id.clone();
        let created = self.snapshot.created.clone();
        let capacity = budget::reserve(budget::intent_bytes(&(&id, &created))?)?;
        let lease = self.lease.clone();
        let registration = tokio::task::spawn_blocking(move || {
            let _capacity = capacity;
            let _lease = lease;
            store.capture_writer(id, created)
        });
        // Timed-out registration is memory-only; its retained job drains safely.
        let writer = tokio::time::timeout(std::time::Duration::from_secs(2), registration)
            .await
            .map_err(|_| StoreError::Expired)?
            .map_err(|_| StoreError::Unavailable)?;
        self.validate_evidence_record().await?;
        let instance = writer.instance_id();
        let current = owner
            .lifecycle_access(
                self.id.clone(),
                self.runtime.clone(),
                Some(self.snapshot.clone()),
                None,
                Access::Async,
                move |ws, _| ws.workflow_observations.instance_id() == instance,
            )
            .await?
            .unwrap_or(false);
        if !current {
            return Err(StoreError::Conflict);
        }
        Ok(CapturedEndingEvidence {
            ending: self,
            writer,
        })
    }
    async fn validate_evidence_record(&self) -> Result<(), StoreError> {
        self.read_evidence_record().await.map(|_| ())
    }
    async fn read_evidence_record(&self) -> Result<metadata::SessionMetadata, StoreError> {
        let record = self
            .owner
            .store
            .get(self.id.clone())
            .await?
            .ok_or(StoreError::Missing)?;
        let stored: metadata::SessionMetadata = serde_json::from_value(record)?;
        if stored.id != self.id
            || stored.created_at != self.snapshot.created
            || stored.lifecycle_phase != "ending"
            || self
                .snapshot
                .harness
                .as_deref()
                .is_some_and(|h| h != stored.harness)
        {
            return Err(StoreError::Conflict);
        }
        Ok(stored)
    }
}
impl CapturedEndingEvidence {
    pub(crate) fn run_generation(&self) -> u64 {
        self.ending.runtime.run_generation
    }

    /// Authoritative fallback data, without renewing the original authority snapshot.
    pub(crate) async fn ending_metadata(&self) -> Result<metadata::SessionMetadata, StoreError> {
        let metadata = self.ending.read_evidence_record().await?;
        let instance = self.writer.instance_id();
        let current = self
            .ending
            .owner
            .lifecycle_access(
                self.ending.id.clone(),
                self.ending.runtime.clone(),
                Some(self.ending.snapshot.clone()),
                None,
                Access::Async,
                move |ws, _| ws.workflow_observations.instance_id() == instance,
            )
            .await?
            .unwrap_or(false);
        if !current {
            return Err(StoreError::Conflict);
        }
        Ok(metadata)
    }
    /// Capture bounded physical output under original memory fences; rejoin off-guard.
    pub(crate) async fn final_scan_output(&self) -> Result<Vec<String>, StoreError> {
        let instance = self.writer.instance_id();
        let (output, cols) = self
            .ending
            .owner
            .lifecycle_access(
                self.ending.id.clone(),
                self.ending.runtime.clone(),
                Some(self.ending.snapshot.clone()),
                None,
                Access::Async,
                move |ws, h| {
                    (ws.workflow_observations.instance_id() == instance)
                        .then(|| (h.output_buffer.snapshot(), h.runtime.last_cols))
                },
            )
            .await?
            .flatten()
            .ok_or(StoreError::Conflict)?;
        tokio::task::spawn_blocking(move || peon::rejoin_hard_wrapped_lines(&output, cols))
            .await
            .map_err(|_| StoreError::Unavailable)
    }
    pub(crate) async fn persist_provider_context(
        &self,
        provider: crate::providers::ProviderObservation,
    ) -> Result<ProviderWrite, StoreError> {
        self.ending.persist_provider_context(provider).await
    }
    pub(crate) async fn complete(
        self,
        snapshot: metadata::ObservedStatusSnapshotMetadata,
        fallback: String,
    ) -> Result<Finalization, StoreError> {
        self.ending.complete(snapshot, fallback).await
    }
    pub(crate) async fn persist_workflow_observations(
        &self,
        generation: u64,
        candidates: Vec<peon::PeonWorkflowObservation>,
        output: Vec<String>,
    ) -> Result<EvidenceWrite, StoreError> {
        if candidates.len() > 5 {
            return Err(StoreError::TooLarge);
        }
        let bytes = budget::intent_bytes(&(
            &self.ending.id,
            &self.ending.snapshot.created,
            generation,
            &candidates,
            &output,
        ))?;
        let capacity = budget::reserve(bytes)?;
        let claim = Arc::new(FileClaim::new());
        let _cancel = CancelOnDrop(claim.clone());
        let lane = self.ending.owner.store.runtime_lane(&self.ending.id)?;
        let ending = CapturedEnding {
            owner: self.ending.owner.clone(),
            id: self.ending.id.clone(),
            runtime: self.ending.runtime.clone(),
            snapshot: self.ending.snapshot.clone(),
            lease: self.ending.lease.clone(),
            root: self.ending.root.clone(),
            workflow: self.ending.workflow.clone(),
        };
        let writer = self.writer.clone();
        let handle = tokio::runtime::Handle::current();
        let job_claim = claim.clone();
        let (result, mut received) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let _capacity = capacity;
            let execution = (|| {
                let _lane = handle.block_on(async {
                    tokio::time::timeout_at(job_claim.deadline.into(), lane.lock_owned())
                        .await
                        .map_err(|_| StoreError::Expired)
                })?;
                handle.block_on(ending.validate_evidence_record())?;
                let instance = writer.instance_id();
                let current = ending
                    .owner
                    .lifecycle_current(
                        &ending.id,
                        &ending.runtime,
                        Some(&ending.snapshot),
                        |ws, _| ws.workflow_observations.instance_id() == instance,
                    )
                    .unwrap_or(false);
                if !current {
                    return Err(StoreError::Conflict);
                }
                let mut results: Vec<_> = (0..candidates.len())
                    .map(|_| CandidateResult::Ungrounded)
                    .collect();
                let grounded = candidates
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, c)| {
                        if !peon::evidence_is_grounded(&c.evidence, &output) {
                            return None;
                        }
                        let candidate = crate::workflow_observations::ObservationCandidate {
                            kind: c.kind,
                            description: c.description,
                            evidence: c.evidence,
                            problem_area: c.problem_area,
                            reported_impact: c.reported_impact,
                            confidence: Some(c.confidence),
                        };
                        if let Err(error) = candidate.validate_peon() {
                            results[index] = CandidateResult::Recorded(Err(error));
                            return None;
                        }
                        Some((index, candidate))
                    })
                    .collect();
                let recorded = writer.record_batch(&job_claim, generation, grounded)?;
                let accepted = recorded
                    .iter()
                    .any(|(_, r)| matches!(r, Ok(RecordOutcome::Accepted(_))));
                for (index, record) in recorded {
                    results[index] = CandidateResult::Recorded(record);
                }
                let current = ending
                    .owner
                    .lifecycle_current(
                        &ending.id,
                        &ending.runtime,
                        Some(&ending.snapshot),
                        |ws, _| ws.workflow_observations.instance_id() == instance,
                    )
                    .unwrap_or(false);
                let scheduled = current
                    && accepted
                    && crate::taskmaster::evaluator::schedule_captured_evaluation(
                        ending.owner.state.clone(),
                        instance,
                        &ending.lease,
                    );
                Ok(EvidenceWrite {
                    detached: !current,
                    candidates: results,
                    evaluation_scheduled: scheduled,
                })
            })();
            if let Err(error) = &execution {
                tracing::warn!(session_id=%ending.id,%error,"captured workflow evidence was not admitted");
            }
            if let Ok(outcome) = &execution {
                for (index, candidate) in outcome.candidates.iter().enumerate() {
                    if let CandidateResult::Recorded(Err(error)) = candidate {
                        tracing::warn!(session_id=%ending.id,index,%error,"captured workflow evidence persistence is incomplete");
                    }
                }
            }
            if let Err(Ok(receipt)) = result.send(execution) {
                let accepted = receipt
                    .candidates
                    .iter()
                    .filter(|r| {
                        matches!(r, CandidateResult::Recorded(Ok(RecordOutcome::Accepted(_))))
                    })
                    .count();
                let duplicates = receipt
                    .candidates
                    .iter()
                    .filter(|r| {
                        matches!(
                            r,
                            CandidateResult::Recorded(Ok(RecordOutcome::Duplicate { .. }))
                        )
                    })
                    .count();
                let failed = receipt
                    .candidates
                    .iter()
                    .filter(|r| matches!(r, CandidateResult::Recorded(Err(_))))
                    .count();
                tracing::info!(session_id=%ending.id, store_instance=writer.instance_id(),
                    detached=receipt.detached, evaluation_scheduled=receipt.evaluation_scheduled,
                    accepted, duplicates, failed,
                    "captured workflow evidence completed without a caller");
            }
        });
        tokio::select! {
            biased;
            result=&mut received=>result.map_err(|_|StoreError::Unavailable)?,
            _=tokio::time::sleep_until(claim.deadline.into())=> {
                if claim.expire() { Err(StoreError::Expired) } else { received.await.map_err(|_|StoreError::Unavailable)? }
            }
        }
    }
}
