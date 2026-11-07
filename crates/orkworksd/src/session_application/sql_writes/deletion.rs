//! Captured external cleanup stages retain a tombstone on every partial failure.
use super::*;
use crate::session_application::sql_writes::file_budget;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy)]
enum Stage {
    Replay,
    Workflow,
    Images,
    Settings,
    Recommendations,
}
impl Stage {
    fn bit(self) -> u8 {
        match self {
            Self::Replay => 0x01,
            Self::Workflow => 0x02,
            Self::Images => 0x04,
            Self::Settings => 0x08,
            Self::Recommendations => 0x10,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Replay => "replay",
            Self::Workflow => "workflow",
            Self::Images => "images",
            Self::Settings => "settings",
            Self::Recommendations => "recommendations",
        }
    }
    fn current(
        self,
        captured: &crate::workflow_observations::WorkflowObservationStore,
        recommendations: &crate::taskmaster::store::RecommendationStore,
        ws: &WorkspaceState,
    ) -> bool {
        match self {
            Self::Workflow => captured.instance_id() == ws.workflow_observations.instance_id(),
            Self::Recommendations => recommendations.same_owner(&ws.recommendation_store),
            Self::Replay | Self::Images | Self::Settings => true,
        }
    }
}
enum Files {
    Replay(Arc<std::io::Result<crate::metadata::replay_cleanup::AnchoredReplayCleanup>>),
    Workflow(crate::workflow_observations::WorkflowObservationStore),
    Images(Arc<std::io::Result<crate::image_context::SqlImageCleanup>>),
    Settings(Arc<std::io::Result<crate::metadata::BoundSettingsCleanup>>),
    Recommendations(
        Arc<Result<crate::taskmaster::store::BoundCleanup, crate::taskmaster::store::StoreError>>,
    ),
}
#[derive(Default)]
struct CleanupGuards {
    _image: Option<crate::image_context::ImageCleanupGuard>,
    _settings: Option<crate::metadata::WorkspaceMemoryEdit>,
    _recommendations: Option<crate::taskmaster::store::JournalReceipt>,
}
impl Files {
    fn remove(
        self,
        id: &str,
        identity: &crate::session_store::SqlDeletionIdentity,
    ) -> (std::io::Result<()>, CleanupGuards) {
        match self {
            Self::Replay(files) => (
                files
                    .as_ref()
                    .as_ref()
                    .map_err(|e| std::io::Error::new(e.kind(), e.to_string()))
                    .and_then(|files| files.remove()),
                CleanupGuards::default(),
            ),
            Self::Workflow(files) => (
                files.delete_session_observations_durable(id),
                CleanupGuards::default(),
            ),
            Self::Images(files) => {
                let (result, guard) = match files.as_ref().as_ref() {
                    Ok(files) => files.remove(),
                    Err(e) => (Err(std::io::Error::new(e.kind(), e.to_string())), None),
                };
                (
                    result,
                    CleanupGuards {
                        _image: guard,
                        ..Default::default()
                    },
                )
            }
            Self::Recommendations(files) => {
                let receipt = files
                    .as_ref()
                    .as_ref()
                    .map_err(recommendation_error)
                    .and_then(|files| {
                        files
                            .remove_for_sql(identity)
                            .map_err(|e| recommendation_error(&e))
                    });
                match receipt {
                    Ok(receipt) => {
                        let result = receipt
                            .result()
                            .as_ref()
                            .map(|_| ())
                            .map_err(recommendation_error);
                        (
                            result,
                            CleanupGuards {
                                _recommendations: Some(receipt),
                                ..Default::default()
                            },
                        )
                    }
                    Err(error) => (Err(error), CleanupGuards::default()),
                }
            }
            Self::Settings(files) => {
                let (result, guard) = match files.as_ref().as_ref() {
                    Ok(files) => files.clear(id),
                    Err(e) => (Err(std::io::Error::new(e.kind(), e.to_string())), None),
                };
                (
                    result,
                    CleanupGuards {
                        _settings: guard,
                        ..Default::default()
                    },
                )
            }
        }
    }
}

fn recommendation_error(error: &crate::taskmaster::store::StoreError) -> std::io::Error {
    use crate::taskmaster::store::StoreError as Error;
    let kind = match error {
        Error::Busy => std::io::ErrorKind::WouldBlock,
        Error::Io(error) => error.kind(),
        _ => std::io::ErrorKind::InvalidData,
    };
    std::io::Error::new(kind, error.to_string())
}

