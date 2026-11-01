use super::*;
use crate::metadata::internal_mutations::{InternalMutation, PeonPolicy};
use crate::session_application::sql_writes::{Outcome, SessionWrites};
use crate::session_store::{test_worker, SessionStore, StoreError};
use serde_json::json;
use std::time::Duration;
const ID: &str = "sql-runtime";
const CREATED: &str = "2026-01-01T00:00:00Z";

type Fixture = (
    tempfile::TempDir,
    Arc<AppState>,
    Arc<SessionStore>,
    Arc<SessionWrites>,
    AttemptContext,
    PeonOutputCaptureIdentity,
    Arc<AtomicBool>,
    tokio::sync::mpsc::Receiver<crate::runtime::session_runtime::RuntimeCommand>,
);
async fn fixture() -> Fixture {
    fixture_with_hold(false).await
}
async fn fixture_with_hold(user_hold: bool) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let (root, workspace) = {
        let mut ws = state.workspace.lock().unwrap();
        let ws = ws.as_mut().unwrap();
        ws.lease = Some(Arc::new(
            crate::workspace_runtime::WorkspaceLease::acquire(&ws.metadata.root_path()).unwrap(),
        ));
        (
            ws.metadata.root_path(),
            ws.path.to_string_lossy().into_owned(),
        )
    };
    let (runtime, commands) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let identity = runtime.identity();
    let mut info = crate::test_support::test_session_info(
        ID,
        "Before",
        dir.path().display().to_string(),
        "running",
        CREATED,
    );
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    if user_hold {
        info.metadata_source = Some("user".into());
        info.provenance.attention_source = Some("user".into());
    }
    state.sessions.lock().unwrap().insert(
        ID.into(),
        crate::SessionHandle {
            info,
            kill_tx: tokio::sync::watch::channel(false).0,
            output_buffer: peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
            active_work_hook: false,
        },
    );
    state.peon.mark_candidate(ID);
    state.peon.in_flight.write().unwrap().insert(ID.into());
    let attempt = state.peon.begin_attempt(ID, identity.clone()).unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let context = AttemptContext::new(state.clone(), ID.into(), attempt, cancelled.clone());
    let capture = PeonOutputCaptureIdentity {
        input_generation: 0,
        min_revision: 0,
        first_revision: 0,
        last_revision: 0,
        runtime_instance_id: identity.runtime_instance_id,
        run_generation: identity.run_generation,
    };
    let store = Arc::new(
        SessionStore::initialize(root, workspace.clone())
            .await
            .unwrap(),
    );
    let mut meta = crate::test_support::test_session_metadata(
        ID,
        "Before",
        workspace,
        "running",
        CREATED,
        "2026-01-02T00:00:00Z",
    );
    meta.lifecycle = "alive".into();
    meta.lifecycle_phase = "active".into();
    meta.metadata_source = "peon".into();
    meta.provenance.attention_source = Some("peon".into());
    if user_hold {
        meta.metadata_source = "user".into();
        meta.provenance.attention_source = Some("user".into());
        meta.provenance.summary_source = Some("user".into());
    }
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .try_write_session(&meta)
        .unwrap();
    store
        .create(serde_json::to_value(meta).unwrap())
        .await
        .unwrap();
    let writes = SessionWrites::bind(state.clone(), store.clone()).unwrap();
    (
        dir, state, store, writes, context, capture, cancelled, commands,
    )
}
fn intent() -> InternalMutation {
    let inference = serde_json::from_value(json!({"observedStatus":"blocked","phase":"review","summary":null,"nextAction":"review results","needsUserInput":false,"detectedQuestion":null,"suggestedOptions":null,"blockerDescription":null,"failedCommand":null,"failedTest":null,"capacityHints":null,"confidence":0.8})).unwrap();
    InternalMutation::Peon {
        inference,
        timestamp: "2026-01-03T00:00:00Z".into(),
        provider: None,
        history_summary: None,
        policy: PeonPolicy::Infer,
    }
}

