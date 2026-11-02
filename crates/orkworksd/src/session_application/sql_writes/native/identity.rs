//! Source-only authenticated native identity application.
use super::*;
use crate::runtime::codex_approval_application::sql_context::NativeIdentityContext;
#[derive(Clone, Debug)]
pub(crate) struct NativeIdentityReport {
    pub(crate) report: crate::metadata::HarnessSessionReport,
    pub(crate) session_start_source: Option<String>,
    pub(crate) session_start_event: Option<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SqlIdentityOutcome {
    Accepted(crate::metadata::HarnessSessionMergeResult),
    Rejected,
    Retry,
    CommittedDetached,
}
impl SessionWrites {
    /// Accept only the authenticated private Codex runtime boundary. Production
    /// HTTP routing and external label/trust follow-ups retain their owners.
    pub(crate) async fn report_native_identity(
        self: &Arc<Self>,
        id: String,
        token: String,
        generation: u64,
        report: NativeIdentityReport,
        cancelled: Arc<AtomicBool>,
    ) -> Result<SqlIdentityOutcome, StoreError> {
        let owner = self.clone();
        let phase = Arc::new(HookOperation::default());
        let _cancellation = HookCancellation(phase.clone());
        let (mut result, received) = oneshot::channel();
        tokio::spawn(async move {
            let mut running = Box::pin(owner.report_native_identity_inner(
                id,
                token,
                generation,
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
    async fn report_native_identity_inner(
        self: &Arc<Self>,
        id: String,
        token: String,
        generation: u64,
        report: NativeIdentityReport,
        cancelled: Arc<AtomicBool>,
        phase: Arc<HookOperation>,
    ) -> Result<SqlIdentityOutcome, StoreError> {
        use crate::metadata::session_transactions::RevisionedSession;
        use crate::metadata::{
            internal_mutations::Disposition, HarnessSessionMergeResult as Merge,
        };
        if report.report.source != "codex_hook"
            || !crate::metadata::valid_harness_session_report_context(
                &report.report,
                report.session_start_source.as_deref(),
                report.session_start_event.as_deref(),
            )
            || !crate::runtime::terminal_runtime::verify_workflow_report_token(&id, &token)
        {
            return Ok(SqlIdentityOutcome::Rejected);
        }
        let label_cancelled = cancelled.clone();
        let capture_id = id.clone();
        let capture_token = token.clone();
        let context = match self
            .native_access(id.clone(), None, None, move |_, h, authority| {
                if !crate::runtime::terminal_runtime::verify_workflow_report_token(
                    &capture_id,
                    &capture_token,
                ) {
                    return None;
                }
                NativeIdentityContext::capture(h, generation, cancelled, authority).map(Arc::new)
            })
            .await?
        {
            NativeAccess::Current(Some(context)) => context,
            NativeAccess::Busy => return Ok(SqlIdentityOutcome::Retry),
            _ => return Ok(SqlIdentityOutcome::Rejected),
        };
        let record: RevisionedSession = serde_json::from_value(
            self.store
                .get(id.clone())
                .await?
                .ok_or(StoreError::Missing)?,
        )?;
        let checking = context.clone();
        let checking_id = id.clone();
        let checking_token = token.clone();
        let metadata = record.metadata.clone();
        let clear = report.session_start_source.as_deref() == Some("clear")
            && report.session_start_event.as_deref() == Some("SessionStart");
        let required_reset = match self
            .native_access(id.clone(), None, None, move |_, h, authority| {
                if !checking.matches(h, &authority)
                    || !crate::runtime::terminal_runtime::verify_workflow_report_token(
                        &checking_id,
                        &checking_token,
                    )
                    || metadata.id != h.info.id
                    || metadata.created_at != h.info.created_at
                    || metadata.harness != "codex"
                    || metadata.lifecycle != "alive"
                    || metadata.lifecycle_phase != "active"
                {
                    return None;
                }
                Some(
                    metadata
                        .resume
                        .as_ref()
                        .and_then(|r| r.harness_session_id.as_ref())
                        .filter(|previous| {
                            clear
                                && crate::codex_session_store::native_identity_reset_is_authorized(
                                    &checking_id,
                                    previous,
                                )
                        })
                        .cloned(),
                )
            })
            .await?
        {
            NativeAccess::Current(Some(required_reset)) => required_reset,
            NativeAccess::Busy => return Ok(SqlIdentityOutcome::Retry),
            _ => return Ok(SqlIdentityOutcome::Rejected),
        };
        let prepared = self.store.prepare_internal_owned(
            id.clone(),
            record.metadata.created_at.clone(),
            Some(record.revision.clone()),
            None,
            InternalMutation::HarnessReport {
                report: report.report,
                timestamp: crate::workspace_runtime::iso_now(),
                allow_codex_identity_replacement: required_reset.is_some(),
            },
        )?;
        let Some(completion) = self
            .execute_native_metadata(
                id.clone(),
                token.clone(),
                context.clone().into(),
                prepared,
                phase,
                required_reset,
            )
            .await?
        else {
            return Ok(SqlIdentityOutcome::Retry);
        };
        let disposition = match &completion.outcome.decision.disposition {
            Disposition::HarnessReport(result) => *result,
            _ => return Err(StoreError::Unavailable),
        };
        if !matches!(disposition, Merge::Accepted | Merge::IgnoredUnchanged) {
            return Ok(SqlIdentityOutcome::Accepted(disposition));
        }
        let changed = completion.outcome.document.is_some();
        let record: RevisionedSession = match completion.outcome.document.as_ref() {
            Some(document) => serde_json::from_value(document.clone())?,
            None => record,
        };
        let publishing_id = id.clone();
        let automatic_label = record.metadata.label_source.is_automatic();
        let identity_metadata = record.metadata.clone();
        let resume = record.metadata.resume;
        let published = self
            .native_access(id, None, Some(completion), move |_, h, authority| {
                if !context.matches(h, &authority)
                    || !crate::runtime::terminal_runtime::verify_workflow_report_token(
                        &publishing_id,
                        &token,
                    )
                {
                    return (false, None);
                }
                let old_root = h
                    .info
                    .resume
                    .as_ref()
                    .and_then(|r| r.harness_session_id.as_deref());
                let new_root = resume
                    .as_ref()
                    .and_then(|r| r.harness_session_id.as_deref());
                if old_root != new_root {
                    if let Some(tracker) = h.runtime.native_approval.as_mut() {
                        tracker.identity_changed();
                    }
                }
                if let Some(root) = new_root {
                    crate::codex_session_store::accept_native_label_identity(&publishing_id, root);
                }
                h.info.resume = resume;
                if let Some(tracker) = h.runtime.native_approval.as_mut() {
                    tracker.sync_identity(&identity_metadata);
                }
                let refresh = automatic_label
                    .then(|| {
                        super::label::NativeLabelRefresh::capture_after_identity(
                            h,
                            token,
                            generation,
                            label_cancelled,
                            authority,
                        )
                    })
                    .flatten();
                (true, refresh)
            })
            .await?;
        if changed {
            self.state.updates.sessions_changed();
        }
        Ok(match published {
            NativeAccess::Current((true, refresh)) => {
                if let Some(refresh) = refresh {
                    self.schedule_native_label_refresh(refresh);
                }
                SqlIdentityOutcome::Accepted(disposition)
            }
            NativeAccess::Busy => SqlIdentityOutcome::Retry,
            _ if changed => SqlIdentityOutcome::CommittedDetached,
            _ => SqlIdentityOutcome::Rejected,
        })
    }
}
