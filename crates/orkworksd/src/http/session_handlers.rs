use crate::harness::registry::ResolvedHarness;
use crate::plan_handoff::{normalize_reported_plan_path, resolve_openable_plan_reference, resolve_printed_plan_path};
use crate::session_types::{MemoryState, SessionInfo};
use crate::session_view::{
    connectivity_for_status, derive_memory_state, detect_conflicts, merge_live_session_info,
    resolve_effective_cwds, session_recommendation, terminal_outcome_for_status,
};
use crate::workspace_runtime::{iso_now, orkworks_global_dir, parse_hook_observed_at};
use crate::{
    git, harness, metadata, migration, peon, watcher, AppState, SessionHandle, WorkspaceState,
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use portable_pty::PtySize;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Deserialize)]
pub(crate) struct WorkspaceRequest {
    pub(crate) path: String,
}

#[derive(Deserialize)]
pub(crate) struct ActiveSessionRequest {
    #[serde(rename = "sessionId")]
    pub(crate) session_id: String,
}

#[derive(Deserialize)]
pub(crate) struct ActiveHarnessesRequest {
    #[serde(rename = "activeHarnessIds", default)]
    pub(crate) active_harness_ids: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct HarnessSessionReportRequest {
    #[serde(rename = "harnessSessionId")]
    pub(crate) harness_session_id: String,
    pub(crate) source: String,
    pub(crate) confidence: f64,
}

#[derive(Deserialize)]
pub(crate) struct AttentionReportRequest {
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) message: Option<String>,
    #[serde(rename = "planPath", default)]
    pub(crate) plan_path: metadata::PlanPathUpdate,
    #[serde(rename = "observedAt", default)]
    pub(crate) observed_at: Option<String>,
    /// The harness's own logical working directory, when it reports one
    /// (currently Claude Code only, forwarded from its hook JSON payload).
    /// Authoritative over the pid-probed/launch-time cwd fallbacks — see
    /// ADR 0032.
    #[serde(default)]
    pub(crate) cwd: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct PlanPathReportRequest {
    #[serde(rename = "planPath")]
    pub(crate) plan_path: String,
}

#[derive(Deserialize)]
pub(crate) struct TerminalPlanSelectionRequest {
    #[serde(rename = "printedPath")]
    pub(crate) printed_path: String,
}

#[derive(Deserialize)]
pub(crate) struct DebugAttentionRequest {
    pub(crate) attention: String,
    #[serde(default)]
    pub(crate) message: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct WorkspaceResponse {
    pub(crate) path: String,
    pub(crate) repo_root: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) dirty: Option<bool>,
    #[serde(rename = "lastActiveSessionId")]
    pub(crate) last_active_session_id: Option<String>,
    #[serde(rename = "activeHarnessIds", skip_serializing_if = "Vec::is_empty")]
    pub(crate) active_harness_ids: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct PlanContentResponse {
    pub(crate) content: String,
}

fn authorize_plan_request(headers: &HeaderMap) -> Result<(), axum::http::StatusCode> {
    let Ok(token) = std::env::var("ORKWORKS_OPEN_PLAN_TOKEN") else {
        return Err(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    };
    if token.is_empty() {
        return Err(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }
    (Some(token.as_str())
        == headers
            .get("x-orkworks-open-plan-token")
            .and_then(|value| value.to_str().ok()))
        .then_some(())
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)
}

pub(crate) async fn get_session_plan_content(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(status) = authorize_plan_request(&headers) {
        return status.into_response();
    }
    let (workspace_root, plan_path) = {
        let workspace = state.workspace.lock().unwrap();
        let Some(workspace) = workspace.as_ref() else {
            return axum::http::StatusCode::CONFLICT.into_response();
        };
        let Some(metadata) = workspace.metadata.read_session(&id) else {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        };
        let Some(plan_path) = metadata.plan_path else {
            return axum::http::StatusCode::CONFLICT.into_response();
        };
        (workspace.path.clone(), plan_path)
    };
    match resolve_openable_plan_reference(&workspace_root, &plan_path)
        .and_then(|path| std::fs::read_to_string(path).map_err(|error| error.to_string()))
    {
        Ok(content) => Json(PlanContentResponse { content }).into_response(),
        Err(_) => axum::http::StatusCode::CONFLICT.into_response(),
    }
}

pub(crate) async fn request_session_plan_review(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(status) = authorize_plan_request(&headers) { return status.into_response(); }
    let plan_path = {
        let workspace = state.workspace.lock().unwrap();
        let Some(workspace) = workspace.as_ref() else { return axum::http::StatusCode::CONFLICT.into_response(); };
        let Some(meta) = workspace.metadata.read_session(&id) else { return axum::http::StatusCode::NOT_FOUND.into_response(); };
        if meta.lifecycle != "alive" { return axum::http::StatusCode::CONFLICT.into_response(); }
        let Some(path) = meta.plan_path else { return axum::http::StatusCode::CONFLICT.into_response(); };
        let resolved = match resolve_openable_plan_reference(&workspace.path, &path) {
            Ok(path) => path,
            Err(_) => return axum::http::StatusCode::CONFLICT.into_response(),
        };
        let launch_root = std::path::Path::new(&meta.cwd).canonicalize().ok();
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
    let prompt = format!("Please review the plan or specification at {plan_path}. If your tooling can spawn a separate review subagent, delegate the review to it instead of reviewing your own work; otherwise review it yourself. Check for missing requirements, risky assumptions, and unclear steps, then report the findings.\r");
    match crate::runtime::terminal_runtime::submit_approved_input(&state, &id, prompt).await {
        Ok(()) => {
            if let Some(workspace) = state.workspace.lock().unwrap().as_ref() {
                workspace.metadata.append_event(&id, &metadata::Event {
                    event_type: "plan_review_requested".into(), timestamp: iso_now(), status: "working".into(),
                    observed_status: Some("working".into()), confidence: None, summary: Some("User requested plan review.".into()), source: Some("user".into()),
                });
            }
            axum::http::StatusCode::NO_CONTENT.into_response()
        }
        Err(()) => axum::http::StatusCode::CONFLICT.into_response(),
    }
}

pub(crate) async fn select_terminal_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<TerminalPlanSelectionRequest>,
) -> impl IntoResponse {
    if let Err(status) = authorize_plan_request(&headers) { return status.into_response(); }
    let result = tokio::task::spawn_blocking(move || {
        let workspace = state.workspace.lock().unwrap();
        let Some(workspace) = workspace.as_ref() else { return Err(axum::http::StatusCode::CONFLICT); };
        let Some(mut meta) = workspace.metadata.read_session(&id) else { return Err(axum::http::StatusCode::NOT_FOUND); };
        let (worktree_root, relative_path) = resolve_printed_plan_path(std::path::Path::new(&meta.cwd), &req.printed_path)
            .map_err(|error| {
                tracing::warn!(session_id = %id, printed_path = %req.printed_path, %error, "select_terminal_plan: plan path resolution failed");
                axum::http::StatusCode::CONFLICT
            })?;
        meta.plan_path = Some(metadata::PlanReference {
            worktree_root: Some(worktree_root.to_string_lossy().into_owned()),
            relative_path,
            source: metadata::PlanSource::UserSelected,
        });
        workspace.metadata.try_write_session(&meta).map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        workspace.metadata.append_event(&id, &metadata::Event {
            event_type: "session.plan_selected_by_user".into(), timestamp: iso_now(), status: meta.status.clone(),
            observed_status: meta.observed_status.clone(), confidence: Some(1.0), summary: None, source: Some("user".into()),
        });
        Ok(())
    }).await;
    match result { Ok(Ok(())) => axum::http::StatusCode::NO_CONTENT.into_response(), Ok(Err(status)) => status.into_response(), Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response() }
}

pub(crate) async fn report_session_plan_path(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<PlanPathReportRequest>,
) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        let workspace = state.workspace.lock().unwrap();
        let Some(workspace) = workspace.as_ref() else { return Err(axum::http::StatusCode::CONFLICT); };
        let Some(mut metadata) = workspace.metadata.read_session(&id) else { return Err(axum::http::StatusCode::NOT_FOUND); };
        if metadata.lifecycle != "alive" { return Err(axum::http::StatusCode::CONFLICT); }
        if metadata.plan_path.as_ref().is_some_and(|reference| reference.source == metadata::PlanSource::UserSelected) {
            return Ok(());
        }
        let relative = normalize_reported_plan_path(&workspace.path, &req.plan_path)
            .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
        metadata.plan_path = Some(metadata::PlanReference {
            worktree_root: Some(workspace.path.to_string_lossy().into_owned()),
            relative_path: relative,
            source: metadata::PlanSource::HookReported,
        });
        // The session JSON is the source of truth. Use the fallible writer
        // and only append the hooked event when the write actually
        // landed, so the event log cannot claim a path association that
        // the session file does not reflect.
        workspace
            .metadata
            .try_write_session(&metadata)
            .map_err(|error| {
                tracing::error!(error = %error, session = %id, "plan path session write failed");
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?;
        workspace.metadata.append_event(&id, &metadata::Event {
            event_type: "session.plan_path_hooked".into(), timestamp: iso_now(), status: metadata.status.clone(),
            observed_status: metadata.observed_status.clone(), confidence: Some(1.0),
            summary: None, source: Some("agent".into()),
        });
        Ok(())
    }).await;
    match result {
        Ok(Ok(())) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(error) => { tracing::error!(error = %error, "plan path metadata task failed"); axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response() }
    }
}

pub(crate) async fn set_workspace(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WorkspaceRequest>,
) -> impl IntoResponse {
    let ws_path = PathBuf::from(&req.path);
    if !ws_path.is_dir() {
        return (axum::http::StatusCode::BAD_REQUEST, "not a directory").into_response();
    }

    let global_dir = match orkworks_global_dir(&ws_path) {
        Some(d) => d,
        None => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "no home directory",
            )
                .into_response();
        }
    };
    for dir in &["sessions", "events", "capacity", "skills"] {
        if let Err(e) = std::fs::create_dir_all(global_dir.join(dir)) {
            tracing::warn!(path = %global_dir.display(), dir = dir, error = %e, "failed to create metadata dir");
        }
    }

    let store = metadata::MetadataStore::new(&global_dir);

    migration::migrate_if_needed(&ws_path, &global_dir);

    let memory = store.read_workspace_memory();
    let last_active_session_id = memory
        .as_ref()
        .and_then(|m| m.last_active_session_id.clone());
    let active_harness_ids = memory.map(|m| m.active_harness_ids).unwrap_or_default();
    let watch_dir = global_dir.join("sessions");
    let watcher = watcher::MetadataWatcher::start(&watch_dir);

    let workflow_observations = match crate::workflow_observations::WorkflowObservationStore::open(
        global_dir.clone(),
    ) {
        Ok(store) => store,
        Err(e) => {
            tracing::warn!(path = %global_dir.display(), error = %e, "failed to open workflow observation store");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to open workflow observation store",
            )
                .into_response();
        }
    };
    let recommendation_store = match crate::taskmaster::store::RecommendationStore::open(
        global_dir.clone(),
    ) {
        Ok(store) => store,
        Err(e) => {
            tracing::warn!(path = %global_dir.display(), error = %e, "failed to open recommendation store");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to open recommendation store",
            )
                .into_response();
        }
    };
    let retained_session_ids = store
        .read_all_sessions()
        .into_iter()
        .map(|session| session.id)
        .collect::<std::collections::HashSet<_>>();
    if let Err(e) = recommendation_store.scrub_orphans(&retained_session_ids) {
        tracing::warn!(path = %global_dir.display(), error = %e, "failed to scrub orphaned recommendations");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "failed to scrub orphaned recommendations",
        )
            .into_response();
    }

    let mut ws = state.workspace.lock().unwrap();
    *ws = Some(WorkspaceState {
        path: ws_path.clone(),
        metadata: store,
        workflow_observations,
        recommendation_store,
        watcher,
    });
    state.bump_harness_probe_generation();

    // Reconcile sessions left in "running"/"creating" from a previous daemon
    // run. This handler also re-runs whenever the Electron app alone
    // restarts and reconnects to an already-running (detached) sidecar, in
    // which case state.sessions is NOT empty and still holds live handles —
    // only a session with no matching in-memory handle is actually orphaned.
    if let Some(ref ws) = *ws {
        let now = iso_now();
        let live_ids: std::collections::HashSet<String> =
            state.sessions.lock().unwrap().keys().cloned().collect();
        for meta in ws.metadata.read_all_sessions() {
            if (meta.status == "running" || meta.status == "creating")
                && !live_ids.contains(&meta.id)
            {
                ws.metadata
                    .write_session(&metadata::reconcile_orphaned_session(meta, &now));
            }
        }
    }

    drop(ws);
    crate::taskmaster::evaluator::schedule_evaluation(state.clone());

    let git_ctx = git::detect(&ws_path);

    Json(WorkspaceResponse {
        path: req.path,
        repo_root: git_ctx.repo_root,
        branch: git_ctx.branch,
        dirty: Some(git_ctx.dirty),
        last_active_session_id,
        active_harness_ids,
    })
    .into_response()
}

pub(crate) async fn set_active_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActiveSessionRequest>,
) -> impl IntoResponse {
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        let existing = ws.metadata.read_workspace_memory();
        ws.metadata
            .write_workspace_memory(&metadata::WorkspaceMemory {
                last_active_session_id: Some(req.session_id),
                last_active_at: Some(now),
                active_harness_ids: existing.map(|m| m.active_harness_ids).unwrap_or_default(),
            });
        return axum::http::StatusCode::OK;
    }
    axum::http::StatusCode::CONFLICT
}

pub(crate) async fn set_active_harnesses(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActiveHarnessesRequest>,
) -> impl IntoResponse {
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        let existing = ws.metadata.read_workspace_memory();
        ws.metadata
            .write_workspace_memory(&metadata::WorkspaceMemory {
                last_active_session_id: existing
                    .as_ref()
                    .and_then(|m| m.last_active_session_id.clone()),
                last_active_at: Some(now),
                active_harness_ids: req.active_harness_ids,
            });
        return axum::http::StatusCode::OK;
    }
    axum::http::StatusCode::CONFLICT
}

