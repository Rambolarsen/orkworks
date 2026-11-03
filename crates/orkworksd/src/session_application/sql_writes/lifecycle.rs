//! Dormant lifecycle metadata application; production lifecycle stays JSON-backed.
#![cfg_attr(not(test), allow(dead_code))] // Production routing needs external-effect qualification.
use super::*;
use crate::metadata::lifecycle_mutations::LifecycleMutation;
use crate::runtime::session_runtime::{RuntimeCommand, RuntimeIdentity};

pub(crate) enum LifecycleWrite {
    Transition {
        status: String,
    },
    Finalize {
        snapshot: metadata::ObservedStatusSnapshotMetadata,
        fallback_status: String,
    },
}
struct Snapshot {
    runtime: RuntimeIdentity,
    control: tokio::sync::mpsc::Sender<RuntimeCommand>,
    created: String,
    phase: String,
    lifecycle: String,
    status: String,
    harness: Option<String>,
    harness_id: Option<String>,
    resume: Option<crate::harness::ResumeMemory>,
    resumed_from: Option<String>,
    attention: AttentionWriteToken,
    provenance: metadata::MetadataProvenance,
    metadata_source: Option<String>,
    metadata_confidence: Option<f64>,
}
impl Snapshot {
    fn capture(h: &crate::SessionHandle) -> Option<Self> {
        Some(Self {
            runtime: h.runtime.identity(),
            control: h.runtime.control_tx.clone(),
            created: h.info.created_at.clone(),
            phase: h.info.lifecycle_phase.clone(),
            lifecycle: h.info.lifecycle.clone(),
            status: h.info.status.clone(),
            harness: h.info.harness.clone(),
            harness_id: h.info.harness_id.clone(),
            resume: h.info.resume.clone(),
            resumed_from: h.info.resumed_from.clone(),
            attention: h.runtime.attention_owner.snapshot()?,
            provenance: h.info.provenance.clone(),
            metadata_source: h.info.metadata_source.clone(),
            metadata_confidence: h.info.metadata_confidence,
        })
    }
    fn matches(&self, h: &crate::SessionHandle) -> bool {
        self.runtime == h.runtime.identity()
            && self.control.same_channel(&h.runtime.control_tx)
            && self.created == h.info.created_at
            && self.phase == h.info.lifecycle_phase
            && self.lifecycle == h.info.lifecycle
            && self.status == h.info.status
            && self.harness == h.info.harness
            && self.harness_id == h.info.harness_id
            && self.resume == h.info.resume
            && self.resumed_from == h.info.resumed_from
            && h.runtime.attention_owner.owns(&self.attention)
            && self.provenance == h.info.provenance
            && self.metadata_source == h.info.metadata_source
            && self.metadata_confidence == h.info.metadata_confidence
    }
}
impl LifecycleWrite {
    fn intent(self, h: &crate::SessionHandle) -> Result<LifecycleMutation, StoreError> {
        let timestamp = iso_now();
        let intent = match self {
            Self::Transition { status } => {
                if h.info.lifecycle_phase == "ended"
                    || (h.info.lifecycle_phase == "ending"
                        && !matches!(status.as_str(), "killed" | "ended" | "error"))
                {
                    return Err(StoreError::Conflict);
                }
                LifecycleMutation::Transition {
                    status,
                    had_live_handle: true,
                    already_ending: h.info.lifecycle_phase == "ending",
                    live_phase: Some(h.info.lifecycle_phase.clone()),
                    resume: h.info.resume.clone(),
                    resumed_from: h.info.resumed_from.clone(),
                    timestamp,
                }
            }
            Self::Finalize {
                snapshot,
                fallback_status,
            } => {
                if h.info.lifecycle_phase != "ending" {
                    return Err(StoreError::Conflict);
                }
                LifecycleMutation::Finalize {
                    snapshot,
                    fallback_status,
                    timestamp,
                }
            }
        };
        if !intent.valid() {
            return Err(StoreError::Invalid("invalid lifecycle intent".into()));
        }
        Ok(intent)
    }
}
impl SessionWrites {
    /// Memory-only callbacks retain the completion's lane/lease/budget through publication.
    async fn lifecycle_access<T: Send + 'static>(
        self: &Arc<Self>,
        id: String,
        runtime: RuntimeIdentity,
        snapshot: Option<Arc<Snapshot>>,
        completion: Option<crate::session_store::InternalCompletion>,
        apply: impl FnOnce(&WorkspaceState, &mut crate::SessionHandle) -> T + Send + 'static,
    ) -> Result<Option<T>, StoreError> {
        let owner = self.clone();
        tokio::task::spawn_blocking(move || {
            let _completion = completion;
            let _projection = owner.state.projection_lock.lock().unwrap();
            let Ok(_transition) = crate::runtime::prompt_authority::transition_lock().try_lock()
            else {
                return None;
            };
            let workspace = owner.state.workspace.lock().unwrap();
            let ws = workspace.as_ref()?;
            if !owner.workspace.matches(ws)
                || !Arc::ptr_eq(&owner.reader, &ws.metadata.session_reader())
            {
                return None;
            }
            let mut sessions = owner.state.sessions.lock().unwrap();
            let h = sessions.get_mut(&id)?;
            if h.info.id != id
                || runtime != h.runtime.identity()
                || snapshot.as_ref().is_some_and(|s| !s.matches(h))
            {
                return None;
            }
            Some(apply(ws, h))
        })
        .await
        .map_err(|_| StoreError::Unavailable)
    }
    /// Metadata only: does not request process exit, persist replay size, append events or spawn.
    pub(crate) async fn lifecycle(
        self: &Arc<Self>,
        id: String,
        runtime: RuntimeIdentity,
        intent: LifecycleWrite,
    ) -> Result<Outcome, StoreError> {
        let (snapshot, lease, intent) = self
            .lifecycle_access(id.clone(), runtime.clone(), None, None, move |ws, h| {
                Ok::<_, StoreError>((
                    Snapshot::capture(h).ok_or(StoreError::Conflict)?,
                    ws.lease.clone().ok_or(StoreError::Conflict)?,
                    intent.intent(h)?,
                ))
            })
            .await?
            .ok_or(StoreError::Conflict)??;
        let snapshot = Arc::new(snapshot);
        let prepared = self.store.prepare_internal(
            id.clone(),
            snapshot.created.clone(),
            InternalMutation::Lifecycle(intent),
        )?;
        let _cancellation = prepared.cancellation();
        let lane = prepared.lane()?;
        let owner = self.clone();
        let (mut result, received) = oneshot::channel();
        tokio::spawn(async move {
            let execution = async {
                let lane = tokio::select! {
                    biased;
                    _ = result.closed() => return Err(StoreError::Expired),
                    lane = tokio::time::timeout_at(prepared.deadline().into(), lane.lock_owned()) =>
                        lane.map_err(|_| StoreError::Expired)?,
                };
                let (ticket, ready) = prepared.enqueue(RuntimeWriteResources::new(lane, Some(lease)))?;
                let mut finished = Box::pin(ticket.finish());
                let mut delivered = None;
                let request = tokio::select! {
                    biased;
                    _ = result.closed() => return Err(StoreError::Expired),
                    completion = &mut finished => { delivered = Some(completion); None },
                    ready = ready => ready.ok(),
                };
                if let Some(request) = request {
                    // Expiry can beat a blocked memory validation. Consume the
                    // definitive result concurrently so cancelled work retires.
                    tokio::select! {
                        biased;
                        completion = &mut finished => delivered = Some(completion),
                        claimed = owner.lifecycle_access(id.clone(), runtime.clone(), Some(snapshot.clone()), None,
                            move |_, _| request.grant()) => {
                            if let Some(Ok(grant)) = claimed? { let _ = grant.execute(); }
                        }
                    }
                }
                let completion = match delivered {
                    Some(completion) => completion?,
                    None => finished.await?,
                };
                let Some(document) = completion.outcome.document.as_ref() else {
                    return Ok(Outcome::Noop { permanent_hold: false });
                };
                let stored: metadata::SessionMetadata = match serde_json::from_value(document.clone()) {
                    Ok(stored) => stored,
                    Err(_) => return Ok(Outcome::CommittedDetached),
                };
                let scope = completion.outcome.decision.scope;
                let published = owner.lifecycle_access(id, runtime, Some(snapshot), Some(completion), move |_, h| {
                    if scope.is_some_and(|scope| scope.attention) {
                        h.runtime.attention_owner.accepted_write();
                    }
                    h.info.status = stored.status;
                    h.info.lifecycle_phase = stored.lifecycle_phase;
                    h.info.lifecycle = stored.lifecycle;
                    h.info.attention = stored.attention;
                    h.info.connectivity = Some(stored.connectivity);
                    h.info.terminal_outcome = stored.terminal_outcome;
                    h.info.observed_status = stored.observed_status;
                    h.info.final_observed_status = stored.final_observed_status_snapshot.and_then(|snapshot| snapshot.value);
                    h.info.last_activity_at = Some(stored.last_activity);
                    h.info.provenance = stored.provenance;
                    if h.info.lifecycle_phase == "ended" { h.resume_in_progress = false; }
                }).await.is_ok_and(|published| published.is_some());
                if published { owner.state.updates.sessions_changed(); }
                Ok(if published { Outcome::Committed } else { Outcome::CommittedDetached })
            }.await;
            let _ = result.send(execution);
        });
        received.await.map_err(|_| StoreError::Unavailable)?
    }
}
