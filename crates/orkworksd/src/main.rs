use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, RwLock as StdRwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod git;
mod harness;
mod http;
mod metadata;
mod migration;
mod peon;
mod plan_handoff;
mod procfs;
mod providers;
mod runtime;
mod session_application;
mod session_projection;
mod session_types;
mod session_view;
mod taskmaster;
mod watcher;
mod workflow_observations;
mod workspace_runtime;

use crate::harness::definition::{BuiltinDocument, EMBEDDED_BUILTINS};
use crate::harness::probe_cache::VersionProbeCache;
use crate::harness::registry::HarnessCatalog;
use crate::harness::store::{global_harnesses_path, HarnessStore};
use crate::http::harness_handlers::{
    create_harness, delete_harness, duplicate_harness, list_harnesses, remove_harness_profile,
    update_harness,
};
use crate::http::integration_handlers::{
    get_grouped_integration_status, get_integration_status, get_workspace_integrations,
    install_grouped_integration, install_integration, repair_grouped_integration,
    uninstall_grouped_integration, uninstall_integration,
};
use crate::http::provider_handlers::{
    discover_provider_models, get_applied_peon_provider, get_providers, set_provider_settings,
    test_and_apply_peon_provider, verify_ollama_settings, verify_peon_provider,
};
use crate::http::retention_handlers::set_retention;
use crate::http::session_handlers::{
    apply_debug_attention, create_session, delete_session, forget_session,
    get_session_plan_content, list_sessions, report_attention, report_harness_session,
    report_session_plan_path, request_session_plan_review, resume_session, select_terminal_plan,
    set_active_harnesses, set_active_session, set_workspace,
};
use crate::http::taskmaster_handlers::{
    dismiss_recommendation, get_recommendation, list_recommendations,
};
use crate::http::workflow_observation_handlers::report_workflow_observation;
use crate::runtime::peon_runtime::peon_loop;
use crate::runtime::retention::retention_cleanup_task;
use crate::runtime::terminal_http::{
    get_summary_log, get_terminal_output, session_terminal_handler,
};
use crate::session_types::{PeonDiagnostics, PeonSchedulerState, SessionInfo};

struct SessionHandle {
    info: SessionInfo,
    active_work_hook: bool,
    kill_tx: tokio::sync::watch::Sender<bool>,
    output_buffer: peon::RingBuffer,
    // Rolling raw PTY text (ANSI-stripped) for TUI apps that use cursor positioning instead of newlines.
    scan_buf: String,
    pending_work_signal: Option<runtime::session_runtime::PendingWorkSignal>,
    runtime: runtime::session_runtime::SessionRuntime,
    terminal_attached: bool,
    // Runtime-only ownership claim installed atomically before a resumed PTY starts.
    // It remains set for the live runtime and is cleared by terminal finalization.
    resume_in_progress: bool,
    // Sticky: once usage limit is detected it stays true until the session is killed/resumed.
    at_usage_limit_latched: bool,
    capacity_check_pending: bool,
    output_lines_seen: u64,
    scan_bytes_seen: u64,
    // Snapshot origin used for one-shot post-resume / post-input fresh-output checks.
    resume_scan_origin: Option<(u64, u64)>,
    pending_capacity_visible_once: bool,
}

struct WorkspaceState {
    path: PathBuf,
    metadata: metadata::MetadataStore,
    workflow_observations: workflow_observations::WorkflowObservationStore,
    recommendation_store: taskmaster::store::RecommendationStore,
    #[allow(dead_code)]
    watcher: watcher::MetadataWatcher,
}

/// A queued `InputLabel` refinement request: the input line Peon should turn
/// into a topic, tagged with the session's label epoch at the moment it was
/// queued. A harness-declared reset command bumps that epoch (ADR 0040), so a
/// refinement from the previous conversation can be recognized as stale. The
/// initial prompt marker prevents a late refinement of an inherited startup
/// prompt from restoring a label after the first real terminal prompt wins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LabelHint {
    text: String,
    epoch: u64,
    from_initial_prompt: bool,
}

