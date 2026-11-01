use super::*;
use crate::session_application::sql_writes::{Outcome, SessionWrites};
use crate::session_store::{test_worker, SessionStore, StoreError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
const ID: &str = "native-sql";
const CREATED: &str = "2026-01-01T00:00:00Z";
type Fixture = (
    tempfile::TempDir,
    Arc<AppState>,
    Arc<SessionStore>,
    Arc<SessionWrites>,
    Effect,
    tokio::sync::mpsc::Receiver<super::super::session_runtime::RuntimeCommand>,
);
async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let (root, path) = {
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
    let mut info =
        crate::test_support::test_session_info(ID, "native", path.clone(), "running", CREATED);
    info.harness = Some("codex".into());
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    info.resume = Some(crate::harness::ResumeMemory {
        state: crate::harness::ResumeState::Available,
        preferred_strategy: crate::harness::ResumeStrategy::Exact,
        harness_session_id: Some("root-id".into()),
        latest_fallback: false,
        last_seen_at: None,
    });
    let (mut runtime, commands) = super::super::session_runtime::SessionRuntime::live(24, 80);
    let mut tracker = NativeApprovalState::new(runtime.run_generation());
    tracker.root = Some("root-id".into());
    let start = Instant::now() - Duration::from_secs(2);
    tracker.reducer.observe(
        super::super::codex_approval::NativeStatus::Active,
        true,
        start,
        start,
    );
    tracker
        .reducer
        .hook(HookEvent::PreToolUse, Some("tool"), start);
    tracker
        .reducer
        .hook(HookEvent::PermissionRequest, None, start);
    let now = Instant::now();
    let effect = tracker.reducer.observe(
        super::super::codex_approval::NativeStatus::ApprovalPending,
        true,
        now,
        now,
    )[0];
    runtime.native_approval = Some(tracker);
    state.sessions.lock().unwrap().insert(
        ID.into(),
        SessionHandle {
            info,
            runtime,
            kill_tx: tokio::sync::watch::channel(false).0,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
            active_work_hook: true,
        },
    );
    let store = Arc::new(SessionStore::initialize(root, path.clone()).await.unwrap());
    let mut meta =
        crate::test_support::test_session_metadata(ID, "native", path, "running", CREATED, CREATED);
    meta.harness = "codex".into();
    meta.lifecycle = "alive".into();
    meta.lifecycle_phase = "active".into();
    meta.resume = state.sessions.lock().unwrap()[ID].info.resume.clone();
    meta.harness_session_id_source = Some("codex_hook".into());
    store
        .create(serde_json::to_value(meta).unwrap())
        .await
        .unwrap();
    let writes = SessionWrites::bind(state.clone(), store.clone()).unwrap();
    (dir, state, store, writes, effect, commands)
}
fn clear(state: &Arc<AppState>) -> Effect {
    let mut sessions = state.sessions.lock().unwrap();
    let tracker = sessions
        .get_mut(ID)
        .unwrap()
        .runtime
        .native_approval
        .as_mut()
        .unwrap();
    let now = Instant::now();
    tracker.reducer.observe(
        super::super::codex_approval::NativeStatus::Active,
        true,
        now,
        now,
    )[0]
}
#[tokio::test]
async fn native_sql_commits_before_live_acknowledgement_and_uses_owned_clear() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, effect, _commands) = fixture().await;
    assert_eq!(
        writes
            .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap(),
        Outcome::Committed
    );
    {
        let sessions = state.sessions.lock().unwrap();
        let h = &sessions[ID];
        assert_eq!(h.info.attention.as_deref(), Some("needs_you"));
        assert!(h
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .protects_permission_wait(&h.runtime.attention_owner, &h.info));
    }
    let effect = clear(&state);
    assert!(
        matches!(
            writes
                .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
                .await,
            Err(StoreError::Conflict)
        ),
        "production coordination gate remains closed"
    );
    // Re-issue from a fresh observation; the test-only path cannot enable production.
    let effect = clear(&state);
    assert_eq!(
        writes
            .native_coordinated_for_test(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap(),
        Outcome::Committed
    );
    let document = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(document["attention"], "working");
    assert_eq!(document["metadataRevision"], "3");
    let sessions = state.sessions.lock().unwrap();
    assert_eq!(sessions[ID].info.attention.as_deref(), Some("working"));
    drop(sessions);
    store.close().await.unwrap();
}
#[tokio::test]
async fn native_sql_commit_cannot_acknowledge_a_replacement_runtime() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, effect, _commands) = fixture().await;
    let mut held = test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move {
        writes
            .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await
    });
    held.entered().await;
    state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
        super::super::session_runtime::SessionRuntime::detached(24, 80);
    drop(held);
    assert_eq!(writer.await.unwrap().unwrap(), Outcome::CommittedDetached);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["attention"],
        "needs_you"
    );
    assert_ne!(
        state.sessions.lock().unwrap()[ID].info.attention.as_deref(),
        Some("needs_you")
    );
    store.close().await.unwrap();
}

