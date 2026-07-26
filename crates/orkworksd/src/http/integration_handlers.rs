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
    let message = error.to_string();
    (status, Json(ErrorResponse { error: message })).into_response()
}

async fn run_integration_action(
    state: &Arc<AppState>,
    harness_id: &str,
    action: impl FnOnce(&ResolvedHarness, &IntegrationContext<'_>) -> Result<
        crate::harness::integration::IntegrationStatus,
        IntegrationError,
    >,
) -> axum::response::Response {
    // Captured before the async probe below (and the lock dropped
    // immediately) so a concurrent workspace switch or harness-definition
    // edit during the probe's up-to-3s window can be *detected* afterward
    // rather than silently acted on — see the re-checks below the probe.
    // This also preserves today's exact error-priority order: a request
    // against a missing workspace reports NoWorkspace even if harness_id is
    // also unknown.
    let workspace_path_at_start = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ws) = ws_guard.as_ref() else {
            return integration_error_response(IntegrationError::NoWorkspace);
        };
        ws.path.clone()
    };

    let harness: ResolvedHarness = {
        let registry = state.harness_catalog.read().expect("harness catalog lock poisoned");
        let Some(harness) = registry.get(harness_id) else {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: format!("unknown harness id \"{harness_id}\"") }),
            )
                .into_response();
        };
        harness.clone()
    };

    // No lock guard is held from here through the `.await` below — both
    // guards above are `!Send`, and the workspace mutex must not be held
    // for the probe's full timeout, which other workspace-touching requests
    // would otherwise queue behind.
    let probed_tool = crate::harness::detect::probe_installed_tool(&harness.launch_command());
    let detected_tool = match (probed_tool, harness.definition.min_version.as_ref()) {
        (Some(mut tool), Some(requirement)) => {
            match crate::harness::detect::probe_tool_version(&tool.executable).await {
                Some(output) => match crate::harness::detect::parse_version_token(&output) {
                    Some(parsed) => {
                        tool.compatible = parsed >= requirement.min;
                        tool.version = Some(output);
                    }
                    None => {
                        tool.compatible = false;
                        tool.version = None;
                    }
                },
                None => {
                    tool.compatible = false;
                    tool.version = None;
                }
            }
            Some(tool)
        }
        (probed_tool, _) => probed_tool,
    };

    // Re-validate both pieces of state captured before the probe. A
    // concurrent harness-definition edit or workspace switch landing during
    // the probe's window must not let this request silently proceed against
    // a target different from the one it started against; the harness clone
    // above and the workspace path captured above would otherwise go stale
    // without either lock ever objecting.
    {
        let registry = state.harness_catalog.read().expect("harness catalog lock poisoned");
        match registry.get(harness_id) {
            Some(current) if current.definition == harness.definition => {}
            _ => {
                return (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "harness definition changed during this request; retry".into(),
                    }),
                )
                    .into_response();
            }
        }
    }

    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return integration_error_response(IntegrationError::NoWorkspace);
    };
    if ws.path != workspace_path_at_start {
        return (
            StatusCode::CONFLICT,
            Json(ErrorResponse { error: "workspace changed during this request; retry".into() }),
        )
            .into_response();
    }

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
        detected_tool: detected_tool.as_ref(),
        reporter_assets: &reporter_assets,
    };

    match action(&harness, &ctx) {
        Ok(status) => Json(status).into_response(),
        Err(error) => integration_error_response(error),
    }
}

pub(crate) async fn get_integration_status(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_status(ctx)).await
}

pub(crate) async fn install_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_install(ctx)).await
}

