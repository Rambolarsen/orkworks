use super::*;
use crate::runtime::session_runtime::{RuntimeCommand, SessionRuntime};
use crate::session_application::session_actions::SessionAction;
use crate::session_archive::SessionState;
use std::time::Duration;
const ID: &str = "organization-runtime";
const CREATED: &str = "2026-01-01T00:00:00Z";

async fn fixture() -> (
    tempfile::TempDir,
    Arc<AppState>,
    Arc<SessionStore>,
    Arc<SessionWrites>,
    tokio::sync::watch::Receiver<bool>,
    tokio::sync::mpsc::Receiver<RuntimeCommand>,
) {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let (root, path) = {
        let mut workspace = state.workspace.lock().unwrap();
        let ws = workspace.as_mut().unwrap();
        ws.lease = Some(Arc::new(
            WorkspaceLease::acquire(&ws.metadata.root_path()).unwrap(),
        ));
        (
            ws.metadata.root_path(),
            ws.path.to_string_lossy().into_owned(),
        )
    };
    let (runtime, commands) = SessionRuntime::live(24, 80);
    let (kill_tx, killed) = tokio::sync::watch::channel(false);
    let mut info =
        crate::test_support::test_session_info(ID, "Before", path.clone(), "running", CREATED);
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    state.sessions.lock().unwrap().insert(
        ID.into(),
        crate::SessionHandle {
            info,
            kill_tx,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
            active_work_hook: false,
        },
    );
    let store = Arc::new(SessionStore::initialize(root, path.clone()).await.unwrap());
    let mut metadata = crate::test_support::test_session_metadata(
        ID,
        "Before",
        path,
        "running",
        CREATED,
        "2026-01-02T00:00:00Z",
    );
    metadata.lifecycle = "alive".into();
    metadata.lifecycle_phase = "active".into();
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .try_write_session(&metadata)
        .unwrap();
    store
        .create(serde_json::to_value(metadata).unwrap())
        .await
        .unwrap();
    let writes = SessionWrites::bind(state.clone(), store.clone()).unwrap();
    (dir, state, store, writes, killed, commands)
}
// Model the runtime driver's finalization with the real worker lifecycle
// reducers and the same session lane. Organization tests control the live
// cleanup flags; unrelated prompt-authority publication is qualified elsewhere.
async fn finalize(state: &Arc<AppState>, writes: &Arc<SessionWrites>) {
    use crate::metadata::{
        internal_mutations::InternalMutation, lifecycle_mutations::LifecycleMutation,
    };
    let identity = state.sessions.lock().unwrap()[ID].runtime.identity();
    let _lane = writes.store.runtime_lane(ID).unwrap().lock_owned().await;
    writes
        .store
        .mutate_internal(
            ID.into(),
            CREATED.into(),
            None,
            InternalMutation::Lifecycle(LifecycleMutation::Transition {
                status: "killed".into(),
                had_live_handle: true,
                already_ending: false,
                live_phase: Some("active".into()),
                resume: None,
                resumed_from: None,
                timestamp: "2026-01-03T00:00:00Z".into(),
            }),
        )
        .await
        .unwrap();
    let result = writes
        .store
        .mutate_internal(
            ID.into(),
            CREATED.into(),
            None,
            InternalMutation::Lifecycle(LifecycleMutation::Finalize {
                snapshot: metadata::ObservedStatusSnapshotMetadata {
                    value: None,
                    source: "process".into(),
                    confidence: None,
                    observed_at: None,
                    attention_origin: None,
                },
                fallback_status: "killed".into(),
                timestamp: "2026-01-03T00:00:01Z".into(),
            }),
        )
        .await
        .unwrap();
    let stored: metadata::SessionMetadata =
        serde_json::from_value(result.document.unwrap()).unwrap();
    let mut sessions = state.sessions.lock().unwrap();
    let h = sessions.get_mut(ID).unwrap();
    assert_eq!(h.runtime.identity(), identity);
    h.runtime.attention_owner.accepted_write();
    h.info.status = stored.status;
    h.info.lifecycle_phase = stored.lifecycle_phase;
    h.info.lifecycle = stored.lifecycle;
    h.info.attention = stored.attention;
    h.info.connectivity = Some(stored.connectivity);
    h.info.terminal_outcome = stored.terminal_outcome;
    h.info.observed_status = stored.observed_status;
    h.info.last_activity_at = Some(stored.last_activity);
    h.info.provenance = stored.provenance;
    h.resume_in_progress = false;
}

