use super::*;
use crate::runtime::session_runtime::{send_runtime_command, RuntimeCommand};
use crate::taskmaster::publication_lifecycle::{testing::Fixture, TaskRevocationGuard};

pub(crate) fn fixture() -> (
    tempfile::TempDir,
    Arc<AppState>,
    Fixture,
    tokio::sync::watch::Receiver<bool>,
) {
    let root = tempfile::tempdir().unwrap();
    let mut state = crate::test_support::test_app_state_with_workspace(root.path());
    Arc::get_mut(&mut state).unwrap().peon.config.enabled = false;
    let authority = Fixture::new();
    let mut handle = super::tests::attention_test_handle("evaluation-close", root.path());
    handle.runtime.evaluation_revocation = Some(authority.runtime.clone());
    let kill = handle.kill_tx.subscribe();
    state
        .sessions
        .lock()
        .unwrap()
        .insert("evaluation-close".into(), handle);
    (root, state, authority, kill)
}

#[tokio::test]
async fn evaluation_close_terminal_kill_denies_response_before_signal() {
    let (_root, state, authority, mut kill) = fixture();
    let task = tokio::spawn(async move {
        send_runtime_command(&state, "evaluation-close", RuntimeCommand::Kill).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), kill.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(
        !authority.can_respond(),
        "kill must follow the exact response cutoff"
    );
    task.await.unwrap().unwrap();
}
#[tokio::test]
async fn evaluation_close_delete_denies_response_before_signal() {
    let (_root, state, authority, mut kill) = fixture();
    let task = tokio::spawn(async move {
        SessionApplication::new(state)
            .delete_session("evaluation-close")
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), kill.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(!authority.can_respond());
    task.await.unwrap().unwrap();
}
#[tokio::test]
async fn evaluation_close_terminal_transition_is_an_idempotent_backstop() {
    let (_root, state, authority, _) = fixture();
    let generation = state.sessions.lock().unwrap()["evaluation-close"]
        .runtime
        .run_generation();
    SessionApplication::new(state.clone())
        .transition_session_status("evaluation-close", Some(generation), "ended")
        .await;
    assert!(!authority.can_respond());
    SessionApplication::new(state)
        .transition_session_status("evaluation-close", Some(generation), "ended")
        .await;
    assert!(!authority.can_respond());
}
#[test]
fn evaluation_close_workspace_reload_closes_old_scope() {
    let (_root, state, authority, _) = fixture();
    state
        .workspace
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .evaluation_revocation = Some(authority.workspace.clone());
    let next = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let _home = crate::test_support::FakeHome::set(home.path());
    SessionApplication::new(state)
        .open_workspace(next.path().to_path_buf())
        .unwrap();
    assert!(!authority.can_respond());
}
#[tokio::test]
async fn evaluation_close_cancellation_before_first_poll_closes_exact_carrier() {
    let old = Fixture::new();
    let replacement = Fixture::new();
    let guard = TaskRevocationGuard(Some(old.runtime.clone()));
    let future = async move {
        let _guard = guard;
        std::future::pending::<()>().await;
    };
    drop(future);
    assert!(!old.can_respond());
    assert!(replacement.can_respond());
}
#[test]
fn evaluation_close_cloned_carrier_drop_is_inert() {
    let f = Fixture::new();
    drop(f.runtime.clone());
    assert!(f.can_respond());
}
#[tokio::test]
async fn evaluation_close_poison_keeps_ordinary_kill_available() {
    let (_root, state, authority, kill) = fixture();
    authority.poison();
    send_runtime_command(&state, "evaluation-close", RuntimeCommand::Kill)
        .await
        .unwrap();
    assert!(*kill.borrow());
    assert!(!authority.can_respond());
}

#[tokio::test]
async fn evaluation_close_row_actions_close_before_their_kill_signal() {
    use super::session_actions::{SessionAction, SessionActionRequest};
    for action in [
        SessionAction::Close,
        SessionAction::Archive,
        SessionAction::Delete,
    ] {
        let (root, state, authority, mut kill) = fixture();
        let meta = crate::test_support::test_session_metadata(
            "evaluation-close",
            "Attention",
            root.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let info = crate::session_projection::SessionProjection::new(state.clone())
            .list()
            .unwrap()
            .remove(0);
        let request = SessionActionRequest {
            action,
            token: info.organization.session_action_token.unwrap(),
        };
        let task = tokio::spawn(async move {
            SessionApplication::new(state)
                .session_action("evaluation-close", request)
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(3), kill.changed())
            .await
            .unwrap()
            .unwrap();
        let denied = !authority.can_respond();
        task.abort();
        let _ = task.await;
        assert!(denied, "row action must revoke before signaling");
    }
}
fn ended_fixture() -> (
    tempfile::TempDir,
    Arc<AppState>,
    Fixture,
    tokio::sync::watch::Receiver<bool>,
) {
    let (root, state, authority, kill) = fixture();
    let meta = crate::test_support::test_session_metadata(
        "evaluation-close",
        "Attention",
        root.path().display().to_string(),
        "ended",
        "now",
        "now",
    );
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .write_session(&meta);
    {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut("evaluation-close").unwrap();
        h.info.lifecycle_phase = "ended".into();
        h.info.lifecycle = "dead".into();
    }
    (root, state, authority, kill)
}
#[test]
fn evaluation_close_resume_exchange_and_rollback_never_rearm() {
    let (root, state, old, _) = ended_fixture();
    let next = Fixture::new();
    let generation = state.sessions.lock().unwrap()["evaluation-close"]
        .runtime
        .run_generation();
    let mut replacement = super::tests::attention_test_handle("evaluation-close", root.path());
    replacement.runtime.evaluation_revocation = Some(next.runtime.clone());
    let admission = try_install_claimed_resume_handle(
        &state,
        "evaluation-close",
        replacement,
        true,
        Some(generation),
    )
    .unwrap();
    assert!(!old.can_respond(), "exchange closes the original runtime");
    drop(admission);
    assert!(
        !old.can_respond(),
        "restoring ordinary handle cannot rearm authority"
    );
    assert!(
        !next.can_respond(),
        "abandoned pre-spawn runtime must close"
    );
    assert_eq!(
        state.sessions.lock().unwrap()["evaluation-close"]
            .runtime
            .run_generation(),
        generation
    );
}
#[tokio::test]
async fn evaluation_close_stale_terminal_backstop_does_not_close_replacement() {
    let (_root, state, authority, _) = fixture();
    let generation = state.sessions.lock().unwrap()["evaluation-close"]
        .runtime
        .run_generation();
    assert!(
        !SessionApplication::new(state)
            .transition_session_status("evaluation-close", Some(generation + 1), "ended")
            .await
    );
    assert!(authority.can_respond());
}
#[test]
fn evaluation_close_same_workspace_reload_is_not_authority_continuity() {
    let (root, state, authority, _) = fixture();
    state
        .workspace
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .evaluation_revocation = Some(authority.workspace.clone());
    let home = tempfile::tempdir().unwrap();
    let _home = crate::test_support::FakeHome::set(home.path());
    SessionApplication::new(state)
        .open_workspace(root.path().to_path_buf())
        .unwrap();
    assert!(!authority.can_respond());
}

fn startup_future(state: Arc<AppState>) -> impl std::future::Future<Output = Result<(), String>> {
    let (mut runtime, control) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let (output, kill, cwd) = {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut("evaluation-close").unwrap();
        runtime.evaluation_revocation = h.runtime.evaluation_revocation.clone();
        let output = runtime.output_tx.clone();
        h.runtime = runtime;
        (output, h.kill_tx.subscribe(), h.info.cwd.clone())
    };
    crate::runtime::session_runtime::start_session_runtime(
        state,
        "evaluation-close".into(),
        crate::harness::CommandSpec {
            program: "orkworks-evaluation-nonexistent-fixture".into(),
            args: vec![],
            cwd,
        },
        None,
        control,
        output,
        kill,
        portable_pty::PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        },
    )
}
#[tokio::test]
async fn evaluation_close_actual_startup_cancellation_before_poll() {
    let (_root, state, authority, _) = fixture();
    let future = startup_future(state);
    drop(future);
    assert!(!authority.can_respond());
}
#[tokio::test]
async fn evaluation_close_actual_startup_cancellation_during_wait() {
    let (_root, state, authority, _) = fixture();
    {
        let future = startup_future(state);
        tokio::pin!(future);
        tokio::select! { biased;
            _ = &mut future => panic!("startup fixture must reach the initial control wait"),
            _ = tokio::task::yield_now() => {}
        }
    }
    assert!(!authority.can_respond());
}
#[tokio::test]
async fn evaluation_close_failed_spawn_closes_exact_runtime() {
    let (_root, state, authority, _) = fixture();
    assert!(startup_future(state).await.is_err());
    assert!(!authority.can_respond());
}

#[test]
fn evaluation_close_kill_releases_session_workspace_and_prompt_locks() {
    let (_root, state, authority, kill) = fixture();
    let entered = authority.observe_close();
    let held = authority.hold_latch();
    let stop_state = state.clone();
    let stop = std::thread::spawn(move || {
        crate::runtime::session_runtime::send_runtime_kill(&stop_state, "evaluation-close", None)
    });
    entered
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap();
    let sessions_free = state.sessions.try_lock().is_ok();
    let workspace_free = state.workspace.try_lock().is_ok();
    let prompt_free = crate::runtime::prompt_authority::transition_lock()
        .try_lock()
        .is_ok();
    let not_signaled = !*kill.borrow();
    drop(held);
    stop.join().unwrap().unwrap();
    assert!(sessions_free && workspace_free && prompt_free);
    assert!(not_signaled && *kill.borrow());
    assert!(!authority.can_respond());
}
#[test]
fn evaluation_close_failed_workspace_tail_preserves_replacement_and_old_claim() {
    let (root, state, old, _) = fixture();
    let home = tempfile::tempdir().unwrap();
    let _home = crate::test_support::FakeHome::set(home.path());
    state
        .workspace
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .evaluation_revocation = Some(old.workspace.clone());
    let claim = old.claim();
    let entered = old.observe_close();
    let held = old.hold_latch();
    let path = tempfile::tempdir().unwrap();
    let target = path.path().to_path_buf();
    let opening_state = state.clone();
    let opening =
        std::thread::spawn(move || SessionApplication::new(opening_state).open_workspace(target));
    entered
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    // Deliberately bypass production projection serialization to force the final
    // exact-instance revalidation branch while the old cutoff is parked.
    crate::test_support::swap_workspace(&state, root.path());
    let replacement = Fixture::new();
    state
        .workspace
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .evaluation_revocation = Some(replacement.workspace.clone());
    let reader = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .session_reader();
    drop(held);
    assert!(matches!(
        opening.join().unwrap(),
        Err(SessionError::Conflict)
    ));
    assert!(!old.can_respond());
    claim.settle();
    assert!(!old.can_respond());
    assert!(replacement.can_respond());
    assert!(Arc::ptr_eq(
        &reader,
        &state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .session_reader()
    ));
}
#[tokio::test]
async fn evaluation_close_unissued_runtime_cannot_register_after_stop() {
    let (_root, state, _original, _) = fixture();
    let unissued = Fixture::unissued();
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut("evaluation-close")
        .unwrap()
        .runtime
        .evaluation_revocation = Some(unissued.runtime.clone());
    send_runtime_command(&state, "evaluation-close", RuntimeCommand::Kill)
        .await
        .unwrap();
    assert!(!unissued.try_register());
}
#[tokio::test]
async fn evaluation_close_unpolled_startup_does_not_revoke_replacement() {
    let (_root, state, old, _) = fixture();
    let future = startup_future(state.clone());
    let next = Fixture::new();
    {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut("evaluation-close").unwrap();
        h.runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
        h.runtime.evaluation_revocation = Some(next.runtime.clone());
    }
    drop(future);
    assert!(!old.can_respond());
    assert!(next.can_respond());
}

static CREATION_ATTACHMENT: std::sync::Mutex<
    Vec<(
        std::sync::Weak<AppState>,
        crate::taskmaster::publication_lifecycle::RuntimeRevocation,
    )>,
> = std::sync::Mutex::new(Vec::new());
pub(super) fn attach_creation(state: &Arc<AppState>, id: &str) {
    let revocation = {
        let mut entries = CREATION_ATTACHMENT.lock().unwrap();
        entries
            .iter()
            .position(|(owner, _)| owner.upgrade().is_some_and(|s| Arc::ptr_eq(&s, state)))
            .map(|i| entries.remove(i).1)
    };
    if let Some(revocation) = revocation {
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut(id)
            .unwrap()
            .runtime
            .evaluation_revocation = Some(revocation);
    }
}
#[tokio::test]
async fn evaluation_close_creating_record_failure_closes_before_removal() {
    let root = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(root.path());
    let authority = Fixture::new();
    CREATION_ATTACHMENT
        .lock()
        .unwrap()
        .push((Arc::downgrade(&state), authority.runtime.clone()));
    let sessions = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .sessions_dir();
    std::fs::create_dir_all(sessions.parent().unwrap()).unwrap();
    std::fs::write(sessions, "blocked storage").unwrap();
    let request = CreateSessionCommand {
        reasoning_effort: None,
        initial_images: vec![],
        expected_workspace_path: None,
        harness_id: Some("generic-shell".into()),
        model: None,
        initial_prompt: None,
    };
    assert!(SessionApplication::new(state.clone())
        .create_session(request)
        .await
        .is_err());
    CREATION_ATTACHMENT
        .lock()
        .unwrap()
        .retain(|(owner, _)| !owner.upgrade().is_some_and(|s| Arc::ptr_eq(&s, &state)));
    assert!(state.sessions.lock().unwrap().is_empty());
    assert!(!authority.can_respond());
}

#[test]
fn evaluation_close_workspace_lease_reference_must_survive_adoption_revalidation() {
    let (_root, state, old, _) = fixture();
    let home = tempfile::tempdir().unwrap();
    let _home = crate::test_support::FakeHome::set(home.path());
    state
        .workspace
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .evaluation_revocation = Some(old.workspace.clone());
    let entered = old.observe_close();
    let held = old.hold_latch();
    let target = tempfile::tempdir().unwrap();
    let path = target.path().to_path_buf();
    let opening_state = state.clone();
    let opening =
        std::thread::spawn(move || SessionApplication::new(opening_state).open_workspace(path));
    entered
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    // Keep the same metadata reader, but replace the retained ordinary lease.
    let lease_root = tempfile::tempdir().unwrap();
    let replacement_lease =
        Arc::new(crate::workspace_runtime::WorkspaceLease::acquire(lease_root.path()).unwrap());
    state.workspace.lock().unwrap().as_mut().unwrap().lease = Some(replacement_lease.clone());
    drop(held);
    let rejected = matches!(opening.join().unwrap(), Err(SessionError::Conflict));
    let preserved = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .lease
        .as_ref()
        .is_some_and(|l| Arc::ptr_eq(l, &replacement_lease));
    drop(_home);
    assert!(
        rejected && preserved,
        "a changed lease reference must reject the stale adoption tail"
    );
    assert!(!old.can_respond());
}

#[test]
fn evaluation_close_concurrent_workspace_adopters_serialize_the_exchange() {
    let (_root, state, old, _) = fixture();
    let home = tempfile::tempdir().unwrap();
    let _home = crate::test_support::FakeHome::set(home.path());
    state
        .workspace
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .evaluation_revocation = Some(old.workspace.clone());
    let entered = old.observe_close();
    let held = old.hold_latch();
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    let path_one = one.path().to_path_buf();
    let path_two = two.path().to_path_buf();
    let expected = std::fs::canonicalize(&path_two).unwrap();
    let first_state = state.clone();
    let first =
        std::thread::spawn(move || SessionApplication::new(first_state).open_workspace(path_one));
    entered
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let second_state = state.clone();
    let second =
        std::thread::spawn(move || SessionApplication::new(second_state).open_workspace(path_two));
    drop(held);
    let first_ok = first.join().unwrap().is_ok();
    let second_ok = second.join().unwrap().is_ok();
    let selected = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path
        .clone();
    drop(_home);
    assert!(first_ok && second_ok);
    assert_eq!(selected, expected);
    assert!(!old.can_respond());
    assert!(state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .evaluation_revocation
        .is_none());
}

#[test]
fn evaluation_close_terminal_kill_does_not_wait_for_unrelated_projection_work() {
    let (_root, state, authority, kill) = fixture();
    let projection = state.projection_lock.lock().unwrap();
    let (done, result) = std::sync::mpsc::channel();
    let stop_state = state.clone();
    let worker = std::thread::spawn(move || {
        let outcome = crate::runtime::session_runtime::send_runtime_kill(
            &stop_state,
            "evaluation-close",
            None,
        );
        let _ = done.send(outcome);
    });
    let completed = result.recv_timeout(std::time::Duration::from_secs(3));
    drop(projection);
    worker.join().unwrap();
    assert_eq!(
        completed,
        Ok(Ok(())),
        "exact terminal kill must not wait on unrelated workspace I/O"
    );
    assert!(*kill.borrow());
    assert!(!authority.can_respond());
}
