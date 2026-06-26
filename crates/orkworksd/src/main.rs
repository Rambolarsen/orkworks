use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use portable_pty::{CommandBuilder, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, RwLock as StdRwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(unix)]
use portable_pty::unix::UnixPtySystem;
#[cfg(windows)]
use portable_pty::win::conpty::ConPtySystem;

mod metadata;
mod watcher;
mod git;
mod harness;
mod peon;
mod providers;
mod domain;
mod application;
mod infrastructure;
mod migration;

use crate::infrastructure::session_module::SessionModule;

#[derive(Clone, Debug, Serialize)]
struct SessionInfo {
    id: String,
    label: String,
    #[serde(rename = "harnessId", skip_serializing_if = "Option::is_none")]
    harness_id: Option<String>,
    #[serde(rename = "modelProviderId", skip_serializing_if = "Option::is_none")]
    model_provider_id: Option<String>,
    #[serde(rename = "modelId", skip_serializing_if = "Option::is_none")]
    model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    harness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    status: String,
    cwd: String,
    created_at: String,
    #[serde(rename = "observedStatus")]
    observed_status: Option<String>,
    pub summary: Option<String>,
    #[serde(rename = "nextAction")]
    next_action: Option<String>,
    #[serde(rename = "needsUserInput")]
    needs_user_input: Option<bool>,
    #[serde(rename = "detectedQuestion")]
    detected_question: Option<String>,
    #[serde(rename = "suggestedOptions")]
    suggested_options: Option<Vec<String>>,
    #[serde(rename = "blockerDescription")]
    blocker_description: Option<String>,
    #[serde(rename = "failedCommand")]
    failed_command: Option<String>,
    #[serde(rename = "failedTest")]
    failed_test: Option<String>,
    #[serde(rename = "capacityHints")]
    capacity_hints: Option<Vec<String>>,
    #[serde(rename = "metadataSource")]
    metadata_source: Option<String>,
    #[serde(rename = "metadataConfidence")]
    metadata_confidence: Option<f64>,
    #[serde(rename = "repoRoot")]
    repo_root: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
    #[serde(rename = "changedFiles")]
    changed_files: Option<usize>,
    #[serde(rename = "isWorktree")]
    is_worktree: Option<bool>,
    #[serde(rename = "conflictWarning")]
    conflict_warning: Option<String>,
    recommendation: Option<String>,
    #[serde(rename = "peonLastInference")]
    peon_last_inference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(rename = "providerModel", skip_serializing_if = "Option::is_none")]
    provider_model: Option<String>,
    #[serde(rename = "providerState", skip_serializing_if = "Option::is_none")]
    provider_state: Option<String>,
    #[serde(rename = "memoryState")]
    memory_state: MemoryState,
    #[serde(rename = "resumeStrategy")]
    resume_strategy: harness::ResumeStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    resume: Option<harness::ResumeMemory>,
    #[serde(rename = "resumedFrom", skip_serializing_if = "Option::is_none")]
    resumed_from: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MemoryState {
    Live,
    Remembered,
    Resumable,
    Unsupported,
}

struct SessionHandle {
    info: SessionInfo,
    kill_tx: tokio::sync::watch::Sender<bool>,
    output_buffer: peon::RingBuffer,
    command: harness::CommandSpec,
    initial_prompt: Option<String>,
}

struct WorkspaceState {
    path: PathBuf,
    metadata: metadata::MetadataStore,
    #[allow(dead_code)]
    watcher: watcher::MetadataWatcher,
}

struct PeonState {
    last_output: StdRwLock<HashMap<String, tokio::time::Instant>>,
    last_inference: StdRwLock<HashMap<String, String>>,
    in_flight: StdRwLock<HashSet<String>>,
    label_hint: StdRwLock<HashMap<String, String>>,
    label_pending: StdRwLock<HashSet<String>>,
    config: peon::PeonConfig,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct RetentionConfig {
    #[serde(rename = "maxSessions", default)]
    max_sessions: usize,
    #[serde(rename = "maxAgeDays", default)]
    max_age_days: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct HarnessVoiceCapabilities {
    #[serde(rename = "nativeVoice", default)]
    native_voice: bool,
    #[serde(rename = "requiresMicrophonePermission", default)]
    requires_microphone_permission: bool,
    #[serde(rename = "orkworksDictation", default)]
    orkworks_dictation: bool,
    #[serde(rename = "orkworksVoiceCommands", default)]
    orkworks_voice_commands: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HarnessConfig {
    id: String,
    name: String,
    harness: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(rename = "defaultModel", default)]
    default_model: String,
    #[serde(rename = "modelPrefix", default)]
    model_prefix: String,
    #[serde(default)]
    capabilities: HarnessVoiceCapabilities,
    #[serde(rename = "isBuiltin", default)]
    is_builtin: bool,
}

struct AppState {
    session_module: SessionModule,
    sessions: Mutex<HashMap<String, SessionHandle>>,
    workspace: Mutex<Option<WorkspaceState>>,
    peon: PeonState,
    providers: providers::ProviderManager,
    adapters: HashMap<String, harness::HarnessAdapter>,
    retention_config: tokio::sync::RwLock<RetentionConfig>,
    harnesses: tokio::sync::RwLock<Vec<HarnessConfig>>,
    bound_port: AtomicU16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "orkworksd=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = Arc::new(AppState {
        session_module: SessionModule::new(),
        sessions: Mutex::new(HashMap::new()),
        workspace: Mutex::new(None),
        peon: PeonState {
            last_output: StdRwLock::new(HashMap::new()),
            last_inference: StdRwLock::new(HashMap::new()),
            in_flight: StdRwLock::new(HashSet::new()),
            label_hint: StdRwLock::new(HashMap::new()),
            label_pending: StdRwLock::new(HashSet::new()),
            config: peon::PeonConfig::from_env(),
        },
        providers: providers::ProviderManager::new(),
        adapters: builtin_adapters(),
        retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
        harnesses: tokio::sync::RwLock::new(load_harnesses()),
        bound_port: AtomicU16::new(0),
    });

    // Start Peon background task
    if state.peon.config.enabled {
        let peon_state = state.clone();
        tokio::spawn(async move {
            peon_loop(peon_state).await;
        });
    }

