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

/// Locates the Claude hook reporter script: a packaged app ships it as a sibling
/// of the sidecar binary (`<resourcesPath>/scripts/...`); a dev checkout falls
/// back to its source location under this crate.
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

    let command = format!("\"{}\"", claude_hook_script_path().display());
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
}
