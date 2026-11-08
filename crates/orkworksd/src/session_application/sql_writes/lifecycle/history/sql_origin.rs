//! Original-before-inference source capture and the sole checked observation read admission.
use super::*;
use crate::session_application::sql_writes::observation_opening::validate_root_lease;
use crate::session_store::ObservationSource;
use crate::workflow_observations::{
    sql_origin::{ScanRequest, Snapshot},
    FileClaim,
};
use crate::workspace_runtime::WorkspaceIdentity;
#[must_use]
pub(crate) struct CapturedSqlObservation {
    ending: CapturedEnding,
    source: ObservationSource,
    root_identity: Arc<WorkspaceIdentity>,
}
/// Keeps original admission, worker, lane, lease and scan capacity until the result is dropped.
#[must_use]
pub(crate) struct SqlObservationRead {
    snapshot: Snapshot,
    _claim: CapturedSqlObservation,
}
struct CancelOnDrop(Arc<FileClaim>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
impl CapturedEnding {
    async fn observation_current(&self, deadline: std::time::Instant) -> Result<(), StoreError> {
        loop {
            if std::time::Instant::now() >= deadline {
                return Err(StoreError::Expired);
            }
            let instance = self.workflow.instance_id();
            match self.owner.observation_try_current(
                &self.id,
                &self.runtime,
                Some(&self.snapshot),
                |ws, _| {
                    ws.workflow_observations.instance_id() == instance
                        && ws.metadata.root_path() == self.root
                        && ws
                            .lease
                            .as_ref()
                            .is_some_and(|current| Arc::ptr_eq(current, &self.lease))
                },
            ) {
                Ok(Some(true)) => return Ok(()),
                Ok(_) => return Err(StoreError::Conflict),
                Err(StoreError::Busy) => {
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await
                }
                Err(error) => return Err(error),
            }
        }
    }
    /// This constructor consumes the actual final-scan owner, never a parsed observation or ID.
    pub(crate) async fn capture_sql_observation_origin(
        self,
    ) -> Result<CapturedSqlObservation, StoreError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let capacity =
            Arc::new(crate::session_application::sql_writes::reserve_observation_read()?);
        tokio::time::timeout_at(deadline.into(), self.observation_current(deadline))
            .await
            .map_err(|_| StoreError::Expired)??;
        let root_identity = self.owner.store.observation_root_identity()?;
        check_root(&self, root_identity.clone(), capacity.clone(), deadline).await?;
        let source = tokio::time::timeout_at(
            deadline.into(),
            self.owner
                .store
                .capture_observation_source(self.id.clone(), self.snapshot.created.clone()),
        )
        .await
        .map_err(|_| StoreError::Expired)??;
        check_root(&self, root_identity.clone(), capacity, deadline).await?;
        tokio::time::timeout_at(deadline.into(), self.observation_current(deadline))
            .await
            .map_err(|_| StoreError::Expired)??;
        if std::time::Instant::now() >= deadline {
            return Err(StoreError::Expired);
        }
        Ok(CapturedSqlObservation {
            ending: self,
            source,
            root_identity,
        })
    }
}
impl SqlObservationRead {
    pub(crate) fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}
