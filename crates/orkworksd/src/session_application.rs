use crate::plan_handoff::{
    normalize_reported_plan_path, resolve_openable_plan_reference, resolve_printed_plan_path,
};
use crate::runtime::observed_status::apply_live_attention_fields;
use crate::session_types::{MemoryState, SessionInfo};
use crate::session_view::{connectivity_for_status, terminal_outcome_for_status};
use crate::taskmaster::{RecommendationStatus, RecommendationType};
use crate::workspace_runtime::parse_hook_observed_at;
use crate::workspace_runtime::{iso_now, orkworks_global_dir};
use crate::{git, metadata, migration, plan_handoff, watcher, AppState, WorkspaceState};
use crate::{harness, peon, SessionHandle};
use portable_pty::PtySize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SessionError {
    BadRequest(&'static str),
    EmptyBadRequest,
    Conflict,
    ConflictWithMessage(&'static str),
    NotFound,
    Internal(&'static str),
}

#[derive(Debug)]
pub(crate) enum WorkflowObservationPersistenceError {
    NoWorkspace,
    SessionNotInWorkspace,
    Record(crate::workflow_observations::RecordError),
}

#[derive(Debug)]
pub(crate) enum RecommendationDismissError {
    Conflict,
    Store(crate::taskmaster::store::StoreError),
}

#[derive(Debug)]
pub(crate) enum RecommendationQueryError {
    Conflict,
    Store(crate::taskmaster::store::StoreError),
}

pub(crate) struct WorkspaceSnapshot {
    pub(crate) path: String,
    pub(crate) repo_root: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) dirty: Option<bool>,
    pub(crate) last_active_session_id: Option<String>,
    pub(crate) active_harness_ids: Vec<String>,
    pub(crate) active_harness_revision: u64,
}

pub(crate) struct SessionApplication {
    state: Arc<AppState>,
}

pub(crate) struct CreateSessionCommand {
    pub(crate) harness_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) initial_prompt: Option<String>,
}

pub(crate) struct AttentionSignal {
    pub(crate) status: String,
    pub(crate) message: Option<String>,
    pub(crate) plan_path: metadata::PlanPathUpdate,
    pub(crate) observed_at: Option<String>,
    pub(crate) cwd: Option<String>,
}

pub(crate) struct DebugAttentionSignal {
    pub(crate) attention: String,
    pub(crate) message: Option<String>,
}

pub(crate) enum DebugHintMutation {
    Preserve,
    Clear,
    Set(String),
}

/// The normalized input to the shared attention merge operation. Callers own
/// transport validation and policy-specific cleanup; this operation owns the
/// workspace → sessions critical section and keeps the two stores in sync.
pub(crate) struct AttentionMergeSignal {
    pub(crate) session_id: String,
    pub(crate) observed_status: String,
    pub(crate) message: Option<String>,
    pub(crate) plan_path: metadata::PlanPathUpdate,
    pub(crate) timestamp: String,
    pub(crate) source: String,
    pub(crate) confidence: f64,
    pub(crate) observed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub(crate) reject_stale_observed_at: bool,
    pub(crate) update_hook_timestamp: bool,
    pub(crate) clear_pending_work_signal: bool,
    pub(crate) require_alive: bool,
    pub(crate) debug_hint_mutation: Option<DebugHintMutation>,
}

pub(crate) struct PlanSelection {
    pub(crate) printed_path: String,
}

pub(crate) struct PeonObservationOutputRange {
    pub(crate) runtime_instance_id: String,
    pub(crate) run_generation: u64,
    pub(crate) first_revision: u64,
    pub(crate) last_revision: u64,
}

pub(crate) struct PeonObservationRecordResult {
    pub(crate) accepted_observation: bool,
    pub(crate) output_range_completed: bool,
}

pub(crate) struct PeonInferencePersistenceResult {
    pub(crate) inference_persisted: bool,
    pub(crate) permanent_hold: bool,
    pub(crate) label_update: Option<(String, u64)>,
    pub(crate) workspace_path: Option<PathBuf>,
}

pub(crate) struct FinalPeonScanResult {
    pub(crate) should_finalize: bool,
    pub(crate) observation_accepted: bool,
    pub(crate) metadata: Option<metadata::SessionMetadata>,
}

pub(crate) struct TerminalOutputQuery {
    pub(crate) lines: Vec<metadata::TerminalOutputRecord>,
    pub(crate) size: Option<(u16, u16)>,
}

pub(crate) struct SummaryLogQueryEntry {
    pub(crate) timestamp: String,
    pub(crate) summary: String,
    pub(crate) source: String,
    pub(crate) confidence: Option<f64>,
}

fn is_placeholder_label(label: &str, id: &str) -> bool {
    label == crate::session_types::placeholder_label(id)
}

// Serializes authoritative terminal-transition writes and best-effort live
// resize writes together. The operation re-reads the current runtime size
// under the sessions lock and checks the lifecycle while holding the shared
// write lock, so a deferred live resize cannot overwrite a final write with a
// stale snapshot.
static TERMINAL_SIZE_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl SessionApplication {
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Evaluates the current workspace's workflow evidence and persists the
    /// resulting recommendations while holding the workspace lock.
    pub(crate) fn refresh_workflow_recommendations(&self) {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let Some(workspace) = workspace_guard.as_ref() else {
            return;
        };
        let Ok(observations) = workspace.workflow_observations.workspace_observations() else {
            return;
        };
        let Ok(existing) = workspace.recommendation_store.list() else {
            return;
        };
        let now = chrono::Utc::now().to_rfc3339();
        let proposals = crate::taskmaster::evaluate_workflow_improvements(
            &observations,
            &existing,
            &workspace.path.display().to_string(),
            &now,
        );
        for proposal in proposals {
            if let Err(error) = workspace.recommendation_store.put(&proposal) {
                tracing::warn!(recommendation_id = %proposal.id, %error, "failed to persist Taskmaster recommendation");
            }
        }
    }

    pub(crate) fn list_recommendations(
        &self,
    ) -> Result<
        (
            Vec<crate::taskmaster::Recommendation>,
            Vec<crate::workflow_observations::ObservationDiagnostic>,
        ),
        RecommendationQueryError,
    > {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard
            .as_ref()
            .ok_or(RecommendationQueryError::Conflict)?;
        let recommendations = workspace
            .recommendation_store
            .list()
            .map_err(RecommendationQueryError::Store)?;
        let diagnostics = workspace.workflow_observations.diagnostics();
        Ok((recommendations, diagnostics))
    }

    pub(crate) fn get_recommendation(
        &self,
        id: &str,
    ) -> Result<Option<crate::taskmaster::Recommendation>, RecommendationQueryError> {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard
            .as_ref()
            .ok_or(RecommendationQueryError::Conflict)?;
        workspace
            .recommendation_store
            .get(id)
            .map_err(RecommendationQueryError::Store)
    }

    pub(crate) fn dismiss_recommendation(
        &self,
        id: &str,
    ) -> Result<Option<crate::taskmaster::Recommendation>, RecommendationDismissError> {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard
            .as_ref()
            .ok_or(RecommendationDismissError::Conflict)?;
        let Some(existing) = workspace
            .recommendation_store
            .get(id)
            .map_err(RecommendationDismissError::Store)?
        else {
            return Ok(None);
        };
        if existing.recommendation_type != RecommendationType::ImproveWorkflow
            || existing.status != RecommendationStatus::Proposed
        {
            return Err(RecommendationDismissError::Conflict);
        }
        workspace
            .recommendation_store
            .dismiss(id, chrono::Utc::now().to_rfc3339())
            .map_err(RecommendationDismissError::Store)
    }

    /// Persists one Peon observation under one workspace snapshot.
    ///
    /// The workspace lock covers both the priority read and metadata merge so
    /// a workspace switch cannot separate the decision from the write. The
    /// caller retains ownership of active-hook normalization, live projection,
    /// and retry scheduling.
    pub(crate) fn persist_peon_observation(
        &self,
        session_id: &str,
        inference: Option<&peon::PeonInference>,
        provider_observation: Option<&crate::providers::ProviderObservation>,
        history_summary: Option<&str>,
        timestamp: &str,
    ) -> PeonInferencePersistenceResult {
        self.persist_peon_observation_inner(
            session_id,
            inference,
            provider_observation,
            history_summary,
            timestamp,
            None,
        )
    }

    pub(crate) fn persist_peon_observation_for_attempt(
        &self,
        session_id: &str,
        attempt: &crate::runtime::peon_runtime::PeonDiagnosticAttempt,
        inference: Option<&peon::PeonInference>,
        provider_observation: Option<&crate::providers::ProviderObservation>,
        history_summary: Option<&str>,
        timestamp: &str,
    ) -> PeonInferencePersistenceResult {
        self.persist_peon_observation_inner(
            session_id,
            inference,
            provider_observation,
            history_summary,
            timestamp,
            Some(attempt),
        )
    }

    fn persist_peon_observation_inner(
        &self,
        session_id: &str,
        inference: Option<&peon::PeonInference>,
        provider_observation: Option<&crate::providers::ProviderObservation>,
        history_summary: Option<&str>,
        timestamp: &str,
        attempt: Option<&crate::runtime::peon_runtime::PeonDiagnosticAttempt>,
    ) -> PeonInferencePersistenceResult {
        let label_epochs = self.state.peon.label_epochs.read().unwrap();
        let captured_label_epoch = label_epochs.get(session_id).copied().unwrap_or(0);
        let workspace_guard = self.state.workspace.lock().unwrap();
        let Some(workspace) = workspace_guard.as_ref() else {
            return PeonInferencePersistenceResult {
                inference_persisted: false,
                permanent_hold: false,
                label_update: None,
                workspace_path: None,
            };
        };
        let workspace_path = Some(workspace.path.clone());

        // The workspace lock is acquired before the sessions lock to match
        // the rest of the application layer. Keep the validation guard while
        // persisting so a replacement cannot pass the check and then receive
        // the old runtime's durable inference.
        let _sessions_guard = if let Some(attempt) = attempt {
            let sessions = self.state.sessions.lock().unwrap();
            let current = sessions.get(session_id).is_some_and(|handle| {
                handle.runtime.matches_identity(&attempt.runtime_identity)
                    && handle.info.lifecycle_phase == "active"
            });
            if !current
                || !self
                    .state
                    .peon
                    .diagnostic_attempt_is_current(session_id, attempt)
            {
                return PeonInferencePersistenceResult {
                    inference_persisted: false,
                    permanent_hold: false,
                    label_update: None,
                    workspace_path: None,
                };
            }
            Some(sessions)
        } else {
            None
        };

        if let Some(observation) = provider_observation {
            workspace
                .metadata
                .persist_provider_context(session_id, observation);
        }

        let Some(inference) = inference else {
            return PeonInferencePersistenceResult {
                inference_persisted: false,
                permanent_hold: false,
                label_update: None,
                workspace_path,
            };
        };

        let (should_write, is_permanent) = workspace
            .metadata
            .read_session(session_id)
            .map(|metadata| {
                let age = workspace.metadata.session_modified_secs_ago(session_id);
                let overwrite = peon::peon_should_overwrite(&metadata.metadata_source, age);
                (overwrite, metadata.metadata_source == "user")
            })
            .unwrap_or((true, false));

        if !should_write {
            tracing::debug!(session_id, "peon: skipping, higher-priority source exists");
            return PeonInferencePersistenceResult {
                inference_persisted: false,
                permanent_hold: is_permanent,
                label_update: None,
                workspace_path,
            };
        }

        match workspace.metadata.merge_peon_inference_with_history(
            session_id,
            inference,
            timestamp,
            provider_observation,
            history_summary,
        ) {
            Ok(()) => PeonInferencePersistenceResult {
                inference_persisted: true,
                permanent_hold: false,
                label_update: history_summary
                    .map(|summary| (summary.chars().take(100).collect(), captured_label_epoch)),
                workspace_path,
            },
            Err(error) => {
                tracing::warn!(session_id, %error, "peon: inference not persisted");
                PeonInferencePersistenceResult {
                    inference_persisted: false,
                    permanent_hold: false,
                    label_update: None,
                    workspace_path,
                }
            }
        }
    }

    /// Persists the final Peon scan under one workspace snapshot.
    ///
    /// The ended-session lifecycle check happens before provider context or
    /// workflow evidence is written. Provider context is attempted whenever a
    /// workspace and scan result exist; workflow evidence additionally
    /// requires session metadata. The caller retains final-snapshot
    /// selection, timeout handling, evaluator scheduling, and completion.
    pub(crate) fn persist_final_peon_scan(
        &self,
        session_id: &str,
        generation: u64,
        scan_result: Option<&crate::providers::ProviderRunResult>,
    ) -> FinalPeonScanResult {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let Some(workspace) = workspace_guard.as_ref() else {
            return FinalPeonScanResult {
                should_finalize: true,
                observation_accepted: false,
                metadata: None,
            };
        };

        let metadata = workspace.metadata.read_session(session_id);
        if metadata
            .as_ref()
            .is_some_and(|session| session.lifecycle_phase == "ended")
        {
            return FinalPeonScanResult {
                should_finalize: false,
                observation_accepted: false,
                metadata,
            };
        }
        let Some(scan_result) = scan_result else {
            return FinalPeonScanResult {
                should_finalize: true,
                observation_accepted: false,
                metadata,
            };
        };

        if let Some(observation) = scan_result.observation.as_ref() {
            workspace
                .metadata
                .persist_provider_context(session_id, observation);
        }

        let Some(metadata) = metadata else {
            return FinalPeonScanResult {
                should_finalize: true,
                observation_accepted: false,
                metadata: None,
            };
        };

        if scan_result.inference.is_none() {
            tracing::warn!(
                session_id = %session_id,
                timeout_secs = self.state.peon.config.final_scan_timeout_secs,
                "final peon scan returned no inference; finalizing with fallback snapshot"
            );
        }

        let mut observation_accepted = false;
        if let Some(inference) = scan_result.inference.as_ref() {
            for (index, candidate) in inference.workflow_observations.iter().enumerate() {
                let key = format!("final-scan:{generation}:{index}");
                let mut recorded = false;
                for attempt in 0..3 {
                    let result = workspace.workflow_observations.record_observation(
                        session_id,
                        crate::workflow_observations::ObservationOrigin::Peon,
                        &key,
                        crate::workflow_observations::ObservationCandidate {
                            kind: candidate.kind,
                            description: candidate.description.clone(),
                            evidence: candidate.evidence.clone(),
                            reported_impact: candidate.reported_impact,
                            confidence: Some(candidate.confidence),
                        },
                    );
                    match result {
                        Ok(crate::workflow_observations::RecordOutcome::Accepted(_)) => {
                            observation_accepted = true;
                            recorded = true;
                            break;
                        }
                        Ok(crate::workflow_observations::RecordOutcome::Duplicate { .. }) => {
                            // A retry of the live Peon path may have already
                            // made this final-scan evidence durable.
                            recorded = true;
                            break;
                        }
                        Err(error)
                            if attempt < 2 && is_retryable_observation_record_error(&error) => {}
                        Err(error) => {
                            tracing::warn!(
                                session_id = %session_id,
                                candidate_index = index,
                                %error,
                                "final peon workflow observation could not be recorded"
                            );
                            break;
                        }
                    }
                }
                if !recorded {
                    tracing::warn!(
                        session_id = %session_id,
                        candidate_index = index,
                        "final peon workflow observation remained unrecorded after retries"
                    );
                }
            }
        }

        FinalPeonScanResult {
            should_finalize: true,
            observation_accepted,
            metadata: Some(metadata),
        }
    }

    /// Records Peon workflow observations for one captured output range.
    ///
    /// Workspace attribution, stable idempotency keys, persistence, and retry
    /// classification live here. The caller retains ownership of capture
    /// cursors, retry timers, and evaluator scheduling.
    pub(crate) fn record_peon_workflow_observations(
        &self,
        session_id: &str,
        captured_workspace_path: Option<&Path>,
        output_range: &PeonObservationOutputRange,
        candidates: &[peon::PeonWorkflowObservation],
    ) -> PeonObservationRecordResult {
        self.record_peon_workflow_observations_inner(
            session_id,
            captured_workspace_path,
            output_range,
            candidates,
            None,
        )
    }

    pub(crate) fn record_peon_workflow_observations_for_attempt(
        &self,
        session_id: &str,
        captured_workspace_path: Option<&Path>,
        attempt: &crate::runtime::peon_runtime::PeonDiagnosticAttempt,
        output_range: &PeonObservationOutputRange,
        candidates: &[peon::PeonWorkflowObservation],
    ) -> PeonObservationRecordResult {
        self.record_peon_workflow_observations_inner(
            session_id,
            captured_workspace_path,
            output_range,
            candidates,
            Some(attempt),
        )
    }

    fn record_peon_workflow_observations_inner(
        &self,
        session_id: &str,
        captured_workspace_path: Option<&Path>,
        output_range: &PeonObservationOutputRange,
        candidates: &[peon::PeonWorkflowObservation],
        attempt: Option<&crate::runtime::peon_runtime::PeonDiagnosticAttempt>,
    ) -> PeonObservationRecordResult {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let Some(workspace) = workspace_guard.as_ref() else {
            return PeonObservationRecordResult {
                accepted_observation: false,
                output_range_completed: true,
            };
        };
        if !workspace_path_matches(&workspace.path, captured_workspace_path) {
            return PeonObservationRecordResult {
                accepted_observation: false,
                output_range_completed: true,
            };
        }

        let _sessions_guard = if let Some(attempt) = attempt {
            let sessions = self.state.sessions.lock().unwrap();
            let current = sessions.get(session_id).is_some_and(|handle| {
                handle.runtime.matches_identity(&attempt.runtime_identity)
                    && handle.info.lifecycle_phase == "active"
            });
            let range_matches_attempt = output_range.runtime_instance_id
                == attempt.runtime_identity.runtime_instance_id
                && output_range.run_generation == attempt.runtime_identity.run_generation;
            if !current
                || !range_matches_attempt
                || !self
                    .state
                    .peon
                    .diagnostic_attempt_is_current(session_id, attempt)
            {
                return PeonObservationRecordResult {
                    accepted_observation: false,
                    output_range_completed: true,
                };
            }
            Some(sessions)
        } else {
            None
        };
        if workspace.metadata.read_session(session_id).is_none() {
            return PeonObservationRecordResult {
                accepted_observation: false,
                output_range_completed: true,
            };
        }

        let mut accepted_observation = false;
        let mut output_range_completed = true;
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            let key = peon_observation_key(
                &output_range.runtime_instance_id,
                session_id,
                output_range.run_generation,
                output_range.first_revision,
                output_range.last_revision,
                candidate_index,
            );
            let result = workspace.workflow_observations.record_observation(
                session_id,
                crate::workflow_observations::ObservationOrigin::Peon,
                &key,
                crate::workflow_observations::ObservationCandidate {
                    kind: candidate.kind,
                    description: candidate.description.clone(),
                    evidence: candidate.evidence.clone(),
                    reported_impact: candidate.reported_impact,
                    confidence: Some(candidate.confidence),
                },
            );
            if matches!(
                &result,
                Ok(crate::workflow_observations::RecordOutcome::Accepted(_))
            ) {
                accepted_observation = true;
            }
            if result
                .as_ref()
                .is_err_and(is_retryable_observation_record_error)
            {
                output_range_completed = false;
            }
        }

        PeonObservationRecordResult {
            accepted_observation,
            output_range_completed,
        }
    }

    /// Records one authenticated Agent workflow observation. The HTTP layer
    /// owns authentication and request validation; this operation owns the
    /// active-workspace/session checks and server-selected Agent origin while
    /// the workspace lock is held.
    pub(crate) fn record_agent_workflow_observation(
        &self,
        session_id: &str,
        idempotency_key: &str,
        candidate: crate::workflow_observations::ObservationCandidate,
    ) -> Result<crate::workflow_observations::RecordOutcome, WorkflowObservationPersistenceError>
    {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let Some(workspace) = workspace_guard.as_ref() else {
            return Err(WorkflowObservationPersistenceError::NoWorkspace);
        };
        if workspace.metadata.read_session(session_id).is_none() {
            return Err(WorkflowObservationPersistenceError::SessionNotInWorkspace);
        }
        workspace
            .workflow_observations
            .record_observation(
                session_id,
                crate::workflow_observations::ObservationOrigin::Agent,
                idempotency_key,
                candidate,
            )
            .map_err(WorkflowObservationPersistenceError::Record)
    }

    /// Arms the first capacity recheck after a usage-limit-latched session
    /// receives accepted input. This is live runtime bookkeeping only.
    pub(crate) fn arm_usage_limit_recheck(&self, id: &str) {
        let mut sessions = self.state.sessions.lock().unwrap();
        let Some(handle) = sessions.get_mut(id) else {
            return;
        };
        if !handle.at_usage_limit_latched
            || handle.capacity_check_pending
            || handle.resume_scan_origin.is_some()
        {
            return;
        }
        handle.resume_scan_origin = Some((handle.output_lines_seen, handle.scan_bytes_seen));
    }

    /// Records accepted, non-sensitive user input and seeds the session topic
    /// exactly once when the persisted label is still the placeholder. The
    /// caller owns sensitivity/descriptive classification and any epoch guard
    /// that surrounds this mutation and subsequent refinement queueing.
    pub(crate) fn record_user_input_topic(
        &self,
        id: &str,
        label_line: &str,
        label_worthy: bool,
    ) -> bool {
        let mut seeded_label = false;
        let workspace_guard = self.state.workspace.lock().unwrap();
        if let Some(workspace) = workspace_guard.as_ref() {
            if let Some(mut metadata) = workspace.metadata.read_session(id) {
                if label_worthy && is_placeholder_label(&metadata.label, id) {
                    metadata.label = label_line.to_string();
                    seeded_label = true;
                }
                metadata.last_user_input = Some(label_line.to_string());
                workspace.metadata.write_session(&metadata);
            }
        }

        if seeded_label {
            if let Some(handle) = self.state.sessions.lock().unwrap().get_mut(id) {
                handle.info.label = label_line.to_string();
            }
        }
        seeded_label
    }

    /// Applies a process status transition to persisted metadata and the live session.
    ///
    /// The entire workflow remains on one blocking task so the live mutation, terminal-size
    /// persistence, metadata update, and event append preserve their existing sequencing.
    pub(crate) async fn transition_session_status(
        &self,
        id: &str,
        expected_generation: Option<crate::runtime::session_runtime::RuntimeGeneration>,
        status: &str,
    ) -> bool {
        let state = self.state.clone();
        let task_state = state.clone();
        let task_id = id.to_string();
        let status = status.to_string();
        tokio::task::spawn_blocking(move || {
            let state = task_state;
            let id = task_id;
            let is_terminal = matches!(status.as_str(), "killed" | "ended" | "error");
            let (handle_decision, session_resume, entered_running, entered_terminal) = {
                let mut sessions = state.sessions.lock().unwrap();
                if expected_generation.is_some_and(|expected| {
                    !sessions
                        .get(&id)
                        .is_some_and(|handle| handle.runtime.run_generation() == expected)
                }) {
                    return false;
                }
                if let Some(handle) = sessions.get_mut(&id) {
                    if !is_terminal
                        && matches!(handle.info.lifecycle_phase.as_str(), "ending" | "ended")
                    {
                        return false;
                    }
                    let entered_running =
                        !is_terminal && status == "running" && handle.info.status != "running";
                    if is_terminal
                        && matches!(handle.info.lifecycle_phase.as_str(), "ending" | "ended")
                    {
                        return false;
                    }
                    if is_terminal {
                        handle.info.status = "running".to_string();
                        handle.info.lifecycle_phase = "ending".to_string();
                        handle.info.lifecycle = "stopping".to_string();
                        handle.info.attention = None;
                        handle.info.connectivity =
                            Some(connectivity_for_status("running").to_string());
                        handle.info.terminal_outcome = None;
                    } else {
                        handle.info.status = status.clone();
                        handle.info.lifecycle_phase = if status == "creating" {
                            "creating".to_string()
                        } else {
                            "active".to_string()
                        };
                        handle.info.lifecycle = if status == "creating" {
                            "creating"
                        } else {
                            "alive"
                        }
                        .to_string();
                        handle.info.connectivity =
                            Some(connectivity_for_status(&status).to_string());
                        handle.info.terminal_outcome = terminal_outcome_for_status(&status);
                    }
                    handle.info.last_activity_at = Some(iso_now());
                    if is_terminal {
                        handle.info.observed_status = None;
                    }
                    (
                        Some(true),
                        (handle.info.resume.clone(), handle.info.resumed_from.clone()),
                        entered_running,
                        is_terminal,
                    )
                } else {
                    (None, (None, None), false, false)
                }
            };
            if entered_terminal {
                state.peon.last_output.write().unwrap().remove(&id);
                // Authoritative final size for dead-session replay. Goes through the
                // same lock-serialized helper live-resize persistence uses
                // (`SessionApplication::persist_terminal_size`) so a live-resize write
                // deferred onto a blocking-pool thread can never land after this one
                // and clobber it with a stale size — see that operation's doc comment.
                // Must run before `ws_guard` below acquires `state.workspace`, since
                // this also locks it internally and the lock isn't reentrant.
                crate::session_application::SessionApplication::new(state.clone())
                    .persist_terminal_size(&id, true);
            } else if entered_running && state.peon.config.enabled {
                state
                    .peon
                    .last_output
                    .write()
                    .unwrap()
                    .entry(id.clone())
                    .or_insert_with(tokio::time::Instant::now);
            }
            let now = iso_now();
            let mut applied = handle_decision.unwrap_or(false);
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                if let Some(mut meta) = ws.metadata.read_session(&id) {
                    // With no in-memory handle, the persisted lifecycle is the guard authority.
                    if handle_decision.is_none()
                        && matches!(meta.lifecycle_phase.as_str(), "ending" | "ended")
                    {
                        return false;
                    }
                    applied = true;
                    if is_terminal {
                        meta.status = "running".to_string();
                        meta.lifecycle_phase = "ending".to_string();
                        meta.lifecycle = "stopping".to_string();
                        meta.attention = None;
                        meta.connectivity = connectivity_for_status("running").to_string();
                        meta.terminal_outcome = None;
                        meta.pending_terminal_status = Some(status.clone());
                        meta.ending_observed_status_snapshot =
                            Some(metadata::ObservedStatusSnapshotMetadata {
                                value: meta.observed_status.clone(),
                                source: meta.metadata_source.clone(),
                                confidence: Some(meta.metadata_confidence),
                                observed_at: Some(now.clone()),
                            });
                    } else {
                        meta.status = status.clone();
                        meta.lifecycle_phase = if status == "creating" {
                            "creating".to_string()
                        } else {
                            "active".to_string()
                        };
                        meta.lifecycle = if status == "creating" {
                            "creating"
                        } else {
                            "alive"
                        }
                        .to_string();
                        meta.connectivity = connectivity_for_status(&status).to_string();
                        meta.terminal_outcome = terminal_outcome_for_status(&status);
                    }
                    meta.last_activity = now.clone();
                    if is_terminal {
                        meta.observed_status = None;
                    }
                    if session_resume.0.is_some() {
                        meta.resume = session_resume.0;
                    }
                    if session_resume.1.is_some() {
                        meta.resumed_from = session_resume.1;
                    }
                    ws.metadata.write_session(&meta);
                }
                if applied {
                    ws.metadata.append_event(
                        &id,
                        &metadata::Event {
                            event_type: "session.status".into(),
                            timestamp: now,
                            status,
                            observed_status: None,
                            confidence: None,
                            summary: None,
                            source: None,
                        },
                    );
                }
            }
            applied
        })
        .await
        .unwrap_or_else(|error| {
            // A poisoned lock (from an unrelated panic elsewhere) is the only realistic
            // cause of a panic here — this crate treats poisoned locks as fatal
            // everywhere via `.lock().unwrap()`. Log with session context (more durable
            // than the bare panic hook in a daemon) and resume the unwind rather than
            // quietly returning `false`: a caller that sees `false` moves on assuming
            // the transition never happened, but the in-memory sessions-lock mutation
            // above may already have applied it, which would leave the session
            // permanently stuck in "ending" with finalization never scheduled.
            //
            // This deliberately differs from every other `spawn_blocking(...).await`
            // call site in this crate (`select_terminal_plan`, `report_session_plan_path`,
            // the harness/provider handlers), which convert a `JoinError` into a safe
            // fallback (a 500 status, `None`, `false`) instead of re-panicking. Those
            // sites can do that safely because a panic inside their closures happens
            // *before* any caller-visible mutation — there's nothing for the fallback to
            // contradict. Do not copy the graceful-fallback tail here without first
            // checking whether the same "already mutated in-memory before the panic
            // could happen" condition applies.
            tracing::error!(
                error = %error,
                session_id = %id,
                "set_session_status: blocking task panicked"
            );
            std::panic::resume_unwind(error.into_panic())
        })
    }

    /// Persists the current runtime terminal size for dead-session replay.
    ///
    /// The sessions lock is released before the workspace lock is acquired.
    /// `authoritative` is used only by the terminal-status transition, which
    /// must write after the runtime enters `ending` or `ended`; live-resize
    /// writes back off during those phases.
    pub(crate) fn persist_terminal_size(&self, id: &str, authoritative: bool) {
        let _write_guard = TERMINAL_SIZE_WRITE_LOCK.lock().unwrap();
        let snapshot = {
            let sessions = self.state.sessions.lock().unwrap();
            sessions.get(id).map(|handle| {
                (
                    matches!(handle.info.lifecycle_phase.as_str(), "ending" | "ended"),
                    handle.runtime.last_cols,
                    handle.runtime.last_rows,
                )
            })
        };
        let Some((is_terminal, cols, rows)) = snapshot else {
            return;
        };
        if is_terminal && !authoritative {
            return;
        }
        if let Some(ref ws) = *self.state.workspace.lock().unwrap() {
            ws.metadata.write_terminal_size(id, cols, rows);
        }
    }

    /// Persists output recency without allowing delayed writes to move the
    /// stored timestamp backwards. The metadata read, monotonicity check, and
    /// write stay adjacent under the workspace lock.
    pub(crate) fn persist_output_recency(&self, id: &str, timestamp: String) {
        let ws_guard = self.state.workspace.lock().unwrap();
        let Some(ws) = ws_guard.as_ref() else {
            return;
        };
        let Some(mut meta) = ws.metadata.read_session(id) else {
            return;
        };
        if should_persist_output_recency(meta.last_output_at.as_deref(), &timestamp) {
            meta.last_output_at = Some(timestamp);
            ws.metadata.write_session(&meta);
        }
    }

    /// Appends one already-parsed terminal-output batch under the workspace
    /// lock. Runtime owns batching, backpressure, and blocking-task scheduling.
    pub(crate) fn append_terminal_output_batch(
        &self,
        session_id: &str,
        records: &[metadata::TerminalOutputRecord],
    ) {
        let workspace_guard = self.state.workspace.lock().unwrap();
        if let Some(workspace) = workspace_guard.as_ref() {
            workspace
                .metadata
                .append_terminal_output_records(session_id, records);
        }
    }

    /// Trims persisted terminal output after runtime has drained all pending
    /// output batches and the persistence writer has completed.
    pub(crate) fn trim_terminal_output(&self, session_id: &str) {
        let workspace_guard = self.state.workspace.lock().unwrap();
        if let Some(workspace) = workspace_guard.as_ref() {
            workspace
                .metadata
                .trim_terminal_output(session_id, metadata::TERMINAL_OUTPUT_MAX_LINES);
        }
    }

    pub(crate) fn get_terminal_output(&self, session_id: &str) -> TerminalOutputQuery {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let Some(workspace) = workspace_guard.as_ref() else {
            return TerminalOutputQuery {
                lines: Vec::new(),
                size: None,
            };
        };
        TerminalOutputQuery {
            lines: workspace
                .metadata
                .read_terminal_output(session_id, metadata::TERMINAL_OUTPUT_MAX_LINES),
            size: workspace.metadata.read_terminal_size(session_id),
        }
    }

    pub(crate) fn get_summary_log(&self, session_id: &str) -> Vec<SummaryLogQueryEntry> {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let Some(workspace) = workspace_guard.as_ref() else {
            return Vec::new();
        };
        if workspace.metadata.read_session(session_id).is_none() {
            return Vec::new();
        }
        workspace
            .metadata
            .read_events(session_id)
            .into_iter()
            .filter_map(|event| {
                let metadata::Event {
                    timestamp,
                    confidence,
                    summary: Some(summary),
                    source: Some(source),
                    ..
                } = event
                else {
                    return None;
                };
                Some(SummaryLogQueryEntry {
                    timestamp,
                    summary,
                    source,
                    confidence,
                })
            })
            .collect()
    }

    /// Records an input frame accepted by the PTY and, for a completed line,
    /// commits the process-owned working transition. The workspace-to-sessions
    /// lock order is deliberate: persisted and live state must not diverge.
    pub(crate) fn commit_accepted_input(
        &self,
        id: &str,
        output_boundary: Option<u64>,
        line_completed: bool,
    ) {
        let ws_guard = self.state.workspace.lock().unwrap();
        let mut sessions = self.state.sessions.lock().unwrap();
        let Some(handle) = sessions.get_mut(id) else {
            return;
        };
        if handle.info.lifecycle != "alive" {
            return;
        }
        let already_working = handle.info.observed_status.as_deref() == Some("working")
            && handle.info.attention.as_deref() == Some("working")
            && handle.info.metadata_source.as_deref() == Some("process")
            && handle.info.metadata_confidence == Some(1.0)
            && handle.info.needs_user_input.is_none()
            && handle.info.detected_question.is_none()
            && handle.info.suggested_options.is_none()
            && handle.pending_work_signal.is_none();
        let commit_working = !already_working && line_completed;
        let Some(next_generation) = handle.runtime.input_generation.checked_add(1) else {
            tracing::warn!(session_id = %id, "input generation overflow");
            return;
        };
        let accepted_at = chrono::Utc::now();
        if !commit_working || already_working {
            handle.runtime.input_generation = next_generation;
            handle.runtime.accepted_input_at = Some(accepted_at);
            handle.runtime.min_peon_output_revision =
                output_boundary.unwrap_or(handle.runtime.peon_output_revision);
            drop(sessions);
            drop(ws_guard);
            self.state
                .peon
                .last_output
                .write()
                .unwrap()
                .insert(id.to_string(), tokio::time::Instant::now());
            return;
        }
        let fields = crate::runtime::observed_status::process_transition_fields(
            crate::runtime::observed_status::ProcessTransition::CommittedWorking,
        );
        if ws_guard.is_none() {
            crate::runtime::observed_status::apply_process_transition_to_handle(
                &mut handle.info,
                &fields,
            );
            handle.pending_work_signal = None;
            handle.runtime.input_generation = next_generation;
            handle.runtime.accepted_input_at = Some(accepted_at);
            handle.runtime.min_peon_output_revision =
                output_boundary.unwrap_or(handle.runtime.peon_output_revision);
            drop(sessions);
            drop(ws_guard);
            self.state
                .peon
                .last_output
                .write()
                .unwrap()
                .insert(id.to_string(), tokio::time::Instant::now());
            return;
        }
        let ws = ws_guard.as_ref().expect("workspace checked above");
        let Some(mut meta) = ws.metadata.read_session(id) else {
            return;
        };
        if meta.lifecycle != "alive" {
            return;
        }
        crate::runtime::observed_status::apply_process_transition_to_meta(&mut meta, &fields);
        if ws.metadata.try_write_session(&meta).is_err() {
            tracing::warn!(session_id = %id, "failed to persist input attention transition");
            return;
        }
        crate::runtime::observed_status::apply_process_transition_to_handle(
            &mut handle.info,
            &fields,
        );
        handle.pending_work_signal = None;
        handle.runtime.input_generation = next_generation;
        handle.runtime.accepted_input_at = Some(accepted_at);
        handle.runtime.min_peon_output_revision =
            output_boundary.unwrap_or(handle.runtime.peon_output_revision);
        drop(sessions);
        drop(ws_guard);
        self.state
            .peon
            .last_output
            .write()
            .unwrap()
            .insert(id.to_string(), tokio::time::Instant::now());
    }

    /// Applies the Peon idle-timeout transition to persisted metadata and the
    /// live session. The persisted state is re-read while the workspace and
    /// sessions locks are held in that order; the live projection happens
    /// only after persistence succeeds.
    pub(crate) fn apply_idle_timeout(&self, id: &str) {
        let ws_guard = self.state.workspace.lock().unwrap();
        let Some(ws) = ws_guard.as_ref() else {
            return;
        };
        let mut sessions = self.state.sessions.lock().unwrap();
        let Some(mut meta) = ws.metadata.read_session(id) else {
            return;
        };
        if !matches!(meta.observed_status.as_deref(), None | Some("working")) {
            return;
        }
        let fields = crate::runtime::observed_status::process_transition_fields(
            crate::runtime::observed_status::ProcessTransition::IdleTimeout,
        );
        crate::runtime::observed_status::apply_process_transition_to_meta(&mut meta, &fields);
        if ws.metadata.try_write_session(&meta).is_err() {
            tracing::warn!(session_id = %id, "failed to persist idle timeout transition");
            return;
        }
        if let Some(handle) = sessions.get_mut(id) {
            crate::runtime::observed_status::apply_process_transition_to_handle(
                &mut handle.info,
                &fields,
            );
        }
    }

    /// Resets a session's conversation topic after the runtime has confirmed
    /// a harness-declared reset command. The label epoch write guard spans the
    /// epoch bump, queued-work clearing, and both label projections so an
    /// older refinement cannot restore the previous conversation's label.
    pub(crate) fn reset_session_topic(&self, id: &str) -> bool {
        let placeholder = crate::session_types::placeholder_label(id);
        let mut epochs = self.state.peon.label_epochs.write().unwrap();
        let epoch = epochs.entry(id.to_string()).or_insert(0);
        *epoch = epoch.saturating_add(1);
        self.state.peon.label_hint.write().unwrap().remove(id);
        self.state.peon.label_pending.write().unwrap().remove(id);

        {
            let ws_guard = self.state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                if let Some(mut meta) = ws.metadata.read_session(id) {
                    meta.label = placeholder.clone();
                    ws.metadata.write_session(&meta);
                }
            }
        }
        if let Some(handle) = self.state.sessions.lock().unwrap().get_mut(id) {
            handle.info.label = placeholder;
        }

        true
    }

    /// Returns whether `line` exactly names a label-reset command declared by
    /// the session's persisted harness. The workspace lock is released before
    /// consulting the harness catalog so the two stores never overlap.
    pub(crate) fn is_persisted_harness_label_reset(&self, id: &str, line: &str) -> bool {
        let harness_id = {
            let workspace_guard = self.state.workspace.lock().unwrap();
            let Some(workspace) = workspace_guard.as_ref() else {
                return false;
            };
            let Some(metadata) = workspace.metadata.read_session(id) else {
                return false;
            };
            metadata.harness
        };
        if harness_id.is_empty() {
            return false;
        }

        let trimmed = line.trim();
        let registry = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned");
        registry.get(&harness_id).is_some_and(|harness| {
            harness
                .definition
                .label_reset_commands
                .iter()
                .any(|command| command == trimmed)
        })
    }

    /// Clears label work that cannot be consumed after a session is forgotten.
    /// The side tables are independent of workspace and session lifecycle
    /// locks, so this deliberately acquires only their existing write guards
    /// in order.
    pub(crate) fn clear_forgotten_session_tracking(&self, id: &str) {
        self.state.peon.label_epochs.write().unwrap().remove(id);
        self.state.peon.label_hint.write().unwrap().remove(id);
        self.state.peon.label_pending.write().unwrap().remove(id);
    }

    /// Clears runtime-owned state once a session's PTY process is gone.
    /// Label epochs and queued label work remain because ended sessions can be
    /// resumed and still belong to the current conversation.
    pub(crate) fn clear_ended_session_tracking(&self, id: &str) {
        self.clear_ended_session_tracking_with_identity(id, None);
    }

    pub(crate) fn clear_ended_session_tracking_for_runtime(
        &self,
        id: &str,
        runtime_identity: &crate::runtime::session_runtime::RuntimeIdentity,
    ) {
        self.clear_ended_session_tracking_with_identity(id, Some(runtime_identity));
    }

    fn clear_ended_session_tracking_with_identity(
        &self,
        id: &str,
        runtime_identity: Option<&crate::runtime::session_runtime::RuntimeIdentity>,
    ) {
        // Do not hold sessions while invalidation waits for in_flight. Peon
        // scans take in_flight before sessions when selecting work.
        let owns_runtime_diagnostics = self
            .state
            .peon
            .invalidate_diagnostic_attempt(id, runtime_identity);
        if runtime_identity.is_some() && !owns_runtime_diagnostics {
            return;
        }
        self.state.peon.last_output.write().unwrap().remove(id);
        self.state.peon.last_inference.write().unwrap().remove(id);
        self.state.peon.input_buf.write().unwrap().remove(id);
        self.state.peon.reported_cwd.write().unwrap().remove(id);
        self.state.session_pids.lock().unwrap().remove(id);
        // ADR 0042: a dead session's reporting capability must stop working
        // immediately, even if a caller captured the token beforehand.
        crate::runtime::terminal_runtime::clear_workflow_report_token(id);
    }

    /// Persists a validated Peon input label while preventing a reset from
    /// racing between the durable and live projections.
    pub(crate) fn persist_input_label(&self, id: &str, label: String, captured_epoch: u64) -> bool {
        let epochs = self.state.peon.label_epochs.read().unwrap();
        let current_epoch = epochs.get(id).copied().unwrap_or(0);
        if captured_epoch != current_epoch {
            return false;
        }

        let mut updated = false;
        let ws_guard = self.state.workspace.lock().unwrap();
        if let Some(ws) = ws_guard.as_ref() {
            if let Some(mut meta) = ws.metadata.read_session(id) {
                meta.label = label.clone();
                ws.metadata.write_session(&meta);
                updated = true;
            }
        }
        if let Some(handle) = self.state.sessions.lock().unwrap().get_mut(id) {
            handle.info.label = label;
            updated = true;
        }
        updated
    }

    pub(crate) fn persist_input_label_for_attempt(
        &self,
        id: &str,
        attempt: &crate::runtime::peon_runtime::PeonDiagnosticAttempt,
        label: String,
        captured_epoch: u64,
    ) -> bool {
        let epochs = self.state.peon.label_epochs.read().unwrap();
        let current_epoch = epochs.get(id).copied().unwrap_or(0);
        if captured_epoch != current_epoch {
            return false;
        }

        let mut updated = false;
        let ws_guard = self.state.workspace.lock().unwrap();
        let mut sessions = self.state.sessions.lock().unwrap();
        let current = sessions.get(id).is_some_and(|handle| {
            handle.runtime.matches_identity(&attempt.runtime_identity)
                && handle.info.lifecycle_phase == "active"
        });
        if !current || !self.state.peon.diagnostic_attempt_is_current(id, attempt) {
            return false;
        }
        if let Some(ws) = ws_guard.as_ref() {
            if let Some(mut meta) = ws.metadata.read_session(id) {
                meta.label = label.clone();
                ws.metadata.write_session(&meta);
                updated = true;
            }
        }
        if let Some(handle) = sessions.get_mut(id) {
            handle.info.label = label;
            updated = true;
        }
        updated
    }

    /// Completes an ending session after the runtime has collected its final
    /// observed-status snapshot. The sessions/workspace/sessions lock order is
    /// deliberate: the initial generation guard prevents stale runtimes from
    /// finalizing a replacement, while the final guard prevents a replacement
    /// from being overwritten after persisted finalization.
    pub(crate) fn complete_session_ending(
        &self,
        id: &str,
        generation: crate::runtime::session_runtime::RuntimeGeneration,
        final_snapshot: metadata::ObservedStatusSnapshotMetadata,
        fallback_terminal_status: &str,
    ) -> bool {
        {
            let sessions = self.state.sessions.lock().unwrap();
            if !sessions.get(id).is_some_and(|handle| {
                handle.runtime.run_generation() == generation
                    && handle.info.lifecycle_phase == "ending"
            }) {
                return false;
            }
        }

        let now = iso_now();
        let mut final_status: Option<String> = None;

        {
            let ws_guard = self.state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                if let Some(mut meta) = ws.metadata.read_session(id) {
                    if meta.lifecycle_phase == "ended" {
                        return false;
                    }
                    let pending = meta
                        .pending_terminal_status
                        .clone()
                        .unwrap_or_else(|| fallback_terminal_status.into());
                    meta.status = pending.clone();
                    meta.lifecycle_phase = "ended".into();
                    meta.lifecycle = "dead".into();
                    meta.attention = None;
                    meta.connectivity = connectivity_for_status(&pending).to_string();
                    meta.terminal_outcome = terminal_outcome_for_status(&pending);
                    meta.pending_terminal_status = None;
                    meta.ending_observed_status_snapshot = None;
                    meta.final_observed_status_snapshot = Some(final_snapshot.clone());
                    meta.observed_status = None;
                    meta.last_activity = now.clone();
                    ws.metadata.write_session(&meta);
                    ws.metadata.append_event(
                        id,
                        &metadata::Event {
                            event_type: "session.status".into(),
                            timestamp: now.clone(),
                            status: pending.clone(),
                            observed_status: final_snapshot.value.clone(),
                            confidence: final_snapshot.confidence,
                            summary: None,
                            source: None,
                        },
                    );
                    final_status = Some(pending);
                }
            }
        }

        let pending = final_status.unwrap_or_else(|| fallback_terminal_status.into());
        let mut sessions = self.state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(id) {
            if handle.runtime.run_generation() != generation
                || handle.info.lifecycle_phase == "ended"
            {
                return false;
            }
            handle.info.status = pending.clone();
            handle.info.lifecycle_phase = "ended".into();
            handle.info.lifecycle = "dead".into();
            handle.info.attention = None;
            handle.info.connectivity = Some(connectivity_for_status(&pending).to_string());
            handle.info.terminal_outcome = terminal_outcome_for_status(&pending);
            handle.info.observed_status = None;
            handle.info.final_observed_status = final_snapshot.value.clone();
            handle.info.last_activity_at = Some(now);
            handle.resume_in_progress = false;
            return true;
        }
        false
    }

    pub(crate) fn open_workspace(&self, path: PathBuf) -> Result<WorkspaceSnapshot, SessionError> {
        if !path.is_dir() {
            return Err(SessionError::BadRequest("not a directory"));
        }
        let global_dir =
            orkworks_global_dir(&path).ok_or(SessionError::Internal("no home directory"))?;
        for dir in &["sessions", "events", "capacity", "skills"] {
            if let Err(error) = std::fs::create_dir_all(global_dir.join(dir)) {
                tracing::warn!(path = %global_dir.display(), dir, %error, "failed to create metadata dir");
            }
        }

        let store = metadata::MetadataStore::new(&global_dir);
        migration::migrate_if_needed(&path, &global_dir);
        let harness_snapshot = self.state.harness_store.snapshot().map_err(|error| {
            tracing::error!(
                ?error,
                "failed to load harness configuration while opening workspace"
            );
            SessionError::Internal("failed to load harness configuration")
        })?;
        *self
            .state
            .harness_catalog
            .write()
            .expect("harness catalog lock poisoned") = harness_snapshot.registry.clone();
        let memory = store.read_workspace_memory();
        let known_harness_ids: std::collections::HashSet<String> = harness_snapshot
            .registry
            .ids()
            .filter(|id| {
                harness_snapshot
                    .registry
                    .get(id)
                    .is_some_and(|harness| !harness.definition.retired)
            })
            .map(str::to_owned)
            .collect();
        let mut memory = memory.unwrap_or_default();
        let original_active_harness_ids = memory.active_harness_ids.clone();
        memory
            .active_harness_ids
            .retain(|id| known_harness_ids.contains(id));
        if memory.active_harness_ids != original_active_harness_ids {
            memory.active_harness_revision = memory.active_harness_revision.saturating_add(1);
            store.write_workspace_memory(&memory);
        }
        let last_active_session_id = memory.last_active_session_id.clone();
        let active_harness_ids = memory.active_harness_ids;
        let active_harness_revision = memory.active_harness_revision;
        let watcher = watcher::MetadataWatcher::start(&global_dir.join("sessions"));

        let mut workspace = self.state.workspace.lock().unwrap();
        let workflow_observations = crate::workflow_observations::WorkflowObservationStore::open(
            global_dir.clone(),
        )
        .map_err(|error| {
            tracing::error!(path = %global_dir.display(), %error, "failed to open workflow observation store");
            SessionError::Internal("failed to open workflow observation store")
        })?;
        let recommendation_store = crate::taskmaster::store::RecommendationStore::open(
            global_dir.clone(),
        )
        .map_err(|error| {
            tracing::error!(path = %global_dir.display(), %error, "failed to open recommendation store");
            SessionError::Internal("failed to open recommendation store")
        })?;
        let retained_session_ids = store
            .read_all_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<std::collections::HashSet<_>>();
        if let Err(error) = recommendation_store.scrub_orphans(&retained_session_ids) {
            tracing::warn!(path = %global_dir.display(), %error, "failed to scrub orphaned recommendations");
            return Err(SessionError::Internal(
                "failed to scrub orphaned recommendations",
            ));
        }
        *workspace = Some(WorkspaceState {
            path: path.clone(),
            metadata: store,
            workflow_observations,
            recommendation_store,
            watcher,
        });
        self.state.bump_harness_probe_generation();

        if let Some(workspace) = workspace.as_ref() {
            let now = iso_now();
            let live_ids: std::collections::HashSet<String> = self
                .state
                .sessions
                .lock()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            for session in workspace.metadata.read_all_sessions() {
                if (session.status == "running" || session.status == "creating")
                    && !live_ids.contains(&session.id)
                {
                    workspace
                        .metadata
                        .write_session(&metadata::reconcile_orphaned_session(session, &now));
                }
            }
        }

        drop(workspace);
        crate::taskmaster::evaluator::schedule_evaluation(self.state.clone());

        let git_context = git::detect(&path);
        Ok(WorkspaceSnapshot {
            path: path.display().to_string(),
            repo_root: git_context.repo_root,
            branch: git_context.branch,
            dirty: Some(git_context.dirty),
            last_active_session_id,
            active_harness_ids,
            active_harness_revision,
        })
    }

    pub(crate) async fn create_session(
        &self,
        request: CreateSessionCommand,
    ) -> Result<SessionInfo, SessionError> {
        create_session_workflow(self.state.clone(), request).await
    }

    pub(crate) async fn resume_session(&self, id: &str) -> Result<SessionInfo, SessionError> {
        resume_session_workflow(self.state.clone(), id).await
    }

    pub(crate) fn report_harness_session(
        &self,
        id: &str,
        report: metadata::HarnessSessionReport,
    ) -> Result<metadata::HarnessSessionMergeResult, SessionError> {
        if !metadata::valid_harness_session_report(&report) {
            return Ok(metadata::HarnessSessionMergeResult::Invalid);
        }

        let now = iso_now();
        let result = {
            let workspace = self.state.workspace.lock().unwrap();
            let Some(workspace) = workspace.as_ref() else {
                return Err(SessionError::Conflict);
            };
            workspace
                .metadata
                .merge_harness_session_report(id, &report, &now)
        };

        if result == metadata::HarnessSessionMergeResult::Accepted {
            let updated_resume = {
                let workspace = self.state.workspace.lock().unwrap();
                workspace
                    .as_ref()
                    .and_then(|workspace| workspace.metadata.read_session(id))
                    .and_then(|meta| meta.resume)
            };
            if let Some(updated_resume) = updated_resume {
                let mut sessions = self.state.sessions.lock().unwrap();
                if let Some(handle) = sessions.get_mut(id) {
                    handle.info.resume = Some(updated_resume);
                }
            }
        }

        Ok(result)
    }

    pub(crate) fn record_codex_hook_observation(
        &self,
        session_id: &str,
        fingerprint: &str,
    ) -> Result<(), SessionError> {
        if !metadata::valid_hook_fingerprint(fingerprint) {
            return Err(SessionError::BadRequest("invalid hook fingerprint"));
        }
        let workspace = self.state.workspace.lock().unwrap();
        let workspace = workspace.as_ref().ok_or(SessionError::Conflict)?;
        let session = workspace
            .metadata
            .read_session(session_id)
            .ok_or(SessionError::NotFound)?;
        if session.harness != "codex" {
            return Err(SessionError::BadRequest(
                "hook fingerprint requires a Codex session",
            ));
        }
        workspace
            .metadata
            .write_codex_hook_observation(&metadata::CodexHookObservation {
                fingerprint: fingerprint.into(),
                observed_at: iso_now(),
            });
        Ok(())
    }

    /// Applies one normalized attention signal while holding the workspace
    /// and sessions locks in that order. The metadata merge, stale hook
    /// rejection, lifecycle check, live projection, and hook-only runtime
    /// bookkeeping are one critical section so the durable and live stores
    /// cannot observe different winners.
    pub(crate) fn apply_attention_signal(
        &self,
        signal: AttentionMergeSignal,
    ) -> Result<metadata::AttentionMergeResult, SessionError> {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard.as_ref().ok_or(SessionError::Conflict)?;
        let mut sessions = self.state.sessions.lock().unwrap();
        let handle = sessions.get(&signal.session_id);

        if signal.require_alive {
            match workspace.metadata.read_session(&signal.session_id) {
                None => return Err(SessionError::NotFound),
                Some(meta) if meta.lifecycle != "alive" => {
                    return Err(SessionError::EmptyBadRequest)
                }
                Some(_) => {}
            }
        }

        if signal.reject_stale_observed_at
            && signal.observed_at.is_some_and(|timestamp| {
                handle.is_some_and(|handle| {
                    handle
                        .runtime
                        .last_hook_attention_at
                        .is_some_and(|previous| timestamp <= previous)
                })
            })
        {
            return Ok(metadata::AttentionMergeResult::Ignored);
        }

        let result = workspace.metadata.merge_agent_attention_signal_with_plan(
            &signal.session_id,
            &signal.observed_status,
            signal.message.as_deref(),
            &signal.plan_path,
            &signal.timestamp,
            &signal.source,
            signal.confidence,
        );
        if result == metadata::AttentionMergeResult::Accepted {
            if let Some(handle) = sessions.get_mut(&signal.session_id) {
                apply_live_attention_fields(
                    &mut handle.info,
                    &signal.observed_status,
                    signal.message.as_deref(),
                    &signal.source,
                    signal.confidence,
                );
                if signal.update_hook_timestamp {
                    if let Some(observed_at) = signal.observed_at {
                        handle.runtime.last_hook_attention_at = Some(observed_at);
                    }
                }
                if signal.clear_pending_work_signal {
                    handle.pending_work_signal = None;
                }
                match &signal.debug_hint_mutation {
                    Some(DebugHintMutation::Preserve) | None => {}
                    Some(DebugHintMutation::Clear) => {
                        handle.info.usage_limit_reset_hint = None;
                    }
                    Some(DebugHintMutation::Set(message)) => {
                        handle.info.usage_limit_reset_hint = Some(message.clone());
                    }
                }
            }
        }
        Ok(result)
    }

    pub(crate) async fn apply_debug_attention(
        &self,
        id: &str,
        signal: DebugAttentionSignal,
    ) -> Result<metadata::AttentionMergeResult, SessionError> {
        if !matches!(
            signal.attention.as_str(),
            "working" | "idle" | "needs_you" | "blocked" | "failed" | "capped"
        ) {
            return Err(SessionError::EmptyBadRequest);
        }

        let observed_status = if signal.attention == "needs_you" {
            "waiting_for_input".to_string()
        } else {
            signal.attention.clone()
        };
        let is_capped = signal.attention == "capped";
        let summary_message = if is_capped {
            None
        } else {
            signal.message.clone()
        };
        let debug_hint_mutation = if is_capped {
            signal
                .message
                .map(DebugHintMutation::Set)
                .unwrap_or(DebugHintMutation::Preserve)
        } else {
            DebugHintMutation::Clear
        };
        let application = SessionApplication::new(self.state.clone());
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let result = application.apply_attention_signal(AttentionMergeSignal {
                session_id: id.clone(),
                observed_status,
                message: summary_message,
                plan_path: metadata::PlanPathUpdate::Unchanged,
                timestamp: iso_now(),
                source: "debug".into(),
                confidence: 0.0,
                observed_at: None,
                reject_stale_observed_at: false,
                update_hook_timestamp: false,
                clear_pending_work_signal: false,
                require_alive: true,
                debug_hint_mutation: Some(debug_hint_mutation),
            })?;
            Ok(result)
        })
        .await
        .map_err(|error| {
            tracing::error!(%error, "debug attention metadata task failed");
            SessionError::Internal("application operation failed")
        })?
    }

    pub(crate) fn report_plan_path(
        &self,
        id: &str,
        reported_path: &str,
    ) -> Result<(), SessionError> {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard.as_ref().ok_or(SessionError::Conflict)?;
        let mut metadata = workspace
            .metadata
            .read_session(id)
            .ok_or(SessionError::NotFound)?;
        if metadata.lifecycle != "alive" {
            return Err(SessionError::Conflict);
        }
        if metadata
            .plan_path
            .as_ref()
            .is_some_and(|reference| reference.source == metadata::PlanSource::UserSelected)
        {
            return Ok(());
        }
        let relative = normalize_reported_plan_path(&workspace.path, reported_path)
            .map_err(|_| SessionError::EmptyBadRequest)?;
        metadata.plan_path = Some(metadata::PlanReference {
            worktree_root: Some(workspace.path.to_string_lossy().into_owned()),
            relative_path: relative,
            source: metadata::PlanSource::HookReported,
        });
        workspace
            .metadata
            .try_write_session(&metadata)
            .map_err(|error| {
                tracing::error!(error = %error, session = %id, "plan path session write failed");
                SessionError::Internal("application operation failed")
            })?;
        workspace.metadata.append_event(
            id,
            &metadata::Event {
                event_type: "session.plan_path_hooked".into(),
                timestamp: iso_now(),
                status: metadata.status.clone(),
                observed_status: metadata.observed_status.clone(),
                confidence: Some(1.0),
                summary: None,
                source: Some("agent".into()),
            },
        );
        Ok(())
    }

    pub(crate) fn persist_printed_plan_fallback(
        &self,
        session_id: &str,
        printed_path: &str,
    ) -> bool {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let Some(workspace) = workspace_guard.as_ref() else {
            return false;
        };
        if plan_handoff::resolve_openable_plan(&workspace.path, printed_path).is_err() {
            return false;
        }
        let Some(mut metadata) = workspace.metadata.read_session(session_id) else {
            return false;
        };
        if metadata.plan_path.is_some()
            || workspace
                .metadata
                .plan_path_is_explicitly_cleared(session_id)
        {
            return false;
        }
        metadata.plan_path = Some(metadata::PlanReference {
            worktree_root: Some(workspace.path.to_string_lossy().into_owned()),
            relative_path: printed_path.to_string(),
            source: metadata::PlanSource::TerminalFallback,
        });
        workspace.metadata.write_session(&metadata);
        true
    }

    /// Persists a process-owned status transition after the runtime has
    /// promoted its live session handle. The sessions lock is intentionally
    /// not acquired here; runtime callers release it before this workspace
    /// metadata write.
    pub(crate) fn persist_process_transition(
        &self,
        session_id: &str,
        transition: crate::runtime::observed_status::ProcessTransition,
    ) -> bool {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let Some(workspace) = workspace_guard.as_ref() else {
            return false;
        };
        let Some(mut metadata) = workspace.metadata.read_session(session_id) else {
            return false;
        };
        let fields = crate::runtime::observed_status::process_transition_fields(transition);
        crate::runtime::observed_status::apply_process_transition_to_meta(&mut metadata, &fields);
        workspace.metadata.write_session(&metadata);
        true
    }

    pub(crate) fn read_plan_content(&self, id: &str) -> Result<String, SessionError> {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard.as_ref().ok_or(SessionError::Conflict)?;
        let metadata = workspace
            .metadata
            .read_session(id)
            .ok_or(SessionError::NotFound)?;
        let plan_path = metadata.plan_path.ok_or(SessionError::Conflict)?;
        let path = resolve_openable_plan_reference(&workspace.path, &plan_path)
            .map_err(|_| SessionError::Conflict)?;
        std::fs::read_to_string(path).map_err(|_| SessionError::Conflict)
    }

    pub(crate) async fn request_plan_review(&self, id: &str) -> Result<(), SessionError> {
        let plan_path = {
            let workspace_guard = self.state.workspace.lock().unwrap();
            let workspace = workspace_guard.as_ref().ok_or(SessionError::Conflict)?;
            let metadata = workspace
                .metadata
                .read_session(id)
                .ok_or(SessionError::NotFound)?;
            if metadata.lifecycle != "alive" {
                return Err(SessionError::Conflict);
            }
            let path = metadata.plan_path.ok_or(SessionError::Conflict)?;
            let resolved = resolve_openable_plan_reference(&workspace.path, &path)
                .map_err(|_| SessionError::Conflict)?;
            let launch_root = std::path::Path::new(&metadata.cwd).canonicalize().ok();
            let selected_root = path
                .worktree_root
                .as_deref()
                .and_then(|root| std::path::Path::new(root).canonicalize().ok());
            if selected_root.is_some() && selected_root != launch_root {
                resolved.to_string_lossy().into_owned()
            } else {
                path.relative_path
            }
        };
        let prompt = format!(
            "Please review the plan or specification at {plan_path}. Delegate this review to a subagent if you can; otherwise review it yourself. Check for missing requirements, risky assumptions, and unclear steps, then report the findings.\r"
        );
        crate::runtime::terminal_runtime::submit_approved_input(&self.state, id, prompt)
            .await
            .map_err(|_| SessionError::Conflict)?;
        if let Some(workspace) = self.state.workspace.lock().unwrap().as_ref() {
            workspace.metadata.append_event(
                id,
                &metadata::Event {
                    event_type: "plan_review_requested".into(),
                    timestamp: iso_now(),
                    status: "working".into(),
                    observed_status: Some("working".into()),
                    confidence: None,
                    summary: Some("User requested plan review.".into()),
                    source: Some("user".into()),
                },
            );
        }
        Ok(())
    }

    pub(crate) async fn report_attention(
        &self,
        id: &str,
        signal: AttentionSignal,
    ) -> Result<(), SessionError> {
        let observed_at = signal
            .observed_at
            .as_deref()
            .map(parse_hook_observed_at)
            .transpose()
            .map_err(|_| SessionError::EmptyBadRequest)?;
        let active_alias = matches!(signal.status.as_str(), "thinking" | "reasoning");
        if !active_alias && !peon::is_valid_observed_status(&signal.status) {
            return Err(SessionError::EmptyBadRequest);
        }
        let supports_active_work = self
            .state
            .sessions
            .lock()
            .unwrap()
            .get(id)
            .is_some_and(|handle| handle.active_work_hook);
        let status = normalize_hook_attention_status(&signal.status, supports_active_work)
            .ok_or(SessionError::EmptyBadRequest)?;
        if observed_at.is_some_and(|timestamp| {
            self.state
                .sessions
                .lock()
                .unwrap()
                .get(id)
                .and_then(|handle| handle.runtime.accepted_input_at)
                .is_some_and(|accepted_at| timestamp <= accepted_at)
        }) {
            return self
                .workspace_exists()
                .then_some(())
                .ok_or(SessionError::Conflict);
        }
        if let Some(cwd) = signal.cwd.as_deref().filter(|cwd| !cwd.is_empty()) {
            self.state
                .peon
                .reported_cwd
                .write()
                .unwrap()
                .insert(id.to_string(), cwd.to_string());
        }
        let state = self.state.clone();
        let id = id.to_string();
        let merge_id = id.clone();
        let merge_status = status.clone();
        let message = signal.message;
        let plan_path = signal.plan_path;
        let result = tokio::task::spawn_blocking(move || {
            if observed_at.is_some_and(|timestamp| {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&merge_id)
                    .and_then(|handle| handle.runtime.accepted_input_at)
                    .is_some_and(|accepted_at| timestamp <= accepted_at)
            }) {
                return Ok(metadata::AttentionMergeResult::Ignored);
            }
            SessionApplication::new(state).apply_attention_signal(AttentionMergeSignal {
                session_id: merge_id,
                observed_status: merge_status,
                message,
                plan_path,
                timestamp: iso_now(),
                source: "agent".into(),
                confidence: 1.0,
                observed_at,
                reject_stale_observed_at: true,
                update_hook_timestamp: true,
                clear_pending_work_signal: true,
                require_alive: false,
                debug_hint_mutation: None,
            })
        })
        .await
        .map_err(|error| {
            tracing::error!(%error, "attention metadata task failed");
            SessionError::Internal("application operation failed")
        })??;
        if result == metadata::AttentionMergeResult::Accepted && status == "working" {
            if let Some(observed_at) = observed_at {
                clear_claude_capacity_after_working(&self.state, &id, observed_at);
            }
        }
        if result == metadata::AttentionMergeResult::Accepted {
            let mut bufs = self.state.peon.input_buf.write().unwrap();
            if bufs
                .get(&id)
                .is_some_and(|buf| !peon::is_descriptive_input(buf))
            {
                bufs.remove(&id);
            }
        }
        match result {
            metadata::AttentionMergeResult::Accepted | metadata::AttentionMergeResult::Ignored => {
                Ok(())
            }
            metadata::AttentionMergeResult::NotFound => Err(SessionError::NotFound),
            metadata::AttentionMergeResult::PersistFailed => {
                Err(SessionError::Internal("application operation failed"))
            }
        }
    }

    fn workspace_exists(&self) -> bool {
        self.state.workspace.lock().unwrap().is_some()
    }

    pub(crate) async fn select_plan(
        &self,
        id: &str,
        selection: PlanSelection,
    ) -> Result<(), SessionError> {
        let PlanSelection { printed_path } = selection;
        let state = self.state.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().ok_or(SessionError::Conflict)?;
            let mut meta = workspace.metadata.read_session(&id).ok_or(SessionError::NotFound)?;
            let (worktree_root, relative_path) = resolve_printed_plan_path(
                std::path::Path::new(&meta.cwd),
                &printed_path,
            ).map_err(|error| {
                tracing::warn!(session_id = %id, printed_path = %printed_path, %error, "select_terminal_plan: plan path resolution failed");
                SessionError::Conflict
            })?;
            meta.plan_path = Some(metadata::PlanReference {
                worktree_root: Some(worktree_root.to_string_lossy().into_owned()),
                relative_path,
                source: metadata::PlanSource::UserSelected,
            });
            workspace.metadata.try_write_session(&meta)
                .map_err(|_| SessionError::Internal("application operation failed"))?;
            workspace.metadata.append_event(&id, &metadata::Event {
                event_type: "session.plan_selected_by_user".into(), timestamp: iso_now(),
                status: meta.status, observed_status: meta.observed_status,
                confidence: Some(1.0), summary: None, source: Some("user".into()),
            });
            Ok(())
        }).await.map_err(|_| SessionError::Internal("application operation failed"))??;
        Ok(())
    }

    pub(crate) fn set_active_session(&self, session_id: &str) -> Result<(), SessionError> {
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard.as_ref().ok_or(SessionError::Conflict)?;
        let existing = workspace.metadata.read_workspace_memory();
        workspace
            .metadata
            .write_workspace_memory(&metadata::WorkspaceMemory {
                last_active_session_id: Some(session_id.to_string()),
                last_active_at: Some(iso_now()),
                active_harness_ids: existing
                    .as_ref()
                    .map(|memory| memory.active_harness_ids.clone())
                    .unwrap_or_default(),
                active_harness_revision: existing
                    .as_ref()
                    .map_or(0, |memory| memory.active_harness_revision),
            });
        Ok(())
    }

    pub(crate) fn set_active_harnesses(
        &self,
        active_harness_ids: Vec<String>,
    ) -> Result<(), SessionError> {
        let expected_active_harness_revision = self
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .ok_or(SessionError::Conflict)?
            .metadata
            .read_workspace_memory()
            .unwrap_or_default()
            .active_harness_revision;
        self.set_active_harnesses_at(active_harness_ids, expected_active_harness_revision)
            .map(|_| ())
    }

    pub(crate) fn set_active_harnesses_at(
        &self,
        active_harness_ids: Vec<String>,
        expected_active_harness_revision: u64,
    ) -> Result<metadata::WorkspaceMemory, SessionError> {
        let _projection = self
            .state
            .projection_lock
            .lock()
            .expect("projection lock poisoned");
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard.as_ref().ok_or(SessionError::Conflict)?;
        let existing = workspace
            .metadata
            .read_workspace_memory()
            .unwrap_or_default();
        if existing.active_harness_revision != expected_active_harness_revision {
            return Err(SessionError::Conflict);
        }
        let registry = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned")
            .clone();
        for id in &active_harness_ids {
            let Some(harness) = registry.get(id) else {
                return Err(SessionError::BadRequest(
                    "The selected coding tool is no longer available.",
                ));
            };
            if harness.definition.retired {
                return Err(SessionError::BadRequest(
                    "The selected coding tool is retired and cannot be enabled.",
                ));
            }
        }
        let memory = metadata::WorkspaceMemory {
            last_active_session_id: existing.last_active_session_id,
            last_active_at: Some(iso_now()),
            active_harness_ids,
            active_harness_revision: existing.active_harness_revision.saturating_add(1),
        };
        workspace.metadata.write_workspace_memory(&memory);
        Ok(memory)
    }

    pub(crate) async fn delete_session(&self, id: &str) -> Result<(), SessionError> {
        let kill_tx = self
            .state
            .sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|handle| handle.kill_tx.clone())
            .ok_or(SessionError::NotFound)?;
        let _ = kill_tx.send(true);
        crate::runtime::terminal_runtime::set_session_status(&self.state, id, "killed").await;
        self.clear_ended_session_tracking(id);
        Ok(())
    }

    pub(crate) async fn forget_session(&self, id: &str) -> Result<(), SessionError> {
        {
            let sessions = self.state.sessions.lock().unwrap();
            if let Some(handle) = sessions.get(id) {
                if matches!(handle.info.status.as_str(), "live" | "creating" | "running") {
                    return Err(SessionError::ConflictWithMessage(
                        "Cannot forget a live session. Kill it first.",
                    ));
                }
            }
        }

        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard.as_ref().ok_or(SessionError::Conflict)?;
        if !workspace.metadata.session_file_exists(id) {
            return Err(SessionError::NotFound);
        }
        if let Err(error) =
            crate::runtime::retention::delete_session_evidence(workspace, id, |session_id| {
                workspace
                    .recommendation_store
                    .delete_referencing_session(session_id)
                    .map_err(|error| error.to_string())
            })
        {
            tracing::error!(session_id = %id, %error, "failed to delete session evidence");
            if !workspace.metadata.session_file_exists(id) {
                drop(workspace_guard);
                self.state.sessions.lock().unwrap().remove(id);
                self.clear_ended_session_tracking(id);
                self.clear_forgotten_session_tracking(id);
            }
            return Err(SessionError::Internal("application operation failed"));
        }
        drop(workspace_guard);

        self.state.sessions.lock().unwrap().remove(id);
        self.clear_ended_session_tracking(id);
        self.clear_forgotten_session_tracking(id);
        Ok(())
    }
}

