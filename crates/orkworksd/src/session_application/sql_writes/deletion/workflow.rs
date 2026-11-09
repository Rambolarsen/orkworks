//! Exact captured Workflow job, original canonical file owner, finite settlement.
use super::*;
use crate::session_application::sql_writes::observation_opening::validate_root_lease;
impl Claim {
    fn pending(&self) -> Result<(), StoreError> {
        if Instant::now() >= self.deadline {
            self.cancel();
            return Err(StoreError::Expired);
        }
        if self.state.load(Ordering::Acquire) == 0 {
            Ok(())
        } else {
            Err(StoreError::Cancelled)
        }
    }
}
impl CapturedDeletion {
    pub(super) async fn remove_sql_workflow(self) -> Result<DeletionCleanupResult, StoreError> {
        let claim = Arc::new(Claim {
            state: AtomicU8::new(0),
            deadline: Instant::now() + Duration::from_secs(2),
        });
        let _cancel = CancelOnDrop(claim.clone());
        let lane = self.owner.store.runtime_lane(&self.id)?;
        let root = self.owner.store.observation_root_identity()?;
        // bind() already ties this exact lexical app path to the original worker;
        // physical identity is checked separately through the canonical root.
        let selected_root = self.owner.store.location().0.to_owned();
        let lease = self.memory._lease.clone().ok_or(StoreError::Conflict)?;
        let handle = tokio::runtime::Handle::current();
        let job_claim = claim.clone();
        let (sent, mut received) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            // All original resources live outside the execution closure so a
            // partial result cannot release them before delivery/detached logging.
            let mut held_lane = None;
            let mut held_reservation = None;
            let mut cleanup: Option<crate::workflow_observations::SqlCleanup> = None;
            let result = (|| {
                held_lane = Some(
                    handle
                        .block_on(tokio::time::timeout_at(
                            job_claim.deadline.into(),
                            lane.lock_owned(),
                        ))
                        .map_err(|_| StoreError::Expired)?,
                );
                job_claim.pending()?;
                if sent.is_closed() {
                    return Err(StoreError::Cancelled);
                }
                validate_root_lease(&root, &lease)?;
                let mut candidate = Some(self.workflow.sql_candidate(&lease)?);
                #[cfg(test)]
                enter_replay_gate(&self.owner.replay_before);
                job_claim.pending()?;
                let record = handle.block_on(async {
                    tokio::time::timeout_at(
                        job_claim.deadline.into(),
                        self.owner.store.workflow_deletion_record(&self.identity),
                    )
                    .await
                    .map_err(|_| StoreError::Expired)?
                })?;
                // The complete record was only a checked SQL snapshot. File work
                // retains its original identity and never needs the large payload.
                drop(record);
                validate_root_lease(&root, &lease)?;
                candidate
                    .as_mut()
                    .ok_or(StoreError::Conflict)?
                    .revalidate(root.canonical_path(), &lease)?;
                loop {
                    job_claim.pending()?;
                    if sent.is_closed() {
                        return Err(StoreError::Cancelled);
                    }
                    match self.owner.organization_try_current(
                        &self.id,
                        Some(&self.memory),
                        true,
                        |ws, h, pid| {
                            if !session_actions::runtime_closed(h.as_deref(), pid)
                                || !Stage::Workflow.current(
                                    &self.workflow,
                                    &self.recommendation_store,
                                    ws,
                                )
                                || ws.metadata.root_path() != selected_root
                                || !ws
                                    .lease
                                    .as_ref()
                                    .is_some_and(|current| Arc::ptr_eq(current, &lease))
                            {
                                return Err(StoreError::Conflict);
                            }
                            let reservation = ws
                                .metadata
                                .organization
                                .reserve(&self.id)
                                .ok_or(StoreError::Conflict)?;
                            job_claim.admit()?;
                            let grant = candidate
                                .take()
                                .ok_or(StoreError::Conflict)?
                                .admit_cleanup(
                                    WorkflowCleanupAdmission {
                                        identity: self.identity.clone(),
                                    },
                                    self.workflow.clone(),
                                )?;
                            Ok((reservation, grant))
                        },
                    ) {
                        Ok(Some(result)) => {
                            let (reservation, grant) = result?;
                            held_reservation = Some(reservation);
                            cleanup = Some(grant);
                            break;
                        }
                        Ok(None) => return Err(StoreError::Conflict),
                        Err(StoreError::Busy) => std::thread::sleep(Duration::from_millis(2)),
                        Err(error) => return Err(error),
                    }
                }
                #[cfg(test)]
                enter_replay_gate(&self.owner.replay_admitted);
                let files = cleanup.as_mut().ok_or(StoreError::Conflict)?.remove();
                #[cfg(test)]
                enter_replay_gate(&self.owner.replay_removed);
                let progress = if files.is_ok() {
                    Some(
                        handle.block_on(
                            self.owner
                                .store
                                .record_workflow_cleanup(self.identity.clone()),
                        ),
                    )
                } else {
                    None
                };
                let detached = self
                    .owner
                    .organization_try_current(&self.id, Some(&self.memory), true, |ws, h, pid| {
                        session_actions::runtime_closed(h.as_deref(), pid)
                            && Stage::Workflow.current(
                                &self.workflow,
                                &self.recommendation_store,
                                ws,
                            )
                            && ws.metadata.root_path() == selected_root
                            && ws
                                .lease
                                .as_ref()
                                .is_some_and(|current| Arc::ptr_eq(current, &lease))
                    })
                    .ok()
                    .flatten()
                    != Some(true);
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
                    tracing::warn!(session_id=%self.id,created=%self.created,%error,"Workflow cleanup incomplete; original job remains pending");
                }
                if let Some(Err(error)) = &receipt.progress {
                    tracing::warn!(session_id=%self.id,created=%self.created,%error,"Workflow files removed but original progress not recorded");
                }
            }
            if sent.send(result).is_err() {
                tracing::info!(session_id=%self.id,created=%self.created,"original Workflow cleanup settled without caller");
            }
            drop(cleanup);
            drop(held_reservation);
            drop(held_lane);
            drop(lease);
            drop(self);
        });
        tokio::select! {biased;
            result=&mut received=>result.map_err(|_|StoreError::Unavailable)?,
            _=tokio::time::sleep_until(claim.deadline.into())=>{
                if claim.cancel(){Err(StoreError::Expired)}else{received.await.map_err(|_|StoreError::Unavailable)?}
            }
        }
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod grant_tests {
    use super::*;
    use crate::session_application::sql_writes::{
        delete_replay_tests::deleted,
        organization_tests::{CREATED, ID},
    };
    #[tokio::test]
    async fn read_admission_cannot_upgrade_to_workflow_cleanup() {
        let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
        let (_dir, state, store, _writes) = deleted().await;
        let (files, lease) = {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            (ws.workflow_observations.clone(), ws.lease.clone().unwrap())
        };
        let identity = store
            .deletion_record(ID.into(), CREATED.into())
            .await
            .unwrap()
            .unwrap()
            .identity;
        let mut candidate = files.sql_candidate(&lease).unwrap();
        candidate
            .admit(
                ObservationOpenAdmission { _private: () },
                &crate::workflow_observations::FileClaim::new(),
            )
            .unwrap();
        // Trusted test proof isolates the file grant's no-upgrade boundary.
        assert!(matches!(
            candidate.admit_cleanup(WorkflowCleanupAdmission { identity }, files),
            Err(StoreError::Conflict)
        ));
        assert!(!store
            .location()
            .0
            .join("workflow-observations/sequence")
            .exists());
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn retained_cleanup_grant_cannot_remove_a_later_replacement_on_reuse() {
        let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
        let (_dir, state, store, _writes) = deleted().await;
        let (files, lease) = {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            (ws.workflow_observations.clone(), ws.lease.clone().unwrap())
        };
        let identity = store
            .deletion_record(ID.into(), CREATED.into())
            .await
            .unwrap()
            .unwrap()
            .identity;
        let path = store
            .location()
            .0
            .join(format!("workflow-observations/{ID}.ndjson"));
        std::fs::write(&path, b"original fixture").unwrap();
        let mut grant = files
            .sql_candidate(&lease)
            .unwrap()
            .admit_cleanup(WorkflowCleanupAdmission { identity }, files.clone())
            .unwrap();
        grant.remove().unwrap();
        assert!(!path.exists());
        std::fs::write(&path, b"later fixture").unwrap();
        assert_eq!(
            grant.remove().unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert_eq!(std::fs::read(path).unwrap(), b"later fixture");
        assert!(files.workspace_observations().is_err());
        store.close().await.unwrap();
    }
}
