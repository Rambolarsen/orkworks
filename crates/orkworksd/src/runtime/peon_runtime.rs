use crate::workspace_runtime::iso_now;
use crate::{peon, providers, AppState};
use std::sync::Arc;

const MAX_DIAGNOSTIC_TEXT_CHARS: usize = 240;

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

impl crate::PeonState {
    fn diagnostic_entry<'a>(
        &self,
        diagnostics: &'a mut std::collections::HashMap<String, crate::PeonDiagnosticEntry>,
        session_id: &str,
    ) -> &'a mut crate::PeonDiagnosticEntry {
        if !diagnostics.contains_key(session_id) {
            if diagnostics.len() >= crate::MAX_PEON_DIAGNOSTIC_SESSIONS {
                let evictable_id = {
                    let in_flight = self.in_flight.read().unwrap();
                    diagnostics
                        .keys()
                        .find(|id| !in_flight.contains(*id))
                        .cloned()
                };
                if let Some(oldest_id) = evictable_id {
                    diagnostics.remove(&oldest_id);
                }
            }
            diagnostics.insert(
                session_id.to_string(),
                crate::PeonDiagnosticEntry::new(),
            );
        }
        diagnostics
            .get_mut(session_id)
            .expect("diagnostic entry was inserted")
    }

    fn mark_candidate(&self, session_id: &str) {
        let mut diagnostics = self.diagnostics.write().unwrap();
        let entry = self.diagnostic_entry(&mut diagnostics, session_id);
        entry.snapshot.scheduler_state = crate::session_types::PeonSchedulerState::Candidate;
        entry.snapshot.reason = Some("selected_for_inference".to_string());
    }

    fn begin_attempt(&self, session_id: &str) -> Option<u64> {
        let mut diagnostics = self.diagnostics.write().unwrap();
        let entry = self.diagnostic_entry(&mut diagnostics, session_id);
        if entry.snapshot.scheduler_state == crate::session_types::PeonSchedulerState::InFlight {
            return None;
        }
        entry.attempt_generation = entry.attempt_generation.saturating_add(1);
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
        Some(entry.attempt_generation)
    }

    fn complete_attempt(
        &self,
        session_id: &str,
        generation: u64,
        result: &providers::ProviderRunResult,
    ) -> bool {
        if result.inference.is_none() {
            return false;
        }
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = diagnostics.get_mut(session_id) else {
            return false;
        };
        if entry.attempt_generation != generation
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
        generation: u64,
        reason: &str,
        error: &str,
    ) -> bool {
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = diagnostics.get_mut(session_id) else {
            return false;
        };
        if entry.attempt_generation != generation
            || entry.snapshot.scheduler_state != crate::session_types::PeonSchedulerState::InFlight
        {
            return false;
        }
        entry.snapshot.scheduler_state = crate::session_types::PeonSchedulerState::Failed;
        entry.snapshot.reason = Some(bounded_diagnostic_text(reason));
        entry.snapshot.error_summary = Some(bounded_error_summary(error));
        true
    }

    fn timeout_attempt(&self, session_id: &str, generation: u64) {
        if self.fail_attempt(session_id, generation, "timeout", "provider inference timed out") {
            self.in_flight.write().unwrap().remove(session_id);
        }
    }

    fn finish_attempt(&self, session_id: &str, generation: u64) {
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = diagnostics.get_mut(session_id) else {
            return;
        };
        if entry.attempt_generation != generation {
            return;
        }
        self.in_flight.write().unwrap().remove(session_id);
    }

    fn mark_idle(&self, session_id: &str, reason: &str) {
        let mut diagnostics = self.diagnostics.write().unwrap();
        let entry = self.diagnostic_entry(&mut diagnostics, session_id);
        if entry.snapshot.scheduler_state != crate::session_types::PeonSchedulerState::InFlight {
            entry.snapshot.scheduler_state = crate::session_types::PeonSchedulerState::Idle;
            entry.snapshot.reason = Some(reason.to_string());
        }
    }

    fn refresh_observation_count(
        &self,
        session_id: &str,
        generation: u64,
        count: Option<usize>,
    ) {
        let mut diagnostics = self.diagnostics.write().unwrap();
        let Some(entry) = diagnostics.get_mut(session_id) else {
            return;
        };
        if entry.attempt_generation == generation {
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

pub(crate) async fn peon_loop(state: Arc<AppState>) {
    let interval = state.peon.config.interval_secs;
    tracing::info!(interval_secs = interval, harness = %state.peon.config.harness, "peon started");

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

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
            let Some(attempt_generation) = state.peon.begin_attempt(&id) else {
                state.peon.in_flight.write().unwrap().remove(&id);
                continue;
            };
            tokio::spawn(async move {
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
                    if cleanup_state.peon.fail_attempt(
                        &id,
                        attempt_generation,
                        "provider_task_failed",
                        &error.to_string(),
                    ) {
                        cleanup_state.peon.finish_attempt(&id, attempt_generation);
                    }
                    return;
                }
                Err(_) => {
                    tracing::warn!(session_id = %id, "peon inference timed out");
                    cleanup_state
                        .peon
                        .timeout_attempt(&id, attempt_generation);
                    let wait_state = cleanup_state.clone();
                    let wait_id = id.clone();
                    tokio::spawn(async move {
                        let _ = provider_task.await;
                        wait_state.peon.finish_attempt(&wait_id, attempt_generation);
                    });
                    return;
                }
            };

            if provider_result.inference.is_some() {
                if !state_clone
                    .peon
                    .complete_attempt(&id, attempt_generation, &provider_result)
                {
                    return;
                }
            } else {
                state_clone.peon.fail_attempt(
                    &id,
                    attempt_generation,
                    "provider_exhausted",
                    &provider_error_summary(&provider_result),
                );
            }

            let _ = tokio::task::spawn_blocking(move || {
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
                        if let Some(hint) = hint.as_ref() {
                            crate::session_application::SessionApplication::new(state_clone.clone())
                                .persist_input_label(&id, label, hint.epoch);
                        }
                    }
                    }
                    state_clone.peon.finish_attempt(&id, attempt_generation);
                    return;
                }
                let active_work_hook = {
                    let sessions = state_clone.sessions.lock().unwrap();
                    sessions.get(&id).and_then(|handle| {
                        let (generation, min_revision, _, _, _) = *output_boundary.as_ref()?;
                        output_inference_is_current(
                            generation,
                            min_revision,
                            handle.runtime.input_generation,
                            handle.runtime.min_peon_output_revision,
                        )
                        .then_some(handle.active_work_hook)
                    })
                };
                let Some(active_work_hook) = active_work_hook else {
                    state_clone.peon.finish_attempt(&id, attempt_generation);
                    return;
                };
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
                .persist_peon_observation(
                    &id,
                    inference.as_ref(),
                    provider_result.observation.as_ref(),
                    history_summary.as_deref(),
                    &now_iso,
                );
                let inference_persisted = persistence.inference_persisted;
                let permanent_hold = persistence.permanent_hold;
                let label_update = persistence.label_update;
                let captured_workspace_path = persistence.workspace_path;
                if let Some(inf) = inference.as_ref() {
                    if let Some((generation, _min_revision, first_revision, last_revision, runtime_instance_id)) =
                        output_boundary.as_ref()
                    {
                        let result = crate::session_application::SessionApplication::new(
                            state_clone.clone(),
                        )
                        .record_peon_workflow_observations(
                            &id,
                            captured_workspace_path.as_deref(),
                            &crate::session_application::PeonObservationOutputRange {
                                runtime_instance_id: runtime_instance_id.clone(),
                                run_generation: *generation,
                                first_revision: *first_revision,
                                last_revision: *last_revision,
                            },
                            &inf.workflow_observations,
                        );
                        accepted_observation = result.accepted_observation;
                        output_range_completed = result.output_range_completed;
                    }
                }

                if accepted_observation {
                    crate::taskmaster::evaluator::schedule_evaluation(state_clone.clone());
                }
                if output_range_completed {
                    if let Some((generation, min_revision, _, last_revision, _)) = output_boundary {
                        if let Some(handle) = state_clone.sessions.lock().unwrap().get_mut(&id) {
                            if output_inference_is_current(
                                generation,
                                min_revision,
                                handle.runtime.input_generation,
                                handle.runtime.min_peon_output_revision,
                            ) {
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
                    state_clone.peon.refresh_observation_count(
                        &id,
                        attempt_generation,
                        observation_count,
                    );
                }
                if let Some(label) = label_update {
                    if let Some(handle) = state_clone.sessions.lock().unwrap().get_mut(&id) {
                        handle.info.label = label;
                    }
                }

                let mut last_inf = state_clone.peon.last_inference.write().unwrap();
                last_inf.insert(id.clone(), now_iso);
                drop(last_inf);

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
                state_clone.peon.finish_attempt(&id, attempt_generation);
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
    fn stale_attempt_cleanup_cannot_release_a_newer_session_lease() {
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
        let first_generation = state
            .peon
            .begin_attempt(session_id)
            .expect("first attempt should start");
        state
            .peon
            .timeout_attempt(session_id, first_generation);

        state
            .peon
            .in_flight
            .write()
            .unwrap()
            .insert(session_id.to_string());
        state.peon.mark_candidate(session_id);
        let second_generation = state
            .peon
            .begin_attempt(session_id)
            .expect("second attempt should start");
        assert!(second_generation > first_generation);

        state.peon.finish_attempt(session_id, first_generation);
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

        state.peon.finish_attempt(session_id, second_generation);
        assert!(!state
            .peon
            .in_flight
            .read()
            .unwrap()
            .contains(session_id));
    }

    #[test]
    fn diagnostic_eviction_preserves_all_in_flight_entries_and_leases() {
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
            state
                .peon
                .begin_attempt(&session_id)
                .expect("in-flight diagnostic should start");
        }

        state.peon.mark_candidate("new-diagnostic");

        let diagnostics = state.peon.diagnostics.read().unwrap();
        let in_flight = state.peon.in_flight.read().unwrap();
        for index in 0..crate::MAX_PEON_DIAGNOSTIC_SESSIONS {
            let session_id = format!("in-flight-{index}");
            assert!(diagnostics.contains_key(&session_id));
            assert!(in_flight.contains(&session_id));
        }
    }

    #[test]
    fn provider_exhaustion_keeps_lease_until_post_processing_finishes() {
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
        let generation = state
            .peon
            .begin_attempt(session_id)
            .expect("provider exhaustion attempt should start");

        state
            .peon
            .fail_attempt(session_id, generation, "provider_exhausted", "all providers failed");

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
            crate::session_types::PeonSchedulerState::Failed
        );

        state.peon.finish_attempt(session_id, generation);
        assert!(!state
            .peon
            .in_flight
            .read()
            .unwrap()
            .contains(session_id));
    }

    #[test]
    fn diagnostic_provider_and_error_strings_are_bounded() {
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
        let generation = state
            .peon
            .begin_attempt(session_id)
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

        assert!(state.peon.complete_attempt(session_id, generation, &result));
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

        state.peon.finish_attempt(session_id, generation);
        state
            .peon
            .in_flight
            .write()
            .unwrap()
            .insert(session_id.to_string());
        state.peon.mark_candidate(session_id);
        let next_generation = state
            .peon
            .begin_attempt(session_id)
            .expect("second diagnostic attempt should start");
        state.peon.fail_attempt(session_id, next_generation, "provider_exhausted", &oversized);
        let error_summary = state.peon.diagnostics.read().unwrap()[session_id]
            .snapshot
            .error_summary
            .clone()
            .unwrap();
        assert!(error_summary.chars().count() <= MAX_DIAGNOSTIC_TEXT_CHARS);
        state.peon.finish_attempt(session_id, next_generation);
    }

    #[tokio::test]
    async fn input_label_inference_only_updates_the_live_label() {
        let dir = tempfile::tempdir().unwrap();
        let id = "label-only".to_string();
        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        task.abort();

        assert_eq!(state.sessions.lock().unwrap()[&id].info.label, "New label");
        assert!(!state.peon.in_flight.read().unwrap().contains(&id));
        assert!(!state.peon.last_inference.read().unwrap().contains_key(&id));
    }

    #[tokio::test]
    async fn input_label_request_survives_a_tick_where_the_session_is_already_in_flight() {
        // Regression: peon_loop used to unconditionally drain label_pending
        // every tick; if the session already had an Output-mode inference
        // in_flight at that moment, the drained request was simply dropped
        // (never re-queued), so the one-shot InputLabel pass would silently
        // never run for that session.
        let dir = tempfile::tempdir().unwrap();
        let id = "label-survives-in-flight".to_string();
        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
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
        task.abort();

        assert_eq!(state.sessions.lock().unwrap()[&id].info.label, "New label");
    }

    #[tokio::test]
    async fn input_label_inference_preserves_committed_working_attention() {
        // A restart/reload must not lose the Peon-authored topic (ADR 0029) —
        // the live SessionInfo update alone isn't durable.
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();
        let id = "label-persist".to_string();
        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        task.abort();

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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        task.abort();

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

        let task = tokio::spawn(peon_loop(state.clone()));
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
        task.abort();

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
        tokio::spawn(peon_loop(state.clone()));

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
                        return; // test passes
                    }
                }
            }
        }

        panic!("Peon did not update metadata within 10 seconds");
    }

    #[tokio::test]
    async fn peon_loop_does_not_start_duplicate_inference_while_in_flight() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let call_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2300)).await;
        task.abort();

        let count = call_counter.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn peon_loop_does_not_repeat_inference_without_new_output() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let call_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2600)).await;
        task.abort();

        let count = call_counter.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, 1, "Peon should not re-infer without new output");
    }

    #[tokio::test]
    async fn peon_loop_records_failed_inference_attempt() {
        let dir = tempfile::tempdir().unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

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
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

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
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

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
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

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
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

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
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

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
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

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
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        task.abort();

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
        let dir = tempfile::tempdir().unwrap();

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

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
        let dir = tempfile::tempdir().unwrap();
        let entry_barrier = Arc::new(Barrier::new(3));
        let release_barrier = Arc::new(Barrier::new(3));

        let state = Arc::new(crate::AppState {
            sessions: Mutex::new(HashMap::new()),
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

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let entry_barrier = entry_barrier.clone();
            tokio::task::spawn_blocking(move || entry_barrier.wait())
                .await
                .expect("entry barrier waiter should complete");
        })
        .await
        .expect("both provider calls should enter before the test deadline");

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

        let release_barrier_waiter = release_barrier.clone();
        tokio::task::spawn_blocking(move || release_barrier_waiter.wait())
            .await
            .expect("release barrier waiter should complete");
        task.abort();
    }
}