fn workspace_path_matches(active_path: &Path, captured_path: Option<&Path>) -> bool {
    captured_path.is_some_and(|captured| captured == active_path)
}

fn peon_observation_key(
    runtime_instance_id: &str,
    session_id: &str,
    run_generation: u64,
    first_revision: u64,
    last_revision: u64,
    candidate_index: usize,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"peon-v1|");
    hasher.update(runtime_instance_id.as_bytes());
    hasher.update(b"|");
    hasher.update(session_id.as_bytes());
    hasher.update(b"|");
    hasher.update(run_generation.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(first_revision.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(last_revision.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(candidate_index.to_string().as_bytes());
    hex::encode(hasher.finalize())
}

fn is_retryable_observation_record_error(
    error: &crate::workflow_observations::RecordError,
) -> bool {
    matches!(
        error,
        crate::workflow_observations::RecordError::PersistFailed
            | crate::workflow_observations::RecordError::RateLimited
            | crate::workflow_observations::RecordError::Degraded
    )
}

pub(crate) fn resume_handle_conflicts(
    handle: &SessionHandle,
    metadata_ended: bool,
    has_tracked_pid: bool,
) -> bool {
    handle.info.lifecycle_phase == "ending"
        || handle.resume_in_progress
        || handle.terminal_attached
        || !metadata_ended
        || has_tracked_pid
}

struct ResumeRollback {
    workspace_path: PathBuf,
    metadata: metadata::SessionMetadata,
    terminal_size: Option<(u16, u16)>,
}

pub(crate) struct ResumeAdmission {
    state: Arc<AppState>,
    id: String,
    generation: crate::runtime::session_runtime::RuntimeGeneration,
    previous_handle: Option<SessionHandle>,
    rollback: Option<ResumeRollback>,
    committed: bool,
}

impl ResumeAdmission {
    pub(crate) fn generation(&self) -> crate::runtime::session_runtime::RuntimeGeneration {
        self.generation
    }

    pub(crate) fn arm_rollback(
        &mut self,
        workspace_path: PathBuf,
        metadata: metadata::SessionMetadata,
        terminal_size: Option<(u16, u16)>,
    ) {
        self.rollback = Some(ResumeRollback {
            workspace_path,
            metadata,
            terminal_size,
        });
    }

    pub(crate) fn commit(mut self) {
        self.committed = true;
        self.previous_handle = None;
        self.rollback = None;
    }
}

fn should_persist_output_recency(existing: Option<&str>, incoming: &str) -> bool {
    let Ok(incoming) = chrono::DateTime::parse_from_rfc3339(incoming) else {
        return false;
    };
    let Some(existing) = existing else {
        return true;
    };
    let Ok(existing) = chrono::DateTime::parse_from_rfc3339(existing) else {
        return true;
    };
    incoming >= existing
}

impl Drop for ResumeAdmission {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        let restored_generation = {
            let mut sessions = self.state.sessions.lock().unwrap();
            if !sessions.get(&self.id).is_some_and(|handle| {
                handle.runtime.run_generation() == self.generation
                    && handle.resume_in_progress
                    && !handle.runtime.startup_spawned()
            }) {
                None
            } else {
                match self.previous_handle.take() {
                    Some(previous) => {
                        let generation = previous.runtime.run_generation();
                        sessions.insert(self.id.clone(), previous);
                        Some(Some(generation))
                    }
                    None => {
                        sessions.remove(&self.id);
                        Some(None)
                    }
                }
            }
        };
        let Some(restored_generation) = restored_generation else {
            return;
        };

        let Some(rollback) = self.rollback.take() else {
            return;
        };
        let ws_guard = self.state.workspace.lock().unwrap();
        let Some(ws) = ws_guard
            .as_ref()
            .filter(|workspace| workspace.path == rollback.workspace_path)
        else {
            return;
        };
        // Another resume may claim the restored handle while cancellation is
        // waiting for the workspace lock. Recheck the post-rollback registry
        // state and keep the sessions lock through both persisted restorations
        // so a newer generation can never be overwritten after this check.
        let sessions = self.state.sessions.lock().unwrap();
        let still_owns_persisted_rollback = match restored_generation {
            Some(generation) => sessions
                .get(&self.id)
                .is_some_and(|handle| handle.runtime.run_generation() == generation),
            None => !sessions.contains_key(&self.id),
        };
        if !still_owns_persisted_rollback {
            return;
        }
        ws.metadata.write_session(&rollback.metadata);
        match rollback.terminal_size {
            Some((cols, rows)) => ws.metadata.write_terminal_size(&self.id, cols, rows),
            None => ws.metadata.clear_terminal_size(&self.id),
        }
    }
}

pub(crate) fn try_install_claimed_resume_handle(
    state: &Arc<AppState>,
    id: &str,
    mut replacement: SessionHandle,
    metadata_ended: bool,
    expected_generation: Option<crate::runtime::session_runtime::RuntimeGeneration>,
) -> Result<ResumeAdmission, ()> {
    let mut sessions = state.sessions.lock().unwrap();
    let current_generation = sessions
        .get(id)
        .map(|handle| handle.runtime.run_generation());
    let has_tracked_pid = state.session_pids.lock().unwrap().contains_key(id);
    if current_generation != expected_generation
        || !metadata_ended
        || has_tracked_pid
        || sessions
            .get(id)
            .is_some_and(|handle| resume_handle_conflicts(handle, metadata_ended, has_tracked_pid))
    {
        return Err(());
    }

    replacement.resume_in_progress = true;
    let generation = replacement.runtime.run_generation();
    let previous_handle = sessions.insert(id.to_string(), replacement);
    Ok(ResumeAdmission {
        state: state.clone(),
        id: id.to_string(),
        generation,
        previous_handle,
        rollback: None,
        committed: false,
    })
}

async fn resume_session_workflow(
    state: Arc<AppState>,
    id: &str,
) -> Result<SessionInfo, crate::session_application::SessionError> {
    let id = id.to_string();
    let now = iso_now();
    let expected_generation = state
        .sessions
        .lock()
        .unwrap()
        .get(&id)
        .map(|handle| handle.runtime.run_generation());
    let registry = state
        .harness_catalog
        .read()
        .expect("harness catalog lock poisoned")
        .clone();
    let (meta, command, strategy, resume_flags, capacity_check_pending, active_work_hook) = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ref ws) = *ws_guard else {
            return Err(crate::session_application::SessionError::Conflict);
        };
        let Some(meta) = ws.metadata.read_session(&id) else {
            return Err(crate::session_application::SessionError::NotFound);
        };
        let Some(resume) = meta.resume.as_ref() else {
            return Err(crate::session_application::SessionError::EmptyBadRequest);
        };
        let session_harness_id = (!meta.harness.is_empty()).then_some(meta.harness.as_str());
        let harness = session_harness_id
            .and_then(|id| registry.get(id))
            .or_else(|| registry.get("generic-shell"))
            .expect("generic-shell builtin exists");
        let active_work_hook = harness
            .effective_capabilities
            .contains(&crate::harness::registry::CapabilityName::Attention);
        let strategy = harness.select_resume_strategy(resume);
        if strategy == harness::ResumeStrategy::None {
            return Err(crate::session_application::SessionError::EmptyBadRequest);
        }
        let Some(command) = harness.build_resume(
            strategy.clone(),
            &meta.cwd,
            resume.harness_session_id.as_deref(),
            meta.repo_root.as_deref(),
            (!meta.model.is_empty()).then_some(meta.model.as_str()),
        ) else {
            return Err(crate::session_application::SessionError::EmptyBadRequest);
        };
        (
            meta,
            command,
            strategy,
            harness.resume_flags(),
            !harness.capacity_patterns().is_empty(),
            active_work_hook,
        )
    };

    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);
    let info = SessionInfo {
        id: id.clone(),
        label: meta.label.clone(),
        harness_id: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
        model_provider_id: meta.provider_id.clone(),
        model_id: (!meta.model.is_empty()).then(|| meta.model.clone()),
        harness: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
        model: (!meta.model.is_empty()).then(|| meta.model.clone()),
        work_phase: meta.work_phase.clone(),
        lifecycle_phase: "creating".into(),
        lifecycle: "creating".into(),
        attention: None,
        status: "creating".into(),
        connectivity: Some(connectivity_for_status("creating").into()),
        terminal_outcome: terminal_outcome_for_status("creating"),
        cwd: command.cwd.clone(),
        created_at: meta.created_at.clone(),
        last_activity_at: Some(now.clone()),
        last_output_at: None,
        // The frozen final state belongs to the previous run; a resumed session
        // is live again and must not resurface it as attention.
        final_observed_status: None,
        observed_status: None,
        summary: meta.summary.clone(),
        next_action: meta.next_action.clone(),
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        at_usage_limit: None,
        capacity_check_pending: capacity_check_pending.then_some(true),
        usage_limit_reset_hint: None,
        metadata_source: Some("process".into()),
        metadata_confidence: Some(1.0),
        repo_root: meta.repo_root.clone(),
        branch: meta.branch.clone(),
        dirty: meta.dirty,
        changed_files: meta.changed_files,
        is_worktree: meta.is_worktree,
        conflict_warning: None,
        recommendation: None,
        peon_last_inference: None,
        peon_diagnostics: None,
        memory_state: MemoryState::Live,
        resume_strategy: strategy.clone(),
        resume: meta.resume.clone(),
        resume_options: metadata::derive_resume_options(
            &strategy,
            meta.resume.as_ref(),
            resume_flags.0,
            resume_flags.1,
            resume_flags.2,
        ),
        resumed_from: meta.resumed_from.clone(),
        has_openable_plan: None,
        provider: meta.provider_label.clone(),
        provider_model: meta.provider_model.clone(),
        provider_state: meta.provider_state.clone(),
    };

    let (runtime, control_rx) = crate::runtime::session_runtime::SessionRuntime::live(
        crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
        crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
    );
    let output_tx = runtime.output_tx.clone();

    let replacement = SessionHandle {
        info: info.clone(),
        active_work_hook,
        kill_tx: kill_tx.clone(),
        output_buffer: peon::RingBuffer::new(state.peon.config.max_lines),
        scan_buf: String::new(),
        pending_work_signal: None,
        runtime,
        terminal_attached: false,
        resume_in_progress: false,
        at_usage_limit_latched: false,
        capacity_check_pending,
        output_lines_seen: 0,
        scan_bytes_seen: 0,
        resume_scan_origin: capacity_check_pending.then_some((0, 0)),
        pending_capacity_visible_once: false,
    };
    let mut admission = match try_install_claimed_resume_handle(
        &state,
        &id,
        replacement,
        meta.lifecycle_phase == "ended",
        expected_generation,
    ) {
        Ok(admission) => admission,
        Err(()) => return Err(crate::session_application::SessionError::Conflict),
    };
    let run_generation = admission.generation();

    {
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            admission.arm_rollback(
                ws.path.clone(),
                meta.clone(),
                ws.metadata.read_terminal_size(&id),
            );
            // Drop any recorded terminal-size sidecar from the prior run before the
            // resumed runtime starts. If the daemon exits before the resumed run
            // reaches another terminal-status transition there is no in-memory
            // handle to overwrite the size, so leaving the prior grid in place
            // would replay the new run's output against a stale grid. Clearing
            // falls back to documented fit-to-container replay for that case.
            ws.metadata.clear_terminal_size(&id);
            if let Some(mut stored_meta) = ws.metadata.read_session(&id) {
                stored_meta.status = "creating".to_string();
                stored_meta.lifecycle_phase = "creating".to_string();
                stored_meta.lifecycle = "creating".to_string();
                stored_meta.attention = None;
                stored_meta.pending_terminal_status = None;
                stored_meta.ending_observed_status_snapshot = None;
                stored_meta.final_observed_status_snapshot = None;
                stored_meta.observed_status = None;
                stored_meta.connectivity = connectivity_for_status("creating").to_string();
                stored_meta.terminal_outcome = None;
                stored_meta.last_activity = now.clone();
                stored_meta.resume = meta.resume.clone();
                stored_meta.resume_options = meta.resume_options.clone();
                stored_meta.resumed_from = meta.resumed_from.clone();
                ws.metadata.write_session(&stored_meta);
            }
        }
    }

    let startup_state = state.clone();
    let startup_id = id.clone();
    let startup_task = tokio::spawn(async move {
        let start_result = crate::runtime::session_runtime::start_session_runtime(
            startup_state.clone(),
            startup_id.clone(),
            command,
            None,
            control_rx,
            output_tx,
            kill_tx.subscribe(),
            PtySize {
                rows: crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                cols: crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .await;

        if let Err(error) = &start_result {
            admission.commit();
            tracing::error!(session_id = %startup_id, %error, "failed to start resumed session runtime");
            if crate::runtime::terminal_runtime::set_session_status_for_generation(
                &startup_state,
                &startup_id,
                run_generation,
                "error",
            )
            .await
            {
                crate::runtime::terminal_runtime::schedule_session_ending_finalization(
                    startup_state.clone(),
                    startup_id.clone(),
                    run_generation,
                    "error".into(),
                );
            }
        } else {
            admission.commit();
        }
        start_result
    });
    let start_result = startup_task
        .await
        .map_err(|error| format!("resumed runtime startup task failed: {error}"))
        .and_then(|result| result);

    match start_result {
        Ok(()) => {}
        Err(error) => {
            tracing::error!(session_id = %id, %error, "failed to start resumed session runtime");
            return Err(crate::session_application::SessionError::Internal(
                "application operation failed",
            ));
        }
    }
    let info = state
        .sessions
        .lock()
        .unwrap()
        .get(&id)
        .map(|handle| handle.info.clone())
        .expect("resumed session remains registered");

    {
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            ws.metadata.append_event(
                &id,
                &metadata::Event {
                    event_type: "session.resumed".into(),
                    timestamp: now,
                    status: "running".into(),
                    observed_status: None,
                    confidence: None,
                    summary: None,
                    source: None,
                },
            );
        }
    }

    Ok(info)
}