pub(crate) async fn uninstall_integration(
    State(state): State<Arc<AppState>>,
    AxumPath(harness_id): AxumPath<String>,
) -> impl IntoResponse {
    run_integration_action(&state, &harness_id, |harness, ctx| harness.integration_uninstall(ctx)).await
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

    fn init_git_workspace_with_copilot_settings_ignored(workspace: &std::path::Path) {
        git2::Repository::init(workspace).unwrap();
        std::fs::write(
            workspace.join(".gitignore"),
            ".github/copilot/settings.local.json\n",
        )
        .unwrap();
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

    #[tokio::test]
    async fn detected_tool_reflects_probe_result_for_a_resolvable_command() {
        use crate::test_support::{make_test_executable, FakePath};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let install_response =
            install_integration(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let fake_bin_dir = tempfile::tempdir().unwrap();
        // On Windows, probe_installed_tool searches PATHEXT candidates
        // (claude.exe, claude.cmd, ...) for a bare "claude" — a plain
        // extensionless file wouldn't match any of them.
        let bin_name = if cfg!(windows) { "claude.exe" } else { "claude" };
        let bin = fake_bin_dir.path().join(bin_name);
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], true);
        assert_eq!(body["activation"], "active");
    }

    #[tokio::test]
    async fn min_version_gating_marks_a_below_threshold_binary_as_needing_trust() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (99, 0, 0) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\necho 'copilot-cli 1.0.0'\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let response = get_integration_status(State(state), AxumPath("copilot".into()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], true);
        assert_eq!(body["activation"], "needs_trust");
        assert!(body["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "unsupported_tool_version"));
    }

    #[tokio::test]
    async fn min_version_gating_leaves_an_above_threshold_binary_fully_active() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\necho 'copilot-cli 1.0.0'\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        // Active (as opposed to Absent/Disabled) only applies once the hook
        // is actually Installed — status_from_document's activation match
        // only reaches self.contract.activation via the `registration ==
        // Installed` arm, so an install must happen first, exactly like the
        // pre-existing Claude reference test this one is modeled on
        // (detected_tool_reflects_probe_result_for_a_resolvable_command).
        // Copilot (like Claude, unlike Gemini) declares
        // activation: IntegrationActivation::Active on its contract
        // (harness/integrations/copilot.rs) — Gemini's own contract
        // declares Unknown even when fully installed and detected (its
        // coverage is Limited by design), which is unrelated to min_version
        // and would make this assertion fail no matter what this task
        // wires up.
        let install_response =
            install_integration(State(state.clone()), AxumPath("copilot".into()))
                .await
                .into_response();
        assert_eq!(install_response.status(), StatusCode::OK);

        let response = get_integration_status(State(state), AxumPath("copilot".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], true);
        assert_eq!(body["activation"], "active");
    }

    #[tokio::test]
    async fn a_slow_version_probe_does_not_block_a_concurrent_workspace_request() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
        let bin = fake_bin_dir.path().join(bin_name);
        // `exec` matters here exactly as it does in detect.rs's kill-on-
        // timeout test: without it, `sh` forks `sleep` as a grandchild that
        // kill_on_drop's signal never reaches, leaking it independently of
        // (and in addition to) whatever detect.rs's own test covers — that
        // test only exercises its own spawned binary, not this one.
        std::fs::write(&bin, "#!/bin/sh\nexec sleep 30\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let slow_state = state.clone();
        let slow_request = tokio::spawn(async move {
            get_integration_status(State(slow_state), AxumPath("copilot".into())).await.into_response()
        });
        // Give the slow request a head start into its probe before firing
        // the concurrent one.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let start = std::time::Instant::now();
        let concurrent_response =
            get_integration_status(State(state.clone()), AxumPath("claude-code".into()))
                .await
                .into_response();
        let elapsed = start.elapsed();

        assert_eq!(concurrent_response.status(), StatusCode::OK);
        // 2s of margin below the probe's 3s timeout: generous enough to
        // absorb scheduling jitter on a loaded CI runner while still being a
        // meaningful signal that the concurrent request didn't queue behind
        // the slow one. If this proves flaky in practice, widen the margin
        // rather than add retry logic — a single fixed threshold is enough
        // signal for what this test is checking.
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "a concurrent request must not wait behind the slow probe's 3s timeout, took {elapsed:?}"
        );

        // Clean up: let the slow request finish so the test doesn't leak a
        // background task past its own scope.
        let slow_response = slow_request.await.unwrap();
        assert_eq!(slow_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn a_workspace_switch_during_the_probe_is_rejected_instead_of_targeting_the_new_one() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, swap_workspace, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\nexec sleep 30\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let slow_state = state.clone();
        let slow_request = tokio::spawn(async move {
            get_integration_status(State(slow_state), AxumPath("copilot".into())).await.into_response()
        });
        // Give the slow request a head start into its probe, then switch
        // the active workspace to a different directory while it's still
        // in flight.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let other_dir = tempfile::tempdir().unwrap();
        swap_workspace(&state, other_dir.path());

        let response = slow_request.await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::CONFLICT,
            "a request whose workspace changed mid-flight must be rejected, not silently \
             completed against the new workspace"
        );
    }

    #[tokio::test]
    async fn a_harness_definition_change_during_the_probe_is_rejected_instead_of_using_stale_data() {
        use crate::harness::definition::{HarnessPatch, VersionRequirement};
        use crate::test_support::{make_test_executable, FakePath};

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_copilot_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "copilot".to_string(),
                    HarnessPatch {
                        min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                        ..Default::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let fake_bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
        let bin = fake_bin_dir.path().join(bin_name);
        std::fs::write(&bin, "#!/bin/sh\nexec sleep 30\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(fake_bin_dir.path());

        let slow_state = state.clone();
        let slow_request = tokio::spawn(async move {
            get_integration_status(State(slow_state), AxumPath("copilot".into())).await.into_response()
        });
        // Give the slow request a head start into its probe, then change the
        // harness definition it's probing against while it's still in
        // flight. This swaps the resolved registry directly rather than
        // going through HarnessStore::mutate a second time on the same
        // store — a second sequential mutate() call on one store hits an
        // unrelated pre-existing bug (issue #230, reproduced with a plain
        // .name patch too, nothing to do with min_version) where the store
        // fails to read back what it just wrote. Swapping the registry
        // directly still exercises exactly what this test needs: the
        // harness_catalog RwLock's content changing between this request's
        // clone and its re-validation.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let mut changed_document = crate::harness::definition::HarnessUserDocument::default();
        changed_document.overrides.insert(
            "copilot".to_string(),
            HarnessPatch {
                min_version: Some(Some(VersionRequirement { min: (99, 0, 0) })),
                ..Default::default()
            },
        );
        let builtins = crate::harness::definition::BuiltinDocument::parse(
            crate::harness::definition::EMBEDDED_BUILTINS,
        )
        .unwrap();
        let changed_registry =
            crate::harness::registry::resolve_document(&builtins, &changed_document).unwrap();
        *state.harness_catalog.write().expect("harness catalog lock poisoned") =
            std::sync::Arc::new(changed_registry);

        let response = slow_request.await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::CONFLICT,
            "a request whose harness definition changed mid-flight must be rejected, not \
             silently completed against the stale pre-patch definition"
        );
    }

    // This one already passes before the fix too (detected_tool was always
    // None, so activation was always forced to Unknown regardless of the
    // real command) — it's a regression guard for the genuinely-not-found
    // path, not a red/green driver. Kept alongside the test above for
    // coverage of both outcomes of the same wiring.
    //
    // It overrides claude-code's launch command to an unresolvable name
    // rather than relying on the ambient PATH lacking a real `claude`
    // binary: FakePath::prepend only prepends (by design — see Task 1), so
    // it can't hide a real `claude` install elsewhere on PATH, and this
    // test must still pass on a machine that has Claude Code installed.
    #[tokio::test]
    async fn detected_tool_stays_absent_when_the_command_is_not_on_path() {
        use crate::harness::definition::{HarnessPatch, LaunchPatch};
        use crate::test_support::FakePath;

        let dir = tempfile::tempdir().unwrap();
        init_git_workspace_with_claude_settings_ignored(dir.path());
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        state
            .harness_store
            .mutate(&state.harness_catalog, |document| {
                document.overrides.insert(
                    "claude-code".to_string(),
                    HarnessPatch {
                        name: None,
                        launch: Some(LaunchPatch {
                            kind: None,
                            command: Some("definitely-not-a-real-binary-xyz".to_string()),
                            args: None,
                            model_prefix: None,
                            login: None,
                        }),
                        default_model: None,
                        resume: None,
                        models: None,
                        peon: None,
                        capacity: None,
                        session_signals: None,
                        integration: None,
                        voice: None,
                        min_version: None,
                    },
                );
                Ok(())
            })
            .unwrap();

        let empty_bin_dir = tempfile::tempdir().unwrap();
        let _fake_path = FakePath::prepend(empty_bin_dir.path());

        let response = get_integration_status(State(state), AxumPath("claude-code".into()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["toolDetected"], false);
        assert_eq!(body["activation"], "unknown");
        assert!(body["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "tool_not_detected"));
    }
}
