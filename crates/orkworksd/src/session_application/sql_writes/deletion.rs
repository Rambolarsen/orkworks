//! One captured replay stage. A tombstone survives all file/progress failures.
use super::*;
use crate::session_application::sql_writes::file_budget;
use std::sync::atomic::{AtomicU8, Ordering};

const REPLAY_REMOVED: u8 = 0x01;
pub(crate) struct CapturedDeletion {
    owner: Arc<SessionWrites>,
    id: String,
    created: String,
    memory: Arc<Memory>,
}
#[derive(Debug)]
#[must_use]
pub(crate) struct ReplayCleanupResult {
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
        let memory = Arc::new(
            self.organization_access(id.clone(), None, true, None, |ws, h, pid| {
                Memory::capture(ws, h.as_deref(), pid)
            })
            .await?
            .ok_or(StoreError::Conflict)?,
        );
        if !memory.closed
            || memory
                .runtime
                .as_ref()
                .is_some_and(|r| r.created != created)
        {
            return Err(StoreError::Conflict);
        }
        self.store
            .deletion_record(id.clone(), created.clone())
            .await?
            .ok_or(StoreError::Missing)?;
        self.organization_access(id.clone(), Some(memory.clone()), true, None, |_, _, _| ())
            .await?
            .ok_or(StoreError::Conflict)?;
        Ok(CapturedDeletion {
            owner: self.clone(),
            id,
            created,
            memory,
        })
    }
}
impl CapturedDeletion {
    pub(crate) async fn remove_replay(self) -> Result<ReplayCleanupResult, StoreError> {
        let capacity =
            file_budget::reserve(file_budget::intent_bytes(&(&self.id, &self.created))?)?;
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
            let _capacity = capacity;
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
                #[cfg(test)]
                enter_replay_gate(&self.owner.replay_before);
                let (_reservation, _lease, files) = self
                    .owner
                    .organization_current(&self.id, Some(&self.memory), true, |ws, h, pid| {
                        if !session_actions::runtime_closed(h.as_deref(), pid) {
                            return Err(StoreError::Conflict);
                        }
                        let lease = ws.lease.clone().ok_or(StoreError::Conflict)?;
                        let reservation = ws
                            .metadata
                            .organization
                            .reserve(&self.id)
                            .ok_or(StoreError::Conflict)?;
                        let files = ws.metadata.capture_replay_cleanup(&self.id)?;
                        job_claim.admit()?;
                        Ok::<_, StoreError>((reservation, lease, files))
                    })
                    .ok_or(StoreError::Conflict)??;
                #[cfg(test)]
                enter_replay_gate(&self.owner.replay_admitted);
                let files = files.remove();
                #[cfg(test)]
                enter_replay_gate(&self.owner.replay_removed);
                let progress = if files.is_ok() {
                    let next = record.job.progress | REPLAY_REMOVED;
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
                    |_, h, pid| session_actions::runtime_closed(h.as_deref(), pid),
                ) != Some(true);
                if !detached {
                    self.owner.state.updates.sessions_changed();
                }
                Ok(ReplayCleanupResult {
                    files,
                    progress,
                    detached,
                })
            })();
            if let Ok(receipt) = &result {
                if let Err(error) = &receipt.files {
                    tracing::warn!(session_id=%self.id,created=%self.created,%error,"replay cleanup incomplete; job remains pending");
                }
                if let Some(Err(error)) = &receipt.progress {
                    tracing::warn!(session_id=%self.id,created=%self.created,%error,"replay removed but cleanup progress not recorded");
                }
            }
            if sent.send(result).is_err() {
                tracing::info!(session_id=%self.id,created=%self.created,"retained replay cleanup finished without caller; pending job remains");
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