pub(crate) struct ResolvedSessionLaunch {
    pub(crate) session_harness_id: Option<String>,
    pub(crate) active_work_hook: bool,
    pub(crate) model: Option<String>,
    pub(crate) command: harness::CommandSpec,
    pub(crate) provider_id: Option<String>,
    pub(crate) provider_label: Option<String>,
}

pub(crate) fn resolve_session_launch(
    registry: &crate::harness::registry::ResolvedHarnessRegistry,
    command: &CreateSessionCommand,
    cwd: String,
) -> ResolvedSessionLaunch {
    let requested_id = command.harness_id.as_deref();
    let harness = requested_id
        .and_then(|id| registry.get(id))
        .filter(|harness| !harness.definition.retired)
        .or_else(|| registry.get("generic-shell"))
        .expect("generic-shell builtin exists");
    let model = command
        .model
        .clone()
        .or_else(|| harness.definition.default_model.clone());
    ResolvedSessionLaunch {
        session_harness_id: Some(harness.definition.id.clone()),
        active_work_hook: harness
            .effective_capabilities
            .contains(&crate::harness::registry::CapabilityName::Attention),
        command: harness.build_launch(&cwd, model.as_deref()),
        provider_id: None,
        provider_label: None,
        model,
    }
}

async fn create_session_workflow(
    state: Arc<AppState>,
    req: CreateSessionCommand,
) -> Result<SessionInfo, crate::session_application::SessionError> {
    let id = uuid::Uuid::new_v4().to_string();
    let (workspace_cwd, workspace_metadata_root) = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .map(|workspace| {
            (
                workspace.path.display().to_string(),
                workspace.metadata.root_path(),
            )
        })
        .unzip();
    let cwd = workspace_cwd
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|path| path.display().to_string())
        })
        .unwrap_or_else(|| "/".into());
    let registry = state
        .harness_catalog
        .read()
        .expect("harness catalog lock poisoned")
        .clone();
    if req.harness_id.as_deref().is_some_and(|id| {
        registry
            .get(id)
            .is_some_and(|harness| harness.definition.retired)
    }) {
        return Err(crate::session_application::SessionError::BadRequest(
            "The selected coding tool is retired and cannot start new sessions.",
        ));
    }
    let mut resolved_launch = resolve_session_launch(&registry, &req, cwd.clone());
    let integration_enabled = if let Some(harness) = resolved_launch
        .session_harness_id
        .as_deref()
        .and_then(|id| registry.get(id))
    {
        let metadata_root = workspace_metadata_root.clone();
        let harness_id = harness.definition.id.clone();
        let registry = registry.clone();
        match tokio::task::spawn_blocking(move || {
            registry.get(&harness_id).map_or(Ok(false), |harness| {
                harness.integration_launch_enabled(metadata_root.as_deref())
            })
        })
        .await
        {
            Ok(Ok(enabled)) => enabled,
            Ok(Err(error)) => {
                tracing::warn!(
                    code = error.code(),
                    "harness launch integration state was ignored"
                );
                false
            }
            Err(error) => {
                tracing::warn!(%error, "harness launch integration state task failed");
                false
            }
        }
    } else {
        false
    };
    if let Some(harness) = resolved_launch
        .session_harness_id
        .as_deref()
        .and_then(|id| registry.get(id))
    {
        let reporter = crate::harness::integration::default_reporter_path();
        if let Err(error) = harness.augment_launch_for_integration(
            &mut resolved_launch.command,
            integration_enabled,
            reporter.as_deref(),
        ) {
            tracing::warn!(
                code = error.code(),
                "harness launch integration was not applied"
            );
        }
    }

    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);

    let git_ctx = git::detect(&PathBuf::from(&cwd));
    let now = iso_now();
    let mut info = SessionInfo {
        id: id.clone(),
        label: crate::session_types::placeholder_label(&id),
        harness_id: resolved_launch.session_harness_id.clone(),
        model_provider_id: resolved_launch.provider_id.clone(),
        model_id: resolved_launch.model.clone(),
        harness: resolved_launch.session_harness_id.clone(),
        model: resolved_launch.model.clone(),
        work_phase: "unknown".into(),
        lifecycle_phase: "creating".into(),
        lifecycle: "creating".into(),
        attention: None,
        status: "creating".into(),
        connectivity: Some(connectivity_for_status("creating").into()),
        terminal_outcome: terminal_outcome_for_status("creating"),
        cwd,
        created_at: now.clone(),
        last_activity_at: Some(now.clone()),
        last_output_at: None,
        final_observed_status: None,
        observed_status: None,
        summary: None,
        next_action: None,
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        at_usage_limit: None,
        capacity_check_pending: None,
        usage_limit_reset_hint: None,
        metadata_source: None,
        metadata_confidence: None,
        repo_root: git_ctx.repo_root.clone(),
        branch: git_ctx.branch.clone(),
        dirty: Some(git_ctx.dirty),
        changed_files: Some(git_ctx.changed_files),
        is_worktree: Some(git_ctx.is_worktree),
        conflict_warning: None,
        recommendation: None,
        peon_last_inference: None,
        peon_diagnostics: None,
        memory_state: MemoryState::Live,
        resume_strategy: harness::ResumeStrategy::None,
        resume: Some(harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some(now.clone()),
        }),
        resume_options: vec![],
        resumed_from: None,
        has_openable_plan: None,
        provider: resolved_launch.provider_label.clone(),
        provider_model: None,
        provider_state: None,
    };

    let command = resolved_launch.command.clone();
    let initial_prompt = req.initial_prompt.clone();
    // A hookless harness never gets a `report_attention` call, so the initial
    // prompt (written straight to the PTY in `start_session_runtime`) must arm
    // the same fallback a typed-and-submitted command would, or the session's
    // first turn never promotes past creating/idle while Peon is disabled.
    let pending_work_signal = initial_prompt
        .as_deref()
        .filter(|prompt| !prompt.is_empty() && !resolved_launch.active_work_hook)
        .map(|prompt| {
            crate::runtime::session_runtime::arm_pending_work_signal(
                prompt,
                tokio::time::Instant::now(),
            )
        });
    // The initial prompt is written straight to the PTY (bypassing the
    // keystroke-based label seeding in terminal_runtime.rs), so it never gets
    // a chance at seeding the label there. Seed it here instead: the
    // synchronous fallback below (so the title is never blank) plus Peon's
    // topic-inference queue for the real, LLM-phrased topic (ADR 0029).
    if let Some(prompt) = initial_prompt.as_deref() {
        let label_line: String = prompt.chars().take(100).collect();
        if peon::is_descriptive_input(&label_line) {
            info.label = label_line.clone();
            crate::runtime::terminal_runtime::queue_label_hint(&state, &id, prompt.to_string());
        }
    }

    let (runtime, control_rx) = crate::runtime::session_runtime::SessionRuntime::live(
        crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
        crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
    );
    let run_generation = runtime.run_generation();
    let output_tx = runtime.output_tx.clone();
    state.sessions.lock().unwrap().insert(
        id.clone(),
        SessionHandle {
            info: info.clone(),
            active_work_hook: resolved_launch.active_work_hook,
            kill_tx: kill_tx.clone(),
            output_buffer: peon::RingBuffer::new(state.peon.config.max_lines),
            scan_buf: String::new(),
            pending_work_signal,
            runtime,
            terminal_attached: false,
            resume_in_progress: false,
            at_usage_limit_latched: false,
            capacity_check_pending: false,
            output_lines_seen: 0,
            scan_bytes_seen: 0,
            resume_scan_origin: None,
            pending_capacity_visible_once: false,
        },
    );

    // Persist the creating record before the PTY reader exists. The runtime
    // promotes it to alive immediately after spawn, before it can classify
    // output, so the first output cannot be lost between memory and metadata.
    let created_at = iso_now();
    {
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            let meta_git_ctx = git::detect(&ws.path);
            ws.metadata.write_session(&metadata::SessionMetadata {
                id: id.clone(),
                label: info.label.clone(),
                workspace: ws.path.display().to_string(),
                task: String::new(),
                harness: resolved_launch
                    .session_harness_id
                    .clone()
                    .unwrap_or_default(),
                model: resolved_launch.model.clone().unwrap_or_default(),
                cwd: info.cwd.clone(),
                status: "creating".into(),
                work_phase: "unknown".into(),
                lifecycle_phase: "creating".into(),
                lifecycle: "creating".into(),
                attention: None,
                plan_path: None,
                connectivity: "online".into(),
                terminal_outcome: None,
                pending_terminal_status: None,
                observed_status: None,
                ending_observed_status_snapshot: None,
                final_observed_status_snapshot: None,
                summary: None,
                next_action: None,
                needs_user_input: None,
                detected_question: None,
                suggested_options: None,
                blocker_description: None,
                failed_command: None,
                failed_test: None,
                capacity_hints: None,
                peon_last_inference: None,
                provider_id: resolved_launch.provider_id.clone(),
                provider_label: resolved_launch.provider_label.clone(),
                provider_model: None,
                provider_state: None,
                created_at: created_at.clone(),
                last_activity: created_at.clone(),
                last_output_at: None,
                metadata_source: "process".into(),
                metadata_confidence: 1.0,
                repo_root: meta_git_ctx.repo_root.clone(),
                branch: meta_git_ctx.branch.clone(),
                dirty: Some(meta_git_ctx.dirty),
                changed_files: Some(meta_git_ctx.changed_files),
                is_worktree: Some(meta_git_ctx.is_worktree),
                last_user_input: None,
                resume: info.resume.clone(),
                resume_options: vec![],
                harness_session_id_source: None,
                harness_session_id_confidence: None,
                harness_session_id_captured_at: None,
                resumed_from: info.resumed_from.clone(),
            });
        }
    }

    // Spawn detached rather than awaiting: awaiting here would delay this
    // handler's response until after the PTY spawn completes, so the client
    // would never actually observe `status: "creating"` (issue #302) — the
    // response below always reports the pre-spawn record. Mirrors the
    // pattern resume_session's startup_task already proved safe, including
    // the generation guards that keep it correct against concurrent deletion.
    let startup_state = state.clone();
    let startup_id = id.clone();
    tokio::spawn(async move {
        match crate::runtime::session_runtime::start_session_runtime(
            startup_state.clone(),
            startup_id.clone(),
            command,
            initial_prompt,
            control_rx,
            output_tx,
            kill_tx.subscribe(),
            PtySize {
                rows: crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                cols: crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .await
        {
            Ok(()) => {
                let now = iso_now();
                let ws_guard = startup_state.workspace.lock().unwrap();
                if let Some(ref ws) = *ws_guard {
                    ws.metadata.append_event(
                        &startup_id,
                        &metadata::Event {
                            event_type: "session.created".into(),
                            timestamp: now,
                            status: "running".into(),
                            observed_status: None,
                            confidence: None,
                            summary: None,
                            source: None,
                        },
                    );
                }
            }
            Err(error) => {
                tracing::error!(session_id = %startup_id, %error, "failed to start session runtime");
                if crate::runtime::terminal_runtime::set_session_status_for_generation(
                    &startup_state,
                    &startup_id,
                    run_generation,
                    "error",
                )
                .await
                {
                    crate::runtime::terminal_runtime::schedule_session_ending_finalization(
                        startup_state.clone(),
                        startup_id.clone(),
                        run_generation,
                        "error".into(),
                    );
                }
            }
        }
    });

    Ok(info)
}

fn normalize_hook_attention_status(status: &str, supports_active_work: bool) -> Option<String> {
    match status {
        "working" | "thinking" | "reasoning" if supports_active_work => Some("working".into()),
        "waiting_for_input" | "blocked" | "failed" | "done" | "stale" | "idle" => {
            Some(status.into())
        }
        _ => None,
    }
}

fn clear_claude_capacity_after_working(
    state: &Arc<AppState>,
    id: &str,
    observed_at: chrono::DateTime<chrono::Utc>,
) {
    let mut sessions = state.sessions.lock().unwrap();
    let Some(harness_id) = sessions
        .get(id)
        .and_then(|handle| handle.info.harness_id.as_deref())
        .map(str::to_owned)
        .filter(|harness_id| *harness_id == "claude-code")
    else {
        return;
    };
    if sessions.values().any(|handle| {
        handle.info.harness_id.as_deref() == Some(harness_id.as_str())
            && handle.at_usage_limit_latched
            && handle
                .runtime
                .usage_limit_latched_at
                .is_some_and(|latched_at| latched_at > observed_at)
    }) {
        return;
    }
    for handle in sessions.values_mut() {
        if handle.info.harness_id.as_deref() == Some(harness_id.as_str()) {
            handle.at_usage_limit_latched = false;
            handle.runtime.usage_limit_latched_at = None;
            handle.resume_scan_origin = Some((handle.output_lines_seen, handle.scan_bytes_seen));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_agent_workflow_observation_through_application() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "agent-workflow-observation-application";
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&crate::test_support::test_session_metadata(
                id,
                "Agent observation",
                &root.path().display().to_string(),
                "running",
                "before",
                "before",
            ));

        let result = SessionApplication::new(state.clone()).record_agent_workflow_observation(
            id,
            "agent-key-1",
            crate::workflow_observations::ObservationCandidate {
                kind: crate::workflow_observations::ObservationKind::Obstacle,
                description: "The same command needed another retry".into(),
                evidence: "cargo test failed again".into(),
                reported_impact: crate::workflow_observations::Impact::Medium,
                confidence: Some(0.1),
            },
        );

        let observation = match result.unwrap() {
            crate::workflow_observations::RecordOutcome::Accepted(observation) => observation,
            other => panic!("expected accepted observation, got {other:?}"),
        };
        assert_eq!(observation.session_id, id);
        assert_eq!(
            observation.source,
            crate::workflow_observations::ObservationSource::Agent
        );
        assert_eq!(observation.confidence, 0.9);
    }

    #[test]
    fn agent_workflow_observation_requires_active_workspace_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let candidate = crate::workflow_observations::ObservationCandidate {
            kind: crate::workflow_observations::ObservationKind::Obstacle,
            description: "The command needed another retry".into(),
            evidence: "cargo test failed again".into(),
            reported_impact: crate::workflow_observations::Impact::Medium,
            confidence: None,
        };

        let missing_session = SessionApplication::new(state.clone())
            .record_agent_workflow_observation("missing-session", "agent-key", candidate.clone());
        assert!(matches!(
            missing_session,
            Err(WorkflowObservationPersistenceError::SessionNotInWorkspace)
        ));

        *state.workspace.lock().unwrap() = None;
        let missing_workspace = SessionApplication::new(state).record_agent_workflow_observation(
            "missing-session",
            "agent-key",
            candidate,
        );
        assert!(matches!(
            missing_workspace,
            Err(WorkflowObservationPersistenceError::NoWorkspace)
        ));
    }

    #[test]
    fn records_peon_observations_with_stable_duplicate_keys() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "workflow-observation-application";
        let metadata = crate::test_support::test_session_metadata(
            id,
            "Workflow observation",
            &root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let range = PeonObservationOutputRange {
            runtime_instance_id: "runtime-a".into(),
            run_generation: 4,
            first_revision: 10,
            last_revision: 12,
        };
        let candidates = vec![peon::PeonWorkflowObservation {
            kind: crate::workflow_observations::ObservationKind::Obstacle,
            description: "A command required an extra retry".into(),
            evidence: "retry output".into(),
            reported_impact: crate::workflow_observations::Impact::Medium,
            confidence: 0.8,
        }];

        let first = SessionApplication::new(state.clone()).record_peon_workflow_observations(
            id,
            Some(root.path()),
            &range,
            &candidates,
        );
        assert!(first.accepted_observation);
        assert!(first.output_range_completed);

        let duplicate = SessionApplication::new(state.clone()).record_peon_workflow_observations(
            id,
            Some(root.path()),
            &range,
            &candidates,
        );
        assert!(!duplicate.accepted_observation);
        assert!(duplicate.output_range_completed);
        let observations = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .workflow_observations
            .workspace_observations()
            .unwrap();
        assert_eq!(observations.len(), 1);
        assert_eq!(
            observations[0].source,
            crate::workflow_observations::ObservationSource::Peon
        );
    }

    #[test]
    fn persists_final_peon_scan_and_treats_duplicate_as_durable_without_acceptance() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "final-scan-application";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Final scan",
            &root.path().display().to_string(),
            "ending",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ending".into();
        metadata.lifecycle = "live".into();
        metadata.terminal_outcome = None;
        metadata.pending_terminal_status = Some("ended".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let candidate = peon::PeonWorkflowObservation {
            kind: crate::workflow_observations::ObservationKind::Obstacle,
            description: "The final scan found a retry".into(),
            evidence: "retry output".into(),
            reported_impact: crate::workflow_observations::Impact::Medium,
            confidence: 0.9,
        };
        let scan = crate::providers::ProviderRunResult {
            inference: Some(peon::PeonInference {
                observed_status: Some("done".into()),
                phase: None,
                summary: None,
                next_action: None,
                needs_user_input: None,
                detected_question: None,
                suggested_options: None,
                blocker_description: None,
                failed_command: None,
                failed_test: None,
                capacity_hints: None,
                confidence: 0.9,
                detected_harness: None,
                detected_model: None,
                harness_session_id: None,
                workflow_observations: vec![candidate],
            }),
            observation: Some(crate::providers::ProviderObservation {
                provider_id: "provider-a".into(),
                provider_label: "Provider A".into(),
                provider_model: Some("model-a".into()),
                provider_state: "healthy".into(),
            }),
            attempts: vec![],
            runtime: std::collections::HashMap::new(),
        };

        let first =
            SessionApplication::new(state.clone()).persist_final_peon_scan(id, 7, Some(&scan));
        assert!(first.should_finalize);
        assert!(first.observation_accepted);
        assert_eq!(first.metadata.unwrap().lifecycle_phase, "ending");

        let duplicate =
            SessionApplication::new(state.clone()).persist_final_peon_scan(id, 7, Some(&scan));
        assert!(duplicate.should_finalize);
        assert!(!duplicate.observation_accepted);
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .workflow_observations
                .workspace_observations()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn persists_provider_context_for_final_scan_without_inference() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "final-scan-provider-only";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Final scan",
            &root.path().display().to_string(),
            "ending",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ending".into();
        metadata.lifecycle = "live".into();
        metadata.terminal_outcome = None;
        metadata.pending_terminal_status = Some("ended".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let scan = crate::providers::ProviderRunResult {
            inference: None,
            observation: Some(crate::providers::ProviderObservation {
                provider_id: "provider-a".into(),
                provider_label: "Provider A".into(),
                provider_model: None,
                provider_state: "degraded".into(),
            }),
            attempts: vec![],
            runtime: std::collections::HashMap::new(),
        };

        let result =
            SessionApplication::new(state.clone()).persist_final_peon_scan(id, 8, Some(&scan));
        assert!(result.should_finalize);
        assert!(!result.observation_accepted);
        let persisted = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(persisted.provider_id.as_deref(), Some("provider-a"));
        assert_eq!(persisted.provider_state.as_deref(), Some("degraded"));
    }

    #[test]
    fn rejects_missing_or_ended_final_scan_sessions_before_writes() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let missing = SessionApplication::new(state.clone()).persist_final_peon_scan(
            "missing-final-scan",
            1,
            None,
        );
        assert!(missing.should_finalize);
        assert!(!missing.observation_accepted);
        assert!(missing.metadata.is_none());

        let provider_scan = crate::providers::ProviderRunResult {
            inference: None,
            observation: Some(crate::providers::ProviderObservation {
                provider_id: "provider-missing-session".into(),
                provider_label: "Provider Missing Session".into(),
                provider_model: None,
                provider_state: "healthy".into(),
            }),
            attempts: vec![],
            runtime: std::collections::HashMap::new(),
        };
        let missing_with_provider = SessionApplication::new(state.clone()).persist_final_peon_scan(
            "missing-final-scan",
            2,
            Some(&provider_scan),
        );
        assert!(missing_with_provider.should_finalize);
        assert!(!missing_with_provider.observation_accepted);
        assert!(missing_with_provider.metadata.is_none());

        let id = "ended-final-scan";
        let metadata = crate::test_support::test_session_metadata(
            id,
            "Ended scan",
            &root.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let ended = SessionApplication::new(state).persist_final_peon_scan(id, 1, None);
        assert!(!ended.should_finalize);
        assert!(!ended.observation_accepted);
        assert_eq!(ended.metadata.unwrap().lifecycle_phase, "ended");
    }

    #[test]
    fn observation_key_binds_runtime_and_captured_range() {
        let first = peon_observation_key("runtime-a", "session", 2, 10, 12, 0);
        assert_eq!(
            first,
            peon_observation_key("runtime-a", "session", 2, 10, 12, 0)
        );
        assert_ne!(
            first,
            peon_observation_key("runtime-b", "session", 2, 10, 12, 0)
        );
        assert_ne!(
            first,
            peon_observation_key("runtime-a", "session", 2, 11, 12, 0)
        );
    }

    fn attention_test_handle(id: &str, cwd: &std::path::Path) -> SessionHandle {
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        SessionHandle {
            info: crate::test_support::test_session_info(
                id,
                "Attention",
                cwd.display().to_string(),
                "running",
                "now",
            ),
            active_work_hook: false,
            kill_tx,
            output_buffer: peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
            ),
            terminal_attached: false,
            resume_in_progress: false,
            at_usage_limit_latched: false,
            capacity_check_pending: false,
            output_lines_seen: 0,
            scan_bytes_seen: 0,
            resume_scan_origin: None,
            pending_capacity_visible_once: false,
        }
    }

    fn usage_limit_test_state(id: &str) -> Arc<AppState> {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let mut handle = attention_test_handle(id, root.path());
        handle.at_usage_limit_latched = true;
        handle.output_lines_seen = 7;
        handle.scan_bytes_seen = 11;
        state.sessions.lock().unwrap().insert(id.into(), handle);
        state
    }

    #[test]
    fn arm_usage_limit_recheck_captures_current_origin() {
        let state = usage_limit_test_state("arm-usage-limit");

        SessionApplication::new(state.clone()).arm_usage_limit_recheck("arm-usage-limit");

        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get("arm-usage-limit")
                .unwrap()
                .resume_scan_origin,
            Some((7, 11))
        );
    }

    #[test]
    fn arm_usage_limit_recheck_does_not_overwrite_existing_origin() {
        let state = usage_limit_test_state("arm-once");
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut("arm-once")
            .unwrap()
            .resume_scan_origin = Some((1, 2));

        SessionApplication::new(state.clone()).arm_usage_limit_recheck("arm-once");

        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get("arm-once")
                .unwrap()
                .resume_scan_origin,
            Some((1, 2))
        );
    }

    #[test]
    fn arm_usage_limit_recheck_skips_pending_capacity_check() {
        let state = usage_limit_test_state("arm-pending");
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut("arm-pending")
            .unwrap()
            .capacity_check_pending = true;

        SessionApplication::new(state.clone()).arm_usage_limit_recheck("arm-pending");

        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get("arm-pending")
                .unwrap()
                .resume_scan_origin,
            None
        );
    }

    #[test]
    fn arm_usage_limit_recheck_skips_unlatched_session() {
        let state = usage_limit_test_state("arm-unlatched");
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut("arm-unlatched")
            .unwrap()
            .at_usage_limit_latched = false;

        SessionApplication::new(state.clone()).arm_usage_limit_recheck("arm-unlatched");

        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get("arm-unlatched")
                .unwrap()
                .resume_scan_origin,
            None
        );
    }

    #[test]
    fn arm_usage_limit_recheck_missing_session_is_noop() {
        let state = usage_limit_test_state("arm-present");

        SessionApplication::new(state.clone()).arm_usage_limit_recheck("arm-missing");

        assert!(state.sessions.lock().unwrap().get("arm-missing").is_none());
    }

    #[test]
    fn apply_idle_timeout_projects_idle_after_persisted_working_gate() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "idle-timeout-application";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Idle timeout",
            &root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.lifecycle = "alive".into();
        metadata.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, root.path()));

        SessionApplication::new(state.clone()).apply_idle_timeout(id);

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("idle"));
        assert_eq!(
            state.sessions.lock().unwrap()[id]
                .info
                .observed_status
                .as_deref(),
            Some("idle")
        );
    }

    #[test]
    fn apply_idle_timeout_persists_idle_for_metadata_only_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "idle-timeout-orphan";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Orphaned idle timeout",
            &root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.lifecycle = "alive".into();
        metadata.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        SessionApplication::new(state.clone()).apply_idle_timeout(id);

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("idle"));
        assert!(!state.sessions.lock().unwrap().contains_key(id));
    }

    #[test]
    fn apply_idle_timeout_skips_specific_persisted_state() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "idle-timeout-specific";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Specific state",
            &root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.lifecycle = "alive".into();
        metadata.lifecycle_phase = "active".into();
        metadata.observed_status = Some("capped".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, root.path()));

        SessionApplication::new(state.clone()).apply_idle_timeout(id);

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("capped"));
        assert_eq!(
            state.sessions.lock().unwrap()[id].info.observed_status,
            None
        );
    }

    #[test]
    fn apply_idle_timeout_does_not_project_when_persist_fails() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "idle-timeout-write-failure";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Write failure",
            &root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.lifecycle = "alive".into();
        metadata.lifecycle_phase = "active".into();
        let sessions_dir = {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            store.write_session(&metadata);
            store.sessions_dir()
        };
        std::fs::create_dir_all(sessions_dir.join(format!("{id}.json.tmp"))).unwrap();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, root.path()));

        SessionApplication::new(state.clone()).apply_idle_timeout(id);

        let persisted = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_ne!(persisted.observed_status.as_deref(), Some("idle"));
        assert_eq!(
            state.sessions.lock().unwrap()[id].info.observed_status,
            None
        );
    }

    #[test]
    fn apply_idle_timeout_ignores_missing_workspace_or_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());

        application.apply_idle_timeout("missing");
        *state.workspace.lock().unwrap() = None;
        application.apply_idle_timeout("missing");
    }

    #[test]
    fn opening_a_workspace_returns_its_application_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state);

        let snapshot = application
            .open_workspace(root.path().to_path_buf())
            .unwrap();

        assert_eq!(snapshot.path, root.path().to_string_lossy());
    }

    #[test]
    fn opening_a_workspace_publishes_a_new_revision_when_pruning_stale_harnesses() {
        let home = tempfile::tempdir().unwrap();
        let _home = crate::test_support::FakeHome::set(home.path());
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        SessionApplication::new(state.clone())
            .open_workspace(root.path().to_path_buf())
            .unwrap();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_workspace_memory(&metadata::WorkspaceMemory {
                last_active_session_id: None,
                last_active_at: None,
                active_harness_ids: vec!["retired-custom".into(), "gemini".into()],
                active_harness_revision: 7,
            });

        let snapshot = SessionApplication::new(state.clone())
            .open_workspace(root.path().to_path_buf())
            .unwrap();

        assert!(snapshot.active_harness_ids.is_empty());
        assert_eq!(snapshot.active_harness_revision, 8);
        let memory = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_workspace_memory()
            .unwrap();
        assert!(memory.active_harness_ids.is_empty());
        assert_eq!(memory.active_harness_revision, 8);
    }

    #[test]
    fn complete_session_ending_application_projects_pending_status_and_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "complete-ending-application";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Ending",
            &root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.lifecycle = "stopping".into();
        metadata.lifecycle_phase = "ending".into();
        metadata.pending_terminal_status = Some("killed".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let mut handle = attention_test_handle(id, root.path());
        handle.info.lifecycle_phase = "ending".into();
        handle.resume_in_progress = true;
        let generation = handle.runtime.run_generation();
        state.sessions.lock().unwrap().insert(id.into(), handle);

        let snapshot = metadata::ObservedStatusSnapshotMetadata {
            value: Some("blocked".into()),
            source: "peon".into(),
            confidence: Some(0.91),
            observed_at: Some("after".into()),
        };
        assert!(
            SessionApplication::new(state.clone()).complete_session_ending(
                id,
                generation,
                snapshot.clone(),
                "error",
            )
        );

        let live = state.sessions.lock().unwrap()[id].info.clone();
        assert_eq!(live.status, "killed");
        assert_eq!(live.lifecycle_phase, "ended");
        assert_eq!(live.connectivity.as_deref(), Some("offline"));
        assert_eq!(live.final_observed_status.as_deref(), Some("blocked"));
        assert!(!state.sessions.lock().unwrap()[id].resume_in_progress);
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.pending_terminal_status, None);
        assert_eq!(stored.final_observed_status_snapshot, Some(snapshot));
    }

    #[test]
    fn complete_session_ending_application_uses_fallback_without_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "complete-ending-no-workspace";
        let mut handle = attention_test_handle(id, root.path());
        handle.info.lifecycle_phase = "ending".into();
        let generation = handle.runtime.run_generation();
        state.sessions.lock().unwrap().insert(id.into(), handle);
        *state.workspace.lock().unwrap() = None;

        assert!(
            SessionApplication::new(state.clone()).complete_session_ending(
                id,
                generation,
                metadata::canonical_null_snapshot("recovery", None),
                "error",
            )
        );

        let live = state.sessions.lock().unwrap()[id].info.clone();
        assert_eq!(live.status, "error");
        assert_eq!(live.lifecycle_phase, "ended");
        assert_eq!(live.terminal_outcome.as_deref(), Some("error"));
    }

    #[test]
    fn commit_accepted_input_updates_detached_live_session_without_persisted_state() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "commit-input-detached";
        let mut handle = attention_test_handle(id, root.path());
        handle.info.attention = Some("needs_you".into());
        handle.info.observed_status = Some("waiting_for_input".into());
        handle.info.needs_user_input = Some(true);
        let prior_generation = handle.runtime.input_generation;
        state.sessions.lock().unwrap().insert(id.into(), handle);
        *state.workspace.lock().unwrap() = None;

        SessionApplication::new(state.clone()).commit_accepted_input(id, Some(7), true);

        let sessions = state.sessions.lock().unwrap();
        let live = &sessions[id];
        assert_eq!(live.info.attention.as_deref(), Some("working"));
        assert_eq!(live.info.observed_status.as_deref(), Some("working"));
        assert_eq!(live.runtime.input_generation, prior_generation + 1);
        assert_eq!(live.runtime.min_peon_output_revision, 7);
        assert!(live.runtime.accepted_input_at.is_some());
    }

    #[test]
    fn commit_accepted_input_advances_partial_frame_without_working_transition() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "commit-input-partial";
        let mut handle = attention_test_handle(id, root.path());
        handle.info.attention = Some("needs_you".into());
        handle.info.observed_status = Some("waiting_for_input".into());
        handle.info.needs_user_input = Some(true);
        state.sessions.lock().unwrap().insert(id.into(), handle);
        *state.workspace.lock().unwrap() = None;

        SessionApplication::new(state.clone()).commit_accepted_input(id, Some(3), false);

        let sessions = state.sessions.lock().unwrap();
        let live = &sessions[id];
        assert_eq!(live.info.attention.as_deref(), Some("needs_you"));
        assert_eq!(
            live.info.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(live.runtime.input_generation, 1);
        assert_eq!(live.runtime.min_peon_output_revision, 3);
        assert!(live.runtime.accepted_input_at.is_some());
    }

    #[test]
    fn commit_accepted_input_ignores_generation_overflow() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "commit-input-overflow";
        let mut handle = attention_test_handle(id, root.path());
        handle.runtime.input_generation = u64::MAX;
        state.sessions.lock().unwrap().insert(id.into(), handle);
        *state.workspace.lock().unwrap() = None;

        SessionApplication::new(state.clone()).commit_accepted_input(id, Some(3), true);

        let sessions = state.sessions.lock().unwrap();
        let live = &sessions[id];
        assert_eq!(live.runtime.input_generation, u64::MAX);
        assert!(live.runtime.accepted_input_at.is_none());
    }

    #[test]
    fn resume_handle_conflicts_for_metadata_pid_attachment_and_claim() {
        let dir = tempfile::tempdir().unwrap();
        let mut handle = attention_test_handle("resume-stale-predicate", dir.path());
        handle.info.lifecycle_phase = "active".into();
        let mut session_pids = std::collections::HashMap::new();

        assert!(resume_handle_conflicts(
            &handle,
            false,
            session_pids.contains_key("resume-stale-predicate"),
        ));
        session_pids.insert("resume-stale-predicate".to_string(), 42);
        assert!(resume_handle_conflicts(
            &handle,
            false,
            session_pids.contains_key("resume-stale-predicate"),
        ));
        assert!(resume_handle_conflicts(&handle, true, true));
        handle.terminal_attached = true;
        assert!(resume_handle_conflicts(&handle, true, false));
        handle.terminal_attached = false;
        assert!(!resume_handle_conflicts(&handle, true, false));
        handle.resume_in_progress = true;
        assert!(resume_handle_conflicts(&handle, true, false));
    }

    fn harness_session_report(id: &str, confidence: f64) -> metadata::HarnessSessionReport {
        metadata::HarnessSessionReport {
            harness_session_id: format!("native-{id}"),
            source: "test".into(),
            confidence,
        }
    }

    #[test]
    fn harness_session_report_application_accepts_and_persists_report() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "harness-report-accepted";
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&crate::test_support::test_session_metadata(
                id,
                "Harness report",
                root.path().display().to_string(),
                "running",
                "before",
                "before",
            ));

        let result = SessionApplication::new(state.clone())
            .report_harness_session(id, harness_session_report(id, 0.9))
            .unwrap();

        assert_eq!(result, metadata::HarnessSessionMergeResult::Accepted);
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(
            stored
                .resume
                .as_ref()
                .and_then(|resume| resume.harness_session_id.as_deref()),
            Some("native-harness-report-accepted")
        );
    }

    #[test]
    fn harness_session_report_application_ignores_lower_confidence() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "harness-report-lower";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Harness report",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.resume = Some(harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::Exact,
            harness_session_id: Some("native-existing".into()),
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        });
        metadata.harness_session_id_confidence = Some(0.9);
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let result = SessionApplication::new(state)
            .report_harness_session(id, harness_session_report(id, 0.2))
            .unwrap();

        assert_eq!(
            result,
            metadata::HarnessSessionMergeResult::IgnoredLowerConfidence
        );
    }

    #[test]
    fn harness_session_report_application_distinguishes_invalid_and_missing() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());

        assert_eq!(
            application
                .report_harness_session(
                    "missing",
                    metadata::HarnessSessionReport {
                        harness_session_id: "bad id".into(),
                        source: "test".into(),
                        confidence: 0.9,
                    },
                )
                .unwrap(),
            metadata::HarnessSessionMergeResult::Invalid
        );
        assert_eq!(
            application
                .report_harness_session("missing", harness_session_report("missing", 0.9))
                .unwrap(),
            metadata::HarnessSessionMergeResult::NotFound
        );
    }

    #[test]
    fn harness_session_report_application_conflicts_without_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        *state.workspace.lock().unwrap() = None;

        assert_eq!(
            SessionApplication::new(state)
                .report_harness_session("missing", harness_session_report("missing", 0.9)),
            Err(SessionError::Conflict)
        );
    }

    #[test]
    fn harness_session_report_application_updates_live_resume_projection() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "harness-report-live";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Harness report",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.resume = Some(harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        });
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let mut handle = attention_test_handle(id, root.path());
        handle.info.resume = metadata.resume.clone();
        state.sessions.lock().unwrap().insert(id.into(), handle);

        let result = SessionApplication::new(state.clone())
            .report_harness_session(id, harness_session_report(id, 0.9))
            .unwrap();

        assert_eq!(result, metadata::HarnessSessionMergeResult::Accepted);
        assert_eq!(
            state.sessions.lock().unwrap()[id]
                .info
                .resume
                .as_ref()
                .and_then(|resume| resume.harness_session_id.as_deref()),
            Some("native-harness-report-live")
        );
    }

    #[test]
    fn report_plan_path_application_preserves_selection_and_validates_session_state() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());

        assert_eq!(
            application.report_plan_path("missing", root.path().join("plan.md").to_str().unwrap()),
            Err(SessionError::NotFound)
        );

        let mut dead = crate::test_support::test_session_metadata(
            "dead-plan",
            "Dead plan",
            root.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        dead.lifecycle = "ended".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&dead);
        assert_eq!(
            application
                .report_plan_path("dead-plan", root.path().join("plan.md").to_str().unwrap()),
            Err(SessionError::Conflict)
        );

        let mut invalid = crate::test_support::test_session_metadata(
            "invalid-plan",
            "Invalid plan",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        invalid.lifecycle = "alive".into();
        invalid.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&invalid);
        assert_eq!(
            application.report_plan_path("invalid-plan", "../outside.md"),
            Err(SessionError::EmptyBadRequest)
        );

        let mut selected = crate::test_support::test_session_metadata(
            "selected-plan",
            "Selected plan",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        selected.lifecycle = "alive".into();
        selected.lifecycle_phase = "active".into();
        selected.plan_path = Some(metadata::PlanReference {
            worktree_root: Some(root.path().display().to_string()),
            relative_path: "user-plan.md".into(),
            source: metadata::PlanSource::UserSelected,
        });
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&selected);
        application
            .report_plan_path(
                "selected-plan",
                root.path().join("hook-plan.md").to_str().unwrap(),
            )
            .unwrap();
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session("selected-plan")
                .unwrap()
                .plan_path
                .unwrap()
                .relative_path,
            "user-plan.md"
        );

        *state.workspace.lock().unwrap() = None;
        assert_eq!(
            application.report_plan_path("missing", "plan.md"),
            Err(SessionError::Conflict)
        );
    }

    #[test]
    fn printed_plan_fallback_persists_valid_path_and_preserves_existing_plan() {
        let root = tempfile::tempdir().unwrap();
        git2::Repository::init(root.path()).unwrap();
        let plan = root.path().join("docs/superpowers/plans/fallback.md");
        std::fs::create_dir_all(plan.parent().unwrap()).unwrap();
        std::fs::write(&plan, "# fallback\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());
        let id = "printed-fallback";
        let mut session = crate::test_support::test_session_metadata(
            id,
            "Fallback",
            root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        session.lifecycle = "alive".into();
        session.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&session);

        assert!(application.persist_printed_plan_fallback(id, "docs/superpowers/plans/fallback.md"));
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        let reference = stored.plan_path.unwrap();
        assert_eq!(
            reference.relative_path,
            "docs/superpowers/plans/fallback.md"
        );
        assert_eq!(
            reference.worktree_root,
            Some(root.path().display().to_string())
        );
        assert_eq!(reference.source, metadata::PlanSource::TerminalFallback);
        assert!(
            !application.persist_printed_plan_fallback(id, "docs/superpowers/plans/fallback.md")
        );
    }

    #[test]
    fn printed_plan_fallback_protects_explicit_clear_and_ignores_invalid_state() {
        let root = tempfile::tempdir().unwrap();
        git2::Repository::init(root.path()).unwrap();
        let plan = root.path().join("specs/fallback.md");
        std::fs::create_dir_all(plan.parent().unwrap()).unwrap();
        std::fs::write(&plan, "# fallback\n").unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_plan = outside.path().join("outside.md");
        std::fs::write(&outside_plan, "# outside\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());
        let id = "printed-clear";
        let mut session = crate::test_support::test_session_metadata(
            id,
            "Fallback",
            root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        session.lifecycle = "alive".into();
        session.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&session);

        assert!(!application.persist_printed_plan_fallback(id, "specs/missing.md"));
        assert!(!application.persist_printed_plan_fallback(id, outside_plan.to_str().unwrap()));
        let result = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .merge_agent_attention_signal_with_plan(
                id,
                "waiting_for_input",
                None,
                &metadata::PlanPathUpdate::Clear,
                "now",
                "agent",
                1.0,
            );
        assert_eq!(result, metadata::AttentionMergeResult::Accepted);
        assert!(!application.persist_printed_plan_fallback(id, "specs/fallback.md"));

        *state.workspace.lock().unwrap() = None;
        assert!(!application.persist_printed_plan_fallback(id, "specs/fallback.md"));
    }

    #[test]
    fn append_terminal_output_batch_persists_records_in_order() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());
        let id = "terminal-output-application";
        let records = vec![
            crate::metadata::TerminalOutputRecord::raw("first", "\r\n"),
            crate::metadata::TerminalOutputRecord::raw("second", "\n"),
        ];

        application.append_terminal_output_batch(id, &records);

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_terminal_output(id, 10);
        assert_eq!(stored, records);
    }

    #[test]
    fn append_terminal_output_batch_is_a_noop_without_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        *state.workspace.lock().unwrap() = None;

        SessionApplication::new(state).append_terminal_output_batch(
            "terminal-output-missing-workspace",
            &[crate::metadata::TerminalOutputRecord::raw("ignored", "\n")],
        );

        assert!(!root
            .path()
            .join(".orkworks/events/terminal-output-missing-workspace.terminal")
            .exists());
    }

    #[test]
    fn terminal_metadata_queries_return_output_size_and_empty_missing_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "terminal-query-application";
        let records = vec![crate::metadata::TerminalOutputRecord::raw("line", "\r\n")];
        {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            workspace
                .metadata
                .append_terminal_output_records(id, &records);
            workspace.metadata.write_terminal_size(id, 120, 40);
        }

        let application = SessionApplication::new(state.clone());
        let query = application.get_terminal_output(id);
        assert_eq!(query.lines, records);
        assert_eq!(query.size, Some((120, 40)));

        *state.workspace.lock().unwrap() = None;
        let query = application.get_terminal_output(id);
        assert!(query.lines.is_empty());
        assert_eq!(query.size, None);
    }

    #[test]
    fn summary_log_query_filters_incomplete_events_and_orphan_sessions() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "summary-query-application";
        {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            workspace
                .metadata
                .write_session(&crate::test_support::test_session_metadata(
                    id,
                    "Summary query",
                    root.path().display().to_string(),
                    "ended",
                    "t0",
                    "t0",
                ));
            for event in [
                metadata::Event {
                    event_type: "status".into(),
                    timestamp: "t0".into(),
                    status: "ended".into(),
                    observed_status: None,
                    confidence: None,
                    summary: None,
                    source: None,
                },
                metadata::Event {
                    event_type: "checkpoint".into(),
                    timestamp: "t1".into(),
                    status: "working".into(),
                    observed_status: Some("working".into()),
                    confidence: Some(0.9),
                    summary: Some("Checkpoint".into()),
                    source: Some("peon".into()),
                },
                metadata::Event {
                    event_type: "checkpoint".into(),
                    timestamp: "t2".into(),
                    status: "working".into(),
                    observed_status: Some("working".into()),
                    confidence: None,
                    summary: Some("No source".into()),
                    source: None,
                },
            ] {
                workspace.metadata.append_event(id, &event);
            }
        }

        let application = SessionApplication::new(state.clone());
        let entries = application.get_summary_log(id);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].timestamp, "t1");
        assert_eq!(entries[0].summary, "Checkpoint");
        assert_eq!(entries[0].source, "peon");
        assert_eq!(entries[0].confidence, Some(0.9));
        assert!(application.get_summary_log("orphan").is_empty());

        *state.workspace.lock().unwrap() = None;
        assert!(application.get_summary_log(id).is_empty());
    }

    #[test]
    fn trim_terminal_output_keeps_recent_records_in_active_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "terminal-output-trim-application";
        let output_path = root
            .path()
            .join(".orkworks-test/events/terminal-output-trim-application.terminal");
        std::fs::create_dir_all(output_path.parent().unwrap()).unwrap();
        let content = (0..=crate::metadata::TERMINAL_OUTPUT_MAX_LINES)
            .map(|index| format!("line {index}\n"))
            .collect::<String>();
        std::fs::write(&output_path, content).unwrap();

        SessionApplication::new(state.clone()).trim_terminal_output(id);

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_terminal_output(id, crate::metadata::TERMINAL_OUTPUT_MAX_LINES + 1);
        assert_eq!(stored.len(), crate::metadata::TERMINAL_OUTPUT_MAX_LINES);
        assert_eq!(stored.first().unwrap().text(), "line 1");
        assert_eq!(stored.last().unwrap().text(), "line 1000");
    }

    #[test]
    fn trim_terminal_output_is_a_noop_without_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        *state.workspace.lock().unwrap() = None;

        SessionApplication::new(state).trim_terminal_output("terminal-output-trim-missing");

        assert!(!root
            .path()
            .join(".orkworks-test/events/terminal-output-trim-missing.terminal")
            .exists());
    }

    #[test]
    fn persist_process_transition_updates_metadata_and_clears_prompt_fields() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "process-transition-application";
        let mut session = crate::test_support::test_session_metadata(
            id,
            "Process transition",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        session.lifecycle = "alive".into();
        session.lifecycle_phase = "active".into();
        session.connectivity = "online".into();
        session.terminal_outcome = None;
        session.needs_user_input = Some(true);
        session.detected_question = Some("continue?".into());
        session.suggested_options = Some(vec!["yes".into()]);
        session.metadata_source = "agent".into();
        session.metadata_confidence = 0.7;
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&session);

        assert!(
            SessionApplication::new(state.clone()).persist_process_transition(
                id,
                crate::runtime::observed_status::ProcessTransition::CommittedWorking,
            )
        );

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("working"));
        assert_eq!(stored.attention.as_deref(), Some("working"));
        assert_eq!(stored.metadata_source, "process");
        assert_eq!(stored.metadata_confidence, 1.0);
        assert!(stored.needs_user_input.is_none());
        assert!(stored.detected_question.is_none());
        assert!(stored.suggested_options.is_none());
    }

    #[test]
    fn persist_process_transition_is_a_safe_noop_without_workspace_or_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());
        let transition = crate::runtime::observed_status::ProcessTransition::CommittedWorking;

        assert!(!application.persist_process_transition("missing", transition));
        *state.workspace.lock().unwrap() = None;
        assert!(!application.persist_process_transition("missing", transition));
    }

    #[test]
    fn read_plan_content_application_returns_persisted_markdown() {
        let root = tempfile::tempdir().unwrap();
        let plan_dir = root.path().join("docs/superpowers/plans");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::write(plan_dir.join("plan.md"), "# persisted plan\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let mut session = crate::test_support::test_session_metadata(
            "plan-content",
            "Plan content",
            root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        session.plan_path = Some("docs/superpowers/plans/plan.md".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&session);

        assert_eq!(
            SessionApplication::new(state).read_plan_content("plan-content"),
            Ok("# persisted plan\n".into())
        );
    }

    #[test]
    fn read_plan_content_application_maps_lookup_and_file_failures_to_conflict_or_not_found() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());

        assert_eq!(
            application.read_plan_content("missing"),
            Err(SessionError::NotFound)
        );

        let mut no_plan = crate::test_support::test_session_metadata(
            "no-plan",
            "No plan",
            root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        no_plan.plan_path = None;
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&no_plan);
        assert_eq!(
            application.read_plan_content("no-plan"),
            Err(SessionError::Conflict)
        );

        let mut invalid = crate::test_support::test_session_metadata(
            "invalid-plan",
            "Invalid plan",
            root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        invalid.plan_path = Some("../outside.md".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&invalid);
        assert_eq!(
            application.read_plan_content("invalid-plan"),
            Err(SessionError::Conflict)
        );

        let mut unreadable = crate::test_support::test_session_metadata(
            "unreadable-plan",
            "Unreadable plan",
            root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        unreadable.plan_path = Some("docs/superpowers/plans/missing.md".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&unreadable);
        assert_eq!(
            application.read_plan_content("unreadable-plan"),
            Err(SessionError::Conflict)
        );

        *state.workspace.lock().unwrap() = None;
        assert_eq!(
            application.read_plan_content("no-plan"),
            Err(SessionError::Conflict)
        );
    }

    #[test]
    fn report_plan_path_application_normalizes_and_appends_agent_event_without_attention_change() {
        let root = tempfile::tempdir().unwrap();
        let plan_dir = root.path().join("docs/superpowers/plans");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("plan.md");
        std::fs::write(&plan, "# plan").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "hook-plan";
        let mut session = crate::test_support::test_session_metadata(
            id,
            "Hook plan",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        session.lifecycle = "alive".into();
        session.lifecycle_phase = "active".into();
        session.attention = Some("working".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&session);

        SessionApplication::new(state.clone())
            .report_plan_path(id, plan.to_str().unwrap())
            .unwrap();

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        let reference = stored.plan_path.unwrap();
        assert_eq!(reference.relative_path, "docs/superpowers/plans/plan.md");
        assert_eq!(reference.source, metadata::PlanSource::HookReported);
        assert_eq!(stored.attention.as_deref(), Some("working"));
        let event = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events(id)
            .into_iter()
            .find(|event| event.event_type == "session.plan_path_hooked")
            .unwrap();
        assert_eq!(event.source.as_deref(), Some("agent"));
        assert_eq!(event.confidence, Some(1.0));
    }

    #[test]
    fn report_plan_path_application_does_not_append_event_when_session_write_fails() {
        let root = tempfile::tempdir().unwrap();
        let plan_dir = root.path().join("docs/superpowers/plans");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("plan.md");
        std::fs::write(&plan, "# plan").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "write-fails-plan";
        let mut session = crate::test_support::test_session_metadata(
            id,
            "Write fails",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        session.lifecycle = "alive".into();
        session.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&session);
        let sessions_path = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .sessions_dir();
        std::fs::create_dir_all(sessions_path.join(format!("{id}.json.tmp"))).unwrap();

        assert_eq!(
            SessionApplication::new(state.clone()).report_plan_path(id, plan.to_str().unwrap()),
            Err(SessionError::Internal("application operation failed"))
        );
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events(id)
            .into_iter()
            .all(|event| event.event_type != "session.plan_path_hooked"));
    }

    #[tokio::test]
    async fn create_returns_pre_spawn_info_after_persisting_creating_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());

        let info = application
            .create_session(CreateSessionCommand {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            })
            .await
            .unwrap();

        assert_eq!(info.status, "creating");
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&info.id)
            .is_some());
    }

    #[tokio::test]
    async fn resume_without_a_workspace_returns_conflict() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        *state.workspace.lock().unwrap() = None;
        let application = SessionApplication::new(state);

        assert!(matches!(
            application.resume_session("missing").await,
            Err(SessionError::Conflict)
        ));
    }

    #[tokio::test]
    async fn resume_admission_conflict_is_returned_through_the_application_seam() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "resume-admission-conflict";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Resume conflict",
            root.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.cwd = root.path().display().to_string();
        metadata.harness = "opencode".into();
        metadata.resume = Some(harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        });
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = crate::test_support::test_session_info(
            id,
            "Resume conflict",
            root.path().display().to_string(),
            "running",
            "before",
        );
        info.lifecycle_phase = "active".into();
        let handle = SessionHandle {
            info,
            active_work_hook: false,
            kill_tx,
            output_buffer: peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
            ),
            terminal_attached: false,
            resume_in_progress: true,
            at_usage_limit_latched: false,
            capacity_check_pending: false,
            output_lines_seen: 0,
            scan_bytes_seen: 0,
            resume_scan_origin: None,
            pending_capacity_visible_once: false,
        };
        state.sessions.lock().unwrap().insert(id.into(), handle);

        let result = SessionApplication::new(state).resume_session(id).await;

        assert!(matches!(result, Err(SessionError::Conflict)));
    }

    #[tokio::test]
    async fn resume_startup_failure_restores_claim_state_and_publishes_error_metadata() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                let override_patch = document.overrides.entry("opencode".into()).or_default();
                override_patch.resume = Some(Some(harness::definition::ResumePatch {
                    latest_cwd: Some(Some(harness::CommandTemplate {
                        command: "orkworks-resume-command-that-does-not-exist".into(),
                        args: vec![],
                    })),
                    ..Default::default()
                }));
                Ok(())
            })
            .unwrap();
        let id = "resume-startup-failure";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Resume startup failure",
            root.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.cwd = root.path().display().to_string();
        metadata.harness = "opencode".into();
        metadata.resume = Some(harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        });
        let ws = state.workspace.lock().unwrap();
        ws.as_ref().unwrap().metadata.write_session(&metadata);
        ws.as_ref()
            .unwrap()
            .metadata
            .write_terminal_size(id, 120, 40);
        drop(ws);

        let result = SessionApplication::new(state.clone())
            .resume_session(id)
            .await;

        assert!(matches!(
            result,
            Err(SessionError::Internal("application operation failed"))
        ));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(id)
                    .is_some_and(|handle| !handle.resume_in_progress)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("startup-failure finalization clears the runtime claim");
        let handle = state.sessions.lock().unwrap().get(id).unwrap().info.clone();
        assert_eq!(handle.status, "error");
        assert_eq!(handle.lifecycle_phase, "ended");
        let ws = state.workspace.lock().unwrap();
        let stored = ws.as_ref().unwrap().metadata.read_session(id).unwrap();
        assert_eq!(stored.status, "error");
        assert_eq!(stored.lifecycle_phase, "ended");
        assert_ne!(
            ws.as_ref().unwrap().metadata.read_terminal_size(id),
            Some((120, 40))
        );
    }

    #[tokio::test]
    async fn report_attention_application_seam_persists_an_accepted_signal() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "attention-application";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Attention",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);

        SessionApplication::new(state.clone())
            .report_attention(
                id,
                AttentionSignal {
                    status: "waiting_for_input".into(),
                    message: Some("question".into()),
                    plan_path: metadata::PlanPathUpdate::Unchanged,
                    observed_at: None,
                    cwd: None,
                },
            )
            .await
            .unwrap();

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("waiting_for_input"));
        assert_eq!(stored.attention.as_deref(), Some("needs_you"));
    }

    #[tokio::test]
    async fn report_attention_application_seam_rejects_invalid_status() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let result = SessionApplication::new(state)
            .report_attention(
                "missing",
                AttentionSignal {
                    status: "invalid".into(),
                    message: None,
                    plan_path: metadata::PlanPathUpdate::Unchanged,
                    observed_at: None,
                    cwd: None,
                },
            )
            .await;
        assert!(matches!(result, Err(SessionError::EmptyBadRequest)));
    }

    #[test]
    fn attention_merge_application_mirrors_accepted_hook_and_clears_pending_work() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "attention-merge-hook";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Attention",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let mut handle = attention_test_handle(id, root.path());
        handle.pending_work_signal =
            Some(crate::runtime::session_runtime::arm_pending_work_signal(
                "y",
                tokio::time::Instant::now(),
            ));
        state.sessions.lock().unwrap().insert(id.into(), handle);

        let result =
            SessionApplication::new(state.clone()).apply_attention_signal(AttentionMergeSignal {
                session_id: id.into(),
                observed_status: "waiting_for_input".into(),
                message: Some("question".into()),
                plan_path: metadata::PlanPathUpdate::Unchanged,
                timestamp: "2026-01-01T00:00:00Z".into(),
                source: "agent".into(),
                confidence: 1.0,
                observed_at: Some(parse_hook_observed_at("2026-01-01T00:00:01.000000Z").unwrap()),
                reject_stale_observed_at: true,
                update_hook_timestamp: true,
                clear_pending_work_signal: true,
                require_alive: false,
                debug_hint_mutation: None,
            });

        assert_eq!(result, Ok(metadata::AttentionMergeResult::Accepted));
        let sessions = state.sessions.lock().unwrap();
        let live = sessions.get(id).unwrap();
        assert!(live.pending_work_signal.is_none());
        assert_eq!(live.info.attention.as_deref(), Some("needs_you"));
        assert_eq!(
            live.runtime.last_hook_attention_at.unwrap().to_rfc3339(),
            "2026-01-01T00:00:01+00:00"
        );
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session(id)
                .unwrap()
                .attention
                .as_deref(),
            Some("needs_you")
        );
    }

    #[test]
    fn attention_merge_application_ignores_stale_hook_without_mutating_either_store() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "attention-merge-stale";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Attention",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let mut handle = attention_test_handle(id, root.path());
        handle.runtime.last_hook_attention_at =
            Some(parse_hook_observed_at("2026-01-01T00:00:02.000000Z").unwrap());
        state.sessions.lock().unwrap().insert(id.into(), handle);

        let result =
            SessionApplication::new(state.clone()).apply_attention_signal(AttentionMergeSignal {
                session_id: id.into(),
                observed_status: "working".into(),
                message: Some("stale".into()),
                plan_path: metadata::PlanPathUpdate::Unchanged,
                timestamp: "2026-01-01T00:00:00Z".into(),
                source: "agent".into(),
                confidence: 1.0,
                observed_at: Some(parse_hook_observed_at("2026-01-01T00:00:01.000000Z").unwrap()),
                reject_stale_observed_at: true,
                update_hook_timestamp: true,
                clear_pending_work_signal: true,
                require_alive: false,
                debug_hint_mutation: None,
            });

        assert_eq!(result, Ok(metadata::AttentionMergeResult::Ignored));
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap()
            .summary
            .is_none());
        assert!(state
            .sessions
            .lock()
            .unwrap()
            .get(id)
            .unwrap()
            .info
            .summary
            .is_none());
    }

    #[test]
    fn attention_merge_application_debug_options_preserve_hook_bookkeeping() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "attention-merge-debug";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Attention",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let mut handle = attention_test_handle(id, root.path());
        handle.pending_work_signal =
            Some(crate::runtime::session_runtime::arm_pending_work_signal(
                "y",
                tokio::time::Instant::now(),
            ));
        state.sessions.lock().unwrap().insert(id.into(), handle);

        let result =
            SessionApplication::new(state.clone()).apply_attention_signal(AttentionMergeSignal {
                session_id: id.into(),
                observed_status: "waiting_for_input".into(),
                message: None,
                plan_path: metadata::PlanPathUpdate::Unchanged,
                timestamp: "2026-01-01T00:00:00Z".into(),
                source: "debug".into(),
                confidence: 0.0,
                observed_at: None,
                reject_stale_observed_at: false,
                update_hook_timestamp: false,
                clear_pending_work_signal: false,
                require_alive: true,
                debug_hint_mutation: Some(DebugHintMutation::Preserve),
            });

        assert_eq!(result, Ok(metadata::AttentionMergeResult::Accepted));
        let sessions = state.sessions.lock().unwrap();
        let live = sessions.get(id).unwrap();
        assert!(live.pending_work_signal.is_some());
        assert!(live.runtime.last_hook_attention_at.is_none());
        assert_eq!(live.info.metadata_source.as_deref(), Some("debug"));
    }

    #[test]
    fn attention_merge_application_keeps_live_and_persisted_winners_together() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "attention-merge-concurrent";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Attention",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, root.path()));

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let calls = [("waiting_for_input", "A"), ("blocked", "B")]
            .into_iter()
            .map(|(status, message)| {
                let state = state.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    SessionApplication::new(state).apply_attention_signal(AttentionMergeSignal {
                        session_id: id.into(),
                        observed_status: status.into(),
                        message: Some(message.into()),
                        plan_path: metadata::PlanPathUpdate::Unchanged,
                        timestamp: "2026-01-01T00:00:00Z".into(),
                        source: "agent".into(),
                        confidence: 1.0,
                        observed_at: None,
                        reject_stale_observed_at: false,
                        update_hook_timestamp: false,
                        clear_pending_work_signal: true,
                        require_alive: false,
                        debug_hint_mutation: None,
                    })
                })
            })
            .collect::<Vec<_>>();
        for call in calls {
            assert_eq!(
                call.join().unwrap(),
                Ok(metadata::AttentionMergeResult::Accepted)
            );
        }

        let persisted = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        let sessions = state.sessions.lock().unwrap();
        let live = &sessions.get(id).unwrap().info;
        assert_eq!(
            live.observed_status.as_deref(),
            persisted.observed_status.as_deref()
        );
        assert_eq!(live.attention.as_deref(), persisted.attention.as_deref());
        assert_eq!(live.summary.as_deref(), persisted.summary.as_deref());
    }

    #[test]
    fn persist_terminal_size_writes_authoritative_size_during_ending() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "terminal-size-application".to_string();
        let mut info = crate::test_support::test_session_info(
            id.clone(),
            "Terminal size",
            root.path().display().to_string(),
            "ended",
            "before",
        );
        info.lifecycle_phase = "ending".into();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            id.clone(),
            SessionHandle {
                info,
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(40, 120),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        SessionApplication::new(state.clone()).persist_terminal_size(&id, true);

        let workspace = state.workspace.lock().unwrap();
        assert_eq!(
            workspace.as_ref().unwrap().metadata.read_terminal_size(&id),
            Some((120, 40))
        );
    }

    #[test]
    fn persist_output_recency_applies_monotonic_timestamps() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "output-recency-application";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Output recency",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.last_output_at = Some("2026-07-29T10:00:00Z".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let application = SessionApplication::new(state.clone());

        application.persist_output_recency(id, "2026-07-29T10:00:01Z".into());
        application.persist_output_recency(id, "2026-07-29T10:00:01Z".into());
        application.persist_output_recency(id, "2026-07-29T09:59:59Z".into());

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(
            stored.last_output_at.as_deref(),
            Some("2026-07-29T10:00:01Z")
        );
    }

    #[test]
    fn persist_output_recency_writes_new_timestamp_without_existing_value() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "output-recency-new";
        let metadata = crate::test_support::test_session_metadata(
            id,
            "Output recency",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        SessionApplication::new(state.clone())
            .persist_output_recency(id, "2026-07-29T10:00:00Z".into());

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(
            stored.last_output_at.as_deref(),
            Some("2026-07-29T10:00:00Z")
        );
    }

    #[test]
    fn persist_output_recency_replaces_malformed_stored_timestamp() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "output-recency-malformed-stored";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Output recency",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.last_output_at = Some("not-a-timestamp".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        SessionApplication::new(state.clone())
            .persist_output_recency(id, "2026-07-29T10:00:00Z".into());

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(
            stored.last_output_at.as_deref(),
            Some("2026-07-29T10:00:00Z")
        );
    }

    #[test]
    fn persist_output_recency_ignores_malformed_incoming_timestamp() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "output-recency-malformed-incoming";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Output recency",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.last_output_at = Some("2026-07-29T10:00:00Z".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        SessionApplication::new(state.clone()).persist_output_recency(id, "not-a-timestamp".into());

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(
            stored.last_output_at.as_deref(),
            Some("2026-07-29T10:00:00Z")
        );
    }

    #[test]
    fn persist_output_recency_ignores_missing_workspace_and_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());

        application.persist_output_recency("missing", "2026-07-29T10:00:00Z".into());
        *state.workspace.lock().unwrap() = None;
        application.persist_output_recency("missing", "2026-07-29T10:00:01Z".into());
    }

    #[tokio::test]
    async fn report_attention_application_seam_ignores_stale_signal() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "attention-stale-application";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Attention",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        meta.observed_status = Some("working".into());
        meta.attention = Some("working".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let mut handle = attention_test_handle(id, root.path());
        handle.runtime.accepted_input_at =
            Some(parse_hook_observed_at("2026-08-22T08:00:01.000000Z").unwrap());
        state.sessions.lock().unwrap().insert(id.into(), handle);

        SessionApplication::new(state.clone())
            .report_attention(
                id,
                AttentionSignal {
                    status: "waiting_for_input".into(),
                    message: Some("old".into()),
                    plan_path: metadata::PlanPathUpdate::Unchanged,
                    observed_at: Some("2026-08-22T08:00:00.000000Z".into()),
                    cwd: Some("/stale".into()),
                },
            )
            .await
            .unwrap();

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("working"));
        assert_eq!(state.peon.reported_cwd.read().unwrap().get(id), None);
    }

    #[tokio::test]
    async fn report_attention_application_seam_returns_persistence_failure() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "attention-persist-failure";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Attention",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.lifecycle = "alive".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        std::fs::create_dir_all(
            root.path()
                .join(".orkworks-test/sessions/attention-persist-failure.json.tmp"),
        )
        .unwrap();

        let result = SessionApplication::new(state)
            .report_attention(
                id,
                AttentionSignal {
                    status: "waiting_for_input".into(),
                    message: Some("not persisted".into()),
                    plan_path: metadata::PlanPathUpdate::Unchanged,
                    observed_at: None,
                    cwd: None,
                },
            )
            .await;

        assert!(matches!(
            result,
            Err(SessionError::Internal("application operation failed"))
        ));
    }

    #[tokio::test]
    async fn select_plan_application_seam_rejects_unresolvable_path() {
        let root = tempfile::tempdir().unwrap();
        git2::Repository::init(root.path()).unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "plan-rejected-application";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Plan",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.cwd = root.path().display().to_string();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);

        let result = SessionApplication::new(state)
            .select_plan(
                id,
                PlanSelection {
                    printed_path: "../outside-plan.md".into(),
                },
            )
            .await;

        assert!(matches!(result, Err(SessionError::Conflict)));
    }

    #[tokio::test]
    async fn select_plan_application_seam_persists_user_selected_reference_and_event() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("docs/superpowers/plans")).unwrap();
        std::fs::write(root.path().join("docs/superpowers/plans/task.md"), "# task").unwrap();
        git2::Repository::init(root.path()).unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "plan-application";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Plan",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.cwd = root.path().display().to_string();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);

        SessionApplication::new(state.clone())
            .select_plan(
                id,
                PlanSelection {
                    printed_path: "docs/superpowers/plans/task.md".into(),
                },
            )
            .await
            .unwrap();

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(
            stored.plan_path.as_ref().unwrap().source,
            metadata::PlanSource::UserSelected
        );
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events(id)
            .iter()
            .any(|event| event.event_type == "session.plan_selected_by_user"));
    }

    #[tokio::test]
    async fn delete_session_application_workflow_kills_a_live_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "delete-application";
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Delete",
            root.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        meta.lifecycle = "alive".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, root.path()));

        SessionApplication::new(state.clone())
            .delete_session(id)
            .await
            .unwrap();

        assert_eq!(state.sessions.lock().unwrap()[id].info.status, "running");
        assert_eq!(
            state.sessions.lock().unwrap()[id].info.lifecycle_phase,
            "ending"
        );
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.status, "running");
        assert_eq!(stored.pending_terminal_status.as_deref(), Some("killed"));
    }

    #[tokio::test]
    async fn forget_session_application_rejects_a_live_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "forget-live-application";
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, root.path()));

        let result = SessionApplication::new(state).forget_session(id).await;

        assert_eq!(
            result,
            Err(SessionError::ConflictWithMessage(
                "Cannot forget a live session. Kill it first."
            ))
        );
    }

    #[tokio::test]
    async fn forget_session_application_deletes_metadata_events_and_last_active() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "forget-application";
        {
            let ws = state.workspace.lock().unwrap();
            let store = &ws.as_ref().unwrap().metadata;
            store.write_session(&crate::test_support::test_session_metadata(
                id,
                "Forget",
                root.path().display().to_string(),
                "ended",
                "before",
                "before",
            ));
            store.append_event(
                id,
                &metadata::Event {
                    event_type: "test.event".into(),
                    timestamp: "now".into(),
                    status: "ended".into(),
                    observed_status: None,
                    confidence: None,
                    summary: None,
                    source: None,
                },
            );
            store.write_workspace_memory(&metadata::WorkspaceMemory {
                last_active_session_id: Some(id.into()),
                last_active_at: Some("now".into()),
                active_harness_ids: vec![],
                active_harness_revision: 0,
            });
        }

        SessionApplication::new(state.clone())
            .forget_session(id)
            .await
            .unwrap();

        let ws = state.workspace.lock().unwrap();
        let store = &ws.as_ref().unwrap().metadata;
        assert!(!store.session_file_exists(id));
        assert!(store.read_events(id).is_empty());
        assert_eq!(
            store
                .read_workspace_memory()
                .unwrap()
                .last_active_session_id,
            None
        );
        assert!(!state.sessions.lock().unwrap().contains_key(id));
    }

    #[tokio::test]
    async fn forget_session_application_maps_session_deletion_failure() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "forget-failure-application";
        {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            store.write_session(&crate::test_support::test_session_metadata(
                id,
                "Forget",
                root.path().display().to_string(),
                "ended",
                "before",
                "before",
            ));
            let session_path = store.sessions_dir().join(format!("{id}.json"));
            std::fs::remove_file(&session_path).unwrap();
            std::fs::create_dir(&session_path).unwrap();
        }

        let result = SessionApplication::new(state).forget_session(id).await;

        assert_eq!(
            result,
            Err(SessionError::Internal("application operation failed"))
        );
    }

    #[test]
    fn set_active_session_application_preserves_active_harnesses() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_workspace_memory(&metadata::WorkspaceMemory {
                last_active_session_id: Some("before-session".into()),
                last_active_at: Some("before-time".into()),
                active_harness_ids: vec!["codex".into(), "claude".into()],
                active_harness_revision: 0,
            });

        SessionApplication::new(state.clone())
            .set_active_session("after-session")
            .unwrap();

        let memory = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_workspace_memory()
            .unwrap();
        assert_eq!(
            memory.last_active_session_id.as_deref(),
            Some("after-session")
        );
        assert_eq!(memory.active_harness_ids, vec!["codex", "claude"]);
        assert_ne!(memory.last_active_at.as_deref(), Some("before-time"));
        assert!(memory.last_active_at.is_some());
    }

    #[test]
    fn set_active_harnesses_application_preserves_active_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_workspace_memory(&metadata::WorkspaceMemory {
                last_active_session_id: Some("active-session".into()),
                last_active_at: Some("before-time".into()),
                active_harness_ids: vec!["before".into()],
                active_harness_revision: 0,
            });

        SessionApplication::new(state.clone())
            .set_active_harnesses(vec!["aider".into(), "claude-code".into()])
            .unwrap();

        let memory = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_workspace_memory()
            .unwrap();
        assert_eq!(
            memory.last_active_session_id.as_deref(),
            Some("active-session")
        );
        assert_eq!(memory.active_harness_ids, vec!["aider", "claude-code"]);
        assert_ne!(memory.last_active_at.as_deref(), Some("before-time"));
        assert!(memory.last_active_at.is_some());
    }

    #[test]
    fn active_harness_writes_require_the_current_revision_and_normalize_missing_ids() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());

        assert_eq!(
            application.set_active_harnesses_at(vec!["codex".into(), "deleted-custom".into()], 0),
            Err(SessionError::BadRequest(
                "The selected coding tool is no longer available."
            ))
        );
        assert_eq!(
            application.set_active_harnesses_at(vec!["gemini".into()], 0),
            Err(SessionError::BadRequest(
                "The selected coding tool is retired and cannot be enabled."
            ))
        );

        let first = application
            .set_active_harnesses_at(vec!["codex".into()], 0)
            .unwrap();
        assert_eq!(first.active_harness_revision, 1);
        assert_eq!(first.active_harness_ids, vec!["codex"]);

        assert_eq!(
            application.set_active_harnesses_at(vec!["claude-code".into()], 0),
            Err(SessionError::Conflict)
        );
        let memory = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_workspace_memory()
            .unwrap();
        assert_eq!(memory.active_harness_ids, vec!["codex"]);
        assert_eq!(memory.active_harness_revision, 1);
    }

    #[test]
    fn active_memory_commands_conflict_without_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        *state.workspace.lock().unwrap() = None;
        let application = SessionApplication::new(state);

        assert_eq!(
            application.set_active_session("session"),
            Err(SessionError::Conflict)
        );
        assert_eq!(
            application.set_active_harnesses(vec!["codex".into()]),
            Err(SessionError::Conflict)
        );
    }

    #[tokio::test]
    async fn plan_review_application_submits_prompt_and_records_event() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("specs")).unwrap();
        std::fs::write(root.path().join("specs/plan.md"), "# plan").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "plan-review-application";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Plan review",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        metadata.plan_path = Some("specs/plan.md".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let mut handle = attention_test_handle(id, root.path());
        let (runtime, mut control_rx) =
            crate::runtime::session_runtime::SessionRuntime::live(24, 80);
        handle.runtime = runtime;
        state.sessions.lock().unwrap().insert(id.into(), handle);

        let application = SessionApplication::new(state.clone());
        let mut request = tokio::spawn(async move { application.request_plan_review(id).await });
        let crate::runtime::session_runtime::RuntimeCommand::Input { data, accepted } = (tokio::select! {
            command = control_rx.recv() => command.unwrap(),
            response = &mut request => panic!("review request returned {:?} before reaching the PTY", response.unwrap()),
        }) else {
            panic!("expected terminal input");
        };
        assert_eq!(data, "Please review the plan or specification at specs/plan.md. Delegate this review to a subagent if you can; otherwise review it yourself. Check for missing requirements, risky assumptions, and unclear steps, then report the findings.\r");
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events(id)
            .is_empty());
        accepted.unwrap().send(Ok(())).unwrap();

        assert_eq!(request.await.unwrap(), Ok(()));
        let events = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events(id);
        assert!(events
            .iter()
            .any(|event| event.event_type == "plan_review_requested"));
    }

    #[tokio::test]
    async fn plan_review_application_rejects_invalid_references_and_input_delivery() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "plan-review-rejected";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Plan review",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let application = SessionApplication::new(state.clone());

        assert_eq!(
            application.request_plan_review(id).await,
            Err(SessionError::Conflict)
        );

        let plan = root.path().join("plan.md");
        std::fs::write(&plan, "# plan").unwrap();
        let mut metadata = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        metadata.plan_path = Some("plan.md".into());
        metadata.lifecycle = "ended".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        assert_eq!(
            application.request_plan_review(id).await,
            Err(SessionError::Conflict)
        );

        metadata.lifecycle = "alive".into();
        metadata.plan_path = Some("missing.md".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        assert_eq!(
            application.request_plan_review(id).await,
            Err(SessionError::Conflict)
        );

        metadata.plan_path = Some("plan.md".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, root.path()));
        assert_eq!(
            application.request_plan_review(id).await,
            Err(SessionError::Conflict)
        );
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events(id)
            .is_empty());
    }

    #[tokio::test]
    async fn debug_attention_application_maps_needs_you_and_updates_live_state() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "debug-attention-application";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Debug attention",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, root.path()));

        let application = SessionApplication::new(state.clone());
        assert_eq!(
            application
                .apply_debug_attention(
                    id,
                    DebugAttentionSignal {
                        attention: "needs_you".into(),
                        message: Some("Answer required".into()),
                    },
                )
                .await,
            Ok(metadata::AttentionMergeResult::Accepted)
        );

        let persisted = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(
            persisted.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(persisted.metadata_source, "debug");
        assert_eq!(persisted.metadata_confidence, 0.0);
        assert_eq!(
            state.sessions.lock().unwrap()[id]
                .info
                .observed_status
                .as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(
            state.sessions.lock().unwrap()[id].info.summary.as_deref(),
            Some("Answer required")
        );
    }

    #[tokio::test]
    async fn transition_session_status_application_updates_persisted_and_live_terminal_state() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "status-transition-application";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Status transition",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        metadata.observed_status = Some("blocked".into());
        metadata.metadata_source = "peon".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let mut handle = attention_test_handle(id, root.path());
        handle.info.lifecycle_phase = "active".into();
        handle.info.lifecycle = "alive".into();
        handle.info.observed_status = Some("blocked".into());
        state.sessions.lock().unwrap().insert(id.into(), handle);

        assert!(
            SessionApplication::new(state.clone())
                .transition_session_status(id, None, "ended")
                .await
        );

        let live = state.sessions.lock().unwrap()[id].info.clone();
        assert_eq!(live.status, "running");
        assert_eq!(live.lifecycle_phase, "ending");
        assert_eq!(live.lifecycle, "stopping");
        assert_eq!(live.observed_status, None);
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.lifecycle_phase, "ending");
        assert_eq!(stored.pending_terminal_status.as_deref(), Some("ended"));
        assert_eq!(
            stored
                .ending_observed_status_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.value.as_deref()),
            Some("blocked")
        );
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_events(id)
                .len(),
            1
        );
    }

    #[test]
    fn reset_session_topic_clears_queued_label_work_and_resets_both_copies() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "reset-topic-application";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Old topic",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.label = "Old topic".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let mut handle = attention_test_handle(id, root.path());
        handle.info.label = "Old topic".into();
        state.sessions.lock().unwrap().insert(id.into(), handle);
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(id.into(), 4);
        state.peon.label_hint.write().unwrap().insert(
            id.into(),
            crate::LabelHint {
                text: "stale topic".into(),
                epoch: 4,
            },
        );
        state.peon.label_pending.write().unwrap().insert(id.into());

        assert!(SessionApplication::new(state.clone()).reset_session_topic(id));

        let placeholder = crate::session_types::placeholder_label(id);
        assert_eq!(state.sessions.lock().unwrap()[id].info.label, placeholder);
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session(id)
                .unwrap()
                .label,
            placeholder
        );
        assert_eq!(state.peon.label_epochs.read().unwrap().get(id), Some(&5));
        assert!(state.peon.label_hint.read().unwrap().get(id).is_none());
        assert!(!state.peon.label_pending.read().unwrap().contains(id));
    }

    #[test]
    fn reset_session_topic_persists_without_live_handle_and_resets_live_without_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let metadata_only_id = "reset-topic-metadata-only";
        let mut metadata = crate::test_support::test_session_metadata(
            metadata_only_id,
            "Old metadata topic",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.label = "Old metadata topic".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(metadata_only_id.into(), 2);
        assert!(SessionApplication::new(state.clone()).reset_session_topic(metadata_only_id));
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session(metadata_only_id)
                .unwrap()
                .label,
            crate::session_types::placeholder_label(metadata_only_id)
        );

        let live_only_id = "reset-topic-live-only";
        let mut handle = attention_test_handle(live_only_id, root.path());
        handle.info.label = "Old live topic".into();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(live_only_id.into(), handle);
        *state.workspace.lock().unwrap() = None;
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(live_only_id.into(), 7);
        assert!(SessionApplication::new(state.clone()).reset_session_topic(live_only_id));
        assert_eq!(
            state.sessions.lock().unwrap()[live_only_id].info.label,
            crate::session_types::placeholder_label(live_only_id)
        );
        assert_eq!(
            state.peon.label_epochs.read().unwrap().get(live_only_id),
            Some(&8)
        );
    }

    #[test]
    fn clear_forgotten_session_tracking_removes_all_label_side_tables() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "clear-forgotten-application";
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(id.into(), 4);
        state.peon.label_hint.write().unwrap().insert(
            id.into(),
            crate::LabelHint {
                text: "stale topic".into(),
                epoch: 4,
            },
        );
        state.peon.label_pending.write().unwrap().insert(id.into());

        SessionApplication::new(state.clone()).clear_forgotten_session_tracking(id);

        assert!(!state.peon.label_epochs.read().unwrap().contains_key(id));
        assert!(!state.peon.label_hint.read().unwrap().contains_key(id));
        assert!(!state.peon.label_pending.read().unwrap().contains(id));
    }

    #[test]
    fn clear_forgotten_session_tracking_ignores_missing_ids() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());

        SessionApplication::new(state.clone()).clear_forgotten_session_tracking("missing");

        assert!(state.peon.label_epochs.read().unwrap().is_empty());
        assert!(state.peon.label_hint.read().unwrap().is_empty());
        assert!(state.peon.label_pending.read().unwrap().is_empty());
    }

    #[test]
    fn clear_ended_session_tracking_removes_runtime_state_but_preserves_label_work() {
        let _lease_guard = crate::runtime::peon_runtime::diagnostic_test_guard();
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "clear-ended-application";
        let token = "ended-session-report-token";

        state
            .peon
            .last_output
            .write()
            .unwrap()
            .insert(id.into(), tokio::time::Instant::now());
        state
            .peon
            .last_inference
            .write()
            .unwrap()
            .insert(id.into(), "working".into());
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert(id.into(), "pending input".into());
        state
            .peon
            .reported_cwd
            .write()
            .unwrap()
            .insert(id.into(), "/tmp/ended-session".into());
        state.session_pids.lock().unwrap().insert(id.into(), 4242);
        crate::runtime::terminal_runtime::set_workflow_report_token(id, token.into());

        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(id.into(), 7);
        state.peon.label_hint.write().unwrap().insert(
            id.into(),
            crate::LabelHint {
                text: "queued topic".into(),
                epoch: 7,
            },
        );
        state.peon.label_pending.write().unwrap().insert(id.into());

        SessionApplication::new(state.clone()).clear_ended_session_tracking(id);

        assert!(!state.peon.last_output.read().unwrap().contains_key(id));
        assert!(!state.peon.last_inference.read().unwrap().contains_key(id));
        assert!(!state.peon.input_buf.read().unwrap().contains_key(id));
        assert!(!state.peon.reported_cwd.read().unwrap().contains_key(id));
        assert!(!state.session_pids.lock().unwrap().contains_key(id));
        assert!(!crate::runtime::terminal_runtime::verify_workflow_report_token(id, token));
        assert_eq!(state.peon.label_epochs.read().unwrap().get(id), Some(&7));
        assert_eq!(
            state.peon.label_hint.read().unwrap().get(id),
            Some(&crate::LabelHint {
                text: "queued topic".into(),
                epoch: 7,
            })
        );
        assert!(state.peon.label_pending.read().unwrap().contains(id));
    }

    #[test]
    fn persist_input_label_updates_durable_and_live_copies() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "persist-input-label-both";
        let metadata = crate::test_support::test_session_metadata(
            id,
            "Old topic",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let handle = attention_test_handle(id, root.path());
        state.sessions.lock().unwrap().insert(id.into(), handle);
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(id.into(), 4);

        assert!(SessionApplication::new(state.clone()).persist_input_label(
            id,
            "New topic".into(),
            4
        ));
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session(id)
                .unwrap()
                .label,
            "New topic"
        );
        assert_eq!(state.sessions.lock().unwrap()[id].info.label, "New topic");
    }

    #[test]
    fn persist_input_label_updates_live_copy_without_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "persist-input-label-live-only";
        let handle = attention_test_handle(id, root.path());
        state.sessions.lock().unwrap().insert(id.into(), handle);
        *state.workspace.lock().unwrap() = None;
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(id.into(), 2);

        assert!(SessionApplication::new(state.clone()).persist_input_label(
            id,
            "Live topic".into(),
            2
        ));
        assert_eq!(state.sessions.lock().unwrap()[id].info.label, "Live topic");
    }

    #[test]
    fn persist_input_label_updates_durable_copy_without_live_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "persist-input-label-metadata-only";
        let metadata = crate::test_support::test_session_metadata(
            id,
            "Old topic",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(id.into(), 7);

        assert!(SessionApplication::new(state.clone()).persist_input_label(
            id,
            "Metadata topic".into(),
            7
        ));
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session(id)
                .unwrap()
                .label,
            "Metadata topic"
        );
        assert!(!state.sessions.lock().unwrap().contains_key(id));
    }

    #[test]
    fn persist_input_label_ignores_stale_epoch() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "persist-input-label-stale";
        let metadata = crate::test_support::test_session_metadata(
            id,
            "Old topic",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let mut handle = attention_test_handle(id, root.path());
        handle.info.label = "Live old topic".into();
        state.sessions.lock().unwrap().insert(id.into(), handle);
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(id.into(), 5);

        assert!(!SessionApplication::new(state.clone()).persist_input_label(
            id,
            "Stale topic".into(),
            4
        ));
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session(id)
                .unwrap()
                .label,
            "Old topic"
        );
        assert_eq!(
            state.sessions.lock().unwrap()[id].info.label,
            "Live old topic"
        );
    }

    #[test]
    fn persist_input_label_is_noop_without_workspace_or_live_session() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "persist-input-label-noop";
        *state.workspace.lock().unwrap() = None;
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(id.into(), 3);

        assert!(!SessionApplication::new(state.clone()).persist_input_label(
            id,
            "Unused topic".into(),
            3
        ));
        assert!(!state.sessions.lock().unwrap().contains_key(id));
    }

    #[test]
    fn record_user_input_topic_persists_metadata_only_and_does_not_queue_without_seed() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "input-topic-metadata-only";
        let metadata = crate::test_support::test_session_metadata(
            id,
            "Existing topic",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let queued =
            SessionApplication::new(state.clone()).record_user_input_topic(id, "follow up", false);

        assert!(!queued);
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.last_user_input.as_deref(), Some("follow up"));
        assert_eq!(stored.label, "Existing topic");
        assert!(state.sessions.lock().unwrap().get(id).is_none());
    }

    #[test]
    fn record_user_input_topic_seeds_placeholder_and_mirrors_live_label_once() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "input-topic-seed";
        let placeholder = crate::session_types::placeholder_label(id);
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            &placeholder,
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.label = placeholder.clone();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let mut handle = attention_test_handle(id, root.path());
        handle.info.label = placeholder;
        state.sessions.lock().unwrap().insert(id.into(), handle);

        assert!(
            SessionApplication::new(state.clone()).record_user_input_topic(
                id,
                "add retry logic",
                true,
            )
        );
        assert_eq!(
            state.sessions.lock().unwrap()[id].info.label,
            "add retry logic"
        );
        assert!(
            !SessionApplication::new(state.clone()).record_user_input_topic(
                id,
                "replace topic",
                true,
            )
        );
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.label, "add retry logic");
        assert_eq!(stored.last_user_input.as_deref(), Some("replace topic"));
    }

    #[test]
    fn record_user_input_topic_missing_state_is_safe_noop() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());

        assert!(!SessionApplication::new(state).record_user_input_topic(
            "missing-input-topic",
            "describe work",
            true,
        ));
    }

    #[test]
    fn refresh_workflow_recommendations_is_a_noop_without_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        *state.workspace.lock().unwrap() = None;

        SessionApplication::new(state).refresh_workflow_recommendations();
    }

    #[test]
    fn refresh_workflow_recommendations_persists_evaluated_proposals() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            for (key, evidence) in [("first", "first failure"), ("second", "second failure")] {
                workspace
                    .workflow_observations
                    .record_observation(
                        "workflow-refresh-session",
                        crate::workflow_observations::ObservationOrigin::Peon,
                        key,
                        crate::workflow_observations::ObservationCandidate {
                            kind: crate::workflow_observations::ObservationKind::Obstacle,
                            description: "The setup blocks progress".into(),
                            evidence: evidence.into(),
                            reported_impact: crate::workflow_observations::Impact::Medium,
                            confidence: Some(0.8),
                        },
                    )
                    .unwrap();
            }
        }

        SessionApplication::new(state.clone()).refresh_workflow_recommendations();

        let workspace = state.workspace.lock().unwrap();
        let proposals = workspace
            .as_ref()
            .unwrap()
            .recommendation_store
            .list()
            .unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(
            proposals[0].source_session_ids,
            vec!["workflow-refresh-session"]
        );
    }

    #[test]
    fn dismiss_recommendation_persists_the_transition_under_the_application() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            for key in ["first", "second"] {
                workspace
                    .workflow_observations
                    .record_observation(
                        "workflow-dismiss-session",
                        crate::workflow_observations::ObservationOrigin::Peon,
                        key,
                        crate::workflow_observations::ObservationCandidate {
                            kind: crate::workflow_observations::ObservationKind::Obstacle,
                            description: "The setup blocks progress".into(),
                            evidence: "The same command failed twice".into(),
                            reported_impact: crate::workflow_observations::Impact::Medium,
                            confidence: Some(0.8),
                        },
                    )
                    .unwrap();
            }
        }

        let application = SessionApplication::new(state.clone());
        application.refresh_workflow_recommendations();
        let recommendation_id = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .list()
            .unwrap()
            .pop()
            .unwrap()
            .id;
        let dismissed = application
            .dismiss_recommendation(&recommendation_id)
            .unwrap()
            .unwrap();

        assert_eq!(
            dismissed.status,
            crate::taskmaster::RecommendationStatus::Dismissed
        );
        let reloaded = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .get(&recommendation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            reloaded.status,
            crate::taskmaster::RecommendationStatus::Dismissed
        );
        assert!(reloaded.workflow_improvement.dismissal_watermark.is_some());
    }

    #[test]
    fn list_recommendations_returns_recommendations_and_diagnostics_under_the_application() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());

        {
            let workspace = state.workspace.lock().unwrap();
            for key in ["first-query-observation", "second-query-observation"] {
                workspace
                    .as_ref()
                    .unwrap()
                    .workflow_observations
                    .record_observation(
                        "workflow-query-session",
                        crate::workflow_observations::ObservationOrigin::Peon,
                        key,
                        crate::workflow_observations::ObservationCandidate {
                            kind: crate::workflow_observations::ObservationKind::Obstacle,
                            description: "The setup blocks progress".into(),
                            evidence: "The same command failed twice".into(),
                            reported_impact: crate::workflow_observations::Impact::Medium,
                            confidence: Some(0.8),
                        },
                    )
                    .unwrap();
            }
        }
        application.refresh_workflow_recommendations();
        let (recommendations, diagnostics) = application.list_recommendations().unwrap();

        assert_eq!(recommendations.len(), 1);
        assert_eq!(
            recommendations[0].source_session_ids,
            ["workflow-query-session"]
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn get_recommendation_returns_persisted_recommendation_under_the_application() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state.clone());

        {
            let workspace = state.workspace.lock().unwrap();
            for key in ["first-query-observation", "second-query-observation"] {
                workspace
                    .as_ref()
                    .unwrap()
                    .workflow_observations
                    .record_observation(
                        "workflow-query-session",
                        crate::workflow_observations::ObservationOrigin::Peon,
                        key,
                        crate::workflow_observations::ObservationCandidate {
                            kind: crate::workflow_observations::ObservationKind::Obstacle,
                            description: "The setup blocks progress".into(),
                            evidence: "The same command failed twice".into(),
                            reported_impact: crate::workflow_observations::Impact::Medium,
                            confidence: Some(0.8),
                        },
                    )
                    .unwrap();
            }
        }
        application.refresh_workflow_recommendations();
        let expected = application.list_recommendations().unwrap().0.pop().unwrap();

        assert_eq!(
            application.get_recommendation(&expected.id).unwrap(),
            Some(expected)
        );
    }

    #[test]
    fn recommendation_queries_return_conflict_without_a_workspace() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        *state.workspace.lock().unwrap() = None;
        let application = SessionApplication::new(state);

        assert!(matches!(
            application.list_recommendations(),
            Err(RecommendationQueryError::Conflict)
        ));
        assert!(matches!(
            application.get_recommendation("missing"),
            Err(RecommendationQueryError::Conflict)
        ));
    }

    #[test]
    fn persist_peon_inference_merges_eligible_metadata_and_returns_history_label() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "peon-inference-application";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "Peon inference",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let inference = crate::peon::PeonInference {
            observed_status: Some("blocked".into()),
            phase: None,
            summary: Some("Need a decision".into()),
            next_action: None,
            needs_user_input: Some(true),
            detected_question: None,
            suggested_options: None,
            blocker_description: Some("Waiting on review".into()),
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 0.8,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
            workflow_observations: Vec::new(),
        };
        let provider_observation = crate::providers::ProviderObservation {
            provider_id: "claude-code".into(),
            provider_label: "Claude Code".into(),
            provider_model: Some("sonnet".into()),
            provider_state: "healthy".into(),
        };

        let result = SessionApplication::new(state.clone()).persist_peon_observation(
            id,
            Some(&inference),
            Some(&provider_observation),
            Some("Need a decision"),
            "later",
        );

        assert!(result.inference_persisted);
        assert!(!result.permanent_hold);
        assert_eq!(
            result
                .label_update
                .as_ref()
                .map(|(label, _)| label.as_str()),
            Some("Need a decision")
        );
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("blocked"));
        assert_eq!(stored.summary.as_deref(), Some("Need a decision"));
        assert_eq!(stored.provider_id.as_deref(), Some("claude-code"));
        assert_eq!(stored.provider_model.as_deref(), Some("sonnet"));
    }

    #[test]
    fn persist_peon_inference_reports_user_source_as_permanent_hold() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "peon-inference-user-hold";
        let mut metadata = crate::test_support::test_session_metadata(
            id,
            "User topic",
            &root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.metadata_source = "user".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let inference = crate::peon::PeonInference {
            observed_status: Some("blocked".into()),
            phase: None,
            summary: Some("Should not land".into()),
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 0.8,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
            workflow_observations: Vec::new(),
        };

        let result = SessionApplication::new(state.clone()).persist_peon_observation(
            id,
            Some(&inference),
            None,
            Some("Should not land"),
            "later",
        );

        assert!(!result.inference_persisted);
        assert!(result.permanent_hold);
        assert!(result.label_update.is_none());
        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.summary, None);
        assert_eq!(stored.metadata_source, "user");
    }
}
