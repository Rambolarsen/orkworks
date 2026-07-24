use crate::harness::integration::{IntegrationContext, IntegrationError, ReporterAssetResolver};
use crate::harness::registry::ResolvedHarness;
use crate::http::ErrorResponse;
use crate::AppState;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn resolve_scripts_source_dir(exe_dir: Option<PathBuf>, manifest_dir: &Path) -> PathBuf {
    if let Some(dir) = exe_dir {
        let packaged = dir.join("scripts");
        if packaged.is_dir() {
            return packaged;
        }
    }
    manifest_dir.join("scripts")
}

fn scripts_source_dir() -> PathBuf {
    resolve_scripts_source_dir(
        std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
}

fn stable_hook_scripts_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".orkworks").join("hook-scripts"))
}

fn reporter_assets() -> Result<ReporterAssetResolver, String> {
    let stable_dir = stable_hook_scripts_dir()
        .ok_or_else(|| "couldn't resolve home directory for the reporter scripts".to_string())?;
    Ok(ReporterAssetResolver { source_dir: scripts_source_dir(), stable_dir })
}

fn integration_error_response(error: IntegrationError) -> axum::response::Response {
    let status = match &error {
        IntegrationError::NoWorkspace
        | IntegrationError::RevisionChanged
        | IntegrationError::OwnershipAmbiguous => StatusCode::CONFLICT,
        IntegrationError::UnsafeTarget { .. } | IntegrationError::InvalidConfig(_) => {
            StatusCode::BAD_REQUEST
        }
        IntegrationError::LaunchConflict | IntegrationError::Io(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    let message = match &error {
        IntegrationError::NoWorkspace => "Open a workspace first.".to_string(),
        IntegrationError::UnsafeTarget { message, .. } => message.clone(),
        IntegrationError::InvalidConfig(message) => message.clone(),
        IntegrationError::OwnershipAmbiguous => {
            "This integration's config entry doesn't match what OrkWorks installed; resolve it manually.".to_string()
        }
        IntegrationError::LaunchConflict => "Unexpected launch conflict.".to_string(),
        IntegrationError::RevisionChanged => "Configuration changed; retry the request.".to_string(),
        IntegrationError::Io(error) => error.to_string(),
    };
    (status, Json(ErrorResponse { error: message })).into_response()
}

fn run_integration_action(
    state: &Arc<AppState>,
    harness_id: &str,
    action: impl FnOnce(&ResolvedHarness, &IntegrationContext<'_>) -> Result<
        crate::harness::integration::IntegrationStatus,
        IntegrationError,
    >,
) -> axum::response::Response {
    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return integration_error_response(IntegrationError::NoWorkspace);
    };

    let registry = state.harness_catalog.read().expect("harness catalog lock poisoned");
    let Some(harness) = registry.get(harness_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("unknown harness id \"{harness_id}\"") }),
        )
            .into_response();
    };

    let reporter_assets = match reporter_assets() {
        Ok(resolver) => resolver,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error })).into_response();
        }
    };

    let orkworks_root = match dirs::home_dir() {
        Some(home) => home.join(".orkworks"),
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "couldn't resolve home directory".into() }),
            )
                .into_response();
        }
    };

    let ctx = IntegrationContext {
        workspace: &ws.path,
        workspace_metadata: Some(&ws.metadata),
        orkworks_root: &orkworks_root,
        enabled: true,
        detected_tool: None,
        reporter_assets: &reporter_assets,
    };

    match action(harness, &ctx) {
        Ok(status) => Json(status).into_response(),
        Err(error) => integration_error_response(error),
    }
}

pub(crate) async fn get_integration_status(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_status(ctx))
}

pub(crate) async fn install_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_install(ctx))
}

pub(crate) async fn uninstall_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_uninstall(ctx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_app_state_with_workspace, FakeHome};
    use serde_json::Value;

    // Claude's handler is a JsonHookHandler: `require_local_or_ignored_untracked`
    // (harness/integration.rs) refuses to read or write its config file unless
    // the workspace is a Git repository with that file gitignored. Every test
    // below that reaches Claude's handler must set this up first, or it hits
    // `UnsafeTarget { code: "not_git_workspace" }` (mapped to 400) instead of
    // the behavior the test name describes — `status()` swallows that into an
    // `"error"` registration, and `install`/`uninstall` return it as an error.
    fn init_git_workspace_with_claude_settings_ignored(workspace: &std::path::Path) {
        git2::Repository::init(workspace).unwrap();
        std::fs::write(workspace.join(".gitignore"), ".claude/settings.local.json\n").unwrap();
    }

    // Pins the packaged-vs-dev fallback that used to be covered by
    // hook_handlers.rs's resolve_claude_hook_script_path_* tests (deleted
    // along with that file) — this is the same AppImage/packaging-sensitive
    // logic, just resolving a scripts directory instead of one script file.
    #[test]
    fn resolve_scripts_source_dir_prefers_packaged_layout_when_present() {
        let exe_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(exe_dir.path().join("scripts")).unwrap();
        let manifest_dir = tempfile::tempdir().unwrap();

        let resolved =
            resolve_scripts_source_dir(Some(exe_dir.path().to_path_buf()), manifest_dir.path());

        assert_eq!(resolved, exe_dir.path().join("scripts"));
    }

    #[test]
    fn resolve_scripts_source_dir_falls_back_to_dev_manifest_dir() {
        let exe_dir = tempfile::tempdir().unwrap();
        let manifest_dir = tempfile::tempdir().unwrap();

        let resolved =
            resolve_scripts_source_dir(Some(exe_dir.path().to_path_buf()), manifest_dir.path());

        assert_eq!(resolved, manifest_dir.path().join("scripts"));
    }

    #[tokio::test]
    async fn status_reports_absent_for_a_fresh_workspace() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "absent");
    }

    #[tokio::test]
    async fn install_then_status_reports_installed() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let install_response = install_integration(State(state.clone()), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let status_response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(status_response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "installed");
    }

    #[tokio::test]
    async fn install_then_uninstall_reports_absent() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let install_response = install_integration(State(state.clone()), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);
        let uninstall_response = uninstall_integration(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(uninstall_response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(uninstall_response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["registration"], "absent");
    }

    #[tokio::test]
    async fn status_without_a_workspace_returns_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        *state.workspace.lock().unwrap() = None;

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn status_for_an_unknown_harness_id_returns_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = get_integration_status(State(state), AxumPath("not-a-real-harness".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn install_rejects_malformed_existing_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("settings.local.json"), "not json").unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = install_integration(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