    // Start retention cleanup background task
    {
        let retention_state = state.clone();
        tokio::spawn(async move {
            retention_cleanup_task(retention_state).await;
        });
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/providers", get(get_providers))
        .route("/providers/:id/models", get(get_provider_models))
        .route("/settings/providers", post(set_provider_settings))
        .route("/workspace", post(set_workspace))
        .route("/workspace/active-session", post(set_active_session))
        .route("/workspace/active-harnesses", put(set_active_harnesses))
        .route("/sessions", post(create_session))
        .route("/sessions", get(list_sessions))
        .route("/sessions/:id", delete(delete_session))
        .route("/sessions/:id/forget", delete(forget_session))
        .route("/sessions/:id/resume", post(resume_session))
        .route("/sessions/:id/harness-session", post(report_harness_session))
        .route("/settings/retention", post(set_retention))
        .route("/harnesses", get(list_harnesses).post(create_harness))
        .route("/harnesses/:id", put(update_harness).delete(delete_harness))
        .route("/sessions/:id/terminal", get(session_terminal_handler))
        .route("/sessions/:id/terminal-output", get(get_terminal_output))
        .layer(cors)
        .with_state(state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let bound_addr = listener.local_addr().unwrap();
    state.bound_port.store(bound_addr.port(), Ordering::Relaxed);

    println!("ORKWORKSD_PORT={}", bound_addr.port());

    tracing::info!(addr = %bound_addr, "orkworksd listening");

    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct WorkspaceRequest {
    path: String,
}

#[derive(Deserialize)]
struct ActiveSessionRequest {
    #[serde(rename = "sessionId")]
    session_id: String,
}

#[derive(Deserialize)]
struct ActiveHarnessesRequest {
    #[serde(rename = "activeHarnessIds", default)]
    active_harness_ids: Vec<String>,
}

#[derive(Deserialize)]
struct HarnessSessionReportRequest {
    #[serde(rename = "harnessSessionId")]
    harness_session_id: String,
    source: String,
    confidence: f64,
}

#[derive(Serialize)]
struct WorkspaceResponse {
    path: String,
    repo_root: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
    #[serde(rename = "lastActiveSessionId")]
    last_active_session_id: Option<String>,
    #[serde(rename = "activeHarnessIds", skip_serializing_if = "Vec::is_empty")]
    active_harness_ids: Vec<String>,
}

async fn set_workspace(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WorkspaceRequest>,
) -> impl IntoResponse {
    let ws_path = PathBuf::from(&req.path);
    if !ws_path.is_dir() {
        return (axum::http::StatusCode::BAD_REQUEST, "not a directory").into_response();
    }

    let global_dir = match orksworks_global_dir(&ws_path) {
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

async fn health_check() -> &'static str {
    "ok"
}

async fn set_active_session(
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

async fn set_active_harnesses(
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

async fn resume_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let now = iso_now();
    let harnesses = state.harnesses.read().await.clone();
    let (meta, command, strategy) = {
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
        (meta, command, strategy)
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
        status: "creating".into(),
        cwd: command.cwd.clone(),
        created_at: meta.created_at.clone(),
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
        resume_strategy: strategy,
        resume: meta.resume.clone(),
        resumed_from: meta.resumed_from.clone(),
        provider: meta.provider_label.clone(),
        provider_model: meta.provider_model.clone(),
        provider_state: meta.provider_state.clone(),
    };

    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(&id) {
            handle.info = info.clone();
            handle.kill_tx = kill_tx;
            handle.output_buffer = peon::RingBuffer::new(state.peon.config.max_lines);
            handle.command = command;
        } else {
            sessions.insert(
                id.clone(),
                SessionHandle {
                    info: info.clone(),
                    kill_tx,
                    output_buffer: peon::RingBuffer::new(state.peon.config.max_lines),
                    command,
                    initial_prompt: None,
                },
            );
        }
    }

    {
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            if let Some(mut stored_meta) = ws.metadata.read_session(&id) {
                stored_meta.status = "creating".to_string();
                stored_meta.resume = meta.resume.clone();
                stored_meta.resumed_from = meta.resumed_from.clone();
                ws.metadata.write_session(&stored_meta);
            }
            ws.metadata.append_event(&id, &metadata::Event {
                event_type: "session.resumed".into(),
                timestamp: now,
                status: "creating".into(),
                observed_status: None,
                confidence: None,
            });
        }
    }

    Json(info).into_response()
}

async fn report_harness_session(
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

#[derive(Deserialize, Default)]
struct CreateSessionRequest {
    #[serde(rename = "harnessId", default)]
    harness_id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(rename = "initialPrompt", default)]
    initial_prompt: Option<String>,
}

struct ResolvedSessionLaunch {
    session_harness_id: Option<String>,
    adapter_harness_id: Option<String>,
    model: Option<String>,
    command: harness::CommandSpec,
    provider_id: Option<String>,
    provider_label: Option<String>,
}

fn resolve_session_launch(
    harnesses: &[HarnessConfig],
    req: &CreateSessionRequest,
    cwd: String,
) -> ResolvedSessionLaunch {
    if let Some(ref harness_id) = req.harness_id {
        if let Some(config) = harnesses.iter().find(|h| h.id == *harness_id) {
            let model = req.model.clone().or_else(|| {
                (!config.default_model.is_empty()).then(|| config.default_model.clone())
            });
            let model_value = format!("{}{}", config.model_prefix, model.as_deref().unwrap_or(""));
            let args: Vec<String> = config.args.iter().map(|arg| {
                arg.replace("{model}", &model_value)
            }).collect();
            return ResolvedSessionLaunch {
                session_harness_id: Some(config.id.clone()),
                adapter_harness_id: Some(config.harness.clone()),
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
        model: req.model.clone(),
        command: default_shell_command(cwd),
        provider_id: None,
        provider_label: None,
    }
}

fn resolve_adapter_harness_id(
    harnesses: &[HarnessConfig],
    session_harness_id: Option<&str>,
) -> Option<String> {
    match session_harness_id {
        Some(id) if !id.is_empty() => harnesses
            .iter()
            .find(|h| h.id == id)
            .map(|h| h.harness.clone())
            .or_else(|| Some(id.to_string())),
        _ => None,
    }
}

async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".into());

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
        status: "creating".into(),
        cwd,
        created_at: now.clone(),
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
        resumed_from: None,
        provider: resolved_launch.provider_label.clone(),
        provider_model: None,
        provider_state: None,
    };

    let handle = SessionHandle {
        info: info.clone(),
        kill_tx,
        output_buffer: peon::RingBuffer::new(state.peon.config.max_lines),
        command: resolved_launch.command,
        initial_prompt: req.initial_prompt.clone(),
    };

    state.sessions.lock().unwrap().insert(id.clone(), handle);

    let now = iso_now();
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
            phase: String::new(),
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
            peon_last_inference: None,
            provider_id: resolved_launch.provider_id.clone(),
            provider_label: resolved_launch.provider_label.clone(),
            provider_model: None,
            provider_state: None,
            created_at: now.clone(),
            last_activity: now.clone(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: meta_git_ctx.repo_root.clone(),
            branch: meta_git_ctx.branch.clone(),
            dirty: Some(meta_git_ctx.dirty),
            changed_files: Some(meta_git_ctx.changed_files),
            is_worktree: Some(meta_git_ctx.is_worktree),
            last_user_input: None,
            resume: info.resume.clone(),
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
            resumed_from: info.resumed_from.clone(),
        });
        ws.metadata.append_event(&id, &metadata::Event {
            event_type: "session.created".into(),
            timestamp: now,
            status: "creating".into(),
            observed_status: None,
            confidence: None,
        });
    }
    drop(ws_guard);

    Json(info)
}

fn detect_conflicts(sessions: &[SessionInfo]) -> Vec<(String, String)> {
    let mut cwd_groups: HashMap<&str, Vec<&SessionInfo>> = HashMap::new();
    for s in sessions {
        if s.status == "running" || s.status == "creating" {
            cwd_groups.entry(&s.cwd).or_default().push(s);
        }
    }
    let mut warnings = Vec::new();
    for (_cwd, group) in &cwd_groups {
        if group.len() >= 2 {
            if let Some(s) = group.first() {
                if s.dirty.unwrap_or(false) {
                    let warning = format!("{} sessions in this dirty workspace", group.len());
                    for session in group {
                        warnings.push((session.id.clone(), warning.clone()));
                    }
                }
            }
        }
    }
    warnings
}

fn session_recommendation(ctx: &git::GitContext, session_count_in_cwd: usize) -> Option<String> {
    if ctx.is_worktree {
        return Some("Running in a separate worktree. Good isolation.".into());
    }
    if session_count_in_cwd >= 2 && ctx.dirty {
        return Some("Multiple sessions in the same dirty workspace. Consider separate worktrees.".into());
    }
    if !ctx.is_worktree && ctx.dirty && ctx.branch.as_deref() != Some("main") {
        return Some("Working outside main in a dirty workspace. A worktree may be safer.".into());
    }
    None
}

async fn list_sessions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let harnesses = state.harnesses.read().await.clone();
    let session_data: Vec<(String, String, String, String, String)> = {
        let sessions = state.sessions.lock().unwrap();
        sessions.values().map(|h| {
            (h.info.id.clone(), h.info.label.clone(), h.info.status.clone(), h.info.cwd.clone(), h.info.created_at.clone())
        }).collect()
    };

    let ws_guard = state.workspace.lock().unwrap();
    let metadata_map = ws_guard.as_ref().map(|ws| {
        let mut metadata = HashMap::new();
        for (id, _, _, _, _) in &session_data {
            if let Some(meta) = ws.metadata.read_session(id) {
                metadata.insert(id.clone(), meta);
            }
        }
        metadata
    }).unwrap_or_default();

    let all_metadata_sessions = ws_guard.as_ref().map(|ws| ws.metadata.read_all_sessions()).unwrap_or_default();
    drop(ws_guard);

    let all_memory_ids: HashSet<String> = session_data.iter()
        .map(|(id, _, _, _, _)| id.clone())
        .collect();

    let peon_times = state.peon.last_inference.read().unwrap();
    let mut infos: Vec<SessionInfo> = session_data.into_iter().map(|(id, label, status, cwd, created_at)| {
        let meta = metadata_map.get(&id);
        let session_harness_id = meta.and_then(|m| (!m.harness.is_empty()).then(|| m.harness.as_str()));
        let adapter_harness_id = resolve_adapter_harness_id(&harnesses, session_harness_id);
        let caps = capabilities_for_harness(&state.adapters, adapter_harness_id.as_deref());
        let is_live = status != "killed" && status != "ended" && status != "error";
        let (memory_state, resume_strategy) =
            derive_memory_state(is_live, meta.and_then(|m| m.resume.as_ref()), &caps);
        SessionInfo {
            id: id.clone(),
            label: meta.map(|m| m.label.clone()).unwrap_or(label),
            harness_id: meta.and_then(|m| (!m.harness.is_empty()).then(|| m.harness.clone())),
            model_provider_id: meta.and_then(|m| m.provider_id.clone()),
            model_id: meta.and_then(|m| (!m.model.is_empty()).then(|| m.model.clone())),
            harness: meta.and_then(|m| (!m.harness.is_empty()).then(|| m.harness.clone())),
            model: meta.and_then(|m| (!m.model.is_empty()).then(|| m.model.clone())),
            status,
            cwd,
            created_at,
            observed_status: meta.and_then(|m| m.observed_status.clone()),
            summary: meta.and_then(|m| m.summary.clone()),
            next_action: meta.and_then(|m| m.next_action.clone()),
            needs_user_input: meta.and_then(|m| m.needs_user_input),
            detected_question: meta.and_then(|m| m.detected_question.clone()),
            suggested_options: meta.and_then(|m| m.suggested_options.clone()),
            blocker_description: meta.and_then(|m| m.blocker_description.clone()),
            failed_command: meta.and_then(|m| m.failed_command.clone()),
            failed_test: meta.and_then(|m| m.failed_test.clone()),
            capacity_hints: meta.and_then(|m| m.capacity_hints.clone()),
            metadata_source: meta.map(|m| m.metadata_source.clone()),
            metadata_confidence: meta.map(|m| m.metadata_confidence),
            peon_last_inference: meta
                .and_then(|m| m.peon_last_inference.clone())
                .or_else(|| peon_times.get(&id).cloned()),
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            memory_state,
            resume_strategy,
            resume: meta.and_then(|m| m.resume.clone()),
            resumed_from: meta.and_then(|m| m.resumed_from.clone()),
            provider: meta.and_then(|m| m.provider_label.clone()),
            provider_model: meta.and_then(|m| m.provider_model.clone()),
            provider_state: meta.and_then(|m| m.provider_state.clone()),
        }
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
            status: "ended".into(),
            cwd: meta.cwd.clone(),
            created_at: meta.created_at.clone(),
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
            resume_strategy,
            resume: meta.resume.clone(),
            resumed_from: meta.resumed_from.clone(),
            provider: meta.provider_label.clone(),
            provider_model: meta.provider_model.clone(),
            provider_state: meta.provider_state.clone(),
        });
    }

    let mut cwd_counts: HashMap<String, usize> = HashMap::new();
    for info in &infos {
        if info.status == "running" || info.status == "creating" {
            *cwd_counts.entry(info.cwd.clone()).or_default() += 1;
        }
    }
    for info in &mut infos {
        let ctx = git::detect(&PathBuf::from(&info.cwd));
        let count = cwd_counts.get(&info.cwd).copied().unwrap_or(1);
        info.recommendation = session_recommendation(&ctx, count);
        info.repo_root = ctx.repo_root;
        info.branch = ctx.branch;
        info.dirty = Some(ctx.dirty);
        info.changed_files = Some(ctx.changed_files);
        info.is_worktree = Some(ctx.is_worktree);
    }

    let conflict_warnings = detect_conflicts(&infos);
    for info in &mut infos {
        info.conflict_warning = conflict_warnings.iter()
            .find(|(id, _)| id == &info.id)
            .map(|(_, w)| w.clone());
    }
    Json(infos)
}