fn root(state: &Arc<AppState>) -> PathBuf {
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path()
}

#[tokio::test]
async fn sql_archive_waits_for_exact_runtime_cleanup_and_does_not_block_its_finalizer() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, mut killed, commands) = fixture().await;
    let legacy = std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap();
    let view = writes.capture_organization(ID.into()).await.unwrap();
    assert_eq!(view.state(), SessionState::Active);
    state.session_pids.lock().unwrap().insert(ID.into(), 123);
    // Capture again because the stale-action view includes current PID presence.
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let mut action = tokio::spawn(view.apply(SessionAction::Archive));
    killed.changed().await.unwrap();
    // The close-intent reservation must never take the finalizer's SQL lane.
    tokio::time::timeout(Duration::from_secs(2), finalize(&state, &writes))
        .await
        .unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(75), &mut action)
        .await
        .is_err());
    assert_eq!(
        store
            .organization_state(ID.into())
            .await
            .unwrap()
            .unwrap()
            .state,
        "closed"
    );
    state.session_pids.lock().unwrap().remove(ID);
    assert!(tokio::time::timeout(Duration::from_millis(75), &mut action)
        .await
        .is_err());
    drop(commands);
    let receipt = tokio::time::timeout(Duration::from_secs(2), action)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(receipt.revision, "1");
    assert!(matches!(receipt.outcome, Outcome::Committed));
    assert_eq!(
        store
            .organization_state(ID.into())
            .await
            .unwrap()
            .unwrap()
            .state,
        "archived"
    );
    assert_eq!(
        std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap(),
        legacy
    );
    assert!(!root(&state)
        .join(format!("session-state/{ID}.json"))
        .exists());
    let before = store.get(ID.into()).await.unwrap();
    let view = writes.capture_organization(ID.into()).await.unwrap();
    assert_eq!(view.state(), SessionState::Archived);
    let receipt = view.apply(SessionAction::Restore).await.unwrap();
    assert_eq!(receipt.revision, "2");
    assert_eq!(
        store
            .organization_state(ID.into())
            .await
            .unwrap()
            .unwrap()
            .state,
        "closed"
    );
    assert_eq!(store.get(ID.into()).await.unwrap(), before);
    store.close().await.unwrap();
}

