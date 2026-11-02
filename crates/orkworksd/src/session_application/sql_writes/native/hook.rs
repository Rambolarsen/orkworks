//! Source-only private hook application; production relay routing remains JSON.
use super::*;
use crate::metadata::internal_mutations::HookTransition;
use crate::metadata::session_transactions::RevisionedSession;
use crate::runtime::codex_approval_application::{
    self as approval, hook_context, HookReportOutcome, NativeHookReport,
};
use crate::runtime::terminal_runtime::verify_workflow_report_token;
use std::time::Instant;
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SqlHookOutcome {
    Accepted,
    Rejected,
    Retry,
    CommittedDetached,
}
struct HookCancellation(Arc<HookOperation>);
impl Drop for HookCancellation {
    fn drop(&mut self) {
        self.0.cancel_before_claim();
    }
}
fn obsolete(committed: bool) -> SqlHookOutcome {
    if committed {
        SqlHookOutcome::CommittedDetached
    } else {
        SqlHookOutcome::Rejected
    }
}
struct PreparedHook {
    context: Arc<ObservationContext>,
    state: hook_context::PreparedHook,
    transition: Option<HookTransition>,
}
struct HookStage {
    record: RevisionedSession,
    context: Option<Arc<ObservationContext>>,
    committed: bool,
    retry: bool,
}
impl SessionWrites {
    /// A validated report can perform multiple scoped writes. An error after
    /// an earlier commit does not imply rollback; retry only through fresh
    /// validation/reads, retaining the report's original first receipt.
    pub(crate) async fn report_native_hook(
        self: &Arc<Self>,
        id: String,
        token: String,
        generation: u64,
        first_receipt: Instant,
        report: NativeHookReport,
        cancelled: Arc<AtomicBool>,
    ) -> Result<SqlHookOutcome, StoreError> {
        let owner = self.clone();
        let (mut result, received) = oneshot::channel();
        // One operation cutoff: dropping the caller before any claim competes
        // with that claim. After it wins, finish every stage definitively.
        let phase = Arc::new(HookOperation::default());
        let _cancellation = HookCancellation(phase.clone());
        tokio::spawn(async move {
            let mut running = Box::pin(owner.report_native_hook_inner(
                id,
                token,
                generation,
                first_receipt,
                report,
                cancelled,
                phase.clone(),
            ));
            let outcome = tokio::select! {
                biased;
                _ = result.closed() => {
                    if phase.cancel_before_claim() { return; }
                    running.await
                }
                outcome = &mut running => outcome,
            };
            let _ = result.send(outcome);
        });
        received.await.map_err(|_| StoreError::Unavailable)?
    }
    async fn report_native_hook_inner(
        self: &Arc<Self>,
        id: String,
        token: String,
        generation: u64,
        first_receipt: Instant,
        report: NativeHookReport,
        cancelled: Arc<AtomicBool>,
        phase: Arc<HookOperation>,
    ) -> Result<SqlHookOutcome, StoreError> {
        if !verify_workflow_report_token(&id, &token) {
            return Ok(SqlHookOutcome::Rejected);
        }
        let fingerprint = tokio::task::spawn_blocking(approval::expected_fingerprint)
            .await
            .map_err(|_| StoreError::Unavailable)?;
        let checking_id = id.clone();
        let checking_token = token.clone();
        let cancellation = cancelled.clone();
        let context = self
            .native_current(id.clone(), None, None, move |_, h, authority| {
                if !verify_workflow_report_token(&checking_id, &checking_token) {
                    return None;
                }
                ObservationContext::capture_hook(h, generation, cancellation, authority)
            })
            .await?;
        let context = match context {
            Some(Some(context)) => Arc::new(context),
            Some(None) => return Ok(SqlHookOutcome::Rejected),
            None => return Ok(SqlHookOutcome::Retry),
        };
        let document = self
            .store
            .get(id.clone())
            .await?
            .ok_or(StoreError::Missing)?;
        let mut record: RevisionedSession = serde_json::from_value(document)?;
        let preparing = context.clone();
        let preparing_id = id.clone();
        let preparing_token = token.clone();
        let preparing_report = report.clone();
        let meta = record.metadata.clone();
        let prepared = self
            .native_current(id.clone(), None, None, move |_, h, authority| {
                if !preparing.matches(h, &authority)
                    || !verify_workflow_report_token(&preparing_id, &preparing_token)
                    || meta.id != h.info.id
                    || meta.created_at != h.info.created_at
                {
                    return Err(HookReportOutcome::Rejected);
                }
                let Some(mut tracker) = h.runtime.native_approval.take() else {
                    return Err(HookReportOutcome::Rejected);
                };
                let prepared = hook_context::prepare(
                    &mut tracker,
                    h,
                    &meta,
                    first_receipt,
                    &preparing_report,
                    fingerprint.as_deref(),
                );
                h.runtime.native_approval = Some(tracker);
                let state = prepared?;
                match h
                    .info
                    .resume
                    .as_ref()
                    .and_then(|r| r.harness_session_id.as_deref())
                {
                    None => return Err(HookReportOutcome::Retry),
                    Some(root) if root != preparing_report.root_id => {
                        return Err(HookReportOutcome::Rejected)
                    }
                    _ => {}
                }
                let transition = if (state.activation || state.boundary)
                    && meta.attention_source() != "user"
                    && h.info
                        .provenance
                        .attention_source
                        .as_deref()
                        .or(h.info.metadata_source.as_deref())
                        != Some("user")
                {
                    Some(match preparing_report.event {
                        crate::runtime::codex_approval::HookEvent::Stop => HookTransition::Stop,
                        crate::runtime::codex_approval::HookEvent::UserPromptSubmit => {
                            HookTransition::UserPromptSubmit
                        }
                        _ => HookTransition::Activate,
                    })
                } else {
                    None
                };
                let context = ObservationContext::capture_hook(
                    h,
                    generation,
                    preparing.cancellation(),
                    authority,
                )
                .ok_or(HookReportOutcome::Rejected)?;
                Ok(PreparedHook {
                    context: Arc::new(context),
                    state,
                    transition,
                })
            })
            .await?;
        let prepared = match prepared {
            Some(Ok(prepared)) => prepared,
            Some(Err(HookReportOutcome::Retry)) | None => return Ok(SqlHookOutcome::Retry),
            _ => return Ok(SqlHookOutcome::Rejected),
        };
        let mut context = prepared.context;
        let mut committed = false;
        if let Some(transition) = prepared.transition {
            let stage = self
                .hook_metadata(
                    id.clone(),
                    token.clone(),
                    context.clone(),
                    record,
                    transition,
                    phase.clone(),
                )
                .await?;
            if stage.retry {
                return Ok(SqlHookOutcome::Retry);
            }
            committed |= stage.committed;
            record = stage.record;
            let Some(current) = stage.context else {
                return Ok(obsolete(committed));
            };
            context = current;
        }
        let advancing = context.clone();
        let advancing_id = id.clone();
        let advancing_token = token.clone();
        let advancing_report = report.clone();
        let meta = record.metadata.clone();
        let advanced = self
            .native_access(id.clone(), None, None, move |_, h, authority| {
                if !advancing.matches(h, &authority)
                    || !verify_workflow_report_token(&advancing_id, &advancing_token)
                {
                    return None;
                }
                hook_context::advance_sql(
                    h,
                    &meta,
                    &advancing_report,
                    &prepared.state,
                    first_receipt,
                    generation,
                    advancing.cancellation(),
                    authority,
                )
            })
            .await?;
        let (context, effects, fallback) = match advanced {
            NativeAccess::Current(Some(advanced)) => advanced,
            NativeAccess::Current(None) | NativeAccess::Obsolete => return Ok(obsolete(committed)),
            NativeAccess::Busy => return Ok(SqlHookOutcome::Retry),
        };
        if fallback {
            let stage = self
                .hook_metadata(
                    id,
                    token,
                    context,
                    record,
                    HookTransition::FallbackPermission,
                    phase,
                )
                .await?;
            if stage.retry {
                return Ok(SqlHookOutcome::Retry);
            }
            committed |= stage.committed;
            return Ok(if stage.context.is_some() {
                SqlHookOutcome::Accepted
            } else {
                obsolete(committed)
            });
        }
        for effect in effects {
            match self
                .native_attempt(
                    id.clone(),
                    effect,
                    context.cancellation(),
                    COORDINATED_METADATA_WRITERS,
                    Some(NativeOrigin {
                        capability: Some(HookCapability {
                            id: id.clone(),
                            token: token.clone(),
                        }),
                        context: context.clone(),
                        operation: Some(phase.clone()),
                    }),
                )
                .await
            {
                Ok(Outcome::Committed) => {}
                Ok(Outcome::CommittedDetached) => return Ok(SqlHookOutcome::CommittedDetached),
                Ok(Outcome::Noop { .. })
                    if report.event
                        == crate::runtime::codex_approval::HookEvent::PermissionRequest =>
                {
                    // A prior ShowWait may be durable without a live acknowledgement.
                    // Fresh authenticated evidence can publish a conservative hook
                    // wait through a new scoped write, never by adopting its proof.
                    let document = self
                        .store
                        .get(id.clone())
                        .await?
                        .ok_or(StoreError::Missing)?;
                    let record: RevisionedSession = serde_json::from_value(document)?;
                    let checking = context.clone();
                    let checking_id = id.clone();
                    let checking_token = token.clone();
                    let metadata = record.metadata.clone();
                    match self
                        .native_access(id.clone(), None, None, move |_, h, authority| {
                            if !checking.matches(h, &authority)
                                || !verify_workflow_report_token(&checking_id, &checking_token)
                            {
                                return None;
                            }
                            Some(hook_context::fallback_allowed(h, &metadata))
                        })
                        .await?
                    {
                        NativeAccess::Busy => return Ok(SqlHookOutcome::Retry),
                        NativeAccess::Obsolete | NativeAccess::Current(None) => {
                            return Ok(obsolete(committed))
                        }
                        NativeAccess::Current(Some(false)) => return Ok(SqlHookOutcome::Accepted),
                        NativeAccess::Current(Some(true)) => {}
                    }
                    let stage = self
                        .hook_metadata(
                            id,
                            token,
                            context,
                            record,
                            HookTransition::FallbackPermission,
                            phase,
                        )
                        .await?;
                    if stage.retry {
                        return Ok(SqlHookOutcome::Retry);
                    }
                    return Ok(if stage.context.is_some() {
                        SqlHookOutcome::Accepted
                    } else {
                        obsolete(committed || stage.committed)
                    });
                }
                Ok(Outcome::Noop { .. }) => {}
                Err(NativeAttemptError::Retry) => return Ok(SqlHookOutcome::Retry),
                Err(NativeAttemptError::Store(error)) => return Err(error),
            }
        }
        Ok(SqlHookOutcome::Accepted)
    }
    async fn hook_metadata(
        self: &Arc<Self>,
        id: String,
        token: String,
        origin: Arc<ObservationContext>,
        record: RevisionedSession,
        transition: HookTransition,
        phase: Arc<HookOperation>,
    ) -> Result<HookStage, StoreError> {
        let intent = InternalMutation::HookTransition {
            root: origin.root().ok_or(StoreError::Conflict)?.into(),
            transition,
            timestamp: crate::workspace_runtime::iso_now(),
        };
        let prepared = self.store.prepare_internal_owned(
            id.clone(),
            record.metadata.created_at.clone(),
            Some(record.revision.clone()),
            None,
            intent,
        )?;
        let _cancellation = prepared.cancellation();
        let lease_origin = origin.clone();
        let lease_id = id.clone();
        let lease_token = token.clone();
        let lease = self
            .native_access(id.clone(), None, None, move |ws, h, authority| {
                (lease_origin.matches(h, &authority)
                    && verify_workflow_report_token(&lease_id, &lease_token))
                .then(|| ws.lease.clone())
                .flatten()
            })
            .await?;
        let lease = match lease {
            NativeAccess::Current(Some(lease)) => lease,
            NativeAccess::Current(None) | NativeAccess::Obsolete => {
                return Err(StoreError::Conflict)
            }
            NativeAccess::Busy => {
                return Ok(HookStage {
                    record,
                    context: None,
                    committed: false,
                    retry: true,
                })
            }
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
            return Ok(HookStage {
                record,
                context: None,
                committed: false,
                retry: true,
            });
        }
        let completion = completion?;
        let changed = completion.outcome.document.is_some();
        let record = match completion.outcome.document.as_ref() {
            Some(document) => serde_json::from_value(document.clone())?,
            None => record,
        };
        let publishing = origin.clone();
        let publishing_id = id.clone();
        let metadata = record.metadata.clone();
        let context = self
            .native_access(id, None, Some(completion), move |_, h, authority| {
                if !publishing.matches(h, &authority)
                    || !verify_workflow_report_token(&publishing_id, &token)
                {
                    return None;
                }
                if changed {
                    hook_context::publish_transition(h, &metadata, transition);
                }
                ObservationContext::capture_hook(
                    h,
                    h.runtime.run_generation(),
                    publishing.cancellation(),
                    authority,
                )
                .map(Arc::new)
            })
            .await?;
        if changed {
            self.state.updates.sessions_changed();
        }
        let (context, retry) = match context {
            NativeAccess::Current(context) => (context, false),
            NativeAccess::Busy => (None, true),
            NativeAccess::Obsolete => (None, false),
        };
        Ok(HookStage {
            record,
            context,
            committed: changed,
            retry,
        })
    }
}
