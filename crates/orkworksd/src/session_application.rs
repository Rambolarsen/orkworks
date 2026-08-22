use crate::{git, metadata, migration, watcher, AppState, WorkspaceState};
use crate::workspace_runtime::{iso_now, orkworks_global_dir};
use crate::session_types::{MemoryState, SessionInfo};
use crate::session_view::{connectivity_for_status, terminal_outcome_for_status};
use crate::plan_handoff::resolve_printed_plan_path;
use crate::runtime::observed_status::apply_attention_signal;
use crate::workspace_runtime::parse_hook_observed_at;
use axum::response::{IntoResponse, Response};
use portable_pty::PtySize;
use std::path::PathBuf;
use std::sync::Arc;
use crate::{harness, peon, SessionHandle};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SessionError {
    BadRequest(&'static str),
    EmptyBadRequest,
    Conflict,
    NotFound,
    Internal(&'static str),
}

pub(crate) struct WorkspaceSnapshot {
    pub(crate) path: String,
    pub(crate) repo_root: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) dirty: Option<bool>,
    pub(crate) last_active_session_id: Option<String>,
    pub(crate) active_harness_ids: Vec<String>,
}

pub(crate) struct SessionApplication {
    state: Arc<AppState>,
}

pub(crate) type SessionSnapshot = Response;

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

pub(crate) struct PlanSelection {
    pub(crate) printed_path: String,
}

impl SessionApplication {
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub(crate) fn open_workspace(
        &self,
        path: PathBuf,
    ) -> Result<WorkspaceSnapshot, SessionError> {
        if !path.is_dir() {
            return Err(SessionError::BadRequest("not a directory"));
        }
        let global_dir = orkworks_global_dir(&path)
            .ok_or(SessionError::Internal("no home directory"))?;
        for dir in &["sessions", "events", "capacity", "skills"] {
            if let Err(error) = std::fs::create_dir_all(global_dir.join(dir)) {
                tracing::warn!(path = %global_dir.display(), dir, %error, "failed to create metadata dir");
            }
        }

        let store = metadata::MetadataStore::new(&global_dir);
        migration::migrate_if_needed(&path, &global_dir);
        let memory = store.read_workspace_memory();
        let last_active_session_id = memory
            .as_ref()
            .and_then(|memory| memory.last_active_session_id.clone());
        let active_harness_ids = memory.map(|memory| memory.active_harness_ids).unwrap_or_default();
        let watcher = watcher::MetadataWatcher::start(&global_dir.join("sessions"));

        let mut workspace = self.state.workspace.lock().unwrap();
        *workspace = Some(WorkspaceState { path: path.clone(), metadata: store, watcher });
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

        let git_context = git::detect(&path);
        Ok(WorkspaceSnapshot {
            path: path.display().to_string(),
            repo_root: git_context.repo_root,
            branch: git_context.branch,
            dirty: Some(git_context.dirty),
            last_active_session_id,
            active_harness_ids,
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
            return self.workspace_exists().then_some(()).ok_or(SessionError::Conflict);
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
            if !state.workspace.lock().unwrap().is_some() {
                return Err(SessionError::Conflict);
            }
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
            apply_attention_signal(
                &state, &merge_id, &merge_status, message.as_deref(), &plan_path, &iso_now(), "agent", 1.0,
                observed_at,
            )
            .ok_or(SessionError::Conflict)
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
            if bufs.get(&id).is_some_and(|buf| !peon::is_descriptive_input(buf)) {
                bufs.remove(&id);
            }
        }
        match result {
            metadata::AttentionMergeResult::Accepted | metadata::AttentionMergeResult::Ignored => Ok(()),
            metadata::AttentionMergeResult::NotFound => Err(SessionError::NotFound),
            metadata::AttentionMergeResult::PersistFailed => Err(SessionError::Internal("application operation failed")),
        }
    }

    fn workspace_exists(&self) -> bool { self.state.workspace.lock().unwrap().is_some() }

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

    pub(crate) async fn delete_session(
        &self,
        id: &str,
        forget: bool,
    ) -> Result<SessionSnapshot, SessionError> {
        let response = if forget {
            crate::http::session_handlers::forget_session_legacy(
                axum::extract::State(self.state.clone()),
                axum::extract::Path(id.to_string()),
            )
            .await
            .into_response()
        } else {
            crate::http::session_handlers::delete_session_legacy(
                axum::extract::State(self.state.clone()),
                axum::extract::Path(id.to_string()),
            )
            .await
            .into_response()
        };
        Ok(response)
    }
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
            return Err(crate::session_application::SessionError::Internal("application operation failed"));
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
    else { return; };
    if sessions.values().any(|handle| {
        handle.info.harness_id.as_deref() == Some(harness_id.as_str())
            && handle.at_usage_limit_latched
            && handle.runtime.usage_limit_latched_at.is_some_and(|latched_at| latched_at > observed_at)
    }) { return; }
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

    fn attention_test_handle(id: &str, cwd: &std::path::Path) -> SessionHandle {
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        SessionHandle {
            info: crate::test_support::test_session_info(
                id, "Attention", cwd.display().to_string(), "running", "now",
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

    #[test]
    fn opening_a_workspace_returns_its_application_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let application = SessionApplication::new(state);

        let snapshot = application.open_workspace(root.path().to_path_buf()).unwrap();

        assert_eq!(snapshot.path, root.path().to_string_lossy());
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
        ws.as_ref().unwrap().metadata.write_terminal_size(id, 120, 40);
        drop(ws);

        let result = SessionApplication::new(state.clone()).resume_session(id).await;

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
            id, "Attention", root.path().display().to_string(), "running", "before", "before",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&meta);

        SessionApplication::new(state.clone())
            .report_attention(id, AttentionSignal {
                status: "waiting_for_input".into(), message: Some("question".into()),
                plan_path: metadata::PlanPathUpdate::Unchanged, observed_at: None, cwd: None,
            }).await.unwrap();

        let stored = state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_session(id).unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("waiting_for_input"));
        assert_eq!(stored.attention.as_deref(), Some("needs_you"));
    }

    #[tokio::test]
    async fn report_attention_application_seam_rejects_invalid_status() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let result = SessionApplication::new(state).report_attention("missing", AttentionSignal {
            status: "invalid".into(), message: None, plan_path: metadata::PlanPathUpdate::Unchanged,
            observed_at: None, cwd: None,
        }).await;
        assert!(matches!(result, Err(SessionError::EmptyBadRequest)));
    }

    #[tokio::test]
    async fn report_attention_application_seam_ignores_stale_signal() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "attention-stale-application";
        let mut meta = crate::test_support::test_session_metadata(
            id, "Attention", root.path().display().to_string(), "running", "before", "before",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        meta.observed_status = Some("working".into());
        meta.attention = Some("working".into());
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&meta);
        let mut handle = attention_test_handle(id, root.path());
        handle.runtime.accepted_input_at = Some(parse_hook_observed_at("2026-08-22T08:00:01.000000Z").unwrap());
        state.sessions.lock().unwrap().insert(id.into(), handle);

        SessionApplication::new(state.clone())
            .report_attention(id, AttentionSignal {
                status: "waiting_for_input".into(), message: Some("old".into()),
                plan_path: metadata::PlanPathUpdate::Unchanged,
                observed_at: Some("2026-08-22T08:00:00.000000Z".into()), cwd: Some("/stale".into()),
            }).await.unwrap();

        let stored = state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_session(id).unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("working"));
        assert_eq!(state.peon.reported_cwd.read().unwrap().get(id), None);
    }

    #[tokio::test]
    async fn report_attention_application_seam_returns_persistence_failure() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "attention-persist-failure";
        let mut meta = crate::test_support::test_session_metadata(
            id, "Attention", root.path().display().to_string(), "running", "before", "before",
        );
        meta.lifecycle = "alive".into();
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&meta);
        std::fs::create_dir_all(root.path().join(".orkworks-test/sessions/attention-persist-failure.json.tmp")).unwrap();

        let result = SessionApplication::new(state).report_attention(id, AttentionSignal {
            status: "waiting_for_input".into(), message: Some("not persisted".into()),
            plan_path: metadata::PlanPathUpdate::Unchanged, observed_at: None, cwd: None,
        }).await;

        assert!(matches!(result, Err(SessionError::Internal("application operation failed"))));
    }

    #[tokio::test]
    async fn select_plan_application_seam_rejects_unresolvable_path() {
        let root = tempfile::tempdir().unwrap();
        git2::Repository::init(root.path()).unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let id = "plan-rejected-application";
        let mut meta = crate::test_support::test_session_metadata(
            id, "Plan", root.path().display().to_string(), "running", "before", "before",
        );
        meta.cwd = root.path().display().to_string();
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&meta);

        let result = SessionApplication::new(state).select_plan(id, PlanSelection {
            printed_path: "../outside-plan.md".into(),
        }).await;

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
            id, "Plan", root.path().display().to_string(), "running", "before", "before",
        );
        meta.cwd = root.path().display().to_string();
        state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&meta);

        SessionApplication::new(state.clone()).select_plan(id, PlanSelection {
            printed_path: "docs/superpowers/plans/task.md".into(),
        }).await.unwrap();

        let stored = state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_session(id).unwrap();
        assert_eq!(stored.plan_path.as_ref().unwrap().source, metadata::PlanSource::UserSelected);
        assert!(state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_events(id)
            .iter().any(|event| event.event_type == "session.plan_selected_by_user"));
    }
}
