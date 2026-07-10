use crate::{metadata, AppState};
use std::sync::Arc;

pub(crate) async fn retention_cleanup_task(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        retention_cleanup_once(&state, chrono::Utc::now()).await;
    }
}

pub(crate) async fn retention_cleanup_once(
    state: &Arc<AppState>,
    now: chrono::DateTime<chrono::Utc>,
) {
    let config = state.retention_config.read().await.clone();
    if config.max_sessions == 0 && config.max_age_days == 0 {
        return;
    }

    let all_sessions = {
        let ws_guard = state.workspace.lock().unwrap();
        match &*ws_guard {
            Some(ws) => ws.metadata.read_all_sessions(),
            None => return,
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
        return;
    }

    candidates.sort_by(|a, b| a.last_activity.cmp(&b.last_activity));

    let mut all_deleted: Vec<String> = Vec::new();

    if config.max_age_days > 0 {
        let cutoff = now - chrono::Duration::days(config.max_age_days as i64);
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
        let mut peon_processed_output = state.peon.last_processed_output.write().unwrap();
        let mut peon_inference = state.peon.last_inference.write().unwrap();
        for id in &all_deleted {
            sessions.remove(id);
            peon_output.remove(id);
            peon_processed_output.remove(id);
            peon_inference.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[tokio::test]
    async fn retention_cleanup_keeps_live_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "still-live".to_string();

        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&test_session_metadata(
                session_id.clone(),
                "Still Live",
                dir.path().display().to_string(),
                "ended",
                "2024-01-01T00:00:00Z",
                "2024-01-01T00:00:00Z",
            ));
        }

        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.clone(),
            crate::SessionHandle {
                info: test_session_info(
                    session_id.clone(),
                    "Still Live",
                    dir.path().display().to_string(),
                    "running",
                    "2024-01-01T00:00:00Z",
                ),
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: crate::harness_registry::default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
                runtime: crate::runtime::session_runtime::SessionRuntime::detached(crate::runtime::session_runtime::DEFAULT_TERMINAL_ROWS, crate::runtime::session_runtime::DEFAULT_TERMINAL_COLS),
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
            },
        );

        {
            let mut config = state.retention_config.write().await;
            config.max_age_days = 1;
            config.max_sessions = 0;
        }

        retention_cleanup_once(&state, chrono::Utc::now()).await;

        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        assert!(ws.metadata.read_session(&session_id).is_some());
    }

    #[tokio::test]
    async fn retention_cleanup_clears_last_active_when_session_is_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let session_id = "old-session".to_string();

        {
            let ws_guard = state.workspace.lock().unwrap();
            let ws = ws_guard.as_ref().unwrap();
            ws.metadata.write_session(&test_session_metadata(
                session_id.clone(),
                "Old Session",
                dir.path().display().to_string(),
                "ended",
                "2024-01-01T00:00:00Z",
                "2024-01-01T00:00:00Z",
            ));
            ws.metadata.write_workspace_memory(&metadata::WorkspaceMemory {
                last_active_session_id: Some(session_id.clone()),
                last_active_at: Some("2024-01-01T00:00:00Z".into()),
                active_harness_ids: vec![],
            });
        }

        {
            let mut config = state.retention_config.write().await;
            config.max_age_days = 1;
            config.max_sessions = 0;
        }

        let now = chrono::DateTime::parse_from_rfc3339("2024-02-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        retention_cleanup_once(&state, now).await;

        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        assert!(ws.metadata.read_session(&session_id).is_none());
        let memory = ws.metadata.read_workspace_memory().unwrap();
        assert_eq!(memory.last_active_session_id, None);
        assert_eq!(memory.last_active_at, None);
    }
}