async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let handle = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&id).map(|h| h.kill_tx.clone())
    };
    match handle {
        Some(kill_tx) => {
            let _ = kill_tx.send(true);
        }
        None => return axum::http::StatusCode::NOT_FOUND,
    }
    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(h) = sessions.get_mut(&id) {
            h.info.status = "killed".to_string();
        }
    }
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(&id) {
            meta.status = "killed".to_string();
            meta.last_activity = now.clone();
            ws.metadata.write_session(&meta);
        }
        ws.metadata.append_event(&id, &metadata::Event {
            event_type: "session.killed".into(),
            timestamp: now,
            status: "killed".into(),
            observed_status: None,
            confidence: None,
        });
        // Preserve persisted terminal output: the session metadata file remains
        // (status=killed), so the scrollback should remain accessible for the
        // resulting remembered session.
    }
    drop(ws_guard);
    state.peon.last_output.write().unwrap().remove(&id);
    state.peon.last_inference.write().unwrap().remove(&id);
    axum::http::StatusCode::OK
}

async fn forget_session(
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

    if ws.metadata.read_session(&id).is_none() {
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

#[derive(Deserialize)]
struct RetentionRequest {
    #[serde(rename = "maxSessions", default)]
    max_sessions: usize,
    #[serde(rename = "maxAgeDays", default)]
    max_age_days: u32,
}

async fn set_retention(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RetentionRequest>,
) -> impl IntoResponse {
    let mut config = state.retention_config.write().await;
    config.max_sessions = req.max_sessions;
    config.max_age_days = req.max_age_days;
    tracing::info!(
        max_sessions = config.max_sessions,
        max_age_days = config.max_age_days,
        "retention config updated"
    );
    axum::http::StatusCode::OK
}

async fn get_providers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    axum::Json(state.providers.get_providers_response())
}

async fn set_provider_settings(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<providers::ProviderSettingsPayload>,
) -> impl IntoResponse {
    let status = state.providers.apply_settings(payload);
    axum::Json(status)
}

#[derive(Serialize)]
struct ProviderModelsResponse {
    models: Vec<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn get_provider_models(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> impl IntoResponse {
    let providers = state.providers.clone();
    match tokio::task::spawn_blocking(move || providers.list_models(&provider_id)).await {
        Ok(Ok(models)) => axum::Json(ProviderModelsResponse { models }).into_response(),
        Ok(Err(msg)) => {
            let status = if msg.starts_with("unknown provider") {
                axum::http::StatusCode::NOT_FOUND
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, axum::Json(ErrorResponse { error: msg })).into_response()
        }
        Err(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ErrorResponse { error: "internal error".into() })).into_response(),
    }
}

async fn retention_cleanup_task(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;

        let config = state.retention_config.read().await.clone();
        if config.max_sessions == 0 && config.max_age_days == 0 {
            continue;
        }

        let all_sessions = {
            let ws_guard = state.workspace.lock().unwrap();
            match &*ws_guard {
                Some(ws) => ws.metadata.read_all_sessions(),
                None => continue,
            }
        };

        let live_ids: std::collections::HashSet<String> = {
            let sessions = state.sessions.lock().unwrap();
            sessions
                .iter()
                .filter(|(_, h)| {
                    h.info.status == "live"
                        || h.info.status == "creating"
                        || h.info.status == "running"
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        let mut candidates: Vec<_> = all_sessions
            .into_iter()
            .filter(|s| !live_ids.contains(&s.id))
            .collect();

        if candidates.is_empty() {
            continue;
        }

        candidates.sort_by(|a, b| a.last_activity.cmp(&b.last_activity));

        let mut all_deleted: Vec<String> = Vec::new();

        if config.max_age_days > 0 {
            let cutoff = chrono::Utc::now()
                - chrono::Duration::days(config.max_age_days as i64);
            let mut expired: Vec<String> = Vec::new();
            for s in &candidates {
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&s.last_activity) {
                    if parsed < cutoff {
                        expired.push(s.id.clone());
                    }
                }
            }
            if !expired.is_empty() {
                let ws_guard = state.workspace.lock().unwrap();
                if let Some(ref ws) = *ws_guard {
                    for id in &expired {
                        tracing::info!(session_id = %id, "retention: deleting expired session");
                        let _ = ws.metadata.delete_session(id);
                        let _ = ws.metadata.delete_events(id);
                        let _ = ws.metadata.clear_last_active_session_if_matches(id);
                    }
                }
                all_deleted.extend(expired.iter().cloned());
                candidates.retain(|s| !expired.contains(&s.id));
            }
        }

        if config.max_sessions > 0 && candidates.len() > config.max_sessions {
            let to_delete = candidates.len() - config.max_sessions;
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                for s in candidates.iter().take(to_delete) {
                    tracing::info!(
                        session_id = %s.id,
                        max_sessions = config.max_sessions,
                        "retention: deleting session (exceeds max)"
                    );
                    let _ = ws.metadata.delete_session(&s.id);
                    let _ = ws.metadata.delete_events(&s.id);
                    let _ = ws.metadata.clear_last_active_session_if_matches(&s.id);
                    all_deleted.push(s.id.clone());
                }
            }
        }

        if !all_deleted.is_empty() {
            let mut sessions = state.sessions.lock().unwrap();
            let mut peon_output = state.peon.last_output.write().unwrap();
            let mut peon_inference = state.peon.last_inference.write().unwrap();
            for id in &all_deleted {
                sessions.remove(id);
                peon_output.remove(id);
                peon_inference.remove(id);
            }
        }
    }
}

#[derive(Serialize)]
struct TerminalOutputResponse {
    lines: Vec<String>,
}

async fn get_terminal_output(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let state_clone = state.clone();
    let id_clone = id.clone();
    let lines = tokio::task::spawn_blocking(move || {
        let ws_guard = state_clone.workspace.lock().unwrap();
        match &*ws_guard {
            Some(ws) => ws.metadata.read_terminal_output(&id_clone, metadata::TERMINAL_OUTPUT_MAX_LINES),
            None => Vec::new(),
        }
    })
    .await
    .unwrap_or_default();
    Json(TerminalOutputResponse { lines })
}

async fn session_terminal_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let session_status = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&id).map(|h| h.info.status.clone())
    };

    match session_status {
        None => {
            ws.on_upgrade(|mut ws| async move {
                let _ = ws
                    .send(Message::Text("session not found".into()))
                    .await;
                let _ = ws.close().await;
            })
        }
        Some(ref status) if status == "killed" || status == "ended" || status == "error" => {
            let msg = format!("session {status}");
            ws.on_upgrade(move |mut ws| async move {
                let _ = ws.send(Message::Text(msg.into())).await;
                let _ = ws.close().await;
            })
        }
        Some(_) => {
            ws.on_upgrade(move |ws| handle_session_terminal(ws, id, state))
        }
    }
}

fn iso_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

async fn peon_loop(state: Arc<AppState>) {
    let interval = state.peon.config.interval_secs;
    tracing::info!(interval_secs = interval, harness = %state.peon.config.harness, "peon started");

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let now = tokio::time::Instant::now();
        let deadline = now - std::time::Duration::from_secs(interval);

        // Sessions with a pending label inference (input-triggered, no debounce)
        let pending: Vec<String> = state.peon.label_pending.write().unwrap().drain().collect();

        // Sessions with new output that has gone silent
        let mut candidates: Vec<String> = {
            let last_output = state.peon.last_output.read().unwrap();
            let in_flight = state.peon.in_flight.read().unwrap();

            last_output.iter()
                .filter(|(id, &t)| {
                    t <= deadline && !in_flight.contains(*id)
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        for id in pending {
            if !state.peon.in_flight.read().unwrap().contains(&id) && !candidates.contains(&id) {
                candidates.push(id);
            }
        }

        for session_id in candidates {
            {
                let mut in_flight = state.peon.in_flight.write().unwrap();
                if !in_flight.insert(session_id.clone()) {
                    continue;
                }
            }

            let hint = state.peon.label_hint.write().unwrap().remove(&session_id);
            let output_snapshot = {
                let sessions = state.sessions.lock().unwrap();
                match sessions.get(&session_id) {
                    Some(handle) => handle.output_buffer.snapshot(),
                    None => {
                        state.peon.in_flight.write().unwrap().remove(&session_id);
                        continue;
                    }
                }
            };

            if output_snapshot.is_empty() && hint.is_none() {
                state.peon.in_flight.write().unwrap().remove(&session_id);
                continue;
            }

            let output_snapshot = if let Some(ref h) = hint {
                let mut lines = vec![format!("[User input]: {}", h)];
                lines.extend(output_snapshot);
                lines
            } else {
                output_snapshot
            };

            let state_clone = state.clone();
            let id = session_id.clone();

            tokio::task::spawn_blocking(move || {
                let provider_result = state_clone.providers.run_inference(providers::PeonScope::Session, &output_snapshot);
                let inference = provider_result.inference;
                let now_iso = iso_now();

                // Check terminal status before moving inference below
                let reached_terminal = matches!(
                    inference.as_ref().and_then(|inf| inf.observed_status.as_deref()),
                    Some("done" | "idle" | "stale")
                );

                if let Some(ref obs) = provider_result.observation {
                    let ws_guard = state_clone.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        ws.metadata.persist_provider_context(&id, obs);
                    }
                }

                if let Some(inf) = inference {
                    let ws_guard = state_clone.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        let should_write = ws.metadata.read_session(&id)
                            .map(|m| {
                                let age = ws.metadata.session_modified_secs_ago(&id);
                                peon::should_overwrite(&m.metadata_source, age)
                            })
                            .unwrap_or(true);

                        if should_write {
                            ws.metadata.merge_peon_inference(&id, &inf, &now_iso, provider_result.observation.as_ref());
                            if let Some(ref summary) = inf.summary {
                                let label: String = summary.chars().take(100).collect();
                                if let Some(handle) = state_clone.sessions.lock().unwrap().get_mut(&id) {
                                    handle.info.label = label;
                                }
                            }
                        } else {
                            tracing::debug!(session_id = %id, "peon: skipping, higher-priority source exists");
                        }
                    }

                }

                let mut last_inf = state_clone.peon.last_inference.write().unwrap();
                last_inf.insert(id.clone(), now_iso);
                drop(last_inf);

                // Keep re-evaluating unless the session reached a terminal
                // observed status — otherwise peon fires once per output burst
                // and never catches later transitions to done/failed/blocked.
                if !reached_terminal {
                    state_clone.peon.last_output.write().unwrap()
                        .insert(id.clone(), tokio::time::Instant::now());
                }
                state_clone.peon.in_flight.write().unwrap().remove(&id);
            });
        }

        // Timer-based idle detection: mark sessions that have been silent
        // for idle_timeout_secs as idle, without waiting for the LLM.
        {
            let idle_timeout = state.peon.config.idle_timeout_secs;
            let idle_deadline = tokio::time::Instant::now()
                - std::time::Duration::from_secs(idle_timeout);
            let last_output = state.peon.last_output.read().unwrap();

            let silent_ids: Vec<String> = {
                let sessions = state.sessions.lock().unwrap();
                sessions.iter()
                    .filter(|(_, h)| h.info.status == "running" && h.info.observed_status.is_none())
                    .filter(|(id, _)| {
                        last_output.get(*id)
                            .map(|&t| t <= idle_deadline)
                            .unwrap_or(true) // no output ever recorded -> idle
                    })
                    .map(|(id, _)| id.clone())
                    .collect()
            };
            drop(last_output);

            if !silent_ids.is_empty() {
                let ws_guard = state.workspace.lock().unwrap();
                let mut sessions = state.sessions.lock().unwrap();
                if let Some(ref ws) = *ws_guard {
                    for id in &silent_ids {
                        if let Some(mut meta) = ws.metadata.read_session(id) {
                            if meta.observed_status.is_none() {
                                meta.observed_status = Some("idle".into());
                                meta.metadata_source = "process".into();
                                ws.metadata.write_session(&meta);
                            }
                        }
                        if let Some(handle) = sessions.get_mut(id) {
                            handle.info.observed_status = Some("idle".into());
                        }
                    }
                }
            }
        }
    }
}

