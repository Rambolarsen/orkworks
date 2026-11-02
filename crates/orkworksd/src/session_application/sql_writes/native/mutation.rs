//! Shared bounded native metadata execution; callers own typed validation and publication.
use super::*;
use crate::runtime::codex_approval_application::sql_context::NativeIdentityContext;
use crate::runtime::terminal_runtime::verify_workflow_report_token;
#[derive(Clone)]
pub(super) enum NativeMetadataOrigin {
    Hook(Arc<ObservationContext>),
    Identity(Arc<NativeIdentityContext>),
}
impl From<Arc<ObservationContext>> for NativeMetadataOrigin {
    fn from(context: Arc<ObservationContext>) -> Self {
        Self::Hook(context)
    }
}
impl From<Arc<NativeIdentityContext>> for NativeMetadataOrigin {
    fn from(context: Arc<NativeIdentityContext>) -> Self {
        Self::Identity(context)
    }
}
impl NativeMetadataOrigin {
    fn matches(
        &self,
        h: &crate::SessionHandle,
        authority: &Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
    ) -> bool {
        match self {
            Self::Hook(context) => context.matches(h, authority),
            Self::Identity(context) => context.matches(h, authority),
        }
    }
}
impl SessionWrites {
    pub(super) async fn execute_native_metadata(
        self: &Arc<Self>,
        id: String,
        token: String,
        origin: NativeMetadataOrigin,
        prepared: crate::session_store::PreparedInternalMutation,
        phase: Arc<HookOperation>,
        required_reset: Option<String>,
    ) -> Result<Option<crate::session_store::InternalCompletion>, StoreError> {
        let _cancellation = prepared.cancellation();
        let lease_origin = origin.clone();
        let lease_id = id.clone();
        let lease_token = token.clone();
        let lease_reset = required_reset.clone();
        let lease = self
            .native_access(id.clone(), None, None, move |ws, h, authority| {
                (lease_origin.matches(h, &authority)
                    && verify_workflow_report_token(&lease_id, &lease_token)
                    && lease_reset.as_deref().is_none_or(|previous| {
                        crate::codex_session_store::native_identity_reset_is_authorized(
                            &lease_id, previous,
                        )
                    }))
                .then(|| ws.lease.clone())
                .flatten()
            })
            .await?;
        let lease = match lease {
            NativeAccess::Current(Some(lease)) => lease,
            NativeAccess::Current(None) | NativeAccess::Obsolete => {
                return Err(StoreError::Conflict)
            }
            NativeAccess::Busy => return Ok(None),
        };
        let lane = prepared.lane()?;
        #[cfg(test)]
        if let Some(sent) = self.before_hook_lane.lock().unwrap().take() {
            let _ = sent.send(());
        }
        let lane = tokio::time::timeout_at(prepared.deadline().into(), lane.lock_owned())
            .await
            .map_err(|_| StoreError::Expired)?;
        let (ticket, ready) = prepared.enqueue(RuntimeWriteResources::new(lane, Some(lease)))?;
        let mut finished = Box::pin(ticket.finish());
        let mut delivered = None;
        let request = tokio::select! {
            biased;
            completion = &mut finished => { delivered = Some(completion); None },
            ready = ready => ready.ok(),
        };
        let mut retry = false;
        if let Some(request) = request {
            let claiming = origin.clone();
            let claim_id = id.clone();
            let claim_token = token.clone();
            let mut claiming =
                Box::pin(
                    self.native_access(id.clone(), None, None, move |_, h, authority| {
                        if !claiming.matches(h, &authority)
                            || !verify_workflow_report_token(&claim_id, &claim_token)
                            || required_reset.as_deref().is_some_and(|previous| {
                                !crate::codex_session_store::native_identity_reset_is_authorized(
                                    &claim_id, previous,
                                )
                            })
                        {
                            return Ok(None);
                        }
                        let grant = request.grant()?;
                        if !phase.mark_claimed() {
                            return Ok(None);
                        }
                        Ok(Some(grant))
                    }),
                );
            let claimed = tokio::select! {
                biased;
                completion = &mut finished => {
                    match completion {
                        Err(StoreError::Conflict) => delivered = Some(Err(StoreError::Conflict)),
                        Err(error) => return Err(error),
                        Ok(_) => return Err(StoreError::Unavailable),
                    }
                    claiming.await?
                }
                claimed = &mut claiming => claimed?,
            };
            match claimed {
                NativeAccess::Current(Ok(Some(grant))) => grant.execute()?,
                NativeAccess::Current(Ok(None)) | NativeAccess::Obsolete => {}
                NativeAccess::Current(Err(error)) => return Err(error),
                NativeAccess::Busy => retry = true,
            }
        }
        let completion = match delivered {
            Some(result) => result,
            None => finished.await,
        };
        if retry {
            // No grant escaped the unavailable guard callback. Drain the worker's
            // rejection before releasing the operation, without calling that
            // deliberate refusal a database conflict or queue expiry.
            match completion {
                Err(StoreError::Conflict | StoreError::Expired) => {}
                Err(error) => return Err(error),
                Ok(_) => return Err(StoreError::Unavailable),
            }
            return Ok(None);
        }
        Ok(Some(completion?))
    }
}
