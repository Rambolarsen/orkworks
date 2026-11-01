//! Dormant native effect application using the same worker/session lane as Peon.
use super::*;
use crate::runtime::codex_approval::Effect;
use crate::runtime::codex_approval_application::{
    sql_context::NativeCapture, COORDINATED_METADATA_WRITERS,
};
use std::sync::atomic::AtomicBool;
struct NativeView {
    capture: NativeCapture,
    authority: Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
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
        let owner = self.clone();
        tokio::task::spawn_blocking(move || {
            // Publication retains lane/lease/budget even if its waiter disappears.
            let _completion = completion;
            let _projection = owner.state.projection_lock.lock().unwrap();
            let _transition = crate::runtime::prompt_authority::transition_lock()
                .try_lock()
                .ok()?;
            let workspace = owner.state.workspace.lock().unwrap();
            let ws = workspace.as_ref()?;
            if !owner.workspace.matches(ws) {
                return None;
            }
            let mut sessions = owner.state.sessions.lock().unwrap();
            let h = sessions.get_mut(&id)?;
            #[cfg(test)]
            if let Some(sent) = owner.before_callback.lock().unwrap().take() {
                let _ = sent.send(());
            }
            crate::runtime::prompt_authority::registry().with_snapshot(&id, |authority| {
                if view
                    .as_ref()
                    .is_some_and(|view| authority != view.authority || !view.capture.is_current(h))
                {
                    return None;
                }
                Some(apply(ws, h, authority))
            })
        })
        .await
        .map_err(|_| StoreError::Unavailable)
    }
    pub(crate) async fn native(
        self: &Arc<Self>,
        id: String,
        effect: Effect,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Outcome, StoreError> {
        self.native_inner(id, effect, cancelled, COORDINATED_METADATA_WRITERS)
            .await
    }
    #[cfg(test)]
    pub(crate) async fn native_coordinated_for_test(
        self: &Arc<Self>,
        id: String,
        effect: Effect,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Outcome, StoreError> {
        self.native_inner(id, effect, cancelled, true).await
    }
    async fn native_inner(
        self: &Arc<Self>,
        id: String,
        effect: Effect,
        cancelled: Arc<AtomicBool>,
        coordinated: bool,
    ) -> Result<Outcome, StoreError> {
        // Read without runtime guards. An exact revision precondition prevents
        // native identity/provenance changing between this read and SQL commit.
        let document = self
            .store
            .get(id.clone())
            .await?
            .ok_or(StoreError::Missing)?;
        let record: metadata::session_transactions::RevisionedSession =
            serde_json::from_value(document)?;
        let revision = record.revision;
        let (view, lease) = self
            .native_current(id.clone(), None, None, move |ws, h, authority| {
                NativeCapture::capture(h, &record.metadata, effect, cancelled, coordinated)
                    .map(|capture| (NativeView { capture, authority }, ws.lease.clone()))
            })
            .await?
            .flatten()
            .ok_or(StoreError::Conflict)?;
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
        let owner = self.clone();
        let (mut result, received) = oneshot::channel();
        tokio::spawn(async move {
            let lane = tokio::select! {
                biased;
                _ = result.closed() => return,
                lane = tokio::time::timeout_at(prepared.deadline().into(), lane.lock_owned()) => match lane {
                    Ok(lane) => lane,
                    Err(_) => { let _ = result.send(Err(StoreError::Expired)); return; }
                }
            };
            let execution = async {
                let (ticket, ready) = prepared.enqueue(RuntimeWriteResources::new(lane, lease))?;
                let mut finished = Box::pin(ticket.finish());
                let mut delivered = None;
                let request = tokio::select! {
                    biased;
                    _ = result.closed() => return Err(StoreError::Expired),
                    completion = &mut finished => { delivered = Some(completion); None },
                    ready = ready => ready.ok(),
                };
                if let Some(request) = request {
                    tokio::select! {
                        biased;
                        completion = &mut finished => delivered = Some(completion),
                        claimed = owner.native_current(id.clone(), Some(view.clone()), None, move |_, _, _| request.grant()) => {
                            // No shared application guards remain when SQL wakes.
                            if let Some(Ok(grant)) = claimed? { let _ = grant.execute(); }
                        }
                    }
                }
                let completion = match delivered { Some(completion) => completion?, None => finished.await? };
                if completion.outcome.document.is_none() { return Ok(Outcome::Noop { permanent_hold: false }); }
                let Some(proof) = completion.outcome.attention_proof.clone() else { return Ok(Outcome::CommittedDetached); };
                let publishing = view.clone();
                let published = owner.native_current(id.clone(), Some(view), Some(completion), move |_, h, _| publishing.capture.publish(h, proof)).await;
                if matches!(published, Ok(Some(()))) {
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
