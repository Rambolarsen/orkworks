//! Source-only SQL observer read/RPC boundary; never selects a production store.
use super::*;
use crate::runtime::codex_approval_application::sql_context::{self, ObservationContext};
use crate::runtime::codex_native::{NativeError, NativeObservation};

pub(crate) struct SqlObservationSnapshot {
    owner: std::sync::Weak<SessionWrites>,
    id: String,
    context: ObservationContext,
}
impl SqlObservationSnapshot {
    pub(crate) fn root(&self) -> &str {
        self.context.root().expect("validated native root")
    }
}
impl SessionWrites {
    pub(crate) fn native_matches_state(&self, state: &Arc<AppState>) -> bool {
        Arc::ptr_eq(&self.state, state)
    }
    pub(crate) async fn begin_native_observation(
        self: &Arc<Self>,
        id: String,
        generation: u64,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Option<SqlObservationSnapshot>, StoreError> {
        let context = self
            .native_current(id.clone(), None, None, move |_, h, authority| {
                ObservationContext::capture(h, generation, cancelled, authority)
            })
            .await?
            .flatten()
            .ok_or(StoreError::Conflict)?;
        let context = Arc::new(context);
        let record = self.native_observer_record(&id).await?;
        let captured = self
            .native_current(id.clone(), None, None, move |_, h, authority| {
                sql_context::begin_sql_observation(h, &record.metadata, &context, authority)
            })
            .await?
            .flatten()
            .ok_or(StoreError::Conflict)?;
        let (context, effects) = captured;
        if !effects.is_empty() {
            // A tick's durable publication changes attention ownership. The
            // next loop captures a new origin rather than adopting a token here.
            for effect in effects {
                self.native(id.clone(), effect, context.cancellation())
                    .await?;
            }
            return Ok(None);
        }
        if context.root().is_none() {
            return Ok(None);
        }
        Ok(Some(SqlObservationSnapshot {
            owner: Arc::downgrade(self),
            id,
            context,
        }))
    }
    async fn native_observer_record(
        &self,
        id: &str,
    ) -> Result<metadata::session_transactions::RevisionedSession, StoreError> {
        let document = self
            .store
            .get(id.into())
            .await?
            .ok_or(StoreError::Missing)?;
        Ok(serde_json::from_value(document)?)
    }
    pub(crate) async fn finish_native_observation(
        self: &Arc<Self>,
        snapshot: SqlObservationSnapshot,
        result: Result<NativeObservation, NativeError>,
    ) -> Result<bool, StoreError> {
        self.finish_native_observation_inner(snapshot, result, COORDINATED_METADATA_WRITERS)
            .await
    }
    #[cfg(test)]
    pub(crate) async fn finish_native_observation_coordinated_for_test(
        self: &Arc<Self>,
        snapshot: SqlObservationSnapshot,
        result: Result<NativeObservation, NativeError>,
    ) -> Result<bool, StoreError> {
        self.finish_native_observation_inner(snapshot, result, true)
            .await
    }
    async fn finish_native_observation_inner(
        self: &Arc<Self>,
        snapshot: SqlObservationSnapshot,
        result: Result<NativeObservation, NativeError>,
        coordinated: bool,
    ) -> Result<bool, StoreError> {
        if !snapshot
            .owner
            .upgrade()
            .is_some_and(|owner| Arc::ptr_eq(&owner, self))
        {
            return Err(StoreError::Conflict);
        }
        let id = snapshot.id;
        let context = Arc::new(snapshot.context);
        let checking = context.clone();
        let current = self
            .native_current(id.clone(), None, None, move |_, h, authority| {
                checking.matches(h, &authority)
            })
            .await?;
        if current != Some(true) {
            return Err(StoreError::Conflict);
        }
        let record = self.native_observer_record(&id).await?;
        let finishing = context.clone();
        let effects = self
            .native_current(id.clone(), None, None, move |_, h, authority| {
                sql_context::finish_sql_observation(
                    h,
                    &record.metadata,
                    &finishing,
                    authority,
                    result,
                )
            })
            .await?
            .flatten()
            .ok_or(StoreError::Conflict)?;
        for effect in effects {
            match self
                .native_inner(id.clone(), effect, context.cancellation(), coordinated)
                .await?
            {
                Outcome::Committed => {}
                Outcome::Noop { .. } | Outcome::CommittedDetached => return Ok(false),
            }
        }
        Ok(true)
    }
}
