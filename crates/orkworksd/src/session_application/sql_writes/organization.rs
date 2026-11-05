//! Captured source-only close-first organization. HTTP/production stays JSON.
#![cfg_attr(not(test), allow(dead_code))]
use super::*;
use crate::runtime::session_runtime::{RuntimeCommand, RuntimeIdentity};
use crate::session_application::session_actions::{self, SessionAction};
use crate::session_archive::{OrganizationStore, Reservation, SessionState};
use crate::session_store::{OrganizationCompletion, OrganizationRecord};
use std::time::{Duration, Instant};

type Control = tokio::sync::mpsc::Sender<RuntimeCommand>;
struct Runtime {
    identity: RuntimeIdentity,
    control: Control,
    created: String,
}
struct Memory {
    organization: Arc<OrganizationStore>,
    _lease: Option<Arc<WorkspaceLease>>,
    runtime: Option<Runtime>,
    key: String,
    has_pid: bool,
    closed: bool,
}
impl Memory {
    fn capture(ws: &WorkspaceState, h: Option<&crate::SessionHandle>, has_pid: bool) -> Self {
        Self {
            organization: ws.metadata.organization.clone(),
            _lease: ws.lease.clone(),
            runtime: h.map(|h| Runtime {
                identity: h.runtime.identity(),
                control: h.runtime.control_tx.clone(),
                created: h.info.created_at.clone(),
            }),
            key: session_actions::runtime_key(h),
            has_pid,
            closed: session_actions::runtime_closed(h, has_pid),
        }
    }
    fn matches(
        &self,
        ws: &WorkspaceState,
        h: Option<&crate::SessionHandle>,
        has_pid: bool,
        exact: bool,
    ) -> bool {
        Arc::ptr_eq(&self.organization, &ws.metadata.organization)
            && match (&self.runtime, h) {
                (None, None) => true,
                (Some(r), Some(h)) => {
                    r.identity == h.runtime.identity()
                        && r.control.same_channel(&h.runtime.control_tx)
                        && r.created == h.info.created_at
                }
                _ => false,
            }
            && (!exact
                || (self.key == session_actions::runtime_key(h)
                    && self.has_pid == has_pid
                    && self.closed == session_actions::runtime_closed(h, has_pid)))
    }
}
pub(crate) struct CapturedOrganization {
    owner: Arc<SessionWrites>,
    id: String,
    memory: Arc<Memory>,
    record: OrganizationRecord,
    state: SessionState,
}
#[derive(Debug)]
pub(crate) struct OrganizationWrite {
    pub(crate) revision: String,
    pub(crate) outcome: Outcome,
}