fn shell_cmd() -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        (
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()),
            vec![],
        )
    } else {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        (shell, vec!["-i".into(), "-l".into()])
    }
}

fn terminal_env_overrides() -> Vec<(String, String)> {
    vec![
        ("TERM".into(), "xterm-256color".into()),
        ("COLORTERM".into(), "truecolor".into()),
        ("FORCE_COLOR".into(), "1".into()),
        ("CLICOLOR".into(), "1".into()),
        ("TERM_PROGRAM".into(), "OrkWorks".into()),
    ]
}

fn session_env_overrides(session_id: &str, port: Option<u16>) -> Vec<(String, String)> {
    let mut env = vec![("ORKWORKS_SESSION_ID".into(), session_id.to_string())];
    if let Some(port) = port {
        env.push(("ORKWORKS_PORT".into(), port.to_string()));
    }
    env
}

fn codex_thread_id_from_jsonl_line(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type").and_then(|v| v.as_str()) != Some("thread.started") {
        return None;
    }
    value.get("thread_id").and_then(|v| v.as_str()).map(str::to_string)
}

// --- Harness config helpers ---

fn global_harnesses_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".orkworks").join("harnesses.json"))
}

fn workspace_hash(path: &std::path::Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

fn orksworks_global_dir(workspace_path: &std::path::Path) -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join(".orkworks")
            .join("workspaces")
            .join(workspace_hash(workspace_path))
    })
}

fn builtin_harness_configs() -> Vec<HarnessConfig> {
    let (shell_program, shell_args) = shell_cmd();
    vec![
        HarnessConfig {
            id: "claude-code".into(),
            name: "Claude Code".into(),
            harness: "claude-code".into(),
            command: "claude".into(),
            args: vec![],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
        },
        HarnessConfig {
            id: "opencode".into(),
            name: "OpenCode".into(),
            harness: "opencode".into(),
            command: "opencode".into(),
            args: vec!["--model".into(), "{model}".into()],
            default_model: String::new(),
            model_prefix: "ollama/".into(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
        },
        HarnessConfig {
            id: "codex".into(),
            name: "Codex".into(),
            harness: "generic-shell".into(),
            command: "codex".into(),
            args: vec![],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
        },
        HarnessConfig {
            id: "gemini".into(),
            name: "Gemini CLI".into(),
            harness: "generic-shell".into(),
            command: "gemini".into(),
            args: vec![],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
        },
        HarnessConfig {
            id: "aider".into(),
            name: "Aider".into(),
            harness: "generic-shell".into(),
            command: "aider".into(),
            args: vec!["--model".into(), "{model}".into()],
            default_model: "claude-sonnet-4-20250514".into(),
            model_prefix: "ollama_chat/".into(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
        },
        HarnessConfig {
            id: "generic-shell".into(),
            name: "Shell".into(),
            harness: "generic-shell".into(),
            command: shell_program,
            args: shell_args,
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
        },
    ]
}

fn load_harnesses() -> Vec<HarnessConfig> {
    let built_ins = builtin_harness_configs();
    let Some(path) = global_harnesses_path() else { return built_ins; };
    let Ok(data) = std::fs::read_to_string(&path) else { return built_ins; };
    let Ok(disk): serde_json::Result<Vec<HarnessConfig>> = serde_json::from_str(&data) else {
        tracing::warn!("failed to parse ~/.orkworks/harnesses.json; using built-ins");
        return built_ins;
    };
    let mut result = built_ins;
    for disk_entry in disk {
        if let Some(pos) = result.iter().position(|h| h.id == disk_entry.id) {
            let is_builtin = result[pos].is_builtin;
            result[pos] = HarnessConfig { is_builtin, ..disk_entry };
        } else {
            result.push(disk_entry);
        }
    }
    result
}

fn save_harnesses(harnesses: &[HarnessConfig]) {
    let Some(path) = global_harnesses_path() else { return; };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(harnesses) {
        Ok(json) => { let _ = std::fs::write(&path, json); }
        Err(e) => tracing::error!(error = %e, "failed to serialize harnesses"),
    }
}

// --- Harness CRUD handlers ---

async fn list_harnesses(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let harnesses = state.harnesses.read().await;
    Json(harnesses.clone())
}

async fn create_harness(
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<HarnessConfig>,
) -> impl IntoResponse {
    req.is_builtin = false;
    if req.id.is_empty() {
        req.id = uuid::Uuid::new_v4().to_string();
    }
    let mut harnesses = state.harnesses.write().await;
    harnesses.push(req.clone());
    save_harnesses(&harnesses);
    (axum::http::StatusCode::CREATED, Json(req)).into_response()
}

async fn update_harness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<HarnessConfig>,
) -> impl IntoResponse {
    let mut harnesses = state.harnesses.write().await;
    if let Some(pos) = harnesses.iter().position(|h| h.id == id) {
        let is_builtin = harnesses[pos].is_builtin;
        harnesses[pos] = HarnessConfig { id: id.clone(), is_builtin, ..req };
        save_harnesses(&harnesses);
        Json(harnesses[pos].clone()).into_response()
    } else {
        axum::http::StatusCode::NOT_FOUND.into_response()
    }
}

async fn delete_harness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut harnesses = state.harnesses.write().await;
    if let Some(pos) = harnesses.iter().position(|h| h.id == id) {
        if harnesses[pos].is_builtin {
            return (axum::http::StatusCode::CONFLICT, "Cannot delete a built-in harness").into_response();
        }
        harnesses.remove(pos);
        save_harnesses(&harnesses);
        axum::http::StatusCode::OK.into_response()
    } else {
        axum::http::StatusCode::NOT_FOUND.into_response()
    }
}

fn default_shell_command(cwd: String) -> harness::CommandSpec {
    let (program, args) = shell_cmd();
    harness::CommandSpec { program, args, cwd }
}

fn default_capabilities() -> harness::HarnessCapabilities {
    harness::HarnessCapabilities {
        launch: true,
        resume_exact: false,
        resume_latest_in_cwd: false,
        resume_latest_in_repo: false,
        detect_session_id: false,
        detect_model: false,
        detect_context_usage: false,
        detect_capacity: false,
        native_voice: false,
    }
}

fn builtin_adapters() -> HashMap<String, harness::HarnessAdapter> {
    let (program, args) = shell_cmd();
    let mut map = HashMap::new();

    let generic = harness::HarnessAdapter::template(
        "generic-shell",
        "Generic Shell",
        default_capabilities(),
        harness::CommandTemplate {
            command: program.clone(),
            args: args.clone(),
        },
        None,
        None,
    );
    map.insert("generic-shell".into(), generic);

    let opencode_caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: true,
        resume_latest_in_cwd: true,
        resume_latest_in_repo: true,
        detect_session_id: true,
        detect_model: true,
        detect_context_usage: true,
        detect_capacity: true,
        native_voice: false,
    };
    let opencode = harness::HarnessAdapter::template(
        "opencode",
        "OpenCode",
        opencode_caps.clone(),
        harness::CommandTemplate {
            command: "opencode".into(),
            args: vec![],
        },
        Some(harness::CommandTemplate {
            command: "opencode".into(),
            args: vec!["--session".into(), "{harnessSessionId}".into()],
        }),
        Some(harness::CommandTemplate {
            command: "opencode".into(),
            args: vec!["--continue".into()],
        }),
    );
    map.insert("opencode".into(), opencode);

    let claude_caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: true,
        resume_latest_in_cwd: true,
        resume_latest_in_repo: true,
        detect_session_id: true,
        detect_model: true,
        detect_context_usage: true,
        detect_capacity: true,
        native_voice: false,
    };
    let claude = harness::HarnessAdapter::template(
        "claude-code",
        "Claude Code",
        claude_caps.clone(),
        harness::CommandTemplate {
            command: "claude".into(),
            args: vec![],
        },
        Some(harness::CommandTemplate {
            command: "claude".into(),
            args: vec!["--resume".into(), "{harnessSessionId}".into()],
        }),
        Some(harness::CommandTemplate {
            command: "claude".into(),
            args: vec!["--continue".into()],
        }),
    );
    map.insert("claude-code".into(), claude);

    map
}

