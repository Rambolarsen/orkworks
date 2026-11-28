use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const RETENTION_SCHEMA_VERSION: u8 = 2;
const MAX_RETENTION_DAYS: u32 = 999;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RetentionRequest {
    pub(crate) version: u8,
    pub(crate) auto_archive_after_days: u32,
    pub(crate) auto_archive_active_sessions: bool,
    pub(crate) delete_archived_after_days: u32,
}

pub(crate) async fn set_retention(
    State(state): State<Arc<AppState>>,
    axum::Json(req): axum::Json<RetentionRequest>,
) -> impl IntoResponse {
    if req.version != RETENTION_SCHEMA_VERSION
        || req.auto_archive_after_days > MAX_RETENTION_DAYS
        || req.delete_archived_after_days > MAX_RETENTION_DAYS
    {
        return (StatusCode::BAD_REQUEST, "Invalid retention settings").into_response();
    }

    let should_clear_warnings = {
        let mut config = state.retention_config.write().await;
        let prev_applied = config.applied;
        let prev_archive_days = config.auto_archive_after_days;
        config.version = req.version;
        config.auto_archive_after_days = req.auto_archive_after_days;
        config.auto_archive_active_sessions = req.auto_archive_active_sessions;
        config.delete_archived_after_days = req.delete_archived_after_days;
        config.applied = true;
        req.auto_archive_after_days == 0
            || !req.auto_archive_active_sessions
            || (prev_applied && req.auto_archive_after_days > prev_archive_days)
    };

    let cleared_warnings = if should_clear_warnings {
        let mut sessions = state.sessions.lock().unwrap();
        let mut cleared = false;
        for handle in sessions.values_mut() {
            if handle.runtime.clear_auto_archive_warning() {
                cleared = true;
            }
        }
        cleared
    } else {
        false
    };
    if cleared_warnings {
        state.updates.sessions_changed();
    }

    tracing::info!(
        version = req.version,
        auto_archive_after_days = req.auto_archive_after_days,
        auto_archive_active_sessions = req.auto_archive_active_sessions,
        delete_archived_after_days = req.delete_archived_after_days,
        cleared_warnings,
        "retention config updated"
    );
    StatusCode::OK.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_app_state_with_workspace, test_session_info};

    #[tokio::test]
    async fn set_retention_applies_valid_v2_config_and_clears_active_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        let now = chrono::Utc::now();
        runtime.set_auto_archive_warning(now, now + chrono::Duration::hours(24));
        state.sessions.lock().unwrap().insert(
            "warned".into(),
            crate::SessionHandle {
                info: test_session_info(
                    "warned",
                    "Warned",
                    dir.path().display().to_string(),
                    "running",
                    "2026-11-28T00:00:00Z",
                ),
                active_work_hook: false,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime,
                terminal_attached: false,
                resume_in_progress: false,
                capacity: crate::capacity_state::CapacityState::default(),
            },
        );

        let response = set_retention(
            State(state.clone()),
            axum::Json(RetentionRequest {
                version: 2,
                auto_archive_after_days: 14,
                auto_archive_active_sessions: false,
                delete_archived_after_days: 60,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let config = state.retention_config.read().await.clone();
        assert!(config.applied);
        assert_eq!(config.version, 2);
        assert_eq!(config.auto_archive_after_days, 14);
        assert!(!config.auto_archive_active_sessions);
        assert_eq!(config.delete_archived_after_days, 60);
        assert!(state
            .sessions
            .lock()
            .unwrap()
            .get("warned")
            .unwrap()
            .runtime
            .auto_archive_warning()
            .is_none());
    }

    #[tokio::test]
    async fn set_retention_keeps_warnings_when_only_delete_window_changes_and_clears_when_extending_archive_window(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let _ = set_retention(
            State(state.clone()),
            axum::Json(RetentionRequest {
                version: 2,
                auto_archive_after_days: 7,
                auto_archive_active_sessions: true,
                delete_archived_after_days: 30,
            }),
        )
        .await;
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        let now = chrono::Utc::now();
        runtime.set_auto_archive_warning(now, now + chrono::Duration::hours(24));
        state.sessions.lock().unwrap().insert(
            "warned".into(),
            crate::SessionHandle {
                info: test_session_info(
                    "warned",
                    "Warned",
                    dir.path().display().to_string(),
                    "running",
                    "2026-11-28T00:00:00Z",
                ),
                active_work_hook: false,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime,
                terminal_attached: false,
                resume_in_progress: false,
                capacity: crate::capacity_state::CapacityState::default(),
            },
        );

        // Changing only deleteArchivedAfterDays must not clear an active warning.
        let response = set_retention(
            State(state.clone()),
            axum::Json(RetentionRequest {
                version: 2,
                auto_archive_after_days: 7,
                auto_archive_active_sessions: true,
                delete_archived_after_days: 60,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(state
            .sessions
            .lock()
            .unwrap()
            .get("warned")
            .unwrap()
            .runtime
            .auto_archive_warning()
            .is_some());

        // Extending autoArchiveAfterDays clears active warnings.
        let response = set_retention(
            State(state.clone()),
            axum::Json(RetentionRequest {
                version: 2,
                auto_archive_after_days: 14,
                auto_archive_active_sessions: true,
                delete_archived_after_days: 60,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(state
            .sessions
            .lock()
            .unwrap()
            .get("warned")
            .unwrap()
            .runtime
            .auto_archive_warning()
            .is_none());
    }

    #[tokio::test]
    async fn set_retention_rejects_invalid_version_or_out_of_range_days() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        for bad in [
            RetentionRequest {
                version: 1,
                auto_archive_after_days: 7,
                auto_archive_active_sessions: true,
                delete_archived_after_days: 30,
            },
            RetentionRequest {
                version: 2,
                auto_archive_after_days: 1000,
                auto_archive_active_sessions: true,
                delete_archived_after_days: 30,
            },
            RetentionRequest {
                version: 2,
                auto_archive_after_days: 7,
                auto_archive_active_sessions: true,
                delete_archived_after_days: 1000,
            },
        ] {
            let response = set_retention(State(state.clone()), axum::Json(bad))
                .await
                .into_response();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert!(!state.retention_config.read().await.applied);
        }
    }
}