fn resume_handle_conflicts(
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

struct ResumeAdmission {
    state: Arc<AppState>,
    id: String,
    generation: crate::runtime::session_runtime::RuntimeGeneration,
    previous_handle: Option<SessionHandle>,
    rollback: Option<ResumeRollback>,
    committed: bool,
}

impl ResumeAdmission {
    fn generation(&self) -> crate::runtime::session_runtime::RuntimeGeneration {
        self.generation
    }

    fn arm_rollback(
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

    fn commit(mut self) {
        self.committed = true;
        self.previous_handle = None;
        self.rollback = None;
    }
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

fn try_install_claimed_resume_handle(
    state: &Arc<AppState>,
    id: &str,
    mut replacement: SessionHandle,
    metadata_ended: bool,
    expected_generation: Option<crate::runtime::session_runtime::RuntimeGeneration>,
) -> Result<ResumeAdmission, ()> {
    // The caller performs the persisted-metadata recheck immediately before
    // admission. Keeping this helper free of the workspace lock is essential:
    // rollback tests and callers may deliberately hold that lock while
    // coordinating generation replacement.
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

pub(crate) async fn resume_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
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
            return axum::http::StatusCode::CONFLICT.into_response();
        };
        let Some(meta) = ws.metadata.read_session(&id) else {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        };
        let Some(resume) = meta.resume.as_ref() else {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
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
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        }
        let Some(command) = harness.build_resume(
            strategy.clone(),
            &meta.cwd,
            resume.harness_session_id.as_deref(),
            meta.repo_root.as_deref(),
            (!meta.model.is_empty()).then_some(meta.model.as_str()),
        ) else {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
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
    let mut admission = {
        let ws_guard = state.workspace.lock().unwrap();
        let metadata_ended = ws_guard.as_ref().is_some_and(|ws| {
            ws.metadata
                .read_session(&id)
                .is_some_and(|metadata| metadata.lifecycle_phase == "ended")
        });
        match try_install_claimed_resume_handle(
            &state,
            &id,
            replacement,
            metadata_ended,
            expected_generation,
        ) {
            Ok(admission) => admission,
            Err(()) => return axum::http::StatusCode::CONFLICT.into_response(),
        }
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
            ) {
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
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
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

    Json(info).into_response()
}

pub(crate) async fn report_harness_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<HarnessSessionReportRequest>,
) -> impl IntoResponse {
    let report = metadata::HarnessSessionReport {
        harness_session_id: req.harness_session_id,
        source: req.source,
        confidence: req.confidence,
    };

    if !metadata::valid_harness_session_report(&report) {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }

    let now = iso_now();
    let result = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ref ws) = *ws_guard else {
            return axum::http::StatusCode::CONFLICT.into_response();
        };
        ws.metadata.merge_harness_session_report(&id, &report, &now)
    };

    if result == metadata::HarnessSessionMergeResult::Accepted {
        let updated_resume = {
            let ws_guard = state.workspace.lock().unwrap();
            ws_guard
                .as_ref()
                .and_then(|ws| ws.metadata.read_session(&id))
                .and_then(|meta| meta.resume)
        };
        if let Some(updated_resume) = updated_resume {
            let mut sessions = state.sessions.lock().unwrap();
            if let Some(handle) = sessions.get_mut(&id) {
                handle.info.resume = Some(updated_resume);
            }
        }
    }

    match result {
        metadata::HarnessSessionMergeResult::Accepted
        | metadata::HarnessSessionMergeResult::IgnoredLowerConfidence => {
            axum::http::StatusCode::OK.into_response()
        }
        metadata::HarnessSessionMergeResult::NotFound => {
            axum::http::StatusCode::NOT_FOUND.into_response()
        }
        metadata::HarnessSessionMergeResult::Invalid => {
            axum::http::StatusCode::BAD_REQUEST.into_response()
        }
    }
}

pub(crate) async fn report_attention(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AttentionReportRequest>,
) -> impl IntoResponse {
    let observed_at = match req.observed_at.as_deref() {
        Some(raw) => match parse_hook_observed_at(raw) {
            Ok(timestamp) => Some(timestamp),
            Err(()) => return axum::http::StatusCode::BAD_REQUEST.into_response(),
        },
        None => None,
    };
    let active_alias = matches!(req.status.as_str(), "thinking" | "reasoning");
    if !active_alias && !peon::is_valid_observed_status(&req.status) {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let supports_active_work = state
        .sessions
        .lock()
        .unwrap()
        .get(&id)
        .is_some_and(|handle| handle.active_work_hook);
    let Some(status) = normalize_hook_attention_status(&req.status, supports_active_work) else {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    };

    if observed_at.is_some_and(|timestamp| {
        state
            .sessions
            .lock()
            .unwrap()
            .get(&id)
            .and_then(|handle| handle.runtime.accepted_input_at)
            .is_some_and(|accepted_at| timestamp <= accepted_at)
    }) {
        return axum::http::StatusCode::OK.into_response();
    }

    // Record the harness's self-reported cwd (issue #241 / ADR 0032) whenever
    // one accompanies this report. Gated behind the same staleness check as
    // the attention status above — a delayed/superseded hook event carries an
    // equally stale cwd, and since this is the top-priority tier in
    // resolve_effective_cwds, a stale write here can't be corrected by the
    // more-accurate live probe underneath it. An empty string is treated as
    // "nothing reported" rather than clearing a previously-known value.
    if let Some(cwd) = req.cwd.as_deref().filter(|c| !c.is_empty()) {
        state
            .peon
            .reported_cwd
            .write()
            .unwrap()
            .insert(id.clone(), cwd.to_string());
    }

    let now = iso_now();
    let persist_state = state.clone();
    let persist_id = id.clone();
    let persist_status = status.clone();
    let message = req.message.clone();
    let plan_path = req.plan_path.clone();
    let observed_at_for_commit = observed_at;
    let result = match tokio::task::spawn_blocking(move || {
        // Workspace existence is checked unconditionally first, matching the
        // pre-refactor order: a torn-down workspace must always mean 409, not
        // 200, regardless of whether this particular report also turns out to
        // be stale.
        if persist_state.workspace.lock().unwrap().is_none() {
            return Err(axum::http::StatusCode::CONFLICT);
        }
        if observed_at_for_commit.is_some_and(|timestamp| {
            persist_state
                .sessions
                .lock()
                .unwrap()
                .get(&persist_id)
                .and_then(|handle| handle.runtime.accepted_input_at)
                .is_some_and(|accepted_at| timestamp <= accepted_at)
        }) {
            return Ok(metadata::AttentionMergeResult::Ignored);
        }
        match crate::runtime::observed_status::apply_attention_signal(
            &persist_state,
            &persist_id,
            &persist_status,
            message.as_deref(),
            &plan_path,
            &now,
            "agent",
            1.0,
            observed_at_for_commit,
        ) {
            Some(result) => Ok(result),
            None => Err(axum::http::StatusCode::CONFLICT),
        }
    })
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(status)) => return status.into_response(),
        Err(error) => {
            tracing::error!(error = %error, "attention metadata task failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if result == metadata::AttentionMergeResult::Accepted && status == "working" {
        if let Some(observed_at) = observed_at {
            clear_claude_capacity_after_working(&state, &id, observed_at);
        }
    }

    if result == metadata::AttentionMergeResult::Accepted {
        let mut bufs = state.peon.input_buf.write().unwrap();
        if bufs
            .get(&id)
            .is_some_and(|buf| !peon::is_descriptive_input(buf))
        {
            bufs.remove(&id);
        }
    }

    match result {
        metadata::AttentionMergeResult::Accepted => axum::http::StatusCode::OK.into_response(),
        metadata::AttentionMergeResult::Ignored => axum::http::StatusCode::OK.into_response(),
        metadata::AttentionMergeResult::NotFound => {
            axum::http::StatusCode::NOT_FOUND.into_response()
        }
        // The signal was lost, not delivered — a 200 here would tell the
        // harness hook its notification landed when it didn't.
        metadata::AttentionMergeResult::PersistFailed => {
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
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

fn normalize_hook_attention_status(status: &str, supports_active_work: bool) -> Option<String> {
    match status {
        "working" | "thinking" | "reasoning" if supports_active_work => Some("working".into()),
        "waiting_for_input" | "blocked" | "failed" | "done" | "stale" | "idle" => {
            Some(status.into())
        }
        _ => None,
    }
}

/// Dev-only convenience for exercising UI/runtime convergence without a real
/// coding-agent session. Writes through the same merge path as `report_attention`
/// but tagged `source = "debug"`, `confidence = 0.0` — the lowest documented
/// priority tier, so any real signal (including the next peon inference pass)
/// reclaims the session immediately. That reclaim is the intended behavior, not
/// a bug: injecting a value and watching it get overwritten by a real signal is
/// itself the convergence test this endpoint exists to support.
pub(crate) async fn apply_debug_attention(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<DebugAttentionRequest>,
) -> impl IntoResponse {
    if !matches!(
        req.attention.as_str(),
        "working" | "idle" | "needs_you" | "blocked" | "failed" | "capped"
    ) {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }

    let observed_status = if req.attention == "needs_you" {
        "waiting_for_input".to_string()
    } else {
        req.attention.clone()
    };
    let is_capped = req.attention == "capped";
    let summary_message = if is_capped { None } else { req.message.clone() };

    let now = iso_now();
    let persist_state = state.clone();
    let persist_id = id.clone();
    let persist_status = observed_status.clone();
    let persist_message = req.message.clone();
    let result = match tokio::task::spawn_blocking(move || {
        // This bypasses apply_attention_signal's self-locking shell and holds
        // the workspace/sessions locks itself, like mark_committed_input_working
        // does -- the lifecycle precheck and usage_limit_reset_hint write both
        // need to stay atomic with the attention-field write, not split into
        // separately-locked critical sections a concurrent call could interleave
        // with.
        let ws_guard = persist_state.workspace.lock().unwrap();
        let Some(ref ws) = *ws_guard else {
            return Err(axum::http::StatusCode::CONFLICT);
        };
        match ws.metadata.read_session(&persist_id) {
            None => return Err(axum::http::StatusCode::NOT_FOUND),
            Some(meta) if meta.lifecycle != "alive" => {
                return Err(axum::http::StatusCode::BAD_REQUEST);
            }
            Some(_) => {}
        }
        let result = ws.metadata.merge_agent_attention_signal_with_plan(
            &persist_id,
            &persist_status,
            summary_message.as_deref(),
            &metadata::PlanPathUpdate::Unchanged,
            &now,
            "debug",
            0.0,
        );
        if result == metadata::AttentionMergeResult::Accepted {
            if let Some(handle) = persist_state.sessions.lock().unwrap().get_mut(&persist_id) {
                crate::runtime::observed_status::apply_live_attention_fields(
                    &mut handle.info,
                    &persist_status,
                    summary_message.as_deref(),
                    "debug",
                    0.0,
                );
                if is_capped {
                    if persist_message.is_some() {
                        handle.info.usage_limit_reset_hint = persist_message.clone();
                    }
                } else {
                    // Moving off capped must not leave a stale reset hint that
                    // can propagate to other live sessions on the harness.
                    handle.info.usage_limit_reset_hint = None;
                }
            }
        }
        Ok(result)
    })
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(status)) => return status.into_response(),
        Err(error) => {
            tracing::error!(error = %error, "debug attention metadata task failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match result {
        metadata::AttentionMergeResult::Accepted => axum::http::StatusCode::OK.into_response(),
        metadata::AttentionMergeResult::Ignored => axum::http::StatusCode::OK.into_response(),
        metadata::AttentionMergeResult::NotFound => {
            axum::http::StatusCode::NOT_FOUND.into_response()
        }
        metadata::AttentionMergeResult::PersistFailed => {
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize, Default)]
pub(crate) struct CreateSessionRequest {
    #[serde(rename = "harnessId", default)]
    pub(crate) harness_id: Option<String>,
    #[serde(default)]
    pub(crate) model: Option<String>,
    #[serde(rename = "initialPrompt", default)]
    pub(crate) initial_prompt: Option<String>,
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
    req: &CreateSessionRequest,
    cwd: String,
) -> ResolvedSessionLaunch {
    let requested_id = req.harness_id.as_deref();
    let harness = requested_id
        .and_then(|id| registry.get(id))
        .filter(|harness| !harness.definition.retired)
        .or_else(|| registry.get("generic-shell"))
        .expect("generic-shell builtin exists");
    let model = req
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

pub(crate) async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
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
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "The selected coding tool is retired and cannot start new sessions.",
        )
            .into_response();
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
                ) {
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

    Json(info).into_response()
}

fn enrich_sessions_with_git_context<F>(
    infos: &mut [SessionInfo],
    effective_cwds: &HashMap<String, String>,
    mut detect_git: F,
) where
    F: FnMut(&std::path::Path) -> git::GitContext,
{
    // `effective_cwds` prefers each session's live PTY-process cwd (issue
    // #241 — an agent that `cd`s or `git worktree add`s mid-session
    // shouldn't be shown frozen at its launch location forever), falling
    // back to the launch-time `info.cwd` when there's no tracked pid or the
    // probe fails. See `session_view::resolve_effective_cwds`.
    let cwd_for = |info: &SessionInfo| -> String {
        effective_cwds
            .get(&info.id)
            .cloned()
            .unwrap_or_else(|| info.cwd.clone())
    };

    let mut cwd_counts: HashMap<String, usize> = HashMap::new();
    for info in infos.iter() {
        if info.status == "running" || info.status == "creating" {
            *cwd_counts.entry(cwd_for(info)).or_default() += 1;
        }
    }

    let mut contexts: HashMap<String, git::GitContext> = HashMap::new();
    for info in infos.iter_mut() {
        let cwd = cwd_for(info);
        if !contexts.contains_key(&cwd) {
            contexts.insert(cwd.clone(), detect_git(std::path::Path::new(&cwd)));
        }
        let ctx = &contexts[&cwd];
        let count = cwd_counts.get(&cwd).copied().unwrap_or(1);
        info.recommendation = session_recommendation(ctx, count);
        info.repo_root = ctx.repo_root.clone();
        info.branch = ctx.branch.clone();
        info.dirty = Some(ctx.dirty);
        info.changed_files = Some(ctx.changed_files);
        info.is_worktree = Some(ctx.is_worktree);
    }
}

pub(crate) async fn list_sessions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let registry = state
        .harness_catalog
        .read()
        .expect("harness catalog lock poisoned")
        .clone();
    let live_sessions: Vec<(
        SessionInfo,
        Vec<String>,
        String,
        bool,
        bool,
        u64,
        u64,
        Option<(u64, u64)>,
        bool,
    )> = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .values()
            .map(|h| {
                (
                    h.info.clone(),
                    h.output_buffer.snapshot(),
                    h.scan_buf.clone(),
                    h.at_usage_limit_latched,
                    h.capacity_check_pending,
                    h.output_lines_seen,
                    h.scan_bytes_seen,
                    h.resume_scan_origin,
                    h.pending_capacity_visible_once,
                )
            })
            .collect()
    };

    let ws_guard = state.workspace.lock().unwrap();
    let workspace_root = ws_guard.as_ref().map(|ws| ws.path.clone());
    let metadata_map = ws_guard
        .as_ref()
        .map(|ws| {
            let mut metadata = HashMap::new();
            for (info, _, _, _, _, _, _, _, _) in &live_sessions {
                if let Some(meta) = ws.metadata.read_session(&info.id) {
                    metadata.insert(info.id.clone(), meta);
                }
            }
            metadata
        })
        .unwrap_or_default();

    let all_metadata_sessions = ws_guard
        .as_ref()
        .map(|ws| ws.metadata.read_all_sessions())
        .unwrap_or_default();
    drop(ws_guard);

    let all_memory_ids: HashSet<String> = live_sessions
        .iter()
        .map(|(info, _, _, _, _, _, _, _, _)| info.id.clone())
        .collect();
    let capacity_snapshots: HashMap<String, (bool, Option<(u64, u64)>, u64, u64)> = live_sessions
        .iter()
        .map(|(info, _, _, latched, _, lines, bytes, origin, _)| {
            (info.id.clone(), (*latched, *origin, *lines, *bytes))
        })
        .collect();

    let peon_times = state.peon.last_inference.read().unwrap();
    let mut pending_transitions: Vec<(String, bool, bool)> = Vec::new();
    let mut capped_recheck_resets: HashSet<String> = HashSet::new();
    let mut capped_clear_baselines: HashMap<String, (u64, u64)> = HashMap::new();
    let mut infos: Vec<SessionInfo> = live_sessions
        .into_iter()
        .map(
            |(
                info,
                snapshot,
                scan_buf,
                prev_latch,
                pending,
                output_lines_seen,
                scan_bytes_seen,
                origin,
                pending_visible_once,
            )| {
                let id = info.id.clone();
                let meta = metadata_map.get(&id);
                let session_harness_id =
                    meta.and_then(|m| (!m.harness.is_empty()).then_some(m.harness.as_str()));
                let resolved_harness = session_harness_id
                    .and_then(|id| registry.get(id))
                    .or_else(|| registry.get("generic-shell"));
                let mut merged =
                    merge_live_session_info(info, meta, peon_times.get(&id), resolved_harness);
                merged.has_openable_plan = meta
                    .and_then(|metadata| metadata.plan_path.as_ref())
                    .and_then(|reference| {
                        workspace_root
                            .as_deref()
                            .map(|root| resolve_openable_plan_reference(root, reference).is_ok())
                    });
                let fresh_output_since_origin = origin
                    .map(|(line_count, scan_len)| {
                        output_lines_seen > line_count || scan_bytes_seen > scan_len
                    })
                    .unwrap_or(false);
                let has_fresh_resume_output =
                    pending && !pending_visible_once && fresh_output_since_origin;
                let limit_patterns = resolved_harness
                    .map(|harness| harness.capacity_patterns())
                    .unwrap_or(&[]);
                let stale_cap_recheck = prev_latch && !pending && origin.is_some();
                let baseline_scoped_detection = !prev_latch && !pending && origin.is_some();
                merged.at_usage_limit = resolved_harness.map(|_| {
                    let detected_full = peon::detect_usage_limit(limit_patterns, &snapshot)
                        || peon::detect_usage_limit_raw(limit_patterns, &scan_buf);
                    if stale_cap_recheck && fresh_output_since_origin {
                        let (line_count, scan_len) = origin.unwrap();
                        let line_window_start =
                            output_lines_seen.saturating_sub(snapshot.len() as u64);
                        let scan_window_start =
                            scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                        let fresh_line_start =
                            line_count.saturating_sub(line_window_start) as usize;
                        let fresh_scan_start = scan_len.saturating_sub(scan_window_start) as usize;
                        let fresh_lines = snapshot
                            .get(fresh_line_start.min(snapshot.len())..)
                            .unwrap_or(&[]);
                        let fresh_scan = scan_buf
                            .get(fresh_scan_start.min(scan_buf.len())..)
                            .unwrap_or("");
                        let detected_scoped = peon::detect_usage_limit(limit_patterns, fresh_lines)
                            || peon::detect_usage_limit_raw(limit_patterns, fresh_scan);
                        capped_recheck_resets.insert(id.clone());
                        if !detected_scoped {
                            capped_clear_baselines
                                .insert(id.clone(), (output_lines_seen, scan_bytes_seen));
                        }
                        detected_scoped
                    } else if baseline_scoped_detection {
                        let (line_count, scan_len) = origin.unwrap();
                        let line_window_start =
                            output_lines_seen.saturating_sub(snapshot.len() as u64);
                        let scan_window_start =
                            scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                        let fresh_line_start =
                            line_count.saturating_sub(line_window_start) as usize;
                        let fresh_scan_start = scan_len.saturating_sub(scan_window_start) as usize;
                        let fresh_lines = snapshot
                            .get(fresh_line_start.min(snapshot.len())..)
                            .unwrap_or(&[]);
                        let fresh_scan = scan_buf
                            .get(fresh_scan_start.min(scan_buf.len())..)
                            .unwrap_or("");
                        let detected_scoped = peon::detect_usage_limit(limit_patterns, fresh_lines)
                            || peon::detect_usage_limit_raw(limit_patterns, fresh_scan);
                        if detected_scoped {
                            capped_recheck_resets.insert(id.clone());
                        }
                        detected_scoped
                    } else {
                        prev_latch || detected_full
                    }
                });
                if merged.lifecycle == "alive" && merged.at_usage_limit == Some(true) {
                    merged.attention = Some("capped".into());
                }
                let detected_reset_hint = resolved_harness.and_then(|_| {
                    if stale_cap_recheck && fresh_output_since_origin {
                        let (line_count, scan_len) = origin.unwrap();
                        let line_window_start =
                            output_lines_seen.saturating_sub(snapshot.len() as u64);
                        let scan_window_start =
                            scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                        let fresh_line_start =
                            line_count.saturating_sub(line_window_start) as usize;
                        let fresh_scan_start = scan_len.saturating_sub(scan_window_start) as usize;
                        let fresh_lines = snapshot
                            .get(fresh_line_start.min(snapshot.len())..)
                            .unwrap_or(&[]);
                        let fresh_scan = scan_buf
                            .get(fresh_scan_start.min(scan_buf.len())..)
                            .unwrap_or("");
                        peon::detect_usage_limit_hint(limit_patterns, fresh_lines).or_else(|| {
                            peon::detect_usage_limit_hint_raw(limit_patterns, fresh_scan)
                        })
                    } else if baseline_scoped_detection {
                        let (line_count, scan_len) = origin.unwrap();
                        let line_window_start =
                            output_lines_seen.saturating_sub(snapshot.len() as u64);
                        let scan_window_start =
                            scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                        let fresh_line_start =
                            line_count.saturating_sub(line_window_start) as usize;
                        let fresh_scan_start = scan_len.saturating_sub(scan_window_start) as usize;
                        let fresh_lines = snapshot
                            .get(fresh_line_start.min(snapshot.len())..)
                            .unwrap_or(&[]);
                        let fresh_scan = scan_buf
                            .get(fresh_scan_start.min(scan_buf.len())..)
                            .unwrap_or("");
                        peon::detect_usage_limit_hint(limit_patterns, fresh_lines).or_else(|| {
                            peon::detect_usage_limit_hint_raw(limit_patterns, fresh_scan)
                        })
                    } else {
                        peon::detect_usage_limit_hint(limit_patterns, &snapshot).or_else(|| {
                            peon::detect_usage_limit_hint_raw(limit_patterns, &scan_buf)
                        })
                    }
                });
                // Non-debug sources are always fully recomputed from the current
                // terminal window (clears the hint once it's no longer detected). A
                // debug-injected hint has no real terminal output to detect from, so
                // it's only preserved (not cleared just because this poll found
                // nothing) while the session is still alive and actually showing
                // "capped" — apply_debug_attention clears the carried value whenever
                // debug attention moves off "capped", but this is the single choke
                // point everything (including cross-session harness propagation
                // below) flows through, so it also guards against a lingering hint
                // surviving lifecycle end or any other path that left it set.
                let preserve_debug_hint = merged.metadata_source.as_deref() == Some("debug")
                    && merged.lifecycle == "alive"
                    && merged.attention.as_deref() == Some("capped");
                if !preserve_debug_hint || detected_reset_hint.is_some() {
                    merged.usage_limit_reset_hint = detected_reset_hint;
                }
                merged.capacity_check_pending = if pending && !pending_visible_once {
                    Some(true)
                } else {
                    None
                };
                pending_transitions.push((id, has_fresh_resume_output, pending_visible_once));
                merged
            },
        )
        .collect();

    // Append remembered (non-live) sessions from metadata
    for meta in &all_metadata_sessions {
        if all_memory_ids.contains(&meta.id) {
            continue;
        }
        let session_harness_id = (!meta.harness.is_empty()).then_some(meta.harness.as_str());
        let resolved_harness = session_harness_id
            .and_then(|id| registry.get(id))
            .or_else(|| registry.get("generic-shell"));
        let (memory_state, resume_strategy) =
            derive_memory_state(false, meta.resume.as_ref(), resolved_harness);
        let (resume_exact, resume_latest_cwd, resume_latest_repo) = resolved_harness
            .map(ResolvedHarness::resume_flags)
            .unwrap_or_default();
        infos.push(SessionInfo {
            id: meta.id.clone(),
            label: meta.label.clone(),
            harness_id: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
            model_provider_id: meta.provider_id.clone(),
            model_id: (!meta.model.is_empty()).then(|| meta.model.clone()),
            harness: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
            model: (!meta.model.is_empty()).then(|| meta.model.clone()),
            work_phase: meta.work_phase.clone(),
            lifecycle_phase: meta.lifecycle_phase.clone(),
            lifecycle: meta.lifecycle.clone(),
            attention: meta.attention.clone(),
            status: meta.status.clone(),
            connectivity: Some(connectivity_for_status(&meta.status).into()),
            terminal_outcome: terminal_outcome_for_status(&meta.status),
            cwd: meta.cwd.clone(),
            created_at: meta.created_at.clone(),
            last_activity_at: Some(meta.last_activity.clone()),
            last_output_at: meta.last_output_at.clone(),
            final_observed_status: meta
                .final_observed_status_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.value.clone()),
            observed_status: meta.observed_status.clone(),
            summary: meta.summary.clone(),
            next_action: meta.next_action.clone(),
            needs_user_input: meta.needs_user_input,
            detected_question: meta.detected_question.clone(),
            suggested_options: meta.suggested_options.clone(),
            blocker_description: meta.blocker_description.clone(),
            failed_command: meta.failed_command.clone(),
            failed_test: meta.failed_test.clone(),
            capacity_hints: meta.capacity_hints.clone(),
            at_usage_limit: None,
            capacity_check_pending: None,
            usage_limit_reset_hint: None,
            metadata_source: Some(meta.metadata_source.clone()),
            metadata_confidence: Some(meta.metadata_confidence),
            peon_last_inference: meta.peon_last_inference.clone(),
            repo_root: meta.repo_root.clone(),
            branch: meta.branch.clone(),
            dirty: meta.dirty,
            changed_files: meta.changed_files,
            is_worktree: meta.is_worktree,
            conflict_warning: None,
            recommendation: None,
            memory_state,
            resume_strategy: resume_strategy.clone(),
            resume: meta.resume.clone(),
            resume_options: metadata::derive_resume_options(
                &resume_strategy,
                meta.resume.as_ref(),
                resume_exact,
                resume_latest_cwd,
                resume_latest_repo,
            ),
            resumed_from: meta.resumed_from.clone(),
            has_openable_plan: meta.plan_path.as_ref().and_then(|reference| {
                workspace_root
                    .as_deref()
                    .map(|root| resolve_openable_plan_reference(root, reference).is_ok())
            }),
            provider: meta.provider_label.clone(),
            provider_model: meta.provider_model.clone(),
            provider_state: meta.provider_state.clone(),
        });
    }

    // Write back newly latched usage limits so they survive ring buffer scroll-off.
    {
        let mut sessions = state.sessions.lock().unwrap();
        for info in &infos {
            if let Some(handle) = sessions.get_mut(&info.id) {
                let Some((latched, origin, lines, bytes)) = capacity_snapshots.get(&info.id) else {
                    continue;
                };
                if handle.at_usage_limit_latched != *latched
                    || handle.resume_scan_origin != *origin
                    || handle.output_lines_seen != *lines
                    || handle.scan_bytes_seen != *bytes
                {
                    continue;
                }
                if info.at_usage_limit == Some(true) {
                    if !handle.at_usage_limit_latched {
                        handle.runtime.usage_limit_latched_at = handle
                            .info
                            .last_output_at
                            .as_deref()
                            .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
                            .map(|timestamp| timestamp.with_timezone(&chrono::Utc));
                    }
                    handle.at_usage_limit_latched = true;
                }
                if let Some(origin) = capped_clear_baselines.get(&info.id) {
                    handle.resume_scan_origin = Some(*origin);
                    handle.at_usage_limit_latched = false;
                } else if capped_recheck_resets.contains(&info.id) {
                    handle.resume_scan_origin = None;
                }
            }
        }
        for (id, has_fresh_resume_output, pending_visible_once) in &pending_transitions {
            let Some(handle) = sessions.get_mut(id) else {
                continue;
            };
            if !handle.capacity_check_pending {
                continue;
            }
            if *pending_visible_once {
                handle.capacity_check_pending = false;
                handle.resume_scan_origin = None;
                handle.pending_capacity_visible_once = false;
                handle.info.capacity_check_pending = None;
            } else if *has_fresh_resume_output {
                handle.pending_capacity_visible_once = true;
                handle.resume_scan_origin = None;
                handle.info.capacity_check_pending = Some(true);
            } else {
                handle.info.capacity_check_pending = Some(true);
            }
        }
    }

    // Propagate capacity state across all live sessions sharing a harness.
    // Remembered sessions keep their own frozen terminal state; only the
    // provider row should reflect another live session's capped runtime state.
    let mut harness_capped: HashMap<String, bool> = HashMap::new();
    let mut harness_reset_hint: HashMap<String, String> = HashMap::new();
    let mut provider_checking: HashSet<String> = HashSet::new();
    for info in &infos {
        if let (Some(hid), Some(capped)) = (&info.harness_id, info.at_usage_limit) {
            let entry = harness_capped.entry(hid.clone()).or_insert(false);
            *entry = *entry || capped;
        }
        if let (Some(hid), Some(hint)) = (&info.harness_id, &info.usage_limit_reset_hint) {
            harness_reset_hint
                .entry(hid.clone())
                .or_insert_with(|| hint.clone());
        }
        // Keyed by harness id, matching harness_capped above — the checking
        // state masks the capped display, so both must land on the same
        // provider row even when the session's model provider differs.
        if info.capacity_check_pending == Some(true) {
            if let Some(hid) = &info.harness_id {
                provider_checking.insert(hid.clone());
            }
        }
    }
    if !harness_capped.is_empty() {
        for info in &mut infos {
            if info.memory_state != MemoryState::Live {
                continue;
            }
            if let Some(ref hid) = info.harness_id {
                if let Some(&capped) = harness_capped.get(hid) {
                    info.at_usage_limit = Some(capped);
                    if capped && info.lifecycle == "alive" {
                        info.attention = Some("capped".into());
                    }
                }
                if info.usage_limit_reset_hint.is_none() {
                    if let Some(hint) = harness_reset_hint.get(hid) {
                        info.usage_limit_reset_hint = Some(hint.clone());
                    }
                }
            }
        }
    }
    state
        .providers
        .update_session_capping(harness_capped, harness_reset_hint, provider_checking);

    let session_pids = state.session_pids.lock().unwrap().clone();
    let reported_cwds = state.peon.reported_cwd.read().unwrap().clone();
    let effective_cwds = resolve_effective_cwds(
        &infos,
        &reported_cwds,
        &session_pids,
        crate::procfs::live_cwds,
    );
    enrich_sessions_with_git_context(&mut infos, &effective_cwds, git::detect);

    let conflict_warnings = detect_conflicts(&infos, &effective_cwds);
    for info in &mut infos {
        info.conflict_warning = conflict_warnings
            .iter()
            .find(|(id, _)| id == &info.id)
            .map(|(_, w)| w.clone());
    }
    Json(infos)
}

pub(crate) async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let handle = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&id).map(|h| h.kill_tx.clone())
    };
    match handle {
        Some(kill_tx) => {
            crate::runtime::terminal_runtime::set_session_status(&state, &id, "killed");
            let _ = kill_tx.send(true);
        }
        None => return axum::http::StatusCode::NOT_FOUND,
    }
    crate::runtime::session_runtime::clear_ended_session_tracking(&state, &id);
    axum::http::StatusCode::OK
}