pub(crate) struct CapturedDeletion {
    owner: Arc<SessionWrites>,
    id: String,
    created: String,
    identity: crate::session_store::SqlDeletionIdentity,
    memory: Arc<Memory>,
    workflow: crate::workflow_observations::WorkflowObservationStore,
    replay: Arc<std::io::Result<crate::metadata::replay_cleanup::AnchoredReplayCleanup>>,
    capacity: file_budget::Reservation,
    images: Arc<std::io::Result<crate::image_context::SqlImageCleanup>>,
    settings: Arc<std::io::Result<crate::metadata::BoundSettingsCleanup>>,
    recommendation_store: crate::taskmaster::store::RecommendationStore,
    recommendations:
        Arc<Result<crate::taskmaster::store::BoundCleanup, crate::taskmaster::store::StoreError>>,
}
#[derive(Debug)]
#[must_use]
pub(crate) struct DeletionCleanupResult {
    pub(crate) files: Result<(), std::io::Error>,
    pub(crate) progress: Option<Result<u8, StoreError>>,
    pub(crate) detached: bool,
}
struct Claim {
    state: AtomicU8,
    deadline: Instant,
}
impl Claim {
    fn cancel(&self) -> bool {
        self.state
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
    fn admit(&self) -> Result<(), StoreError> {
        if Instant::now() >= self.deadline {
            self.cancel();
            return Err(StoreError::Expired);
        }
        self.state
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| StoreError::Cancelled)
    }
}
struct CancelOnDrop(Arc<Claim>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
impl SessionWrites {
    pub(crate) async fn capture_deletion(
        self: &Arc<Self>,
        id: String,
        created: String,
    ) -> Result<CapturedDeletion, StoreError> {
        let id_for_capture = id.clone();
        let (memory, workflow, replay, image_root, settings, recommendation_store) = self
            .organization_access(id.clone(), None, true, None, move |ws, h, pid| {
                (
                    Arc::new(Memory::capture(ws, h.as_deref(), pid)),
                    ws.workflow_observations.clone(),
                    ws.metadata.capture_replay_cleanup(&id_for_capture),
                    ws.metadata.root_path(),
                    ws.metadata.capture_settings_cleanup(),
                    ws.recommendation_store.clone(),
                )
            })
            .await?
            .ok_or(StoreError::Conflict)?;
        if !memory.closed
            || memory
                .runtime
                .as_ref()
                .is_some_and(|r| r.created != created)
        {
            return Err(StoreError::Conflict);
        }
        let identity = self
            .store
            .deletion_record(id.clone(), created.clone())
            .await?
            .ok_or(StoreError::Missing)?
            .identity;
        let capacity =
            file_budget::reserve(file_budget::intent_bytes(&(&id, &created, &identity))?)?;
        let lease = memory._lease.clone().ok_or(StoreError::Conflict)?;
        #[cfg(test)]
        let binding_owner = self.clone();
        let image_id = id.clone();
        let recommendation_intent = recommendation_store.capture_cleanup();
        let binding = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            enter_replay_gate(&binding_owner.replay_binding);
            (
                capacity,
                Arc::new(replay.and_then(|intent| intent.bind(&lease))),
                Arc::new(crate::image_context::bind_sql_cleanup(
                    &image_root,
                    &image_id,
                    &lease,
                )),
                Arc::new(settings.bind(&lease)),
                Arc::new(recommendation_intent.bind(lease.clone())),
            )
        });
        let (capacity, replay, images, settings, recommendations) =
            tokio::time::timeout(Duration::from_secs(2), binding)
                .await
                .map_err(|_| StoreError::Expired)?
                .map_err(|_| StoreError::Unavailable)?;
        self.organization_access(id.clone(), Some(memory.clone()), true, None, |_, _, _| ())
            .await?
            .ok_or(StoreError::Conflict)?;
        Ok(CapturedDeletion {
            owner: self.clone(),
            id,
            created,
            identity,
            memory,
            workflow,
            replay,
            capacity,
            images,
            settings,
            recommendation_store,
            recommendations,
        })
    }
}
impl CapturedDeletion {
    pub(crate) async fn remove_replay(self) -> Result<DeletionCleanupResult, StoreError> {
        self.remove(Stage::Replay).await
    }
    pub(crate) async fn remove_workflow_evidence(
        self,
    ) -> Result<DeletionCleanupResult, StoreError> {
        self.remove(Stage::Workflow).await
    }
    pub(crate) async fn remove_images(self) -> Result<DeletionCleanupResult, StoreError> {
        self.remove(Stage::Images).await
    }
    pub(crate) async fn clear_settings(self) -> Result<DeletionCleanupResult, StoreError> {
        self.remove(Stage::Settings).await
    }
    // Source-only: producer fencing, startup routing and finalization remain
    // activation gates. This method never completes the deletion job itself.
    pub(crate) async fn remove_recommendations(self) -> Result<DeletionCleanupResult, StoreError> {
        self.remove(Stage::Recommendations).await
    }
    async fn remove(self, stage: Stage) -> Result<DeletionCleanupResult, StoreError> {
        let claim = Arc::new(Claim {
            state: AtomicU8::new(0),
            deadline: Instant::now() + Duration::from_secs(2),
        });
        let _cancel = CancelOnDrop(claim.clone());
        let lane = self.owner.store.runtime_lane(&self.id)?;
        let handle = tokio::runtime::Handle::current();
        let job_claim = claim.clone();
        let (sent, mut received) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let _capacity = self.capacity;
            let result = (|| {
                let _lane = handle
                    .block_on(tokio::time::timeout_at(
                        job_claim.deadline.into(),
                        lane.lock_owned(),
                    ))
                    .map_err(|_| StoreError::Expired)?;
                let record = handle
                    .block_on(
                        self.owner
                            .store
                            .deletion_record(self.id.clone(), self.created.clone()),
                    )?
                    .ok_or(StoreError::Missing)?;
                // A capture cannot move to a different database incarnation.
                // Identity is reread under the original lane before admission;
                // it supplies identification, never runtime/lease authority.
                if matches!(stage, Stage::Recommendations) && record.identity != self.identity {
                    return Err(StoreError::Conflict);
                }
                #[cfg(test)]
                enter_replay_gate(&self.owner.replay_before);
                let (_reservation, _lease, files) = self
                    .owner
                    .organization_current(&self.id, Some(&self.memory), true, |ws, h, pid| {
                        if !session_actions::runtime_closed(h.as_deref(), pid)
                            || !stage.current(&self.workflow, &self.recommendation_store, ws)
                        {
                            return Err(StoreError::Conflict);
                        }
                        let lease = ws.lease.clone().ok_or(StoreError::Conflict)?;
                        let reservation = ws
                            .metadata
                            .organization
                            .reserve(&self.id)
                            .ok_or(StoreError::Conflict)?;
                        let files = match stage {
                            Stage::Replay => Files::Replay(self.replay.clone()),
                            Stage::Workflow => Files::Workflow(self.workflow.clone()),
                            Stage::Images => Files::Images(self.images.clone()),
                            Stage::Settings => Files::Settings(self.settings.clone()),
                            Stage::Recommendations => {
                                Files::Recommendations(self.recommendations.clone())
                            }
                        };
                        job_claim.admit()?;
                        Ok::<_, StoreError>((reservation, lease, files))
                    })
                    .ok_or(StoreError::Conflict)??;
                #[cfg(test)]
                enter_replay_gate(&self.owner.replay_admitted);
                let (files, _cleanup_guards) = files.remove(&self.id, &record.identity);
                #[cfg(test)]
                enter_replay_gate(&self.owner.replay_removed);
                let progress = if files.is_ok() {
                    let next = record.job.progress | stage.bit();
                    Some(
                        handle
                            .block_on(self.owner.store.deletion_progress(
                                self.id.clone(),
                                self.created.clone(),
                                next,
                                false,
                            ))
                            .map(|()| next),
                    )
                } else {
                    None
                };
                let detached = self.owner.organization_current(
                    &self.id,
                    Some(&self.memory),
                    true,
                    |ws, h, pid| {
                        session_actions::runtime_closed(h.as_deref(), pid)
                            && stage.current(&self.workflow, &self.recommendation_store, ws)
                    },
                ) != Some(true);
                if !detached {
                    self.owner.state.updates.sessions_changed();
                }
                Ok(DeletionCleanupResult {
                    files,
                    progress,
                    detached,
                })
            })();
            if let Ok(receipt) = &result {
                if let Err(error) = &receipt.files {
                    tracing::warn!(session_id=%self.id,created=%self.created, cleanup_stage=stage.name(),%error,"external cleanup incomplete; job remains pending");
                }
                if let Some(Err(error)) = &receipt.progress {
                    tracing::warn!(session_id=%self.id,created=%self.created, cleanup_stage=stage.name(),%error,"external files removed but cleanup progress not recorded");
                }
            }
            if sent.send(result).is_err() {
                tracing::info!(session_id=%self.id,created=%self.created, cleanup_stage=stage.name(),"retained external cleanup finished without caller; pending job remains");
            }
        });
        tokio::select! { biased;
            result=&mut received=>result.map_err(|_|StoreError::Unavailable)?,
            _=tokio::time::sleep_until(claim.deadline.into())=>{
                if claim.cancel() { Err(StoreError::Expired) }
                else { received.await.map_err(|_|StoreError::Unavailable)? }
            }
        }
    }
}