fn organization(state: &Arc<AppState>) -> Arc<crate::session_archive::OrganizationStore> {
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .organization
        .clone()
}
async fn pending(store: &Arc<crate::session_archive::OrganizationStore>, value: bool) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while store.pending(ID) != value {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn sql_organization_stale_views_never_stop_replacement_authority() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "control",
        "runtime",
        "creation",
        "pid",
        "terminal",
        "resume",
        "phase",
        "lease",
        "reader",
        "organization",
    ] {
        let (_dir, state, store, writes, killed, _commands) = fixture().await;
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let before = store.get(ID.into()).await.unwrap();
        match change {
            "control" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .runtime
                    .control_tx = tokio::sync::mpsc::channel(2).0
            }
            "runtime" => {
                state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
                    SessionRuntime::live(24, 80).0
            }
            "creation" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .info
                    .created_at = "replacement".into()
            }
            "pid" => {
                state.session_pids.lock().unwrap().insert(ID.into(), 123);
            }
            "terminal" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .terminal_attached = true
            }
            "resume" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .resume_in_progress = true
            }
            "phase" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .info
                    .lifecycle_phase = "ending".into()
            }
            "lease" => state.workspace.lock().unwrap().as_mut().unwrap().lease = None,
            "reader" => {
                let root = root(&state);
                state.workspace.lock().unwrap().as_mut().unwrap().metadata =
                    metadata::MetadataStore::new(&root);
            }
            "organization" => {
                let root = root(&state);
                state
                    .workspace
                    .lock()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .metadata
                    .organization = Arc::new(crate::session_archive::OrganizationStore::new(&root));
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                view.apply(SessionAction::Archive).await,
                Err(StoreError::Conflict)
            ),
            "{change}"
        );
        assert!(!*killed.borrow(), "stale {change} signalled a stop");
        assert_eq!(store.get(ID.into()).await.unwrap(), before);
        assert_eq!(
            store
                .organization_state(ID.into())
                .await
                .unwrap()
                .unwrap()
                .organization_revision,
            "0"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_organization_stale_durable_choice_and_unsupported_actions_do_not_mutate() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let stale = writes.capture_organization(ID.into()).await.unwrap();
    let created = store.get(ID.into()).await.unwrap().unwrap()["createdAt"]
        .as_str()
        .unwrap()
        .to_owned();
    store
        .organize(ID.into(), created, "0".into(), "archived".into())
        .await
        .unwrap();
    let before = store.get(ID.into()).await.unwrap();
    assert!(matches!(
        stale.apply(SessionAction::Archive).await,
        Err(StoreError::Conflict)
    ));
    for action in [
        SessionAction::Archive,
        SessionAction::Close,
        SessionAction::Delete,
    ] {
        let view = writes.capture_organization(ID.into()).await.unwrap();
        assert!(matches!(
            view.apply(action).await,
            Err(StoreError::Conflict)
        ));
    }
    assert!(!*killed.borrow());
    assert_eq!(store.get(ID.into()).await.unwrap(), before);
    assert_eq!(
        store
            .organization_state(ID.into())
            .await
            .unwrap()
            .unwrap()
            .organization_revision,
        "1"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_archive_waits_for_all_physical_close_predicates_after_metadata_commit() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for remaining in ["resume", "terminal", "phase", "lifecycle"] {
        let (_dir, state, store, writes, mut killed, commands) = fixture().await;
        state.session_pids.lock().unwrap().insert(ID.into(), 123);
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let mut action = tokio::spawn(view.apply(SessionAction::Archive));
        killed.changed().await.unwrap();
        finalize(&state, &writes).await;
        {
            let mut sessions = state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            match remaining {
                "resume" => h.resume_in_progress = true,
                "terminal" => h.terminal_attached = true,
                "phase" => h.info.lifecycle_phase = "ending".into(),
                "lifecycle" => h.info.lifecycle = "stopping".into(),
                _ => unreachable!(),
            }
        }
        state.session_pids.lock().unwrap().remove(ID);
        drop(commands);
        assert!(
            tokio::time::timeout(Duration::from_millis(75), &mut action)
                .await
                .is_err(),
            "{remaining}"
        );
        assert_eq!(
            store
                .organization_state(ID.into())
                .await
                .unwrap()
                .unwrap()
                .state,
            "closed"
        );
        {
            let mut sessions = state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            h.resume_in_progress = false;
            h.terminal_attached = false;
            h.info.lifecycle_phase = "ended".into();
            h.info.lifecycle = "dead".into();
        }
        action.await.unwrap().unwrap();
        assert_eq!(
            store
                .organization_state(ID.into())
                .await
                .unwrap()
                .unwrap()
                .state,
            "archived"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_archive_timeout_and_cancelled_close_leave_no_later_organization_write() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [false, true] {
        let (_dir, state, store, writes, mut killed, commands) = fixture().await;
        let original = organization(&state);
        let view = writes.capture_organization(ID.into()).await.unwrap();
        if cancel {
            let action = tokio::spawn(view.apply(SessionAction::Archive));
            killed.changed().await.unwrap();
            action.abort();
            assert!(action.await.unwrap_err().is_cancelled());
        } else {
            assert!(matches!(
                view.apply_with_timeout(SessionAction::Archive, Duration::ZERO)
                    .await,
                Err(StoreError::Expired)
            ));
        }
        pending(&original, false).await;
        assert!(*killed.borrow());
        finalize(&state, &writes).await;
        drop(commands);
        assert_eq!(
            store
                .organization_state(ID.into())
                .await
                .unwrap()
                .unwrap()
                .state,
            "closed"
        );
        assert_eq!(
            store
                .organization_state(ID.into())
                .await
                .unwrap()
                .unwrap()
                .organization_revision,
            "0"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_organization_lane_wait_expires_or_cancels_without_effects() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [false, true] {
        let (_dir, state, store, writes, _killed, commands) = fixture().await;
        finalize(&state, &writes).await;
        drop(commands);
        let original = organization(&state);
        let lane = store.runtime_lane(ID).unwrap().lock_owned().await;
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let action = tokio::spawn(view.apply(SessionAction::Archive));
        pending(&original, true).await;
        if cancel {
            action.abort();
            assert!(action.await.unwrap_err().is_cancelled());
        } else {
            assert!(matches!(
                tokio::time::timeout(Duration::from_secs(3), action)
                    .await
                    .unwrap()
                    .unwrap(),
                Err(StoreError::Expired)
            ));
        }
        pending(&original, false).await;
        assert_eq!(
            store
                .organization_state(ID.into())
                .await
                .unwrap()
                .unwrap()
                .state,
            "closed"
        );
        drop(lane);
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_organization_rechecks_original_runtime_and_closure_at_execution_grant() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for replacement in ["pid", "control", "lease", "runtime"] {
        let (_dir, state, store, writes, _killed, commands) = fixture().await;
        finalize(&state, &writes).await;
        drop(commands);
        let original = organization(&state);
        let lane = store.runtime_lane(ID).unwrap().lock_owned().await;
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let (lane_sent, lane_ready) = oneshot::channel();
        *writes.before_organization_lane.lock().unwrap() = Some(lane_sent);
        let action = tokio::spawn(view.apply(SessionAction::Archive));
        lane_ready.await.unwrap();
        assert!(original.pending(ID));
        match replacement {
            "pid" => {
                state.session_pids.lock().unwrap().insert(ID.into(), 123);
            }
            "control" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .runtime
                    .control_tx = tokio::sync::mpsc::channel(2).0
            }
            "lease" => state.workspace.lock().unwrap().as_mut().unwrap().lease = None,
            "runtime" => {
                state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
                    SessionRuntime::live(24, 80).0
            }
            _ => unreachable!(),
        }
        drop(lane);
        assert!(
            matches!(action.await.unwrap(), Err(StoreError::Conflict)),
            "{replacement}"
        );
        pending(&original, false).await;
        assert_eq!(
            store
                .organization_state(ID.into())
                .await
                .unwrap()
                .unwrap()
                .state,
            "closed"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_organization_admitted_cancel_retains_reservation_lane_and_lease_until_commit() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let original = organization(&state);
    let weak = Arc::downgrade(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .lease
            .as_ref()
            .unwrap(),
    );
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let mut admitted = crate::session_store::test_worker::hold_next_organization(&store, ID);
    let action = tokio::spawn(view.apply(SessionAction::Archive));
    admitted.entered().await;
    action.abort();
    assert!(action.await.unwrap_err().is_cancelled());
    state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
    assert!(weak.upgrade().is_some());
    assert!(original.pending(ID));
    assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_err());
    drop(admitted);
    assert_eq!(
        store
            .organization_state(ID.into())
            .await
            .unwrap()
            .unwrap()
            .state,
        "archived"
    );
    pending(&original, false).await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while weak.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_ok());
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_organization_admitted_commit_reports_detachment_without_touching_replacement() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let mut admitted = crate::session_store::test_worker::hold_next_organization(&store, ID);
    let action = tokio::spawn(view.apply(SessionAction::Archive));
    admitted.entered().await;
    state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime = SessionRuntime::live(24, 80).0;
    let replacement = state.sessions.lock().unwrap()[ID].runtime.identity();
    drop(admitted);
    assert!(matches!(
        action.await.unwrap().unwrap().outcome,
        Outcome::CommittedDetached
    ));
    assert_eq!(
        state.sessions.lock().unwrap()[ID].runtime.identity(),
        replacement
    );
    assert_eq!(
        store
            .organization_state(ID.into())
            .await
            .unwrap()
            .unwrap()
            .state,
        "archived"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_organization_unavailable_sql_never_falls_back_to_valid_json() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, killed, _commands) = fixture().await;
    let legacy = std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap();
    let view = writes.capture_organization(ID.into()).await.unwrap();
    store.close().await.unwrap();
    assert!(matches!(
        view.apply(SessionAction::Archive).await,
        Err(StoreError::Unavailable)
    ));
    assert!(!*killed.borrow());
    assert_eq!(
        std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap(),
        legacy
    );
}

#[tokio::test]
async fn sql_organization_capture_rejects_replacement_during_authoritative_read() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, killed, _commands) = fixture().await;
    let mut read = crate::session_store::test_worker::hold_nth_organization_state(&store, ID, 1);
    let owner = writes.clone();
    let capture = tokio::spawn(async move { owner.capture_organization(ID.into()).await });
    read.entered().await;
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut(ID)
        .unwrap()
        .runtime
        .control_tx = tokio::sync::mpsc::channel(2).0;
    drop(read);
    assert!(matches!(capture.await.unwrap(), Err(StoreError::Conflict)));
    assert!(!*killed.borrow());
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_organization_missing_or_invalid_sql_preserves_legacy_and_never_stops() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for fault in ["missing", "shape", "identity"] {
        let (_dir, state, store, writes, killed, _commands) = fixture().await;
        let legacy = std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap();
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let db = rusqlite::Connection::open_with_flags(
            root(&state).join("session-store/v1.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
        )
        .unwrap();
        if fault == "missing" {
            db.execute("DELETE FROM sessions WHERE id=?1", [ID])
                .unwrap();
        } else {
            let mut doc = store.get(ID.into()).await.unwrap().unwrap();
            let bytes = if fault == "shape" {
                "[]".into()
            } else {
                doc["id"] = serde_json::json!("replacement");
                serde_json::to_string(&doc).unwrap()
            };
            db.execute(
                "UPDATE sessions SET metadata=?2 WHERE id=?1",
                rusqlite::params![ID, bytes],
            )
            .unwrap();
        }
        drop(db);
        let result = view.apply(SessionAction::Archive).await;
        assert!(
            matches!(
                result,
                Err(StoreError::Missing | StoreError::Invalid(_) | StoreError::Corrupt)
            ),
            "{fault}: {result:?}"
        );
        assert!(!*killed.borrow());
        assert_eq!(
            std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap(),
            legacy
        );
        assert!(!organization(&state).pending(ID));
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_organization_remembered_closed_row_needs_no_runtime_and_preserves_metadata() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    state.sessions.lock().unwrap().remove(ID);
    let before = store.get(ID.into()).await.unwrap();
    let view = writes.capture_organization(ID.into()).await.unwrap();
    assert_eq!(view.state(), SessionState::Closed);
    assert!(matches!(
        view.apply(SessionAction::Archive).await.unwrap().outcome,
        Outcome::Committed
    ));
    let view = writes.capture_organization(ID.into()).await.unwrap();
    view.apply(SessionAction::Restore).await.unwrap();
    assert!(state.sessions.lock().unwrap().is_empty());
    assert_eq!(store.get(ID.into()).await.unwrap(), before);
    store.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn sql_organization_stall_keeps_guards_and_another_runtime_input_receiver_available() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let input_id = "organization-input";
    let (runtime, mut input_commands) = SessionRuntime::live(24, 80);
    let mut info = crate::test_support::test_session_info(
        input_id,
        "Input",
        dir.path().display().to_string(),
        "running",
        CREATED,
    );
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    state.sessions.lock().unwrap().insert(
        input_id.into(),
        crate::SessionHandle {
            info,
            runtime,
            kill_tx: tokio::sync::watch::channel(false).0,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
            active_work_hook: false,
        },
    );
    let meta = crate::test_support::test_session_metadata(
        input_id,
        "Input",
        dir.path().display().to_string(),
        "running",
        CREATED,
        "2026-01-02T00:00:00Z",
    );
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .try_write_session(&meta)
        .unwrap();
    let mut pause = crate::session_store::test_worker::hold_next_organization(&store, ID);
    let (entered_sent, entered) = oneshot::channel();
    let (progress_sent, progress) = oneshot::channel();
    let watchdog = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let progressed = runtime.block_on(async {
            pause.entered().await;
            let _ = entered_sent.send(());
            tokio::time::timeout(Duration::from_secs(2), progress)
                .await
                .is_ok_and(|v| v.is_ok())
        });
        drop(pause);
        progressed
    });
    let writer = tokio::spawn(view.apply(SessionAction::Archive));
    entered.await.unwrap();
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(state.session_pids.try_lock().is_ok());
    let delivery = tokio::spawn(async move {
        match input_commands.recv().await.unwrap() {
            RuntimeCommand::Input {
                data,
                accepted: Some(accepted),
            } => {
                assert_eq!(data, "independent-input\r");
                let _ = accepted.send(Ok(()));
            }
            _ => panic!("expected acknowledged input"),
        }
    });
    let delivered = tokio::time::timeout(
        Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            input_id,
            "independent-input\r".into(),
        ),
    )
    .await;
    let delivery_result = tokio::time::timeout(Duration::from_secs(1), delivery).await;
    let _ = progress_sent.send(());
    assert!(
        watchdog.join().unwrap(),
        "SQL organization blocked the sole Tokio worker until OS watchdog release"
    );
    delivery_result.unwrap().unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    writer.await.unwrap().unwrap();
    store.close().await.unwrap();
}

#[test]
fn sql_organization_finishes_with_one_blocking_worker() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
        let (_dir, state, store, writes, _killed, commands) = fixture().await;
        finalize(&state, &writes).await;
        drop(commands);
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let result =
            tokio::time::timeout(Duration::from_secs(5), view.apply(SessionAction::Archive)).await;
        store.close().await.unwrap();
        result
    });
    runtime.shutdown_timeout(Duration::from_secs(2));
    assert!(result.is_ok_and(|v| v.is_ok()));
}