fn capabilities_for_harness(
    adapters: &HashMap<String, harness::HarnessAdapter>,
    harness_id: Option<&str>,
) -> harness::HarnessCapabilities {
    match harness_id {
        Some(h) if !h.is_empty() => adapters
            .get(h)
            .map(|a| a.capabilities.clone())
            .unwrap_or_else(default_capabilities),
        _ => default_capabilities(),
    }
}

fn adapter_for_harness<'a>(
    adapters: &'a HashMap<String, harness::HarnessAdapter>,
    harness_id: Option<&str>,
) -> &'a harness::HarnessAdapter {
    match harness_id {
        Some(h) if !h.is_empty() => adapters.get(h),
        _ => None,
    }
    .unwrap_or_else(|| adapters.get("generic-shell").unwrap())
}

fn derive_memory_state(
    is_live: bool,
    resume: Option<&harness::ResumeMemory>,
    capabilities: &harness::HarnessCapabilities,
) -> (MemoryState, harness::ResumeStrategy) {
    if is_live {
        return (MemoryState::Live, harness::ResumeStrategy::None);
    }
    let Some(resume) = resume else {
        return (MemoryState::Remembered, harness::ResumeStrategy::None);
    };
    let strategy = harness::select_resume_strategy(resume, capabilities);
    if strategy == harness::ResumeStrategy::None {
        (MemoryState::Unsupported, strategy)
    } else {
        (MemoryState::Resumable, strategy)
    }
}

#[derive(Debug, PartialEq)]
enum TerminalAction {
    Input(String),
    Resize { rows: u16, cols: u16 },
    Kill,
    Noop,
}

fn dispatch_terminal_message(msg: &serde_json::Value) -> TerminalAction {
    match msg["type"].as_str() {
        Some("input") => {
            let data = msg["data"].as_str().unwrap_or("").to_string();
            TerminalAction::Input(data)
        }
        Some("resize") => {
            let rows = msg["rows"].as_u64().unwrap_or(24) as u16;
            let cols = msg["cols"].as_u64().unwrap_or(80) as u16;
            TerminalAction::Resize { rows, cols }
        }
        Some("kill") => TerminalAction::Kill,
        _ => TerminalAction::Noop,
    }
}

fn collect_input_line(buf: &mut String, data: &str) -> Option<String> {
    let mut result: Option<String> = None;
    let mut chars = data.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' | '\n' => {
                let raw: String = buf.chars().take(100).collect();
                let line = raw.trim().to_string();
                buf.clear();
                if !line.is_empty() && result.is_none() {
                    result = Some(line);
                }
            }
            '\x7f' => { buf.pop(); }
            '\x03' | '\x04' => { buf.clear(); }
            '\x1b' => {
                match chars.peek().copied() {
                    Some('[') => {
                        // CSI: ESC [ params letter/~
                        chars.next();
                        while let Some(&c) = chars.peek() {
                            chars.next();
                            if c.is_ascii_alphabetic() || c == '~' { break; }
                        }
                    }
                    Some('O') => {
                        // SS3: ESC O letter (arrows/F1-F4 in application cursor mode)
                        chars.next();
                        if chars.peek().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
                            chars.next();
                        }
                    }
                    Some(']') => {
                        // OSC: ESC ] ... BEL or ESC \
                        chars.next();
                        while let Some(c) = chars.next() {
                            if c == '\x07' { break; }
                            if c == '\x1b' {
                                if chars.peek() == Some(&'\\') { chars.next(); }
                                break;
                            }
                        }
                    }
                    Some(_) => { chars.next(); } // alt-key: ESC + one char
                    None => {}                   // bare ESC at end of frame
                }
            }
            ch if !ch.is_ascii_control() => { buf.push(ch); }
            _ => {}
        }
    }
    result
}

fn should_forward_terminal_env(key: &str) -> bool {
    key != "NODE_OPTIONS"
        && key != "VSCODE_INSPECTOR_OPTIONS"
        && !key.starts_with("VSCODE_")
        && !key.starts_with("ELECTRON_")
}

#[cfg(unix)]
fn make_pty_system() -> UnixPtySystem {
    UnixPtySystem {}
}
#[cfg(windows)]
fn make_pty_system() -> ConPtySystem {
    ConPtySystem {}
}

fn set_session_status(state: &Arc<AppState>, id: &str, status: &str) {
    let session_resume = {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(id) {
            handle.info.status = status.to_string();
            (handle.info.resume.clone(), handle.info.resumed_from.clone())
        } else {
            (None, None)
        }
    };
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(id) {
            meta.status = status.to_string();
            meta.last_activity = now.clone();
            if session_resume.0.is_some() {
                meta.resume = session_resume.0;
            }
            if session_resume.1.is_some() {
                meta.resumed_from = session_resume.1;
            }
            ws.metadata.write_session(&meta);
        }
        ws.metadata.append_event(id, &metadata::Event {
            event_type: "session.status".into(),
            timestamp: now,
            status: status.to_string(),
            observed_status: None,
            confidence: None,
        });
    }
}

