//! Source-only async SQL application boundary. Format selection remains gated.
//! All conflicting writers must use this owner before SQL can be selected.
use super::*;
use crate::metadata::internal_mutations::InternalMutation;
use crate::runtime::observed_status::AttentionWriteToken;
use crate::runtime::peon_runtime::{AttemptContextView, PeonOutputCaptureIdentity};
use crate::session_store::{RuntimeWriteResources, SessionStore, StoreError};
use tokio::sync::oneshot;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Noop { permanent_hold: bool },
    Committed,
    CommittedDetached,
}

pub(crate) struct SessionWrites {
    state: Arc<AppState>,
    store: Arc<SessionStore>,
    workspace: RecommendationDeliveryWorkspace,
    #[cfg(test)]
    before_callback: std::sync::Mutex<Option<oneshot::Sender<()>>>,
}
struct Capture {
    created: String,
    attention: AttentionWriteToken,
    provenance: metadata::MetadataProvenance,
    metadata_source: Option<String>,
    metadata_confidence: Option<f64>,
    committed_input_sequence: u64,
    active_work_hook: bool,
    terminal_status_active: bool,
    prompt_kind: Option<crate::runtime::session_runtime::RuntimePromptKind>,
    protected_permission: bool,
    prompt_authority: Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
    control: tokio::sync::mpsc::Sender<crate::runtime::session_runtime::RuntimeCommand>,
}
impl SessionWrites {
    pub(crate) fn bind(
        state: Arc<AppState>,
        store: Arc<SessionStore>,
    ) -> Result<Arc<Self>, StoreError> {
        let workspace = {
            let guard = state.workspace.lock().unwrap();
            let ws = guard.as_ref().ok_or(StoreError::Conflict)?;
            if ws.lease.is_none() {
                return Err(StoreError::Conflict);
            }
            let (root, path) = store.location();
            if root != ws.metadata.root_path() || path != ws.path.to_string_lossy() {
                return Err(StoreError::Conflict);
            }
            RecommendationDeliveryWorkspace::capture(ws)
        };
        Ok(Arc::new(Self {
            state,
            store,
            workspace,
            #[cfg(test)]
            before_callback: std::sync::Mutex::new(None),
        }))
    }
    #[cfg(test)]
    pub(crate) fn test_before_callback(&self) -> oneshot::Receiver<()> {
        let (sent, received) = oneshot::channel();
        *self.before_callback.lock().unwrap() = Some(sent);
        received
    }
    /// No filesystem or SQL work here; callers hold these guards only for
    /// bounded identity validation, the execution claim, or live publication.
    fn current<T>(
        &self,
        attempt: &AttemptContextView,
        output: &PeonOutputCaptureIdentity,
        captured: Option<&Capture>,
        apply: impl FnOnce(
            &WorkspaceState,
            &mut crate::SessionHandle,
            Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
        ) -> T,
    ) -> Option<T> {
        if !Arc::ptr_eq(attempt.state(), &self.state) {
            return None;
        }
        let _projection = self.state.projection_lock.lock().unwrap();
        // Native identity/reset writers share this transition mutex. Never
        // wait for it while holding projection: contention rejects this view.
        let _transition = crate::runtime::prompt_authority::transition_lock()
            .try_lock()
            .ok()?;
        let workspace = self.state.workspace.lock().unwrap();
        let ws = workspace.as_ref()?;
        if !self.workspace.matches(ws) {
            return None;
        }
        let mut sessions = self.state.sessions.lock().unwrap();
        let handle = sessions.get_mut(attempt.session_id())?;
        if !output.is_current(
            handle.runtime.input_generation,
            handle.runtime.min_peon_output_revision,
            &handle.runtime.identity(),
        ) || handle.info.lifecycle != "alive"
            || captured.is_some_and(|c| {
                handle.info.created_at != c.created
                    || !handle.runtime.attention_owner.owns(&c.attention)
                    || handle.info.provenance != c.provenance
                    || handle.info.metadata_source != c.metadata_source
                    || handle.info.metadata_confidence != c.metadata_confidence
                    || handle.runtime.committed_input_sequence != c.committed_input_sequence
                    || handle.active_work_hook != c.active_work_hook
                    || handle.runtime.codex_terminal_status_active != c.terminal_status_active
                    || handle.runtime.active_prompt_kind != c.prompt_kind
                    || protected_permission(handle) != c.protected_permission
                    || !handle.runtime.control_tx.same_channel(&c.control)
            })
        {
            return None;
        }
        #[cfg(test)]
        if let Some(sent) = self.before_callback.lock().unwrap().take() {
            let _ = sent.send(());
        }
        crate::runtime::prompt_authority::registry().with_snapshot(
            attempt.session_id(),
            |authority| {
                if captured.is_some_and(|c| authority != c.prompt_authority) {
                    return None;
                }
                attempt.with_current_session(handle, |handle| apply(ws, handle, authority))
            },
        )
    }
    async fn current_async<T: Send + 'static>(
        self: &Arc<Self>,
        attempt: &AttemptContextView,
        output: &PeonOutputCaptureIdentity,
        captured: Option<Arc<Capture>>,
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
        let attempt = attempt.clone();
        let output = output.clone();
        tokio::task::spawn_blocking(move || {
            // Keep publication resources with the blocking job itself, even
            // if its async waiter is dropped. current() releases its guards
            // before this outer scope releases the lane/lease/budget.
            let _completion = completion;
            owner.current(&attempt, &output, captured.as_deref(), apply)
        })
        .await
        .map_err(|_| StoreError::Unavailable)
    }
    pub(crate) async fn peon(
        self: &Arc<Self>,
        attempt: AttemptContextView,
        output: PeonOutputCaptureIdentity,
        intent: InternalMutation,
    ) -> Result<Outcome, StoreError> {
        if !matches!(intent, InternalMutation::Peon { .. }) {
            return Err(StoreError::Invalid(
                "only Peon metadata is supported by this boundary".into(),
            ));
        }
        let (capture, lease) = self
            .current_async(&attempt, &output, None, None, move |ws, h, authority| {
                h.runtime.attention_owner.snapshot().map(|attention| {
                    (
                        Capture {
                            created: h.info.created_at.clone(),
                            attention,
                            provenance: h.info.provenance.clone(),
                            metadata_source: h.info.metadata_source.clone(),
                            metadata_confidence: h.info.metadata_confidence,
                            committed_input_sequence: h.runtime.committed_input_sequence,
                            active_work_hook: h.active_work_hook,
                            terminal_status_active: h.runtime.codex_terminal_status_active,
                            prompt_kind: h.runtime.active_prompt_kind,
                            protected_permission: protected_permission(h),
                            prompt_authority: authority,
                            control: h.runtime.control_tx.clone(),
                        },
                        ws.lease.clone(),
                    )
                })
            })
            .await?
            .flatten()
            .ok_or(StoreError::Conflict)?;
        let capture = Arc::new(capture);
        let prepared = self.store.prepare_internal(
            attempt.session_id().into(),
            capture.created.clone(),
            intent,
        )?;
        let _cancellation = prepared.cancellation();
        let lane = prepared.lane()?;
        let owner = self.clone();
        let (mut result, received) = oneshot::channel();
        // Before execution the caller can cancel. After the grant claims
        // execution this owner, and then the worker, retain lane/lease/result.
        tokio::spawn(async move {
            let deadline = prepared.deadline();
            let lane = tokio::select! {
                biased;
                _ = result.closed() => return,
                lane = tokio::time::timeout_at(deadline.into(), lane.lock_owned()) => match lane {
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
                    completion = &mut finished => {
                        delivered = Some(completion);
                        None
                    },
                    ready = ready => ready.ok(),
                };
                if let Some(request) = request {
                    // A blocking validation may outlive queued expiry. Always
                    // consume the definitive worker result concurrently so an
                    // expired delivery releases its lane/lease/budget promptly.
                    tokio::select! {
                        biased;
                        completion = &mut finished => delivered = Some(completion),
                        claimed = owner.current_async(&attempt, &output, Some(capture.clone()), None, move |_, _, _| request.grant()) => {
                            // current() has released every shared guard.
                            if let Some(Ok(grant)) = claimed? {
                                let _ = grant.execute();
                            }
                        }
                    }
                }
                let completion = match delivered {
                    Some(completion) => completion?,
                    None => finished.await?,
                };
                let Some(document) = completion.outcome.document.as_ref() else {
                    return Ok(Outcome::Noop {
                        permanent_hold: matches!(
                            completion.outcome.decision.disposition,
                            crate::metadata::internal_mutations::Disposition::PeonSkipped {
                                permanent_hold: true
                            }
                        ),
                    });
                };
                let stored: metadata::SessionMetadata = match serde_json::from_value(document.clone()) {
                    Ok(stored) => stored,
                    Err(error) => {
                        tracing::warn!(session_id = %attempt.session_id(), ?error, "committed metadata cannot be projected");
                        return Ok(Outcome::CommittedDetached);
                    }
                };
                let scope = completion.outcome.decision.scope;
                let published = owner
                    .current_async(&attempt, &output, Some(capture.clone()), Some(completion), move |_, h, _| {
                        if scope.is_some_and(|scope| scope.attention) {
                            h.runtime.attention_owner.accepted_write();
                        }
                        h.info.observed_status = stored.observed_status;
                        h.info.attention = stored.attention;
                        h.info.needs_user_input = stored.needs_user_input;
                        h.info.detected_question = stored.detected_question;
                        h.info.suggested_options = stored.suggested_options;
                        h.info.provenance = stored.provenance;
                        h.info.metadata_source = Some(stored.metadata_source);
                        h.info.metadata_confidence = Some(stored.metadata_confidence);
                    })
                    .await;
                let published = match published {
                    Ok(published) => published.is_some(),
                    Err(error) => {
                        tracing::warn!(session_id = %attempt.session_id(), ?error, "committed metadata could not be published");
                        false
                    }
                };
                if published {
                    owner.state.updates.sessions_changed();
                }
                // The blocking publication job retained the completion resources.
                Ok(if published {
                    Outcome::Committed
                } else {
                    Outcome::CommittedDetached
                })
            }
            .await;
            let _ = result.send(execution);
        });
        received.await.map_err(|_| StoreError::Unavailable)?
    }
}

fn protected_permission(handle: &crate::SessionHandle) -> bool {
    handle
        .runtime
        .native_approval
        .as_ref()
        .is_some_and(|tracker| {
            tracker.protects_permission_wait(&handle.runtime.attention_owner, &handle.info)
        })
}
