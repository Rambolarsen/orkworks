//! Exact-row organization commands. Admission and publication are synchronous;
//! runtime shutdown is awaited without holding application/store locks.
use super::{SessionApplication, SessionError};
use crate::{
    session_archive::{OrganizationStore, SessionState},
    SessionHandle,
};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionAction {
    Close,
    Archive,
    Restore,
    Delete,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionActionRequest {
    pub(crate) action: SessionAction,
    pub(crate) token: String,
}

pub(crate) fn runtime_key(handle: Option<&SessionHandle>) -> String {
    handle.map_or_else(
        || "remembered".into(),
        |h| {
            format!(
                "{}:{}:{}:{}:{}",
                h.runtime.runtime_instance_id,
                h.runtime.run_generation(),
                h.info.lifecycle_phase,
                h.resume_in_progress,
                h.terminal_attached
            )
        },
    )
}

pub(crate) fn runtime_closed(handle: Option<&SessionHandle>, has_pid: bool) -> bool {
    !has_pid
        && handle.is_none_or(|h| {
            h.info.lifecycle_phase == "ended"
                && h.info.lifecycle == "dead"
                && !h.resume_in_progress
                && !h.terminal_attached
                && h.runtime.commands_closed()
        })
}

struct CapturedAction {
    store: Arc<OrganizationStore>,
    created_at: String,
    revision: u64,
    generation: Option<u64>,
}

impl SessionApplication {
    pub(crate) async fn session_action(
        &self,
        id: &str,
        request: SessionActionRequest,
    ) -> Result<(), SessionError> {
        self.session_action_with_timeout(id, request, Duration::from_secs(15))
            .await
    }

    async fn session_action_with_timeout(
        &self,
        id: &str,
        request: SessionActionRequest,
        wait: Duration,
    ) -> Result<(), SessionError> {
        let (captured, _reservation, kill) =
            {
                let _projection = self.state.projection_lock.lock().unwrap();
                let workspace = self.state.workspace.lock().unwrap();
                let ws = workspace.as_ref().ok_or(SessionError::Conflict)?;
                let meta = ws
                    .metadata
                    .try_read_revisioned_session(id)
                    .map_err(|_| SessionError::ConflictWithMessage("Session metadata unavailable"))?
                    .ok_or(SessionError::NotFound)?
                    .metadata;
                let sessions = self.state.sessions.lock().unwrap();
                let handle = sessions.get(id);
                let has_pid = self.state.session_pids.lock().unwrap().contains_key(id);
                let closed = meta.lifecycle_phase == "ended"
                    && meta.lifecycle == "dead"
                    && runtime_closed(handle, has_pid);
                let projected = ws.metadata.organization.project(
                    id,
                    &meta.created_at,
                    closed,
                    !runtime_closed(handle, has_pid),
                    &runtime_key(handle),
                );
                if projected.session_action_token.as_deref() != Some(request.token.as_str()) {
                    return Err(SessionError::ConflictWithMessage(
                        "Session changed; refresh and try again",
                    ));
                }
                let (current, revision) = ws
                    .metadata
                    .organization
                    .state(
                        id,
                        &meta.created_at,
                        closed,
                        !runtime_closed(handle, has_pid),
                    )
                    .map_err(|_| SessionError::ConflictWithMessage("Session state unavailable"))?;
                match (current, request.action) {
                    (SessionState::Archived, SessionAction::Restore | SessionAction::Delete) => {}
                    (SessionState::Closed, SessionAction::Archive | SessionAction::Delete) => {}
                    (
                        SessionState::Active,
                        SessionAction::Close | SessionAction::Archive | SessionAction::Delete,
                    ) if handle.is_some() => {}
                    _ => {
                        return Err(SessionError::ConflictWithMessage(
                            "Action is unavailable in this session state",
                        ))
                    }
                }
                let reservation = ws.metadata.organization.reserve(id).ok_or(
                    SessionError::ConflictWithMessage("A session action is already running"),
                )?;
                let kill = if current == SessionState::Active {
                    let handle = handle.unwrap();
                    // Send under the same admission guard that validated the token.
                    handle.kill_tx.send_replace(true);
                    Some(handle.runtime.run_generation())
                } else {
                    None
                };
                (
                    CapturedAction {
                        store: ws.metadata.organization.clone(),
                        created_at: meta.created_at,
                        revision,
                        generation: handle.map(|h| h.runtime.run_generation()),
                    },
                    reservation,
                    kill,
                )
            };
        if kill.is_some() {
            // The runtime driver owns finalization. A request to stop is not proof
            // that it has drained output, cleaned up, and persisted its ending.
            let deadline = tokio::time::Instant::now() + wait;
            loop {
                if tokio::time::Instant::now() >= deadline {
                    return Err(SessionError::ConflictWithMessage(
                        "Close is still finishing; refresh and try again",
                    ));
                }
                if self.finish_session_action(id, &captured, request.action, Some(deadline))? {
                    return Ok(());
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(SessionError::ConflictWithMessage(
                        "Close is still finishing; refresh and try again",
                    ));
                }
                // State-based bounded wait also covers a control receiver closing
                // after the finalizer publishes ended. No lock survives this await.
                tokio::time::sleep(
                    Duration::from_millis(25)
                        .min(deadline.saturating_duration_since(tokio::time::Instant::now())),
                )
                .await;
            }
        }
        if self.finish_session_action(id, &captured, request.action, None)? {
            Ok(())
        } else {
            Err(SessionError::ConflictWithMessage(
                "Session is still closing",
            ))
        }
    }

    fn finish_session_action(
        &self,
        id: &str,
        captured: &CapturedAction,
        action: SessionAction,
        deadline: Option<tokio::time::Instant>,
    ) -> Result<bool, SessionError> {
        let _projection = self.state.projection_lock.lock().unwrap();
        let workspace = self.state.workspace.lock().unwrap();
        let ws = workspace.as_ref().ok_or(SessionError::Conflict)?;
        if !Arc::ptr_eq(&captured.store, &ws.metadata.organization) {
            return Err(SessionError::Conflict);
        }
        let meta = ws
            .metadata
            .try_read_revisioned_session(id)
            .map_err(|_| SessionError::ConflictWithMessage("Session metadata unavailable"))?
            .ok_or(SessionError::NotFound)?
            .metadata;
        if meta.created_at != captured.created_at {
            return Err(SessionError::Conflict);
        }
        let mut sessions = self.state.sessions.lock().unwrap();
        let handle = sessions.get(id);
        if handle.map(|h| h.runtime.run_generation()) != captured.generation {
            return Err(SessionError::Conflict);
        }
        let (_, revision) = ws
            .metadata
            .organization
            .state(id, &meta.created_at, true, false)
            .map_err(|_| SessionError::Conflict)?;
        if revision != captured.revision {
            return Err(SessionError::Conflict);
        }
        if meta.lifecycle_phase != "ended"
            || meta.lifecycle != "dead"
            || !runtime_closed(
                handle,
                self.state.session_pids.lock().unwrap().contains_key(id),
            )
        {
            return Ok(false);
        }
        // Recheck after acquiring locks and reading state: waiting for either
        // may have exhausted the close-first budget.
        if deadline.is_some_and(|deadline| tokio::time::Instant::now() >= deadline) {
            return Err(SessionError::ConflictWithMessage(
                "Close is still finishing; refresh and try again",
            ));
        }
        let mut deleted = false;
        let mut deletion_failed = false;
        match action {
            SessionAction::Close => {}
            SessionAction::Archive | SessionAction::Restore => ws
                .metadata
                .organization
                .write(
                    id,
                    &meta.created_at,
                    if action == SessionAction::Archive {
                        SessionState::Archived
                    } else {
                        SessionState::Closed
                    },
                )
                .map_err(|_| SessionError::Internal("Could not save session state"))?,
            SessionAction::Delete => {
                let result = crate::runtime::retention::delete_session_evidence(ws, id, |id| {
                    ws.recommendation_store
                        .delete_referencing_session(id)
                        .map_err(|e| e.to_string())
                });
                // Evidence cleanup can fail after metadata has already been
                // removed. Match the legacy deletion owner's partial-failure
                // behavior: do not let the live mirror resurrect that record.
                deleted = result.is_ok() || !ws.metadata.session_file_exists(id);
                if deleted {
                    sessions.remove(id);
                }
                if let Err(error) = result {
                    tracing::error!(session_id = %id, %error, "session action: evidence cleanup failed");
                    deletion_failed = true;
                }
            }
        }
        drop(sessions);
        drop(workspace);
        if deleted {
            self.clear_ended_session_tracking(id);
            self.clear_forgotten_session_tracking(id);
        }
        if deletion_failed {
            return Err(SessionError::Internal("Could not delete session evidence"));
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        metadata,
        session_projection::SessionProjection,
        test_support::{test_app_state_with_workspace, test_session_info, test_session_metadata},
    };
    fn request(state: &Arc<crate::AppState>, action: SessionAction) -> SessionActionRequest {
        let info = SessionProjection::new(state.clone())
            .list()
            .unwrap()
            .remove(0);
        SessionActionRequest {
            action,
            token: info.organization.session_action_token.unwrap(),
        }
    }
    fn fixture(
        active: bool,
    ) -> (
        tempfile::TempDir,
        Arc<crate::AppState>,
        tokio::sync::watch::Receiver<bool>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let status = if active { "running" } else { "ended" };
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&test_session_metadata(
                "one",
                "One",
                dir.path().display().to_string(),
                status,
                "2024-01-01T00:00:00Z",
                "2024-01-01T00:00:00Z",
            ));
        let (kill_tx, kill_rx) = tokio::sync::watch::channel(false);
        if active {
            state.sessions.lock().unwrap().insert(
                "one".into(),
                SessionHandle {
                    info: test_session_info(
                        "one",
                        "One",
                        dir.path().display().to_string(),
                        "running",
                        "2024-01-01T00:00:00Z",
                    ),
                    kill_tx,
                    output_buffer: crate::peon::RingBuffer::new(200),
                    scan_buf: String::new(),
                    pending_work_signal: None,
                    runtime: crate::runtime::session_runtime::SessionRuntime::detached_test(),
                    terminal_attached: false,
                    resume_in_progress: false,
                    capacity: Default::default(),
                    active_work_hook: false,
                },
            );
        }
        (dir, state, kill_rx)
    }
    #[tokio::test]
    async fn archive_restore_preserve_activity_and_reject_stale_actions() {
        let (_dir, state, _) = fixture(false);
        let app = SessionApplication::new(state.clone());
        let archive = request(&state, SessionAction::Archive);
        let stale = SessionActionRequest {
            action: SessionAction::Delete,
            token: archive.token.clone(),
        };
        app.session_action("one", archive).await.unwrap();
        assert!(app.session_action("one", stale).await.is_err());
        let info = SessionProjection::new(state.clone())
            .list()
            .unwrap()
            .remove(0);
        assert_eq!(
            info.organization.session_state,
            Some(SessionState::Archived)
        );
        assert!(app.resume_session("one", None).await.is_err());
        app.session_action("one", request(&state, SessionAction::Restore))
            .await
            .unwrap();
        let workspace = state.workspace.lock().unwrap();
        let ws = workspace.as_ref().unwrap();
        let meta = ws.metadata.read_session("one").unwrap();
        assert_eq!(meta.last_activity, "2024-01-01T00:00:00Z");
        assert_eq!(
            ws.metadata
                .organization
                .state("one", &meta.created_at, true, false)
                .unwrap()
                .0,
            SessionState::Closed
        );
        assert!(!state.sessions.lock().unwrap().contains_key("one"));
    }
    async fn archived_with_regressed_metadata() -> (tempfile::TempDir, Arc<crate::AppState>) {
        let (dir, state, _) = fixture(false);
        SessionApplication::new(state.clone())
            .session_action("one", request(&state, SessionAction::Archive))
            .await
            .unwrap();
        {
            let workspace = state.workspace.lock().unwrap();
            let ws = workspace.as_ref().unwrap();
            let mut meta = ws.metadata.read_session("one").unwrap();
            meta.status = "running".into();
            meta.lifecycle_phase = "alive".into();
            meta.lifecycle = "alive".into();
            ws.metadata.write_session(&meta);
        }
        let info = SessionProjection::new(state.clone())
            .list()
            .unwrap()
            .remove(0);
        assert_eq!(
            info.organization.session_state,
            Some(SessionState::Archived)
        );
        (dir, state)
    }

    fn assert_regressed_archive_unchanged(state: &Arc<crate::AppState>) {
        let workspace = state.workspace.lock().unwrap();
        let ws = workspace.as_ref().unwrap();
        let meta = ws.metadata.read_session("one").unwrap();
        assert_eq!(meta.lifecycle_phase, "alive");
        assert_eq!(meta.last_activity, "2024-01-01T00:00:00Z");
        assert_eq!(
            ws.metadata
                .organization
                .state("one", &meta.created_at, false, false)
                .unwrap()
                .0,
            SessionState::Archived
        );
    }

    fn restore_persisted_ending(state: &Arc<crate::AppState>) {
        let workspace = state.workspace.lock().unwrap();
        let ws = workspace.as_ref().unwrap();
        let mut meta = ws.metadata.read_session("one").unwrap();
        meta.status = "ended".into();
        meta.lifecycle_phase = "ended".into();
        meta.lifecycle = "dead".into();
        ws.metadata.write_session(&meta);
    }

    #[tokio::test]
    async fn archived_actions_require_current_persisted_finalization_after_producer_regression() {
        for action in [SessionAction::Restore, SessionAction::Delete] {
            let (_dir, state) = archived_with_regressed_metadata().await;
            let app = SessionApplication::new(state.clone());
            assert!(app
                .session_action("one", request(&state, action))
                .await
                .is_err());
            assert_regressed_archive_unchanged(&state);
            restore_persisted_ending(&state);
            app.session_action("one", request(&state, action))
                .await
                .unwrap();
            let infos = SessionProjection::new(state.clone()).list().unwrap();
            if action == SessionAction::Restore {
                assert_eq!(
                    infos[0].organization.session_state,
                    Some(SessionState::Closed)
                );
                assert!(!state.sessions.lock().unwrap().contains_key("one"));
                app.session_action("one", request(&state, SessionAction::Archive))
                    .await
                    .unwrap();
                app.forget_session("one").await.unwrap();
            } else {
                assert!(infos.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn archived_forget_requires_current_persisted_finalization_after_producer_regression() {
        let (_dir, state) = archived_with_regressed_metadata().await;
        assert!(SessionApplication::new(state.clone())
            .forget_session("one")
            .await
            .is_err());
        assert_regressed_archive_unchanged(&state);
        restore_persisted_ending(&state);
        SessionApplication::new(state.clone())
            .forget_session("one")
            .await
            .unwrap();
        assert!(SessionProjection::new(state).list().unwrap().is_empty());
    }

    #[tokio::test]
    async fn forget_preserves_unreadable_metadata() {
        for directory in [false, true] {
            let (_dir, state, _) = fixture(false);
            let root = state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .root_path();
            let path = root.join("sessions/one.json");
            if directory {
                std::fs::remove_file(&path).unwrap();
                std::fs::create_dir(&path).unwrap();
            } else {
                std::fs::write(&path, "{malformed").unwrap();
            }
            let result = SessionApplication::new(state).forget_session("one").await;
            assert!(result.is_err(), "unknown finalization must block forget");
            assert!(path.exists());
            if !directory {
                assert_eq!(std::fs::read_to_string(path).unwrap(), "{malformed");
            }
        }
    }

    #[test]
    fn expired_followup_cannot_archive_or_delete_after_waiting_for_guard() {
        for action in [SessionAction::Archive, SessionAction::Delete] {
            let (_dir, state, _) = fixture(false);
            let captured = CapturedAction {
                store: state
                    .workspace
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .metadata
                    .organization
                    .clone(),
                created_at: "2024-01-01T00:00:00Z".into(),
                revision: 0,
                generation: None,
            };
            let guard = state.projection_lock.lock().unwrap();
            let deadline = tokio::time::Instant::now() + Duration::from_millis(20);
            let app = SessionApplication::new(state.clone());
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let thread = std::thread::spawn(move || {
                started_tx.send(()).unwrap();
                app.finish_session_action("one", &captured, action, Some(deadline))
            });
            started_rx.recv().unwrap();
            std::thread::sleep(Duration::from_millis(40));
            drop(guard);
            assert!(
                thread.join().unwrap().is_err(),
                "expired guard acquisition must not mutate"
            );
            let infos = SessionProjection::new(state).list().unwrap();
            assert_eq!(infos.len(), 1);
            assert_eq!(
                infos[0].organization.session_state,
                Some(SessionState::Closed)
            );
        }
    }

    #[tokio::test]
    async fn active_archive_waits_for_persisted_finalization() {
        let (_dir, state, mut killed) = fixture(true);
        let req = request(&state, SessionAction::Archive);
        let operation_state = state.clone();
        let operation = tokio::spawn(async move {
            SessionApplication::new(operation_state)
                .session_action("one", req)
                .await
        });
        killed.changed().await.unwrap();
        assert!(!operation.is_finished());
        {
            let ws = state.workspace.lock().unwrap();
            assert!(!ws
                .as_ref()
                .unwrap()
                .metadata
                .root_path()
                .join("session-state/one.json")
                .exists());
        }
        let generation = state.sessions.lock().unwrap()["one"]
            .runtime
            .run_generation();
        crate::runtime::terminal_runtime::set_session_status_for_generation(
            &state, "one", generation, "killed",
        )
        .await;
        assert!(!operation.is_finished());
        assert!(
            SessionApplication::new(state.clone()).complete_session_ending(
                "one",
                generation,
                metadata::ObservedStatusSnapshotMetadata {
                    value: None,
                    source: "process".into(),
                    confidence: None,
                    observed_at: None,
                    attention_origin: None,
                },
                "killed"
            )
        );
        operation.await.unwrap().unwrap();
        assert_eq!(
            SessionProjection::new(state).list().unwrap()[0]
                .organization
                .session_state,
            Some(SessionState::Archived)
        );
    }
    #[tokio::test]
    async fn timeout_and_direct_forget_preserve_unfinished_runtime_evidence() {
        let (_dir, state, mut killed) = fixture(true);
        let app = SessionApplication::new(state.clone());
        let req = request(&state, SessionAction::Delete);
        assert!(app
            .session_action_with_timeout("one", req, Duration::ZERO)
            .await
            .is_err());
        killed.changed().await.unwrap();
        assert!(*killed.borrow());
        let generation = state.sessions.lock().unwrap()["one"]
            .runtime
            .run_generation();
        crate::runtime::terminal_runtime::set_session_status_for_generation(
            &state, "one", generation, "killed",
        )
        .await;
        assert!(app.forget_session("one").await.is_err());
        let ws = state.workspace.lock().unwrap();
        let ws = ws.as_ref().unwrap();
        assert!(ws.metadata.session_file_exists("one"));
        assert!(!ws
            .metadata
            .root_path()
            .join("session-state/one.json")
            .exists());
        assert!(!ws.metadata.organization.pending("one"));
    }
    #[tokio::test]
    async fn partial_delete_failure_cannot_resurrect_a_handle_after_metadata_removal() {
        let (_dir, state, _) = fixture(false);
        let (_other_dir, other, _) = fixture(true);
        let mut ended = other.sessions.lock().unwrap().remove("one").unwrap();
        ended.info.status = "ended".into();
        ended.info.lifecycle = "dead".into();
        ended.info.lifecycle_phase = "ended".into();
        state.sessions.lock().unwrap().insert("one".into(), ended);
        let request = request(&state, SessionAction::Delete);
        {
            let workspace = state.workspace.lock().unwrap();
            let root = workspace
                .as_ref()
                .unwrap()
                .metadata
                .root_path()
                .join("recommendations");
            std::fs::remove_dir_all(&root).unwrap();
            std::fs::write(root, "unavailable recommendation store").unwrap();
        }
        assert!(SessionApplication::new(state.clone())
            .session_action("one", request)
            .await
            .is_err());
        assert!(!state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .session_file_exists("one"));
        assert!(!state.sessions.lock().unwrap().contains_key("one"));
        assert!(SessionProjection::new(state).list().unwrap().is_empty());
    }

    #[tokio::test]
    async fn stale_runtime_token_cannot_stop_a_replacement_generation() {
        let (_dir, state, _) = fixture(true);
        let stale = request(&state, SessionAction::Archive);
        let (new_kill, new_receiver) = tokio::sync::watch::channel(false);
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut("one").unwrap();
            handle.runtime = crate::runtime::session_runtime::SessionRuntime::detached_test();
            handle.kill_tx = new_kill;
        }
        assert!(SessionApplication::new(state)
            .session_action("one", stale)
            .await
            .is_err());
        assert!(!*new_receiver.borrow());
    }
    #[tokio::test]
    async fn cancelled_action_releases_reservation_without_archiving_later() {
        let (_dir, state, mut killed) = fixture(true);
        let req = request(&state, SessionAction::Archive);
        let task_state = state.clone();
        let operation = tokio::spawn(async move {
            SessionApplication::new(task_state)
                .session_action("one", req)
                .await
        });
        killed.changed().await.unwrap();
        let duplicate = request(&state, SessionAction::Delete);
        assert!(SessionApplication::new(state.clone())
            .session_action("one", duplicate)
            .await
            .is_err());
        operation.abort();
        assert!(operation.await.unwrap_err().is_cancelled());
        let ws = state.workspace.lock().unwrap();
        let ws = ws.as_ref().unwrap();
        assert!(!ws.metadata.organization.pending("one"));
        assert!(!ws
            .metadata
            .root_path()
            .join("session-state/one.json")
            .exists());
    }
    #[test]
    fn archived_resume_is_rejected_at_runtime_admission() {
        let (_dir, state, _) = fixture(false);
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .organization
                .write("one", "2024-01-01T00:00:00Z", SessionState::Archived)
                .unwrap();
        }
        let (_other_dir, other, _) = fixture(true);
        let replacement = other.sessions.lock().unwrap().remove("one").unwrap();
        assert!(super::super::try_install_claimed_resume_handle(
            &state,
            "one",
            replacement,
            true,
            None
        )
        .is_err());
        assert!(state.sessions.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn stale_workspace_token_never_stops_current_runtime() {
        let (dir, state, killed) = fixture(true);
        let req = request(&state, SessionAction::Delete);
        crate::test_support::swap_workspace(&state, dir.path());
        assert!(SessionApplication::new(state.clone())
            .session_action("one", req)
            .await
            .is_err());
        assert!(!*killed.borrow());
    }
}
