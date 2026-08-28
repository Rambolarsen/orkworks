use crate::runtime::session_runtime::RuntimeIdentity;
use crate::workspace_runtime::iso_now;
use crate::{peon, providers, AppState};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const MAX_DIAGNOSTIC_TEXT_CHARS: usize = 240;
const PEON_SHUTDOWN_DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

fn bounded_diagnostic_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_DIAGNOSTIC_TEXT_CHARS)
        .collect()
}

fn bounded_error_summary(error: &str) -> String {
    let summary = bounded_diagnostic_text(error);
    if summary.trim().is_empty() {
        "provider inference failed".to_string()
    } else {
        summary
    }
}

fn provider_error_summary(result: &providers::ProviderRunResult) -> String {
    result
        .attempts
        .iter()
        .rev()
        .find_map(|attempt| {
            result
                .runtime
                .get(&attempt.provider_id)
                .and_then(|runtime| runtime.last_error_summary.as_deref())
        })
        .map(bounded_error_summary)
        .unwrap_or_else(|| "all configured providers failed".to_string())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PeonDiagnosticAttempt {
    pub(crate) generation: u64,
    pub(crate) runtime_identity: RuntimeIdentity,
}

type DiagnosticLease = (u64, RuntimeIdentity);

static DIAGNOSTIC_LEASES: OnceLock<Mutex<HashMap<String, DiagnosticLease>>> = OnceLock::new();

fn diagnostic_leases() -> &'static Mutex<HashMap<String, DiagnosticLease>> {
    DIAGNOSTIC_LEASES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn runtime_identity_is_active(
    state: &AppState,
    session_id: &str,
    identity: &RuntimeIdentity,
) -> bool {
    state.sessions.lock().unwrap().get(session_id).is_some_and(|handle| {
        handle.runtime.matches_identity(identity) && handle.info.lifecycle_phase == "active"
    })
}

fn diagnostic_attempt_is_active(
    state: &AppState,
    session_id: &str,
    attempt: &PeonDiagnosticAttempt,
) -> bool {
    runtime_identity_is_active(state, session_id, &attempt.runtime_identity)
        && state.peon.diagnostic_attempt_is_current(session_id, attempt)
}

fn fail_attempt_if_active(
    state: &AppState,
    session_id: &str,
    attempt: &PeonDiagnosticAttempt,
    reason: &str,
    error: &str,
) -> bool {
    if !diagnostic_attempt_is_active(state, session_id, attempt) {
        return false;
    }
    state.peon.fail_attempt(session_id, attempt, reason, error)
}

fn complete_attempt_if_active(
    state: &AppState,
    session_id: &str,
    attempt: &PeonDiagnosticAttempt,
    result: &providers::ProviderRunResult,
) -> bool {
    if !diagnostic_attempt_is_active(state, session_id, attempt) {
        return false;
    }
    state.peon.complete_attempt(session_id, attempt, result)
}

fn timeout_attempt_if_active(
    state: &AppState,
    session_id: &str,
    attempt: &PeonDiagnosticAttempt,
) {
    if diagnostic_attempt_is_active(state, session_id, attempt) {
        state.peon.timeout_attempt(session_id, attempt);
    }
}

fn finish_attempt_if_active(
    state: &AppState,
    session_id: &str,
    attempt: &PeonDiagnosticAttempt,
) {
    if diagnostic_attempt_is_active(state, session_id, attempt) {
        state.peon.finish_attempt(session_id, attempt);
    }
}

fn refresh_observation_count_if_active(
    state: &AppState,
    session_id: &str,
    attempt: &PeonDiagnosticAttempt,
    count: Option<usize>,
) {
    if diagnostic_attempt_is_active(state, session_id, attempt) {
        state
            .peon
            .refresh_observation_count(session_id, attempt, count);
    }
}

impl crate::PeonState {
    fn diagnostic_entry<'a>(
        &self,
        diagnostics: &'a mut std::collections::HashMap<String, crate::PeonDiagnosticEntry>,
        leases: &HashMap<String, DiagnosticLease>,
        session_id: &str,
    ) -> Option<&'a mut crate::PeonDiagnosticEntry> {
        if !diagnostics.contains_key(session_id) {
            if diagnostics.len() >= crate::MAX_PEON_DIAGNOSTIC_SESSIONS {
                let evictable_id = diagnostics
                    .keys()
                    .find(|id| !leases.contains_key(*id))
                    .cloned();
                let Some(evictable_id) = evictable_id else {
                    return None;
                };
                diagnostics.remove(&evictable_id);
            }
            diagnostics.insert(
                session_id.to_string(),
                crate::PeonDiagnosticEntry::new(),
            );
        }
        diagnostics.get_mut(session_id)
    }

    fn mark_candidate(&self, session_id: &str) {
        let leases = diagnostic_leases().lock().unwrap();
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = self.diagnostic_entry(&mut diagnostics, &leases, session_id) else {
            return;
        };
        entry.snapshot.scheduler_state = crate::session_types::PeonSchedulerState::Candidate;
        entry.snapshot.reason = Some("selected_for_inference".to_string());
    }

    fn begin_attempt(
        &self,
        session_id: &str,
        runtime_identity: RuntimeIdentity,
    ) -> Option<PeonDiagnosticAttempt> {
        let mut leases = diagnostic_leases().lock().unwrap();
        let mut diagnostics = self.diagnostics.write().unwrap();
        let entry = self.diagnostic_entry(&mut diagnostics, &leases, session_id)?;
        if entry.snapshot.scheduler_state == crate::session_types::PeonSchedulerState::InFlight {
            return None;
        }
        entry.attempt_generation = entry.attempt_generation.saturating_add(1);
        entry.runtime_identity = Some(runtime_identity.clone());
        entry.snapshot.scheduler_state = crate::session_types::PeonSchedulerState::InFlight;
        entry.snapshot.reason = None;
        entry.snapshot.last_attempt_at = Some(iso_now());
        entry.snapshot.attempt_count = Some(
            entry
                .snapshot
                .attempt_count
                .unwrap_or_default()
                .saturating_add(1),
        );
        entry.snapshot.error_summary = None;
        let attempt = PeonDiagnosticAttempt {
            generation: entry.attempt_generation,
            runtime_identity,
        };
        leases.insert(
            session_id.to_string(),
            (attempt.generation, attempt.runtime_identity.clone()),
        );
        Some(attempt)
    }

    pub(crate) fn diagnostic_attempt_is_current(
        &self,
        session_id: &str,
        attempt: &PeonDiagnosticAttempt,
    ) -> bool {
        let leases = diagnostic_leases().lock().unwrap();
        if leases.get(session_id)
            != Some(&(attempt.generation, attempt.runtime_identity.clone()))
        {
            return false;
        }
        let diagnostics = self.diagnostics.read().unwrap();
        diagnostics.get(session_id).is_some_and(|entry| {
            entry.attempt_generation == attempt.generation
                && matches!(
                    entry.snapshot.scheduler_state,
                    crate::session_types::PeonSchedulerState::InFlight
                        | crate::session_types::PeonSchedulerState::Completed
                        | crate::session_types::PeonSchedulerState::Failed
                )
        })
    }

    pub(crate) fn invalidate_diagnostic_attempt(
        &self,
        session_id: &str,
        expected_runtime_identity: Option<&RuntimeIdentity>,
    ) -> bool {
        let mut leases = diagnostic_leases().lock().unwrap();
        let owns_diagnostics = match expected_runtime_identity {
            None => true,
            Some(expected_runtime_identity) => match leases.get(session_id) {
                Some((_, identity)) => identity == expected_runtime_identity,
                None => self
                    .diagnostics
                    .read()
                    .unwrap()
                    .get(session_id)
                    .and_then(|entry| entry.runtime_identity.as_ref())
                    .is_none_or(|identity| identity == expected_runtime_identity),
            },
        };
        if !owns_diagnostics {
            return false;
        }
        leases.remove(session_id);
        self.diagnostics.write().unwrap().remove(session_id);
        self.in_flight.write().unwrap().remove(session_id);
        true
    }

    fn complete_attempt(
        &self,
        session_id: &str,
        attempt: &PeonDiagnosticAttempt,
        result: &providers::ProviderRunResult,
    ) -> bool {
        if result.inference.is_none() {
            return false;
        }
        let leases = diagnostic_leases().lock().unwrap();
        if leases.get(session_id)
            != Some(&(attempt.generation, attempt.runtime_identity.clone()))
        {
            return false;
        }
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = diagnostics.get_mut(session_id) else {
            return false;
        };
        if entry.attempt_generation != attempt.generation
            || entry.snapshot.scheduler_state != crate::session_types::PeonSchedulerState::InFlight
        {
            return false;
        }

        let successful_attempt = result
            .attempts
            .iter()
            .rev()
            .find(|attempt| attempt.outcome == providers::AttemptOutcome::Succeeded);
        entry.snapshot.scheduler_state = crate::session_types::PeonSchedulerState::Completed;
        entry.snapshot.reason = None;
        entry.snapshot.last_successful_inference_at = Some(iso_now());
        entry.snapshot.provider_id = result
            .observation
            .as_ref()
            .map(|observation| bounded_diagnostic_text(&observation.provider_id))
            .or_else(|| {
                successful_attempt.map(|attempt| bounded_diagnostic_text(&attempt.provider_id))
            });
        entry.snapshot.provider_model = result
            .observation
            .as_ref()
            .and_then(|observation| {
                observation
                    .provider_model
                    .as_deref()
                    .map(bounded_diagnostic_text)
            });
        entry.snapshot.fallback_step = successful_attempt.map(|attempt| attempt.step);
        entry.snapshot.error_summary = None;
        true
    }

    fn fail_attempt(
        &self,
        session_id: &str,
        attempt: &PeonDiagnosticAttempt,
        reason: &str,
        error: &str,
    ) -> bool {
        let leases = diagnostic_leases().lock().unwrap();
        if leases.get(session_id)
            != Some(&(attempt.generation, attempt.runtime_identity.clone()))
        {
            return false;
        }
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = diagnostics.get_mut(session_id) else {
            return false;
        };
        if entry.attempt_generation != attempt.generation
            || entry.snapshot.scheduler_state != crate::session_types::PeonSchedulerState::InFlight
        {
            return false;
        }
        entry.snapshot.scheduler_state = crate::session_types::PeonSchedulerState::Failed;
        entry.snapshot.reason = Some(bounded_diagnostic_text(reason));
        entry.snapshot.error_summary = Some(bounded_error_summary(error));
        true
    }

    fn timeout_attempt(&self, session_id: &str, attempt: &PeonDiagnosticAttempt) {
        let leases = diagnostic_leases().lock().unwrap();
        if leases.get(session_id)
            != Some(&(attempt.generation, attempt.runtime_identity.clone()))
        {
            return;
        }
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = diagnostics.get_mut(session_id) else {
            return;
        };
        if entry.attempt_generation != attempt.generation
            || entry.snapshot.scheduler_state
                != crate::session_types::PeonSchedulerState::InFlight
        {
            return;
        }
        entry.snapshot.scheduler_state = crate::session_types::PeonSchedulerState::Failed;
        entry.snapshot.reason = Some("timeout".to_string());
        entry.snapshot.error_summary = Some("provider inference timed out".to_string());
        self.in_flight.write().unwrap().remove(session_id);
    }

    fn finish_attempt(&self, session_id: &str, attempt: &PeonDiagnosticAttempt) {
        let mut leases = diagnostic_leases().lock().unwrap();
        if leases.get(session_id)
            != Some(&(attempt.generation, attempt.runtime_identity.clone()))
        {
            return;
        }
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = diagnostics.get_mut(session_id) else {
            return;
        };
        if entry.attempt_generation != attempt.generation {
            return;
        }
        self.in_flight.write().unwrap().remove(session_id);
        leases.remove(session_id);
    }

    fn cleanup_attempt(&self, session_id: &str, attempt: &PeonDiagnosticAttempt) {
        let mut leases = diagnostic_leases().lock().unwrap();
        if leases.get(session_id)
            != Some(&(attempt.generation, attempt.runtime_identity.clone()))
        {
            return;
        }
        let mut diagnostics = self.diagnostics.write().unwrap();
        let was_in_flight = match diagnostics.get(session_id) {
            None => {
                self.in_flight.write().unwrap().remove(session_id);
                leases.remove(session_id);
                return;
            }
            Some(entry) if entry.attempt_generation == attempt.generation => {
                entry.snapshot.scheduler_state
                    == crate::session_types::PeonSchedulerState::InFlight
            }
            Some(_) => return,
        };
        self.in_flight.write().unwrap().remove(session_id);
        leases.remove(session_id);
        if was_in_flight {
            diagnostics.remove(session_id);
        }
    }

    fn mark_idle(&self, session_id: &str, reason: &str) {
        let leases = diagnostic_leases().lock().unwrap();
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = self.diagnostic_entry(&mut diagnostics, &leases, session_id) else {
            return;
        };
        if entry.snapshot.scheduler_state != crate::session_types::PeonSchedulerState::InFlight {
            entry.snapshot.scheduler_state = crate::session_types::PeonSchedulerState::Idle;
            entry.snapshot.reason = Some(reason.to_string());
        }
    }

    fn refresh_observation_count(
        &self,
        session_id: &str,
        attempt: &PeonDiagnosticAttempt,
        count: Option<usize>,
    ) {
        let leases = diagnostic_leases().lock().unwrap();
        if leases.get(session_id)
            != Some(&(attempt.generation, attempt.runtime_identity.clone()))
        {
            return;
        }
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = diagnostics.get_mut(session_id) else {
            return;
        };
        if entry.attempt_generation == attempt.generation {
            entry.snapshot.observation_count = count;
        }
    }
}

