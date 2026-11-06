use super::*;

fn memory(id: &str) -> WorkspaceMemory {
    WorkspaceMemory {
        last_active_session_id: Some(id.into()),
        last_active_at: Some("before".into()),
        active_harness_ids: vec!["codex".into()],
        active_harness_revision: 7,
    }
}

#[test]
fn workspace_memory_edit_fences_replacement_and_retention_clear() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());
    store.try_write_workspace_memory(&memory("old")).unwrap();
    let edit = store.edit_workspace_memory().unwrap();
    assert_eq!(
        store
            .try_write_workspace_memory(&memory("new"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(
        store
            .clear_last_active_session_if_matches("old")
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(edit.read().unwrap(), Some(memory("old")));
    drop(edit);
    store.clear_last_active_session_if_matches("old").unwrap();
    let saved = store.read_workspace_memory().unwrap();
    assert_eq!(saved.last_active_session_id, None);
    assert_eq!(saved.last_active_at, None);
    assert_eq!(saved.active_harness_ids, vec!["codex"]);
    assert_eq!(saved.active_harness_revision, 7);
    store.try_write_workspace_memory(&memory("new")).unwrap();
    assert_eq!(store.read_workspace_memory(), Some(memory("new")));
}

#[test]
fn workspace_memory_selection_producers_reject_busy_owner_without_effects() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let (edit, path) = {
        let ws = state.workspace.lock().unwrap();
        let metadata = &ws.as_ref().unwrap().metadata;
        metadata.try_write_workspace_memory(&memory("old")).unwrap();
        (
            metadata.edit_workspace_memory().unwrap(),
            metadata.workspace_memory_path(),
        )
    };
    let before = fs::read(&path).unwrap();
    let app = crate::session_application::SessionApplication::new(state);
    assert!(matches!(
        app.set_active_session("new"),
        Err(crate::session_application::SessionError::Conflict)
    ));
    assert!(matches!(
        app.set_active_harnesses_at(vec![], 7),
        Err(crate::session_application::SessionError::Conflict)
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    drop(edit);
    app.set_active_session("new").unwrap();
    let saved: WorkspaceMemory = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(saved.last_active_session_id.as_deref(), Some("new"));
    assert_eq!(saved.active_harness_ids, vec!["codex"]);
    assert_eq!(saved.active_harness_revision, 7);
}

#[test]
fn workspace_memory_mutations_preserve_malformed_and_unreadable_files() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let path = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .workspace_memory_path();
    let app = crate::session_application::SessionApplication::new(state.clone());
    for bad in [
        b"not json".as_slice(),
        br#"{"activeHarnessIds":false}"#,
        b"\xff",
    ] {
        fs::write(&path, bad).unwrap();
        assert!(app.set_active_session("new").is_err());
        assert!(app.set_active_harnesses_at(vec![], 0).is_err());
        assert!(state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .clear_last_active_session_if_matches("old")
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), bad);
    }
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(app.set_active_session("new").is_err());
    assert!(app.set_active_harnesses_at(vec![], 0).is_err());
    assert!(path.is_dir());
}

#[test]
fn workspace_memory_selection_reports_write_failure_and_can_retry() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let path = {
        let ws = state.workspace.lock().unwrap();
        let metadata = &ws.as_ref().unwrap().metadata;
        metadata.try_write_workspace_memory(&memory("old")).unwrap();
        metadata.workspace_memory_path()
    };
    let before = fs::read(&path).unwrap();
    let obstruction = path.with_file_name("workspace.json.tmp");
    fs::create_dir(&obstruction).unwrap();
    let app = crate::session_application::SessionApplication::new(state);
    assert!(matches!(
        app.set_active_session("new"),
        Err(crate::session_application::SessionError::Internal(_))
    ));
    assert!(matches!(
        app.set_active_harnesses_at(vec![], 7),
        Err(crate::session_application::SessionError::Internal(_))
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::remove_dir(obstruction).unwrap();
    let saved = app.set_active_harnesses_at(vec![], 7).unwrap();
    assert_eq!(saved.active_harness_ids, Vec::<String>::new());
    assert_eq!(saved.active_harness_revision, 8);
    assert_eq!(saved.last_active_session_id.as_deref(), Some("old"));
    app.set_active_session("new").unwrap();
    let saved: WorkspaceMemory = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(saved.active_harness_revision, 8);
    assert!(saved.active_harness_ids.is_empty());
    assert_eq!(saved.last_active_session_id.as_deref(), Some("new"));
}

#[test]
fn workspace_memory_missing_file_initializes_and_stale_revision_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let path = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .workspace_memory_path();
    let app = crate::session_application::SessionApplication::new(state);
    assert!(!path.exists());
    let saved = app.set_active_harnesses_at(vec![], 0).unwrap();
    assert_eq!(saved.active_harness_revision, 1);
    let before = fs::read(&path).unwrap();
    assert!(matches!(
        app.set_active_harnesses_at(vec![], 0),
        Err(crate::session_application::SessionError::Conflict)
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    app.set_active_session("selected").unwrap();
    let saved: WorkspaceMemory = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(saved.last_active_session_id.as_deref(), Some("selected"));
    assert_eq!(saved.active_harness_revision, 1);
}

#[test]
fn workspace_memory_owned_edit_survives_store_drop_without_blocking_another_store() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(first.path());
    let edit = store.edit_workspace_memory().unwrap();
    drop(store);
    MetadataStore::new(second.path())
        .try_write_workspace_memory(&memory("other"))
        .unwrap();
    edit.write(&memory("retained")).unwrap();
    assert_eq!(edit.read().unwrap(), Some(memory("retained")));
    assert_eq!(
        MetadataStore::new(second.path()).read_workspace_memory(),
        Some(memory("other"))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn workspace_memory_busy_selection_releases_guards_for_acknowledged_input() {
    use crate::runtime::session_runtime::{RuntimeCommand, SessionRuntime};
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let (mut runtime, mut commands) = SessionRuntime::live(24, 80);
    runtime.active_prompt_kind =
        Some(crate::runtime::session_runtime::RuntimePromptKind::Permission);
    let mut info = crate::test_support::test_session_info(
        "input",
        "Input",
        dir.path().display().to_string(),
        "running",
        "2026-01-01T00:00:00Z",
    );
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    info.harness_id = Some("claude-code".into());
    info.observed_status = Some("waiting_for_input".into());
    state.sessions.lock().unwrap().insert(
        "input".into(),
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
    let authority = crate::runtime::prompt_authority::registry();
    authority.issue_with_native_id(
        "input",
        "claude-code",
        "memory-permit-generation",
        Some("native"),
    );
    assert!(authority.activate("input", "claude-code", "memory-permit-generation", "native"));
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .try_write_workspace_memory(&WorkspaceMemory {
            active_harness_ids: vec!["claude-code".into()],
            ..Default::default()
        })
        .unwrap();
    let edit = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .edit_workspace_memory()
        .unwrap();
    let app = crate::session_application::SessionApplication::new(state.clone());
    assert!(matches!(
        app.set_active_session("other"),
        Err(crate::session_application::SessionError::Conflict)
    ));
    assert!(matches!(
        app.set_active_harnesses_at(vec![], 0),
        Err(crate::session_application::SessionError::Conflict)
    ));
    assert!(authority.is_active("input"));
    {
        let sessions = state.sessions.lock().unwrap();
        let handle = &sessions["input"];
        assert_eq!(
            handle.info.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(
            handle.runtime.active_prompt_kind,
            Some(crate::runtime::session_runtime::RuntimePromptKind::Permission)
        );
    }
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(state.session_pids.try_lock().is_ok());
    let driver = tokio::spawn(async move {
        match commands.recv().await.unwrap() {
            RuntimeCommand::Input {
                data,
                accepted: Some(accepted),
            } => {
                assert_eq!(data, "memory-input\r");
                let _ = accepted.send(Ok(()));
            }
            _ => panic!("expected acknowledged input"),
        }
    });
    let delivered = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            "input",
            "memory-input\r".into(),
        ),
    )
    .await;
    driver.await.unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    drop(edit);
    authority.remove("input");
}

#[test]
fn workspace_memory_reopen_rejects_original_busy_owner_before_adoption() {
    let home = tempfile::tempdir().unwrap();
    let (rejected, preserved, same_owner) =
        crate::test_support::with_fake_home(home.path(), || {
            let root = tempfile::tempdir().unwrap();
            let state = crate::test_support::test_app_state_with_workspace(root.path());
            let app = crate::session_application::SessionApplication::new(state.clone());
            app.open_workspace(root.path().into()).unwrap();
            let (edit, path, reader) = {
                let ws = state.workspace.lock().unwrap();
                let metadata = &ws.as_ref().unwrap().metadata;
                let mut before = memory("selected");
                before.active_harness_ids = vec!["retired-custom".into()];
                metadata.try_write_workspace_memory(&before).unwrap();
                (
                    metadata.edit_workspace_memory().unwrap(),
                    metadata.workspace_memory_path(),
                    metadata.session_reader(),
                )
            };
            let before = fs::read(&path).unwrap();
            let result = app.open_workspace(root.path().into());
            let same = Arc::ptr_eq(
                &reader,
                &state
                    .workspace
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .metadata
                    .session_reader(),
            );
            let preserved = fs::read(&path).unwrap() == before;
            drop(edit);
            (
                matches!(
                    result,
                    Err(crate::session_application::SessionError::Conflict)
                ),
                preserved,
                same,
            )
        });
    assert!(rejected, "reopen bypassed the adopted memory edit owner");
    assert!(preserved);
    assert!(same_owner);
}

#[test]
fn workspace_memory_reopen_reports_normalization_write_failure_before_adoption() {
    let home = tempfile::tempdir().unwrap();
    let (failed, preserved, same_owner) = crate::test_support::with_fake_home(home.path(), || {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let app = crate::session_application::SessionApplication::new(state.clone());
        app.open_workspace(root.path().into()).unwrap();
        let (path, reader) = {
            let ws = state.workspace.lock().unwrap();
            let metadata = &ws.as_ref().unwrap().metadata;
            let mut before = memory("selected");
            before.active_harness_ids = vec!["retired-custom".into()];
            metadata.try_write_workspace_memory(&before).unwrap();
            (metadata.workspace_memory_path(), metadata.session_reader())
        };
        let before = fs::read(&path).unwrap();
        fs::create_dir(path.with_file_name("workspace.json.tmp")).unwrap();
        let result = app.open_workspace(root.path().into());
        let same = Arc::ptr_eq(
            &reader,
            &state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .session_reader(),
        );
        (
            matches!(
                result,
                Err(crate::session_application::SessionError::Internal(_))
            ),
            fs::read(&path).unwrap() == before,
            same,
        )
    });
    assert!(
        failed,
        "normalization acknowledged an unpersisted settings revision"
    );
    assert!(preserved);
    assert!(same_owner);
}
