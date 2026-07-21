use crate::harness_registry::{
    adapter_for_harness, capabilities_for_harness, default_shell_command,
    resolve_adapter_harness_id,
};
use crate::session_types::{MemoryState, SessionInfo};
use crate::session_view::{
    connectivity_for_status, derive_memory_state, detect_conflicts, merge_live_session_info,
    session_recommendation, terminal_outcome_for_status,
};
use crate::workspace_runtime::{iso_now, orkworks_global_dir};
use crate::{git, harness, metadata, migration, peon, watcher, AppState, SessionHandle, WorkspaceState};
use axum::{
    extract::{Path, State},
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
        None => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "no home directory").into_response(),
    };
    for dir in &["sessions", "events", "capacity", "skills"] {
        if let Err(e) = std::fs::create_dir_all(global_dir.join(dir)) {
            tracing::warn!(path = %global_dir.display(), dir = dir, error = %e, "failed to create metadata dir");
        }
    }

    let store = metadata::MetadataStore::new(&global_dir);

    migration::migrate_if_needed(&ws_path, &global_dir);

    let memory = store.read_workspace_memory();
    let last_active_session_id = memory.as_ref().and_then(|m| m.last_active_session_id.clone());
    let active_harness_ids = memory.map(|m| m.active_harness_ids).unwrap_or_default();
    let watch_dir = global_dir.join("sessions");
    let watcher = watcher::MetadataWatcher::start(&watch_dir);

    let mut ws = state.workspace.lock().unwrap();
    *ws = Some(WorkspaceState {
        path: ws_path.clone(),
        metadata: store,
        watcher,
    });

    // Reconcile sessions left in "running"/"creating" from a previous daemon run.
    // On restart state.sessions is empty, so anything still "running" in metadata is orphaned.
    if let Some(ref ws) = *ws {
        let now = iso_now();
        for meta in ws.metadata.read_all_sessions() {
            if meta.status == "running" || meta.status == "creating" {
                ws.metadata
                    .write_session(&metadata::reconcile_orphaned_session(meta, &now));
            }
        }
    }

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
        ws.metadata.write_workspace_memory(&metadata::WorkspaceMemory {
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
        ws.metadata.write_workspace_memory(&metadata::WorkspaceMemory {
            last_active_session_id: existing.as_ref().and_then(|m| m.last_active_session_id.clone()),
            last_active_at: Some(now),
            active_harness_ids: req.active_harness_ids,
        });
        return axum::http::StatusCode::OK;
    }
    axum::http::StatusCode::CONFLICT
}