#[derive(Clone, Copy)]
enum InferenceMode {
    Output,
    InputLabel,
}

fn output_inference_is_current(
    captured_generation: u64,
    captured_min_revision: u64,
    current_generation: u64,
    current_min_revision: u64,
) -> bool {
    captured_generation == current_generation && captured_min_revision == current_min_revision
}

fn apply_output_label_update(
    state: &Arc<AppState>,
    session_id: &str,
    attempt: &PeonDiagnosticAttempt,
    label_update: (String, u64),
) {
    let (label, captured_epoch) = label_update;
    let label_epochs = state.peon.label_epochs.read().unwrap();
    if label_epochs.get(session_id).copied().unwrap_or(0) != captured_epoch {
        return;
    }
    let mut sessions = state.sessions.lock().unwrap();
    if state.peon.diagnostic_attempt_is_current(session_id, attempt) {
        if let Some(handle) = sessions.get_mut(session_id).filter(|handle| {
            handle.runtime.matches_identity(&attempt.runtime_identity)
                && handle.info.lifecycle_phase == "active"
        }) {
            handle.info.label = label;
        }
    }
}

pub(crate) async fn peon_loop(state: Arc<AppState>) {
    peon_loop_until(state, std::future::pending::<()>()).await;
}

async fn shutdown_inference_tasks(
    inference_tasks: &mut tokio::task::JoinSet<()>,
    cancellation: &AtomicBool,
) {
    cancellation.store(true, Ordering::SeqCst);
    let drain = async { while inference_tasks.join_next().await.is_some() {} };
    if tokio::time::timeout(PEON_SHUTDOWN_DRAIN_TIMEOUT, drain)
        .await
        .is_err()
    {
        tracing::warn!("peon shutdown timed out while waiting for inference tasks; aborting them");
        inference_tasks.abort_all();
    }
}

#[cfg(test)]
async fn peon_loop_for_test(
    state: Arc<AppState>,
    shutdown: tokio::sync::oneshot::Receiver<()>,
) {
    peon_loop_until(state, async {
        let _ = shutdown.await;
    })
    .await;
}