async fn handle_session_terminal(mut ws: WebSocket, id: String, state: Arc<AppState>) {
    let kill_result = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&id).map(|h| h.kill_tx.subscribe())
    };

    let mut kill_rx = match kill_result {
        Some(rx) => rx,
        None => {
            let _ = ws.close().await;
            return;
        }
    };

    if *kill_rx.borrow() {
        set_session_status(&state, &id, "killed");
        let _ = ws.close().await;
        return;
    }

    {
        let should_reject = {
            let sessions = state.sessions.lock().unwrap();
            sessions
                .get(&id)
                .map(|h| {
                    let s = &h.info.status;
                    s == "killed" || s == "ended" || s == "error"
                })
                .unwrap_or(false)
        };
        if should_reject {
            tracing::warn!(session_id = %id, "rejected terminal WebSocket: session in terminal state");
            let _ = ws.close().await;
            return;
        }
    }

    let cwd = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&id)
            .map(|h| h.info.cwd.clone())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "/".into())
            })
    };

    let command = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&id)
            .map(|h| h.command.clone())
            .unwrap_or_else(|| default_shell_command(cwd.clone()))
    };

    let pty_sys = make_pty_system();

    // Wait for the frontend's initial resize message so the PTY opens at the
    // actual terminal dimensions, not the 24x80 fallback.  Without this, the
    // spawned command writes its first prompt / banner at 80 columns while the
    // frontend is already displaying at window width, producing choppy text.
    // Non-resize messages that arrive first (e.g. a keypress racing the resize)
    // are saved and replayed into the main loop after the PTY is ready.
    let mut pending_first_msg: Option<String> = None;
    let (initial_rows, initial_cols) = tokio::select! {
        _ = kill_rx.changed() => {
            if *kill_rx.borrow() {
                set_session_status(&state, &id, "killed");
                let _ = ws.close().await;
                return;
            }
            (24u16, 80u16)
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
            (24u16, 80u16)
        }
        msg = ws.recv() => {
            match msg {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if val.get("type").and_then(|v| v.as_str()) == Some("resize") {
                            let r = val.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
                            let c = val.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
                            (r, c)
                        } else {
                            pending_first_msg = Some(text);
                            (24u16, 80u16)
                        }
                    } else {
                        pending_first_msg = Some(text);
                        (24u16, 80u16)
                    }
                }
                Some(Ok(Message::Close(_))) | None => {
                    let _ = ws.close().await;
                    return;
                }
                _ => (24u16, 80u16),
            }
        }
    };

    let pty_size = PtySize {
        rows: initial_rows,
        cols: initial_cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = match pty_sys.openpty(pty_size) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to open PTY");
            set_session_status(&state, &id, "error");
            let _ = ws.close().await;
            return;
        }
    };

    let mut cmd = CommandBuilder::new(&command.program);
    cmd.args(&command.args);
    cmd.cwd(&command.cwd);
    for (key, value) in std::env::vars() {
        if should_forward_terminal_env(&key) {
            cmd.env(&key, &value);
        } else {
            cmd.env_remove(&key);
        }
    }
    for (key, value) in terminal_env_overrides() {
        cmd.env(&key, &value);
    }
    let port = match state.bound_port.load(Ordering::Relaxed) {
        0 => None,
        value => Some(value),
    };
    for (key, value) in session_env_overrides(&id, port) {
        cmd.env(&key, &value);
    }

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to spawn shell");
            set_session_status(&state, &id, "error");
            let _ = ws.close().await;
            return;
        }
    };

    let mut reader = match pair.master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "failed to clone PTY reader");
            set_session_status(&state, &id, "error");
            let _ = ws.close().await;
            return;
        }
    };

    let mut writer = match pair.master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            tracing::error!(error = %e, "failed to take PTY writer");
            set_session_status(&state, &id, "error");
            let _ = ws.close().await;
            return;
        }
    };

    set_session_status(&state, &id, "running");

    // Send initial prompt to the PTY if one was set on session creation
    {
        let initial_prompt = {
            let sessions = state.sessions.lock().unwrap();
            sessions.get(&id).and_then(|h| h.initial_prompt.clone())
        };
        if let Some(prompt) = initial_prompt {
            let prompt_bytes = format!("{}\n", prompt).into_bytes();
            if let Err(e) = writer.write_all(&prompt_bytes) {
                tracing::warn!(session_id = %id, error = %e, "failed to write initial prompt");
            }
        }
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let id_for_reader = id.clone();

    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];
    loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(session_id = %id_for_reader, error = %e, "PTY read error");
                    break;
                }
            }
        }
    });

    // Serial persistence writer: drains lines from an unbounded channel so that
    // append + trim never race and chunks are persisted in arrival order.
    let (persist_tx, mut persist_rx) =
        tokio::sync::mpsc::unbounded_channel::<Vec<String>>();
    let persist_state = state.clone();
    let persist_id = id.clone();
    let persist_writer = tokio::spawn(async move {
        while let Some(lines) = persist_rx.recv().await {
            let st = persist_state.clone();
            let i = persist_id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let ws_guard = st.workspace.lock().unwrap();
                if let Some(ref ws) = *ws_guard {
                    ws.metadata.append_terminal_output_lines(&i, &lines);
                }
            })
            .await;
        }
    });

    // Byte-level buffer of unflushed terminal output. We split on raw '\n' so a
    // chunk that splits a multi-byte UTF-8 sequence or breaks a line in the middle
    // doesn't corrupt persistence: only complete lines are written, the rest
    // stays here until more bytes arrive.
    let mut persist_buffer: Vec<u8> = Vec::new();
    let mut input_buf = String::new();

    if let Some(text) = pending_first_msg {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
            if let TerminalAction::Input(data) = dispatch_terminal_message(&val) {
                let _ = writer.write_all(data.as_bytes());
                let _ = writer.flush();
                if let Some(line) = collect_input_line(&mut input_buf, &data) {
                    let ws_guard = state.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        if let Some(mut meta) = ws.metadata.read_session(&id) {
                            meta.label = line.clone();
                            meta.last_user_input = Some(line.clone());
                            ws.metadata.write_session(&meta);
                        }
                    }
                    drop(ws_guard);
                    let mut sessions = state.sessions.lock().unwrap();
                    if let Some(handle) = sessions.get_mut(&id) {
                        handle.info.label = line.clone();
                        if peon::is_terminal_observed_status(handle.info.observed_status.as_deref()) {
                            handle.info.observed_status = None;
                        }
                    }
                    drop(sessions);
                    // Clear stale idle/done/stale state in metadata when user types.
                    {
                        let ws_guard = state.workspace.lock().unwrap();
                        if let Some(ref ws) = *ws_guard {
                            if let Some(mut meta) = ws.metadata.read_session(&id) {
                                if peon::is_terminal_observed_status(meta.observed_status.as_deref()) {
                                    meta.observed_status = None;
                                    meta.metadata_source = "process".into();
                                    ws.metadata.write_session(&meta);
                                }
                            }
                        }
                    }
                    if state.peon.config.enabled && line.len() > 10 {
                        state.peon.label_hint.write().unwrap().insert(id.clone(), line);
                        state.peon.label_pending.write().unwrap().insert(id.clone());
                    }
                }
                if state.peon.config.enabled && !data.is_empty() {
                    state.peon.last_output.write().unwrap()
                        .insert(id.clone(), tokio::time::Instant::now());
                    state.peon.last_inference.write().unwrap().remove(&id);
                }
            }
        }
    }

    loop {
        tokio::select! {
            _ = kill_rx.changed() => {
                if *kill_rx.borrow() {
                    tracing::info!(session_id = %id, "kill signal received");
                    let _ = child.kill();
                    set_session_status(&state, &id, "killed");
                    break;
                }
            }
            data = rx.recv() => {
                match data {
                    Some(data) => {
                persist_buffer.extend_from_slice(&data);

                let mut raw_persist_lines: Vec<String> = Vec::new();
                while let Some(nl) = persist_buffer.iter().position(|&b| b == b'\n') {
                    let line: Vec<u8> = persist_buffer.drain(..=nl).collect();
                    // Strip the trailing \n (and a preceding \r if present) so persisted
                    // lines are bare content; replay re-adds line terminators.
                    let end = if line.ends_with(b"\r\n") {
                        line.len() - 2
                    } else {
                        line.len() - 1
                    };
                    raw_persist_lines.push(String::from_utf8_lossy(&line[..end]).into_owned());
                }

                if state.peon.config.enabled {
                    let mut sessions = state.sessions.lock().unwrap();
                    if let Some(handle) = sessions.get_mut(&id) {
                        for raw in &raw_persist_lines {
                            let trimmed = raw.trim();
                            if !trimmed.is_empty() {
                                handle.output_buffer.push(trimmed.to_string());
                            }
                        }
                        if peon::is_terminal_observed_status(handle.info.observed_status.as_deref()) {
                            handle.info.observed_status = None;
                        }
                    }
                }

                // Any PTY traffic at all counts as activity for Peon debounce —
                // including chunks that haven't yet completed a line.
                if state.peon.config.enabled {
                    state.peon.last_output.write().unwrap()
                        .insert(id.clone(), tokio::time::Instant::now());
                    state.peon.last_inference.write().unwrap().remove(&id);
                }

                // New terminal output means the session is no longer idle.
                // Clear any stale terminal observed status in metadata.
                {
                    let ws_guard = state.workspace.lock().unwrap();
                    if let Some(ref ws) = *ws_guard {
                        if let Some(mut meta) = ws.metadata.read_session(&id) {
                            if peon::is_terminal_observed_status(meta.observed_status.as_deref()) {
                                meta.observed_status = None;
                                meta.metadata_source = "process".into();
                                ws.metadata.write_session(&meta);
                            }
                        }
                    }
                }

                if !raw_persist_lines.is_empty() {
                    let _ = persist_tx.send(raw_persist_lines);
                }

                if ws.send(Message::Binary(data)).await.is_err() {
                    break;
                }
                    }
                    None => {
                        // PTY reader channel closed: child process exited (e.g. user typed "exit").
                        // Reap the child and clean up so the frontend's WebSocket onclose fires.
                        let _ = child.kill();
                        set_session_status(&state, &id, "ended");
                        break;
                    }
                }
            }
            msg = ws.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let val: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        match dispatch_terminal_message(&val) {
                            TerminalAction::Input(data) => {
                                let _ = writer.write_all(data.as_bytes());
                                let _ = writer.flush();

                                let mut triggered_label = false;
                                if let Some(line) = collect_input_line(&mut input_buf, &data) {
                                    let ws_guard = state.workspace.lock().unwrap();
                                    if let Some(ref ws) = *ws_guard {
                                        if let Some(mut meta) = ws.metadata.read_session(&id) {
                                            meta.label = line.clone();
                                            meta.last_user_input = Some(line.clone());
                                            ws.metadata.write_session(&meta);
                                        }
                                    }
                                    drop(ws_guard);
                                    let mut sessions = state.sessions.lock().unwrap();
                                    if let Some(handle) = sessions.get_mut(&id) {
                                        handle.info.label = line.clone();
                                    }
                                    drop(sessions);
                                    if state.peon.config.enabled && line.len() > 10 {
                                        state.peon.label_hint.write().unwrap().insert(id.clone(), line);
                                        state.peon.label_pending.write().unwrap().insert(id.clone());
                                        triggered_label = true;
                                    }
                                }

                                if state.peon.config.enabled && !data.is_empty() {
                                    let ts = if triggered_label {
                                        tokio::time::Instant::now() - std::time::Duration::from_secs(3600)
                                    } else {
                                        tokio::time::Instant::now()
                                    };
                                    state.peon.last_output.write().unwrap().insert(id.clone(), ts);
                                    state.peon.last_inference.write().unwrap().remove(&id);
                                }
                            }
                            TerminalAction::Resize { rows, cols } => {
                                if let Err(e) = pair.master.resize(PtySize {
                                    rows,
                                    cols,
                                    pixel_width: 0,
                                    pixel_height: 0,
                                }) {
                                    tracing::warn!(error = %e, "PTY resize error");
                                }
                            }
                            TerminalAction::Kill => {
                                tracing::info!(session_id = %id, "kill message received");
                                let _ = child.kill();
                                set_session_status(&state, &id, "killed");
                                break;
                            }
                            TerminalAction::Noop => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        let _ = child.kill();
                        if *kill_rx.borrow() {
                            set_session_status(&state, &id, "killed");
                        } else {
                            set_session_status(&state, &id, "ended");
                        }
                        break;
                    }
                    _ => {
                        let _ = child.kill();
                        set_session_status(&state, &id, "error");
                        break;
                    }
                }
            }
        }
    }

    {
        // Flush any unterminated tail so the user's last visible line survives.
        if !persist_buffer.is_empty() {
            let tail = String::from_utf8_lossy(&persist_buffer).into_owned();
            let _ = persist_tx.send(vec![tail]);
        }
        // Close the channel and let the serial writer drain all pending appends
        // before the trim runs, so trimming never races with a write.
        drop(persist_tx);
        let _ = persist_writer.await;

        let state_clone = state.clone();
        let id_clone = id.clone();
        tokio::task::spawn_blocking(move || {
            let ws_guard = state_clone.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                ws.metadata.trim_terminal_output(&id_clone, metadata::TERMINAL_OUTPUT_MAX_LINES);
            }
        });
    }

    tracing::info!(session_id = %id, "session terminal ended");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_env_overrides_force_color_capability() {
        let overrides = terminal_env_overrides();

        assert!(overrides.contains(&("TERM".into(), "xterm-256color".into())));
        assert!(overrides.contains(&("COLORTERM".into(), "truecolor".into())));
        assert!(overrides.contains(&("FORCE_COLOR".into(), "1".into())));
        assert!(overrides.contains(&("CLICOLOR".into(), "1".into())));
        assert!(overrides.contains(&("TERM_PROGRAM".into(), "OrkWorks".into())));
    }

    #[test]
    fn session_env_overrides_include_orkworks_session_and_port() {
        let overrides = session_env_overrides("session-123", Some(5173));
        assert!(overrides.contains(&("ORKWORKS_SESSION_ID".into(), "session-123".into())));
        assert!(overrides.contains(&("ORKWORKS_PORT".into(), "5173".into())));
    }

    #[test]
    fn session_env_overrides_omit_port_when_unknown() {
        let overrides = session_env_overrides("session-123", None);
        assert!(overrides.contains(&("ORKWORKS_SESSION_ID".into(), "session-123".into())));
        assert!(!overrides.iter().any(|(key, _)| key == "ORKWORKS_PORT"));
    }

    #[test]
    fn opencode_reporter_script_posts_native_session_env() {
        let script = include_str!("../scripts/report-opencode-session.sh");
        assert!(script.contains("OPENCODE_SESSION_ID"));
        assert!(script.contains("ORKWORKS_SESSION_ID"));
        assert!(script.contains("ORKWORKS_PORT"));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/harness-session"));
        assert!(script.contains("\"source\":\"opencode_env\""));
    }

    #[test]
    fn codex_jsonl_parser_extracts_thread_started_id() {
        let line = r#"{"type":"thread.started","thread_id":"0199a213-81c0-7800-8aa1-bbab2a035a53"}"#;
        assert_eq!(
            codex_thread_id_from_jsonl_line(line).as_deref(),
            Some("0199a213-81c0-7800-8aa1-bbab2a035a53"),
        );
    }

    #[test]
    fn codex_jsonl_parser_ignores_other_events() {
        let line = r#"{"type":"turn.started"}"#;
        assert_eq!(codex_thread_id_from_jsonl_line(line), None);
    }

    #[test]
    fn claude_hook_reporter_extracts_session_id_and_posts() {
        let script = include_str!("../scripts/report-claude-session-from-hook.sh");
        assert!(script.contains("session_id"));
        assert!(script.contains("ORKWORKS_SESSION_ID"));
        assert!(script.contains("ORKWORKS_PORT"));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/harness-session"));
        assert!(script.contains("\"source\":\"claude_hook\""));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/attention"));
    }

    #[test]
    fn terminal_env_filter_removes_launcher_debug_variables() {
        assert!(!should_forward_terminal_env("NODE_OPTIONS"));
        assert!(!should_forward_terminal_env("VSCODE_INSPECTOR_OPTIONS"));
        assert!(!should_forward_terminal_env("VSCODE_PID"));
        assert!(!should_forward_terminal_env("ELECTRON_RUN_AS_NODE"));
    }

    #[test]
    fn terminal_env_filter_keeps_normal_shell_variables() {
        assert!(should_forward_terminal_env("PATH"));
        assert!(should_forward_terminal_env("HOME"));
        assert!(should_forward_terminal_env("SHELL"));
        assert!(should_forward_terminal_env("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn terminal_message_dispatches_kill() {
        let msg = serde_json::json!({"type": "kill"});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Kill);
    }

    #[test]
    fn terminal_message_dispatches_input() {
        let msg = serde_json::json!({"type": "input", "data": "hello"});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Input("hello".into()));
    }

    #[test]
    fn terminal_message_dispatches_resize() {
        let msg = serde_json::json!({"type": "resize", "rows": 40, "cols": 120});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Resize { rows: 40, cols: 120 });
    }

    #[test]
    fn terminal_message_dispatches_unknown_as_noop() {
        let msg = serde_json::json!({"type": "unknown"});
        let action = dispatch_terminal_message(&msg);
        assert_eq!(action, TerminalAction::Noop);
    }

    fn test_app_state_with_workspace(path: &std::path::Path) -> Arc<AppState> {
        let metadata_root = path.join(".orkworks-test");
        Arc::new(AppState {
            session_module: SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(WorkspaceState {
                path: path.to_path_buf(),
                metadata: metadata::MetadataStore::new(&metadata_root),
                watcher: watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
            })),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::new(),
        })
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
                phase: "".into(),
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
                    id: session_id.clone(),
                    label: "Known".into(),
                    harness_id: Some("opencode".into()),
                    model_provider_id: None,
                    model_id: None,
                    harness: Some("opencode".into()),
                    model: None,
                    status: "running".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "before".into(),
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
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    provider: None,
                    provider_model: None,
                    provider_state: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::LatestCwd,
                    resume: Some(resume.clone()),
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
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
                phase: "".into(),
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

    #[test]
    fn session_registry_create_and_list() {
        let state = Arc::new(AppState {
            session_module: SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::new(),
        });

        assert!(state.sessions.lock().unwrap().is_empty());

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "test-1".to_string();
        let info = SessionInfo {
            id: id.clone(),
            label: "Test".into(),
            harness_id: None,
            model_provider_id: None,
            model_id: None,
            harness: None,
            model: None,
            status: "creating".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
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
            metadata_source: None,
            metadata_confidence: None,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        };

        state
            .sessions
            .lock()
            .unwrap()
            .insert(id, SessionHandle { info: info.clone(), kill_tx, output_buffer: peon::RingBuffer::new(200), command: default_shell_command("/tmp".into()), initial_prompt: None });

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        let stored = sessions.get("test-1").unwrap();
        assert_eq!(stored.info.label, "Test");
        assert_eq!(stored.info.status, "creating");
    }

    #[tokio::test]
    async fn list_sessions_does_not_duplicate_killed_sessions_with_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        let state = Arc::new(AppState {
            session_module: SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::new(),
        });

        let session_id = "killed-with-metadata".to_string();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionHandle {
                info: SessionInfo {
                    id: session_id.clone(),
                    label: "Killed".into(),
                    harness_id: None,
                    model_provider_id: None,
                    model_id: None,
                    harness: None,
                    model: None,
                    status: "killed".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "2026-06-25T10:00:00Z".into(),
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
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    provider: None,
                    provider_model: None,
                    provider_state: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
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
                phase: "".into(),
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
        let matching = sessions
            .iter()
            .filter(|session| session.get("id").and_then(|id| id.as_str()) == Some(session_id.as_str()))
            .count();

        assert_eq!(matching, 1);
    }

    #[test]
    fn set_session_status_updates_registry() {
        let state = Arc::new(AppState {
            session_module: SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::new(),
        });

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "test-2".to_string();
        state.sessions.lock().unwrap().insert(
            id.clone(),
            SessionHandle {
                info: SessionInfo {
                    id: id.clone(),
                    label: "Test".into(),
                    harness_id: None,
                    model_provider_id: None,
                    model_id: None,
                    harness: None,
                    model: None,
                    status: "creating".into(),
                    cwd: "/tmp".into(),
                    created_at: "now".into(),
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
                    metadata_source: None,
                    metadata_confidence: None,
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    provider: None,
                    provider_model: None,
                    provider_state: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
                initial_prompt: None,
            },
        );

        set_session_status(&state, "test-2", "running");
        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get("test-2")
                .unwrap()
                .info
                .status,
            "running"
        );

        set_session_status(&state, "test-2", "ended");
        assert_eq!(
            state
                .sessions
                .lock()
                .unwrap()
                .get("test-2")
                .unwrap()
                .info
                .status,
            "ended"
        );
    }

    #[test]
    fn kill_signal_detected_by_subscriber() {
        let (kill_tx, _rx) = tokio::sync::watch::channel(false);

        let _ = kill_tx.send(true);

        // subscribe after send — should see current value as true
        let rx = kill_tx.subscribe();
        assert!(*rx.borrow());
    }

    #[test]
    fn kill_signal_not_seen_when_false() {
        let (kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        drop(kill_rx);

        let rx = kill_tx.subscribe();
        assert!(!*rx.borrow());
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
    fn resolve_session_launch_preserves_selected_harness_id_for_generic_shell_configs() {
        let harnesses = builtin_harness_configs();
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
        assert_eq!(launch.adapter_harness_id.as_deref(), Some("generic-shell"));
        assert_eq!(launch.command.program, "codex");
    }

    #[test]
    fn resolve_session_launch_does_not_infer_model_provider_from_harness() {
        let harnesses = builtin_harness_configs();
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

    #[test]
    fn session_info_serializes_provider_fields() {
        let info = SessionInfo {
            id: "test".into(),
            label: "Test".into(),
            harness_id: Some("codex".into()),
            model_provider_id: Some("openrouter".into()),
            model_id: Some("gpt-5".into()),
            harness: None,
            model: None,
            status: "running".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
            observed_status: Some("waiting_for_input".into()),
            summary: Some("Needs approval".into()),
            next_action: Some("Choose an option".into()),
            needs_user_input: Some(true),
            detected_question: Some("Proceed?".into()),
            suggested_options: Some(vec!["yes".into(), "no".into()]),
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            metadata_source: Some("process".into()),
            metadata_confidence: Some(1.0),
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            provider: Some("Claude Code".into()),
            provider_model: Some("sonnet".into()),
            provider_state: Some("healthy".into()),
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"harnessId\":\"codex\""));
        assert!(json.contains("\"modelProviderId\":\"openrouter\""));
        assert!(json.contains("\"modelId\":\"gpt-5\""));
        assert!(json.contains("\"provider\":\"Claude Code\""));
        assert!(json.contains("\"providerModel\":\"sonnet\""));
        assert!(json.contains("\"providerState\":\"healthy\""));
    }

    #[test]
    fn session_info_includes_metadata_fields() {
        let info = SessionInfo {
            id: "test".into(),
            label: "Test".into(),
            harness_id: None,
            model_provider_id: None,
            model_id: None,
            harness: None,
            model: None,
            status: "running".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
            observed_status: Some("waiting_for_input".into()),
            summary: Some("Needs approval".into()),
            next_action: Some("Choose an option".into()),
            needs_user_input: Some(true),
            detected_question: Some("Proceed?".into()),
            suggested_options: Some(vec!["yes".into(), "no".into()]),
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            metadata_source: Some("process".into()),
            metadata_confidence: Some(1.0),
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"metadataSource\":\"process\""));
        assert!(json.contains("\"metadataConfidence\":1.0"));
        assert!(json.contains("\"observedStatus\":\"waiting_for_input\""));
        assert!(json.contains("\"needsUserInput\":true"));
    }

    #[test]
    fn session_info_without_metadata_is_valid() {
        let info = SessionInfo {
            id: "test".into(),
            label: "Test".into(),
            harness_id: None,
            model_provider_id: None,
            model_id: None,
            harness: None,
            model: None,
            status: "creating".into(),
            cwd: "/tmp".into(),
            created_at: "now".into(),
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
            metadata_source: None,
            metadata_confidence: None,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"metadataSource\":null"));
        assert!(json.contains("\"metadataConfidence\":null"));
    }

    #[test]
    fn detect_conflicts_warns_on_multiple_dirty_sessions() {
        let sessions = vec![
            SessionInfo {
                id: "a".into(), label: "A".into(), harness_id: None, model_provider_id: None, model_id: None, harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(true),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
            SessionInfo {
                id: "b".into(), label: "B".into(), harness_id: None, model_provider_id: None, model_id: None, harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(true),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
        ];
        let warnings = detect_conflicts(&sessions);
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|(id, _)| id == "a"));
        assert!(warnings.iter().any(|(id, _)| id == "b"));
        for (_, w) in &warnings {
            assert!(w.contains("2 sessions"));
        }
    }

    #[test]
    fn detect_conflicts_no_warning_on_clean_sessions() {
        let sessions = vec![
            SessionInfo {
                id: "a".into(), label: "A".into(), harness_id: None, model_provider_id: None, model_id: None, harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(false),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
            SessionInfo {
                id: "b".into(), label: "B".into(), harness_id: None, model_provider_id: None, model_id: None, harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: Some(false),
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
        ];
        let warnings = detect_conflicts(&sessions);
        assert!(warnings.is_empty());
    }

    #[test]
    fn detect_conflicts_no_warning_when_dirty_is_none() {
        let sessions = vec![
            SessionInfo {
                id: "a".into(), label: "A".into(), harness_id: None, model_provider_id: None, model_id: None, harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: None,
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
            SessionInfo {
                id: "b".into(), label: "B".into(), harness_id: None, model_provider_id: None, model_id: None, harness: None, model: None, status: "running".into(),
                cwd: "/repo".into(), created_at: "now".into(),
                observed_status: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None,
                metadata_source: None, metadata_confidence: None,
                repo_root: None, branch: None,
                dirty: None,
                changed_files: None, is_worktree: None,
                conflict_warning: None, recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resumed_from: None,
        },
        ];
        let warnings = detect_conflicts(&sessions);
        assert!(warnings.is_empty());
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
            std::fs::set_permissions(&harness_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let state = Arc::new(AppState {
            session_module: SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
            let mut handle = SessionHandle {
                info: SessionInfo {
                    id: session_id.clone(),
                    label: "Test".into(),
                    harness_id: None,
                    model_provider_id: None,
                    model_id: None,
                    harness: None,
                    model: None,
                    status: "running".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "now".into(),
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
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    provider: None,
                    provider_model: None,
                    provider_state: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
                initial_prompt: None,
            };
            handle.output_buffer.push("running cargo test...".into());
            handle.output_buffer.push("test result: ok. 5 passed; 0 failed;".into());
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
                    phase: "".into(),
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
                        assert_eq!(meta.summary, Some("test".into()));
                        assert_eq!(meta.peon_last_inference.is_some(), true);
                        assert_eq!(meta.metadata_source, "peon");
                        assert!((meta.metadata_confidence - 0.85).abs() < 0.001);
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

        let state = Arc::new(AppState {
            session_module: SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig::from_env(),
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
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
            let mut handle = SessionHandle {
                info: SessionInfo {
                    id: session_id.clone(),
                    label: "Test".into(),
                    harness_id: None,
                    model_provider_id: None,
                    model_id: None,
                    harness: None,
                    model: None,
                    status: "running".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "now".into(),
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
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    provider: None,
                    provider_model: None,
                    provider_state: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: harness::CommandSpec { program: "/bin/sh".into(), args: vec!["-i".into(), "-l".into()], cwd: "/tmp".into() },
                initial_prompt: None,
            };
            handle.output_buffer.push("quiet output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
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
    async fn peon_loop_records_failed_inference_attempt() {
        let dir = tempfile::tempdir().unwrap();

        let state = Arc::new(AppState {
            session_module: SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 15,
                    enabled: true,
                },
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::new(),
        });

        let session_id = "peon-failed-attempt-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = SessionHandle {
                info: SessionInfo {
                    id: session_id.clone(),
                    label: "Test".into(),
                    harness_id: None,
                    model_provider_id: None,
                    model_id: None,
                    harness: None,
                    model: None,
                    status: "running".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "now".into(),
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
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    provider: None,
                    provider_model: None,
                    provider_state: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
            };
            handle.output_buffer.push("quiet output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
        }

        state.peon.last_output.write().unwrap().insert(
            session_id.clone(),
            tokio::time::Instant::now() - std::time::Duration::from_secs(5),
        );

        let task = tokio::spawn(peon_loop(state.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        task.abort();

        assert!(
            state.peon.last_inference.read().unwrap().contains_key(&session_id),
            "failed Peon attempts should still be recorded to avoid tight retry loops"
        );
    }

    #[tokio::test]
    async fn peon_loop_marks_idle_when_silent() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(AppState {
            session_module: SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1, // fast idle detection for test
                    enabled: true,
                },
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            providers: providers::ProviderManager::new(),
        });

        let session_id = "peon-idle-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = SessionHandle {
                info: SessionInfo {
                    id: session_id.clone(),
                    label: "Test".into(),
                    harness_id: None,
                    model_provider_id: None,
                    model_id: None,
                    harness: None,
                    model: None,
                    status: "running".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "now".into(),
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
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    provider: None,
                    provider_model: None,
                    provider_state: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
            };
            handle.output_buffer.push("some past output".into());
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
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
                    phase: "".into(),
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
    async fn peon_loop_does_not_overwrite_existing_observed_status_with_idle() {
        let dir = tempfile::tempdir().unwrap();
        let orkworks = dir.path().join(".orkworks");
        std::fs::create_dir_all(orkworks.join("sessions")).unwrap();
        std::fs::create_dir_all(orkworks.join("events")).unwrap();

        let state = Arc::new(AppState {
            session_module: SessionModule::new(),
            sessions: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(WorkspaceState {
                path: dir.path().to_path_buf(),
                metadata: metadata::MetadataStore::new(&orkworks),
                watcher: watcher::MetadataWatcher::start(&orkworks.join("sessions")),
            })),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                config: peon::PeonConfig {
                    harness: dir.path().join("missing-harness").display().to_string(),
                    harness_args: vec!["--print".into()],
                    model: None,
                    interval_secs: 1,
                    max_lines: 200,
                    timeout_secs: 10,
                    idle_timeout_secs: 1,
                    enabled: true,
                },
            },
            adapters: builtin_adapters(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            harnesses: tokio::sync::RwLock::new(vec![]),
            providers: providers::ProviderManager::new(),
        });

        let session_id = "peon-idle-no-overwrite-test".to_string();
        {
            let (kill_tx, _) = tokio::sync::watch::channel(false);
            let mut handle = SessionHandle {
                info: SessionInfo {
                    id: session_id.clone(),
                    label: "Test".into(),
                    harness_id: None,
                    model_provider_id: None,
                    model_id: None,
                    harness: None,
                    model: None,
                    status: "running".into(),
                    cwd: dir.path().display().to_string(),
                    created_at: "now".into(),
                    observed_status: Some("blocked".into()),
                    summary: None,
                    next_action: None,
                    needs_user_input: None,
                    detected_question: None,
                    suggested_options: None,
                    blocker_description: None,
                    failed_command: None,
                    failed_test: None,
                    capacity_hints: None,
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    repo_root: None,
                    branch: None,
                    dirty: None,
                    changed_files: None,
                    is_worktree: None,
                    conflict_warning: None,
                    recommendation: None,
                    peon_last_inference: None,
                    provider: None,
                    provider_model: None,
                    provider_state: None,
                    memory_state: MemoryState::Live,
                    resume_strategy: harness::ResumeStrategy::None,
                    resume: None,
                    resumed_from: None,
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                command: default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
            };
            state.sessions.lock().unwrap().insert(session_id.clone(), handle);
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
                    phase: "".into(),
                    observed_status: Some("blocked".into()),
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

    #[test]
    fn memory_state_marks_absent_session_as_resumable_when_strategy_exists() {
        let caps = harness::HarnessCapabilities {
            launch: true,
            resume_exact: true,
            resume_latest_in_cwd: true,
            resume_latest_in_repo: false,
            detect_session_id: true,
            detect_model: true,
            detect_context_usage: false,
            detect_capacity: false,
            native_voice: false,
        };
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::Exact,
            harness_session_id: Some("sess-1".into()),
            latest_fallback: true,
            last_seen_at: None,
        };

        let (memory_state, strategy) = derive_memory_state(false, Some(&resume), &caps);

        assert_eq!(memory_state, MemoryState::Resumable);
        assert_eq!(strategy, harness::ResumeStrategy::Exact);
    }

    #[test]
    fn memory_state_marks_active_session_as_live() {
        let caps = harness::HarnessCapabilities {
            launch: true,
            resume_exact: false,
            resume_latest_in_cwd: false,
            resume_latest_in_repo: false,
            detect_session_id: false,
            detect_model: false,
            detect_context_usage: false,
            detect_capacity: false,
            native_voice: false,
        };

        let (memory_state, strategy) = derive_memory_state(true, None, &caps);

        assert_eq!(memory_state, MemoryState::Live);
        assert_eq!(strategy, harness::ResumeStrategy::None);
    }

    #[test]
    fn generic_shell_memory_state_is_not_resumable() {
        let capabilities = default_capabilities();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::Exact,
            harness_session_id: Some("sess-1".into()),
            latest_fallback: true,
            last_seen_at: None,
        };

        let (memory_state, strategy) = derive_memory_state(false, Some(&resume), &capabilities);
        let command = builtin_adapters().get("generic-shell").unwrap().build_resume_command(&harness::ResumeRequest {
            strategy: harness::ResumeStrategy::Exact,
            cwd: "/tmp".into(),
            repo_root: Some("/tmp".into()),
            harness_session_id: Some("sess-1".into()),
            model: None,
        });

        assert_eq!(memory_state, MemoryState::Unsupported);
        assert_eq!(strategy, harness::ResumeStrategy::None);
        assert!(command.is_none());
    }

}
