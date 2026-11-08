//! Compound finalization owner. External files never run on the SQL worker.
use super::*;
use crate::session_application::ending_completion::WriteResult;

#[derive(Debug, Default)]
pub(crate) enum MetadataResult {
    #[default]
    NotAttempted,
    Completed(Outcome),
    Failed(StoreError),
}
#[derive(Debug, Default)]
pub(crate) struct Finalization {
    pub(crate) replay: WriteResult,
    pub(crate) metadata: MetadataResult,
    pub(crate) event: WriteResult,
}
impl Finalization {
    fn log_failures(&self, id: &str) {
        for (stage, result) in [("replay", &self.replay), ("event", &self.event)] {
            if let WriteResult::Failed(error) = result {
                tracing::warn!(session_id = id, stage, %error, metadata = ?self.metadata,
                    "SQL finalization history is incomplete");
            }
        }
        if let MetadataResult::Failed(error) = &self.metadata {
            tracing::warn!(session_id = id, %error, replay = ?self.replay,
                "SQL finalization metadata is incomplete");
        }
    }
}
/// Immutable authority captured before a runtime's final-scan wait.
/// Dropping it before completion releases the lease without any file/SQL effect.
#[must_use]
pub(crate) struct CapturedEnding {
    owner: Arc<SessionWrites>,
    id: String,
    runtime: RuntimeIdentity,
    snapshot: Arc<Snapshot>,
    lease: Arc<crate::workspace_runtime::WorkspaceLease>,
    root: std::path::PathBuf,
    workflow: crate::workflow_observations::WorkflowObservationStore,
}
impl SessionWrites {
    pub(crate) async fn capture_ending_history(
        self: &Arc<Self>,
        state: &Arc<AppState>,
        id: String,
        runtime: RuntimeIdentity,
    ) -> Result<CapturedEnding, StoreError> {
        if !Arc::ptr_eq(state, &self.state) {
            return Err(StoreError::Conflict);
        }
        let (snapshot, lease, root, workflow) = self
            .lifecycle_access(
                id.clone(),
                runtime.clone(),
                None,
                None,
                Access::Async,
                move |ws, h| {
                    if h.info.lifecycle_phase != "ending" {
                        return Err(StoreError::Conflict);
                    }
                    Ok::<_, StoreError>((
                        Snapshot::capture(h).ok_or(StoreError::Conflict)?,
                        ws.lease.clone().ok_or(StoreError::Conflict)?,
                        ws.metadata.root_path(),
                        ws.workflow_observations.clone(),
                    ))
                },
            )
            .await?
            .ok_or(StoreError::Conflict)??;
        Ok(CapturedEnding {
            owner: self.clone(),
            id,
            runtime,
            snapshot: Arc::new(snapshot),
            lease,
            root,
            workflow,
        })
    }
    /// Direct source-only callers capture and immediately consume the same owner.
    pub(crate) async fn finalize_with_history(
        self: &Arc<Self>,
        id: String,
        runtime: RuntimeIdentity,
        final_snapshot: metadata::ObservedStatusSnapshotMetadata,
        fallback_status: String,
    ) -> Result<Finalization, StoreError> {
        self.capture_ending_history(&self.state, id, runtime)
            .await?
            .complete(final_snapshot, fallback_status)
            .await
    }
}
impl CapturedEnding {
    /// Consume original authority; completion never captures a replacement owner.
    pub(crate) async fn complete(
        self,
        final_snapshot: metadata::ObservedStatusSnapshotMetadata,
        fallback_status: String,
    ) -> Result<Finalization, StoreError> {
        let Self {
            owner,
            id,
            runtime,
            snapshot,
            lease,
            root,
            workflow: _workflow,
        } = self;
        let intent = LifecycleMutation::Finalize {
            snapshot: final_snapshot,
            fallback_status,
            timestamp: iso_now(),
        };
        if !intent.valid() {
            return Err(StoreError::Invalid("invalid lifecycle intent".into()));
        }
        let prepared = owner.store.prepare_internal(
            id.clone(),
            snapshot.created.clone(),
            InternalMutation::Lifecycle(intent),
        )?;
        // Caller cancellation competes with the existing SQL execution claim.
        // The retained file job may already have confirmed a grid at that point.
        let _cancellation = prepared.cancellation();
        let lane = prepared.lane()?;
        let handle = tokio::runtime::Handle::current();
        let (mut result, received) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let execution = (|| {
                let lane = handle.block_on(async {
                    tokio::select! {
                        biased;
                        _ = result.closed() => Err(StoreError::Cancelled),
                        lane = tokio::time::timeout_at(prepared.deadline().into(), lane.lock_owned()) =>
                            lane.map_err(|_| StoreError::Expired),
                    }
                })?;
                // Validate authoritative SQL identity before touching history. Never read
                // legacy session JSON, including when SQL has no matching record.
                let record = handle
                    .block_on(owner.store.get(id.clone()))?
                    .ok_or(StoreError::Missing)?;
                let stored: metadata::SessionMetadata = serde_json::from_value(record)?;
                if stored.created_at != snapshot.created
                    || stored.id != id
                    || stored.lifecycle_phase != "ending"
                    || snapshot
                        .harness
                        .as_deref()
                        .is_some_and(|kind| kind != stored.harness)
                {
                    return Err(StoreError::Conflict);
                }
                #[cfg(test)]
                crate::session_application::ending_completion::testing::checkpoint(
                    &owner.state,
                    crate::session_application::ending_completion::testing::Stage::BeforeHistory,
                );
                // This guard stays on this blocking thread across worker waits. No
                // application guards or Tokio workers span file I/O or SQL waits.
                let _grid = TERMINAL_SIZE_WRITE_LOCK.lock().unwrap();
                if result.is_closed() {
                    return Err(StoreError::Cancelled);
                }
                if std::time::Instant::now() >= prepared.deadline() {
                    return Err(StoreError::Expired);
                }
                let size = owner
                    .lifecycle_current(&id, &runtime, Some(&snapshot), |_, h| {
                        (h.runtime.last_cols, h.runtime.last_rows)
                    })
                    .ok_or(StoreError::Conflict)?;
                // External-file-only handle, bound to the captured root and retained
                // original lease. Its session JSON API is never used here.
                let files = metadata::MetadataStore::new(&root);
                let mut outcome = Finalization::default();
                let grid = files
                    .try_write_terminal_size(&id, size.0, size.1)
                    .and_then(|()| {
                        (files.read_terminal_size(&id) == Some(size))
                            .then_some(())
                            .ok_or_else(|| {
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "final terminal grid could not be confirmed",
                                )
                            })
                    });
                outcome.replay = match grid {
                    Ok(()) => WriteResult::Written,
                    Err(error) => {
                        outcome.replay = WriteResult::Failed(error);
                        return Ok(outcome);
                    }
                };
                #[cfg(test)]
                crate::session_application::ending_completion::testing::checkpoint(
                    &owner.state,
                    crate::session_application::ending_completion::testing::Stage::BeforeMetadata,
                );
                let completion = match handle.block_on(owner.execute_lifecycle(
                    &id,
                    &runtime,
                    &snapshot,
                    prepared,
                    RuntimeWriteResources::new(lane, Some(lease)),
                    &mut result,
                    Access::BlockingOwner,
                )) {
                    Ok(completion) => completion,
                    Err(error) => {
                        outcome.metadata = MetadataResult::Failed(error);
                        return Ok(outcome);
                    }
                };
                let Some(document) = completion.outcome.document.as_ref() else {
                    outcome.metadata = MetadataResult::Completed(Outcome::Noop {
                        permanent_hold: false,
                    });
                    return Ok(outcome);
                };
                // Retain the definitive completion (lane, lease, byte reservation)
                // through event append and original live publication.
                let stored: metadata::SessionMetadata =
                    match serde_json::from_value(document.clone()) {
                        Ok(stored) => stored,
                        Err(error) => {
                            outcome.metadata =
                                MetadataResult::Completed(Outcome::CommittedDetached);
                            outcome.event = WriteResult::Failed(std::io::Error::other(error));
                            return Ok(outcome);
                        }
                    };
                #[cfg(test)]
                crate::session_application::ending_completion::testing::checkpoint(
                    &owner.state,
                    crate::session_application::ending_completion::testing::Stage::BeforeEvent,
                );
                let final_snapshot = stored.final_observed_status_snapshot.as_ref();
                outcome.event = match files.try_append_event(
                    &id,
                    &metadata::Event {
                        event_type: "session.status".into(),
                        timestamp: stored.last_activity.clone(),
                        status: stored.status.clone(),
                        observed_status: final_snapshot.and_then(|s| s.value.clone()),
                        confidence: final_snapshot.and_then(|s| s.confidence),
                        summary: None,
                        source: None,
                        recommendation_id: None,
                    },
                ) {
                    Ok(()) => WriteResult::Written,
                    Err(error) => WriteResult::Failed(error),
                };
                #[cfg(test)]
                crate::session_application::ending_completion::testing::checkpoint(
                    &owner.state,
                    crate::session_application::ending_completion::testing::Stage::BeforePublish,
                );
                outcome.metadata =
                    MetadataResult::Completed(handle.block_on(owner.publish_lifecycle(
                        id.clone(),
                        runtime,
                        snapshot,
                        completion,
                        Access::BlockingOwner,
                    ))?);
                Ok(outcome)
            })();
            match &execution {
                Ok(outcome) => outcome.log_failures(&id),
                Err(error) => {
                    tracing::warn!(session_id = id, %error, "SQL finalization did not begin history effects")
                }
            }
            let _ = result.send(execution);
        });
        received.await.map_err(|_| StoreError::Unavailable)?
    }
}

mod provider;
pub(crate) use provider::ProviderWrite;

mod evidence;

#[cfg(test)]
pub(crate) use evidence::CandidateResult;
pub(crate) use evidence::{CapturedEndingEvidence, EvidenceWrite};

mod sql_origin;

#[cfg(test)]
pub(crate) use sql_origin::{hold_observation, ObservationStage};
