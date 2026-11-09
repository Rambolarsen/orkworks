//! Original-owner final-scan composition; production selects JSON explicitly.
#![cfg_attr(not(test), allow(dead_code))]
use super::ending_persistence::{EndingCompletion, EndingPersistence};
use super::*;
use crate::providers::ProviderRunResult;
use crate::session_application::ending_completion::Completion;
use crate::session_application::sql_writes::{
    lifecycle::{CapturedSqlObservation, Finalization, ProviderWrite, SqlEvidenceWrite},
    SessionWrites,
};
use crate::session_store::StoreError;

#[must_use]
pub(crate) enum FinalScanPersistence {
    Json {
        state: Arc<AppState>,
        id: String,
        generation: u64,
    },
    Sql(CapturedSqlObservation),
}
#[derive(Debug)]
#[must_use]
pub(crate) struct SqlScanCompletion {
    pub(crate) provider: Option<Result<ProviderWrite, StoreError>>,
    pub(crate) evidence: Option<Result<SqlEvidenceWrite, StoreError>>,
    pub(crate) history: Result<Finalization, StoreError>,
}
#[derive(Debug)]
#[must_use]
pub(crate) enum FinalScanCompletion {
    Skipped,
    CaptureFailed(StoreError),
    Json(Result<Completion, tokio::task::JoinError>),
    Sql(Result<SqlScanCompletion, StoreError>),
}
impl FinalScanCompletion {
    pub(crate) fn log_failures(&self, id: &str) {
        match self {
            Self::Json(Err(error)) => {
                tracing::error!(session_id=id,%error,"session ending persistence completion is unknown")
            }
            Self::CaptureFailed(error) | Self::Sql(Err(error)) => {
                tracing::warn!(session_id=id,%error,"final scan original owner is unavailable")
            }
            Self::Sql(Ok(stages)) => {
                if let Some(Err(error)) = &stages.provider {
                    tracing::warn!(session_id=id,%error,"final scan provider context is incomplete");
                }
                if let Some(Err(error)) = &stages.evidence {
                    tracing::warn!(session_id=id,%error,"final scan workflow evidence is incomplete");
                }
                if let Err(error) = &stages.history {
                    tracing::warn!(session_id=id,%error,"final scan history is incomplete");
                }
            }
            _ => {}
        }
    }
}
impl FinalScanPersistence {
    pub(crate) fn json(state: Arc<AppState>, id: String, generation: u64) -> Self {
        Self::Json {
            state,
            id,
            generation,
        }
    }
    /// Capture the same original SQL owner and its file permit BEFORE inference.
    pub(crate) async fn sql(
        state: Arc<AppState>,
        writes: Arc<SessionWrites>,
        id: String,
        generation: u64,
    ) -> Result<Self, StoreError> {
        let EndingPersistence::Sql(captured) =
            EndingPersistence::sql(state, writes, id, generation).await?
        else {
            unreachable!("explicit SQL history constructor changed backend")
        };
        Ok(Self::Sql(captured.capture_sql_observation_origin().await?))
    }
    async fn output(&self) -> Result<Vec<String>, StoreError> {
        match self {
            Self::Sql(captured) => captured.final_scan_output().await,
            Self::Json {
                state,
                id,
                generation,
            } => {
                let state = state.clone();
                let id = id.clone();
                let generation = *generation;
                tokio::task::spawn_blocking(move || {
                    let (output, cols) = {
                        let sessions = state.sessions.lock().unwrap();
                        let h = sessions
                            .get(&id)
                            .filter(|h| {
                                h.runtime.run_generation() == generation
                                    && h.info.lifecycle_phase == "ending"
                            })
                            .ok_or(StoreError::Conflict)?;
                        (h.output_buffer.snapshot(), h.runtime.last_cols)
                    };
                    Ok(peon::rejoin_hard_wrapped_lines(&output, cols))
                })
                .await
                .map_err(|_| StoreError::Unavailable)?
            }
        }
    }
    /// The injected inference future is the same seam used by the real provider runner.
    pub(crate) async fn run<F, Fut>(self, fallback: String, infer: F) -> FinalScanCompletion
    where
        F: FnOnce(Vec<String>) -> Fut,
        Fut: std::future::Future<Output = Option<ProviderRunResult>>,
    {
        let output = match self.output().await {
            Ok(output) => output,
            Err(error) => {
                return match self {
                    Self::Json { .. } if matches!(error, StoreError::Conflict) => {
                        FinalScanCompletion::Skipped
                    }
                    Self::Json { .. } => FinalScanCompletion::CaptureFailed(error),
                    Self::Sql(_) => FinalScanCompletion::Sql(Err(error)),
                }
            }
        };
        let scan = if output.is_empty() {
            None
        } else {
            infer(output.clone()).await
        };
        let now = iso_now();
        match self {
            Self::Json {
                state,
                id,
                generation,
            } => Self::finish_json(state, id, generation, scan, output, now, fallback).await,
            Self::Sql(captured) => {
                let metadata = match captured.ending_metadata().await {
                    Ok(metadata) => metadata,
                    Err(error) => return FinalScanCompletion::Sql(Err(error)),
                };
                let provider = match scan.as_ref().and_then(|r| r.observation.clone()) {
                    Some(observation) => Some(captured.persist_provider_context(observation).await),
                    None => None,
                };
                let inference = scan.as_ref().and_then(|r| r.inference.as_ref());
                if scan.is_some() && inference.is_none() {
                    tracing::warn!(
                        "final peon scan returned no inference; finalizing with fallback snapshot"
                    );
                }
                let evidence = match inference {
                    Some(inference) if !inference.workflow_observations.is_empty() => Some(
                        captured
                            .persist_workflow_observations(
                                // Same original runtime generation used by the existing final-scan keys.
                                captured.run_generation(),
                                inference.workflow_observations.clone(),
                                output,
                            )
                            .await,
                    ),
                    _ => None,
                };
                let snapshot = final_snapshot_from_inference(inference, &now)
                    .unwrap_or_else(|| fallback_final_snapshot(&metadata, &now));
                let history = captured.complete(snapshot, fallback).await;
                FinalScanCompletion::Sql(Ok(SqlScanCompletion {
                    provider,
                    evidence,
                    history,
                }))
            }
        }
    }
    async fn finish_json(
        state: Arc<AppState>,
        id: String,
        generation: u64,
        scan: Option<ProviderRunResult>,
        output: Vec<String>,
        now: String,
        fallback: String,
    ) -> FinalScanCompletion {
        let scan_state = state.clone();
        let scan_id = id.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            {
                let sessions = scan_state.sessions.lock().unwrap();
                if !sessions.get(&scan_id).is_some_and(|h| {
                    h.runtime.run_generation() == generation && h.info.lifecycle_phase == "ending"
                }) {
                    return None;
                }
            }
            // Preserve the legacy check and synchronous file effects together.
            let result = crate::session_application::SessionApplication::new(scan_state.clone())
                .persist_final_peon_scan(&scan_id, generation, scan.as_ref(), &output);
            if !result.should_finalize {
                return None;
            }
            let inferred = final_snapshot_from_inference(
                scan.as_ref().and_then(|r| r.inference.as_ref()),
                &now,
            );
            let snapshot = inferred.unwrap_or_else(|| match result.metadata.as_ref() {
                Some(metadata) => fallback_final_snapshot(metadata, &now),
                None => metadata::canonical_null_snapshot("recovery", Some(now)),
            });
            if result.observation_accepted {
                crate::taskmaster::evaluator::schedule_evaluation(scan_state);
            }
            Some(snapshot)
        })
        .await;
        let snapshot = match prepared {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => return FinalScanCompletion::Skipped,
            Err(error) => return FinalScanCompletion::Json(Err(error)),
        };
        let EndingCompletion::Json(completion) = EndingPersistence::json(state, id, generation)
            .complete(snapshot, fallback)
            .await
        else {
            unreachable!("explicit JSON history constructor changed backend")
        };
        FinalScanCompletion::Json(completion)
    }
}
