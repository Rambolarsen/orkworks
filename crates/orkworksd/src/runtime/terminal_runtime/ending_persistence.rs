//! Runtime history completion only. Full SQL final-scan routing remains gated.
use crate::runtime::session_runtime::RuntimeGeneration;
use crate::session_application::{
    ending_completion::Completion,
    sql_writes::{
        lifecycle::{CapturedEnding, Finalization},
        SessionWrites,
    },
};
use crate::session_store::StoreError;
use crate::{metadata::ObservedStatusSnapshotMetadata, AppState};
use std::sync::Arc;

#[must_use]
pub(crate) enum EndingPersistence {
    Json {
        state: Arc<AppState>,
        id: String,
        generation: RuntimeGeneration,
    },
    Sql(CapturedEnding),
}
#[derive(Debug)]
#[must_use]
pub(crate) enum EndingCompletion {
    Json(Result<Completion, tokio::task::JoinError>),
    Sql(Result<Finalization, StoreError>),
}
impl EndingPersistence {
    pub(crate) fn json(state: Arc<AppState>, id: String, generation: RuntimeGeneration) -> Self {
        Self::Json {
            state,
            id,
            generation,
        }
    }
    /// Source-only capture BEFORE final-scan waits. The full SQL scan still needs
    /// its provider-context and workflow-observation owners qualified separately.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) async fn sql(
        state: Arc<AppState>,
        writes: Arc<SessionWrites>,
        id: String,
        generation: RuntimeGeneration,
    ) -> Result<Self, StoreError> {
        let capture_state = state.clone();
        let capture_id = id.clone();
        #[cfg(test)]
        crate::session_application::ending_completion::testing::dispatched(&state);
        let runtime = tokio::task::spawn_blocking(move || {
            let sessions = capture_state.sessions.lock().unwrap();
            sessions
                .get(&capture_id)
                .filter(|h| {
                    h.info.id == capture_id
                        && h.runtime.run_generation() == generation
                        && h.info.lifecycle_phase == "ending"
                })
                .map(|h| h.runtime.identity())
                .ok_or(StoreError::Conflict)
        })
        .await
        .map_err(|_| StoreError::Unavailable)??;
        writes
            .capture_ending_history(&state, id, runtime)
            .await
            .map(Self::Sql)
    }
    pub(crate) async fn complete(
        self,
        snapshot: ObservedStatusSnapshotMetadata,
        fallback_status: String,
    ) -> EndingCompletion {
        match self {
            Self::Json {
                state,
                id,
                generation,
            } => EndingCompletion::Json(
                crate::session_application::SessionApplication::new(state)
                    .finish_session_ending(id, generation, snapshot, fallback_status)
                    .await,
            ),
            Self::Sql(captured) => {
                EndingCompletion::Sql(captured.complete(snapshot, fallback_status).await)
            }
        }
    }
}