impl CapturedSqlObservation {
    pub(crate) fn source(&self) -> &ObservationSource {
        &self.source
    }
    /// Read-only original grant. Future producers/deletion need their own explicit composition.
    pub(crate) async fn open_observations(self) -> Result<SqlObservationRead, StoreError> {
        let claim = Arc::new(FileClaim::new());
        let _cancel = CancelOnDrop(claim.clone());
        let scan = ScanRequest::capture(claim.deadline)?;
        let lane = self.ending.owner.store.runtime_lane(&self.ending.id)?;
        let handle = tokio::runtime::Handle::current();
        let job_claim = claim.clone();
        let (sent, mut received) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let execute = (|| {
                let lane = handle.block_on(async {
                    tokio::time::timeout_at(job_claim.deadline.into(), lane.lock_owned())
                        .await
                        .map_err(|_| StoreError::Expired)
                })?;
                if sent.is_closed() {
                    return Err(StoreError::Cancelled);
                }
                validate_root_lease(&self.root_identity, &self.ending.lease)?;
                let mut candidate = self.ending.workflow.sql_candidate(&self.ending.lease)?;
                #[cfg(test)]
                enter_replay_gate(&self.ending.owner.observation_before_admit);
                // The original lane and memory-only operation claim precede this final
                // read transaction; file locks never span SQL waits.
                handle.block_on(
                    self.ending
                        .owner
                        .store
                        .validate_observation_source(self.source.clone()),
                )?;
                validate_root_lease(&self.root_identity, &self.ending.lease)?;
                candidate.revalidate(self.root_identity.canonical_path(), &self.ending.lease)?;
                let instance = self.ending.workflow.instance_id();
                let lease = self.ending.lease.clone();
                let root = &self.ending.root;
                loop {
                    job_claim.pending()?;
                    if sent.is_closed() {
                        return Err(StoreError::Cancelled);
                    }
                    match self.ending.owner.observation_try_current(
                        &self.ending.id,
                        &self.ending.runtime,
                        Some(&self.ending.snapshot),
                        |ws, _| {
                            if ws.workflow_observations.instance_id() != instance
                                || ws.metadata.root_path() != *root
                                || !ws
                                    .lease
                                    .as_ref()
                                    .is_some_and(|current| Arc::ptr_eq(current, &lease))
                            {
                                return Err(StoreError::Conflict);
                            }
                            candidate.admit(ObservationOpenAdmission { _private: () }, &job_claim)
                        },
                    ) {
                        Ok(Some(result)) => {
                            result?;
                            break;
                        }
                        Ok(None) => return Err(StoreError::Conflict),
                        Err(StoreError::Busy) => {
                            std::thread::sleep(std::time::Duration::from_millis(2))
                        }
                        Err(error) => return Err(error),
                    }
                }
                #[cfg(test)]
                enter_replay_gate(&self.ending.owner.observation_admitted);
                let snapshot = candidate.read(scan, &self.source)?;
                if std::time::Instant::now() >= job_claim.deadline {
                    return Err(StoreError::Expired);
                }
                drop(candidate);
                drop(lane);
                Ok(SqlObservationRead {
                    snapshot,
                    _claim: self,
                })
            })();
            let _ = sent.send(execute);
        });
        tokio::select! {biased;result=&mut received=>{let result=result.map_err(|_|StoreError::Unavailable)?;if std::time::Instant::now()>=claim.deadline{Err(StoreError::Expired)}else{result}},_=tokio::time::sleep_until(claim.deadline.into())=>{claim.expire();Err(StoreError::Expired)}}
    }
}

async fn check_root(
    ending: &CapturedEnding,
    identity: Arc<WorkspaceIdentity>,
    capacity: Arc<crate::session_application::sql_writes::ObservationReadCapacity>,
    deadline: std::time::Instant,
) -> Result<(), StoreError> {
    let lease = ending.lease.clone();
    let owner = ending.owner.clone();
    let workflow = ending.workflow.clone();
    let job = tokio::task::spawn_blocking(move || {
        let _capacity = capacity;
        let _owner = owner;
        let _workflow = workflow;
        #[cfg(test)]
        enter_replay_gate(&_owner.observation_root);
        if std::time::Instant::now() >= deadline {
            return Err(StoreError::Expired);
        }
        validate_root_lease(&identity, &lease)?;
        if std::time::Instant::now() >= deadline {
            return Err(StoreError::Expired);
        }
        Ok::<_, StoreError>(())
    });
    tokio::time::timeout_at(deadline.into(), job)
        .await
        .map_err(|_| StoreError::Expired)?
        .map_err(|_| StoreError::Unavailable)?
}
#[cfg(test)]
pub(crate) enum ObservationStage {
    Root,
    BeforeAdmit,
    Admitted,
}
#[cfg(test)]
pub(crate) fn hold_observation(
    writes: &SessionWrites,
    stage: ObservationStage,
) -> (oneshot::Receiver<()>, std::sync::mpsc::Sender<()>) {
    let (entered, reached) = oneshot::channel();
    let (release, rx) = std::sync::mpsc::channel();
    let slot = match stage {
        ObservationStage::Root => &writes.observation_root,
        ObservationStage::BeforeAdmit => &writes.observation_before_admit,
        ObservationStage::Admitted => &writes.observation_admitted,
    };
    *slot.lock().unwrap() = Some((entered, rx));
    (reached, release)
}