#[tokio::test]
async fn sql_organization_queued_expiry_releases_action_while_worker_is_stalled() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let original = organization(&state);
    let lane = store.runtime_lane(ID).unwrap().lock_owned().await;
    let view = writes.capture_organization(ID.into()).await.unwrap();
    let (ready_sent, ready) = oneshot::channel();
    *writes.before_organization_lane.lock().unwrap() = Some(ready_sent);
    let action = tokio::spawn(view.apply(SessionAction::Archive));
    ready.await.unwrap();
    let stalled = crate::session_store::test_worker::pause(&store);
    drop(lane);
    crate::session_store::test_worker::wait_for_organization(&store, ID).await;
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), action)
            .await
            .unwrap()
            .unwrap(),
        Err(StoreError::Expired)
    ));
    pending(&original, false).await;
    assert!(store.runtime_lane(ID).unwrap().try_lock_owned().is_ok());
    drop(stalled);
    assert_eq!(
        store
            .organization_state(ID.into())
            .await
            .unwrap()
            .unwrap()
            .state,
        "closed"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_organization_atomic_grant_survives_cancellation_before_grant_delivery() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for cancel in [true, false] {
        let (_dir, state, store, writes, mut killed, commands) = fixture().await;
        let original = organization(&state);
        let mut commands = Some(commands);
        if cancel {
            finalize(&state, &writes).await;
            drop(commands.take());
        }
        let view = writes.capture_organization(ID.into()).await.unwrap();
        let (entered_sent, entered) = oneshot::channel();
        let (release, delivery) = oneshot::channel();
        *writes.after_organization_grant.lock().unwrap() = Some((entered_sent, delivery));
        let mut action =
            tokio::spawn(view.apply_with_timeout(SessionAction::Archive, Duration::from_secs(2)));
        if cancel {
            drop(commands.take());
        } else {
            killed.changed().await.unwrap();
            finalize(&state, &writes).await;
            drop(commands.take());
        }
        tokio::time::timeout(Duration::from_secs(5), entered)
            .await
            .unwrap()
            .unwrap();
        if cancel {
            action.abort();
            assert!((&mut action).await.unwrap_err().is_cancelled());
        } else {
            assert!(
                tokio::time::timeout(Duration::from_millis(2100), &mut action)
                    .await
                    .is_err(),
                "a successful atomic grant was expired before its delivery"
            );
        }
        // Let the disconnected caller branch run while the successful grant
        // remains parked in validation, before execute wakes the SQL worker.
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(
            original.pending(ID),
            "an admitted action released its reservation"
        );
        release.send(()).expect("admitted grant was discarded");
        if !cancel {
            action.await.unwrap().unwrap();
        }
        pending(&original, false).await;
        assert_eq!(
            store
                .organization_state(ID.into())
                .await
                .unwrap()
                .unwrap()
                .state,
            "archived"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_organization_does_not_depend_on_unrelated_prompt_transition() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _killed, commands) = fixture().await;
    finalize(&state, &writes).await;
    drop(commands);
    let transition = crate::runtime::prompt_authority::transition_lock()
        .lock()
        .unwrap();
    let result = match writes.capture_organization(ID.into()).await {
        Ok(view) => view.apply(SessionAction::Archive).await,
        Err(error) => Err(error),
    };
    drop(transition);
    assert!(result.is_ok_and(|receipt| matches!(receipt.outcome, Outcome::Committed)));
    store.close().await.unwrap();
}