#[tokio::test]
async fn queued_runtime_write_revalidates_changed_input_before_execution() {
    for sequence_only in [false, true] {
        let _lease_guard = diagnostic_test_guard();
        let (_dir, state, store, writes, context, capture, _, _commands) = fixture().await;
        let paused = test_worker::pause(&store);
        let writer =
            tokio::spawn(async move { writes.peon(context.view(), capture, intent()).await });
        test_worker::wait_for_internal(&store, ID).await;
        assert!(state.projection_lock.try_lock().is_ok());
        assert!(state.workspace.try_lock().is_ok());
        {
            let mut sessions = state.sessions.lock().unwrap();
            let runtime = &mut sessions.get_mut(ID).unwrap().runtime;
            if sequence_only {
                runtime.committed_input_sequence += 1;
            } else {
                runtime.input_generation += 1;
            }
        }
        drop(paused);
        assert!(matches!(writer.await.unwrap(), Err(StoreError::Conflict)));
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "1"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn changed_workspace_or_attempt_cannot_execute_a_queued_write() {
    let _lease_guard = diagnostic_test_guard();
    for swap in [false, true] {
        let (dir, state, store, writes, context, capture, cancelled, _commands) = fixture().await;
        let paused = test_worker::pause(&store);
        let writer =
            tokio::spawn(async move { writes.peon(context.view(), capture, intent()).await });
        test_worker::wait_for_internal(&store, ID).await;
        if swap {
            crate::test_support::swap_workspace(&state, dir.path());
        } else {
            cancelled.store(true, Ordering::SeqCst);
        }
        drop(paused);
        assert!(matches!(writer.await.unwrap(), Err(StoreError::Conflict)));
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "1"
        );
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn committed_result_cannot_publish_to_a_changed_runtime() {
    let _lease_guard = diagnostic_test_guard();
    let (_dir, state, store, writes, context, capture, _, _commands) = fixture().await;
    let mut claimed = test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move { writes.peon(context.view(), capture, intent()).await });
    tokio::time::timeout(Duration::from_secs(5), claimed.entered())
        .await
        .unwrap();
    state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
        crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
    drop(claimed);
    assert_eq!(writer.await.unwrap().unwrap(), Outcome::CommittedDetached);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["observedStatus"],
        "blocked"
    );
    assert_ne!(
        state
            .sessions
            .lock()
            .unwrap()
            .get(ID)
            .unwrap()
            .info
            .observed_status
            .as_deref(),
        Some("blocked")
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn current_runtime_receives_only_the_committed_metadata() {
    let _lease_guard = diagnostic_test_guard();
    let (_dir, state, store, writes, context, capture, _, _commands) = fixture().await;
    assert_eq!(
        writes
            .peon(context.view(), capture, intent())
            .await
            .unwrap(),
        Outcome::Committed
    );
    let doc = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(doc["metadataRevision"], "2");
    assert_eq!(
        state
            .sessions
            .lock()
            .unwrap()
            .get(ID)
            .unwrap()
            .info
            .observed_status
            .as_deref(),
        Some("blocked")
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn caller_cancellation_after_execution_keeps_the_workspace_lease_until_completion() {
    let _lease_guard = diagnostic_test_guard();
    let (_dir, state, store, writes, context, capture, _, _commands) = fixture().await;
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut claimed = test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move { writes.peon(context.view(), capture, intent()).await });
    tokio::time::timeout(Duration::from_secs(5), claimed.entered())
        .await
        .unwrap();
    writer.abort();
    let _ = writer.await;
    *state.workspace.lock().unwrap() = None;
    assert!(crate::workspace_runtime::WorkspaceLease::acquire(&root).is_err());
    drop(claimed);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(lease) = crate::workspace_runtime::WorkspaceLease::acquire(&root) {
                drop(lease);
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    store.close().await.unwrap();
}

#[tokio::test]
async fn caller_cancellation_before_execution_does_not_need_attempt_invalidation() {
    let _lease_guard = diagnostic_test_guard();
    let (_dir, _state, store, writes, context, capture, _, _commands) = fixture().await;
    let paused = test_worker::pause(&store);
    let view = context.view();
    let writer = tokio::spawn(async move { writes.peon(view, capture, intent()).await });
    test_worker::wait_for_internal(&store, ID).await;
    writer.abort();
    let _ = writer.await;
    assert!(context.is_current());
    drop(paused);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn changed_attention_or_prompt_kind_rejects_a_queued_inference() {
    for change_prompt in [false, true] {
        let _lease_guard = diagnostic_test_guard();
        let (_dir, state, store, writes, context, capture, _, _commands) = fixture().await;
        let paused = test_worker::pause(&store);
        let writer =
            tokio::spawn(async move { writes.peon(context.view(), capture, intent()).await });
        test_worker::wait_for_internal(&store, ID).await;
        {
            let mut sessions = state.sessions.lock().unwrap();
            let runtime = &mut sessions.get_mut(ID).unwrap().runtime;
            if change_prompt {
                runtime.active_prompt_kind =
                    Some(crate::runtime::session_runtime::RuntimePromptKind::Permission);
            } else {
                runtime.attention_owner.accepted_write();
            }
        }
        drop(paused);
        assert!(matches!(writer.await.unwrap(), Err(StoreError::Conflict)));
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "1"
        );
        store.close().await.unwrap();
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_input_delivery_continues_while_application_sql_write_is_stalled() {
    use crate::runtime::session_runtime::RuntimeCommand;
    use std::io::{Read, Write};
    let _lease_guard = diagnostic_test_guard();
    let (_dir, state, store, writes, context, capture, _, mut commands) = fixture().await;
    let paused = test_worker::pause(&store);
    let view = context.view();
    let writer = tokio::spawn(async move { writes.peon(view, capture, intent()).await });
    test_worker::wait_for_internal(&store, ID).await;
    let pair = portable_pty::native_pty_system()
        .openpty(portable_pty::PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut child = pair
        .slave
        .spawn_command(portable_pty::CommandBuilder::new("/bin/cat"))
        .unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut terminal = pair.master.take_writer().unwrap();
    let (seen, observed) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = [0; 4096];
        let mut captured = Vec::new();
        let result = loop {
            match reader.read(&mut bytes) {
                Ok(0) => break Ok(captured),
                Ok(n) => {
                    captured.extend_from_slice(&bytes[..n]);
                    if captured.windows(16).any(|s| s == b"responsive-input")
                        || captured.len() > 16_384
                    {
                        break Ok(captured);
                    }
                }
                Err(error) => break Err(error),
            }
        };
        let _ = seen.send(result);
    });
    let delivery = tokio::task::spawn_blocking(move || match commands.blocking_recv().unwrap() {
        RuntimeCommand::Input {
            data,
            accepted: Some(accepted),
        } => {
            let result = terminal
                .write_all(data.as_bytes())
                .and_then(|()| terminal.flush())
                .map_err(|_| ());
            let _ = accepted.send(result);
        }
        _ => panic!("expected ordinary input command"),
    });
    let delivered = tokio::time::timeout(
        Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            ID,
            "responsive-input\r".into(),
        ),
    )
    .await;
    let echoed = observed.recv_timeout(Duration::from_secs(1));
    let _ = child.kill();
    let _ = child.wait();
    drop(pair.master);
    reader.join().unwrap();
    delivery.await.unwrap();
    drop(paused);
    assert_eq!(delivered.unwrap(), Ok(()));
    let echoed = echoed.unwrap().unwrap();
    assert!(echoed.windows(16).any(|s| s == b"responsive-input"));
    assert!(matches!(writer.await.unwrap(), Err(StoreError::Conflict)));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn a_user_owned_noop_retains_the_permanent_hold_decision() {
    let _lease_guard = diagnostic_test_guard();
    let (_dir, state, store, writes, context, capture, _, _commands) =
        fixture_with_hold(true).await;
    assert_eq!(
        writes
            .peon(context.view(), capture, intent())
            .await
            .unwrap(),
        Outcome::Noop {
            permanent_hold: true
        }
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    assert_eq!(
        state
            .sessions
            .lock()
            .unwrap()
            .get(ID)
            .unwrap()
            .info
            .metadata_source
            .as_deref(),
        Some("user")
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn an_application_sql_owner_requires_the_selected_workspace_lease() {
    let _lease_guard = diagnostic_test_guard();
    let (_dir, state, store, _writes, _context, _, _, _commands) = fixture().await;
    state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
    assert!(matches!(
        SessionWrites::bind(state, store.clone()),
        Err(StoreError::Conflict)
    ));
    store.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn contended_application_guards_do_not_block_the_async_executor() {
    let _lease_guard = diagnostic_test_guard();
    let (_dir, state, store, writes, context, capture, _, _commands) = fixture().await;
    let (entered, ready) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let held_state = state.clone();
    let blocker = std::thread::spawn(move || {
        let _held = held_state.projection_lock.lock().unwrap();
        entered.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(1)).is_ok()
    });
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let release_from_executor = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        let _ = release.send(());
    });
    let outcome = writes.peon(context.view(), capture, intent()).await;
    release_from_executor.await.unwrap();
    let responsive = blocker.join().unwrap();
    assert!(
        responsive,
        "application mutex acquisition blocked the executor until the OS watchdog released it"
    );
    assert_eq!(outcome.unwrap(), Outcome::Committed);
    store.close().await.unwrap();
}

#[tokio::test]
async fn publication_failure_after_commit_is_reported_as_committed_detached() {
    let _lease_guard = diagnostic_test_guard();
    let (_dir, state, store, writes, context, capture, _, _commands) = fixture().await;
    let mut claimed = test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move { writes.peon(context.view(), capture, intent()).await });
    tokio::time::timeout(Duration::from_secs(2), claimed.entered())
        .await
        .unwrap();
    let failed = state.clone();
    let _ = std::thread::spawn(move || {
        let _guard = failed.projection_lock.lock().unwrap();
        panic!("controlled publication-guard failure");
    })
    .join();
    drop(claimed);
    assert_eq!(writer.await.unwrap().unwrap(), Outcome::CommittedDetached);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn a_waiting_session_lane_expires_without_releasing_the_executing_owner() {
    let _lease_guard = diagnostic_test_guard();
    let (_dir, _state, store, writes, context, capture, _, _commands) = fixture().await;
    let mut claimed = test_worker::hold_next_internal(&store, ID);
    let first_owner = writes.clone();
    let first_view = context.view();
    let first_capture = capture.clone();
    let first =
        tokio::spawn(async move { first_owner.peon(first_view, first_capture, intent()).await });
    tokio::time::timeout(Duration::from_secs(2), claimed.entered())
        .await
        .unwrap();
    let second = tokio::time::timeout(
        Duration::from_millis(2500),
        writes.peon(context.view(), capture, intent()),
    )
    .await;
    assert!(matches!(second, Ok(Err(StoreError::Expired))));
    assert!(!first.is_finished());
    drop(claimed);
    assert_eq!(first.await.unwrap().unwrap(), Outcome::Committed);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn replaced_prompt_authority_with_unchanged_flags_rejects_queued_write() {
    for replacement_generation in ["generation-one", "generation-two"] {
        let _lease_guard = diagnostic_test_guard();
        let (_dir, _state, store, writes, context, capture, _, _commands) = fixture().await;
        let registry = crate::runtime::prompt_authority::registry();
        registry.issue_with_native_id(ID, "claude", "generation-one", Some("native-one"));
        let paused = test_worker::pause(&store);
        let writer =
            tokio::spawn(async move { writes.peon(context.view(), capture, intent()).await });
        test_worker::wait_for_internal(&store, ID).await;
        registry.issue_with_native_id(ID, "claude", replacement_generation, Some("native-one"));
        assert!(!registry.is_active(ID));
        assert!(!registry.identity_reset_pending(ID));
        drop(paused);
        let result = writer.await.unwrap();
        registry.remove(ID);
        assert!(matches!(result, Err(StoreError::Conflict)));
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "1"
        );
        store.close().await.unwrap();
    }
}
