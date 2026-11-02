//! Dormant native effect application using the same worker/session lane as Peon.
use super::*;
use crate::runtime::codex_approval_application::{
    sql_context::{IssuedEffect, NativeCapture, ObservationContext},
    COORDINATED_METADATA_WRITERS,
};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
/// Shared by a hook's metadata and native effects. Cancellation can retire
/// an unclaimed operation; an already claimed operation remains definitive.
#[derive(Default)]
struct HookOperation(AtomicU8);
impl HookOperation {
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire) == 2
    }
    fn cancel_before_claim(&self) -> bool {
        self.0
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            || self.is_cancelled()
    }
    fn mark_claimed(&self) -> bool {
        self.0
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            || self.0.load(Ordering::Acquire) == 1
    }
}
struct HookCapability {
    id: String,
    token: String,
}
impl HookCapability {
    fn is_current(&self) -> bool {
        crate::runtime::terminal_runtime::verify_workflow_report_token(&self.id, &self.token)
    }
}
enum NativeAttemptError {
    Store(StoreError),
    Retry,
}
impl From<StoreError> for NativeAttemptError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}
struct NativeOrigin {
    capability: Option<HookCapability>,
    context: Arc<ObservationContext>,
    operation: Option<Arc<HookOperation>>,
}
impl From<Arc<ObservationContext>> for NativeOrigin {
    fn from(context: Arc<ObservationContext>) -> Self {
        Self {
            capability: None,
            context,
            operation: None,
        }
    }
}
struct NativeView {
    capability: Option<HookCapability>,
    capture: NativeCapture,
    operation: Option<Arc<HookOperation>>,
    authority: Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
}
impl NativeView {
    fn is_current(
        &self,
        h: &crate::SessionHandle,
        authority: &Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
    ) -> bool {
        authority == &self.authority
            && self.capture.is_current(h)
            && self
                .capability
                .as_ref()
                .is_none_or(HookCapability::is_current)
    }
}
enum NativeAccess<T> {
    Busy,
    Obsolete,
    Current(T),
}
impl SessionWrites {
    async fn native_current<T: Send + 'static>(
        self: &Arc<Self>,
        id: String,
        view: Option<Arc<NativeView>>,
        completion: Option<crate::session_store::InternalCompletion>,
        apply: impl FnOnce(
                &WorkspaceState,
                &mut crate::SessionHandle,
                Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
            ) -> T
            + Send
            + 'static,
    ) -> Result<Option<T>, StoreError> {
        Ok(
            match self.native_access(id, view, completion, apply).await? {
                NativeAccess::Current(value) => Some(value),
                NativeAccess::Busy | NativeAccess::Obsolete => None,
            },
        )
    }
    async fn native_access<T: Send + 'static>(
        self: &Arc<Self>,
        id: String,
        view: Option<Arc<NativeView>>,
        completion: Option<crate::session_store::InternalCompletion>,
        apply: impl FnOnce(
                &WorkspaceState,
                &mut crate::SessionHandle,
                Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
            ) -> T
            + Send
            + 'static,
    ) -> Result<NativeAccess<T>, StoreError> {
        let owner = self.clone();
        tokio::task::spawn_blocking(move || {
            // Publication retains lane/lease/budget even if its waiter disappears.
            let _completion = completion;
            let _projection = owner.state.projection_lock.lock().unwrap();
            let Ok(_transition) = crate::runtime::prompt_authority::transition_lock().try_lock()
            else {
                return NativeAccess::Busy;
            };
            let workspace = owner.state.workspace.lock().unwrap();
            let Some(ws) = workspace.as_ref() else {
                return NativeAccess::Obsolete;
            };
            if !owner.workspace.matches(ws) {
                return NativeAccess::Obsolete;
            }
            let mut sessions = owner.state.sessions.lock().unwrap();
            let Some(h) = sessions.get_mut(&id) else {
                return NativeAccess::Obsolete;
            };
            #[cfg(test)]
            if let Some(sent) = owner.before_callback.lock().unwrap().take() {
                let _ = sent.send(());
            }
            crate::runtime::prompt_authority::registry().with_snapshot(&id, |authority| {
                if view
                    .as_ref()
                    .is_some_and(|view| !view.is_current(h, &authority))
                {
                    return NativeAccess::Obsolete;
                }
                NativeAccess::Current(apply(ws, h, authority))
            })
        })
        .await
        .map_err(|_| StoreError::Unavailable)
    }
    pub(crate) async fn native(
        self: &Arc<Self>,
        id: String,
        effect: IssuedEffect,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Outcome, StoreError> {
        self.native_inner(id, effect, cancelled, COORDINATED_METADATA_WRITERS, None)
            .await
    }
    #[cfg(test)]
    pub(crate) async fn native_coordinated_for_test(
        self: &Arc<Self>,
        id: String,
        effect: IssuedEffect,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Outcome, StoreError> {
        self.native_inner(id, effect, cancelled, true, None).await
    }
    async fn native_inner(
        self: &Arc<Self>,
        id: String,
        effect: IssuedEffect,
        cancelled: Arc<AtomicBool>,
        coordinated: bool,
        origin: Option<NativeOrigin>,
    ) -> Result<Outcome, StoreError> {
        self.native_attempt(id, effect, cancelled, coordinated, origin)
            .await
            .map_err(|error| match error {
                NativeAttemptError::Store(error) => error,
                NativeAttemptError::Retry => StoreError::Conflict,
            })
    }
    async fn native_attempt(
        self: &Arc<Self>,
        id: String,
        effect: IssuedEffect,
        cancelled: Arc<AtomicBool>,
        coordinated: bool,
        origin: Option<NativeOrigin>,
    ) -> Result<Outcome, NativeAttemptError> {
        // Read without runtime guards. An exact revision precondition prevents
        // native identity/provenance changing between this read and SQL commit.
        let document = self
            .store
            .get(id.clone())
            .await?
            .ok_or(StoreError::Missing)?;
        let record: metadata::session_transactions::RevisionedSession =
            serde_json::from_value(document).map_err(StoreError::from)?;
        let revision = record.revision;
        let captured = self
            .native_access(id.clone(), None, None, move |ws, h, authority| {
                // An observer effect keeps the owner captured before this read;
                // admission must not adopt a replacement controller or authority.
                if origin.as_ref().is_some_and(|origin| {
                    !origin.context.matches(h, &authority)
                        || origin
                            .capability
                            .as_ref()
                            .is_some_and(|capability| !capability.is_current())
                        || origin
                            .operation
                            .as_ref()
                            .is_some_and(|phase| phase.is_cancelled())
                }) {
                    return None;
                }
                let (operation, capability) =
                    origin.map_or((None, None), |origin| (origin.operation, origin.capability));
                NativeCapture::capture(h, &record.metadata, effect, cancelled, coordinated).map(
                    |capture| {
                        (
                            NativeView {
                                capability,
                                capture,
                                authority,
                                operation,
                            },
                            ws.lease.clone(),
                        )
                    },
                )
            })
            .await?;
        let (view, lease) = match captured {
            NativeAccess::Current(Some(captured)) => captured,
            NativeAccess::Current(None) | NativeAccess::Obsolete => {
                return Err(StoreError::Conflict.into())
            }
            NativeAccess::Busy => return Err(NativeAttemptError::Retry),
        };
        let prepared = self.store.prepare_internal_owned(
            id.clone(),
            view.capture.created.clone(),
            Some(revision),
            view.capture.proof.clone(),
            view.capture.intent(),
        )?;
        let view = Arc::new(view);
        let _cancellation = prepared.cancellation();
        let lane = prepared.lane()?;
        #[cfg(test)]
        if let Some(sent) = self.before_hook_lane.lock().unwrap().take() {
            let _ = sent.send(());
        }
        let owner = self.clone();
        let (mut result, received) = oneshot::channel();
        tokio::spawn(async move {
            let lane = tokio::select! {
                biased;
                _ = result.closed() => return,
                lane = tokio::time::timeout_at(prepared.deadline().into(), lane.lock_owned()) => match lane {
                    Ok(lane) => lane,
                    Err(_) => { let _ = result.send(Err(StoreError::Expired.into())); return; }
                }
            };
            let execution = async {
                let (ticket, ready) = prepared.enqueue(RuntimeWriteResources::new(lane, lease))?;
                let mut finished = Box::pin(ticket.finish());
                let mut delivered = None;
                let request = tokio::select! {
                    biased;
                    _ = result.closed() => return Err(NativeAttemptError::Store(StoreError::Expired)),
                    completion = &mut finished => { delivered = Some(completion); None },
                    ready = ready => ready.ok(),
                };
                let mut retry = false;
                if let Some(request) = request {
                    let claiming = view.clone();
                    let mut claiming = Box::pin(owner.native_access(id.clone(), None, None, move |_, h, authority| {
                        if !claiming.is_current(h, &authority) { return Err(StoreError::Conflict); }
                        let grant = request.grant()?;
                        if claiming.operation.as_ref().is_some_and(|phase| !phase.mark_claimed()) {
                            return Err(StoreError::Conflict);
                        }
                        Ok(grant)
                    }));
                    let claimed = tokio::select! {
                        biased;
                        completion = &mut finished => {
                            match completion {
                                Err(StoreError::Conflict) => delivered = Some(Err(StoreError::Conflict)),
                                Err(error) => return Err(error.into()),
                                Ok(_) => return Err(StoreError::Unavailable.into()),
                            }
                            claiming.await?
                        }
                        claimed = &mut claiming => claimed?,
                    };
                    // The callback can drop its request before its result arrives.
                    // Await that result before classifying the worker rejection.
                    match claimed {
                        NativeAccess::Current(Ok(grant)) => { grant.execute()?; },
                        NativeAccess::Current(Err(error)) => return Err(error.into()),
                        NativeAccess::Obsolete => return Err(StoreError::Conflict.into()),
                        NativeAccess::Busy => retry = true,
                    }
                }
                let completion = match delivered { Some(completion) => completion, None => finished.await };
                if retry {
                    match completion {
                        Err(StoreError::Conflict | StoreError::Expired) => return Err(NativeAttemptError::Retry),
                        Err(error) => return Err(error.into()),
                        Ok(_) => return Err(StoreError::Unavailable.into()),
                    }
                }
                let completion = completion?;
                if completion.outcome.document.is_none() { return Ok(Outcome::Noop { permanent_hold: false }); }
                let Some(proof) = completion.outcome.attention_proof.clone() else { return Ok(Outcome::CommittedDetached); };
                let retry_on_busy = view.capability.is_some();
                let publishing = view.clone();
                let published = owner.native_access(id.clone(), None, Some(completion), move |_, h, authority| {
                    if !publishing.is_current(h, &authority) { return false; }
                    publishing.capture.publish(h, proof);
                    true
                }).await;
                if retry_on_busy && matches!(published, Ok(NativeAccess::Busy)) { return Err(NativeAttemptError::Retry); }
                if matches!(published, Ok(NativeAccess::Current(true))) {
                    owner.state.updates.sessions_changed();
                    Ok(Outcome::Committed)
                } else {
                    Ok(Outcome::CommittedDetached)
                }
            }.await;
            let _ = result.send(execution);
        });
        received.await.map_err(|_| StoreError::Unavailable)?
    }
}

pub(super) mod observer;

pub(super) mod hook;
