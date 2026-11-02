//! Source-only native title lookup and persistence. No JSON metadata fallback.
use super::mutation::NativeMetadataOrigin;
use super::*;
use crate::metadata::internal_mutations::Disposition;
use crate::metadata::session_transactions::RevisionedSession;
use crate::runtime::codex_approval_application::sql_context::NativeIdentityContext;
use crate::runtime::terminal_runtime::verify_workflow_report_token;

/// Process-local authority captured before any native-store lookup; never reconstructed from SQL.
pub(crate) struct NativeLabelRefresh {
    id: String,
    token: String,
    root: String,
    context: NativeIdentityContext,
    refresh: crate::codex_session_store::LabelRefreshToken,
}
impl NativeLabelRefresh {
    pub(super) fn capture_after_identity(
        h: &crate::SessionHandle,
        token: String,
        generation: u64,
        cancelled: Arc<AtomicBool>,
        authority: Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
    ) -> Option<Arc<Self>> {
        if !verify_workflow_report_token(&h.info.id, &token) {
            return None;
        }
        let root = h.info.resume.as_ref()?.harness_session_id.clone()?;
        if crate::codex_session_store::native_label_refresh_is_blocked(&h.info.id, &root) {
            return None;
        }
        let context = NativeIdentityContext::capture(h, generation, cancelled, authority)?;
        let refresh =
            crate::codex_session_store::try_reserve_native_label_refresh(&h.info.id, &root)?;
        Some(Arc::new(Self {
            id: h.info.id.clone(),
            token,
            root,
            context,
            refresh,
        }))
    }
    pub(super) fn matches(
        &self,
        h: &crate::SessionHandle,
        authority: &Option<crate::runtime::prompt_authority::AuthoritySnapshot>,
    ) -> bool {
        self.context.matches(h, authority)
            && self.refresh.is_current(&self.id)
            && verify_workflow_report_token(&self.id, &self.token)
            && !crate::codex_session_store::native_label_refresh_is_blocked(&self.id, &self.root)
            && h.info
                .resume
                .as_ref()
                .and_then(|r| r.harness_session_id.as_deref())
                == Some(self.root.as_str())
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SqlLabelOutcome {
    Applied,
    Noop,
    Rejected,
    Retry,
    CommittedDetached,
}
impl SessionWrites {
    pub(crate) async fn begin_native_label_refresh(
        self: &Arc<Self>,
        id: String,
        token: String,
        generation: u64,
        root: String,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Option<Arc<NativeLabelRefresh>>, StoreError> {
        if !verify_workflow_report_token(&id, &token) {
            return Ok(None);
        }
        let capture_id = id.clone();
        let capture_token = token.clone();
        let context = match self
            .native_access(id.clone(), None, None, move |_, h, authority| {
                if !verify_workflow_report_token(&capture_id, &capture_token) {
                    return None;
                }
                NativeIdentityContext::capture(h, generation, cancelled, authority)
            })
            .await?
        {
            NativeAccess::Current(Some(context)) => context,
            _ => return Ok(None),
        };
        let record: RevisionedSession = serde_json::from_value(
            self.store
                .get(id.clone())
                .await?
                .ok_or(StoreError::Missing)?,
        )?;
        let meta = record.metadata;
        let result = self
            .native_access(id.clone(), None, None, move |_, h, authority| {
                if !context.matches(h, &authority)
                    || !verify_workflow_report_token(&id, &token)
                    || meta.id != h.info.id
                    || meta.created_at != h.info.created_at
                    || meta.harness != "codex"
                    || meta.lifecycle != "alive"
                    || meta.lifecycle_phase != "active"
                    || !meta.label_source.is_automatic()
                    || meta
                        .resume
                        .as_ref()
                        .and_then(|r| r.harness_session_id.as_deref())
                        != Some(root.as_str())
                    || h.info
                        .resume
                        .as_ref()
                        .and_then(|r| r.harness_session_id.as_deref())
                        != Some(root.as_str())
                    || crate::codex_session_store::native_label_refresh_is_blocked(&id, &root)
                {
                    return None;
                }
                let refresh =
                    crate::codex_session_store::try_reserve_native_label_refresh(&id, &root)?;
                Some(Arc::new(NativeLabelRefresh {
                    id,
                    token,
                    root,
                    context,
                    refresh,
                }))
            })
            .await?;
        Ok(match result {
            NativeAccess::Current(value) => value,
            _ => None,
        })
    }
    pub(crate) async fn persist_native_label(
        self: &Arc<Self>,
        refresh: Arc<NativeLabelRefresh>,
        label: String,
    ) -> Result<SqlLabelOutcome, StoreError> {
        // Preserve the definitive commit/publication after the execution claim,
        // even if a lookup scheduler or its caller disappears.
        let owner = self.clone();
        let phase = Arc::new(HookOperation::default());
        let _cancellation = HookCancellation(phase.clone());
        let (mut result, received) = oneshot::channel();
        tokio::spawn(async move {
            let mut running =
                Box::pin(owner.persist_native_label_inner(refresh, label, phase.clone()));
            let outcome = tokio::select! {
                biased;
                _ = result.closed() => { if phase.cancel_before_claim() { return; } running.await }
                outcome = &mut running => outcome,
            };
            let _ = result.send(outcome);
        });
        received.await.map_err(|_| StoreError::Unavailable)?
    }
    async fn persist_native_label_inner(
        self: &Arc<Self>,
        refresh: Arc<NativeLabelRefresh>,
        label: String,
        phase: Arc<HookOperation>,
    ) -> Result<SqlLabelOutcome, StoreError> {
        if label.trim().is_empty()
            || label.chars().any(char::is_control)
            || label.chars().count() > 100
        {
            return Err(StoreError::Invalid("invalid native title".into()));
        }
        let record: RevisionedSession = serde_json::from_value(
            self.store
                .get(refresh.id.clone())
                .await?
                .ok_or(StoreError::Missing)?,
        )?;
        let checking = refresh.clone();
        let meta = record.metadata.clone();
        match self
            .native_access(refresh.id.clone(), None, None, move |_, h, authority| {
                checking.matches(h, &authority)
                    && meta.id == h.info.id
                    && meta.created_at == h.info.created_at
                    && meta.harness == "codex"
                    && meta.lifecycle == "alive"
                    && meta.lifecycle_phase == "active"
                    && meta.label_source.is_automatic()
                    && meta
                        .resume
                        .as_ref()
                        .and_then(|r| r.harness_session_id.as_deref())
                        == Some(checking.root.as_str())
            })
            .await?
        {
            NativeAccess::Current(true) => {}
            NativeAccess::Busy => return Ok(SqlLabelOutcome::Retry),
            _ => return Ok(SqlLabelOutcome::Rejected),
        }
        let prepared = self.store.prepare_internal_owned(
            refresh.id.clone(),
            record.metadata.created_at.clone(),
            Some(record.revision),
            None,
            InternalMutation::CodexLabel {
                root: refresh.root.clone(),
                label: label.clone(),
            },
        )?;
        let Some(completion) = self
            .execute_native_metadata(
                refresh.id.clone(),
                refresh.token.clone(),
                NativeMetadataOrigin::Label(refresh.clone()),
                prepared,
                phase,
                None,
            )
            .await?
        else {
            return Ok(SqlLabelOutcome::Retry);
        };
        if !matches!(
            completion.outcome.decision.disposition,
            Disposition::NativeLabel { accepted: true }
        ) {
            return Ok(SqlLabelOutcome::Rejected);
        }
        let changed = completion.outcome.document.is_some();
        let publishing = refresh.clone();
        let published = self
            .native_access(
                refresh.id.clone(),
                None,
                Some(completion),
                move |_, h, authority| {
                    if !publishing.matches(h, &authority) {
                        return false;
                    }
                    h.info.label = label;
                    true
                },
            )
            .await?;
        if changed {
            self.state.updates.sessions_changed();
        }
        Ok(match published {
            NativeAccess::Current(true) if changed => SqlLabelOutcome::Applied,
            NativeAccess::Current(true) => SqlLabelOutcome::Noop,
            NativeAccess::Busy => SqlLabelOutcome::Retry,
            _ if changed => SqlLabelOutcome::CommittedDetached,
            _ => SqlLabelOutcome::Rejected,
        })
    }
}

impl SessionWrites {
    pub(super) fn schedule_native_label_refresh(
        self: &Arc<Self>,
        refresh: Arc<NativeLabelRefresh>,
    ) {
        let owner = self.clone();
        tokio::spawn(async move {
            if let Err(error) = owner
                .refresh_native_label_with(
                    refresh,
                    crate::codex_session_store::lookup_label_candidate,
                )
                .await
            {
                tracing::warn!(error = ?error, "SQL native label enrichment failed");
            }
        });
    }
    async fn refresh_native_label_with(
        self: &Arc<Self>,
        refresh: Arc<NativeLabelRefresh>,
        lookup: impl Fn(
                &str,
            ) -> Result<
                Option<crate::codex_session_store::CodexLabelCandidate>,
                crate::codex_session_store::CodexStoreError,
            > + Send
            + Sync
            + 'static,
    ) -> Result<SqlLabelOutcome, StoreError> {
        let lookup = Arc::new(lookup);
        let mut outcome = SqlLabelOutcome::Noop;
        for delay in [
            std::time::Duration::ZERO,
            std::time::Duration::from_millis(100),
            std::time::Duration::from_millis(500),
        ] {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            let checking = refresh.clone();
            match self
                .native_access(refresh.id.clone(), None, None, move |_, h, authority| {
                    checking.matches(h, &authority)
                })
                .await?
            {
                NativeAccess::Current(true) => {}
                NativeAccess::Busy => {
                    outcome = SqlLabelOutcome::Retry;
                    continue;
                }
                _ => return Ok(SqlLabelOutcome::Rejected),
            }
            // A missing/unavailable native row preserves the previous title.
            let reading = lookup.clone();
            let root = refresh.root.clone();
            let candidate = tokio::task::spawn_blocking(move || reading(&root))
                .await
                .map_err(|_| StoreError::Unavailable)?
                .ok()
                .flatten();
            let Some(candidate) = candidate else {
                continue;
            };
            outcome = self
                .persist_native_label(refresh.clone(), candidate.text)
                .await?;
            if outcome != SqlLabelOutcome::Retry {
                return Ok(outcome);
            }
        }
        Ok(outcome)
    }
    #[cfg(test)]
    pub(crate) async fn test_refresh_native_label_at(
        self: &Arc<Self>,
        refresh: Arc<NativeLabelRefresh>,
        path: std::path::PathBuf,
    ) -> Result<SqlLabelOutcome, StoreError> {
        self.refresh_native_label_with(refresh, move |root| {
            crate::codex_session_store::test_lookup_label_candidate_at(&path, root)
        })
        .await
    }
}