// Losing any captured authority must reject before the real worker executes.
#[tokio::test]
async fn native_sql_queued_effect_rechecks_every_runtime_boundary() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "input",
        "sequence",
        "hook",
        "tracker",
        "root",
        "attention",
        "question",
        "options",
        "user",
        "ending",
        "inactive",
        "cancel",
        "workspace",
    ] {
        let (dir, state, store, writes, effect, _commands) = fixture().await;
        let reservation = store
            .prepare_internal(
                ID.into(),
                CREATED.into(),
                crate::metadata::internal_mutations::InternalMutation::NativeShowWait {
                    timestamp: CREATED.into(),
                },
            )
            .unwrap();
        let lane = reservation.lane().unwrap().lock_owned().await;
        let captured = writes.test_before_callback();
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let writer = tokio::spawn(async move { writes.native(ID.into(), effect, flag).await });
        tokio::time::timeout(Duration::from_secs(5), captured)
            .await
            .unwrap()
            .unwrap();
        {
            let mut sessions = state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            match change {
                "input" => h.runtime.input_generation += 1,
                "sequence" => h.runtime.committed_input_sequence += 1,
                "hook" => {
                    let t = h.runtime.native_approval.as_mut().unwrap();
                    t.hook_revision = t.hook_revision.map(|r| r + 1);
                }
                "tracker" => {
                    // Same generation/fence/root/candidate, different tracker instance.
                    let t = h.runtime.native_approval.as_mut().unwrap();
                    t.instance = Arc::new(());
                }
                "root" => {
                    h.info.resume.as_mut().unwrap().harness_session_id = Some("other-root".into())
                }
                "attention" => {
                    h.runtime.attention_owner.accepted_write();
                }
                "question" => h.info.detected_question = Some("answer me".into()),
                "options" => h.info.suggested_options = Some(vec!["choice".into()]),
                "user" => h.info.provenance.attention_source = Some("user".into()),
                "ending" => h.info.lifecycle_phase = "ending".into(),
                "inactive" => h.active_work_hook = false,
                "cancel" => cancelled.store(true, Ordering::Release),
                "workspace" => {}
                _ => unreachable!(),
            }
        }
        if change == "workspace" {
            crate::test_support::swap_workspace(&state, dir.path());
        }
        drop(lane);
        drop(reservation);
        assert!(
            matches!(writer.await.unwrap(), Err(StoreError::Conflict)),
            "{change}"
        );
        let doc = store.get(ID.into()).await.unwrap().unwrap();
        assert_eq!(doc["metadataRevision"], "1", "{change}");
        assert_ne!(doc["attention"], "needs_you", "{change}");
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn native_sql_queued_caller_cancellation_never_writes() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, _state, store, writes, effect, _commands) = fixture().await;
    let reservation = store
        .prepare_internal(
            ID.into(),
            CREATED.into(),
            crate::metadata::internal_mutations::InternalMutation::NativeShowWait {
                timestamp: CREATED.into(),
            },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let captured = writes.test_before_callback();
    let writer = tokio::spawn(async move {
        writes
            .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await
    });
    captured.await.unwrap();
    writer.abort();
    assert!(writer.await.unwrap_err().is_cancelled());
    drop(lane);
    drop(reservation);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn native_sql_executing_caller_cancellation_retains_commit_and_acknowledgement() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, effect, _commands) = fixture().await;
    let mut held = test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move {
        writes
            .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await
    });
    held.entered().await;
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    writer.abort();
    assert!(writer.await.unwrap_err().is_cancelled());
    drop(held);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if state.sessions.lock().unwrap()[ID].info.attention.as_deref() == Some("needs_you") {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn native_sql_intervening_durable_revision_change_is_a_conflict() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, effect, _commands) = fixture().await;
    let reservation = store
        .prepare_internal(
            ID.into(),
            CREATED.into(),
            crate::metadata::internal_mutations::InternalMutation::NativeShowWait {
                timestamp: CREATED.into(),
            },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let captured = writes.test_before_callback();
    let writer = tokio::spawn(async move {
        writes
            .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await
    });
    captured.await.unwrap();
    // An ordinary work write advances revision without granting native ownership.
    store
        .mutate_internal(
            ID.into(),
            CREATED.into(),
            None,
            crate::metadata::internal_mutations::InternalMutation::UserTitle {
                title: Some("newer work".into()),
            },
        )
        .await
        .unwrap();
    drop(lane);
    drop(reservation);
    assert!(matches!(writer.await.unwrap(), Err(StoreError::Conflict)));
    let doc = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(doc["label"], "newer work");
    assert_eq!(doc["metadataRevision"], "2");
    assert_ne!(
        state.sessions.lock().unwrap()[ID].info.attention.as_deref(),
        Some("needs_you")
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn native_sql_expired_clear_never_publishes_or_consumes_ownership() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, effect, _commands) = fixture().await;
    writes
        .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    let effect = clear(&state);
    let mut held = test_worker::hold_next_internal(&store, ID);
    let owner = writes.clone();
    let writer = tokio::spawn(async move {
        owner
            .native_coordinated_for_test(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await
    });
    held.entered().await;
    tokio::time::sleep(Duration::from_millis(310)).await;
    drop(held);
    assert!(matches!(writer.await.unwrap(), Err(StoreError::Expired)));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.attention.as_deref(),
        Some("needs_you")
    );
    let effect = clear(&state);
    assert_eq!(
        writes
            .native_coordinated_for_test(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap(),
        Outcome::Committed
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "3"
    );
    store.close().await.unwrap();
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_sql_stall_leaves_actual_pty_input_delivery_available() {
    use crate::runtime::session_runtime::RuntimeCommand;
    use std::io::{Read, Write};
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, effect, mut commands) = fixture().await;
    let mut held = test_worker::hold_next_internal(&store, ID);
    let writer = tokio::spawn(async move {
        writes
            .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await
    });
    held.entered().await;
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
    let reading = std::thread::spawn(move || {
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
        _ => panic!("expected ordinary input"),
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
    reading.join().unwrap();
    delivery.await.unwrap();
    drop(held);
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(echoed
        .unwrap()
        .unwrap()
        .windows(16)
        .any(|s| s == b"responsive-input"));
    // Input arrived after the execution cutoff: retain the definitive commit,
    // but do not apply or acknowledge its stale effect in the live runtime.
    assert_eq!(writer.await.unwrap().unwrap(), Outcome::CommittedDetached);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn native_sql_refuses_an_issued_effect_carried_into_a_new_runtime() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, effect, _commands) = fixture().await;
    {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        let old = h.runtime.native_approval.take();
        h.runtime = super::super::session_runtime::SessionRuntime::detached(24, 80);
        h.runtime.native_approval = old;
    }
    assert!(matches!(
        writes
            .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn native_sql_refuses_an_effect_with_already_superseded_live_revision() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, effect, _commands) = fixture().await;
    {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.runtime.native_approval.as_mut().unwrap().live_revision =
            h.runtime.attention_owner.snapshot();
        h.runtime.attention_owner.accepted_write();
    }
    assert!(matches!(
        writes
            .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn native_sql_transition_contention_rejects_execution_or_detaches_publication() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for publication in [false, true] {
        let (_dir, state, store, writes, effect, _commands) = fixture().await;
        let reservation = store
            .prepare_internal(
                ID.into(),
                CREATED.into(),
                crate::metadata::internal_mutations::InternalMutation::NativeShowWait {
                    timestamp: CREATED.into(),
                },
            )
            .unwrap();
        let lane = reservation.lane().unwrap().lock_owned().await;
        let captured = writes.test_before_callback();
        let mut held = test_worker::hold_next_internal(&store, ID);
        let writer = tokio::spawn(async move {
            writes
                .native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
                .await
        });
        captured.await.unwrap();
        if publication {
            drop(lane);
            held.entered().await;
        } else {
            let transition = super::super::prompt_authority::transition_lock()
                .lock()
                .unwrap();
            drop(lane);
            drop(reservation);
            assert!(matches!(writer.await.unwrap(), Err(StoreError::Conflict)));
            drop(transition);
            drop(held);
            assert_eq!(
                store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
                "1"
            );
            store.close().await.unwrap();
            continue;
        }
        let transition = super::super::prompt_authority::transition_lock()
            .lock()
            .unwrap();
        drop(held);
        assert_eq!(writer.await.unwrap().unwrap(), Outcome::CommittedDetached);
        drop(transition);
        drop(reservation);
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "2"
        );
        assert_ne!(
            state.sessions.lock().unwrap()[ID].info.attention.as_deref(),
            Some("needs_you")
        );
        store.close().await.unwrap();
    }
}
