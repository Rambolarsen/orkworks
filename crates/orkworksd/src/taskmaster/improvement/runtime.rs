//! Workspace-bound manual investigation and publication ownership.
use super::{
    access::{self, Capture},
    engine,
    model::RUN_TIMEOUT,
    publication,
    store::{self, Phase, Record, ReportStore, RunState},
};
use crate::taskmaster::runtime::TaskmasterRuntime;
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};

// Activation is deliberately code-owned. Signed delivery and paired usefulness
// qualification must both be established before changing this constant.
const PRODUCTION_QUALIFIED: bool = false;
const MONITOR_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone)]
struct Active {
    id: String,
    instance: u64,
    cancelled: Arc<AtomicBool>,
    user_cancelled: Arc<AtomicBool>,
    inputs_changed: Arc<AtomicBool>,
    settled: Arc<AtomicBool>,
    dispatch: Arc<Mutex<()>>,
}
static ACTIVE: OnceLock<Mutex<HashMap<PathBuf, Active>>> = OnceLock::new();
fn active() -> &'static Mutex<HashMap<PathBuf, Active>> {
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}

struct ActiveGuard {
    root: PathBuf,
    id: String,
}
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        if let Ok(mut runs) = active().lock() {
            if runs.get(&self.root).is_some_and(|run| run.id == self.id) {
                runs.remove(&self.root);
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Admission {
    pub outcome: &'static str,
    pub run_id: Option<String>,
    pub reason_code: Option<&'static str>,
}
fn admission(
    outcome: &'static str,
    run_id: Option<String>,
    reason_code: Option<&'static str>,
) -> Admission {
    Admission {
        outcome,
        run_id,
        reason_code,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Cancellation {
    pub outcome: &'static str,
    pub run_id: Option<String>,
    pub reason_code: Option<&'static str>,
}

pub(crate) fn cancel(state: &Arc<crate::AppState>, id: &str) -> Cancellation {
    let selected = state.workspace.lock().ok().and_then(|w| {
        w.as_ref().map(|w| {
            (
                w.metadata.root_path(),
                w.workflow_observations.instance_id(),
            )
        })
    });
    let Some((root, instance)) = selected else {
        return Cancellation {
            outcome: "unavailable",
            run_id: Some(id.into()),
            reason_code: Some("workspace_unavailable"),
        };
    };
    let run = match active().lock() {
        Ok(runs) => runs
            .get(&root)
            .filter(|run| run.id == id && run.instance == instance)
            .cloned(),
        Err(_) => {
            return Cancellation {
                outcome: "unavailable",
                run_id: Some(id.into()),
                reason_code: Some("runtime_unavailable"),
            }
        }
    };
    let Some(run) = run else {
        return Cancellation {
            outcome: "not_running",
            run_id: Some(id.into()),
            reason_code: None,
        };
    };
    let Ok(_dispatch) = run.dispatch.lock() else {
        return Cancellation {
            outcome: "unavailable",
            run_id: Some(id.into()),
            reason_code: Some("runtime_unavailable"),
        };
    };
    if run.settled.load(Ordering::SeqCst) {
        return Cancellation {
            outcome: "not_running",
            run_id: Some(id.into()),
            reason_code: None,
        };
    }
    run.user_cancelled.store(true, Ordering::SeqCst);
    run.cancelled.store(true, Ordering::SeqCst);
    Cancellation {
        outcome: "cancel_requested",
        run_id: Some(id.into()),
        reason_code: None,
    }
}

type FixtureInvoke = Box<dyn FnMut(&str, Duration) -> Result<String, String> + Send>;

pub(crate) fn schedule(state: Arc<crate::AppState>, runtime_root: PathBuf) -> Admission {
    if !PRODUCTION_QUALIFIED {
        return admission("unavailable", None, Some("qualification_disabled"));
    }
    if tokio::runtime::Handle::try_current().is_err() {
        return admission("unavailable", None, Some("runtime_unavailable"));
    }
    schedule_inner(state, Arc::new(TaskmasterRuntime::open(runtime_root)), None)
}

#[cfg(test)]
fn schedule_qualified(
    state: Arc<crate::AppState>,
    runtime: Arc<TaskmasterRuntime>,
    invoke: FixtureInvoke,
) -> Admission {
    schedule_inner(state, runtime, Some(invoke))
}

fn schedule_inner(
    state: Arc<crate::AppState>,
    runtime: Arc<TaskmasterRuntime>,
    invoke: Option<FixtureInvoke>,
) -> Admission {
    let started = Instant::now();
    let lease = match runtime.try_analysis_lease() {
        Ok(Some(lease)) => lease,
        Ok(None) => return admission("already_running", None, Some("analysis_busy")),
        Err(_) => return admission("unavailable", None, Some("metadata_unavailable")),
    };
    let capture = match access::capture(&state, &runtime, None) {
        Ok(capture) => capture,
        Err(_) => {
            return admission(
                "unavailable",
                None,
                Some("guidance_or_provider_unavailable"),
            )
        }
    };
    if recover_under_lease(&state, &runtime, &capture.root).is_err() {
        return admission("unavailable", None, Some("recovery_unavailable"));
    }
    // Preserve the report that owns an already-actionable improvement card.
    // A stale report does not block a fresh investigation.
    let current = status_with_runtime(&state, &runtime, true);
    let active_id = current.run.and_then(|run| run.report).and_then(|report| {
        report
            .get("recommendationId")
            .and_then(|id| id.as_str())
            .map(str::to_owned)
    });
    if let Some(id) = active_id {
        let is_active = state
            .workspace
            .lock()
            .ok()
            .and_then(|lock| {
                lock.as_ref()
                    .and_then(|workspace| workspace.recommendation_store.get(&id).ok().flatten())
            })
            .is_some_and(|item| {
                matches!(
                    item.status,
                    crate::taskmaster::RecommendationStatus::Proposed
                        | crate::taskmaster::RecommendationStatus::Accepted
                        | crate::taskmaster::RecommendationStatus::Executing
                )
            });
        if is_active {
            return admission("unavailable", None, Some("active_recommendation"));
        }
    }
    // Recovery can publish a recommendation. Capture all inputs again before admission.
    let capture = match access::capture(&state, &runtime, None) {
        Ok(capture) => capture,
        Err(_) => return admission("unavailable", None, Some("inputs_changed")),
    };
    let id = format!("improvement-{}", uuid::Uuid::new_v4());
    let record = Record::queued(&id, &crate::taskmaster::readiness_store::now());
    let entry = Active {
        id: id.clone(),
        instance: capture.instance,
        cancelled: Arc::new(AtomicBool::new(false)),
        user_cancelled: Arc::new(AtomicBool::new(false)),
        inputs_changed: Arc::new(AtomicBool::new(false)),
        settled: Arc::new(AtomicBool::new(false)),
        dispatch: Arc::new(Mutex::new(())),
    };
    {
        let Ok(mut runs) = active().lock() else {
            return admission("unavailable", None, Some("runtime_unavailable"));
        };
        if runs.contains_key(&capture.root) {
            return admission("already_running", None, Some("analysis_busy"));
        }
        runs.insert(capture.root.clone(), entry.clone());
    }
    let owner = ActiveGuard {
        root: capture.root.clone(),
        id: id.clone(),
    };
    if access::guarded(&state, &runtime, &capture, None, None, |_| {
        ReportStore::new(&capture.root).save(&record)
    })
    .is_err()
    {
        return admission("unavailable", None, Some("inputs_changed"));
    }
    let run_id = id.clone();
    tokio::task::spawn_blocking(move || {
        let _lease = lease;
        let _active = owner;
        execute(state, runtime, capture, record, entry, started, invoke);
    });
    admission("scheduled", Some(run_id), None)
}

fn monitor(
    state: Arc<crate::AppState>,
    runtime: Arc<TaskmasterRuntime>,
    capture: Capture,
    entry: Active,
    stop: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            let mut selected = false;
            let valid = runtime
                .with_current_evaluation(
                    &state.harness_store,
                    &capture.path,
                    &capture.evaluation,
                    || {
                        selected = state
                            .workspace
                            .lock()
                            .ok()
                            .and_then(|w| {
                                w.as_ref().map(|w| {
                                    w.path == capture.path
                                        && w.metadata.root_path() == capture.root
                                        && w.workflow_observations.instance_id() == capture.instance
                                })
                            })
                            .unwrap_or(false);
                    },
                )
                .unwrap_or(false);
            if !valid || !selected {
                if let Ok(_dispatch) = entry.dispatch.lock() {
                    if !entry.settled.load(Ordering::SeqCst) {
                        entry.inputs_changed.store(true, Ordering::SeqCst);
                        entry.cancelled.store(true, Ordering::SeqCst);
                    }
                }
                break;
            }
            std::thread::park_timeout(MONITOR_INTERVAL);
        }
    })
}

struct MonitorGuard {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for MonitorGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

fn settle_failure(entry: &Active, deadline: Instant) -> (&'static str, bool) {
    let Ok(_dispatch) = entry.dispatch.lock() else {
        entry.settled.store(true, Ordering::SeqCst);
        return ("investigation_failed", false);
    };
    let disposition = if entry.user_cancelled.load(Ordering::SeqCst) {
        ("cancelled", true)
    } else if entry.inputs_changed.load(Ordering::SeqCst) {
        ("inputs_changed", true)
    } else if Instant::now() >= deadline {
        ("deadline", true)
    } else {
        ("investigation_failed", false)
    };
    entry.settled.store(true, Ordering::SeqCst);
    disposition
}

fn finish_failure(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    capture: &Capture,
    id: &str,
    code: &'static str,
    interrupted: bool,
) {
    let _ = runtime.with_workspace_settings(&capture.path, |_| {
        let lock = state
            .workspace
            .lock()
            .map_err(|_| "Workspace unavailable")?;
        let _workspace = lock
            .as_ref()
            .filter(|w| {
                w.path == capture.path
                    && w.metadata.root_path() == capture.root
                    && w.workflow_observations.instance_id() == capture.instance
            })
            .ok_or("Workspace changed")?;
        let store = ReportStore::new(&capture.root);
        if let Some(mut record) = store.load()? {
            if record.id == id && matches!(record.state, RunState::Queued | RunState::Running) {
                record.state = if interrupted {
                    RunState::Interrupted
                } else {
                    RunState::Failed
                };
                record.phase = None;
                record.completed_at = Some(crate::taskmaster::readiness_store::now());
                record.error_code = Some(code.into());
                record.report = None;
                record.pending_recommendation = None;
                store.save(&record)?;
            }
        }
        Ok::<(), String>(())
    });
}

fn recover_under_lease(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    root: &Path,
) -> Result<(), String> {
    let store = ReportStore::new(root);
    let Some(mut record) = store.load()? else {
        return Ok(());
    };
    match record.state {
        RunState::Queued | RunState::Running => {
            record.state = RunState::Interrupted;
            record.phase = None;
            record.completed_at = Some(crate::taskmaster::readiness_store::now());
            record.error_code = Some("interrupted".into());
            store.save(&record)
        }
        RunState::PendingRecommendation => {
            let report = record.report.as_ref().ok_or("Pending report unavailable")?;
            let rec_id = report
                .recommendation_id
                .clone()
                .ok_or("Pending recommendation unavailable")?;
            let capture = access::capture(state, runtime, Some(&rec_id))?;
            if capture.root != root {
                return Err("Workspace changed".into());
            }
            let manifest = report.manifest.clone();
            let mut entered = false;
            let recovered = access::guarded(
                state,
                runtime,
                &capture,
                Some(&manifest),
                Some(&rec_id),
                |workspace| {
                    publication::current_report(
                        record.report.as_ref().ok_or("Pending report unavailable")?,
                        &capture,
                    )?;
                    entered = true;
                    store::recover_with(
                        &mut record,
                        |value| store.save(value),
                        |recommendation| {
                            crate::taskmaster::assessment::upsert_assessment_recommendation(
                                workspace,
                                recommendation,
                            )
                        },
                    )?;
                    publish_history(&capture, &mut record, &store)
                },
            );
            match recovered {
                Err(ref reason) if !entered && reason.contains("changed") => runtime
                    .with_workspace_settings(&capture.path, |_| {
                        let lock = state
                            .workspace
                            .lock()
                            .map_err(|_| "Workspace unavailable")?;
                        let _workspace = lock
                            .as_ref()
                            .filter(|w| {
                                w.path == capture.path
                                    && w.metadata.root_path() == capture.root
                                    && w.workflow_observations.instance_id() == capture.instance
                            })
                            .ok_or("Workspace changed")?;
                        if store.load()?.as_ref() != Some(&record) {
                            return Err("Pending report changed".into());
                        }
                        record.state = RunState::Interrupted;
                        record.phase = None;
                        record.completed_at = Some(crate::taskmaster::readiness_store::now());
                        record.error_code = Some("inputs_changed".into());
                        record.report = None;
                        record.pending_recommendation = None;
                        store.save(&record)
                    })?,
                other => other,
            }
        }
        RunState::Succeeded
            if record
                .report
                .as_ref()
                .is_some_and(|r| r.history_error.as_deref() == Some("history_pending")) =>
        {
            let report = record.report.as_ref().ok_or("Report unavailable")?;
            let recommendation_id = report.recommendation_id.clone();
            let manifest = report.manifest.clone();
            let capture = access::capture(state, runtime, recommendation_id.as_deref())?;
            if capture.root != root {
                return Err("Workspace changed".into());
            }
            publication::current_report(report, &capture)?;
            access::guarded(
                state,
                runtime,
                &capture,
                Some(&manifest),
                recommendation_id.as_deref(),
                |_| publish_history(&capture, &mut record, &store),
            )
        }
        _ => Ok(()),
    }
}

fn publish_history(
    capture: &Capture,
    record: &mut Record,
    store: &ReportStore,
) -> Result<(), String> {
    let report = record.report.as_ref().ok_or("Report unavailable")?;
    let snapshot = super::history::snapshot(capture, record, report);
    let mut updated = record.clone();
    let report = updated.report.as_mut().ok_or("Report unavailable")?;
    match snapshot.and_then(|snapshot| {
        let id = snapshot.id.clone();
        crate::taskmaster::readiness_store::ReadinessStore::new(&capture.root).append(snapshot)?;
        Ok(id)
    }) {
        Ok(id) => {
            report.history_snapshot_id = Some(id);
            report.history_error = None;
        }
        Err(_) => {
            report.history_snapshot_id = None;
            report.history_error = Some("history_unavailable".into());
        }
    }
    store.save(&updated)?;
    *record = updated;
    Ok(())
}

// Lock order is runtime inference authority -> per-run dispatch -> workspace.
// Custom transports enter here with inference authority already held through spawn.
fn authorized_dispatch<T>(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    capture: &Capture,
    manifest: &store::Manifest,
    entry: &Active,
    deadline: Instant,
    authority_held: bool,
    apply: impl FnOnce(&crate::WorkspaceState) -> Result<T, String>,
) -> Result<T, String> {
    let mut apply = Some(apply);
    let mut inner = || {
        let _dispatch = entry.dispatch.lock().map_err(|_| "runtime_unavailable")?;
        if entry.settled.load(Ordering::SeqCst)
            || entry.cancelled.load(Ordering::SeqCst)
            || Instant::now() >= deadline
        {
            return Err("cancelled".into());
        }
        access::guarded_workspace(
            state,
            capture,
            Some(manifest),
            None,
            apply.take().ok_or("dispatch_unavailable")?,
        )
    };
    if authority_held {
        return inner();
    }
    let mut outcome = None;
    let valid = runtime.with_current_evaluation(
        &state.harness_store,
        &capture.path,
        &capture.evaluation,
        || {
            outcome = Some(inner());
        },
    )?;
    if !valid {
        return Err("inputs_changed".into());
    }
    outcome.ok_or("dispatch_unavailable")?
}

fn controlled_invoke(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    capture: &Capture,
    manifest: &store::Manifest,
    entry: &Active,
    deadline: Instant,
    prompt: &str,
    final_call: bool,
) -> Result<String, String> {
    use crate::providers::{
        native_inference::{NativeProfile, NativeResponseShape},
        InferenceControl,
    };
    let selection = capture
        .evaluation
        .settings
        .selection
        .as_ref()
        .ok_or("provider_unavailable")?;
    let control = InferenceControl::new(deadline, entry.cancelled.clone());
    if control.is_cancelled() || control.remaining().is_zero() {
        return Err("cancelled".into());
    }
    let custom = capture.evaluation.custom_inference.is_some();
    let mut gate = |start: &mut dyn FnMut() -> std::io::Result<()>| {
        authorized_dispatch(
            state,
            runtime,
            capture,
            manifest,
            entry,
            deadline,
            custom,
            |_| Ok(start()),
        )
        .ok()
    };
    let result = if let Some(captured) = capture.evaluation.custom_inference.as_ref() {
        runtime.invoke_custom_inference_controlled(
            &state.harness_store,
            captured,
            prompt.into(),
            &mut gate,
            &control,
        )
    } else if let Some(native) = capture.evaluation.native_revision.as_ref() {
        state.providers.invoke_native_taskmaster_prompt_controlled(
            native.profile,
            &selection.model,
            selection.reasoning_effort.as_deref(),
            selection.ollama_base_url.as_deref(),
            prompt.into(),
            (native.profile == NativeProfile::Codex).then_some(if final_call {
                NativeResponseShape::ImprovementFinalV1
            } else {
                NativeResponseShape::ImprovementEvidenceV1
            }),
            &mut gate,
            &control,
        )
    } else {
        return Err("provider_unavailable".into());
    };
    result.map_err(|error| crate::taskmaster::assessment_failure::provider_code(&error).to_owned())
}

fn execute(
    state: Arc<crate::AppState>,
    runtime: Arc<TaskmasterRuntime>,
    capture: Capture,
    mut record: Record,
    entry: Active,
    started: Instant,
    mut fixture: Option<FixtureInvoke>,
) {
    let deadline = started.checked_add(RUN_TIMEOUT).unwrap_or(started);
    let stop = Arc::new(AtomicBool::new(false));
    let monitor = MonitorGuard {
        stop: stop.clone(),
        thread: Some(monitor(
            state.clone(),
            runtime.clone(),
            capture.clone(),
            entry.clone(),
            stop,
        )),
    };
    let result = (|| -> Result<(), String> {
        let cancelled = entry.cancelled.clone();
        let evidence = crate::taskmaster::assessment_context::improvement::ProgressiveEvidence::collect_controlled(
            &capture.path,
            capture.evaluation.settings.context_level,
            &capture.evaluation.settings.excluded_paths,
            &record.started_at,
            move || {
                if cancelled.load(Ordering::SeqCst) || Instant::now() >= deadline {
                    Err("cancelled".into())
                } else {
                    Ok(())
                }
            },
        )?;
        record.state = RunState::Running;
        record.phase = Some(Phase::ReadingEvidence);
        access::guarded(&state, &runtime, &capture, None, None, |_| {
            ReportStore::new(&capture.root).save(&record)
        })?;
        let bundle = capture
            .evaluation
            .knowledge
            .as_ref()
            .ok_or("guidance_unavailable")?;
        let mut invoke = |invocation: engine::Invocation<'_>| -> Result<String, String> {
            let manifest = access::supplied_manifest(
                &capture,
                invocation.evidence,
                invocation.selected_page_ids,
            )?;
            if entry.cancelled.load(Ordering::SeqCst) || Instant::now() >= deadline {
                return Err("cancelled".into());
            }
            record.phase = Some(match invocation.phase {
                engine::InvocationPhase::Evidence => Phase::ReadingEvidence,
                engine::InvocationPhase::Final => Phase::Assessing,
            });
            access::guarded(&state, &runtime, &capture, Some(&manifest), None, |_| {
                ReportStore::new(&capture.root).save(&record)
            })?;
            if let Some(invoke) = fixture.as_mut() {
                authorized_dispatch(
                    &state,
                    &runtime,
                    &capture,
                    &manifest,
                    &entry,
                    deadline,
                    false,
                    |_| Ok(()),
                )?;
                invoke(invocation.prompt, invocation.remaining)
            } else {
                controlled_invoke(
                    &state,
                    &runtime,
                    &capture,
                    &manifest,
                    &entry,
                    deadline,
                    invocation.prompt,
                    matches!(invocation.phase, engine::InvocationPhase::Final),
                )
            }
        };
        let investigation = engine::investigate_from(
            started,
            evidence,
            &capture.rubric,
            &bundle.pages,
            crate::taskmaster::runtime::assessment::IMPROVEMENT_ENTRY_PAGE_ID,
            &mut invoke,
            || entry.cancelled.load(Ordering::SeqCst),
        )?;
        let mut report = publication::report(&capture, investigation)?;
        let manifest = report.manifest.clone();
        let store = ReportStore::new(&capture.root);
        authorized_dispatch(
            &state,
            &runtime,
            &capture,
            &manifest,
            &entry,
            deadline,
            false,
            |workspace| {
                let now = crate::taskmaster::readiness_store::now();
                let recommendation =
                    publication::recommendation(workspace, &report, &record.id, &now)?;
                report.recommendation_id = recommendation.as_ref().map(|value| value.id.clone());
                if report.result.preferred().is_some() && recommendation.is_none() {
                    report.publication_limitation =
                        Some("An existing recommendation prevented a new card.".into());
                }
                // History is optional secondary publication; same-response rows remain authoritative.
                report.history_error = Some("history_pending".into());
                record.completed_at = Some(now);
                record.report = Some(report);
                record.pending_recommendation = recommendation;
                store::publish_with(
                    &mut record,
                    |value| {
                        store.save(value)?;
                        // Once the durable pending marker exists, recovery owns the exact
                        // transaction. Cancellation can no longer withdraw publication.
                        if matches!(
                            value.state,
                            RunState::PendingRecommendation | RunState::Succeeded
                        ) {
                            entry.settled.store(true, Ordering::SeqCst);
                        }
                        Ok(())
                    },
                    |recommendation| {
                        crate::taskmaster::assessment::upsert_assessment_recommendation(
                            workspace,
                            recommendation,
                        )
                    },
                )?;
                publish_history(&capture, &mut record, &store)
            },
        )?;
        Ok(())
    })();
    drop(monitor);
    if result.is_err() {
        let (code, interrupted) = settle_failure(&entry, deadline);
        finish_failure(&state, &runtime, &capture, &record.id, code, interrupted);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    pub workspace_path: Option<String>,
    pub available: bool,
    pub reason_code: Option<&'static str>,
    pub run: Option<PublicRun>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicRun {
    pub id: String,
    pub state: RunState,
    pub phase: Option<Phase>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_code: Option<String>,
    pub report: Option<serde_json::Value>,
}

pub(crate) fn status(state: &Arc<crate::AppState>, runtime_root: PathBuf) -> Status {
    status_with_runtime(
        state,
        &TaskmasterRuntime::open(runtime_root),
        PRODUCTION_QUALIFIED,
    )
}
fn status_with_runtime(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    qualified: bool,
) -> Status {
    let selected = state.workspace.lock().ok().and_then(|lock| {
        lock.as_ref().map(|w| {
            (
                w.path.clone(),
                w.metadata.root_path(),
                w.workflow_observations.instance_id(),
            )
        })
    });
    let Some((path, root, instance)) = selected else {
        return Status {
            workspace_path: None,
            available: false,
            reason_code: Some("workspace_unavailable"),
            run: None,
        };
    };
    let path_text = path.to_str().map(str::to_owned);
    let lease = runtime.try_analysis_lease().ok().flatten();
    let recovery_failed = lease
        .as_ref()
        .is_some_and(|_| recover_under_lease(state, runtime, &root).is_err());
    drop(lease);
    let loaded = runtime.with_workspace_settings(&path, |settings| {
        let lock = state
            .workspace
            .lock()
            .map_err(|_| "Workspace unavailable")?;
        let _workspace = lock
            .as_ref()
            .filter(|w| {
                w.path == path
                    && w.metadata.root_path() == root
                    && w.workflow_observations.instance_id() == instance
            })
            .ok_or("Workspace changed")?;
        let store = ReportStore::new(&root);
        store.reconcile(settings)?;
        store.load()
    });
    let record = match loaded {
        Ok(Ok(record)) => record,
        _ => {
            return Status {
                workspace_path: path_text,
                available: false,
                reason_code: Some("metadata_unavailable"),
                run: None,
            }
        }
    };
    let available = qualified && !recovery_failed && access::capture(state, runtime, None).is_ok();
    let reason_code = if recovery_failed {
        Some("recovery_unavailable")
    } else if !qualified {
        Some("qualification_disabled")
    } else if !available {
        Some("guidance_or_provider_unavailable")
    } else {
        None
    };
    let run = record.map(|record| {
        let mut public = PublicRun {
            id: record.id.clone(),
            state: record.state.clone(),
            phase: record.phase.clone(),
            started_at: record.started_at.clone(),
            completed_at: record.completed_at.clone(),
            error_code: record.error_code.clone(),
            report: None,
        };
        if record.state == RunState::Succeeded {
            if let Some(report) = record.report.as_ref() {
                if let Ok(capture) =
                    access::capture(state, runtime, report.recommendation_id.as_deref())
                {
                    let projected = access::guarded(
                        state,
                        runtime,
                        &capture,
                        Some(&report.manifest),
                        report.recommendation_id.as_deref(),
                        |workspace| {
                            if ReportStore::new(&capture.root).load()?.as_ref() != Some(&record) {
                                return Err("Report changed".into());
                            }
                            publication::current_report(report, &capture)?;
                            if let Some(id) = report.recommendation_id.as_deref() {
                                let recommendation = workspace
                                    .recommendation_store
                                    .get(id)
                                    .map_err(|_| "Recommendation unavailable")?
                                    .ok_or("Recommendation unavailable")?;
                                if !publication::join_valid(report, &record.id, &recommendation) {
                                    return Err("Recommendation changed".into());
                                }
                            }
                            serde_json::to_value(report.public()?)
                                .map_err(|_| "Report unavailable".into())
                        },
                    );
                    public.report = projected.ok();
                }
            }
        }
        public
    });
    Status {
        workspace_path: path_text,
        available,
        reason_code,
        run,
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