struct PeonState {
    last_output: StdRwLock<HashMap<String, tokio::time::Instant>>,
    last_inference: StdRwLock<HashMap<String, String>>,
    in_flight: StdRwLock<HashSet<String>>,
    diagnostics: StdRwLock<HashMap<String, PeonDiagnosticEntry>>,
    label_hint: StdRwLock<HashMap<String, LabelHint>>,
    label_pending: StdRwLock<HashSet<String>>,
    // Per-session label generation. Incremented by a harness-declared label
    // reset; queued and in-flight label work carries the epoch it was created
    // under so a reset can invalidate it.
    label_epochs: StdRwLock<HashMap<String, u64>>,
    // Pending (not yet newline-terminated) terminal input line per session, used
    // to detect a descriptive label. Cleared on an attention-hook report, since
    // that signals a harness turn boundary — without it, isolated hotkey
    // keystrokes (e.g. single-key "accept" prompts) from unrelated turns would
    // otherwise glue together into one garbled label the next time a real line
    // is submitted.
    input_buf: StdRwLock<HashMap<String, String>>,
    // The harness's own logical cwd, when it reports one via its hook (issue
    // #241 / ADR 0032) — authoritative over the pid-probed/launch-time cwd
    // fallbacks. Currently only populated for Claude Code sessions.
    reported_cwd: StdRwLock<HashMap<String, String>>,
    config: peon::PeonConfig,
}

const MAX_PEON_DIAGNOSTIC_SESSIONS: usize = 1_024;

struct PeonDiagnosticEntry {
    snapshot: PeonDiagnostics,
    attempt_generation: u64,
    runtime_identity: Option<crate::runtime::session_runtime::RuntimeIdentity>,
}

