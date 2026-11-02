//! External hook-observation follow-up; never reads or writes session JSON.
use super::*;
use crate::metadata::hook_observation::{Admission, Stamp};
use crate::runtime::codex_approval_application::sql_context::NativeIdentityContext;
impl SessionWrites {
    pub(super) async fn finish_identity_observation(
        self: &Arc<Self>,
        id: String,
        token: String,
        context: Option<Arc<NativeIdentityContext>>,
        stamp: Stamp,
        fingerprint: Option<String>,
        disposition: crate::metadata::HarnessSessionMergeResult,
    ) -> Result<super::identity::SqlIdentityOutcome, StoreError> {
        use super::identity::SqlIdentityOutcome;
        let (Some(context), Some(fingerprint)) = (context, fingerprint) else {
            return Ok(SqlIdentityOutcome::Accepted(disposition));
        };
        let admitted = self
            .native_access(id.clone(), None, None, move |ws, h, authority| {
                if !stamp.belongs_to(&ws.metadata.hook_observation_owner())
                    || !context.matches(h, &authority)
                    || !crate::runtime::terminal_runtime::verify_workflow_report_token(&id, &token)
                {
                    return None;
                }
                let lease = ws.lease.clone()?;
                Some((stamp.try_claim(), lease))
            })
            .await?;
        let (claim, lease) = match admitted {
            NativeAccess::Current(Some((Admission::Claimed(claim), lease))) => (claim, lease),
            NativeAccess::Busy | NativeAccess::Current(Some((Admission::Busy, _))) => {
                return Ok(SqlIdentityOutcome::Retry)
            }
            _ => return Ok(SqlIdentityOutcome::Accepted(disposition)),
        };
        // SQL may already be committed. A file failure is typed and leaves the
        // packet retryable; it must never be described as a SQL rollback.
        tokio::task::spawn_blocking(move || {
            let _lease = lease;
            claim.write(&crate::metadata::CodexHookObservation {
                fingerprint,
                observed_at: crate::workspace_runtime::iso_now(),
            })
        })
        .await
        .map_err(|_| StoreError::Unavailable)??;
        Ok(SqlIdentityOutcome::Accepted(disposition))
    }
}