impl SessionWrites {
    fn organization_current<T>(
        &self,
        id: &str,
        captured: Option<&Memory>,
        exact: bool,
        apply: impl FnOnce(&WorkspaceState, Option<&mut crate::SessionHandle>, bool) -> T,
    ) -> Option<T> {
        let _projection = self.state.projection_lock.lock().unwrap();
        let workspace = self.state.workspace.lock().unwrap();
        let ws = workspace.as_ref()?;
        if !self.workspace.matches(ws) || !Arc::ptr_eq(&self.reader, &ws.metadata.session_reader())
        {
            return None;
        }
        let mut sessions = self.state.sessions.lock().unwrap();
        let h = sessions.get_mut(id);
        let pids = self.state.session_pids.lock().unwrap();
        let has_pid = pids.contains_key(id);
        if h.as_ref().is_some_and(|h| h.info.id != id)
            || captured.is_some_and(|m| !m.matches(ws, h.as_deref(), has_pid, exact))
        {
            return None;
        }
        Some(apply(ws, h, has_pid))
    }
    async fn organization_access<T: Send + 'static>(
        self: &Arc<Self>,
        id: String,
        captured: Option<Arc<Memory>>,
        exact: bool,
        completion: Option<OrganizationCompletion>,
        apply: impl FnOnce(&WorkspaceState, Option<&mut crate::SessionHandle>, bool) -> T
            + Send
            + 'static,
    ) -> Result<Option<T>, StoreError> {
        let owner = self.clone();
        tokio::task::spawn_blocking(move || {
            let _completion = completion;
            owner.organization_current(&id, captured.as_deref(), exact, apply)
        })
        .await
        .map_err(|_| StoreError::Unavailable)
    }
    pub(crate) async fn capture_organization(
        self: &Arc<Self>,
        id: String,
    ) -> Result<CapturedOrganization, StoreError> {
        let memory = Arc::new(
            self.organization_access(id.clone(), None, true, None, |ws, h, pid| {
                Memory::capture(ws, h.as_deref(), pid)
            })
            .await?
            .ok_or(StoreError::Conflict)?,
        );
        let record = self
            .store
            .organization_state(id.clone())
            .await?
            .ok_or(StoreError::Missing)?;
        let created = record.document["createdAt"]
            .as_str()
            .ok_or(StoreError::Corrupt)?;
        if memory
            .runtime
            .as_ref()
            .is_some_and(|r| r.created != created)
        {
            return Err(StoreError::Conflict);
        }
        if record.state == "unknown" {
            return Err(StoreError::Conflict);
        }
        self.organization_access(id.clone(), Some(memory.clone()), true, None, |_, _, _| ())
            .await?
            .ok_or(StoreError::Conflict)?;
        let state = if !memory.closed {
            SessionState::Active
        } else if record.state == "archived" {
            SessionState::Archived
        } else if record.state == "closed"
            && record.document["lifecyclePhase"] == "ended"
            && record.document["lifecycle"] == "dead"
        {
            SessionState::Closed
        } else {
            SessionState::Active
        };
        Ok(CapturedOrganization {
            owner: self.clone(),
            id,
            memory,
            record,
            state,
        })
    }
}
impl CapturedOrganization {
    pub(crate) fn state(&self) -> SessionState {
        self.state
    }
    pub(crate) async fn apply(
        self,
        action: SessionAction,
    ) -> Result<OrganizationWrite, StoreError> {
        self.apply_with_timeout(action, Duration::from_secs(15))
            .await
    }
    pub(super) async fn apply_with_timeout(
        self,
        action: SessionAction,
        wait: Duration,
    ) -> Result<OrganizationWrite, StoreError> {
        if !matches!(
            (self.state, action),
            (
                SessionState::Active | SessionState::Closed,
                SessionAction::Archive
            ) | (SessionState::Archived, SessionAction::Restore)
        ) {
            return Err(StoreError::Conflict);
        }
        // Re-read SQL outside guards before any stop signal. The original memory
        // snapshot is rechecked afterwards; replacement authority is never adopted.
        let current = self
            .owner
            .store
            .organization_state(self.id.clone())
            .await?
            .ok_or(StoreError::Missing)?;
        if current.document["createdAt"] != self.record.document["createdAt"]
            || current.organization_revision != self.record.organization_revision
            || current.state != self.record.state
        {
            return Err(StoreError::Conflict);
        }
        let active = self.state == SessionState::Active;
        let id = self.id.clone();
        let (reservation, lease) = self
            .owner
            .organization_access(
                self.id.clone(),
                Some(self.memory.clone()),
                true,
                None,
                move |ws, h, _| {
                    let lease = ws.lease.clone().ok_or(StoreError::Conflict)?;
                    let reservation = ws
                        .metadata
                        .organization
                        .reserve(&id)
                        .ok_or(StoreError::Conflict)?;
                    if active {
                        h.ok_or(StoreError::Conflict)?.kill_tx.send_replace(true);
                    }
                    Ok::<_, StoreError>((reservation, lease))
                },
            )
            .await?
            .ok_or(StoreError::Conflict)??;
        let deadline = active.then(|| Instant::now() + wait);
        let (mut sent, received) = oneshot::channel();
        tokio::spawn(async move {
            let result = self
                .finish(action, reservation, lease, deadline, &mut sent)
                .await;
            let _ = sent.send(result);
        });
        received.await.map_err(|_| StoreError::Unavailable)?
    }
    async fn finish(
        self,
        action: SessionAction,
        reservation: Reservation,
        lease: Arc<WorkspaceLease>,
        close_deadline: Option<Instant>,
        sent: &mut oneshot::Sender<Result<OrganizationWrite, StoreError>>,
    ) -> Result<OrganizationWrite, StoreError> {
        loop {
            if sent.is_closed() || close_deadline.is_some_and(|d| Instant::now() >= d) {
                return Err(StoreError::Expired);
            }
            let closed = self
                .owner
                .organization_access(
                    self.id.clone(),
                    Some(self.memory.clone()),
                    false,
                    None,
                    move |_, h, pid| session_actions::runtime_closed(h.as_deref(), pid),
                )
                .await?
                .ok_or(StoreError::Conflict)?;
            if closed {
                break;
            }
            let deadline = close_deadline.ok_or(StoreError::Conflict)?;
            tokio::select! { biased; _=sent.closed()=>return Err(StoreError::Expired),
            _=tokio::time::sleep(Duration::from_millis(25).min(deadline.saturating_duration_since(Instant::now())))=>{} }
        }
        let choice = if action == SessionAction::Archive {
            "archived"
        } else {
            "closed"
        };
        let created = self.record.document["createdAt"]
            .as_str()
            .ok_or(StoreError::Corrupt)?
            .to_owned();
        let prepared = self.owner.store.prepare_organization(
            self.id.clone(),
            created,
            self.record.organization_revision.clone(),
            choice.into(),
        )?;
        let cancellation = prepared.cancellation();
        let deadline = close_deadline.map_or(prepared.deadline(), |d| d.min(prepared.deadline()));
        let lane = prepared.lane()?;
        #[cfg(test)]
        if let Some(ready) = self.owner.before_organization_lane.lock().unwrap().take() {
            let _ = ready.send(());
        }

        let lane = tokio::select! { biased; _=sent.closed()=>return Err(StoreError::Expired),
        lane=tokio::time::timeout_at(deadline.into(),lane.lock_owned())=>lane.map_err(|_|StoreError::Expired)? };
        let resources =
            RuntimeWriteResources::new(lane, Some(lease)).with_organization(reservation);
        let (ticket, ready) = prepared.enqueue(resources)?;
        let mut finished = Box::pin(ticket.finish());
        let request = tokio::select! { biased; _=sent.closed()=>return Err(StoreError::Expired),
        _=tokio::time::sleep_until(deadline.into())=>return Err(StoreError::Expired),
        completion=&mut finished=>return self.publish(completion?).await,
        ready=ready=>ready.map_err(|_|StoreError::Unavailable)? };
        let owner = self.owner.clone();
        let id = self.id.clone();
        let memory = self.memory.clone();
        let validation = async move {
            let grant = owner
                .organization_access(id, Some(memory), false, None, move |_, h, pid| {
                    if Instant::now() >= deadline {
                        return Err(StoreError::Expired);
                    }
                    if !session_actions::runtime_closed(h.as_deref(), pid) {
                        return Err(StoreError::Conflict);
                    }
                    request.grant()
                })
                .await;
            #[cfg(test)]
            if matches!(&grant, Ok(Some(Ok(_)))) {
                let gate = owner.after_organization_grant.lock().unwrap().take();
                if let Some((entered, release)) = gate {
                    let _ = entered.send(());
                    let _ = release.await;
                }
            }
            grant
        };
        let mut validation = Box::pin(validation);
        // Race the same atomic state as grant. Once cancellation/expiry loses,
        // drain validation: dropping its pending successful grant would reject
        // work that is already admitted but not yet delivered to this task.
        let mut observe_cancel = true;
        let mut observe_deadline = true;
        let grant = loop {
            tokio::select! { biased;
                completion=&mut finished=>return self.publish(completion?).await,
                grant=&mut validation=>break grant?.ok_or(StoreError::Conflict)??,
                _=sent.closed(), if observe_cancel=>{
                    observe_cancel=false;
                    if cancellation.cancel() { return Err(StoreError::Expired); }
                },
                _=tokio::time::sleep_until(deadline.into()), if observe_deadline=>{
                    observe_deadline=false;
                    if cancellation.cancel() { return Err(StoreError::Expired); }
                },
            }
        };
        // Application guards have been released by the blocking validation job.
        // Caller cancellation after the atomic grant cannot assert rollback.
        grant.execute()?;
        self.publish(finished.await?).await
    }
    async fn publish(
        self,
        completion: OrganizationCompletion,
    ) -> Result<OrganizationWrite, StoreError> {
        let revision = completion.revision.clone();
        let published = self
            .owner
            .organization_access(
                self.id,
                Some(self.memory),
                false,
                Some(completion),
                |_, h, pid| session_actions::runtime_closed(h.as_deref(), pid),
            )
            .await
            .is_ok_and(|current| current == Some(true));
        if published {
            self.owner.state.updates.sessions_changed();
        }
        Ok(OrganizationWrite {
            revision,
            outcome: if published {
                Outcome::Committed
            } else {
                Outcome::CommittedDetached
            },
        })
    }
}
