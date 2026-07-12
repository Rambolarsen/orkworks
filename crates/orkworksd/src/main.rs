use axum::{
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

mod metadata;
mod watcher;
mod git;
mod harness;
mod harness_registry;
mod peon;
mod providers;
mod domain;
mod application;
mod http;
mod infrastructure;
mod migration;
mod runtime;
mod session_types;
mod session_view;
mod workspace_runtime;

use crate::infrastructure::session_module::SessionModule;
use crate::harness_registry::{builtin_adapters, load_harnesses, HarnessConfig};
use crate::http::harness_handlers::{
    create_harness, delete_harness, list_harnesses, update_harness,
};
use crate::http::hook_handlers::{get_attention_hook_status, install_attention_hook};
use crate::http::provider_handlers::{get_provider_models, get_providers, set_provider_settings, verify_ollama_settings};
use crate::http::retention_handlers::set_retention;
use crate::http::session_handlers::{
    create_session, delete_session, forget_session, list_sessions, report_attention,
    report_harness_session, resume_session, set_active_harnesses, set_active_session,
    set_workspace,
};
use crate::runtime::peon_runtime::peon_loop;
use crate::runtime::retention::retention_cleanup_task;
use crate::runtime::terminal_http::{get_terminal_output, session_terminal_handler};
use crate::session_types::SessionInfo;

struct SessionHandle {
    info: SessionInfo,
    kill_tx: tokio::sync::watch::Sender<bool>,
    output_buffer: peon::RingBuffer,
    // Rolling raw PTY text (ANSI-stripped) for TUI apps that use cursor positioning instead of newlines.
    scan_buf: String,
    command: harness::CommandSpec,
    initial_prompt: Option<String>,
    runtime: runtime::session_runtime::SessionRuntime,
    terminal_attached: bool,
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

    let harnesses = load_harnesses();
    let providers = providers::ProviderManager::new_with_harnesses(&harnesses);

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
        providers,
        adapters: builtin_adapters(),
        retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
        harnesses: tokio::sync::RwLock::new(harnesses),
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
        .route("/settings/providers/ollama/verify", post(verify_ollama_settings))
        .route("/workspace", post(set_workspace))
        .route("/workspace/active-session", post(set_active_session))
        .route("/workspace/active-harnesses", put(set_active_harnesses))
        .route("/workspace/attention-hook/status", get(get_attention_hook_status))
        .route("/workspace/attention-hook/install", post(install_attention_hook))
        .route("/sessions", post(create_session))
        .route("/sessions", get(list_sessions))
        .route("/sessions/:id", delete(delete_session))
        .route("/sessions/:id/forget", delete(forget_session))
        .route("/sessions/:id/resume", post(resume_session))
        .route("/sessions/:id/harness-session", post(report_harness_session))
        .route("/sessions/:id/attention", post(report_attention))
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

async fn health_check() -> &'static str {
    "ok"
}

#[cfg(test)]
#[test]
fn session_metadata_serializes_connectivity_terminal_outcome_and_last_activity() {
    metadata::assert_session_metadata_serializes_connectivity_terminal_outcome_and_last_activity();
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
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

    pub(crate) fn with_fake_home<T>(home: &std::path::Path, f: impl FnOnce() -> T) -> T {
        let _home = FakeHome::set(home);
        f()
    }

    pub(crate) fn test_app_state_with_workspace(path: &std::path::Path) -> Arc<AppState> {
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
            status,
            connectivity,
            terminal_outcome,
            cwd: cwd.into(),
            created_at: created_at.clone(),
            last_activity_at: Some(created_at),
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
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resume_options: vec![],
            resumed_from: None,
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
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        Router::new()
            .route("/health", get(health_check))
            .route("/providers", get(get_providers))
            .route("/providers/:id/models", get(get_provider_models))
            .route("/settings/providers", post(set_provider_settings))
            .route("/workspace", post(set_workspace))
            .route("/workspace/active-session", post(set_active_session))
            .route("/workspace/active-harnesses", put(set_active_harnesses))
            .route("/workspace/attention-hook/status", get(get_attention_hook_status))
            .route("/workspace/attention-hook/install", post(install_attention_hook))
            .route("/sessions", post(create_session))
            .route("/sessions", get(list_sessions))
            .route("/sessions/:id", delete(delete_session))
            .route("/sessions/:id/forget", delete(forget_session))
            .route("/sessions/:id/resume", post(resume_session))
            .route("/sessions/:id/harness-session", post(report_harness_session))
            .route("/sessions/:id/attention", post(report_attention))
            .route("/settings/retention", post(set_retention))
            .route("/harnesses", get(list_harnesses).post(create_harness))
            .route("/harnesses/:id", put(update_harness).delete(delete_harness))
            .route("/sessions/:id/terminal", get(session_terminal_handler))
            .route("/sessions/:id/terminal-output", get(get_terminal_output))
            .layer(cors)
            .with_state(state)
    }

    async fn test_server_base_url(state: Arc<AppState>) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
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
            (reqwest::Method::POST, format!("{}/sessions/test-id", base_url)),
        ];

        for (method, url) in cases {
            let response = client.request(method, url).send().await.unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);
        }

        server.abort();
        let _ = server.await;
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
        let info = test_session_info(id.clone(), "Test", "/tmp", "creating", "now");

        state
            .sessions
            .lock()
            .unwrap()
            .insert(id, SessionHandle {
                info: info.clone(),
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: harness_registry::default_shell_command("/tmp".into()),
                initial_prompt: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            });

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
