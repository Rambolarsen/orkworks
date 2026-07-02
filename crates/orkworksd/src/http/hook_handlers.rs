use crate::http::ErrorResponse;
use crate::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const CLAUDE_HOOK_SCRIPT_NAME: &str = "report-claude-session-from-hook.sh";

#[derive(Serialize)]
pub(crate) struct AttentionHookStatusResponse {
    pub(crate) installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

/// Locates the Claude hook reporter script's current source: a packaged app ships
/// it as a sibling of the sidecar binary (`<resourcesPath>/scripts/...`); a dev
/// checkout falls back to its source location under this crate.
///
/// This is only a source to copy from, not the path installed into the hook
/// command — see `ensure_stable_claude_hook_script`. On Linux AppImage builds,
/// `current_exe()` resolves inside the per-launch temporary FUSE mount, so
/// persisting it directly into the hook command would break on the next launch.
pub(crate) fn claude_hook_script_path() -> PathBuf {
    resolve_claude_hook_script_path(
        std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
}

fn resolve_claude_hook_script_path(exe_dir: Option<PathBuf>, manifest_dir: &Path) -> PathBuf {
    if let Some(dir) = exe_dir {
        let packaged = dir.join("scripts").join(CLAUDE_HOOK_SCRIPT_NAME);
        if packaged.is_file() {
            return packaged;
        }
    }
    manifest_dir.join("scripts").join(CLAUDE_HOOK_SCRIPT_NAME)
}

/// Stable, packaging-independent location the installed hook command should
/// point at: `~/.orkworks/hook-scripts/<script>`, mirroring the existing
/// `~/.orkworks/harnesses.json` global-config convention (harness_registry.rs).
fn stable_claude_hook_script_path() -> Option<PathBuf> {
    stable_claude_hook_script_path_under(dirs::home_dir())
}

fn stable_claude_hook_script_path_under(home_dir: Option<PathBuf>) -> Option<PathBuf> {
    home_dir.map(|h| h.join(".orkworks").join("hook-scripts").join(CLAUDE_HOOK_SCRIPT_NAME))
}

/// Copies the current reporter script to the stable path and returns it. Runs on
/// every install so the copy self-heals across app updates and AppImage mount
/// points changing between launches, rather than persisting a path that can stop
/// existing by the next time the hook fires.
fn ensure_stable_claude_hook_script() -> Result<PathBuf, String> {
    ensure_stable_claude_hook_script_at(&claude_hook_script_path(), stable_claude_hook_script_path())
}

fn ensure_stable_claude_hook_script_at(
    source_path: &Path,
    stable_path: Option<PathBuf>,
) -> Result<PathBuf, String> {
    let stable_path = stable_path.ok_or_else(|| "couldn't resolve home directory for the hook script".to_string())?;

    if let Some(parent) = stable_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("couldn't create {}: {e}", parent.display()))?;
    }
    std::fs::copy(source_path, &stable_path)
        .map_err(|e| format!("couldn't copy {} to {}: {e}", source_path.display(), stable_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&stable_path)
            .map_err(|e| format!("couldn't stat {}: {e}", stable_path.display()))?
            .permissions();
        perms.set_mode(perms.mode() | 0o755);
        std::fs::set_permissions(&stable_path, perms)
            .map_err(|e| format!("couldn't chmod {}: {e}", stable_path.display()))?;
    }

    Ok(stable_path)
}

fn settings_local_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".claude").join("settings.local.json")
}

fn read_settings_local(path: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(data) => {
            let value: Value = serde_json::from_str(&data)
                .map_err(|e| format!("couldn't parse .claude/settings.local.json: {e}"))?;
            if !value.is_object() {
                return Err(
                    "couldn't parse .claude/settings.local.json: expected a JSON object at the top level".into(),
                );
            }
            Ok(value)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(e) => Err(format!("couldn't read .claude/settings.local.json: {e}")),
    }
}

fn is_hook_installed(settings: &Value) -> bool {
    let notification_entries = settings
        .get("hooks")
        .and_then(|h| h.get("Notification"))
        .and_then(|n| n.as_array())
        .into_iter()
        .flatten();

    notification_entries
        .flat_map(|entry| entry.get("hooks").and_then(|hs| hs.as_array()).into_iter().flatten())
        .any(|h| {
            h.get("command")
                .and_then(|c| c.as_str())
                .is_some_and(|c| c.contains(CLAUDE_HOOK_SCRIPT_NAME))
        })
}

fn insert_hook_entry(settings: &mut Value, command: &str) -> Result<(), String> {
    let obj = settings
        .as_object_mut()
        .ok_or_else(|| "settings root is not an object".to_string())?;

    let hooks_value = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
    let hooks_obj = hooks_value
        .as_object_mut()
        .ok_or_else(|| "\"hooks\" is not an object".to_string())?;

    let notification_value = hooks_obj
        .entry("Notification")
        .or_insert_with(|| serde_json::json!([]));
    let notification_arr = notification_value
        .as_array_mut()
        .ok_or_else(|| "\"hooks.Notification\" is not an array".to_string())?;

    notification_arr.push(serde_json::json!({
        "hooks": [
            { "type": "command", "command": command }
        ]
    }));

    Ok(())
}