pub(crate) async fn forget_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let ws_guard = state.workspace.lock().unwrap();
    let ws = match &*ws_guard {
        Some(ws) => ws,
        None => return axum::http::StatusCode::CONFLICT.into_response(),
    };
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(h) = sessions.get(&id) {
        if h.info.status == "live" || h.info.status == "creating" || h.info.status == "running" {
            return (
                axum::http::StatusCode::CONFLICT,
                "Cannot forget a live session. Kill it first.",
            )
                .into_response();
        }
    }

    // Existence, not parseability: a corrupt-but-present metadata file must
    // still be forgettable, or the session becomes undeletable via the API.
    if !ws.metadata.session_file_exists(&id) {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }

    if let Err(error) = crate::runtime::retention::delete_session_evidence(ws, &id, |session_id| {
        ws.recommendation_store
            .delete_referencing_session(session_id)
            .map_err(|error| error.to_string())
    }) {
        tracing::error!(session_id = %id, %error, "failed to delete session evidence");
        if !ws.metadata.session_file_exists(&id) {
            sessions.remove(&id);
            drop(ws_guard);
            drop(sessions);
            crate::runtime::session_runtime::clear_ended_session_tracking(&state, &id);
            crate::runtime::session_runtime::clear_forgotten_session_tracking(&state, &id);
        }
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    drop(ws_guard);

    sessions.remove(&id);
    drop(sessions);
    crate::runtime::session_runtime::clear_ended_session_tracking(&state, &id);
    crate::runtime::session_runtime::clear_forgotten_session_tracking(&state, &id);

    axum::http::StatusCode::OK.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::terminal_runtime::set_session_status;
    use crate::test_support::*;

    static PLAN_TOKEN_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// `create_session` now returns before the detached spawn completes (see
    /// issue #302), so any test that needs the session past its `"creating"`
    /// interval — e.g. to call an endpoint that requires `lifecycle ==
    /// "alive"` — must wait for the spawn like a real client polling would.
    async fn wait_for_session_status(state: &Arc<AppState>, id: &str, status: &str) {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let matches = state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(id)
                    .is_some_and(|handle| handle.info.status == status);
                if matches {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("session {id} did not reach status {status:?} in time"));
    }

    #[test]
    fn session_git_context_is_resolved_once_per_unique_cwd() {
        let shared = "/workspace/shared";
        let separate = "/workspace/separate";
        let mut infos = vec![
            test_session_info("one", "One", shared, "running", "now"),
            test_session_info("two", "Two", shared, "running", "now"),
            test_session_info("three", "Three", separate, "ended", "now"),
        ];
        let mut calls: HashMap<String, usize> = HashMap::new();

        enrich_sessions_with_git_context(&mut infos, &HashMap::new(), |cwd| {
            *calls.entry(cwd.display().to_string()).or_default() += 1;
            git::GitContext {
                repo_root: Some(format!("{}/repo", cwd.display())),
                branch: Some("test-branch".into()),
                dirty: true,
                changed_files: 2,
                is_worktree: cwd == std::path::Path::new(separate),
            }
        });

        assert_eq!(calls.get(shared), Some(&1));
        assert_eq!(calls.get(separate), Some(&1));
        assert_eq!(calls.len(), 2);
        assert_eq!(
            infos[0].repo_root.as_deref(),
            Some("/workspace/shared/repo")
        );
        assert_eq!(infos[1].branch.as_deref(), Some("test-branch"));
        assert_eq!(infos[1].dirty, Some(true));
        assert_eq!(infos[1].changed_files, Some(2));
        assert_eq!(infos[2].is_worktree, Some(true));
        assert!(infos[0].recommendation.is_some());
    }

    #[test]
    fn session_git_context_uses_supplied_effective_cwd_over_launch_cwd() {
        let launch_cwd = "/workspace/launched-here";
        let mut infos = vec![
            test_session_info("moved", "Moved", launch_cwd, "running", "now"),
            test_session_info("stayed", "Stayed", launch_cwd, "running", "now"),
        ];
        // "moved" resolved to a live cwd distinct from its launch cwd (e.g. via
        // `resolve_effective_cwds`); "stayed" has no entry, so it should fall
        // back to its launch cwd.
        let effective_cwds: HashMap<String, String> =
            HashMap::from([("moved".to_string(), "/workspace/worktree".to_string())]);
        let mut detect_calls: Vec<String> = Vec::new();

        enrich_sessions_with_git_context(&mut infos, &effective_cwds, |cwd| {
            detect_calls.push(cwd.display().to_string());
            git::GitContext {
                repo_root: Some(format!("{}/repo", cwd.display())),
                branch: Some(format!("branch-for-{}", cwd.display())),
                dirty: false,
                changed_files: 0,
                is_worktree: false,
            }
        });

        assert_eq!(
            infos[0].branch.as_deref(),
            Some("branch-for-/workspace/worktree")
        );
        assert_eq!(
            infos[1].branch.as_deref(),
            Some(format!("branch-for-{launch_cwd}").as_str())
        );
        assert!(detect_calls.contains(&"/workspace/worktree".to_string()));
        assert!(detect_calls.contains(&launch_cwd.to_string()));
    }

    #[test]
    fn session_git_context_falls_back_to_launch_cwd_when_not_in_effective_cwds() {
        let launch_cwd = "/workspace/no-entry-tracked";
        let mut infos = vec![test_session_info(
            "untracked",
            "Untracked",
            launch_cwd,
            "running",
            "now",
        )];

        enrich_sessions_with_git_context(&mut infos, &HashMap::new(), |cwd| git::GitContext {
            repo_root: Some(cwd.display().to_string()),
            branch: None,
            dirty: false,
            changed_files: 0,
            is_worktree: false,
        });

        assert_eq!(infos[0].repo_root.as_deref(), Some(launch_cwd));
    }

    fn attention_test_handle(id: &str, cwd: &std::path::Path) -> SessionHandle {
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        SessionHandle {
            info: test_session_info(id, "Known", cwd.display().to_string(), "running", "now"),
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

    fn orphan_test_metadata(id: &str, workspace: &std::path::Path) -> metadata::SessionMetadata {
        let workspace = workspace.display().to_string();
        metadata::SessionMetadata {
            id: id.into(),
            label: "Test".into(),
            workspace: workspace.clone(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: workspace,
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
        }
    }

    /// A daemon restart truly does empty `state.sessions`, so a session
    /// found "running" in persisted metadata with no matching in-memory
    /// handle is genuinely orphaned and must be reconciled to "ended".
    #[tokio::test]
    async fn set_workspace_reconciles_running_session_with_no_live_handle() {
        let home_dir = tempfile::tempdir().unwrap();
        let workspace_dir = tempfile::tempdir().unwrap();
        let _home = FakeHome::set(home_dir.path());

        let state = test_app_state_with_workspace(workspace_dir.path());
        let global_dir = orkworks_global_dir(workspace_dir.path()).unwrap();
        std::fs::create_dir_all(global_dir.join("sessions")).unwrap();
        metadata::MetadataStore::new(&global_dir)
            .write_session(&orphan_test_metadata("orphaned", workspace_dir.path()));

        let response = set_workspace(
            State(state.clone()),
            Json(WorkspaceRequest {
                path: workspace_dir.path().display().to_string(),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let ws_guard = state.workspace.lock().unwrap();
        let reloaded = ws_guard
            .as_ref()
            .unwrap()
            .metadata
            .read_session("orphaned")
            .unwrap();
        assert_eq!(reloaded.status, "ended");
    }

    /// Only the sidecar (`orkworksd`) itself restarting empties
    /// `state.sessions`. When just the Electron app restarts and
    /// reconnects to an already-running sidecar, `set_workspace` runs
    /// again against a sidecar whose in-memory session handles are still
    /// alive — a session with a live handle here must not be reconciled
    /// to "ended" out from under its still-running process.
    #[tokio::test]
    async fn set_workspace_does_not_reconcile_session_with_live_in_memory_handle() {
        let home_dir = tempfile::tempdir().unwrap();
        let workspace_dir = tempfile::tempdir().unwrap();
        let _home = FakeHome::set(home_dir.path());

        let state = test_app_state_with_workspace(workspace_dir.path());
        let global_dir = orkworks_global_dir(workspace_dir.path()).unwrap();
        std::fs::create_dir_all(global_dir.join("sessions")).unwrap();
        metadata::MetadataStore::new(&global_dir)
            .write_session(&orphan_test_metadata("alive", workspace_dir.path()));

        state.sessions.lock().unwrap().insert(
            "alive".into(),
            attention_test_handle("alive", workspace_dir.path()),
        );

        let response = set_workspace(
            State(state.clone()),
            Json(WorkspaceRequest {
                path: workspace_dir.path().display().to_string(),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let ws_guard = state.workspace.lock().unwrap();
        let reloaded = ws_guard
            .as_ref()
            .unwrap()
            .metadata
            .read_session("alive")
            .unwrap();
        assert_eq!(
            reloaded.status, "running",
            "a session with a live in-memory handle must not be reconciled as orphaned"
        );
    }

    #[tokio::test]
    async fn harness_session_report_rejects_invalid_native_id() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_harness_session(
            State(state),
            Path("missing".into()),
            Json(HarnessSessionReportRequest {
                harness_session_id: "bad id".into(),
                source: "test".into(),
                confidence: 0.9,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn harness_session_report_returns_not_found_for_unknown_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_harness_session(
            State(state),
            Path("missing".into()),
            Json(HarnessSessionReportRequest {
                harness_session_id: "native-123".into(),
                source: "test".into(),
                confidence: 0.9,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn harness_session_report_writes_metadata_for_known_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: "known".into(),
                    label: "Known".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "opencode".into(),
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

        let response = report_harness_session(
            State(state.clone()),
            Path("known".into()),
            Json(HarnessSessionReportRequest {
                harness_session_id: "native-123".into(),
                source: "opencode_env".into(),
                confidence: 0.98,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws.as_ref().unwrap().metadata.read_session("known").unwrap();
        assert_eq!(
            updated
                .resume
                .as_ref()
                .and_then(|r| r.harness_session_id.as_deref()),
            Some("native-123"),
        );
    }

    #[tokio::test]
    async fn harness_session_report_keeps_resume_memory_in_sync_for_later_status_updates() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "live-known".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("opencode".into()),
                    harness: Some("opencode".into()),
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    resume_strategy: harness::ResumeStrategy::LatestCwd,
                    resume: Some(resume.clone()),
                    ..test_session_info(
                        session_id.clone(),
                        "Known",
                        dir.path().display().to_string(),
                        "running",
                        "before",
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
            },
        );

        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Known".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "opencode".into(),
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
                    created_at: "before".into(),
                    last_activity: "before".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: Some(resume),
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
        }

        let response = report_harness_session(
            State(state.clone()),
            Path(session_id.clone()),
            Json(HarnessSessionReportRequest {
                harness_session_id: "native-123".into(),
                source: "opencode_env".into(),
                confidence: 0.98,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        set_session_status(&state, &session_id, "ended");

        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&session_id)
            .unwrap();
        let updated_resume = updated.resume.unwrap();
        assert_eq!(
            updated_resume.harness_session_id.as_deref(),
            Some("native-123")
        );
        assert_ne!(updated_resume.last_seen_at.as_deref(), Some("before"));
    }

    #[test]
    fn resume_handle_conflicts_for_metadata_pid_attachment_and_claim() {
        let dir = tempfile::tempdir().unwrap();
        let mut handle = attention_test_handle("resume-stale-predicate", dir.path());
        handle.info.lifecycle_phase = "active".into();
        let mut session_pids = HashMap::new();

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

    #[tokio::test]
    async fn resume_session_replaces_unattached_ended_stale_handle() {
        use crate::test_support::FakePath;
        #[cfg(unix)]
        use crate::test_support::make_test_executable;

        let dir = tempfile::tempdir().unwrap();
        let fake_bin_dir = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        let _opencode = {
            let path = fake_bin_dir.path().join("opencode");
            std::fs::write(&path, "#!/bin/sh\nexec sleep 30\n").unwrap();
            make_test_executable(&path);
            path
        };
        #[cfg(windows)]
        let _opencode = {
            let path = fake_bin_dir.path().join("opencode.cmd");
            std::fs::write(
                &path,
                "@echo off\r\n%ComSpec% /c timeout /T 30 /NOBREAK >nul\r\n",
            )
            .unwrap();
            path
        };
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-stale-ended".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        let mut info = test_session_info(
            session_id.clone(),
            "Resume Stale Ended",
            dir.path().display().to_string(),
            "running",
            "before",
        );
        info.harness_id = Some("opencode".into());
        info.harness = Some("opencode".into());
        info.resume_strategy = harness::ResumeStrategy::LatestCwd;
        info.resume = Some(resume.clone());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
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

        let mut metadata = test_session_metadata(
            session_id.clone(),
            "Resume Stale Ended",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.harness = "opencode".into();
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "ended".into();
        metadata.resume = Some(resume);
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.write_session(&metadata);
        }

        let response = resume_session(State(state.clone()), Path(session_id.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(state.sessions.lock().unwrap()[&session_id].resume_in_progress);

        crate::runtime::session_runtime::send_runtime_command(
            &state,
            &session_id,
            crate::runtime::session_runtime::RuntimeCommand::Kill,
        )
        .await
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if !state.sessions.lock().unwrap()[&session_id].resume_in_progress {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("terminal finalization clears the runtime claim");
    }

    #[tokio::test]
    async fn cancelled_resume_after_spawn_keeps_live_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (program, args) = if cfg!(windows) {
            (
                "cmd.exe".to_string(),
                vec!["/C".to_string(), "timeout /T 5 /NOBREAK".to_string()],
            )
        } else {
            ("sh".to_string(), vec!["-c".to_string(), "exec sleep 5".to_string()])
        };
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                let override_patch = document.overrides.entry("opencode".into()).or_default();
                override_patch.resume = Some(Some(harness::definition::ResumePatch {
                    latest_cwd: Some(Some(harness::CommandTemplate {
                        command: program,
                        args,
                    })),
                    ..Default::default()
                }));
                Ok(())
            })
            .unwrap();

        let session_id = "resume-cancelled-after-spawn".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let mut metadata = test_session_metadata(
            session_id.clone(),
            "Cancelled Resume",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.harness = "opencode".into();
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        metadata.resume = Some(resume);
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&metadata);
            ws.metadata.write_terminal_size(&session_id, 120, 40);
        }

        let (startup_checked, resume_startup) =
            crate::runtime::session_runtime::pause_startup_after_ending_check(session_id.clone());
        let task = tokio::spawn(resume_session(State(state.clone()), Path(session_id.clone())));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if state.session_pids.lock().unwrap().contains_key(&session_id) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("resumed child reaches the PTY spawn boundary");
        startup_checked.await.unwrap();

        task.abort();
        assert!(matches!(task.await, Err(error) if error.is_cancelled()));
        let _ = resume_startup.send(());

        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let active = state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&session_id)
                    .is_some_and(|handle| {
                        handle.info.status == "running"
                            && handle.info.lifecycle_phase == "active"
                    });
                if active {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("detached startup continues after request cancellation");

        let replacement_generation = state.sessions.lock().unwrap()[&session_id]
            .runtime
            .run_generation();
        assert!(state.sessions.lock().unwrap()[&session_id].resume_in_progress);
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        assert_ne!(ws.metadata.read_session(&session_id).unwrap().status, "ended");
        assert_eq!(ws.metadata.read_terminal_size(&session_id), None);
        drop(ws_guard);

        crate::runtime::session_runtime::send_runtime_command(
            &state,
            &session_id,
            crate::runtime::session_runtime::RuntimeCommand::Kill,
        )
        .await
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&session_id)
                    .is_some_and(|handle| !handle.resume_in_progress)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("terminal finalization releases the replacement claim");
        assert_eq!(
            state.sessions.lock().unwrap()[&session_id]
                .runtime
                .run_generation(),
            replacement_generation
        );
    }

    #[tokio::test]
    async fn delete_during_startup_finalizes_same_generation() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (program, args) = if cfg!(windows) {
            (
                "cmd.exe".to_string(),
                vec!["/C".to_string(), "timeout /T 30 /NOBREAK".to_string()],
            )
        } else {
            ("sh".to_string(), vec!["-c".to_string(), "exec sleep 30".to_string()])
        };
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                let override_patch = document.overrides.entry("opencode".into()).or_default();
                override_patch.resume = Some(Some(harness::definition::ResumePatch {
                    latest_cwd: Some(Some(harness::CommandTemplate {
                        command: program,
                        args,
                    })),
                    ..Default::default()
                }));
                Ok(())
            })
            .unwrap();

        let session_id = "delete-during-startup".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let mut metadata = test_session_metadata(session_id.clone(), "Delete During Startup", dir.path().display().to_string(), "ended", "before", "before");
        metadata.harness = "opencode".into();
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        metadata.resume = Some(resume);
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&metadata);

        let (checked_rx, resume_tx) =
            crate::runtime::session_runtime::pause_startup_after_ending_check(session_id.clone());
        let resume_task = tokio::spawn(resume_session(State(state.clone()), Path(session_id.clone())));
        tokio::time::timeout(std::time::Duration::from_secs(5), checked_rx)
            .await
            .expect("startup reaches the post-check transition gap")
            .expect("startup test hook remains installed");

        let response = delete_session(State(state.clone()), Path(session_id.clone())).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        resume_tx
            .send(())
            .expect("startup is waiting to attempt the running transition");

        let response = tokio::time::timeout(std::time::Duration::from_secs(5), resume_task)
            .await
            .expect("startup request returns after its generation is finalized")
            .expect("startup task does not panic");
        assert_eq!(response.into_response().status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);

        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let finalized = state.sessions.lock().unwrap().get(&session_id).is_some_and(|handle| {
                    handle.info.status == "killed"
                        && handle.info.lifecycle_phase == "ended"
                        && !handle.resume_in_progress
                });
                if finalized {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("deleted startup generation is finalized");

        let metadata = state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_session(&session_id).unwrap();
        assert_eq!(metadata.status, "killed");
        assert_eq!(metadata.lifecycle_phase, "ended");
        assert!(!state.session_pids.lock().unwrap().contains_key(&session_id));
        assert!(!state.peon.last_output.read().unwrap().contains_key(&session_id));
    }

    #[tokio::test]
    async fn resume_session_startup_failure_eventually_clears_runtime_claim() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
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
        let session_id = "resume-startup-failure".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let mut metadata = test_session_metadata(
            session_id.clone(),
            "Resume Startup Failure",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.harness = "opencode".into();
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        metadata.resume = Some(resume);
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let response = resume_session(State(state.clone()), Path(session_id.clone()))
            .await
            .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let claim_cleared = state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&session_id)
                    .is_some_and(|handle| !handle.resume_in_progress);
                if claim_cleared {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("startup-failure finalization clears the runtime claim");
        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions[&session_id].info.status, "error");
        assert_eq!(sessions[&session_id].info.lifecycle_phase, "ended");
        assert!(!sessions[&session_id].resume_in_progress);
    }

    #[test]
    fn resume_admission_installs_one_claimed_runtime_after_both_callers_observe_ended_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-no-handle-concurrent".to_string();
        let mut metadata = test_session_metadata(
            &session_id,
            "Concurrent Resume",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let mut callers = Vec::new();

        for _ in 0..2 {
            let state = state.clone();
            let session_id = session_id.clone();
            let cwd = dir.path().to_path_buf();
            let barrier = barrier.clone();
            callers.push(std::thread::spawn(move || {
                // Both request paths have already read the same ended metadata
                // before either is allowed to enter atomic admission.
                let metadata_ended = true;
                let replacement = attention_test_handle(&session_id, &cwd);
                barrier.wait();
                try_install_claimed_resume_handle(
                    &state,
                    &session_id,
                    replacement,
                    metadata_ended,
                    None,
                )
            }));
        }

        barrier.wait();
        let results = callers
            .into_iter()
            .map(|caller| caller.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains_key(&session_id));
        assert!(sessions[&session_id].resume_in_progress);
    }

    #[test]
    fn resume_admission_rejects_a_stale_observation_after_the_first_claim_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-stale-observation";

        let mut metadata = test_session_metadata(
            session_id,
            "Stale Resume",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        let mut old_handle = attention_test_handle(session_id, dir.path());
        old_handle.info.status = "ended".into();
        old_handle.info.lifecycle_phase = "ended".into();
        old_handle.info.lifecycle = "dead".into();
        let observed_generation = old_handle.runtime.run_generation();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), old_handle);

        let first = try_install_claimed_resume_handle(
            &state,
            session_id,
            attention_test_handle(session_id, dir.path()),
            true,
            Some(observed_generation),
        )
        .unwrap();
        first.commit();

        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut(session_id).unwrap();
            handle.resume_in_progress = false;
            handle.info.status = "ended".into();
            handle.info.lifecycle_phase = "ended".into();
            handle.info.lifecycle = "dead".into();
        }

        let second = try_install_claimed_resume_handle(
            &state,
            session_id,
            attention_test_handle(session_id, dir.path()),
            true,
            Some(observed_generation),
        );

        assert!(second.is_err());
    }

    #[tokio::test]
    async fn late_old_runtime_exit_is_a_noop_after_resume_replaces_its_generation() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-generation-replacement";
        let mut metadata = test_session_metadata(
            session_id,
            "Replacement",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);
        let old_handle = attention_test_handle(session_id, dir.path());
        let old_generation = old_handle.runtime.run_generation();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), old_handle);

        let replacement = attention_test_handle(session_id, dir.path());
        let admission = try_install_claimed_resume_handle(
            &state,
            session_id,
            replacement,
            true,
            Some(old_generation),
        )
        .unwrap();
        let replacement_generation = admission.generation();
        admission.commit();
        assert!(replacement_generation > old_generation);

        let mut metadata = test_session_metadata(
            session_id,
            "Replacement",
            dir.path().display().to_string(),
            "running",
            "before",
            "before",
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
            .session_pids
            .lock()
            .unwrap()
            .insert(session_id.to_string(), 4242);
        state
            .peon
            .last_output
            .write()
            .unwrap()
            .insert(session_id.to_string(), tokio::time::Instant::now());
        state
            .peon
            .last_inference
            .write()
            .unwrap()
            .insert(session_id.to_string(), "replacement inference".into());
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert(session_id.to_string(), "replacement input".into());

        assert!(!crate::runtime::session_runtime::handle_runtime_exit(
            &state,
            session_id,
            old_generation,
            "ended",
        )
        .await);
        tokio::task::yield_now().await;

        let sessions = state.sessions.lock().unwrap();
        let replacement = &sessions[session_id];
        assert_eq!(replacement.runtime.run_generation(), replacement_generation);
        assert_eq!(replacement.info.status, "running");
        assert_eq!(replacement.info.lifecycle_phase, "active");
        assert!(replacement.resume_in_progress);
        drop(sessions);
        assert_eq!(state.session_pids.lock().unwrap()[session_id], 4242);
        assert!(state.peon.last_output.read().unwrap().contains_key(session_id));
        assert_eq!(
            state.peon.last_inference.read().unwrap()[session_id],
            "replacement inference"
        );
        assert_eq!(
            state.peon.input_buf.read().unwrap()[session_id],
            "replacement input"
        );
        let persisted = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(session_id)
            .unwrap();
        assert_eq!(persisted.status, "running");
        assert_eq!(persisted.lifecycle_phase, "active");

        // A finalizer that was already scheduled by the old generation must
        // also be harmless if the replacement independently enters ending.
        {
            let mut sessions = state.sessions.lock().unwrap();
            let replacement = sessions.get_mut(session_id).unwrap();
            replacement.info.lifecycle_phase = "ending".into();
            replacement.info.lifecycle = "stopping".into();
        }
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            let mut ending = ws.metadata.read_session(session_id).unwrap();
            ending.lifecycle_phase = "ending".into();
            ending.lifecycle = "stopping".into();
            ending.pending_terminal_status = Some("killed".into());
            ws.metadata.write_session(&ending);
        }
        crate::runtime::terminal_runtime::finalize_session_ending(
            state.clone(),
            session_id.to_string(),
            old_generation,
            "ended".into(),
        )
        .await;
        let sessions = state.sessions.lock().unwrap();
        let replacement = &sessions[session_id];
        assert_eq!(replacement.runtime.run_generation(), replacement_generation);
        assert_eq!(replacement.info.lifecycle_phase, "ending");
        assert!(replacement.resume_in_progress);
        drop(sessions);
        let persisted = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(session_id)
            .unwrap();
        assert_eq!(persisted.lifecycle_phase, "ending");
        assert_eq!(persisted.pending_terminal_status.as_deref(), Some("killed"));
    }

    #[test]
    fn cancelled_resume_admission_restores_prior_runtime_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-cancelled-admission";
        let mut old_handle = attention_test_handle(session_id, dir.path());
        old_handle.info.status = "ended".into();
        old_handle.info.lifecycle_phase = "ended".into();
        old_handle.info.lifecycle = "dead".into();
        let old_generation = old_handle.runtime.run_generation();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), old_handle);

        let mut metadata = test_session_metadata(
            session_id,
            "Prior Runtime",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ended".into();
        metadata.lifecycle = "dead".into();
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        ws.metadata.write_session(&metadata);
        ws.metadata.write_terminal_size(session_id, 120, 40);
        drop(ws_guard);

        let replacement = attention_test_handle(session_id, dir.path());
        let mut admission = try_install_claimed_resume_handle(
            &state,
            session_id,
            replacement,
            true,
            Some(old_generation),
        )
        .unwrap();
        admission.arm_rollback(
            dir.path().to_path_buf(),
            metadata.clone(),
            Some((120, 40)),
        );
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            let mut creating = metadata.clone();
            creating.status = "creating".into();
            creating.lifecycle_phase = "creating".into();
            ws.metadata.write_session(&creating);
            ws.metadata.clear_terminal_size(session_id);
        }

        // Dropping the request future drops its admission guard.
        drop(admission);

        let sessions = state.sessions.lock().unwrap();
        let restored = &sessions[session_id];
        assert_eq!(restored.runtime.run_generation(), old_generation);
        assert_eq!(restored.info.lifecycle_phase, "ended");
        assert!(!restored.resume_in_progress);
        drop(sessions);
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        let restored_metadata = ws.metadata.read_session(session_id).unwrap();
        assert_eq!(restored_metadata.status, "ended");
        assert_eq!(restored_metadata.lifecycle_phase, "ended");
        assert_eq!(ws.metadata.read_terminal_size(session_id), Some((120, 40)));
    }

    #[test]
    fn cancelled_resume_does_not_overwrite_newer_generation_metadata_between_rollback_stages() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-cancelled-interleaving";
        let mut old_handle = attention_test_handle(session_id, dir.path());
        old_handle.info.status = "ended".into();
        old_handle.info.lifecycle_phase = "ended".into();
        old_handle.info.lifecycle = "dead".into();
        let old_generation = old_handle.runtime.run_generation();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), old_handle);

        let mut old_metadata = test_session_metadata(
            session_id,
            "Prior Runtime",
            dir.path().display().to_string(),
            "ended",
            "before",
            "before",
        );
        old_metadata.lifecycle_phase = "ended".into();
        old_metadata.lifecycle = "dead".into();
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&old_metadata);
            ws.metadata.write_terminal_size(session_id, 120, 40);
        }

        let replacement = attention_test_handle(session_id, dir.path());
        let mut cancelled_admission = try_install_claimed_resume_handle(
            &state,
            session_id,
            replacement,
            true,
            Some(old_generation),
        )
        .unwrap();
        cancelled_admission.arm_rollback(
            dir.path().to_path_buf(),
            old_metadata.clone(),
            Some((120, 40)),
        );
        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            let mut creating = old_metadata.clone();
            creating.status = "creating".into();
            creating.lifecycle_phase = "creating".into();
            creating.lifecycle = "creating".into();
            ws.metadata.write_session(&creating);
            ws.metadata.clear_terminal_size(session_id);
        }

        // Hold the workspace lock so cancellation must pause after restoring
        // the prior handle and before attempting its persisted rollback.
        let ws_guard = state.workspace.lock().unwrap();
        let drop_thread = std::thread::spawn(move || drop(cancelled_admission));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let restored = state
                .sessions
                .lock()
                .unwrap()
                .get(session_id)
                .is_some_and(|handle| handle.runtime.run_generation() == old_generation);
            if restored {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "cancelled admission did not restore its prior handle"
            );
            std::thread::yield_now();
        }

        let newer_handle = attention_test_handle(session_id, dir.path());
        let newer_admission = try_install_claimed_resume_handle(
            &state,
            session_id,
            newer_handle,
            true,
            Some(old_generation),
        )
        .unwrap();
        let newer_generation = newer_admission.generation();
        let ws = ws_guard.as_ref().unwrap();
        let mut newer_metadata = old_metadata.clone();
        newer_metadata.label = "Newer Runtime".into();
        newer_metadata.status = "running".into();
        newer_metadata.lifecycle_phase = "active".into();
        newer_metadata.lifecycle = "alive".into();
        ws.metadata.write_session(&newer_metadata);
        ws.metadata.write_terminal_size(session_id, 150, 50);
        newer_admission.commit();
        drop(ws_guard);
        drop_thread.join().unwrap();

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[session_id].runtime.run_generation(),
            newer_generation
        );
        drop(sessions);
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        let persisted = ws.metadata.read_session(session_id).unwrap();
        assert_eq!(persisted.label, "Newer Runtime");
        assert_eq!(persisted.status, "running");
        assert_eq!(persisted.lifecycle_phase, "active");
        assert_eq!(ws.metadata.read_terminal_size(session_id), Some((150, 50)));
    }

    #[test]
    fn resume_admission_rejects_active_metadata_without_a_session_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-no-handle-active-metadata";
        let replacement = attention_test_handle(session_id, dir.path());

        let result = try_install_claimed_resume_handle(&state, session_id, replacement, false, None);

        assert!(result.is_err());
        assert!(!state.sessions.lock().unwrap().contains_key(session_id));
    }

    #[test]
    fn resume_admission_rejects_tracked_pid_without_a_session_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-no-handle-tracked-pid";
        state
            .session_pids
            .lock()
            .unwrap()
            .insert(session_id.to_string(), 42);
        let replacement = attention_test_handle(session_id, dir.path());

        let result = try_install_claimed_resume_handle(&state, session_id, replacement, true, None);

        assert!(result.is_err());
        assert!(!state.sessions.lock().unwrap().contains_key(session_id));
    }

    #[tokio::test]
    async fn resume_admission_waits_for_ending_handle_finalization_before_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-ending-finalization";
        let mut ending_handle = attention_test_handle(session_id, dir.path());
        ending_handle.info.lifecycle_phase = "ending".into();
        ending_handle.info.lifecycle = "stopping".into();
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), ending_handle);
        let mut metadata = test_session_metadata(
            session_id,
            "Resume Ending Finalization",
            dir.path().display().to_string(),
            "running",
            "before",
            "before",
        );
        metadata.lifecycle_phase = "ending".into();
        metadata.lifecycle = "stopping".into();
        metadata.pending_terminal_status = Some("ended".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        // The request-side metadata snapshot may already say `ended` while
        // the old finalizer is waiting to update the in-memory handle.
        let generation = state.sessions.lock().unwrap()[session_id]
            .runtime
            .run_generation();
        let premature_replacement = attention_test_handle(session_id, dir.path());
        let premature_result = try_install_claimed_resume_handle(
            &state,
            session_id,
            premature_replacement,
            true,
            Some(generation),
        );

        assert!(premature_result.is_err());
        assert_eq!(
            state.sessions.lock().unwrap()[session_id]
                .info
                .lifecycle_phase,
            "ending"
        );

        crate::runtime::terminal_runtime::finalize_session_ending(
            state.clone(),
            session_id.to_string(),
            generation,
            "ended".into(),
        )
        .await;
        assert_eq!(
            state.sessions.lock().unwrap()[session_id]
                .info
                .lifecycle_phase,
            "ended"
        );

        let replacement = attention_test_handle(session_id, dir.path());
        let result = try_install_claimed_resume_handle(
            &state,
            session_id,
            replacement,
            true,
            Some(generation),
        );

        assert!(result.is_ok());
        let sessions = state.sessions.lock().unwrap();
        assert!(sessions[session_id].resume_in_progress);
        assert_ne!(sessions[session_id].info.lifecycle_phase, "ended");
    }

    #[tokio::test]
    async fn resume_session_rejects_attached_live_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-attached".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("opencode".into()),
                    harness: Some("opencode".into()),
                    resume_strategy: harness::ResumeStrategy::LatestCwd,
                    resume: Some(resume.clone()),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Attached",
                        dir.path().display().to_string(),
                        "running",
                        "before",
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
            terminal_attached: true,
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
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Resume Attached".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "opencode".into(),
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
                    created_at: "before".into(),
                    last_activity: "before".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: Some(resume),
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
        }

        let response = resume_session(State(state), Path(session_id))
            .await
            .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn resume_session_rejects_detached_live_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "resume-detached-live".to_string();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("before".into()),
        };
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("opencode".into()),
                    harness: Some("opencode".into()),
                    resume_strategy: harness::ResumeStrategy::LatestCwd,
                    resume: Some(resume.clone()),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Detached Live",
                        dir.path().display().to_string(),
                        "running",
                        "before",
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
            },
        );

        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Resume Detached Live".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "opencode".into(),
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
                    created_at: "before".into(),
                    last_activity: "before".into(),
        last_output_at: None,
                    metadata_source: "process".into(),
                    metadata_confidence: 1.0,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    resume: Some(resume),
                    resume_options: vec![],
                    harness_session_id_source: None,
                    harness_session_id_confidence: None,
                    harness_session_id_captured_at: None,
                    resumed_from: None,
                    last_user_input: None,
                });
        }

        let response = resume_session(State(state), Path(session_id))
            .await
            .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn report_attention_rejects_invalid_status() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_attention(
            State(state),
            Path("missing".into()),
            Json(AttentionReportRequest {
                status: "not_a_real_status".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn report_attention_rejects_malformed_observed_at() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_attention(
            State(state),
            Path("missing".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: Some("not-a-timestamp".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn report_attention_returns_not_found_for_unknown_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = report_attention(
            State(state),
            Path("missing".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn report_attention_writes_metadata_for_known_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: "attention-known".into(),
                    label: "Known".into(),
                    workspace: dir.path().display().to_string(),
                    task: "".into(),
                    harness: "claude-code".into(),
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

        let response = report_attention(
            State(state.clone()),
            Path("attention-known".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("attention-known")
            .unwrap();
        assert_eq!(
            updated.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(updated.metadata_source, "agent");
    }

    #[tokio::test]
    async fn report_attention_stores_reported_cwd_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.write_session(
                &crate::test_support::test_session_metadata(
                    "cwd-report",
                    "CwdReport",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ),
            );
        }

        let response = report_attention(
            State(state.clone()),
            Path("cwd-report".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: Some("/harness-reported/worktree".into()),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            state
                .peon
                .reported_cwd
                .read()
                .unwrap()
                .get("cwd-report")
                .map(String::as_str),
            Some("/harness-reported/worktree")
        );
    }

    #[tokio::test]
    async fn report_attention_does_not_clobber_reported_cwd_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.write_session(
                &crate::test_support::test_session_metadata(
                    "cwd-sticky",
                    "CwdSticky",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ),
            );
        }
        state
            .peon
            .reported_cwd
            .write()
            .unwrap()
            .insert("cwd-sticky".into(), "/previously-reported".into());

        let response = report_attention(
            State(state.clone()),
            Path("cwd-sticky".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            state
                .peon
                .reported_cwd
                .read()
                .unwrap()
                .get("cwd-sticky")
                .map(String::as_str),
            Some("/previously-reported")
        );
    }

    #[tokio::test]
    async fn report_attention_ignores_stale_observed_at_before_side_effects() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "attention-stale-observed-at";
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                id,
                "Known",
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
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        let mut handle = attention_test_handle(id, dir.path());
        handle.info.observed_status = Some("working".into());
        handle.info.attention = Some("working".into());
        handle.info.metadata_source = Some("process".into());
        handle.runtime.accepted_input_at = Some(
            crate::workspace_runtime::parse_hook_observed_at("2026-07-21T08:00:01.000000Z")
                .unwrap(),
        );
        state.sessions.lock().unwrap().insert(id.into(), handle);
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert(id.into(), "y".into());

        let response = report_attention(
            State(state.clone()),
            Path(id.into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: Some("old prompt".into()),
                plan_path: Default::default(),
                observed_at: Some("2026-07-21T08:00:00.000000Z".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let meta = ws.as_ref().unwrap().metadata.read_session(id).unwrap();
        assert_eq!(meta.observed_status.as_deref(), Some("working"));
        drop(ws);
        assert_eq!(
            state.sessions.lock().unwrap()[id].info.attention.as_deref(),
            Some("working")
        );
        assert_eq!(
            state
                .peon
                .input_buf
                .read()
                .unwrap()
                .get(id)
                .map(String::as_str),
            Some("y")
        );
    }

    #[tokio::test]
    async fn report_attention_ignores_an_out_of_order_hook_event() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "attention-hook-order";
        let mut handle = attention_test_handle(id, dir.path());
        handle.active_work_hook = true;
        state.sessions.lock().unwrap().insert(id.into(), handle);
        let ws = state.workspace.lock().unwrap();
        let mut meta = test_session_metadata(id, "Known", dir.path().display().to_string(), "running", "now", "now");
        meta.lifecycle = "alive".into();
        ws.as_ref().unwrap().metadata.write_session(&meta);
        drop(ws);
        for (status, observed_at) in [("waiting_for_input", "2026-08-01T08:00:02.000000Z"), ("working", "2026-08-01T08:00:01.000000Z")] {
            let response = report_attention(State(state.clone()), Path(id.into()), Json(AttentionReportRequest { status: status.into(), message: None, plan_path: Default::default(), observed_at: Some(observed_at.into()), cwd: None })).await.into_response();
            assert_eq!(response.status(), axum::http::StatusCode::OK);
        }
        assert_eq!(state.sessions.lock().unwrap()[id].info.observed_status.as_deref(), Some("waiting_for_input"));
    }

    #[tokio::test]
    async fn claude_working_hook_clears_shared_stale_capacity_latches() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let source_id = "claude-working-source";
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                source_id,
                "Claude source",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.harness = "claude-code".into();
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        for (id, lines, bytes) in [(source_id, 7, 11), ("claude-working-peer", 3, 5)] {
            let mut handle = attention_test_handle(id, dir.path());
            handle.active_work_hook = true;
            handle.info.harness_id = Some("claude-code".into());
            handle.info.harness = Some("claude-code".into());
            handle.info.last_output_at = Some("2026-08-01T08:00:00.000000Z".into());
            handle.at_usage_limit_latched = true;
            handle.output_lines_seen = lines;
            handle.scan_bytes_seen = bytes;
            state.sessions.lock().unwrap().insert(id.into(), handle);
        }
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut("claude-working-peer")
            .unwrap()
            .info
            .last_output_at = Some("2026-08-01T08:00:02.000000Z".into());

        let response = report_attention(
            State(state.clone()),
            Path(source_id.into()),
            Json(AttentionReportRequest {
                status: "working".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: Some("2026-08-01T08:00:01.000000Z".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let sessions = state.sessions.lock().unwrap();
        assert!(!sessions[source_id].at_usage_limit_latched);
        assert_eq!(sessions[source_id].resume_scan_origin, Some((7, 11)));
        assert!(!sessions["claude-working-peer"].at_usage_limit_latched);
        assert_eq!(
            sessions["claude-working-peer"].resume_scan_origin,
            Some((3, 5))
        );
        drop(sessions);

        {
            let mut sessions = state.sessions.lock().unwrap();
            sessions.get_mut(source_id).unwrap().at_usage_limit_latched = true;
            let peer = sessions.get_mut("claude-working-peer").unwrap();
            peer.at_usage_limit_latched = false;
            peer.runtime.usage_limit_latched_at = Some(
                crate::workspace_runtime::parse_hook_observed_at("2026-08-01T08:00:05.000000Z")
                    .unwrap(),
            );
        }
        let response = report_attention(
            State(state.clone()),
            Path(source_id.into()),
            Json(AttentionReportRequest {
                status: "working".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: Some("2026-08-01T08:00:04.000000Z".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(!state.sessions.lock().unwrap()[source_id].at_usage_limit_latched);
    }

    #[tokio::test]
    async fn claude_working_hook_does_not_clear_a_newer_capacity_signal() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "claude-working-after-cap";
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                id,
                "Claude source",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.harness = "claude-code".into();
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        let mut handle = attention_test_handle(id, dir.path());
        handle.active_work_hook = true;
        handle.info.harness_id = Some("claude-code".into());
        handle.info.harness = Some("claude-code".into());
        handle.at_usage_limit_latched = true;
        handle.runtime.usage_limit_latched_at = Some(
            crate::workspace_runtime::parse_hook_observed_at("2026-08-01T08:00:02.000000Z")
                .unwrap(),
        );
        state.sessions.lock().unwrap().insert(id.into(), handle);

        let response = report_attention(
            State(state.clone()),
            Path(id.into()),
            Json(AttentionReportRequest {
                status: "working".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: Some("2026-08-01T08:00:01.000000Z".into()),
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(state.sessions.lock().unwrap()[id].at_usage_limit_latched);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn report_attention_metadata_io_does_not_block_tokio_worker() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "attention-nonblocking",
                "Known",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let (locked_tx, locked_rx) = std::sync::mpsc::sync_channel(1);
        let locked_state = state.clone();
        let locker = std::thread::spawn(move || {
            let _guard = locked_state.workspace.lock().unwrap();
            locked_tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(250));
        });
        locked_rx.recv().unwrap();

        let request_state = state.clone();
        let response = tokio::spawn(async move {
            report_attention(
                State(request_state),
                Path("attention-nonblocking".into()),
                Json(AttentionReportRequest {
                    status: "blocked".into(),
                    message: Some("Waiting".into()),
                    plan_path: Default::default(),
                    observed_at: None,
                    cwd: None,
                }),
            )
            .await
            .into_response()
        });
        let started = std::time::Instant::now();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(
            started.elapsed() < std::time::Duration::from_millis(100),
            "Tokio worker was blocked for {:?}",
            started.elapsed()
        );

        assert_eq!(response.await.unwrap().status(), axum::http::StatusCode::OK);
        locker.join().unwrap();
    }

    // The "concurrent calls keep live/persisted in agreement" guarantee for
    // both report_attention and apply_debug_attention now lives in a single
    // test on the shared apply_attention_signal module they both call
    // through: runtime::observed_status::tests::
    // apply_attention_signal_keeps_stores_in_agreement_under_concurrent_calls
    // (see ADR 0027).

    #[tokio::test]
    async fn apply_debug_attention_rejects_invalid_attention() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = apply_debug_attention(
            State(state),
            Path("missing".into()),
            Json(DebugAttentionRequest {
                attention: "not_a_real_value".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn apply_debug_attention_returns_not_found_for_unknown_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = apply_debug_attention(
            State(state),
            Path("missing".into()),
            Json(DebugAttentionRequest {
                attention: "working".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn apply_debug_attention_rejects_when_lifecycle_not_alive() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            // test_session_metadata defaults to a dead/ended session.
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "debug-dead",
                    "Dead",
                    dir.path().display().to_string(),
                    "ended",
                    "now",
                    "now",
                ));
        }

        let response = apply_debug_attention(
            State(state),
            Path("debug-dead".into()),
            Json(DebugAttentionRequest {
                attention: "working".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn apply_debug_attention_writes_debug_source_and_confidence() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "debug-alive",
                "Alive",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let response = apply_debug_attention(
            State(state.clone()),
            Path("debug-alive".into()),
            Json(DebugAttentionRequest {
                attention: "blocked".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("debug-alive")
            .unwrap();
        assert_eq!(updated.observed_status.as_deref(), Some("blocked"));
        assert_eq!(updated.attention.as_deref(), Some("blocked"));
        assert_eq!(updated.metadata_source, "debug");
        assert_eq!(updated.metadata_confidence, 0.0);
    }

    #[tokio::test]
    async fn apply_debug_attention_cannot_clobber_live_agent_signal() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "debug-vs-agent",
                "Alive",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            meta.metadata_source = "agent".into();
            meta.observed_status = Some("working".into());
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let response = apply_debug_attention(
            State(state.clone()),
            Path("debug-vs-agent".into()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        // Ignored (not rejected) mirrors report_attention's own handling of an
        // unwritable target: the request is well-formed, it just didn't land.
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("debug-vs-agent")
            .unwrap();
        assert_eq!(
            updated.observed_status.as_deref(),
            Some("working"),
            "a live agent signal must survive a debug injection"
        );
        assert_eq!(updated.metadata_source, "agent");
    }

    #[tokio::test]
    async fn apply_debug_attention_maps_needs_you_to_waiting_for_input_observed_status() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "debug-needs-you",
                "Alive",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let response = apply_debug_attention(
            State(state.clone()),
            Path("debug-needs-you".into()),
            Json(DebugAttentionRequest {
                attention: "needs_you".into(),
                message: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("debug-needs-you")
            .unwrap();
        assert_eq!(
            updated.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(updated.attention.as_deref(), Some("needs_you"));
    }

    #[tokio::test]
    async fn apply_debug_attention_injected_value_is_reclaimed_by_next_real_signal() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "debug-reclaim",
                "Alive",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        let _ = apply_debug_attention(
            State(state.clone()),
            Path("debug-reclaim".into()),
            Json(DebugAttentionRequest {
                attention: "failed".into(),
                message: None,
            }),
        )
        .await
        .into_response();
        {
            let ws = state.workspace.lock().unwrap();
            let injected = ws
                .as_ref()
                .unwrap()
                .metadata
                .read_session("debug-reclaim")
                .unwrap();
            assert_eq!(injected.metadata_source, "debug");
        }

        // Any real attention source is unconditionally accepted over "debug",
        // since debug is the lowest documented priority tier. "blocked" is
        // accepted regardless of active_work_hook capability, unlike "working".
        let _ = report_attention(
            State(state.clone()),
            Path("debug-reclaim".into()),
            Json(AttentionReportRequest {
                status: "blocked".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        let ws = state.workspace.lock().unwrap();
        let reclaimed = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("debug-reclaim")
            .unwrap();
        assert_eq!(reclaimed.metadata_source, "agent");
        assert_eq!(reclaimed.observed_status.as_deref(), Some("blocked"));
    }

    #[tokio::test]
    async fn apply_debug_attention_capped_message_lands_in_usage_limit_reset_hint_not_summary() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: Some("resets in 2h".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&created_id)
            .unwrap();
        assert_eq!(
            updated.summary, None,
            "capped hint must not land in the generic summary field"
        );
        drop(ws);

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[&created_id].info.usage_limit_reset_hint.as_deref(),
            Some("resets in 2h"),
        );
        drop(sessions);

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    /// Regression test for a race apply_debug_attention used to have: the
    /// usage_limit_reset_hint write was a separate, later critical section
    /// from the attention-field write, so a concurrent call could leave the
    /// two fields disagreeing (see ADR 0027 / PR #208 review). Both calls
    /// race through spawn_blocking; whichever lands last must leave
    /// usage_limit_reset_hint consistent with its own attention, not a mix
    /// of the two calls' state.
    #[tokio::test]
    async fn apply_debug_attention_keeps_reset_hint_consistent_with_attention_under_concurrent_calls(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = "debug-concurrent-reset-hint";
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                id,
                "Known",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        state
            .sessions
            .lock()
            .unwrap()
            .insert(id.into(), attention_test_handle(id, dir.path()));

        let capped = apply_debug_attention(
            State(state.clone()),
            Path(id.into()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: Some("resets in 2h".into()),
            }),
        );
        let working = apply_debug_attention(
            State(state.clone()),
            Path(id.into()),
            Json(DebugAttentionRequest {
                attention: "working".into(),
                message: None,
            }),
        );
        let (capped_response, working_response) = tokio::join!(capped, working);
        assert_eq!(
            capped_response.into_response().status(),
            axum::http::StatusCode::OK
        );
        assert_eq!(
            working_response.into_response().status(),
            axum::http::StatusCode::OK
        );

        let sessions = state.sessions.lock().unwrap();
        let info = &sessions.get(id).unwrap().info;
        if info.attention.as_deref() == Some("capped") {
            assert!(
                info.usage_limit_reset_hint.is_some(),
                "attention is capped but reset hint is missing"
            );
        } else {
            assert!(
                info.usage_limit_reset_hint.is_none(),
                "attention is {:?} but a stale capped reset hint survived",
                info.attention
            );
        }
    }

    #[tokio::test]
    async fn debug_injected_capped_hint_survives_list_sessions_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: Some("resets in 2h".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // The bug this guards against: list_sessions used to unconditionally
        // recompute usage_limit_reset_hint from live terminal-output
        // scanning, discarding the debug-injected value on the very next
        // poll since a generic-shell session has no real usage-limit text
        // in its terminal output to detect.
        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(created_id.as_str()))
            .expect("created session should be listed");
        assert_eq!(
            session.get("usageLimitResetHint").and_then(|v| v.as_str()),
            Some("resets in 2h"),
            "debug-injected capped hint must survive a list_sessions refresh"
        );

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn debug_injection_off_capped_clears_stale_reset_hint() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: Some("resets in 2h".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // The bug this guards against: apply_debug_attention only ever SET
        // usage_limit_reset_hint on a capped injection; it never cleared it
        // on a later non-capped injection, so the stale hint survived
        // indefinitely -- and would keep feeding the cross-session harness
        // reset-hint propagation in list_sessions even after this session
        // moved off "capped".
        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "working".into(),
                message: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[&created_id].info.usage_limit_reset_hint, None,
            "moving debug attention off capped must clear the stale reset hint",
        );
        drop(sessions);

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(created_id.as_str()))
            .expect("created session should be listed");
        assert_eq!(
            session.get("usageLimitResetHint"),
            None,
            "list_sessions must not surface a stale hint once attention is off capped"
        );

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn apply_debug_attention_capped_without_message_does_not_clear_existing_hint() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        // A real capacity hint is already present, as if genuine harness capacity
        // detection had set it before the debug picker was ever touched.
        {
            let mut sessions = state.sessions.lock().unwrap();
            sessions
                .get_mut(&created_id)
                .unwrap()
                .info
                .usage_limit_reset_hint = Some("resets in 45m".into());
        }

        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest {
                attention: "capped".into(),
                message: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[&created_id].info.usage_limit_reset_hint.as_deref(),
            Some("resets in 45m"),
            "a message-less capped injection must not wipe an existing real hint",
        );
        drop(sessions);

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn created_generic_session_does_not_advertise_active_work_hook() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(!state.sessions.lock().unwrap()[&created_id].active_work_hook);

        let response = report_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(AttentionReportRequest {
                status: "thinking".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        assert_eq!(
            delete_session(State(state), Path(created_id))
                .await
                .into_response()
                .status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn creation_with_descriptive_initial_prompt_queues_topic_inference() {
        // The initial prompt bypasses terminal_runtime's keystroke-based label
        // seeding entirely (it's written straight to the PTY), so session
        // creation must seed the synchronous fallback and queue Peon's
        // InputLabel pass (ADR 0029) rather than leaving the label stuck on
        // its placeholder.
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: Some("fix the login redirect bug".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let created_id = body["id"].as_str().unwrap().to_owned();
        assert_eq!(body["label"], "fix the login redirect bug");

        assert_eq!(
            state.peon.label_hint.read().unwrap().get(&created_id),
            Some(&crate::LabelHint {
                text: "fix the login redirect bug".into(),
                epoch: 0,
            })
        );
        assert!(state.peon.label_pending.read().unwrap().contains(&created_id));

        let meta = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&created_id)
            .unwrap();
        assert_eq!(meta.label, "fix the login redirect bug");
    }

    #[tokio::test]
    async fn creation_keeps_full_initial_hint_when_pr_is_after_display_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let prompt = format!("review {} PR #249", "important changes ".repeat(6));
        let display: String = prompt.chars().take(100).collect();
        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: Some(prompt.clone()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        let created_id = body["id"].as_str().unwrap();

        assert_eq!(body["label"], display);
        assert_eq!(
            state.peon.label_hint.read().unwrap().get(created_id),
            Some(&crate::LabelHint {
                text: prompt.clone(),
                epoch: 0,
            })
        );
    }

    #[tokio::test]
    async fn creation_with_non_descriptive_initial_prompt_does_not_queue_topic_inference() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: Some("y".into()),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        wait_for_session_status(&state, &created_id, "running").await;

        assert!(state.peon.label_hint.read().unwrap().get(&created_id).is_none());
        assert!(!state.peon.label_pending.read().unwrap().contains(&created_id));
    }

    #[tokio::test]
    async fn unsupported_hook_rejects_thinking_without_changing_attention() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "attention-unsupported-thinking",
                    "Known",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ));
        }

        let response = report_attention(
            State(state.clone()),
            Path("attention-unsupported-thinking".into()),
            Json(AttentionReportRequest {
                status: "thinking".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("attention-unsupported-thinking")
            .unwrap();
        assert_eq!(updated.observed_status, None);
    }

    #[tokio::test]
    async fn session_without_active_work_hook_rejects_thinking_after_registry_changes() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let mut meta = test_session_metadata(
                "attention-session-scoped",
                "Known",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            meta.harness = "claude-code".into();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        state.sessions.lock().unwrap().insert(
            "attention-session-scoped".into(),
            attention_test_handle("attention-session-scoped", dir.path()),
        );

        let response = report_attention(
            State(state.clone()),
            Path("attention-session-scoped".into()),
            Json(AttentionReportRequest {
                status: "thinking".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let ws = state.workspace.lock().unwrap();
        let updated = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("attention-session-scoped")
            .unwrap();
        assert_eq!(updated.observed_status, None);
    }

    #[tokio::test]
    async fn report_attention_clears_pending_input_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "attention-clears-buf",
                    "Known",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ));
        }
        // A single-key "accept" hotkey press leaves an unterminated keystroke
        // sitting in the pending input-line buffer from an earlier prompt.
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert("attention-clears-buf".into(), "a".into());

        let response = report_attention(
            State(state.clone()),
            Path("attention-clears-buf".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(state
            .peon
            .input_buf
            .read()
            .unwrap()
            .get("attention-clears-buf")
            .is_none());
    }

    #[tokio::test]
    async fn report_attention_preserves_in_progress_descriptive_input() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "attention-preserves-buf",
                    "Known",
                    dir.path().display().to_string(),
                    "running",
                    "now",
                    "now",
                ));
        }
        // The user already started typing a real, unterminated response before
        // this (possibly delayed) hook POST landed; it must not be discarded.
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert("attention-preserves-buf".into(), "please also".into());

        let response = report_attention(
            State(state.clone()),
            Path("attention-preserves-buf".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            state
                .peon
                .input_buf
                .read()
                .unwrap()
                .get("attention-preserves-buf")
                .cloned(),
            Some("please also".to_string())
        );
    }

    #[tokio::test]
    async fn report_attention_returns_500_when_persist_fails() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            let store = &ws.as_ref().unwrap().metadata;
            store.write_session(&test_session_metadata(
                "attention-persist-fail",
                "Known",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            ));
            // A directory squatting on the atomic-write temp path makes the
            // persist fail while the session stays readable.
            std::fs::create_dir_all(store.sessions_dir().join("attention-persist-fail.json.tmp"))
                .unwrap();
        }

        let response = report_attention(
            State(state),
            Path("attention-persist-fail".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
                plan_path: Default::default(),
                observed_at: None,
                cwd: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "a lost attention signal must not be acknowledged with 200"
        );
    }

    #[tokio::test]
    async fn forget_session_deletes_session_with_unparseable_metadata_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (json_path, corrupt_path) = {
            let ws = state.workspace.lock().unwrap();
            let store = &ws.as_ref().unwrap().metadata;
            std::fs::create_dir_all(store.sessions_dir()).unwrap();
            let json_path = store.sessions_dir().join("corrupt-forget.json");
            std::fs::write(&json_path, "{\"id\": \"corrupt-forget\",").unwrap();
            (
                json_path,
                store.sessions_dir().join("corrupt-forget.json.corrupt"),
            )
        };

        let response = forget_session(State(state), Path("corrupt-forget".into()))
            .await
            .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::OK,
            "a corrupt-but-present session file must be forgettable, not 404"
        );
        assert!(!json_path.exists());
        assert!(!corrupt_path.exists());
    }

    #[tokio::test]
    async fn forget_session_deletes_terminal_output() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "forget-terminal-output".to_string();
        {
            let ws = state.workspace.lock().unwrap();
            let store = &ws.as_ref().unwrap().metadata;
            store.write_session(&test_session_metadata(
                session_id.clone(),
                "Forget Terminal Output",
                dir.path().display().to_string(),
                "ended",
                "2024-01-01T00:00:00Z",
                "2024-01-01T00:00:00Z",
            ));
            store.append_terminal_output_lines(&session_id, &["hello".to_string()]);
            assert_eq!(
                store.read_terminal_output(&session_id, 10),
                vec!["hello".to_string()]
            );
        }
        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(session_id.clone(), 2);

        let response = forget_session(State(state.clone()), Path(session_id.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let ws = state.workspace.lock().unwrap();
        let store = &ws.as_ref().unwrap().metadata;
        assert!(
            store.read_terminal_output(&session_id, 10).is_empty(),
            "forgetting a session must delete its terminal output file, not just its metadata"
        );
        assert!(!state
            .peon
            .label_epochs
            .read()
            .unwrap()
            .contains_key(&session_id));
    }

    #[tokio::test]
    async fn forget_session_rejects_live_session_with_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "live-session".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: test_session_info(
                    session_id.clone(),
                    "Live Session",
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

        let response = forget_session(State(state), Path(session_id))
            .await
            .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn list_sessions_does_not_duplicate_killed_sessions_with_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(Some(WorkspaceState {
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
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "killed-with-metadata".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(
                        session_id.clone(),
                        "Killed",
                        dir.path().display().to_string(),
                        "killed",
                        "2026-06-25T10:00:00Z",
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
            },
        );

        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            ws.metadata.write_session(&metadata::SessionMetadata {
                id: session_id.clone(),
                label: "Killed".into(),
                workspace: dir.path().display().to_string(),
                task: "".into(),
                harness: "".into(),
                model: "".into(),
                cwd: dir.path().display().to_string(),
                status: "killed".into(),
                work_phase: "unknown".into(),
                lifecycle_phase: "ended".into(),
                lifecycle: "dead".into(),
                attention: None,
                plan_path: None,
                connectivity: "offline".into(),
                terminal_outcome: Some("killed".into()),
                pending_terminal_status: None,
                observed_status: None,
                ending_observed_status_snapshot: None,
                final_observed_status_snapshot: Some(metadata::ObservedStatusSnapshotMetadata {
                    value: None,
                    source: "recovery".into(),
                    confidence: None,
                    observed_at: None,
                }),
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
                created_at: "2026-06-25T10:00:00Z".into(),
                last_activity: "2026-06-25T10:00:00Z".into(),
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

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let matching = sessions
            .iter()
            .filter(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .count();

        assert_eq!(matching, 1);
    }

    #[tokio::test]
    async fn delete_session_enters_ending_lifecycle_instead_of_marking_terminal_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "delete-ending".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: test_session_info(
                    session_id.clone(),
                    "Delete Ending",
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
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&metadata::SessionMetadata {
                    id: session_id.clone(),
                    label: "Delete Ending".into(),
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
                    metadata_source: "peon".into(),
                    metadata_confidence: 0.8,
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

        let response = delete_session(State(state.clone()), Path(session_id.clone())).await;
        assert_eq!(
            response.into_response().status(),
            axum::http::StatusCode::OK
        );

        let info = state
            .sessions
            .lock()
            .unwrap()
            .get(&session_id)
            .unwrap()
            .info
            .clone();
        assert_eq!(info.status, "running");
        assert_eq!(info.lifecycle_phase, "ending");

        let ws = state.workspace.lock().unwrap();
        let meta = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session(&session_id)
            .unwrap();
        assert_eq!(meta.status, "running");
        assert_eq!(meta.lifecycle_phase, "ending");
        assert_eq!(meta.pending_terminal_status.as_deref(), Some("killed"));
        assert_eq!(
            meta.ending_observed_status_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.value.as_deref()),
            Some("blocked")
        );
    }

    #[tokio::test]
    async fn delete_session_clears_pending_input_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "delete-clears-input-buf".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);

        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: test_session_info(
                    session_id.clone(),
                    "Delete Clears Input Buf",
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
        // A stale, unterminated keystroke left over from an earlier prompt.
        state
            .peon
            .input_buf
            .write()
            .unwrap()
            .insert(session_id.clone(), "a".into());

        let response = delete_session(State(state.clone()), Path(session_id.clone())).await;
        assert_eq!(
            response.into_response().status(),
            axum::http::StatusCode::OK
        );

        assert!(state
            .peon
            .input_buf
            .read()
            .unwrap()
            .get(&session_id)
            .is_none());
    }

    #[tokio::test]
    async fn list_sessions_uses_live_session_contract_fields_without_metadata() {
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "offline-live-only".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    connectivity: Some("offline".into()),
                    terminal_outcome: Some("ended".into()),
                    last_activity_at: Some("2026-06-28T09:05:00Z".into()),
                    ..test_session_info(
                        session_id.clone(),
                        "Offline Live Only",
                        "/tmp/project",
                        "ended",
                        "2026-06-28T09:00:00Z",
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
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session.get("connectivity").and_then(|value| value.as_str()),
            Some("offline")
        );
        assert_eq!(
            session
                .get("terminalOutcome")
                .and_then(|value| value.as_str()),
            Some("ended")
        );
        assert_eq!(
            session
                .get("lastActivityAt")
                .and_then(|value| value.as_str()),
            Some("2026-06-28T09:05:00Z"),
        );
    }

    #[tokio::test]
    async fn list_sessions_keeps_pending_without_fresh_resume_output() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "resume-pending-empty".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    capacity_check_pending: Some(true),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Pending Empty",
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
                capacity_check_pending: true,
                output_lines_seen: 1,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((0, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("capacityCheckPending")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[tokio::test]
    async fn list_sessions_keys_checking_state_by_harness_like_capped_state() {
        let dir = tempfile::tempdir().unwrap();
        let settings = crate::providers::ProviderSettingsPayload {
            version: 1,
            revision: 1,
            peon_model: None,
            ollama_base_url: crate::providers::default_ollama_base_url(),
            providers: vec![crate::providers::ProviderSettingsEntry {
                id: "opencode".into(),
                enabled: true,
                fallback_order: 0,
                default_state: crate::providers::ProviderCapacityState::Healthy,
                override_state: None,
            }],
        };
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::for_tests(settings, vec![]),
        });

        // Session on the opencode harness whose model provider is ollama:
        // capped state is keyed by harness, so checking must be too, or the
        // pending badge lands on a different provider row than the capped one.
        let session_id = "resume-pending-provider-mismatch".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("opencode".into()),
                    model_provider_id: Some("ollama".into()),
                    capacity_check_pending: Some(true),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Pending Provider Mismatch",
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
                capacity_check_pending: true,
                output_lines_seen: 1,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((0, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        list_sessions(State(state.clone())).await.into_response();

        let response = state.providers.get_providers_response();
        let opencode = response
            .providers
            .iter()
            .find(|provider| provider.id == "opencode")
            .unwrap();
        assert_eq!(opencode.effective_state, "checking_capacity");
    }

    #[tokio::test]
    async fn list_sessions_requires_one_visible_fresh_output_cycle_before_clearing_pending() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        let session_id = "resume-pending-fresh".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(200);
        output_buffer.push("Welcome back".into());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    capacity_check_pending: Some(true),
                    ..test_session_info(
                        session_id.clone(),
                        "Resume Pending Fresh",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: false,
                capacity_check_pending: true,
                output_lines_seen: 1,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((0, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();
        assert_eq!(
            session
                .get("capacityCheckPending")
                .and_then(|value| value.as_bool()),
            Some(true)
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();
        assert_eq!(session.get("capacityCheckPending"), None);
    }

    #[tokio::test]
    async fn list_sessions_does_not_mark_remembered_sessions_capped_from_other_live_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        state
            .providers
            .apply_settings(crate::providers::ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: crate::providers::default_ollama_base_url(),
                providers: vec![crate::providers::ProviderSettingsEntry {
                    id: "codex".into(),
                    enabled: true,
                    fallback_order: 0,
                    default_state: crate::providers::ProviderCapacityState::Unknown,
                    override_state: None,
                }],
            });
        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            let mut remembered = test_session_metadata(
                "remembered-codex",
                "Remembered Codex",
                dir.path().display().to_string(),
                "ended",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            remembered.harness = "codex".into();
            remembered.cwd = dir.path().display().to_string();
            ws.metadata.write_session(&remembered);

            let mut live_meta = test_session_metadata(
                "live-capped-codex",
                "Live Capped Codex",
                dir.path().display().to_string(),
                "running",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            live_meta.harness = "codex".into();
            live_meta.cwd = dir.path().display().to_string();
            live_meta.status = "running".into();
            live_meta.lifecycle_phase = "active".into();
            live_meta.connectivity = "online".into();
            live_meta.terminal_outcome = None;
            live_meta.final_observed_status_snapshot = None;
            ws.metadata.write_session(&live_meta);
        }

        let live_id = "live-capped-codex".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(200);
        output_buffer.push("You've hit your usage limit".into());
        state.sessions.lock().unwrap().insert(
            live_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    harness: Some("codex".into()),
                    ..test_session_info(
                        live_id.clone(),
                        "Live Capped Codex",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
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
                output_lines_seen: 1,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let live = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(live_id.as_str()))
            .unwrap();
        let remembered = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some("remembered-codex")
            })
            .unwrap();

        assert_eq!(
            live.get("atUsageLimit").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            remembered
                .get("memoryState")
                .and_then(|value| value.as_str()),
            Some("remembered")
        );
        assert_eq!(remembered.get("atUsageLimit"), None);

        let providers = state.providers.get_providers_response();
        let codex = providers
            .providers
            .iter()
            .find(|provider| provider.id == "codex")
            .unwrap();
        assert_eq!(codex.effective_state, "capped");
    }

    #[tokio::test]
    async fn list_sessions_clears_live_capped_after_fresh_post_input_output_without_new_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let session_id = "codex-cap-clear".to_string();
        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            let mut meta = test_session_metadata(
                session_id.clone(),
                "Codex Cap Clear",
                dir.path().display().to_string(),
                "running",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            meta.harness = "codex".into();
            meta.cwd = dir.path().display().to_string();
            meta.status = "running".into();
            meta.lifecycle_phase = "active".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            meta.final_observed_status_snapshot = None;
            ws.metadata.write_session(&meta);
        }
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(200);
        output_buffer.push("You've hit your usage limit".into());
        output_buffer.push("Back in the thread and working again".into());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    harness: Some("codex".into()),
                    ..test_session_info(
                        session_id.clone(),
                        "Codex Cap Clear",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: true,
                capacity_check_pending: false,
                output_lines_seen: 2,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((1, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("atUsageLimit")
                .and_then(|value| value.as_bool()),
            Some(false)
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("atUsageLimit")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
    }

    #[tokio::test]
    async fn list_sessions_keeps_live_capped_when_fresh_post_input_output_still_contains_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let session_id = "codex-cap-still-capped".to_string();
        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            let mut meta = test_session_metadata(
                session_id.clone(),
                "Codex Cap Still Capped",
                dir.path().display().to_string(),
                "running",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            meta.harness = "codex".into();
            meta.cwd = dir.path().display().to_string();
            meta.status = "running".into();
            meta.lifecycle_phase = "active".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            meta.final_observed_status_snapshot = None;
            ws.metadata.write_session(&meta);
        }
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(200);
        output_buffer.push("You've hit your usage limit".into());
        output_buffer.push("You've hit your usage limit".into());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    harness: Some("codex".into()),
                    ..test_session_info(
                        session_id.clone(),
                        "Codex Cap Still Capped",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: true,
                capacity_check_pending: false,
                output_lines_seen: 2,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((1, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state)).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("atUsageLimit")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[tokio::test]
    async fn list_sessions_clears_live_capped_even_when_ring_buffer_length_stays_flat() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let session_id = "codex-cap-clear-saturated".to_string();
        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            let mut meta = test_session_metadata(
                session_id.clone(),
                "Codex Cap Clear Saturated",
                dir.path().display().to_string(),
                "running",
                "2026-07-05T09:00:00Z",
                "2026-07-05T09:05:00Z",
            );
            meta.harness = "codex".into();
            meta.cwd = dir.path().display().to_string();
            meta.status = "running".into();
            meta.lifecycle_phase = "active".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            meta.final_observed_status_snapshot = None;
            ws.metadata.write_session(&meta);
        }
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut output_buffer = peon::RingBuffer::new(1);
        output_buffer.push("Back in the thread and working again".into());
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    harness_id: Some("codex".into()),
                    harness: Some("codex".into()),
                    ..test_session_info(
                        session_id.clone(),
                        "Codex Cap Clear Saturated",
                        dir.path().display().to_string(),
                        "running",
                        "now",
                    )
                },
                kill_tx,
                output_buffer,
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
                    crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
                ),
                terminal_attached: false,
                resume_in_progress: false,
                at_usage_limit_latched: true,
                capacity_check_pending: false,
                output_lines_seen: 2,
                scan_bytes_seen: 0,
                resume_scan_origin: Some((1, 0)),
                pending_capacity_visible_once: false,
                active_work_hook: false,
            },
        );

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str())
            })
            .unwrap();

        assert_eq!(
            session
                .get("atUsageLimit")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
    }

    #[tokio::test]
    async fn list_sessions_derives_resume_options_for_remembered_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            session_pids: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(Some(WorkspaceState {
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
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_epochs: std::sync::RwLock::new(std::collections::HashMap::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                reported_cwd: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: crate::test_support::test_harness_components().0,
            harness_store: crate::test_support::test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            bound_port: std::sync::atomic::AtomicU16::new(0),
            providers: crate::providers::ProviderManager::new(),
        });

        {
            let ws = state.workspace.lock().unwrap();
            let ws = ws.as_ref().unwrap();
            ws.metadata.write_session(&metadata::SessionMetadata {
                id: "remembered-derived".into(),
                label: "Remembered Derived".into(),
                workspace: dir.path().display().to_string(),
                task: "".into(),
                harness: "opencode".into(),
                model: "".into(),
                cwd: dir.path().display().to_string(),
                status: "ended".into(),
                work_phase: "unknown".into(),
                lifecycle_phase: "ended".into(),
                lifecycle: "dead".into(),
                attention: None,
                plan_path: None,
                connectivity: "offline".into(),
                terminal_outcome: Some("ended".into()),
                pending_terminal_status: None,
                observed_status: None,
                ending_observed_status_snapshot: None,
                final_observed_status_snapshot: Some(metadata::ObservedStatusSnapshotMetadata {
                    value: None,
                    source: "recovery".into(),
                    confidence: None,
                    observed_at: None,
                }),
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
                created_at: "2026-06-28T09:00:00Z".into(),
                last_activity: "2026-06-28T09:05:00Z".into(),
        last_output_at: None,
                metadata_source: "process".into(),
                metadata_confidence: 1.0,
                repo_root: Some(dir.path().display().to_string()),
                branch: Some("main".into()),
                dirty: Some(false),
                changed_files: Some(0),
                is_worktree: Some(false),
                resume: Some(harness::ResumeMemory {
                    state: harness::ResumeState::Available,
                    preferred_strategy: harness::ResumeStrategy::Exact,
                    harness_session_id: None,
                    latest_fallback: true,
                    last_seen_at: Some("2026-06-28T09:05:00Z".into()),
                }),
                resume_options: vec![metadata::ResumeOption {
                    strategy: harness::ResumeStrategy::Exact,
                    label: "Resume exact session".into(),
                    available: true,
                    preferred: true,
                    reason: None,
                }],
                harness_session_id_source: None,
                harness_session_id_confidence: None,
                harness_session_id_captured_at: None,
                resumed_from: None,
                last_user_input: None,
            });
        }

        let response = list_sessions(State(state)).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| {
                session.get("id").and_then(|id| id.as_str()) == Some("remembered-derived")
            })
            .unwrap();
        let options = session
            .get("resumeOptions")
            .and_then(|value| value.as_array())
            .unwrap();

        assert_eq!(options.len(), 3);
        assert_eq!(options[0]["strategy"], "exact");
        assert_eq!(options[0]["available"], false);
        assert_eq!(options[1]["strategy"], "latest_cwd");
        assert_eq!(options[1]["available"], true);
        assert_eq!(options[1]["preferred"], true);
        assert_eq!(options[2]["strategy"], "latest_repo");
        assert_eq!(options[2]["available"], true);
    }

    #[test]
    fn workspace_request_deserializes_path() {
        let json = r#"{"path": "/home/user/project"}"#;
        let req: WorkspaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.path, "/home/user/project");
    }

    #[test]
    fn workspace_response_serializes_all_fields() {
        let resp = WorkspaceResponse {
            path: "/tmp".into(),
            repo_root: Some("/tmp".into()),
            branch: Some("main".into()),
            dirty: Some(false),
            last_active_session_id: Some("session-1".into()),
            active_harness_ids: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"path\":\"/tmp\""));
        assert!(json.contains("\"repo_root\":\"/tmp\""));
        assert!(json.contains("\"branch\":\"main\""));
        assert!(json.contains("\"dirty\":false"));
        assert!(json.contains("\"lastActiveSessionId\":\"session-1\""));
    }

    #[test]
    fn workspace_response_without_git() {
        let resp = WorkspaceResponse {
            path: "/tmp".into(),
            repo_root: None,
            branch: None,
            dirty: None,
            last_active_session_id: None,
            active_harness_ids: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"path\":\"/tmp\""));
        assert!(json.contains("\"repo_root\":null"));
        assert!(json.contains("\"branch\":null"));
        assert!(json.contains("\"dirty\":null"));
        assert!(json.contains("\"lastActiveSessionId\":null"));
    }

    fn test_resolved_registry() -> crate::harness::registry::ResolvedHarnessRegistry {
        let builtins = crate::harness::definition::BuiltinDocument::parse(
            crate::harness::definition::EMBEDDED_BUILTINS,
        )
        .unwrap();
        crate::harness::registry::resolve_document(
            &builtins,
            &crate::harness::definition::HarnessUserDocument::default(),
        )
        .unwrap()
    }

    #[test]
    fn resolve_session_launch_codex_wires_to_codex_definition() {
        let registry = test_resolved_registry();
        let launch = resolve_session_launch(
            &registry,
            &CreateSessionRequest {
                harness_id: Some("codex".into()),
                model: None,
                initial_prompt: None,
            },
            "/repo".into(),
        );

        assert_eq!(launch.session_harness_id.as_deref(), Some("codex"));
        assert_eq!(launch.command.program, "codex");
    }

    #[tokio::test]
    async fn create_session_rejects_a_retired_harness() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = create_session(
            State(state),
            Json(CreateSessionRequest {
                harness_id: Some("gemini".into()),
                model: None,
                initial_prompt: Some("do not send this to a shell".into()),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    /// Regression test for issue #302: `create_session` used to await the PTY
    /// spawn before responding, so the client's response body — and thus the
    /// first render of the new session in `sessions` state — never actually
    /// observed `status: "creating"`. The response must reflect the
    /// pre-spawn record; the transition to `"running"` happens afterward,
    /// asynchronously.
    #[tokio::test]
    async fn create_session_returns_creating_status_before_spawn_completes() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("generic-shell".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body["status"], "creating",
            "the create response must reflect the pre-spawn record, not wait for spawn to finish"
        );
        let created_id = body["id"].as_str().unwrap().to_owned();

        wait_for_session_status(&state, &created_id, "running").await;
    }

    /// Regression test for issue #302: a spawn failure must no longer surface
    /// as a synchronous 500 from `POST /sessions` (that response has already
    /// gone out reporting `"creating"`), but as an async transition to
    /// `status: "error"` observed via the existing poll/metadata mechanism —
    /// the same signal resume and daemon-restart reconciliation already
    /// produce for this failure mode.
    #[tokio::test]
    async fn create_session_spawn_failure_surfaces_as_async_error_status() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                let override_patch = document.overrides.entry("opencode".into()).or_default();
                override_patch.launch = Some(harness::definition::LaunchPatch {
                    command: Some("orkworks-create-command-that-does-not-exist".into()),
                    ..Default::default()
                });
                Ok(())
            })
            .unwrap();

        let response = create_session(
            State(state.clone()),
            Json(CreateSessionRequest {
                harness_id: Some("opencode".into()),
                model: None,
                initial_prompt: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::OK,
            "spawn failures are no longer synchronous 500s"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["status"], "creating");
        let created_id = body["id"].as_str().unwrap().to_owned();

        wait_for_session_status(&state, &created_id, "error").await;
    }

    #[test]
    fn resolve_session_launch_opencode_no_model_omits_model_args() {
        let registry = test_resolved_registry();
        let launch = resolve_session_launch(
            &registry,
            &CreateSessionRequest {
                harness_id: Some("opencode".into()),
                model: None,
                initial_prompt: None,
            },
            "/repo".into(),
        );
        assert!(
            !launch.command.args.contains(&"--model".into()),
            "bare --model should be dropped"
        );
        assert!(
            !launch.command.args.iter().any(|a| a.starts_with("ollama/")),
            "bare prefix should not appear"
        );
    }

    #[test]
    fn resolve_session_launch_opencode_with_model_uses_prefix() {
        let registry = test_resolved_registry();
        let launch = resolve_session_launch(
            &registry,
            &CreateSessionRequest {
                harness_id: Some("opencode".into()),
                model: Some("qwen2.5-coder:latest".into()),
                initial_prompt: None,
            },
            "/repo".into(),
        );
        assert!(launch
            .command
            .args
            .contains(&"ollama/qwen2.5-coder:latest".into()));
    }

    #[test]
    fn resolve_session_launch_does_not_infer_model_provider_from_harness() {
        let registry = test_resolved_registry();
        let launch = resolve_session_launch(
            &registry,
            &CreateSessionRequest {
                harness_id: Some("codex".into()),
                model: Some("gpt-5".into()),
                initial_prompt: None,
            },
            "/repo".into(),
        );

        assert_eq!(launch.session_harness_id.as_deref(), Some("codex"));
        assert_eq!(launch.model.as_deref(), Some("gpt-5"));
        assert_eq!(launch.provider_id, None);
        assert_eq!(launch.provider_label, None);
    }

    #[test]
    fn attention_report_plan_path_distinguishes_set_clear_and_omission() {
        let set: AttentionReportRequest =
            serde_json::from_str(r#"{"status":"waiting_for_input","planPath":"docs/plan.md"}"#)
                .unwrap();
        assert_eq!(
            set.plan_path,
            metadata::PlanPathUpdate::Set("docs/plan.md".into())
        );

        let clear: AttentionReportRequest =
            serde_json::from_str(r#"{"status":"waiting_for_input","planPath":null}"#).unwrap();
        assert_eq!(clear.plan_path, metadata::PlanPathUpdate::Clear);

        let unchanged: AttentionReportRequest =
            serde_json::from_str(r#"{"status":"waiting_for_input"}"#).unwrap();
        assert_eq!(unchanged.plan_path, metadata::PlanPathUpdate::Unchanged);
    }

    #[tokio::test]
    async fn plan_content_requires_a_valid_plan_and_sidecar_token() {
        let _token_lock = PLAN_TOKEN_TEST_LOCK.lock().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        std::fs::create_dir(workspace.path().join("docs")).unwrap();
        let plan = workspace.path().join("docs/plan.md");
        std::fs::write(&plan, "# plan").unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        let mut metadata = test_session_metadata(
            "plan-session",
            "Plan session",
            workspace.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        metadata.plan_path = Some("docs/plan.md".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&metadata);

        std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "test-token");
        let mut headers = HeaderMap::new();
        headers.insert("x-orkworks-open-plan-token", "test-token".parse().unwrap());
        let response = get_session_plan_content(State(state), Path("plan-session".into()), headers)
            .await
            .into_response();
        std::env::remove_var("ORKWORKS_OPEN_PLAN_TOKEN");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn report_session_plan_path_normalizes_the_path_without_changing_attention() {
        let workspace = tempfile::tempdir().unwrap();
        let plan_dir = workspace.path().join("docs/superpowers/plans");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("plan.md");
        std::fs::write(&plan, "# plan").unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        let mut metadata = test_session_metadata(
            "plan-session", "Plan session", workspace.path().display().to_string(), "running", "now", "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        metadata.attention = Some("working".into());
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&metadata);

        let response = report_session_plan_path(
            State(state.clone()),
            Path("plan-session".into()),
            Json(PlanPathReportRequest { plan_path: plan.display().to_string() }),
        ).await.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
        let metadata = state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_session("plan-session").unwrap();
        assert_eq!(metadata.plan_path.as_deref(), Some("docs/superpowers/plans/plan.md"));
        assert_eq!(metadata.attention.as_deref(), Some("working"));
    }

    #[tokio::test]
    async fn report_session_plan_path_returns_internal_error_and_skips_event_when_session_write_fails() {
        let workspace = tempfile::tempdir().unwrap();
        let plan_dir = workspace.path().join("docs/superpowers/plans");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("plan.md");
        std::fs::write(&plan, "# plan").unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        let mut metadata = test_session_metadata(
            "plan-session", "Plan session", workspace.path().display().to_string(), "running", "now", "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        metadata.attention = Some("working".into());
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&metadata);

        // Squat a directory on the per-session temp path so the atomic write
        // fails (write_session returns Err) while the session JSON remains
        // readable — mirrors the established failure-mode test pattern.
        let sessions_path =
            state.workspace.lock().unwrap().as_ref().unwrap().metadata.sessions_dir();
        std::fs::create_dir_all(sessions_path.join("plan-session.json.tmp")).unwrap();

        let response = report_session_plan_path(
            State(state.clone()),
            Path("plan-session".into()),
            Json(PlanPathReportRequest { plan_path: plan.display().to_string() }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        // The session JSON is the source of truth; the handler must not
        // append a `session.plan_path_hooked` event when the write never
        // landed.
        let events = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_events("plan-session");
        assert!(
            events
                .iter()
                .all(|event| event.event_type != "session.plan_path_hooked"),
            "no plan-path-hooked event should be appended on a write failure, got: {events:?}"
        );
    }

    #[tokio::test]
    async fn plan_endpoints_reject_a_missing_sidecar_token() {
        let _token_lock = PLAN_TOKEN_TEST_LOCK.lock().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "test-token");

        let content = get_session_plan_content(
            State(state.clone()),
            Path("missing".into()),
            HeaderMap::new(),
        )
        .await
        .into_response();
        let review = request_session_plan_review(
            State(state),
            Path("missing".into()),
            HeaderMap::new(),
        )
        .await
        .into_response();
        std::env::remove_var("ORKWORKS_OPEN_PLAN_TOKEN");

        assert_eq!(content.status(), axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(review.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn plan_review_submits_the_fixed_prompt_before_recording_the_event() {
        let _token_lock = PLAN_TOKEN_TEST_LOCK.lock().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        std::fs::create_dir(workspace.path().join("specs")).unwrap();
        std::fs::write(workspace.path().join("specs/plan.md"), "# plan").unwrap();
        let state = test_app_state_with_workspace(workspace.path());
        let mut metadata = test_session_metadata(
            "plan-session", "Plan session", workspace.path().display().to_string(), "running", "now", "now",
        );
        metadata.lifecycle_phase = "active".into();
        metadata.lifecycle = "alive".into();
        metadata.plan_path = Some("specs/plan.md".into());
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&metadata);

        let mut handle = attention_test_handle("plan-session", workspace.path());
        let (runtime, mut control_rx) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
        handle.runtime = runtime;
        state.sessions.lock().unwrap().insert("plan-session".into(), handle);

        std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "test-token");
        let mut headers = HeaderMap::new();
        headers.insert("x-orkworks-open-plan-token", "test-token".parse().unwrap());
        let mut request = tokio::spawn(request_session_plan_review(
            State(state.clone()),
            Path("plan-session".into()),
            headers,
        ));
        let crate::runtime::session_runtime::RuntimeCommand::Input { data, accepted } =
            (tokio::select! {
                command = control_rx.recv() => command.unwrap(),
                response = &mut request => panic!("review request returned {} before reaching the PTY", response.unwrap().into_response().status()),
            })
        else {
            panic!("expected terminal input")
        };
        assert_eq!(data, "Please review the plan or specification at specs/plan.md. If your tooling can spawn a separate review subagent, delegate the review to it instead of reviewing your own work; otherwise review it yourself. Check for missing requirements, risky assumptions, and unclear steps, then report the findings.\r");
        accepted.unwrap().send(Ok(())).unwrap();
        let response = request.await.unwrap().into_response();
        std::env::remove_var("ORKWORKS_OPEN_PLAN_TOKEN");

        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
        let events = state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_events("plan-session");
        assert!(events.iter().any(|event| event.event_type == "plan_review_requested"));
    }
}