async fn peon_loop_until<F>(state: Arc<AppState>, shutdown: F)
where
    F: std::future::Future<Output = ()> + Send,
{
    let interval = state.peon.config.interval_secs;
    tracing::info!(interval_secs = interval, harness = %state.peon.config.harness, "peon started");
    tokio::pin!(shutdown);
    let mut inference_tasks = tokio::task::JoinSet::new();
    let cancellation = Arc::new(AtomicBool::new(false));

    loop {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            _ = &mut shutdown => {
                shutdown_inference_tasks(&mut inference_tasks, &cancellation).await;
                return;
            }
        }
        while inference_tasks.try_join_next().is_some() {}

        let now = tokio::time::Instant::now();
        let deadline = now - std::time::Duration::from_secs(interval);

        // Sessions with a pending label inference (input-triggered, no debounce)
        let pending: Vec<String> = state.peon.label_pending.write().unwrap().drain().collect();

        // Sessions with new output that has gone silent
        let mut candidates: Vec<(String, InferenceMode)> = {
            let last_output = state.peon.last_output.read().unwrap();
            let in_flight = state.peon.in_flight.read().unwrap();
            let sessions = state.sessions.lock().unwrap();

            last_output
                .iter()
                .filter(|(id, &t)| {
                    t <= deadline
                        && !in_flight.contains(*id)
                        && sessions
                            .get(*id)
                            .map(|handle| {
                                handle.info.lifecycle_phase == "active"
                                    && handle
                                        .output_buffer
                                        .has_after(handle.runtime.min_peon_output_revision)
                            })
                            .unwrap_or(false)
                })
                .map(|(id, _)| (id.clone(), InferenceMode::Output))
                .collect()
        };

        for (session_id, _) in &candidates {
            state.peon.mark_candidate(session_id);
        }

        for id in pending {
            if !state.peon.in_flight.read().unwrap().contains(&id)
                && !candidates
                    .iter()
                    .any(|(candidate_id, _)| candidate_id == &id)
            {
                candidates.push((id, InferenceMode::InputLabel));
            } else {
                // Can't schedule this tick (already in flight, or this
                // session already has a same-tick Output candidate) — put it
                // back rather than dropping the one-shot request entirely.
                state.peon.label_pending.write().unwrap().insert(id);
            }
        }

        for (session_id, mode) in candidates {
            {
                let mut in_flight = state.peon.in_flight.write().unwrap();
                if !in_flight.insert(session_id.clone()) {
                    continue;
                }
            }

            let (output_snapshot, output_boundary) = {
                let mut sessions = state.sessions.lock().unwrap();
                match sessions.get_mut(&session_id) {
                    Some(handle) => match mode {
                        InferenceMode::Output => {
                            let capture_is_current = handle
                                .runtime
                                .peon_output_capture
                                .as_ref()
                                .is_some_and(|capture| {
                                    output_inference_is_current(
                                        capture.input_generation,
                                        capture.min_revision,
                                        handle.runtime.input_generation,
                                        handle.runtime.min_peon_output_revision,
                                    )
                                });
                            if !capture_is_current {
                                handle.runtime.peon_output_capture = None;
                            }
                            if handle.runtime.peon_output_capture.is_none() {
                                handle.runtime.peon_output_capture = handle
                                    .output_buffer
                                    .snapshot_after_with_revisions(
                                        handle.runtime.min_peon_output_revision,
                                    )
                                    .map(|snapshot| peon::PeonOutputCapture {
                                        lines: snapshot.lines,
                                        input_generation: handle.runtime.input_generation,
                                        min_revision: handle.runtime.min_peon_output_revision,
                                        first_revision: snapshot.first_revision,
                                        last_revision: snapshot.last_revision,
                                        runtime_instance_id: handle.runtime.runtime_instance_id.clone(),
                                    });
                            }
                            handle.runtime.peon_output_capture.clone().map(|capture| {
                                    (
                                        capture.lines.clone(),
                                        Some((
                                            capture.input_generation,
                                            capture.min_revision,
                                            capture.first_revision,
                                            capture.last_revision,
                                        capture.runtime_instance_id,
                                        handle.runtime.run_generation(),
                                    )),
                                    )
                                })
                                .unwrap_or((Vec::new(), None))
                        }
                        InferenceMode::InputLabel => (Vec::new(), None),
                    },
                    None => {
                        state.peon.in_flight.write().unwrap().remove(&session_id);
                        state.peon.mark_idle(&session_id, "not_active");
                        continue;
                    }
                }
            };

            let hint = (matches!(mode, InferenceMode::InputLabel))
                .then(|| state.peon.label_hint.write().unwrap().remove(&session_id))
                .flatten();
            if output_snapshot.is_empty() && hint.is_none() {
                state.peon.in_flight.write().unwrap().remove(&session_id);
                state.peon.mark_idle(&session_id, "no_new_silent_output");
                continue;
            }

            let output_snapshot = hint
                .as_ref()
                .map(|h| vec![format!("[User input]: {}", h.text)])
                .unwrap_or(output_snapshot);

            let state_clone = state.clone();
            let id = session_id.clone();
            let Some(runtime_identity) = output_boundary
                .as_ref()
                .map(|(_, _, _, _, runtime_instance_id, run_generation)| RuntimeIdentity {
                    runtime_instance_id: runtime_instance_id.clone(),
                    run_generation: *run_generation,
                })
                .or_else(|| {
                    state
                        .sessions
                        .lock()
                        .unwrap()
                        .get(&id)
                        .map(|handle| handle.runtime.identity())
                })
            else {
                state.peon.in_flight.write().unwrap().remove(&id);
                state.peon.mark_idle(&id, "not_active");
                continue;
            };
            let Some(attempt) = state.peon.begin_attempt(&id, runtime_identity) else {
                state.peon.in_flight.write().unwrap().remove(&id);
                continue;
            };
            let attempt_cleanup = DiagnosticAttemptCleanup {
                state: state_clone.clone(),
                session_id: id.clone(),
                attempt: attempt.clone(),
            };
            let cancellation = cancellation.clone();
            inference_tasks.spawn(async move {
            let _attempt_cleanup = attempt_cleanup;
            let provider_state = state_clone.clone();
            let cleanup_state = state_clone.clone();
            let provider_output = output_snapshot.clone();
            let mut provider_task = tokio::task::spawn_blocking(move || {
                provider_state
                    .providers
                    .run_inference(providers::PeonScope::Session, &provider_output)
            });
            let provider_result = match tokio::time::timeout(
                std::time::Duration::from_secs(120),
                &mut provider_task,
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(error)) => {
                    tracing::warn!(session_id = %id, %error, "peon inference task failed");
                    if fail_attempt_if_active(
                        &cleanup_state,
                        &id,
                        &attempt,
                        "provider_task_failed",
                        &error.to_string(),
                    ) {
                        finish_attempt_if_active(&cleanup_state, &id, &attempt);
                    }
                    return;
                }
                Err(_) => {
                    tracing::warn!(session_id = %id, "peon inference timed out");
                    timeout_attempt_if_active(&cleanup_state, &id, &attempt);
                    let _ = provider_task.await;
                    finish_attempt_if_active(&cleanup_state, &id, &attempt);
                    return;
                }
            };

            if cancellation.load(Ordering::SeqCst) {
                cleanup_state.peon.cleanup_attempt(&id, &attempt);
                return;
            }

            if provider_result.inference.is_some() {
                if !complete_attempt_if_active(&state_clone, &id, &attempt, &provider_result) {
                    return;
                }
            } else {
                if !diagnostic_attempt_is_active(&state_clone, &id, &attempt) {
                    return;
                }
                fail_attempt_if_active(
                    &state_clone,
                    &id,
                    &attempt,
                    "provider_exhausted",
                    &provider_error_summary(&provider_result),
                );
            }

            // The blocking post-processing task may outlive this JoinSet task
            // when shutdown has to abort its bounded drain. Keep the existing
            // runtime/attempt identity guards in the persistence methods and
            // also make the detached path cooperatively cancel before every
            // side effect it can still reach.
            let post_processing_cancellation = cancellation.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if post_processing_cancellation.load(Ordering::SeqCst) {
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                }
                if matches!(mode, InferenceMode::InputLabel) {
                    if let Some(inference) = provider_result.inference {
                        if let Some(label) = inference
                            .summary
                            .map(|summary| summary.chars().take(100).collect::<String>())
                        .filter(|label| !label.trim().is_empty())
                        .filter(|label| {
                            hint.as_ref()
                                .is_some_and(|hint| peon::is_usable_input_label(label, &hint.text))
                        })
                    {
                        if !post_processing_cancellation.load(Ordering::SeqCst) {
                            if let Some(hint) = hint.as_ref() {
                                crate::session_application::SessionApplication::new(
                                    state_clone.clone(),
                                )
                                .persist_input_label_for_attempt(&id, &attempt, label, hint.epoch);
                            }
                        }
                    }
                    }
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                }
                if post_processing_cancellation.load(Ordering::SeqCst) {
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                }
                let active_work_hook = {
                    let sessions = state_clone.sessions.lock().unwrap();
                    sessions.get(&id).and_then(|handle| {
                        let (generation, min_revision, _, _, runtime_instance_id, run_generation) =
                            output_boundary.as_ref()?;
                        (output_inference_is_current(
                            *generation,
                            *min_revision,
                            handle.runtime.input_generation,
                            handle.runtime.min_peon_output_revision,
                        ) && handle.runtime.runtime_instance_id == *runtime_instance_id
                            && handle.runtime.run_generation() == *run_generation)
                            .then_some(handle.active_work_hook)
                    })
                };
                let Some(active_work_hook) = active_work_hook else {
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                };
                if !diagnostic_attempt_is_active(&state_clone, &id, &attempt) {
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                }
                if post_processing_cancellation.load(Ordering::SeqCst) {
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                }
                let inference = provider_result.inference;
                let now_iso = iso_now();

                // Check terminal status before moving inference below
                let reached_terminal = matches!(
                    inference
                        .as_ref()
                        .and_then(|inf| inf.observed_status.as_deref()),
                    Some("done" | "idle" | "stale")
                );

                let mut output_range_completed = false;
                let mut accepted_observation = false;
                let mut inference = inference;
                if let Some(inf) = inference.as_mut() {
                    // Active-hook sessions are hook-authoritative for the working
                    // transition specifically: Peon may still persist summary/label/
                    // etc, but must not be the one to flip observed_status to working
                    // out from under the fail-closed hook contract (mirrors the
                    // synchronous PTY-output fallback's own active_work_hook gate).
                    if active_work_hook && inf.observed_status.as_deref() == Some("working") {
                        inf.observed_status = None;
                    }
                }
                let history_summary = inference
                    .as_ref()
                    .and_then(|inf| {
                        peon::work_history_summary(&output_snapshot, inf.summary.as_deref())
                    });
                let persistence = crate::session_application::SessionApplication::new(
                    state_clone.clone(),
                )
                .persist_peon_observation_for_attempt(
                    &id,
                    &attempt,
                    inference.as_ref(),
                    provider_result.observation.as_ref(),
                    history_summary.as_deref(),
                    &now_iso,
                );
                if post_processing_cancellation.load(Ordering::SeqCst) {
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                }
                let inference_persisted = persistence.inference_persisted;
                let permanent_hold = persistence.permanent_hold;
                let label_update = persistence.label_update;
                let captured_workspace_path = persistence.workspace_path;
                if let Some(inf) = inference.as_ref() {
                    if let Some((_input_generation, _min_revision, first_revision, last_revision, runtime_instance_id, run_generation)) =
                        output_boundary.as_ref()
                    {
                        if post_processing_cancellation.load(Ordering::SeqCst) {
                            finish_attempt_if_active(&state_clone, &id, &attempt);
                            return;
                        }
                        let result = crate::session_application::SessionApplication::new(
                            state_clone.clone(),
                        )
                        .record_peon_workflow_observations_for_attempt(
                            &id,
                            captured_workspace_path.as_deref(),
                            &attempt,
                            &crate::session_application::PeonObservationOutputRange {
                                runtime_instance_id: runtime_instance_id.clone(),
                                run_generation: *run_generation,
                                first_revision: *first_revision,
                                last_revision: *last_revision,
                            },
                            &inf.workflow_observations,
                        );
                        accepted_observation = result.accepted_observation;
                        output_range_completed = result.output_range_completed;
                    }
                }

                if accepted_observation
                    && !post_processing_cancellation.load(Ordering::SeqCst)
                {
                    crate::taskmaster::evaluator::schedule_evaluation(state_clone.clone());
                }
                if post_processing_cancellation.load(Ordering::SeqCst) {
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                }
                if output_range_completed {
                    if let Some((generation, min_revision, _, last_revision, runtime_instance_id, run_generation)) = output_boundary {
                        if let Some(handle) = state_clone.sessions.lock().unwrap().get_mut(&id) {
                            if output_inference_is_current(
                                generation,
                                min_revision,
                                handle.runtime.input_generation,
                                handle.runtime.min_peon_output_revision,
                            ) && handle.runtime.runtime_instance_id == runtime_instance_id
                                && handle.runtime.run_generation() == run_generation
                                && handle.info.lifecycle_phase == "active"
                            {
                                handle.runtime.min_peon_output_revision = last_revision;
                                handle.runtime.peon_output_capture = None;
                            }
                        }
                    }
                }
                if output_range_completed {
                    let observation_count = state_clone
                        .workspace
                        .lock()
                        .unwrap()
                        .as_ref()
                        .and_then(|workspace| {
                            workspace
                                .workflow_observations
                                .session_observation_count(&id)
                                .ok()
                        });
                    refresh_observation_count_if_active(
                        &state_clone,
                        &id,
                        &attempt,
                        observation_count,
                    );
                }
                if let Some(label) = label_update {
                    if post_processing_cancellation.load(Ordering::SeqCst) {
                        finish_attempt_if_active(&state_clone, &id, &attempt);
                        return;
                    }
                    apply_output_label_update(&state_clone, &id, &attempt, label);
                }

                if post_processing_cancellation.load(Ordering::SeqCst) {
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                }
                let sessions = state_clone.sessions.lock().unwrap();
                if !sessions.get(&id).is_some_and(|handle| {
                    handle.runtime.matches_identity(&attempt.runtime_identity)
                        && handle.info.lifecycle_phase == "active"
                }) || !state_clone.peon.diagnostic_attempt_is_current(&id, &attempt) {
                    drop(sessions);
                    finish_attempt_if_active(&state_clone, &id, &attempt);
                    return;
                }
                state_clone
                    .peon
                    .last_inference
                    .write()
                    .unwrap()
                    .insert(id.clone(), now_iso);

                // Three scheduling outcomes:
                // 1. Persisted + terminal: don't update last_output; lifecycle change removes session.
                // 2. Permanent hold (user source) + terminal: remove from pool entirely; new PTY
                //    output via terminal_runtime re-adds when the session becomes active again.
                // 3. A persisted non-terminal inference waits for new terminal output. A
                //    failed write or transient hold remains eligible for retry.
                if reached_terminal && inference_persisted {
                    // outcome 1: leave last_output unchanged
                } else if reached_terminal && permanent_hold {
                    state_clone.peon.last_output.write().unwrap().remove(&id);
                } else if inference_persisted {
                    state_clone.peon.last_output.write().unwrap().remove(&id);
                } else {
                    state_clone
                        .peon
                        .last_output
                        .write()
                        .unwrap()
                        .insert(id.clone(), tokio::time::Instant::now());
                }
                drop(sessions);
                finish_attempt_if_active(&state_clone, &id, &attempt);
            })
            .await;
            });
        }

        // Timer-based idle detection: mark sessions that have been silent
        // for idle_timeout_secs as idle, without waiting for the LLM.
        {
            let idle_timeout = state.peon.config.idle_timeout_secs;
            let idle_deadline =
                tokio::time::Instant::now() - std::time::Duration::from_secs(idle_timeout);
            let last_output = state.peon.last_output.read().unwrap();

            let (silent_ids, missing_last_output_ids): (Vec<String>, Vec<String>) = {
                let sessions = state.sessions.lock().unwrap();
                let mut silent_ids = Vec::new();
                let mut missing_last_output_ids = Vec::new();

                for (id, handle) in sessions.iter() {
                    // Active-hook sessions are hook-authoritative: the hook may
                    // legitimately go silent on the PTY for long stretches while
                    // still working, and only the hook (or process end) may
                    // clear that state — this timer must not race it to idle.
                    if handle.info.status != "running"
                        || handle.info.lifecycle_phase != "active"
                        || handle.active_work_hook
                        || !matches!(
                            handle.info.observed_status.as_deref(),
                            None | Some("working")
                        )
                    {
                        continue;
                    }

                    match last_output.get(id) {
                        Some(&t) if t <= idle_deadline => silent_ids.push(id.clone()),
                        Some(_) => {}
                        None => missing_last_output_ids.push(id.clone()),
                    }
                }

                (silent_ids, missing_last_output_ids)
            };
            drop(last_output);

            if !missing_last_output_ids.is_empty() {
                let now = tokio::time::Instant::now();
                let mut last_output = state.peon.last_output.write().unwrap();
                for id in missing_last_output_ids {
                    // Self-heal the transient gap where a session is visible
                    // as running before its startup idle timer origin exists.
                    last_output.entry(id).or_insert(now);
                }
            }

            for id in &silent_ids {
                crate::session_application::SessionApplication::new(state.clone())
                    .apply_idle_timeout(id);
            }
        }
    }
}