impl PeonDiagnosticEntry {
    fn new() -> Self {
        Self {
            snapshot: PeonDiagnostics {
                scheduler_state: PeonSchedulerState::Idle,
                reason: None,
                last_attempt_at: None,
                last_successful_inference_at: None,
                provider_id: None,
                provider_model: None,
                fallback_step: None,
                attempt_count: None,
                error_summary: None,
                observation_count: None,
            },
            attempt_generation: 0,
            runtime_identity: None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct RetentionConfig {
    #[serde(rename = "maxSessions", default)]
    max_sessions: usize,
    #[serde(rename = "maxAgeDays", default)]
    max_age_days: u32,
}

struct AppState {
    sessions: Mutex<HashMap<String, SessionHandle>>,
    // Coordinates complete session projections with workspace replacement.
    projection_lock: Mutex<()>,
    // OS pid of each session's PTY child, captured at spawn. Used to probe
    // the process's live cwd (issue #241) instead of trusting the frozen
    // launch-time cwd forever.
    session_pids: Mutex<HashMap<String, u32>>,
    workspace: Mutex<Option<WorkspaceState>>,
    peon: PeonState,
    providers: providers::ProviderManager,
    harness_catalog: HarnessCatalog,
    harness_store: Arc<HarnessStore>,
    integration_probe_cache: VersionProbeCache,
    retention_config: tokio::sync::RwLock<RetentionConfig>,
    bound_port: AtomicU16,
}

impl AppState {
    fn bump_harness_probe_generation(&self) {
        self.integration_probe_cache.bump_generation();
    }
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

    let (harness_catalog, harness_store, providers) = {
        let builtins =
            Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).expect("embedded harnesses parse"));
        let harness_store = Arc::new(HarnessStore::new(global_harnesses_path(), builtins));
        let loaded_harnesses = harness_store
            .load()
            .expect("harness configuration must load");
        if loaded_harnesses.migrated_from_v1 {
            tracing::info!("migrated legacy harness configuration to version 2");
        }
        let harness_catalog: HarnessCatalog = Arc::new(StdRwLock::new(loaded_harnesses.registry));
        let providers = providers::ProviderManager::new_with_catalog(harness_catalog.clone());
        (harness_catalog, harness_store, providers)
    };

    let state = Arc::new(AppState {
        sessions: Mutex::new(HashMap::new()),
        projection_lock: Mutex::new(()),
        session_pids: Mutex::new(HashMap::new()),
        workspace: Mutex::new(None),
        peon: PeonState {
            last_output: StdRwLock::new(HashMap::new()),
            last_inference: StdRwLock::new(HashMap::new()),
            in_flight: StdRwLock::new(HashSet::new()),
            label_hint: StdRwLock::new(HashMap::new()),
            label_pending: StdRwLock::new(HashSet::new()),
            label_epochs: StdRwLock::new(HashMap::new()),
            input_buf: StdRwLock::new(HashMap::new()),
            reported_cwd: StdRwLock::new(HashMap::new()),
            diagnostics: StdRwLock::new(HashMap::new()),
            config: peon::PeonConfig::from_env(),
        },
        providers,
        harness_catalog,
        harness_store,
        integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
        retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
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

    let app = build_router(state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let bound_addr = listener.local_addr().unwrap();
    state.bound_port.store(bound_addr.port(), Ordering::Relaxed);

    println!("ORKWORKSD_PORT={}", bound_addr.port());

    tracing::info!(addr = %bound_addr, "orkworksd listening");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "ok"
}

/// Single source of truth for the HTTP router. Both the runtime `main`
/// listener and the test fixtures build their `Router` through this so
/// route drift between production and tests is structurally impossible.
pub(crate) fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/providers", get(get_providers))
        .route("/settings/providers", post(set_provider_settings))
        .route(
            "/settings/providers/ollama/verify",
            post(verify_ollama_settings),
        )
        .route(
            "/settings/providers/:provider_id/models",
            post(discover_provider_models),
        )
        .route("/settings/peon/provider/verify", post(verify_peon_provider))
        .route(
            "/settings/peon/test-and-apply",
            post(test_and_apply_peon_provider),
        )
        .route("/settings/peon/applied", get(get_applied_peon_provider))
        .route("/workspace", post(set_workspace))
        .route("/workspace/active-session", post(set_active_session))
        .route("/workspace/active-harnesses", put(set_active_harnesses))
        .route(
            "/workspace/integrations/:harness_id/status",
            get(get_integration_status),
        )
        .route(
            "/workspace/integrations/:harness_id/install",
            post(install_integration),
        )
        .route(
            "/workspace/integrations/:harness_id/uninstall",
            post(uninstall_integration),
        )
        .route("/workspace/integrations", get(get_workspace_integrations))
        .route(
            "/workspace/integrations/:adapter_id/:target_id/status",
            get(get_grouped_integration_status),
        )
        .route(
            "/workspace/integrations/:adapter_id/:target_id/install",
            post(install_grouped_integration),
        )
        .route(
            "/workspace/integrations/:adapter_id/:target_id/repair",
            post(repair_grouped_integration),
        )
        .route(
            "/workspace/integrations/:adapter_id/:target_id/uninstall",
            post(uninstall_grouped_integration),
        )
        .route("/sessions", post(create_session))
        .route("/sessions", get(list_sessions))
        .route("/sessions/:id", delete(delete_session))
        .route("/sessions/:id/forget", delete(forget_session))
        .route("/sessions/:id/resume", post(resume_session))
        .route(
            "/sessions/:id/harness-session",
            post(report_harness_session),
        )
        .route("/sessions/:id/attention", post(report_attention))
        .route("/sessions/:id/plan-path", post(report_session_plan_path))
        .route(
            "/sessions/:id/select-terminal-plan",
            post(select_terminal_plan),
        )
        .route("/sessions/:id/debug-injection", post(apply_debug_attention))
        .route("/sessions/:id/plan-content", get(get_session_plan_content))
        .route(
            "/sessions/:id/request-plan-review",
            post(request_session_plan_review),
        )
        .route("/settings/retention", post(set_retention))
        .route("/taskmaster/recommendations", get(list_recommendations))
        .route("/taskmaster/recommendations/:id", get(get_recommendation))
        .route(
            "/taskmaster/recommendations/:id/dismiss",
            post(dismiss_recommendation),
        )
        .route("/harnesses", get(list_harnesses).post(create_harness))
        .route("/harnesses/:source_id/duplicate", post(duplicate_harness))
        .route(
            "/harnesses/:id/remove-profile",
            post(remove_harness_profile),
        )
        .route("/harnesses/:id", put(update_harness).delete(delete_harness))
        .route("/sessions/:id/terminal", get(session_terminal_handler))
        .route("/sessions/:id/terminal-output", get(get_terminal_output))
        .route("/sessions/:id/summary-log", get(get_summary_log))
        .merge(
            // Isolated so the 8 KiB body cap (ADR 0042) applies only to this
            // route, not the rest of the API.
            Router::new()
                .route(
                    "/sessions/:id/workflow-observations",
                    post(report_workflow_observation),
                )
                .layer(DefaultBodyLimit::max(8 * 1024)),
        )
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
#[test]
fn session_metadata_serializes_connectivity_terminal_outcome_and_last_activity() {
    metadata::assert_session_metadata_serializes_connectivity_terminal_outcome_and_last_activity();
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_HARNESS_COUNTER: AtomicU64 = AtomicU64::new(0);

    pub(crate) fn test_harness_components() -> (HarnessCatalog, Arc<HarnessStore>) {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let path = std::env::temp_dir().join(format!(
            "orkworksd-test-harnesses-{}-{}.json",
            std::process::id(),
            TEST_HARNESS_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let store = Arc::new(HarnessStore::new(path, builtins));
        let registry = store.load().unwrap().registry;
        let catalog = Arc::new(StdRwLock::new(registry));
        (catalog, store)
    }
    use crate::session_types::MemoryState;
    use crate::session_view::{connectivity_for_status, terminal_outcome_for_status};
    use std::sync::{Mutex as StdMutex, MutexGuard, OnceLock};

    pub(crate) struct FakeHome {
        previous: Option<std::ffi::OsString>,
        _lock: MutexGuard<'static, ()>,
    }

    impl FakeHome {
        pub(crate) fn set(home: &std::path::Path) -> Self {
            static HOME_LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
            let lock = HOME_LOCK.get_or_init(|| StdMutex::new(()));
            let _lock = lock.lock().unwrap();
            let previous = std::env::var_os("HOME");
            std::env::set_var("HOME", home);
            Self { previous, _lock }
        }
    }

    impl Drop for FakeHome {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    pub(crate) struct FakePath {
        previous: Option<std::ffi::OsString>,
        _lock: MutexGuard<'static, ()>,
    }

    impl FakePath {
        /// Prepends `dir` to the real `PATH`, so a test can plant a fake
        /// executable that resolves first without stripping real system
        /// binaries other concurrently-running tests may need.
        pub(crate) fn prepend(dir: &std::path::Path) -> Self {
            static PATH_LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
            let lock = PATH_LOCK.get_or_init(|| StdMutex::new(()));
            let _lock = lock.lock().unwrap();
            let previous = std::env::var_os("PATH");
            let mut dirs = vec![dir.to_path_buf()];
            if let Some(existing) = &previous {
                dirs.extend(std::env::split_paths(existing));
            }
            let joined =
                std::env::join_paths(dirs).expect("test PATH directories must be joinable");
            std::env::set_var("PATH", joined);
            Self { previous, _lock }
        }
    }

    impl Drop for FakePath {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var("PATH", value),
                None => std::env::remove_var("PATH"),
            }
        }
    }

    #[cfg(unix)]
    pub(crate) fn make_test_executable(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(not(unix))]
    pub(crate) fn make_test_executable(_path: &std::path::Path) {}

    pub(crate) fn with_fake_home<T>(home: &std::path::Path, f: impl FnOnce() -> T) -> T {
        let _home = FakeHome::set(home);
        f()
    }

    pub(crate) fn test_app_state_with_workspace(path: &std::path::Path) -> Arc<AppState> {
        let metadata_root = path.join(".orkworks-test");
        let (harness_catalog, harness_store) = test_harness_components();
        Arc::new(AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(Some(WorkspaceState {
                path: path.to_path_buf(),
                metadata: metadata::MetadataStore::new(&metadata_root),
                workflow_observations: workflow_observations::WorkflowObservationStore::open(
                    metadata_root.clone(),
                )
                .expect("open workflow observation store"),
                recommendation_store: taskmaster::store::RecommendationStore::open(
                    metadata_root.clone(),
                )
                .expect("open recommendation store"),
                watcher: watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
            })),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                label_epochs: StdRwLock::new(HashMap::new()),
                input_buf: StdRwLock::new(HashMap::new()),
                reported_cwd: StdRwLock::new(HashMap::new()),
                diagnostics: StdRwLock::new(HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: harness_catalog.clone(),
            harness_store,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::new_with_catalog(harness_catalog),
        })
    }

    /// Replaces the active workspace with a fresh one rooted at `path`,
    /// simulating a mid-request workspace switch. `WorkspaceState` is
    /// private to this module, so tests elsewhere in the crate that need to
    /// exercise a workspace change (rather than just clearing it, which
    /// `*state.workspace.lock().unwrap() = None` already handles inline)
    /// go through this helper instead of constructing one directly.
    pub(crate) fn swap_workspace(state: &AppState, path: &std::path::Path) {
        let metadata_root = path.join(".orkworks-test");
        *state.workspace.lock().unwrap() = Some(WorkspaceState {
            path: path.to_path_buf(),
            metadata: metadata::MetadataStore::new(&metadata_root),
            workflow_observations: workflow_observations::WorkflowObservationStore::open(
                metadata_root.clone(),
            )
            .expect("open workflow observation store"),
            recommendation_store: taskmaster::store::RecommendationStore::open(
                metadata_root.clone(),
            )
            .expect("open recommendation store"),
            watcher: watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
        });
        state.bump_harness_probe_generation();
    }

    pub(crate) fn test_session_info(
        id: impl Into<String>,
        label: impl Into<String>,
        cwd: impl Into<String>,
        status: impl Into<String>,
        created_at: impl Into<String>,
    ) -> SessionInfo {
        let status = status.into();
        let connectivity = Some(connectivity_for_status(&status).to_string());
        let terminal_outcome = terminal_outcome_for_status(&status);
        let created_at = created_at.into();

        SessionInfo {
            id: id.into(),
            label: label.into(),
            harness_id: None,
            model_provider_id: None,
            model_id: None,
            harness: None,
            model: None,
            work_phase: "unknown".into(),
            lifecycle_phase: "active".into(),
            lifecycle: "alive".into(),
            attention: None,
            status,
            connectivity,
            terminal_outcome,
            cwd: cwd.into(),
            created_at: created_at.clone(),
            last_activity_at: Some(created_at),
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
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            peon_diagnostics: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resume_options: vec![],
            resumed_from: None,
            has_openable_plan: None,
        }
    }

    pub(crate) fn test_session_metadata(
        id: impl Into<String>,
        label: impl Into<String>,
        workspace: impl Into<String>,
        status: impl Into<String>,
        created_at: impl Into<String>,
        last_activity: impl Into<String>,
    ) -> metadata::SessionMetadata {
        metadata::SessionMetadata {
            id: id.into(),
            label: label.into(),
            label_from_initial_prompt: false,
            workspace: workspace.into(),
            task: String::new(),
            harness: String::new(),
            model: String::new(),
            cwd: "/tmp".into(),
            status: status.into(),
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
            created_at: created_at.into(),
            last_activity: last_activity.into(),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    fn test_router(state: Arc<AppState>) -> Router {
        // The shared builder is the production router; the test fixture
        // delegates to it so route-registration drift is impossible.
        build_router(state)
    }

    async fn test_server_base_url(state: Arc<AppState>) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let app = test_router(state);
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{}", addr), server)
    }

    #[tokio::test]
    async fn session_routes_remain_registered_with_current_methods_and_paths() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (base_url, server) = test_server_base_url(state).await;
        let client = reqwest::Client::new();

        let cases = [
            (reqwest::Method::GET, format!("{}/workspace", base_url)),
            (
                reqwest::Method::GET,
                format!("{}/workspace/active-session", base_url),
            ),
            (
                reqwest::Method::GET,
                format!("{}/workspace/active-harnesses", base_url),
            ),
            (reqwest::Method::PUT, format!("{}/sessions", base_url)),
            (
                reqwest::Method::GET,
                format!("{}/sessions/test-id/forget", base_url),
            ),
            (
                reqwest::Method::GET,
                format!("{}/sessions/test-id/resume", base_url),
            ),
            (
                reqwest::Method::GET,
                format!("{}/sessions/test-id/harness-session", base_url),
            ),
            (
                reqwest::Method::POST,
                format!("{}/sessions/test-id", base_url),
            ),
            // Pins the fix that wired `/sessions/:id/plan-path` through the
            // shared `build_router` so production and tests cannot drift.
            (
                reqwest::Method::GET,
                format!("{}/sessions/test-id/plan-path", base_url),
            ),
        ];

        for (method, url) in cases {
            let response = client.request(method, url).send().await.unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);
        }

        server.abort();
        let _ = server.await;
    }

    #[tokio::test]
    async fn peon_provider_routes_expose_staged_api_and_remove_legacy_model_listing() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (base_url, server) = test_server_base_url(state).await;
        let client = reqwest::Client::new();

        let applied = client
            .get(format!("{base_url}/settings/peon/applied"))
            .send()
            .await
            .unwrap();
        assert_eq!(applied.status(), reqwest::StatusCode::OK);
        let applied_body: serde_json::Value = applied.json().await.unwrap();
        assert_eq!(applied_body["provider"], serde_json::Value::Null);
        assert_eq!(applied_body["connectionRevision"], 0);

        let legacy_models = client
            .get(format!("{base_url}/providers/ollama/models"))
            .send()
            .await
            .unwrap();
        assert_eq!(legacy_models.status(), reqwest::StatusCode::NOT_FOUND);

        let apply = client
            .post(format!("{base_url}/settings/peon/test-and-apply"))
            .json(&serde_json::json!({
                "selection": {
                    "provider": "ollama",
                    "model": "manual-model"
                },
                "generation": 1
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(apply.status(), reqwest::StatusCode::CONFLICT);
        let apply_body: serde_json::Value = apply.json().await.unwrap();
        assert_eq!(apply_body["error"]["code"], "verification_required");

        let malformed_verify = client
            .post(format!("{base_url}/settings/peon/provider/verify"))
            .header("content-type", "application/json")
            .body("not-json")
            .send()
            .await
            .unwrap();
        assert_eq!(malformed_verify.status(), reqwest::StatusCode::BAD_REQUEST);
        let verify_body: serde_json::Value = malformed_verify.json().await.unwrap();
        assert_eq!(verify_body["error"]["code"], "malformed");

        server.abort();
        let _ = server.await;
    }

    #[tokio::test]
    async fn peon_provider_routes_verify_apply_and_serialize_applied_state() {
        use std::io::{Read as _, Write as _};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 4096];
                let count = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..count]);
                let body = if request.starts_with("GET /api/tags") {
                    r#"{"models":[{"name":"manual-model"}]}"#
                } else {
                    r#"{"response":"{\"observedStatus\":\"working\",\"confidence\":0.9}","done":true}"#
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });

        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (base_url, app_server) = test_server_base_url(state).await;
        let client = reqwest::Client::new();
        let ollama_url = format!("http://{address}/");

        let verify = client
            .post(format!("{base_url}/settings/peon/provider/verify"))
            .json(&serde_json::json!({
                "provider": "ollama",
                "ollamaBaseUrl": ollama_url,
                "generation": 1
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(verify.status(), reqwest::StatusCode::OK);
        let verify_body: serde_json::Value = verify.json().await.unwrap();
        assert_eq!(verify_body["ok"], true);
        assert_eq!(verify_body["provider"], "ollama");
        assert_eq!(verify_body["capabilities"]["connectivity"], true);
        assert_eq!(verify_body["capabilities"]["modelDiscovery"], true);
        assert_eq!(verify_body["capabilities"]["providerDefault"], false);
        assert_eq!(verify_body["capabilities"]["testInference"], true);
        assert_eq!(verify_body["models"], serde_json::json!(["manual-model"]));
        assert_eq!(verify_body["ollamaBaseUrl"], format!("http://{address}"));
        assert_eq!(verify_body["generation"], 1);

        let apply = client
            .post(format!("{base_url}/settings/peon/test-and-apply"))
            .json(&serde_json::json!({
                "selection": {
                    "provider": "ollama",
                    "model": "manual-model",
                    "ollamaBaseUrl": ollama_url
                },
                "generation": 1
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(apply.status(), reqwest::StatusCode::OK);
        let apply_body: serde_json::Value = apply.json().await.unwrap();
        assert_eq!(apply_body["provider"], "ollama");
        assert_eq!(apply_body["model"], "manual-model");
        assert_eq!(apply_body["ollamaBaseUrl"], format!("http://{address}"));
        assert_eq!(apply_body["connectionRevision"], 1);
        assert!(apply_body["appliedAt"].as_str().is_some());

        let applied = client
            .get(format!("{base_url}/settings/peon/applied"))
            .send()
            .await
            .unwrap();
        assert_eq!(applied.status(), reqwest::StatusCode::OK);
        let applied_body: serde_json::Value = applied.json().await.unwrap();
        assert_eq!(applied_body, apply_body);

        server.join().unwrap();
        app_server.abort();
        let _ = app_server.await;
    }

    #[tokio::test]
    async fn summary_log_route_is_registered() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (base_url, server) = test_server_base_url(state).await;

        let response = reqwest::get(format!("{}/sessions/missing-session/summary-log", base_url))
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap(),
            serde_json::json!({ "entries": [] })
        );

        server.abort();
        let _ = server.await;
    }

    #[tokio::test]
    async fn deleting_a_harness_distinguishes_unknown_id_from_builtin() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (base_url, server) = test_server_base_url(state).await;
        let client = reqwest::Client::new();
        let revision = serde_json::json!({"expectedRevision": null});

        let missing = client
            .delete(format!("{}/harnesses/not-a-real-harness", base_url))
            .json(&revision)
            .send()
            .await
            .unwrap();
        assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);

        let builtin = client
            .delete(format!("{}/harnesses/codex", base_url))
            .json(&revision)
            .send()
            .await
            .unwrap();
        assert_eq!(builtin.status(), reqwest::StatusCode::CONFLICT);

        server.abort();
        let _ = server.await;
    }

    #[test]
    fn session_registry_create_and_list() {
        let state = Arc::new(AppState {
            sessions: Mutex::new(HashMap::new()),
            projection_lock: Mutex::new(()),
            session_pids: Mutex::new(HashMap::new()),
            workspace: Mutex::new(None),
            peon: PeonState {
                last_output: StdRwLock::new(HashMap::new()),
                last_inference: StdRwLock::new(HashMap::new()),
                in_flight: StdRwLock::new(HashSet::new()),
                label_hint: StdRwLock::new(HashMap::new()),
                label_pending: StdRwLock::new(HashSet::new()),
                label_epochs: StdRwLock::new(HashMap::new()),
                input_buf: StdRwLock::new(HashMap::new()),
                reported_cwd: StdRwLock::new(HashMap::new()),
                diagnostics: StdRwLock::new(HashMap::new()),
                config: peon::PeonConfig::from_env(),
            },
            harness_catalog: test_harness_components().0,
            harness_store: test_harness_components().1,
            integration_probe_cache: crate::harness::probe_cache::VersionProbeCache::new(),
            retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
            bound_port: AtomicU16::new(0),
            providers: providers::ProviderManager::new(),
        });

        assert!(state.sessions.lock().unwrap().is_empty());

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let id = "test-1".to_string();
        let info = test_session_info(id.clone(), "Test", "/tmp", "creating", "now");

        state.sessions.lock().unwrap().insert(
            id,
            SessionHandle {
                info: info.clone(),
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

        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        let stored = sessions.get("test-1").unwrap();
        assert_eq!(stored.info.label, "Test");
        assert_eq!(stored.info.status, "creating");
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
}
