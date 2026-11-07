// Included in the application test module to reuse its owned-runtime fixtures.
#[tokio::test]
async fn startup_failure_cancellation_and_replacement_do_not_publish() {
    use crate::startup_failure::{StartupError, StartupFailureCode as Code, StartupOperation};
    for replaced in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let handle = attention_test_handle("startup-cancel", root.path());
        let generation = handle.runtime.run_generation();
        if !replaced { handle.kill_tx.send_replace(true); }
        state.sessions.lock().unwrap().insert("startup-cancel".into(), handle);
        if replaced {
            state.sessions.lock().unwrap().insert("startup-cancel".into(), attention_test_handle("startup-cancel", root.path()));
        }
        let result = publish_startup_failure(state.clone(), "startup-cancel".into(), generation,
            StartupError::classified(Code::ProcessSpawnFailed, "\x1b[31msecret-token/path\x1b[0m"), StartupOperation::Resume).await;
        assert!(matches!(result, SessionError::Conflict));
        assert!(state.sessions.lock().unwrap()["startup-cancel"].info.startup_failure.is_none());
    }
}

#[tokio::test]
async fn startup_failure_retains_safe_live_detail_when_persistence_fails() {
    use crate::startup_failure::{StartupError, StartupFailureCode as Code, StartupOperation};
    let root = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(root.path());
    let id = "startup-io-failure";
    let mut meta = crate::test_support::test_session_metadata(id, "Safe", root.path().display().to_string(), "creating", "before", "before");
    meta.lifecycle_phase = "creating".into();
    state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&meta);
    let blocked = root.path().join(".orkworks-test/sessions/startup-io-failure.json.tmp");
    std::fs::create_dir(&blocked).unwrap();
    let mut handle = attention_test_handle(id, root.path());
    handle.info.status = "creating".into();
    handle.info.lifecycle_phase = "creating".into();
    let generation = handle.runtime.run_generation();
    state.sessions.lock().unwrap().insert(id.into(), handle);
    let result = publish_startup_failure(state.clone(), id.into(), generation,
        StartupError::classified(Code::TerminalIoFailed, "secret-token /private/path"), StartupOperation::Launch).await;
    assert!(matches!(result, SessionError::StartupFailed(_)));
    let info = state.sessions.lock().unwrap()[id].info.clone();
    assert_eq!(info.startup_failure.as_ref().unwrap().code, Code::TerminalIoFailed);
    assert!(!serde_json::to_string(&info.startup_failure).unwrap().contains("secret-token"));
    assert!(state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_session(id).unwrap().startup_failure.is_none());
}

#[test]
fn startup_failure_uncommitted_resume_restores_previous_diagnostic() {
    use crate::startup_failure::{StartupFailure, StartupFailureCode as Code, StartupOperation};
    let root = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(root.path());
    let id = "startup-rollback";
    let previous = StartupFailure::new(Code::CodexHelpFailed, StartupOperation::Launch);
    let mut saved = crate::test_support::test_session_metadata(id, "Safe", root.path().display().to_string(), "error", "before", "before");
    saved.startup_failure = Some(previous.clone());
    let mut creating = saved.clone();
    creating.status = "creating".into();
    creating.lifecycle_phase = "creating".into();
    creating.startup_failure = None;
    state.workspace.lock().unwrap().as_ref().unwrap().metadata.write_session(&creating);
    let mut handle = attention_test_handle(id, root.path());
    handle.resume_in_progress = true;
    handle.info.status = "creating".into();
    handle.info.lifecycle_phase = "creating".into();
    let generation = handle.runtime.run_generation();
    state.sessions.lock().unwrap().insert(id.into(), handle);
    let workspace_path = state.workspace.lock().unwrap().as_ref().unwrap().path.clone();
    let admission = ResumeAdmission { state:state.clone(), id:id.into(), generation,
        previous_handle:None, rollback:Some(ResumeRollback { workspace_path, metadata:saved, terminal_size:None }), committed:false };
    drop(admission);
    assert!(!state.sessions.lock().unwrap().contains_key(id));
    let ws = state.workspace.lock().unwrap();
    let record = ws.as_ref().unwrap().metadata.read_session(id).unwrap();
    assert_eq!(record.startup_failure, Some(previous));
    assert_eq!(record.status, "error");
}