pub(crate) async fn resume_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let now = iso_now();
    let harnesses = state.harnesses.read().await.clone();
    let (meta, command, strategy, capabilities, active_work_hook) = {
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
        let session_harness_id = (!meta.harness.is_empty()).then(|| meta.harness.as_str());
        let active_work_hook = session_harness_id
            .and_then(|id| harnesses.iter().find(|h| h.id == id))
            .is_some_and(|h| h.attention.active_work_hook);
        let adapter_harness_id = resolve_adapter_harness_id(&harnesses, session_harness_id);
        let capabilities = capabilities_for_harness(&state.adapters, adapter_harness_id.as_deref());
        let strategy = harness::select_resume_strategy(resume, &capabilities);
        if strategy == harness::ResumeStrategy::None {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        }
        let adapter = adapter_for_harness(&state.adapters, adapter_harness_id.as_deref());
        let request = harness::ResumeRequest {
            strategy: strategy.clone(),
            cwd: meta.cwd.clone(),
            repo_root: meta.repo_root.clone(),
            harness_session_id: resume.harness_session_id.clone(),
            model: (!meta.model.is_empty()).then(|| meta.model.clone()),
        };
        let Some(command) = adapter.build_resume_command(&request) else {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        };
        (meta, command, strategy, capabilities, active_work_hook)
    };

    {
        let sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get(&id) {
            let still_live = !matches!(handle.info.lifecycle_phase.as_str(), "ended");
            if handle.terminal_attached || still_live {
                return axum::http::StatusCode::CONFLICT.into_response();
            }
        }
    }

    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);
    let capacity_check_pending = capabilities.detect_capacity;
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
            capabilities.resume_exact,
            capabilities.resume_latest_in_cwd,
            capabilities.resume_latest_in_repo,
        ),
        resumed_from: meta.resumed_from.clone(),
        provider: meta.provider_label.clone(),
        provider_model: meta.provider_model.clone(),
        provider_state: meta.provider_state.clone(),
    };

    let (runtime, control_rx) = crate::runtime::session_runtime::SessionRuntime::live(
        crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
        crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
    );
    let output_tx = runtime.output_tx.clone();

    {
        let mut sessions = state.sessions.lock().unwrap();
        sessions.remove(&id);
        sessions.insert(
            id.clone(),
            SessionHandle {
                info: info.clone(),
                active_work_hook,
                kill_tx: kill_tx.clone(),
                output_buffer: peon::RingBuffer::new(state.peon.config.max_lines),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime,
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: capacity_check_pending.then_some((0, 0)),
                pending_capacity_visible_once: false,
            },
        );
    }

    {
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
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

    match crate::runtime::session_runtime::start_session_runtime(
        state.clone(),
        id.clone(),
        command.clone(),
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
    .await
    {
        Ok(()) => {}
        Err(error) => {
            tracing::error!(session_id = %id, %error, "failed to start resumed session runtime");
            if crate::runtime::terminal_runtime::set_session_status(&state, &id, "error") {
                crate::runtime::terminal_runtime::schedule_session_ending_finalization(
                    state.clone(),
                    id.clone(),
                    "error".into(),
                );
            }
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
            ws.metadata.append_event(&id, &metadata::Event {
                event_type: "session.resumed".into(),
                timestamp: now,
                status: "running".into(),
                observed_status: None,
                confidence: None,
            });
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
        metadata::HarnessSessionMergeResult::NotFound => axum::http::StatusCode::NOT_FOUND.into_response(),
        metadata::HarnessSessionMergeResult::Invalid => axum::http::StatusCode::BAD_REQUEST.into_response(),
    }
}

pub(crate) async fn report_attention(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AttentionReportRequest>,
) -> impl IntoResponse {
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
    let Some(status) = crate::harness_registry::normalize_hook_attention_status(
        &req.status,
        supports_active_work,
    ) else {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    };

    // A hook report is a harness turn boundary: any keystroke still sitting in
    // the pending input-line buffer (e.g. a single-key "accept" prompt with no
    // trailing Enter) belongs to a prompt that's now resolved. Drop it so it
    // doesn't glue onto the next unrelated line typed into the terminal. Only
    // drop it if it's not already descriptive, since the hook POST is
    // asynchronous and can land after the user has started typing a real,
    // unterminated response to a fresh prompt — that in-progress line must
    // survive to be joined with the rest once Enter is pressed.
    {
        let mut bufs = state.peon.input_buf.write().unwrap();
        if bufs.get(&id).is_some_and(|buf| !peon::is_descriptive_input(buf)) {
            bufs.remove(&id);
        }
    }

    let now = iso_now();
    let result = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ref ws) = *ws_guard else {
            return axum::http::StatusCode::CONFLICT.into_response();
        };
        ws.metadata
            .merge_agent_attention_signal(&id, &status, req.message.as_deref(), &now, "agent", 1.0)
    };

    match result {
        metadata::AttentionMergeResult::Accepted => {
            if let Some(handle) = state.sessions.lock().unwrap().get_mut(&id) {
                handle.info.observed_status = Some(status.clone());
                if handle.info.lifecycle == "alive" {
                    handle.info.attention = metadata::canonical_attention(Some(status.as_str()));
                }
                handle.info.metadata_source = Some("agent".into());
                handle.info.metadata_confidence = Some(1.0);
            }
            axum::http::StatusCode::OK.into_response()
        }
        metadata::AttentionMergeResult::Ignored => axum::http::StatusCode::OK.into_response(),
        metadata::AttentionMergeResult::NotFound => axum::http::StatusCode::NOT_FOUND.into_response(),
        // The signal was lost, not delivered — a 200 here would tell the
        // harness hook its notification landed when it didn't.
        metadata::AttentionMergeResult::PersistFailed => {
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
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

    // The debug picker speaks the `attention` vocabulary directly; the merge
    // path stores `observed_status` and derives `attention` from it via
    // canonical_attention. Every value round-trips as a passthrough except
    // needs_you, which canonical_attention only produces from waiting_for_input.
    let observed_status = if req.attention == "needs_you" { "waiting_for_input" } else { &req.attention };
    let is_capped = req.attention == "capped";
    let summary_message = if is_capped { None } else { req.message.as_deref() };

    let now = iso_now();
    let result = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ref ws) = *ws_guard else {
            return axum::http::StatusCode::CONFLICT.into_response();
        };
        match ws.metadata.read_session(&id) {
            None => return axum::http::StatusCode::NOT_FOUND.into_response(),
            Some(meta) if meta.lifecycle != "alive" => {
                return axum::http::StatusCode::BAD_REQUEST.into_response();
            }
            Some(_) => {}
        }
        ws.metadata.merge_agent_attention_signal(
            &id,
            observed_status,
            summary_message,
            &now,
            "debug",
            0.0,
        )
    };

    match result {
        metadata::AttentionMergeResult::Accepted => {
            if let Some(handle) = state.sessions.lock().unwrap().get_mut(&id) {
                handle.info.observed_status = Some(observed_status.to_string());
                if handle.info.lifecycle == "alive" {
                    handle.info.attention = Some(req.attention.clone());
                }
                handle.info.metadata_source = Some("debug".into());
                handle.info.metadata_confidence = Some(0.0);
                if is_capped {
                    if req.message.is_some() {
                        handle.info.usage_limit_reset_hint = req.message.clone();
                    }
                } else {
                    // A debug injection that moves attention off "capped"
                    // must not leave a stale reset hint behind — it would
                    // otherwise survive indefinitely (list_sessions only
                    // recomputes this field from live output for non-debug
                    // sources) and leak into other sessions on the same
                    // harness via the cross-session capacity propagation
                    // below.
                    handle.info.usage_limit_reset_hint = None;
                }
            }
            axum::http::StatusCode::OK.into_response()
        }
        metadata::AttentionMergeResult::Ignored => axum::http::StatusCode::OK.into_response(),
        metadata::AttentionMergeResult::NotFound => axum::http::StatusCode::NOT_FOUND.into_response(),
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
    #[allow(dead_code)]
    pub(crate) adapter_harness_id: Option<String>,
    pub(crate) active_work_hook: bool,
    pub(crate) model: Option<String>,
    pub(crate) command: harness::CommandSpec,
    pub(crate) provider_id: Option<String>,
    pub(crate) provider_label: Option<String>,
}

pub(crate) fn resolve_session_launch(
    harnesses: &[crate::harness_registry::HarnessConfig],
    req: &CreateSessionRequest,
    cwd: String,
) -> ResolvedSessionLaunch {
    if let Some(ref harness_id) = req.harness_id {
        if let Some(config) = harnesses.iter().find(|h| h.id == *harness_id) {
            let model = req.model.clone().or_else(|| {
                (!config.default_model.is_empty()).then(|| config.default_model.clone())
            });
            let args: Vec<String> = match model.as_deref() {
                Some(m) => {
                    let model_value = format!("{}{}", config.model_prefix, m);
                    config.args.iter().map(|arg| arg.replace("{model}", &model_value)).collect()
                }
                None => {
                    let mut out: Vec<String> = Vec::new();
                    for arg in &config.args {
                        if arg.contains("{model}") {
                            out.pop(); // drop the preceding flag (e.g. "--model")
                        } else {
                            out.push(arg.clone());
                        }
                    }
                    out
                }
            };
            return ResolvedSessionLaunch {
                session_harness_id: Some(config.id.clone()),
                adapter_harness_id: Some(config.harness.clone()),
                active_work_hook: config.attention.active_work_hook,
                model,
                command: harness::CommandSpec {
                    program: config.command.clone(),
                    args,
                    cwd,
                },
                provider_id: None,
                provider_label: None,
            };
        }
    }

    ResolvedSessionLaunch {
        session_harness_id: None,
        adapter_harness_id: None,
        active_work_hook: false,
        model: req.model.clone(),
        command: default_shell_command(cwd),
        provider_id: None,
        provider_label: None,
    }
}

pub(crate) async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    let cwd = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .map(|workspace| workspace.path.display().to_string())
        .or_else(|| std::env::current_dir().ok().map(|path| path.display().to_string()))
        .unwrap_or_else(|| "/".into());

    let resolved_launch = {
        let harnesses = state.harnesses.read().await;
        resolve_session_launch(&harnesses, &req, cwd.clone())
    };

    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);

    let git_ctx = git::detect(&PathBuf::from(&cwd));
    let now = iso_now();
    let info = SessionInfo {
        id: id.clone(),
        label: format!("Session {}", &id[..8]),
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
        .map(|prompt| crate::runtime::session_runtime::arm_pending_work_signal(
            prompt,
            tokio::time::Instant::now(),
        ));
    let (runtime, control_rx) = crate::runtime::session_runtime::SessionRuntime::live(
        crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS,
        crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS,
    );
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
                harness: resolved_launch.session_harness_id.clone().unwrap_or_default(),
                model: resolved_launch.model.clone().unwrap_or_default(),
                cwd: info.cwd.clone(),
                status: "creating".into(),
                work_phase: "unknown".into(),
                lifecycle_phase: "creating".into(),
                lifecycle: "creating".into(),
                attention: None,
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

    match crate::runtime::session_runtime::start_session_runtime(
        state.clone(),
        id.clone(),
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
        Ok(()) => {}
        Err(error) => {
            tracing::error!(session_id = %id, %error, "failed to start session runtime");
            if crate::runtime::terminal_runtime::set_session_status(&state, &id, "error") {
                crate::runtime::terminal_runtime::schedule_session_ending_finalization(
                    state.clone(),
                    id.clone(),
                    "error".into(),
                );
            }
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    let info = state
        .sessions
        .lock()
        .unwrap()
        .get(&id)
        .map(|handle| handle.info.clone())
        .expect("newly started session remains registered");

    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        ws.metadata.append_event(&id, &metadata::Event {
            event_type: "session.created".into(),
            timestamp: now,
            status: "running".into(),
            observed_status: None,
            confidence: None,
        });
    }
    drop(ws_guard);

    Json(info).into_response()
}

fn enrich_sessions_with_git_context<F>(infos: &mut [SessionInfo], mut detect_git: F)
where
    F: FnMut(&std::path::Path) -> git::GitContext,
{
    let mut cwd_counts: HashMap<String, usize> = HashMap::new();
    for info in infos.iter() {
        if info.status == "running" || info.status == "creating" {
            *cwd_counts.entry(info.cwd.clone()).or_default() += 1;
        }
    }

    let mut contexts: HashMap<String, git::GitContext> = HashMap::new();
    for info in infos {
        if !contexts.contains_key(&info.cwd) {
            contexts.insert(
                info.cwd.clone(),
                detect_git(std::path::Path::new(&info.cwd)),
            );
        }
        let ctx = &contexts[&info.cwd];
        let count = cwd_counts.get(&info.cwd).copied().unwrap_or(1);
        info.recommendation = session_recommendation(ctx, count);
        info.repo_root = ctx.repo_root.clone();
        info.branch = ctx.branch.clone();
        info.dirty = Some(ctx.dirty);
        info.changed_files = Some(ctx.changed_files);
        info.is_worktree = Some(ctx.is_worktree);
    }
}

pub(crate) async fn list_sessions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let harnesses = state.harnesses.read().await.clone();
    let live_sessions: Vec<(SessionInfo, Vec<String>, String, bool, bool, u64, u64, Option<(u64, u64)>, bool)> = {
        let sessions = state.sessions.lock().unwrap();
        sessions.values().map(|h| {
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
        }).collect()
    };

    let ws_guard = state.workspace.lock().unwrap();
    let metadata_map = ws_guard.as_ref().map(|ws| {
        let mut metadata = HashMap::new();
        for (info, _, _, _, _, _, _, _, _) in &live_sessions {
            if let Some(meta) = ws.metadata.read_session(&info.id) {
                metadata.insert(info.id.clone(), meta);
            }
        }
        metadata
    }).unwrap_or_default();

    let all_metadata_sessions = ws_guard.as_ref().map(|ws| ws.metadata.read_all_sessions()).unwrap_or_default();
    drop(ws_guard);

    let all_memory_ids: HashSet<String> = live_sessions.iter()
        .map(|(info, _, _, _, _, _, _, _, _)| info.id.clone())
        .collect();

    let peon_times = state.peon.last_inference.read().unwrap();
    let mut pending_transitions: Vec<(String, bool, bool)> = Vec::new();
    let mut capped_recheck_resets: HashSet<String> = HashSet::new();
    let mut capped_clear_baselines: HashMap<String, (u64, u64)> = HashMap::new();
    let mut infos: Vec<SessionInfo> = live_sessions.into_iter().map(|(info, snapshot, scan_buf, prev_latch, pending, output_lines_seen, scan_bytes_seen, origin, pending_visible_once)| {
        let id = info.id.clone();
        let meta = metadata_map.get(&id);
        let session_harness_id = meta.and_then(|m| (!m.harness.is_empty()).then(|| m.harness.as_str()));
        let adapter_harness_id = resolve_adapter_harness_id(&harnesses, session_harness_id);
        let caps = capabilities_for_harness(&state.adapters, adapter_harness_id.as_deref());
        let mut merged = merge_live_session_info(info, meta, peon_times.get(&id), &caps);
        let fresh_output_since_origin = origin.map(|(line_count, scan_len)| {
            output_lines_seen > line_count || scan_bytes_seen > scan_len
        }).unwrap_or(false);
        let has_fresh_resume_output = pending && !pending_visible_once && fresh_output_since_origin;
        let limit_adapter = adapter_harness_id
            .as_deref()
            .and_then(|hid| state.adapters.get(hid));
        let stale_cap_recheck = prev_latch && !pending && origin.is_some();
        let baseline_scoped_detection = !prev_latch && !pending && origin.is_some();
        merged.at_usage_limit = limit_adapter.map(|adapter| {
            let detected_full =
                peon::detect_usage_limit(&adapter.limit_patterns, &snapshot)
                    || peon::detect_usage_limit_raw(&adapter.limit_patterns, &scan_buf);
            if stale_cap_recheck && fresh_output_since_origin {
                let (line_count, scan_len) = origin.unwrap();
                let line_window_start = output_lines_seen.saturating_sub(snapshot.len() as u64);
                let scan_window_start = scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                let fresh_line_start = line_count.saturating_sub(line_window_start) as usize;
                let fresh_scan_start = scan_len.saturating_sub(scan_window_start) as usize;
                let fresh_lines = snapshot.get(fresh_line_start.min(snapshot.len())..).unwrap_or(&[]);
                let fresh_scan = scan_buf.get(fresh_scan_start.min(scan_buf.len())..).unwrap_or("");
                let detected_scoped =
                    peon::detect_usage_limit(&adapter.limit_patterns, fresh_lines)
                        || peon::detect_usage_limit_raw(&adapter.limit_patterns, fresh_scan);
                capped_recheck_resets.insert(id.clone());
                if !detected_scoped {
                    capped_clear_baselines.insert(id.clone(), (output_lines_seen, scan_bytes_seen));
                }
                detected_scoped
            } else if baseline_scoped_detection {
                let (line_count, scan_len) = origin.unwrap();
                let line_window_start = output_lines_seen.saturating_sub(snapshot.len() as u64);
                let scan_window_start = scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                let fresh_line_start = line_count.saturating_sub(line_window_start) as usize;
                let fresh_scan_start = scan_len.saturating_sub(scan_window_start) as usize;
                let fresh_lines = snapshot.get(fresh_line_start.min(snapshot.len())..).unwrap_or(&[]);
                let fresh_scan = scan_buf.get(fresh_scan_start.min(scan_buf.len())..).unwrap_or("");
                let detected_scoped =
                    peon::detect_usage_limit(&adapter.limit_patterns, fresh_lines)
                        || peon::detect_usage_limit_raw(&adapter.limit_patterns, fresh_scan);
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
        let detected_reset_hint = limit_adapter.and_then(|adapter| {
            if stale_cap_recheck && fresh_output_since_origin {
                let (line_count, scan_len) = origin.unwrap();
                let line_window_start = output_lines_seen.saturating_sub(snapshot.len() as u64);
                let scan_window_start = scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                let fresh_line_start = line_count.saturating_sub(line_window_start) as usize;
                let fresh_scan_start = scan_len.saturating_sub(scan_window_start) as usize;
                let fresh_lines = snapshot.get(fresh_line_start.min(snapshot.len())..).unwrap_or(&[]);
                let fresh_scan = scan_buf.get(fresh_scan_start.min(scan_buf.len())..).unwrap_or("");
                peon::detect_usage_limit_hint(&adapter.limit_patterns, fresh_lines)
                    .or_else(|| peon::detect_usage_limit_hint_raw(&adapter.limit_patterns, fresh_scan))
            } else if baseline_scoped_detection {
                let (line_count, scan_len) = origin.unwrap();
                let line_window_start = output_lines_seen.saturating_sub(snapshot.len() as u64);
                let scan_window_start = scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
                let fresh_line_start = line_count.saturating_sub(line_window_start) as usize;
                let fresh_scan_start = scan_len.saturating_sub(scan_window_start) as usize;
                let fresh_lines = snapshot.get(fresh_line_start.min(snapshot.len())..).unwrap_or(&[]);
                let fresh_scan = scan_buf.get(fresh_scan_start.min(scan_buf.len())..).unwrap_or("");
                peon::detect_usage_limit_hint(&adapter.limit_patterns, fresh_lines)
                    .or_else(|| peon::detect_usage_limit_hint_raw(&adapter.limit_patterns, fresh_scan))
            } else {
                peon::detect_usage_limit_hint(&adapter.limit_patterns, &snapshot)
                    .or_else(|| peon::detect_usage_limit_hint_raw(&adapter.limit_patterns, &scan_buf))
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
    }).collect();

    // Append remembered (non-live) sessions from metadata
    for meta in &all_metadata_sessions {
        if all_memory_ids.contains(&meta.id) {
            continue;
        }
        let session_harness_id = (!meta.harness.is_empty()).then(|| meta.harness.as_str());
        let adapter_harness_id = resolve_adapter_harness_id(&harnesses, session_harness_id);
        let caps = capabilities_for_harness(&state.adapters, adapter_harness_id.as_deref());
        let (memory_state, resume_strategy) =
            derive_memory_state(false, meta.resume.as_ref(), &caps);
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
                caps.resume_exact,
                caps.resume_latest_in_cwd,
                caps.resume_latest_in_repo,
            ),
            resumed_from: meta.resumed_from.clone(),
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
                if info.at_usage_limit == Some(true) {
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
            harness_reset_hint.entry(hid.clone()).or_insert_with(|| hint.clone());
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
    state.providers.update_session_capping(harness_capped, harness_reset_hint, provider_checking);

    enrich_sessions_with_git_context(&mut infos, git::detect);

    let conflict_warnings = detect_conflicts(&infos);
    for info in &mut infos {
        info.conflict_warning = conflict_warnings.iter()
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
    state.peon.last_output.write().unwrap().remove(&id);
    state.peon.last_inference.write().unwrap().remove(&id);
    state.peon.input_buf.write().unwrap().remove(&id);
    axum::http::StatusCode::OK
}

pub(crate) async fn forget_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    {
        let sessions = state.sessions.lock().unwrap();
        if let Some(h) = sessions.get(&id) {
            if h.info.status == "live" || h.info.status == "creating" || h.info.status == "running" {
                return (axum::http::StatusCode::CONFLICT, "Cannot forget a live session. Kill it first.").into_response();
            }
        }
    }

    let ws_guard = state.workspace.lock().unwrap();
    let ws = match &*ws_guard {
        Some(ws) => ws,
        None => return axum::http::StatusCode::CONFLICT.into_response(),
    };

    // Existence, not parseability: a corrupt-but-present metadata file must
    // still be forgettable, or the session becomes undeletable via the API.
    if !ws.metadata.session_file_exists(&id) {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }

    if let Err(e) = ws.metadata.delete_session(&id) {
        tracing::error!(session_id = %id, error = %e, "failed to delete session");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if let Err(e) = ws.metadata.delete_events(&id) {
        tracing::error!(session_id = %id, error = %e, "failed to delete session events");
    }
    let _ = ws.metadata.clear_last_active_session_if_matches(&id);
    drop(ws_guard);

    state.sessions.lock().unwrap().remove(&id);
    state.peon.last_output.write().unwrap().remove(&id);
    state.peon.last_inference.write().unwrap().remove(&id);

    axum::http::StatusCode::OK.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::runtime::terminal_runtime::set_session_status;

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

        enrich_sessions_with_git_context(&mut infos, |cwd| {
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
        assert_eq!(infos[0].repo_root.as_deref(), Some("/workspace/shared/repo"));
        assert_eq!(infos[1].branch.as_deref(), Some("test-branch"));
        assert_eq!(infos[1].dirty, Some(true));
        assert_eq!(infos[1].changed_files, Some(2));
        assert_eq!(infos[2].is_worktree, Some(true));
        assert!(infos[0].recommendation.is_some());
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
            at_usage_limit_latched: false,
            capacity_check_pending: false,
            output_lines_seen: 0,
            scan_bytes_seen: 0,
            resume_scan_origin: None,
            pending_capacity_visible_once: false,
        }
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
            ws.as_ref().unwrap().metadata.write_session(&metadata::SessionMetadata {
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
            updated.resume.as_ref().and_then(|r| r.harness_session_id.as_deref()),
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
            ws.as_ref().unwrap().metadata.write_session(&metadata::SessionMetadata {
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
        let updated = ws.as_ref().unwrap().metadata.read_session(&session_id).unwrap();
        let updated_resume = updated.resume.unwrap();
        assert_eq!(updated_resume.harness_session_id.as_deref(), Some("native-123"));
        assert_ne!(updated_resume.last_seen_at.as_deref(), Some("before"));
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: true,
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
            ws.as_ref().unwrap().metadata.write_session(&metadata::SessionMetadata {
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
            ws.as_ref().unwrap().metadata.write_session(&metadata::SessionMetadata {
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
            ws.as_ref().unwrap().metadata.write_session(&metadata::SessionMetadata {
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
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws.as_ref().unwrap().metadata.read_session("attention-known").unwrap();
        assert_eq!(updated.observed_status.as_deref(), Some("waiting_for_input"));
        assert_eq!(updated.metadata_source, "agent");
    }

    #[tokio::test]
    async fn apply_debug_attention_rejects_invalid_attention() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let response = apply_debug_attention(
            State(state),
            Path("missing".into()),
            Json(DebugAttentionRequest { attention: "not_a_real_value".into(), message: None }),
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
            Json(DebugAttentionRequest { attention: "working".into(), message: None }),
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
            ws.as_ref().unwrap().metadata.write_session(&test_session_metadata(
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
            Json(DebugAttentionRequest { attention: "working".into(), message: None }),
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
            Json(DebugAttentionRequest { attention: "blocked".into(), message: None }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws.as_ref().unwrap().metadata.read_session("debug-alive").unwrap();
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
            Json(DebugAttentionRequest { attention: "capped".into(), message: None }),
        )
        .await
        .into_response();

        // Ignored (not rejected) mirrors report_attention's own handling of an
        // unwritable target: the request is well-formed, it just didn't land.
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws.as_ref().unwrap().metadata.read_session("debug-vs-agent").unwrap();
        assert_eq!(updated.observed_status.as_deref(), Some("working"), "a live agent signal must survive a debug injection");
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
            Json(DebugAttentionRequest { attention: "needs_you".into(), message: None }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws.as_ref().unwrap().metadata.read_session("debug-needs-you").unwrap();
        assert_eq!(updated.observed_status.as_deref(), Some("waiting_for_input"));
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
            Json(DebugAttentionRequest { attention: "failed".into(), message: None }),
        )
        .await
        .into_response();
        {
            let ws = state.workspace.lock().unwrap();
            let injected = ws.as_ref().unwrap().metadata.read_session("debug-reclaim").unwrap();
            assert_eq!(injected.metadata_source, "debug");
        }

        // Any real attention source is unconditionally accepted over "debug",
        // since debug is the lowest documented priority tier. "blocked" is
        // accepted regardless of active_work_hook capability, unlike "working".
        let _ = report_attention(
            State(state.clone()),
            Path("debug-reclaim".into()),
            Json(AttentionReportRequest { status: "blocked".into(), message: None }),
        )
        .await
        .into_response();

        let ws = state.workspace.lock().unwrap();
        let reclaimed = ws.as_ref().unwrap().metadata.read_session("debug-reclaim").unwrap();
        assert_eq!(reclaimed.metadata_source, "agent");
        assert_eq!(reclaimed.observed_status.as_deref(), Some("blocked"));
    }

    #[tokio::test]
    async fn apply_debug_attention_capped_message_lands_in_usage_limit_reset_hint_not_summary() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let mut harnesses = state.harnesses.write().await;
            *harnesses = crate::harness_registry::builtin_harness_configs();
        }

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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();

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
        let updated = ws.as_ref().unwrap().metadata.read_session(&created_id).unwrap();
        assert_eq!(updated.summary, None, "capped hint must not land in the generic summary field");
        drop(ws);

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[&created_id].info.usage_limit_reset_hint.as_deref(),
            Some("resets in 2h"),
        );
        drop(sessions);

        assert_eq!(
            delete_session(State(state), Path(created_id)).await.into_response().status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn debug_injected_capped_hint_survives_list_sessions_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let mut harnesses = state.harnesses.write().await;
            *harnesses = crate::harness_registry::builtin_harness_configs();
        }

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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();

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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
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
            delete_session(State(state), Path(created_id)).await.into_response().status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn debug_injection_off_capped_clears_stale_reset_hint() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let mut harnesses = state.harnesses.write().await;
            *harnesses = crate::harness_registry::builtin_harness_configs();
        }

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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();

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
            Json(DebugAttentionRequest { attention: "working".into(), message: None }),
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
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
            delete_session(State(state), Path(created_id)).await.into_response().status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn apply_debug_attention_capped_without_message_does_not_clear_existing_hint() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let mut harnesses = state.harnesses.write().await;
            *harnesses = crate::harness_registry::builtin_harness_configs();
        }

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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();

        // A real capacity hint is already present, as if genuine harness capacity
        // detection had set it before the debug picker was ever touched.
        {
            let mut sessions = state.sessions.lock().unwrap();
            sessions.get_mut(&created_id).unwrap().info.usage_limit_reset_hint =
                Some("resets in 45m".into());
        }

        let response = apply_debug_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(DebugAttentionRequest { attention: "capped".into(), message: None }),
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
            delete_session(State(state), Path(created_id)).await.into_response().status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn created_capable_session_normalizes_thinking_to_working() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let mut harnesses = state.harnesses.write().await;
            *harnesses = crate::harness_registry::builtin_harness_configs();
            harnesses
                .iter_mut()
                .find(|h| h.id == "generic-shell")
                .unwrap()
                .attention
                .active_work_hook = true;
        }

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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created_id = serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(
            state.sessions.lock().unwrap()[&created_id].active_work_hook,
            "the created handle should retain the capable harness's hook support"
        );

        let response = report_attention(
            State(state.clone()),
            Path(created_id.clone()),
            Json(AttentionReportRequest { status: "thinking".into(), message: None }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let ws = state.workspace.lock().unwrap();
        let updated = ws.as_ref().unwrap().metadata.read_session(&created_id).unwrap();
        assert_eq!(updated.observed_status.as_deref(), Some("working"));
        drop(ws);
        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions[&created_id].info.observed_status.as_deref(),
            Some("working")
        );
        drop(sessions);

        assert_eq!(
            delete_session(State(state), Path(created_id)).await.into_response().status(),
            axum::http::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn unsupported_hook_rejects_thinking_without_changing_attention() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.write_session(&test_session_metadata(
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
            Json(AttentionReportRequest { status: "thinking".into(), message: None }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let ws = state.workspace.lock().unwrap();
        let updated = ws.as_ref().unwrap().metadata.read_session("attention-unsupported-thinking").unwrap();
        assert_eq!(updated.observed_status, None);
    }

    #[tokio::test]
    async fn session_without_active_work_hook_rejects_thinking_after_registry_changes() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        {
            let mut harnesses = state.harnesses.write().await;
            *harnesses = crate::harness_registry::builtin_harness_configs();
            harnesses
                .iter_mut()
                .find(|h| h.id == "claude-code")
                .unwrap()
                .attention
                .active_work_hook = true;
        }
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
            Json(AttentionReportRequest { status: "thinking".into(), message: None }),
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
            ws.as_ref().unwrap().metadata.write_session(&test_session_metadata(
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
            ws.as_ref().unwrap().metadata.write_session(&test_session_metadata(
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
            std::fs::create_dir_all(
                store.sessions_dir().join("attention-persist-fail.json.tmp"),
            )
            .unwrap();
        }

        let response = report_attention(
            State(state),
            Path("attention-persist-fail".into()),
            Json(AttentionReportRequest {
                status: "waiting_for_input".into(),
                message: None,
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
            (json_path, store.sessions_dir().join("corrupt-forget.json.corrupt"))
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
            assert_eq!(store.read_terminal_output(&session_id, 10), vec!["hello".to_string()]);
        }

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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
            workspace: std::sync::Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let matching = sessions
            .iter()
            .filter(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
            ws.as_ref().unwrap().metadata.write_session(&metadata::SessionMetadata {
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
        assert_eq!(response.into_response().status(), axum::http::StatusCode::OK);

        let info = state.sessions.lock().unwrap().get(&session_id).unwrap().info.clone();
        assert_eq!(info.status, "running");
        assert_eq!(info.lifecycle_phase, "ending");

        let ws = state.workspace.lock().unwrap();
        let meta = ws.as_ref().unwrap().metadata.read_session(&session_id).unwrap();
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
            .unwrap();

        assert_eq!(session.get("connectivity").and_then(|value| value.as_str()), Some("offline"));
        assert_eq!(session.get("terminalOutcome").and_then(|value| value.as_str()), Some("ended"));
        assert_eq!(
            session.get("lastActivityAt").and_then(|value| value.as_str()),
            Some("2026-06-28T09:05:00Z"),
        );
    }

    #[tokio::test]
    async fn list_sessions_keeps_pending_without_fresh_resume_output() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
            .unwrap();

        assert_eq!(session.get("capacityCheckPending").and_then(|value| value.as_bool()), Some(true));
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
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
        let opencode = response.providers.iter().find(|provider| provider.id == "opencode").unwrap();
        assert_eq!(opencode.effective_state, "checking_capacity");
    }

    #[tokio::test]
    async fn list_sessions_requires_one_visible_fresh_output_cycle_before_clearing_pending() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(None),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
            .unwrap();
        assert_eq!(session.get("capacityCheckPending").and_then(|value| value.as_bool()), Some(true));

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
            .unwrap();
        assert_eq!(session.get("capacityCheckPending"), None);
    }

    #[tokio::test]
    async fn list_sessions_does_not_mark_remembered_sessions_capped_from_other_live_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        state.providers.apply_settings(crate::providers::ProviderSettingsPayload {
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let live = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(live_id.as_str()))
            .unwrap();
        let remembered = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some("remembered-codex"))
            .unwrap();

        assert_eq!(live.get("atUsageLimit").and_then(|value| value.as_bool()), Some(true));
        assert_eq!(remembered.get("memoryState").and_then(|value| value.as_str()), Some("remembered"));
        assert_eq!(remembered.get("atUsageLimit"), None);

        let providers = state.providers.get_providers_response();
        let codex = providers.providers.iter().find(|provider| provider.id == "codex").unwrap();
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
            .unwrap();

        assert_eq!(session.get("atUsageLimit").and_then(|value| value.as_bool()), Some(false));

        let response = list_sessions(State(state.clone())).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
            .unwrap();

        assert_eq!(session.get("atUsageLimit").and_then(|value| value.as_bool()), Some(false));
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
            .unwrap();

        assert_eq!(session.get("atUsageLimit").and_then(|value| value.as_bool()), Some(true));
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
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
            .unwrap();

        assert_eq!(session.get("atUsageLimit").and_then(|value| value.as_bool()), Some(false));
    }

    #[tokio::test]
    async fn list_sessions_derives_resume_options_for_remembered_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        let state = Arc::new(crate::AppState {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            workspace: std::sync::Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: crate::PeonState {
                last_output: std::sync::RwLock::new(std::collections::HashMap::new()),
                last_inference: std::sync::RwLock::new(std::collections::HashMap::new()),
                in_flight: std::sync::RwLock::new(std::collections::HashSet::new()),
                label_hint: std::sync::RwLock::new(std::collections::HashMap::new()),
                label_pending: std::sync::RwLock::new(std::collections::HashSet::new()),
                input_buf: std::sync::RwLock::new(std::collections::HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: crate::harness_registry::builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(crate::RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let session = sessions
            .iter()
            .find(|session| session.get("id").and_then(|id| id.as_str()) == Some("remembered-derived"))
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

    #[test]
    fn resolve_session_launch_codex_wires_to_codex_adapter() {
        let harnesses = crate::harness_registry::builtin_harness_configs();
        let launch = resolve_session_launch(
            &harnesses,
            &CreateSessionRequest {
                harness_id: Some("codex".into()),
                model: None,
                initial_prompt: None,
            },
            "/repo".into(),
        );

        assert_eq!(launch.session_harness_id.as_deref(), Some("codex"));
        assert_eq!(launch.adapter_harness_id.as_deref(), Some("codex"));
        assert_eq!(launch.command.program, "codex");
    }

    #[test]
    fn resolve_session_launch_opencode_no_model_omits_model_args() {
        let harnesses = crate::harness_registry::builtin_harness_configs();
        let launch = resolve_session_launch(
            &harnesses,
            &CreateSessionRequest { harness_id: Some("opencode".into()), model: None, initial_prompt: None },
            "/repo".into(),
        );
        assert!(!launch.command.args.contains(&"--model".into()), "bare --model should be dropped");
        assert!(!launch.command.args.iter().any(|a| a.starts_with("ollama/")), "bare prefix should not appear");
    }

    #[test]
    fn resolve_session_launch_opencode_with_model_uses_prefix() {
        let harnesses = crate::harness_registry::builtin_harness_configs();
        let launch = resolve_session_launch(
            &harnesses,
            &CreateSessionRequest {
                harness_id: Some("opencode".into()),
                model: Some("qwen2.5-coder:latest".into()),
                initial_prompt: None,
            },
            "/repo".into(),
        );
        assert!(launch.command.args.contains(&"ollama/qwen2.5-coder:latest".into()));
    }

    #[test]
    fn resolve_session_launch_does_not_infer_model_provider_from_harness() {
        let harnesses = crate::harness_registry::builtin_harness_configs();
        let launch = resolve_session_launch(
            &harnesses,
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
}
