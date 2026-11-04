//! Metadata-only provider receipt for an original ending owner.
use super::*;
#[derive(Debug, PartialEq, Eq)]
#[must_use]
pub(crate) enum ProviderWrite {
    Unchanged,
    Committed,
    CommittedDetached,
}
impl CapturedEnding {
    /// Borrow original authority so history completion can consume it afterwards.
    /// A current receipt does not project provider fields or prove process state.
    pub(crate) async fn persist_provider_context(
        &self,
        provider: crate::providers::ProviderObservation,
    ) -> Result<ProviderWrite, StoreError> {
        let prepared = self.owner.store.prepare_internal(
            self.id.clone(),
            self.snapshot.created.clone(),
            InternalMutation::EndingProviderContext(provider),
        )?;
        let _cancellation = prepared.cancellation();
        let lane = prepared.lane()?;
        let owner = self.owner.clone();
        let id = self.id.clone();
        let runtime = self.runtime.clone();
        let snapshot = self.snapshot.clone();
        let lease = self.lease.clone();
        let (mut result, received) = oneshot::channel();
        tokio::spawn(async move {
            let execution = async {
                let lane = tokio::select! {
                    biased;
                    _=result.closed()=>return Err(StoreError::Cancelled),
                    lane=tokio::time::timeout_at(prepared.deadline().into(),lane.lock_owned())=>
                        lane.map_err(|_|StoreError::Expired)?,
                };
                let completion = owner
                    .execute_lifecycle(
                        &id,
                        &runtime,
                        &snapshot,
                        prepared,
                        RuntimeWriteResources::new(lane, Some(lease)),
                        &mut result,
                        Access::Async,
                    )
                    .await?;
                if completion.outcome.document.is_none() {
                    return Ok(ProviderWrite::Unchanged);
                }
                // Retain definitive resources until the original-owner receipt check.
                // Provider context changes no attention/display/work clocks and does
                // not publish extra live fields, matching its metadata-only owner.
                let current = owner
                    .lifecycle_access(
                        id.clone(),
                        runtime,
                        Some(snapshot),
                        Some(completion),
                        Access::Async,
                        |_, _| (),
                    )
                    .await
                    .is_ok_and(|current| current.is_some());
                Ok(if current {
                    ProviderWrite::Committed
                } else {
                    ProviderWrite::CommittedDetached
                })
            }
            .await;
            if let Err(error) = &execution {
                if !matches!(
                    error,
                    StoreError::Cancelled | StoreError::Expired | StoreError::Conflict
                ) {
                    tracing::warn!(session_id=%id,%error,"ending provider persistence is incomplete");
                }
            }
            let _ = result.send(execution);
        });
        received.await.map_err(|_| StoreError::Unavailable)?
    }
}