pub(crate) async fn get_attention_hook_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return axum::http::StatusCode::CONFLICT.into_response();
    };

    match read_settings_local(&settings_local_path(&ws.path)) {
        Ok(settings) => Json(AttentionHookStatusResponse {
            installed: is_hook_installed(&settings),
            error: None,
        })
        .into_response(),
        Err(error) => Json(AttentionHookStatusResponse {
            installed: false,
            error: Some(error),
        })
        .into_response(),
    }
}

pub(crate) async fn install_attention_hook(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return axum::http::StatusCode::CONFLICT.into_response();
    };

    let path = settings_local_path(&ws.path);
    let mut settings = match read_settings_local(&path) {
        Ok(value) => value,
        Err(error) => {
            return (axum::http::StatusCode::BAD_REQUEST, Json(ErrorResponse { error })).into_response();
        }
    };

    if is_hook_installed(&settings) {
        return Json(AttentionHookStatusResponse { installed: true, error: None }).into_response();
    }

    let claude_dir = ws.path.join(".claude");
    if let Err(e) = std::fs::create_dir_all(&claude_dir) {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: format!("couldn't create .claude directory: {e}") }),
        )
            .into_response();
    }

    let stable_script_path = match ensure_stable_claude_hook_script() {
        Ok(path) => path,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: format!("couldn't install the reporter script: {e}") }),
            )
                .into_response();
        }
    };

    let command = format!("\"{}\"", stable_script_path.display());
    if let Err(error) = insert_hook_entry(&mut settings, &command) {
        return (axum::http::StatusCode::BAD_REQUEST, Json(ErrorResponse { error })).into_response();
    }

    let serialized = match serde_json::to_string_pretty(&settings) {
        Ok(s) => s,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: format!("couldn't serialize settings: {e}") }),
            )
                .into_response();
        }
    };

    if let Err(e) = std::fs::write(&path, serialized) {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("couldn't write .claude/settings.local.json: {e}"),
            }),
        )
            .into_response();
    }

    Json(AttentionHookStatusResponse { installed: true, error: None }).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_app_state_with_workspace;
    use std::sync::{Mutex as StdMutex, MutexGuard, OnceLock};

    /// Mirrors harness_registry.rs's test helper: serializes tests that mutate the
    /// process-global `HOME` env var so `dirs::home_dir()`-based code (like
    /// ensure_stable_claude_hook_script) never touches the real developer/CI home
    /// directory, and never races another test doing the same thing in parallel.
    /// RAII rather than a closure so `#[tokio::test]` bodies can freely `.await`
    /// while it's held — each test runs its own single-threaded runtime, so
    /// holding the guard across an await point here can't deadlock.
    struct FakeHome {
        previous: Option<std::ffi::OsString>,
        _lock: MutexGuard<'static, ()>,
    }

    impl FakeHome {
        fn set(home: &std::path::Path) -> Self {
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

    #[test]
    fn resolve_claude_hook_script_path_prefers_packaged_layout_when_present() {
        let exe_dir = tempfile::tempdir().unwrap();
        let scripts_dir = exe_dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join(CLAUDE_HOOK_SCRIPT_NAME), "#!/bin/sh\n").unwrap();

        let manifest_dir = tempfile::tempdir().unwrap();
        let resolved =
            resolve_claude_hook_script_path(Some(exe_dir.path().to_path_buf()), manifest_dir.path());

        assert_eq!(resolved, scripts_dir.join(CLAUDE_HOOK_SCRIPT_NAME));
    }

    #[test]
    fn resolve_claude_hook_script_path_falls_back_to_dev_manifest_dir() {
        let exe_dir = tempfile::tempdir().unwrap();
        let manifest_dir = tempfile::tempdir().unwrap();

        let resolved =
            resolve_claude_hook_script_path(Some(exe_dir.path().to_path_buf()), manifest_dir.path());

        assert_eq!(
            resolved,
            manifest_dir.path().join("scripts").join(CLAUDE_HOOK_SCRIPT_NAME),
        );
    }

    #[tokio::test]
    async fn install_creates_claude_dir_and_hook_entry_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let response = install_attention_hook(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let settings_path = dir.path().join(".claude").join("settings.local.json");
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
        assert!(is_hook_installed(&written));
    }

    #[tokio::test]
    async fn install_preserves_unrelated_existing_keys() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.local.json"),
            r#"{"permissions": {"allow": ["Bash(git *)"]}}"#,
        )
        .unwrap();

        let state = test_app_state_with_workspace(dir.path());
        let response = install_attention_hook(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let written: Value = serde_json::from_str(
            &std::fs::read_to_string(claude_dir.join("settings.local.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(written["permissions"]["allow"][0], "Bash(git *)");
        assert!(is_hook_installed(&written));
    }

    #[tokio::test]
    async fn install_is_idempotent_and_does_not_duplicate_entries() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let _ = install_attention_hook(State(state.clone())).await.into_response();
        let response = install_attention_hook(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let settings_path = dir.path().join(".claude").join("settings.local.json");
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
        let notification_entries = written["hooks"]["Notification"].as_array().unwrap();
        assert_eq!(notification_entries.len(), 1);
    }

    #[tokio::test]
    async fn install_rejects_malformed_existing_file_without_touching_it() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("settings.local.json"), "not json").unwrap();

        let state = test_app_state_with_workspace(dir.path());
        let response = install_attention_hook(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        let unchanged = std::fs::read_to_string(claude_dir.join("settings.local.json")).unwrap();
        assert_eq!(unchanged, "not json");
    }

    #[tokio::test]
    async fn status_reports_not_installed_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());

        let response = get_attention_hook_status(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["installed"], false);
    }

    #[tokio::test]
    async fn status_reports_installed_after_install() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());
        let _ = install_attention_hook(State(state.clone())).await.into_response();

        let response = get_attention_hook_status(State(state)).await.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["installed"], true);
    }

    #[tokio::test]
    async fn status_surfaces_malformed_json_error_without_failing() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("settings.local.json"), "not json").unwrap();

        let state = test_app_state_with_workspace(dir.path());
        let response = get_attention_hook_status(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["installed"], false);
        assert!(body["error"].as_str().unwrap().contains("couldn't parse"));
    }

    #[test]
    fn stable_claude_hook_script_path_lives_under_dot_orkworks() {
        let home = tempfile::tempdir().unwrap();
        let resolved = stable_claude_hook_script_path_under(Some(home.path().to_path_buf())).unwrap();
        assert_eq!(
            resolved,
            home.path().join(".orkworks").join("hook-scripts").join(CLAUDE_HOOK_SCRIPT_NAME),
        );
    }

    #[test]
    fn stable_claude_hook_script_path_none_without_home() {
        assert_eq!(stable_claude_hook_script_path_under(None), None);
    }

    #[test]
    fn ensure_stable_claude_hook_script_copies_and_makes_executable() {
        let source_dir = tempfile::tempdir().unwrap();
        let source_path = source_dir.path().join(CLAUDE_HOOK_SCRIPT_NAME);
        std::fs::write(&source_path, "#!/usr/bin/env bash\necho hi\n").unwrap();

        let home = tempfile::tempdir().unwrap();
        let stable_path = stable_claude_hook_script_path_under(Some(home.path().to_path_buf()));

        let result = ensure_stable_claude_hook_script_at(&source_path, stable_path.clone()).unwrap();

        assert_eq!(result, stable_path.unwrap());
        assert_eq!(std::fs::read_to_string(&result).unwrap(), "#!/usr/bin/env bash\necho hi\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&result).unwrap().permissions().mode();
            assert_ne!(mode & 0o111, 0, "expected the copied script to be executable");
        }
    }

    #[test]
    fn ensure_stable_claude_hook_script_self_heals_on_repeated_calls() {
        let source_dir = tempfile::tempdir().unwrap();
        let source_path = source_dir.path().join(CLAUDE_HOOK_SCRIPT_NAME);
        std::fs::write(&source_path, "v1").unwrap();

        let home = tempfile::tempdir().unwrap();
        let stable_path = stable_claude_hook_script_path_under(Some(home.path().to_path_buf()));

        ensure_stable_claude_hook_script_at(&source_path, stable_path.clone()).unwrap();
        // Simulate the "source" moving, as happens across an AppImage relaunch at a
        // different /tmp/.mount_* path or an app update — a fresh install should
        // still produce a working, up-to-date copy at the same stable location.
        std::fs::write(&source_path, "v2").unwrap();
        let result = ensure_stable_claude_hook_script_at(&source_path, stable_path).unwrap();

        assert_eq!(std::fs::read_to_string(&result).unwrap(), "v2");
    }

    #[tokio::test]
    async fn install_points_the_hook_command_at_the_stable_home_path_not_the_packaged_source() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let state = test_app_state_with_workspace(dir.path());

        let response = install_attention_hook(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let settings_path = dir.path().join(".claude").join("settings.local.json");
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
        let command = written["hooks"]["Notification"][0]["hooks"][0]["command"].as_str().unwrap();

        assert!(
            command.contains(home.path().to_str().unwrap()),
            "expected the installed command to reference the stable home-based path, got: {command}",
        );
        assert!(!command.contains(env!("CARGO_MANIFEST_DIR")), "must not persist the packaged/dev source path");
    }
}
