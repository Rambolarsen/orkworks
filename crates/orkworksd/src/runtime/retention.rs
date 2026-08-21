use crate::AppState;
use std::sync::Arc;

pub(crate) fn delete_session_evidence(
    workspace: &crate::WorkspaceState,
    session_id: &str,
    mut delete_recommendations: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    workspace
        .workflow_observations
        .delete_session_observations(session_id)
        .map_err(|error| error.to_string())?;
    workspace
        .metadata
        .delete_events(session_id)
        .map_err(|error| error.to_string())?;
    workspace
        .metadata
        .clear_last_active_session_if_matches(session_id)
        .map_err(|error| error.to_string())?;
    workspace
        .metadata
        .delete_session(session_id)
        .map_err(|error| error.to_string())?;
    // Recommendation cleanup is last: if deleting the session metadata fails,
    // the session remains retryable and its evidence-backed recommendations
    // are preserved for that retry. Orphan cleanup handles the inverse case.
    if let Err(first_error) = delete_recommendations(session_id) {
        delete_recommendations(session_id).map_err(|retry_error| {
            format!("{first_error}; recommendation cleanup retry failed: {retry_error}")
        })?;
    }
    Ok(())
}

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

    let mut candidates: Vec<_> = all_sessions
        .into_iter()
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
            let mut deleted_expired = Vec::new();
            let ws_guard = state.workspace.lock().unwrap();
            if let Some(ref ws) = *ws_guard {
                let mut sessions = state.sessions.lock().unwrap();
                for id in &expired {
                    if sessions.get(id).is_some_and(|h| {
                        h.info.status == "live"
                            || h.info.status == "creating"
                            || h.info.status == "running"
                    }) {
                        continue;
                    }
                    tracing::info!(session_id = %id, "retention: deleting expired session");
                    if let Err(error) = delete_session_evidence(ws, id, |session_id| {
                        ws.recommendation_store
                            .delete_referencing_session(session_id)
                            .map_err(|error| error.to_string())
                    }) {
                        tracing::error!(session_id = %id, %error, "retention: failed to delete expired session");
                        if !ws.metadata.session_file_exists(id) {
                            sessions.remove(id);
                            deleted_expired.push(id.clone());
                        }
                        continue;
                    }
                    sessions.remove(id);
                    deleted_expired.push(id.clone());
                }
            }
            all_deleted.extend(deleted_expired.iter().cloned());
            candidates.retain(|s| !deleted_expired.contains(&s.id));
        }
    }

    if config.max_sessions > 0 && candidates.len() > config.max_sessions {
        let to_delete = candidates.len() - config.max_sessions;
        let ws_guard = state.workspace.lock().unwrap();
        if let Some(ref ws) = *ws_guard {
            let mut sessions = state.sessions.lock().unwrap();
            let eligible: Vec<_> = candidates
                .iter()
                .filter(|s| {
                    !sessions.get(&s.id).is_some_and(|h| {
                        h.info.status == "live"
                            || h.info.status == "creating"
                            || h.info.status == "running"
                    })
                })
                .take(to_delete)
                .collect();
            for s in eligible {
                if sessions.get(&s.id).is_some_and(|h| {
                    h.info.status == "live"
                        || h.info.status == "creating"
                        || h.info.status == "running"
                }) {
                    continue;
                }
                tracing::info!(
                    session_id = %s.id,
                    max_sessions = config.max_sessions,
                    "retention: deleting session (exceeds max)"
                );
                if let Err(error) = delete_session_evidence(ws, &s.id, |session_id| {
                    ws.recommendation_store
                        .delete_referencing_session(session_id)
                        .map_err(|error| error.to_string())
                }) {
                    tracing::error!(session_id = %s.id, %error, "retention: failed to delete session");
                    if !ws.metadata.session_file_exists(&s.id) {
                        sessions.remove(&s.id);
                        all_deleted.push(s.id.clone());
                    }
                    continue;
                }
                sessions.remove(&s.id);
                all_deleted.push(s.id.clone());
            }
        }
    }

    if !all_deleted.is_empty() {
        let mut peon_output = state.peon.last_output.write().unwrap();
        let mut peon_inference = state.peon.last_inference.write().unwrap();
        for id in &all_deleted {
            peon_output.remove(id);
            peon_inference.remove(id);
        }
        drop(peon_inference);
        drop(peon_output);
        for id in &all_deleted {
            crate::runtime::session_runtime::clear_forgotten_session_tracking(state, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata;
    use crate::test_support::*;
    use crate::workflow_observations::{
        Impact, ObservationCandidate, ObservationKind, ObservationOrigin,
    };

    fn record_test_observation(
        store: &crate::workflow_observations::WorkflowObservationStore,
        session_id: &str,
        key: &str,
    ) {
        store
            .record_observation(
                session_id,
                ObservationOrigin::Agent,
                key,
                ObservationCandidate {
                    kind: ObservationKind::VerificationGap,
                    description: "test observation".into(),
                    evidence: "test evidence".into(),
                    reported_impact: Impact::Low,
                    confidence: None,
                },
            )
            .unwrap();
    }

    #[test]
    fn delete_session_evidence_removes_only_one_session_and_preserves_sequence() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        for id in ["session-a", "session-b"] {
            ws.metadata.write_session(&test_session_metadata(
                id,
                id,
                dir.path().display().to_string(),
                "ended",
                "2024-01-01T00:00:00Z",
                "2024-01-01T00:00:00Z",
            ));
            ws.metadata
                .append_terminal_output_lines(id, &["terminal".into()]);
            record_test_observation(&ws.workflow_observations, id, id);
        }
        ws.metadata.write_workspace_memory(&metadata::WorkspaceMemory {
            last_active_session_id: Some("session-a".into()),
            last_active_at: Some("2024-01-01T00:00:00Z".into()),
            active_harness_ids: vec![],
        });

        delete_session_evidence(ws, "session-a", |_| Ok(())).unwrap();

        assert!(!ws.metadata.session_file_exists("session-a"));
        assert!(ws.metadata.read_session("session-b").is_some());
        assert!(ws.metadata.read_terminal_output("session-a", 10).is_empty());
        assert_eq!(
            ws.workflow_observations
                .workspace_observations()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            ws.workflow_observations.workspace_observations().unwrap()[0].session_id,
            "session-b"
        );
        record_test_observation(&ws.workflow_observations, "session-b", "after-delete");
        assert_eq!(
            ws.workflow_observations
                .workspace_observations()
                .unwrap()[1]
                .sequence,
            3
        );
    }

    #[test]
    fn delete_session_evidence_keeps_recommendations_retryable_when_cleanup_fails() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        ws.metadata.write_session(&test_session_metadata(
            "session-a",
            "session-a",
            dir.path().display().to_string(),
            "ended",
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00Z",
        ));
        record_test_observation(&ws.workflow_observations, "session-a", "session-a");

        let result = delete_session_evidence(ws, "session-a", |_| {
            Err("recommendations failed".into())
        });

        assert_eq!(
            result.unwrap_err(),
            "recommendations failed; recommendation cleanup retry failed: recommendations failed"
        );
        // The metadata deletion completed before the failing final cleanup;
        // the recommendation remains available for orphan cleanup/retry, and
        // the session is no longer intact.
        assert!(!ws.metadata.session_file_exists("session-a"));
        assert_eq!(
            ws.workflow_observations
                .workspace_observations()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn delete_session_evidence_reports_observation_cleanup_failure() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        ws.metadata.write_session(&test_session_metadata(
            "session-a",
            "session-a",
            dir.path().display().to_string(),
            "ended",
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00Z",
        ));
        record_test_observation(&ws.workflow_observations, "session-a", "session-a");
        let segment_path = dir
            .path()
            .join(".orkworks-test/workflow-observations/session-a.ndjson");
        std::fs::remove_file(&segment_path).unwrap();
        std::fs::create_dir(&segment_path).unwrap();

        let result = delete_session_evidence(ws, "session-a", |_| Ok(()));

        assert!(result.is_err());
        assert!(ws.metadata.session_file_exists("session-a"));
    }

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
            ws.metadata
                .write_workspace_memory(&metadata::WorkspaceMemory {
                    last_active_session_id: Some(session_id.clone()),
                    last_active_at: Some("2024-01-01T00:00:00Z".into()),
                    active_harness_ids: vec![],
            });
        }

        state
            .peon
            .label_epochs
            .write()
            .unwrap()
            .insert(session_id.clone(), 2);

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
        assert!(!state
            .peon
            .label_epochs
            .read()
            .unwrap()
            .contains_key(&session_id));
    }
}
