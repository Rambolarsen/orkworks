//! The original captured owner is the only producer of an admitted SQL file batch.
use super::*;
use crate::workflow_observations::{sql_origin::WriteError, ObservationCandidate, RecordOutcome};
#[derive(Debug)]
pub(crate) enum SqlCandidateResult {
    Ungrounded,
    Recorded(Result<RecordOutcome, WriteError>),
}
#[derive(Debug)]
#[must_use]
pub(crate) struct SqlEvidenceWrite {
    pub(crate) detached: bool,
    pub(crate) candidates: Vec<SqlCandidateResult>,
}
impl CapturedSqlObservation {
    pub(crate) async fn persist_workflow_observations(
        &self,
        generation: u64,
        candidates: Vec<peon::PeonWorkflowObservation>,
        output: Vec<String>,
    ) -> Result<SqlEvidenceWrite, StoreError> {
        if candidates.len() > 5 {
            return Err(StoreError::TooLarge);
        }
        // Bound escaped source, input and grounding output before their queued
        // ownership or normalization. The file phase keeps this same full slot.
        let claim = Arc::new(FileClaim::new());
        let _cancel = CancelOnDrop(claim.clone());
        crate::session_application::sql_writes::file_budget::intent_bytes(&(
            &self.source,
            generation,
            &candidates,
            &output,
        ))?;
        let scan = ScanRequest::capture(claim.deadline)?;
        let capacity = scan.hold_capacity();
        let ending = CapturedEnding {
            owner: self.ending.owner.clone(),
            id: self.ending.id.clone(),
            runtime: self.ending.runtime.clone(),
            snapshot: self.ending.snapshot.clone(),
            lease: self.ending.lease.clone(),
            root: self.ending.root.clone(),
            workflow: self.ending.workflow.clone(),
        };
        let root_identity = self.root_identity.clone();
        let source = self.source.clone();
        let revocation = self.revocation.clone();
        let lane = ending.owner.store.runtime_lane(&ending.id)?;
        let handle = tokio::runtime::Handle::current();
        let job_claim = claim.clone();
        let (sent, mut received) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let _capacity = capacity;
            let mut lane_guard = None;
            let mut writer = None;
            let execute = (|| {
                let mut results: Vec<_> = (0..candidates.len())
                    .map(|_| SqlCandidateResult::Ungrounded)
                    .collect();
                let grounded = candidates
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, c)| {
                        if !peon::evidence_is_grounded(&c.evidence, &output) {
                            return None;
                        }
                        let candidate = ObservationCandidate {
                            kind: c.kind,
                            description: c.description,
                            evidence: c.evidence,
                            problem_area: c.problem_area,
                            reported_impact: c.reported_impact,
                            confidence: Some(c.confidence),
                        };
                        Some((index, candidate))
                    })
                    .collect();
                drop(output);
                lane_guard = Some(handle.block_on(async {
                    tokio::time::timeout_at(job_claim.deadline.into(), lane.lock_owned())
                        .await
                        .map_err(|_| StoreError::Expired)
                })?);
                job_claim.pending()?;
                if sent.is_closed() {
                    return Err(StoreError::Cancelled);
                }
                validate_root_lease(&root_identity, &ending.lease)?;
                let mut candidate = Some(ending.workflow.sql_candidate(&ending.lease)?);
                #[cfg(test)]
                enter_replay_gate(&ending.owner.observation_before_admit);
                handle.block_on(async {
                    tokio::time::timeout_at(
                        job_claim.deadline.into(),
                        ending.owner.store.validate_observation_ending(
                            source.clone(),
                            ending.snapshot.harness.clone(),
                        ),
                    )
                    .await
                    .map_err(|_| StoreError::Expired)?
                })?;
                validate_root_lease(&root_identity, &ending.lease)?;
                candidate
                    .as_mut()
                    .unwrap()
                    .revalidate(root_identity.canonical_path(), &ending.lease)?;
                let instance = ending.workflow.instance_id();
                loop {
                    job_claim.pending()?;
                    if sent.is_closed() {
                        return Err(StoreError::Cancelled);
                    }
                    let admitted = ending.owner.observation_try_current(
                        &ending.id,
                        &ending.runtime,
                        Some(&ending.snapshot),
                        |ws, _| {
                            if ws.workflow_observations.instance_id() != instance
                                || ws.metadata.root_path() != ending.root
                                || !ws
                                    .lease
                                    .as_ref()
                                    .is_some_and(|lease| Arc::ptr_eq(lease, &ending.lease))
                            {
                                return Err(StoreError::Conflict);
                            }
                            candidate.take().ok_or(StoreError::Conflict)?.admit_write(
                                ObservationWriteAdmission {
                                    source: source.clone(),
                                    revocation: revocation.clone(),
                                },
                                &job_claim,
                            )
                        },
                    );
                    match admitted {
                        Ok(Some(result)) => {
                            writer = Some(result?);
                            break;
                        }
                        Ok(None) => return Err(StoreError::Conflict),
                        Err(StoreError::Busy) => {
                            std::thread::sleep(std::time::Duration::from_millis(2))
                        }
                        Err(e) => return Err(e),
                    }
                }
                #[cfg(test)]
                enter_replay_gate(&ending.owner.observation_admitted);
                #[cfg(test)]
                {
                    let gate = ending
                        .owner
                        .observation_after_counter
                        .lock()
                        .unwrap()
                        .take();
                    if let Some((entered, release)) = gate {
                        crate::workflow_observations::sql_origin::test_after_counter(move || {
                            let _ = entered.send(());
                            let _ = release.recv();
                        });
                    }
                }
                let recorded = writer
                    .as_mut()
                    .unwrap()
                    .persist_batch(scan, generation, grounded);
                #[cfg(test)]
                crate::workflow_observations::sql_origin::test_after_counter(|| {});
                let recorded = recorded.map_err(|e| match e {
                    WriteError::Storage(e) => e,
                    WriteError::Candidate(_) => StoreError::Invalid("invalid batch".into()),
                })?;
                for (index, result) in recorded {
                    results[index] = SqlCandidateResult::Recorded(result);
                }
                // A busy or replaced live view cannot renew scope or undo committed
                // receipts. Publish them only as detached, without evaluator work.
                let current = ending
                    .owner
                    .observation_try_current(
                        &ending.id,
                        &ending.runtime,
                        Some(&ending.snapshot),
                        |ws, _| {
                            ws.workflow_observations.instance_id() == instance
                                && ws.metadata.root_path() == ending.root
                                && ws
                                    .lease
                                    .as_ref()
                                    .is_some_and(|lease| Arc::ptr_eq(lease, &ending.lease))
                        },
                    )
                    .ok()
                    .flatten()
                    .unwrap_or(false);
                Ok(SqlEvidenceWrite {
                    detached: !current,
                    candidates: results,
                })
            })();
            match &execute {
                Err(error) => {
                    tracing::warn!(session_id=%ending.id,%error,"captured SQL observation batch refused")
                }
                Ok(receipt) => {
                    for (index, result) in receipt.candidates.iter().enumerate() {
                        if let SqlCandidateResult::Recorded(Err(error)) = result {
                            tracing::warn!(session_id=%ending.id,index,?error,"captured SQL observation candidate incomplete");
                        }
                    }
                }
            }
            if let Err(Ok(receipt)) = sent.send(execute) {
                let accepted = receipt
                    .candidates
                    .iter()
                    .filter(|r| {
                        matches!(
                            r,
                            SqlCandidateResult::Recorded(Ok(RecordOutcome::Accepted(_)))
                        )
                    })
                    .count();
                let duplicates = receipt
                    .candidates
                    .iter()
                    .filter(|r| {
                        matches!(
                            r,
                            SqlCandidateResult::Recorded(Ok(RecordOutcome::Duplicate { .. }))
                        )
                    })
                    .count();
                let failed = receipt
                    .candidates
                    .iter()
                    .filter(|r| matches!(r, SqlCandidateResult::Recorded(Err(_))))
                    .count();
                tracing::info!(session_id=%ending.id,database_epoch=source.database_epoch(),created_at=source.created_at(),generation,
                    detached=receipt.detached,accepted,duplicates,failed,"captured SQL observation batch settled after caller loss");
            }
            // Original lane, worker, lease, file owner and slot span result delivery.
            drop(writer);
            drop(lane_guard);
            drop(ending);
        });
        tokio::select! {biased;
            result=&mut received=>result.map_err(|_|StoreError::Unavailable)?,
            _=tokio::time::sleep_until(claim.deadline.into())=>{
                if claim.expire(){Err(StoreError::Expired)}else{received.await.map_err(|_|StoreError::Unavailable)?}
            }
        }
    }
}