struct DiagnosticAttemptCleanup {
    state: Arc<AppState>,
    session_id: String,
    attempt: PeonDiagnosticAttempt,
}

impl Drop for DiagnosticAttemptCleanup {
    fn drop(&mut self) {
        self.state
            .peon
            .cleanup_attempt(&self.session_id, &self.attempt);
    }
}

#[cfg(test)]
static DIAGNOSTIC_TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub(crate) struct DiagnosticTestGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl Drop for DiagnosticTestGuard {
    fn drop(&mut self) {
        match diagnostic_leases().lock() {
            Ok(mut leases) => leases.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }
}

#[cfg(test)]
pub(crate) fn diagnostic_test_guard() -> DiagnosticTestGuard {
    let lock = DIAGNOSTIC_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match diagnostic_leases().lock() {
        Ok(mut leases) => leases.clear(),
        Err(poisoned) => poisoned.into_inner().clear(),
    }
    DiagnosticTestGuard { _lock: lock }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata;
    use crate::test_support::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicU16, AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier, Mutex, RwLock};

    #[test]
    fn output_inference_generation_is_stale_after_accepted_input() {
        assert!(!output_inference_is_current(4, 12, 5, 12));
    }

    #[test]
    fn output_label_update_cannot_restore_a_topic_after_reset() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let session_id = "output-label-reset";
        let runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        let runtime_identity = runtime.identity();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.to_string(),
            crate::SessionHandle {
                info: test_session_info(
                    session_id,
                    "Old topic",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime,
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
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&crate::test_support::test_session_metadata(
                session_id,
                "Old topic",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            ));
        state.peon.in_flight.write().unwrap().insert(session_id.into());
        state.peon.mark_candidate(session_id);
        let attempt = state
            .peon
            .begin_attempt(session_id, runtime_identity)
            .expect("output attempt should start");
        let inference = peon::parse_inference(r#"{"status":"working","confidence":0.85}"#);
        let persistence = crate::session_application::SessionApplication::new(state.clone())
            .persist_peon_observation_for_attempt(
                session_id,
                &attempt,
                inference.as_ref(),
                None,
                Some("Old inferred topic"),
                "later",
            );
        let label_update = persistence
            .label_update
            .expect("output inference should return a label update");

        crate::session_application::SessionApplication::new(state.clone())
            .reset_session_topic(session_id);
        apply_output_label_update(&state, session_id, &attempt, label_update);

        let placeholder = crate::session_types::placeholder_label(session_id);
        assert_eq!(state.sessions.lock().unwrap()[session_id].info.label, placeholder);
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session(session_id)
                .unwrap()
                .label,
            placeholder
        );
    }

    fn test_runtime_identity(name: &str, generation: u64) -> RuntimeIdentity {
        RuntimeIdentity {
            runtime_instance_id: name.to_string(),
            run_generation: generation,
        }
    }

    fn spawn_test_peon_loop(
        state: Arc<AppState>,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (shutdown, receiver) = tokio::sync::oneshot::channel();
        (
            tokio::spawn(peon_loop_for_test(state, receiver)),
            shutdown,
        )
    }

    async fn stop_test_peon_loop(
        task: tokio::task::JoinHandle<()>,
        shutdown: tokio::sync::oneshot::Sender<()>,
    ) {
        let _ = shutdown.send(());
        task.await.expect("test Peon loop should shut down cleanly");
    }

    #[tokio::test]
    async fn graceful_shutdown_bounds_inference_task_drain() {
        let mut inference_tasks = tokio::task::JoinSet::new();
        inference_tasks.spawn(std::future::pending::<()>());
        let cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let started = tokio::time::Instant::now();
        shutdown_inference_tasks(&mut inference_tasks, &cancellation).await;

        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "graceful shutdown must not wait indefinitely for inference tasks"
        );
        assert!(inference_tasks.join_next().await.is_some());
        assert!(cancellation.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn shutdown_cancels_post_processing_before_detached_work_can_start() {
        let mut inference_tasks = tokio::task::JoinSet::new();
        let cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));

        shutdown_inference_tasks(&mut inference_tasks, &cancellation).await;

        assert!(cancellation.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn shutdown_of_started_blocking_inference_releases_diagnostic_attempt_state() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let mut state = crate::test_support::test_app_state_with_workspace(dir.path());
        let provider_entered = Arc::new(Barrier::new(2));
        let provider_release = Arc::new(Barrier::new(2));
        let settings = providers::ProviderSettingsPayload {
            providers: vec![providers::ProviderSettingsEntry {
                id: "ollama".into(),
                enabled: true,
                fallback_order: 0,
                model: None,
                default_state: providers::ProviderCapacityState::Healthy,
                override_state: None,
            }],
            ..Default::default()
        };
        let fake_provider = providers::FakeProvider::new("ollama")
            .stdout(r#"{"status":"working","confidence":0.9}"#)
            .with_barriers(provider_entered.clone(), provider_release.clone());
        let state_mut = Arc::get_mut(&mut state).unwrap();
        state_mut.providers = providers::ProviderManager::for_tests(settings, vec![fake_provider]);
        state_mut.providers.mark_applied_for_tests("ollama", Some("test-model"));
        state_mut.peon.config.interval_secs = 0;

        let session_id = "shutdown-blocking-inference";
        let runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        state.sessions.lock().unwrap().insert(
            session_id.into(),
            crate::SessionHandle {
                info: test_session_info(
                    session_id,
                    "Blocking inference",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
                kill_tx: tokio::sync::watch::channel(false).0,
                output_buffer: {
                    let mut output = peon::RingBuffer::new(200);
                    output.push("provider input".into());
                    output
                },
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime,
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
        state.peon.last_output.write().unwrap().insert(
            session_id.into(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(1),
        );

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let loop_task = tokio::spawn(peon_loop_for_test(state.clone(), shutdown_rx));
        tokio::task::spawn_blocking(move || provider_entered.wait())
            .await
            .unwrap();

        shutdown_tx.send(()).unwrap();
        // The provider call is deliberately blocking. Release it after
        // shutdown starts so the runtime can finish its bounded drain instead
        // of waiting forever for a barrier that this test releases too late.
        tokio::task::spawn_blocking(move || provider_release.wait())
            .await
            .unwrap();
        loop_task.await.unwrap();

        assert!(!state
            .peon
            .in_flight
            .read()
            .unwrap()
            .contains(session_id));
        assert!(!diagnostic_leases()
            .lock()
            .unwrap()
            .contains_key(session_id));
        assert!(!state
            .peon
            .diagnostics
            .read()
            .unwrap()
            .contains_key(session_id));
    }

    #[test]
    fn stale_attempt_cleanup_cannot_release_a_newer_session_lease() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let session_id = "generation-guard";

        state.peon.mark_candidate(session_id);
        state
            .peon
            .in_flight
            .write()
            .unwrap()
            .insert(session_id.to_string());
        let runtime_identity = test_runtime_identity(session_id, 1);
        let first_attempt = state
            .peon
            .begin_attempt(session_id, runtime_identity.clone())
            .expect("first attempt should start");
        state
            .peon
            .timeout_attempt(session_id, &first_attempt);

        state
            .peon
            .in_flight
            .write()
            .unwrap()
            .insert(session_id.to_string());
        state.peon.mark_candidate(session_id);
        let second_attempt = state
            .peon
            .begin_attempt(session_id, runtime_identity)
            .expect("second attempt should start");
        assert!(second_attempt.generation > first_attempt.generation);

        state.peon.finish_attempt(session_id, &first_attempt);
        assert_eq!(
            diagnostic_leases().lock().unwrap().get(session_id),
            Some(&(second_attempt.generation, second_attempt.runtime_identity.clone()))
        );
        assert!(state
            .peon
            .in_flight
            .read()
            .unwrap()
            .contains(session_id));
        assert_eq!(
            state
                .peon
                .diagnostics
                .read()
                .unwrap()
                .get(session_id)
                .unwrap()
                .snapshot
                .attempt_count,
            Some(2)
        );

        state.peon.finish_attempt(session_id, &second_attempt);
        assert!(!state
            .peon
            .in_flight
            .read()
            .unwrap()
            .contains(session_id));
    }

    #[test]
    fn timed_out_runtime_completion_is_rejected_after_runtime_replacement() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let session_id = "runtime-replacement-diagnostic";
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let old_runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        let old_identity = old_runtime.identity();
        state.sessions.lock().unwrap().insert(
            session_id.to_string(),
            crate::SessionHandle {
                info: test_session_info(
                    session_id.to_string(),
                    "Runtime replacement",
                    &dir.path().display().to_string(),
                    "running",
                    "now",
                ),
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: old_runtime,
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

        state.peon.mark_candidate(session_id);
        state
            .peon
            .in_flight
            .write()
            .unwrap()
            .insert(session_id.to_string());
        let old_attempt = state
            .peon
            .begin_attempt(session_id, old_identity.clone())
            .expect("old runtime attempt should start");
        state.peon.timeout_attempt(session_id, &old_attempt);

        crate::session_application::SessionApplication::new(state.clone())
            .clear_ended_session_tracking(session_id);
        assert!(!state.peon.diagnostics.read().unwrap().contains_key(session_id));

        let new_runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        let new_identity = new_runtime.identity();
        assert_ne!(old_identity, new_identity);
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut(session_id)
            .unwrap()
            .runtime = new_runtime;
        state.peon.mark_candidate(session_id);
        state
            .peon
            .in_flight
            .write()
            .unwrap()
            .insert(session_id.to_string());
        let new_attempt = state
            .peon
            .begin_attempt(session_id, new_identity)
            .expect("replacement runtime attempt should start");
        crate::session_application::SessionApplication::new(state.clone())
            .clear_ended_session_tracking_for_runtime(session_id, &old_identity);
        assert_eq!(
            diagnostic_leases().lock().unwrap().get(session_id),
            Some(&(new_attempt.generation, new_attempt.runtime_identity.clone()))
        );

        let result = providers::ProviderRunResult {
            inference: peon::parse_inference(r#"{"status":"blocked","confidence":0.9}"#),
            observation: None,
            attempts: Vec::new(),
            runtime: HashMap::new(),
        };
        assert!(!diagnostic_attempt_is_active(&state, session_id, &old_attempt));
        assert!(!state.peon.complete_attempt(session_id, &old_attempt, &result));
        state.peon.finish_attempt(session_id, &old_attempt);
        assert_eq!(
            diagnostic_leases().lock().unwrap().get(session_id),
            Some(&(new_attempt.generation, new_attempt.runtime_identity.clone()))
        );
        assert!(state
            .peon
            .in_flight
            .read()
            .unwrap()
            .contains(session_id));
        assert_eq!(
            state.peon.diagnostics.read().unwrap()[session_id]
                .snapshot
                .scheduler_state,
            crate::session_types::PeonSchedulerState::InFlight
        );
        let application = crate::session_application::SessionApplication::new(state.clone());
        assert!(!application
            .persist_peon_observation_for_attempt(
                session_id,
                &old_attempt,
                result.inference.as_ref(),
                None,
                None,
                "later",
            )
            .inference_persisted);
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&test_session_metadata(
                session_id,
                "Runtime replacement",
                &dir.path().display().to_string(),
                "running",
                "now",
                "now",
            ));
        let observation_result = application.record_peon_workflow_observations_for_attempt(
            session_id,
            Some(dir.path()),
            &old_attempt,
            &crate::session_application::PeonObservationOutputRange {
                runtime_instance_id: old_attempt.runtime_identity.runtime_instance_id.clone(),
                run_generation: old_attempt.runtime_identity.run_generation,
                first_revision: 1,
                last_revision: 1,
            },
            &[peon::PeonWorkflowObservation {
                kind: crate::workflow_observations::ObservationKind::Obstacle,
                description: "old runtime observation".into(),
                evidence: "old runtime evidence".into(),
                reported_impact: crate::workflow_observations::Impact::Low,
                confidence: 0.8,
            }],
        );
        assert!(!observation_result.accepted_observation);
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .workflow_observations
            .workspace_observations()
            .unwrap()
            .is_empty());
        assert_eq!(new_attempt.generation, 1);
    }

    #[test]
    fn completed_diagnostic_cleanup_uses_retained_runtime_identity_after_lease_release() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let session_id = "completed-diagnostic-cleanup";
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        let identity = runtime.identity();
        state.sessions.lock().unwrap().insert(
            session_id.to_string(),
            crate::SessionHandle {
                info: test_session_info(
                    session_id,
                    "Completed diagnostic",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime,
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
        state.peon.in_flight.write().unwrap().insert(session_id.into());
        state.peon.mark_candidate(session_id);
        let attempt = state
            .peon
            .begin_attempt(session_id, identity.clone())
            .expect("diagnostic attempt should start");
        let result = providers::ProviderRunResult {
            inference: peon::parse_inference(r#"{"status":"working","confidence":0.85}"#),
            observation: None,
            attempts: Vec::new(),
            runtime: HashMap::new(),
        };
        assert!(state.peon.complete_attempt(session_id, &attempt, &result));
        state.peon.finish_attempt(session_id, &attempt);
        assert!(!diagnostic_leases().lock().unwrap().contains_key(session_id));
        assert!(state.peon.diagnostics.read().unwrap().contains_key(session_id));

        crate::session_application::SessionApplication::new(state.clone())
            .clear_ended_session_tracking_for_runtime(session_id, &identity);

        assert!(!state.peon.diagnostics.read().unwrap().contains_key(session_id));
    }

    #[test]
    fn stale_runtime_cleanup_does_not_clear_a_replacement_completed_diagnostic() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let session_id = "replacement-completed-diagnostic";
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let old_runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        let old_identity = old_runtime.identity();
        state.sessions.lock().unwrap().insert(
            session_id.to_string(),
            crate::SessionHandle {
                info: test_session_info(
                    session_id,
                    "Replacement diagnostic",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: old_runtime,
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
        state.peon.in_flight.write().unwrap().insert(session_id.into());
        state.peon.mark_candidate(session_id);
        let old_attempt = state
            .peon
            .begin_attempt(session_id, old_identity.clone())
            .expect("old attempt should start");
        let result = providers::ProviderRunResult {
            inference: peon::parse_inference(r#"{"status":"working","confidence":0.85}"#),
            observation: None,
            attempts: Vec::new(),
            runtime: HashMap::new(),
        };
        assert!(state.peon.complete_attempt(session_id, &old_attempt, &result));
        state.peon.finish_attempt(session_id, &old_attempt);

        let new_runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        let new_identity = new_runtime.identity();
        assert_ne!(old_identity, new_identity);
        state.sessions.lock().unwrap().get_mut(session_id).unwrap().runtime = new_runtime;
        state.peon.in_flight.write().unwrap().insert(session_id.into());
        state.peon.mark_candidate(session_id);
        let new_attempt = state
            .peon
            .begin_attempt(session_id, new_identity)
            .expect("replacement attempt should start");
        assert!(state.peon.complete_attempt(session_id, &new_attempt, &result));
        state.peon.finish_attempt(session_id, &new_attempt);
        state.peon.last_output.write().unwrap().insert(
            session_id.into(),
            tokio::time::Instant::now(),
        );

        crate::session_application::SessionApplication::new(state.clone())
            .clear_ended_session_tracking_for_runtime(session_id, &old_identity);

        assert!(state.peon.diagnostics.read().unwrap().contains_key(session_id));
        assert!(state
            .peon
            .last_output
            .read()
            .unwrap()
            .contains_key(session_id));
    }

    #[test]
    fn diagnostic_eviction_preserves_all_in_flight_entries_and_leases() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());

        for index in 0..crate::MAX_PEON_DIAGNOSTIC_SESSIONS {
            let session_id = format!("in-flight-{index}");
            state
                .peon
                .in_flight
                .write()
                .unwrap()
                .insert(session_id.clone());
            state.peon.mark_candidate(&session_id);
            let attempt = state
                .peon
                .begin_attempt(&session_id, test_runtime_identity(&session_id, index as u64 + 1))
                .expect("in-flight diagnostic should start");
            if index == 0 {
                state.peon.timeout_attempt(&session_id, &attempt);
            }
        }

        state.peon.mark_candidate("new-diagnostic");

        {
            let leases = diagnostic_leases().lock().unwrap();
            let diagnostics = state.peon.diagnostics.read().unwrap();
            let in_flight = state.peon.in_flight.read().unwrap();
            assert_eq!(diagnostics.len(), crate::MAX_PEON_DIAGNOSTIC_SESSIONS);
            assert!(!diagnostics.contains_key("new-diagnostic"));
            for index in 0..crate::MAX_PEON_DIAGNOSTIC_SESSIONS {
                let session_id = format!("in-flight-{index}");
                assert!(diagnostics.contains_key(&session_id));
                assert_eq!(
                    leases.get(&session_id).map(|lease| lease.0),
                    Some(1)
                );
                if index == 0 {
                    assert!(!in_flight.contains(&session_id));
                } else {
                    assert!(in_flight.contains(&session_id));
                }
            }
        }
    }

    #[test]
    fn provider_exhaustion_keeps_lease_until_post_processing_finishes() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let session_id = "provider-exhaustion-lease";

        state
            .peon
            .in_flight
            .write()
            .unwrap()
            .insert(session_id.to_string());
        state.peon.mark_candidate(session_id);
        let attempt = state
            .peon
            .begin_attempt(session_id, test_runtime_identity(session_id, 1))
            .expect("provider exhaustion attempt should start");

        state
            .peon
            .fail_attempt(session_id, &attempt, "provider_exhausted", "all providers failed");

        assert!(state
            .peon
            .in_flight
            .read()
            .unwrap()
            .contains(session_id));
        assert_eq!(
            diagnostic_leases().lock().unwrap().get(session_id),
            Some(&(attempt.generation, attempt.runtime_identity.clone()))
        );
        assert_eq!(
            state.peon.diagnostics.read().unwrap()[session_id]
                .snapshot
                .scheduler_state,
            crate::session_types::PeonSchedulerState::Failed
        );

        state.peon.finish_attempt(session_id, &attempt);
        assert!(!state
            .peon
            .in_flight
            .read()
            .unwrap()
            .contains(session_id));
    }

    #[test]
    fn diagnostic_provider_and_error_strings_are_bounded() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let session_id = "bounded-diagnostics";
        let oversized = "x".repeat(MAX_DIAGNOSTIC_TEXT_CHARS + 20);

        state
            .peon
            .in_flight
            .write()
            .unwrap()
            .insert(session_id.to_string());
        state.peon.mark_candidate(session_id);
        let attempt = state
            .peon
            .begin_attempt(session_id, test_runtime_identity(session_id, 1))
            .expect("diagnostic attempt should start");
        let result = providers::ProviderRunResult {
            inference: peon::parse_inference(r#"{"status":"working","confidence":0.85}"#),
            observation: Some(providers::ProviderObservation {
                provider_id: oversized.clone(),
                provider_label: "provider".into(),
                provider_model: Some(oversized.clone()),
                provider_state: "healthy".into(),
            }),
            attempts: vec![providers::AttemptRecord {
                provider_id: oversized.clone(),
                step: 1,
                outcome: providers::AttemptOutcome::Succeeded,
            }],
            runtime: HashMap::new(),
        };

        assert!(state.peon.complete_attempt(session_id, &attempt, &result));
        {
            let diagnostics = state.peon.diagnostics.read().unwrap();
            let diagnostic = &diagnostics[session_id].snapshot;
            assert!(diagnostic
                .provider_id
                .as_ref()
                .unwrap()
                .chars()
                .count()
                <= MAX_DIAGNOSTIC_TEXT_CHARS);
            assert!(diagnostic
                .provider_model
                .as_ref()
                .unwrap()
                .chars()
                .count()
                <= MAX_DIAGNOSTIC_TEXT_CHARS);
        }

        state.peon.finish_attempt(session_id, &attempt);
        state
            .peon
            .in_flight
            .write()
            .unwrap()
            .insert(session_id.to_string());
        state.peon.mark_candidate(session_id);
        let next_attempt = state
            .peon
            .begin_attempt(session_id, test_runtime_identity(session_id, 1))
            .expect("second diagnostic attempt should start");
        state.peon.fail_attempt(session_id, &next_attempt, "provider_exhausted", &oversized);
        let error_summary = state.peon.diagnostics.read().unwrap()[session_id]
            .snapshot
            .error_summary
            .clone()
            .unwrap();
        assert!(error_summary.chars().count() <= MAX_DIAGNOSTIC_TEXT_CHARS);
        state.peon.finish_attempt(session_id, &next_attempt);
    }

    #[tokio::test]
    async fn input_label_inference_only_updates_the_live_label() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let id = "label-only".to_string();
        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: "unused".into(),
                    harness_args: vec![],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".into(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .stdout(r#"{"status":"working","summary":"New label","confidence":0.85}"#)],
            ),
        });
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: test_session_info(
                    id.clone(),
                    "Old label",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
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
                active_work_hook: false,
            },
        );
        state
            .peon
            .label_hint
            .write()
            .unwrap()
            .insert(
                id.clone(),
                crate::LabelHint {
                    text: "describe this task".into(),
                    epoch: 0,
                },
            );
        state.peon.label_pending.write().unwrap().insert(id.clone());

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        stop_test_peon_loop(task, shutdown).await;

        assert_eq!(state.sessions.lock().unwrap()[&id].info.label, "New label");
        assert!(!state.peon.in_flight.read().unwrap().contains(&id));
        assert!(!state.peon.last_inference.read().unwrap().contains_key(&id));
    }

    #[tokio::test]
    async fn input_label_request_survives_a_tick_where_the_session_is_already_in_flight() {
        let _lease_guard = diagnostic_test_guard();
        // Regression: peon_loop used to unconditionally drain label_pending
        // every tick; if the session already had an Output-mode inference
        // in_flight at that moment, the drained request was simply dropped
        // (never re-queued), so the one-shot InputLabel pass would silently
        // never run for that session.
        let dir = tempfile::tempdir().unwrap();
        let id = "label-survives-in-flight".to_string();
        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: "unused".into(),
                    harness_args: vec![],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".into(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .stdout(r#"{"status":"working","summary":"New label","confidence":0.85}"#)],
            ),
        });
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: test_session_info(
                    id.clone(),
                    "Old label",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
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
                active_work_hook: false,
            },
        );
        state
            .peon
            .label_hint
            .write()
            .unwrap()
            .insert(
                id.clone(),
                crate::LabelHint {
                    text: "describe this task".into(),
                    epoch: 0,
                },
            );
        state.peon.label_pending.write().unwrap().insert(id.clone());
        // Simulate an Output-mode inference already in flight for this
        // session at the moment the InputLabel request arrives.
        state.peon.in_flight.write().unwrap().insert(id.clone());

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

        assert_eq!(
            state.sessions.lock().unwrap()[&id].info.label,
            "Old label",
            "blocked by in_flight, so no inference should have run yet"
        );
        assert!(
            state.peon.label_pending.read().unwrap().contains(&id),
            "the request must be re-queued for a later tick, not dropped"
        );

        // The other inference finishes; the re-queued request should now go
        // through on a subsequent tick.
        state.peon.in_flight.write().unwrap().remove(&id);
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
        stop_test_peon_loop(task, shutdown).await;

        assert_eq!(state.sessions.lock().unwrap()[&id].info.label, "New label");
    }

    #[tokio::test]
    async fn input_label_inference_preserves_committed_working_attention() {
        let _lease_guard = diagnostic_test_guard();
        // A restart/reload must not lose the Peon-authored topic (ADR 0029) —
        // the live SessionInfo update alone isn't durable.
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();
        let id = "label-persist".to_string();
        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: "unused".into(),
                    harness_args: vec![],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".into(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode").stdout(
                    r#"{"status":"waiting_for_input","summary":"Persisted topic","confidence":0.85}"#,
                )],
            ),
        });
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(
            id.clone(),
            "Session labelpe",
            dir.path().display().to_string(),
            "running",
            "now",
        );
        info.observed_status = Some("working".into());
        info.attention = Some("working".into());
        info.metadata_source = Some("process".into());
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info,
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
                active_work_hook: false,
            },
        );
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            let mut meta = test_session_metadata(
                &id,
                "Session labelpe",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            meta.observed_status = Some("working".into());
            meta.attention = Some("working".into());
            meta.metadata_source = "process".into();
            ws.metadata.write_session(&meta);
        }
        state
            .peon
            .label_hint
            .write()
            .unwrap()
            .insert(
                id.clone(),
                crate::LabelHint {
                    text: "describe this task".into(),
                    epoch: 0,
                },
            );
        state.peon.label_pending.write().unwrap().insert(id.clone());

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        stop_test_peon_loop(task, shutdown).await;

        let info = state.sessions.lock().unwrap()[&id].info.clone();
        assert_eq!(info.label, "Persisted topic");
        assert_eq!(info.attention.as_deref(), Some("working"));
        let ws_guard = state.workspace.lock().unwrap();
        let meta = ws_guard
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&id)
            .unwrap();
        assert_eq!(meta.label, "Persisted topic");
        assert_eq!(meta.observed_status.as_deref(), Some("working"));
        assert_eq!(meta.attention.as_deref(), Some("working"));
    }

    #[tokio::test]
    async fn input_label_inference_rejects_a_blank_inferred_label() {
        let _lease_guard = diagnostic_test_guard();
        // Regression: an otherwise-valid inference with an empty/whitespace
        // summary must not durably blank out the synchronous fallback label
        // (the one-shot request would then be gone for good, per ADR 0029).
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();
        let id = "label-blank-rejected".to_string();
        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: "unused".into(),
                    harness_args: vec![],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".into(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .stdout(r#"{"status":"working","summary":"   ","confidence":0.85}"#)],
            ),
        });
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: test_session_info(
                    id.clone(),
                    "fallback from typed input",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
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
                active_work_hook: false,
            },
        );
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&test_session_metadata(
                &id,
                "fallback from typed input",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            ));
        }
        state
            .peon
            .label_hint
            .write()
            .unwrap()
            .insert(
                id.clone(),
                crate::LabelHint {
                    text: "describe this task".into(),
                    epoch: 0,
                },
            );
        state.peon.label_pending.write().unwrap().insert(id.clone());

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        stop_test_peon_loop(task, shutdown).await;

        assert_eq!(
            state.sessions.lock().unwrap()[&id].info.label,
            "fallback from typed input"
        );
        let ws_guard = state.workspace.lock().unwrap();
        let meta = ws_guard
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&id)
            .unwrap();
        assert_eq!(meta.label, "fallback from typed input");
    }

    #[tokio::test]
    async fn input_label_inference_rejects_a_pr_number_dropping_label() {
        let _lease_guard = diagnostic_test_guard();
        // A provider summary that drops a PR number from the typed input is
        // not descriptive enough to replace the synchronous fallback label.
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();
        let id = "label-pr-number-rejected".to_string();
        // PR #249 is beyond the display fallback. A number-dropping model
        // response must therefore leave the bounded fallback untouched.
        let input_hint = format!(
            "keep watching {}PR #249",
            "important changes ".repeat(6)
        );
        let fallback_label: String = input_hint.chars().take(100).collect();
        let call_counter = Arc::new(AtomicUsize::new(0));
        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: "unused".into(),
                    harness_args: vec![],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".into(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .stdout(
                        r#"{"status":"working","summary":"Monitoring pull request","confidence":0.85}"#,
                    )
                    .with_counter(call_counter.clone())],
            ),
        });
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info: test_session_info(
                    id.clone(),
                    &fallback_label,
                    dir.path().display().to_string(),
                    "running",
                    "now",
                ),
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
                active_work_hook: false,
            },
        );
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&test_session_metadata(
                &id,
                &fallback_label,
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            ));
        }
        state
            .peon
            .label_hint
            .write()
            .unwrap()
            .insert(
                id.clone(),
                crate::LabelHint {
                    text: input_hint,
                    epoch: 0,
                },
            );
        state.peon.label_pending.write().unwrap().insert(id.clone());

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if call_counter.load(Ordering::SeqCst) == 1
                    && !state.peon.in_flight.read().unwrap().contains(&id)
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("input label inference should complete");
        stop_test_peon_loop(task, shutdown).await;

        assert_eq!(
            state.sessions.lock().unwrap()[&id].info.label,
            fallback_label
        );
        let ws_guard = state.workspace.lock().unwrap();
        let meta = ws_guard
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&id)
            .unwrap();
        assert_eq!(meta.label, fallback_label);
    }

    #[tokio::test]
    async fn test_peon_inference_writes_metadata() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        // Create a mock harness script that echoes known JSON
        let harness_path = dir.path().join("mock-harness.sh");
        std::fs::write(&harness_path, "#!/bin/bash\necho '{\"status\":\"working\",\"summary\":\"test\",\"confidence\":0.85}'\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&harness_path, std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".to_string(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .stdout(r#"{"status":"working","summary":"test","confidence":0.85}"#)],
            ),
        });

        // Create a session with some output in the ring buffer
        let session_id = "peon-test-1".to_string();
        {
            let mut sessions = state.sessions.lock().unwrap();
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
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
                active_work_hook: false,
            };
            handle.output_buffer.push("$ cargo test".into());
            handle
                .output_buffer
                .push("test result: ok. 5 passed; 0 failed;".into());
            sessions.insert(session_id.clone(), handle);
        }

        // Write initial metadata
        {
            let ws = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
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
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
            }
        }

        // Set last_output to trigger inference (5s ago = past debounce interval)
        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        // Run peon_loop in background
        let (task, shutdown) = spawn_test_peon_loop(state.clone());

        // Wait for metadata to be updated (poll up to 10 seconds)
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let ws = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws {
                if let Some(meta) = ws.metadata.read_session("peon-test-1") {
                    if meta.metadata_source == "peon" {
                        // Verify metadata was updated correctly
                        assert_eq!(meta.status, "running");
                        assert_eq!(meta.observed_status, Some("working".into()));
                        assert_eq!(meta.summary, Some("Tests passed".into()));
                        assert_eq!(meta.peon_last_inference.is_some(), true);
                        assert_eq!(meta.metadata_source, "peon");
                        assert!((meta.metadata_confidence - 0.85).abs() < 0.001);
                        let diagnostics = state.peon.diagnostics.read().unwrap();
                        let diagnostic = diagnostics
                            .get("peon-test-1")
                            .expect("successful inference should have diagnostics");
                        assert_eq!(
                            diagnostic.snapshot.scheduler_state,
                            crate::session_types::PeonSchedulerState::Completed
                        );
                        assert!(diagnostic.snapshot.last_successful_inference_at.is_some());
                        assert_eq!(diagnostic.snapshot.provider_id.as_deref(), Some("opencode"));
                        assert_eq!(diagnostic.snapshot.attempt_count, Some(1));
                        drop(ws);
                        stop_test_peon_loop(task, shutdown).await;
                        return; // test passes
                    }
                }
            }
        }

        panic!("Peon did not update metadata within 10 seconds");
    }

    #[tokio::test]
    async fn peon_loop_does_not_start_duplicate_inference_while_in_flight() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let call_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".to_string(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .stdout(r#"{"observedStatus":"working","confidence":0.85}"#)
                    .sleep_ms(3000)
                    .with_counter(call_counter.clone())],
            ),
        });

        let session_id = "peon-duplicate-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
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
                active_work_hook: false,
            };
            handle.output_buffer.push("quiet output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        state.peon.last_output.write().unwrap().insert(
            session_id,
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        stop_test_peon_loop(task, shutdown).await;

        let count = call_counter.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn peon_loop_does_not_repeat_inference_without_new_output() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let call_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".to_string(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .stdout(r#"{"observedStatus":"working","summary":"still working","confidence":0.85}"#)
                    .with_counter(call_counter.clone())],
            ),
        });

        let session_id = "peon-no-repeat-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
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
                active_work_hook: false,
            };
            handle.output_buffer.push("unchanged output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        state.peon.last_output.write().unwrap().insert(
            session_id,
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2600)).await;
        stop_test_peon_loop(task, shutdown).await;

        let count = call_counter.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, 1, "Peon should not re-infer without new output");
    }

    #[tokio::test]
    async fn peon_loop_records_failed_inference_attempt() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 15,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::new(),
        });

        let session_id = "peon-failed-attempt-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
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
                active_work_hook: false,
            };
            handle.output_buffer.push("quiet output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        stop_test_peon_loop(task, shutdown).await;

        assert!(
            state
                .peon
                .last_inference
                .read()
                .unwrap()
                .contains_key(&session_id),
            "failed Peon attempts should still be recorded in last_inference"
        );
        let diagnostics = state.peon.diagnostics.read().unwrap();
        let diagnostic = diagnostics
            .get(&session_id)
            .expect("failed inference should have diagnostics");
        assert_eq!(
            diagnostic.snapshot.scheduler_state,
            crate::session_types::PeonSchedulerState::Failed
        );
        assert_eq!(diagnostic.snapshot.reason.as_deref(), Some("provider_exhausted"));
        assert!(diagnostic.snapshot.error_summary.is_some());
        assert_eq!(diagnostic.snapshot.attempt_count, Some(1));
    }

    #[tokio::test]
    async fn peon_loop_marks_idle_when_silent() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1, // fast idle detection for test
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-idle-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
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
                active_work_hook: false,
            };
            handle.output_buffer.push("some past output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        // Set last_output to 5 seconds ago (well past the 1s idle timeout)
        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        // Initialize session metadata so the idle timer can write observed_status.
        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
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
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
            }
        }

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        stop_test_peon_loop(task, shutdown).await;

        // Check metadata: observed_status should be "idle"
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status.as_deref(), Some("idle"));
                assert_eq!(meta.metadata_source, "process");
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }
    }

    #[tokio::test]
    async fn peon_loop_marks_silent_working_session_idle() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-working-idle-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut info = test_session_info(
                session_id.clone(),
                "Test",
                dir.path().display().to_string(),
                "running",
                "now",
            );
            info.lifecycle_phase = "active".into();
            info.metadata_source = Some("peon".into());
            info.metadata_confidence = Some(0.85);
            info.observed_status = Some("working".into());
            state.sessions.lock().unwrap().insert(
                session_id.clone(),
                crate::SessionHandle {
                    info,
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
                    active_work_hook: false,
                },
            );
        }

        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
                    attention: None,
                    plan_path: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: None,
                    observed_status: Some("working".into()),
                    ending_observed_status_snapshot: None,
                    final_observed_status_snapshot: None,
                    summary: Some("Still working".into()),
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    peon_last_inference: Some("before".into()),
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
        last_output_at: None,
                    metadata_source: "peon".into(),
                    metadata_confidence: 0.85,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
            }
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );
        state
            .peon
            .last_inference
            .write()
            .unwrap()
            .insert(session_id.clone(), "before".into());

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        stop_test_peon_loop(task, shutdown).await;

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status.as_deref(), Some("idle"));
                assert_eq!(meta.metadata_source, "process");
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions
                .get(&session_id)
                .unwrap()
                .info
                .observed_status
                .as_deref(),
            Some("idle")
        );
    }

    #[tokio::test]
    async fn peon_loop_does_not_mark_recently_started_silent_session_idle() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 5,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-recent-start-test".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(
            session_id.clone(),
            "Test",
            dir.path().display().to_string(),
            "creating",
            "now",
        );
        info.lifecycle_phase = "creating".into();
        info.metadata_source = Some("process".into());
        info.metadata_confidence = Some(1.0);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info,
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
                active_work_hook: false,
            },
        );

        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
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
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
            }
        }

        crate::runtime::terminal_runtime::set_session_status(&state, &session_id, "running").await;

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        stop_test_peon_loop(task, shutdown).await;

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status, None);
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions.get(&session_id).unwrap().info.observed_status,
            None
        );
    }

    #[tokio::test]
    async fn peon_loop_does_not_mark_running_session_without_last_output_idle() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 5,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-missing-last-output-test".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(
            session_id.clone(),
            "Test",
            dir.path().display().to_string(),
            "running",
            "now",
        );
        info.lifecycle_phase = "active".into();
        info.metadata_source = Some("process".into());
        info.metadata_confidence = Some(1.0);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info,
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
                active_work_hook: false,
            },
        );

        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
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
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
            }
        }

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        stop_test_peon_loop(task, shutdown).await;

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status, None);
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions.get(&session_id).unwrap().info.observed_status,
            None
        );
    }

    #[tokio::test]
    async fn peon_loop_eventually_marks_running_session_without_last_output_idle() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-missing-last-output-eventual-idle-test".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = test_session_info(
            session_id.clone(),
            "Test",
            dir.path().display().to_string(),
            "running",
            "now",
        );
        info.lifecycle_phase = "active".into();
        info.metadata_source = Some("process".into());
        info.metadata_confidence = Some(1.0);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info,
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
                active_work_hook: false,
            },
        );

        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
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
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
            }
        }

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        stop_test_peon_loop(task, shutdown).await;

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status.as_deref(), Some("idle"));
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions
                .get(&session_id)
                .unwrap()
                .info
                .observed_status
                .as_deref(),
            Some("idle"),
        );
    }

    #[tokio::test]
    async fn peon_loop_does_not_overwrite_existing_observed_status_with_idle() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            providers: providers::ProviderManager::new(),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-idle-no-overwrite-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    observed_status: Some("blocked".into()),
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
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
                active_work_hook: false,
            };
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "active".into(),
                    lifecycle: "alive".into(),
                    attention: None,
                    plan_path: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: None,
                    observed_status: Some("blocked".into()),
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
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
            }
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        stop_test_peon_loop(task, shutdown).await;

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert_eq!(meta.observed_status.as_deref(), Some("blocked"));
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }
    }

    #[tokio::test]
    async fn peon_loop_skips_sessions_in_ending_lifecycle() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(crate::WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                workflow_observations: crate::workflow_observations::WorkflowObservationStore::open(
                    orkworks.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: crate::taskmaster::store::RecommendationStore::open(
                    orkworks.clone(),
                )
                .expect("open recommendation store"),
                watcher: crate::watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".to_string(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode").stdout(
                    r#"{"status":"working","summary":"should-not-run","confidence":0.85}"#,
                )],
            ),
            bound_port: AtomicU16::new(0),
        });

        let session_id = "peon-ending-skip-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
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
                active_work_hook: false,
            };
            handle.info.lifecycle_phase = "ending".into();
            handle.output_buffer.push("finishing up".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        {
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Test".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "".into(),
                    model: "".into(),
                    cwd: dir.path().display().to_string(),
                    status: "running".into(),
                    work_phase: "unknown".into(),
                    lifecycle_phase: "ending".into(),
                    lifecycle: "stopping".into(),
                    attention: None,
                    plan_path: None,
                    connectivity: "online".into(),
                    terminal_outcome: None,
                    pending_terminal_status: Some("ended".into()),
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
                    provider_id: None,
                    provider_label: None,
                    provider_model: None,
                    provider_state: None,
                    created_at: "now".into(),
                    last_activity: "now".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: None,
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
            }
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        stop_test_peon_loop(task, shutdown).await;

        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(meta) = ws.metadata.read_session(&session_id) {
                assert!(meta.peon_last_inference.is_none());
            } else {
                panic!("session metadata not found");
            }
        } else {
            panic!("workspace not set up");
        }
    }

    // Regression: persist skipped must not drop session from candidate pool (issue #87).
    #[tokio::test]
    async fn peon_loop_retries_when_persist_skipped_despite_terminal_inference() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None), // no workspace → persist is always skipped
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec![],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            // FakeProvider returns "idle" → reached_terminal=true inside spawn_blocking
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".to_string(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .stdout(r#"{"status":"idle","confidence":0.85}"#)],
            ),
        });

        let session_id = "peon-retry-persist-skipped-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
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
                active_work_hook: false,
            };
            handle.output_buffer.push("some terminal output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), handle);
        }

        // Plant last_output 5s in the past — past the 1s interval, so session is
        // immediately eligible as a candidate.
        let before_test = tokio::time::Instant::now();
        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            before_test - std::time::Duration::from_secs(5),
        );

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        stop_test_peon_loop(task, shutdown).await;

        let lo = state.peon.last_output.read().unwrap();
        let updated_at = lo
            .get(&session_id)
            .copied()
            .expect("last_output entry removed");
        assert!(
            updated_at >= before_test,
            "last_output should be refreshed even when persist is skipped and inference was terminal; \
             session must remain eligible for retry, not silently exit the candidate pool"
        );
    }

    #[tokio::test]
    async fn peon_loop_runs_two_sessions_concurrently() {
        let _lease_guard = diagnostic_test_guard();
        let dir = tempfile::tempdir().unwrap();
        let entry_barrier = Arc::new(Barrier::new(3));
        let release_barrier = Arc::new(Barrier::new(3));

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: crate::PeonState {
                last_output: RwLock::new(HashMap::new()),
                last_inference: RwLock::new(HashMap::new()),
                in_flight: RwLock::new(HashSet::new()),
                label_hint: RwLock::new(HashMap::new()),
                label_pending: RwLock::new(HashSet::new()),
                label_epochs: RwLock::new(HashMap::new()),
                input_buf: RwLock::new(HashMap::new()),
                reported_cwd: RwLock::new(HashMap::new()),
                diagnostics: RwLock::new(HashMap::new()),
                config: peon::PeonConfig {
                    harness: "unused".into(),
                    harness_args: vec![],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 30,
                    final_scan_timeout_secs: 2,
                    enabled: true,
                },
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::for_tests(
                providers::ProviderSettingsPayload {
                    version: 1,
                    revision: 1,
                    peon_model: None,
                    peon_selection: None,
                    ollama_base_url: providers::default_ollama_base_url(),
                    providers: vec![providers::ProviderSettingsEntry {
                        id: "opencode".into(),
                        enabled: true,
                        fallback_order: 0,
                        model: None,
                        default_state: providers::ProviderCapacityState::Healthy,
                        override_state: None,
                    }],
                },
                vec![providers::FakeProvider::new("opencode")
                    .stdout(r#"{"observedStatus":"working","confidence":0.85}"#)
                    .with_barriers(entry_barrier.clone(), release_barrier.clone())],
            ),
        });

        let session_ids = ["peon-concurrent-a", "peon-concurrent-b"];
        for session_id in session_ids {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = crate::SessionHandle {
                info: crate::session_types::SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id,
                        "Test",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
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
                active_work_hook: false,
            };
            handle.output_buffer.push("quiet output".into());
            state
                .sessions
                .lock()
                .unwrap()
                .insert(session_id.to_string(), handle);
            state.peon.last_output.write().unwrap().insert(
                session_id.to_string(),
                tokio::time::Instant::now() - std::time::Duration::from_secs(5),
            );
        }

        let (task, shutdown) = spawn_test_peon_loop(state.clone());
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let entry_barrier = entry_barrier.clone();
            tokio::task::spawn_blocking(move || entry_barrier.wait())
                .await
                .expect("entry barrier waiter should complete");
        })
        .await
        .expect("both provider calls should enter before the test deadline");

        {
            let diagnostics = state.peon.diagnostics.read().unwrap();
            for session_id in session_ids {
                let diagnostic = diagnostics
                    .get(session_id)
                    .expect("eligible session should have diagnostic state");
                assert_eq!(
                    diagnostic.snapshot.scheduler_state,
                    crate::session_types::PeonSchedulerState::InFlight
                );
                assert_eq!(diagnostic.snapshot.attempt_count, Some(1));
            }
        }

        let release_barrier_waiter = release_barrier.clone();
        tokio::task::spawn_blocking(move || release_barrier_waiter.wait())
            .await
            .expect("release barrier waiter should complete");
        stop_test_peon_loop(task, shutdown).await;
    }
}
