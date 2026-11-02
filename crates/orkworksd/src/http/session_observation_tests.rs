use super::*;
use crate::test_support::{test_app_state_with_workspace, test_session_metadata};
use std::time::Duration;

fn fixture() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let mut meta = test_session_metadata(
        "observation",
        "Before",
        dir.path().display().to_string(),
        "running",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:00Z",
    );
    meta.harness = "codex".into();
    meta.lifecycle = "alive".into();
    meta.lifecycle_phase = "active".into();
    {
        let mut workspace = state.workspace.lock().unwrap();
        let ws = workspace.as_mut().unwrap();
        ws.lease = Some(Arc::new(
            crate::workspace_runtime::WorkspaceLease::acquire(&ws.metadata.root_path()).unwrap(),
        ));
        ws.metadata.try_write_session(&meta).unwrap();
    }
    (dir, state)
}
fn request() -> HarnessSessionReportRequest {
    HarnessSessionReportRequest {
        harness_session_id: "native-observation".into(),
        source: "codex_hook".into(),
        confidence: 0.98,
        hook_fingerprint: Some("a".repeat(64)),
        session_start_source: None,
        session_start_event: None,
        prompt_hook_generation: None,
        session_start_observed_at: None,
    }
}

// Returning to synchronous external-file work under either application guard
// must fail this test, even when it blocks the async executor itself.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_stalled_write_does_not_hold_application_guards() {
    let (_dir, state) = fixture();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut pause = crate::metadata::hook_observation::testing::hold_next_write(root);
    let checking = state.clone();
    let watchdog = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let entered = runtime
            .block_on(async {
                tokio::time::timeout(Duration::from_secs(10), pause.entered()).await
            })
            .is_ok();
        let projection = checking.projection_lock.try_lock().is_ok();
        let workspace = checking.workspace.try_lock().is_ok();
        drop(pause);
        (entered, projection, workspace)
    });
    let response = report_harness_session(
        State(state.clone()),
        Path("observation".into()),
        Json(request()),
    )
    .await
    .into_response();
    let (entered, projection, workspace) = watchdog.join().unwrap();
    assert!(entered, "the actual observation write was not reached");
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert!(projection, "observation I/O retained the projection guard");
    assert!(workspace, "observation I/O retained the workspace guard");
    assert_eq!(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_codex_hook_observation()
            .unwrap()
            .fingerprint,
        "a".repeat(64)
    );
}

fn add_runtime(
    state: &Arc<AppState>,
) -> (
    u64,
    tokio::sync::mpsc::Receiver<crate::runtime::session_runtime::RuntimeCommand>,
) {
    let meta = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_session("observation")
        .unwrap();
    let mut info = crate::session_types::SessionInfo::baseline_from_metadata(&meta);
    info.harness = Some("codex".into());
    info.harness_id = Some("codex".into());
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    let (runtime, commands) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let generation = runtime.run_generation();
    state.sessions.lock().unwrap().insert(
        "observation".into(),
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
    (generation, commands)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_cancelled_mailbox_read_keeps_original_clear_epoch() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state) = fixture();
    let (generation, _commands) = add_runtime(&state);
    let token = "json-observation-read-token";
    crate::runtime::terminal_runtime::set_workflow_report_token("observation", token.into());
    let relay =
        Arc::new(crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    let name = std::ffi::OsString::from(format!("{}.json", uuid::Uuid::new_v4().simple()));
    std::fs::write(relay.mailbox_path().join(&name), serde_json::to_vec(&serde_json::json!({
        "report": {"harnessSessionId":"native-observation", "source":"codex_hook", "confidence":0.98, "hookFingerprint":"a".repeat(64)}
    })).unwrap()).unwrap();
    let mut held = crate::runtime::codex_hook_report_relay::read_testing::hold_next(name);
    let app = state.clone();
    let drain = relay.clone();
    let pending = tokio::spawn(async move {
        drain
            .consume_ready(app, "observation", token, generation)
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), held.entered())
        .await
        .expect("valid live relay never reached mailbox read");
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    let owner = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .hook_observation_owner();
    owner.clear().unwrap();
    drop(held);
    let result = relay
        .consume_ready(state.clone(), "observation", token, generation)
        .await;
    crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
    assert_eq!(result.reports_accepted, 1);
    assert!(
        owner.read().is_none(),
        "retry renewed a packet's observation authority after clear"
    );
    assert_eq!(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session("observation")
            .unwrap()
            .resume
            .unwrap()
            .harness_session_id
            .as_deref(),
        Some("native-observation")
    );
}

fn owner(state: &AppState) -> Arc<crate::metadata::hook_observation::Store> {
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .hook_observation_owner()
}
async fn capture(state: &AppState) -> crate::session_application::hook_observation::Capture {
    crate::session_application::hook_observation::capture_target(state, "observation", true, None)
        .await
        .unwrap()
        .1
        .unwrap()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_rejects_stale_original_context_before_file_claim() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "clear",
        "workspace",
        "owner",
        "creation",
        "runtime",
        "control",
        "token",
        "lease",
        "adoption",
    ] {
        let (_dir, state) = fixture();
        let file = owner(&state);
        let _commands = if change != "adoption" {
            Some(add_runtime(&state).1)
        } else {
            None
        };
        let mut captured = capture(&state).await;
        let replacement = tempfile::tempdir().unwrap();
        match change {
            "clear" => file.clear().unwrap(),
            "workspace" => crate::test_support::swap_workspace(&state, replacement.path()),
            "owner" => {
                let mut ws = state.workspace.lock().unwrap();
                let ws = ws.as_mut().unwrap();
                ws.metadata = metadata::MetadataStore::new(&ws.metadata.root_path());
            }
            "creation" => {
                let ws = state.workspace.lock().unwrap();
                let metadata = &ws.as_ref().unwrap().metadata;
                let mut meta = metadata.read_session("observation").unwrap();
                meta.created_at = "replacement-created".into();
                metadata.try_write_session(&meta).unwrap();
            }
            "runtime" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80)
            }
            "control" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .runtime
                    .control_tx = tokio::sync::mpsc::channel(8).0
            }
            "token" => {
                let token = "original-observation-token";
                crate::runtime::terminal_runtime::set_workflow_report_token(
                    "observation",
                    token.into(),
                );
                captured.authenticate(token);
                crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
            }
            "lease" => state.workspace.lock().unwrap().as_mut().unwrap().lease = None,
            "adoption" => {
                let _ = add_runtime(&state);
            }
            _ => unreachable!(),
        }
        let result = SessionApplication::new(state)
            .record_codex_hook_observation("observation", &"a".repeat(64), Ok(captured))
            .await;
        assert!(
            result.is_ok(),
            "stale {change} should be skipped: {result:?}"
        );
        assert!(
            file.read().is_none(),
            "stale {change} recorded an observation"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_busy_file_is_best_effort_and_fresh_report_recovers() {
    let (_dir, state) = fixture();
    let file = owner(&state);
    let crate::metadata::hook_observation::Admission::Claimed(held) = file.stamp().try_claim()
    else {
        panic!("expected free file");
    };
    let (release, released) = std::sync::mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        let _ = released.recv_timeout(Duration::from_secs(3));
        drop(held);
    });
    let start = std::time::Instant::now();
    let response = report_harness_session(
        State(state.clone()),
        Path("observation".into()),
        Json(request()),
    )
    .await
    .into_response();
    let elapsed = start.elapsed();
    let _ = release.send(());
    watchdog.join().unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert!(
        elapsed < Duration::from_secs(1),
        "optional telemetry waited behind the file owner"
    );
    assert!(file.read().is_none());
    let response = report_harness_session(
        State(state.clone()),
        Path("observation".into()),
        Json(request()),
    )
    .await
    .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(file.read().unwrap().fingerprint, "a".repeat(64));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_failure_does_not_undo_identity_or_replace_previous_file() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state) = fixture();
    let file = owner(&state);
    let previous = metadata::CodexHookObservation {
        fingerprint: "b".repeat(64),
        observed_at: "before".into(),
    };
    file.write(&previous).unwrap();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    std::fs::create_dir(root.join("codex-hook-observation.json.tmp")).unwrap();
    let response = report_harness_session(
        State(state.clone()),
        Path("observation".into()),
        Json(request()),
    )
    .await
    .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(file.read(), Some(previous));
    assert_eq!(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session("observation")
            .unwrap()
            .resume
            .unwrap()
            .harness_session_id
            .as_deref(),
        Some("native-observation")
    );
    std::fs::remove_dir(root.join("codex-hook-observation.json.tmp")).unwrap();
    let _ = report_harness_session(
        State(state.clone()),
        Path("observation".into()),
        Json(request()),
    )
    .await
    .into_response();
    assert_eq!(file.read().unwrap().fingerprint, "a".repeat(64));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_missing_or_invalid_fingerprint_does_not_write() {
    for fingerprint in [None, Some("invalid".into())] {
        let (_dir, state) = fixture();
        let mut report = request();
        report.hook_fingerprint = fingerprint;
        let response = report_harness_session(
            State(state.clone()),
            Path("observation".into()),
            Json(report),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(owner(&state).read().is_none());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_claimed_write_outlives_cancelled_waiter_and_retains_old_lease() {
    let (_dir, state) = fixture();
    let file = owner(&state);
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
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
    let mut pause = crate::metadata::hook_observation::testing::hold_next_write(root);
    let app = state.clone();
    let task = tokio::spawn(async move {
        report_harness_session(State(app), Path("observation".into()), Json(request()))
            .await
            .into_response()
    });
    tokio::time::timeout(Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let next = tempfile::tempdir().unwrap();
    crate::test_support::swap_workspace(&state, next.path());
    assert!(
        weak.upgrade().is_some(),
        "claimed file write lost its workspace lease"
    );
    pause.finish().await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while weak.upgrade().is_some() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(file.read().unwrap().fingerprint, "a".repeat(64));
    assert!(owner(&state).read().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_clear_follows_claimed_write_and_prevents_resurrection() {
    let (_dir, state) = fixture();
    let file = owner(&state);
    let stamp = file.stamp();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut pause = crate::metadata::hook_observation::testing::hold_next_write(root);
    let app = state.clone();
    let task = tokio::spawn(async move {
        report_harness_session(State(app), Path("observation".into()), Json(request()))
            .await
            .into_response()
    });
    tokio::time::timeout(Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    let clearing = file.clone();
    let clear = tokio::task::spawn_blocking(move || clearing.clear());
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(
                stamp.clone().try_claim(),
                crate::metadata::hook_observation::Admission::Obsolete
            ) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(!clear.is_finished());
    pause.finish().await;
    assert_eq!(task.await.unwrap().status(), axum::http::StatusCode::OK);
    clear.await.unwrap().unwrap();
    assert!(file.read().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_lower_confidence_identity_still_records_execution() {
    let (_dir, state) = fixture();
    {
        let ws = state.workspace.lock().unwrap();
        let metadata = &ws.as_ref().unwrap().metadata;
        let mut meta = metadata.read_session("observation").unwrap();
        meta.resume = Some(harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::Exact,
            harness_session_id: Some("original-native".into()),
            latest_fallback: false,
            last_seen_at: Some("before".into()),
        });
        meta.harness_session_id_confidence = Some(1.0);
        meta.harness_session_id_source = Some("codex_hook".into());
        metadata.try_write_session(&meta).unwrap();
    }
    let mut report = request();
    report.confidence = 0.2;
    report.harness_session_id = "original-native".into();
    let response = report_harness_session(
        State(state.clone()),
        Path("observation".into()),
        Json(report),
    )
    .await
    .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(owner(&state).read().unwrap().fingerprint, "a".repeat(64));
    assert_eq!(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session("observation")
            .unwrap()
            .harness_session_id_confidence,
        Some(1.0)
    );
}

fn queue_identity(
    relay: &crate::runtime::codex_hook_report_relay::CodexHookReportRelay,
) -> std::ffi::OsString {
    let name = std::ffi::OsString::from(format!("{}.json", uuid::Uuid::new_v4().simple()));
    std::fs::write(relay.mailbox_path().join(&name), serde_json::to_vec(&serde_json::json!({
        "report": {"harnessSessionId":"native-observation", "source":"codex_hook", "confidence":0.98, "hookFingerprint":"a".repeat(64)}
    })).unwrap()).unwrap();
    name
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_cancelled_unlink_retains_receipt_until_definitive_removal() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state) = fixture();
    let (generation, _commands) = add_runtime(&state);
    let token = "json-observation-unlink-token";
    crate::runtime::terminal_runtime::set_workflow_report_token("observation", token.into());
    let relay =
        Arc::new(crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    let name = queue_identity(&relay);
    let mut held = crate::runtime::codex_hook_report_relay::read_testing::hold_next_unlink(name);
    let drain = relay.clone();
    let app = state.clone();
    let task = tokio::spawn(async move {
        drain
            .consume_ready(app, "observation", token, generation)
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), held.entered())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let file = owner(&state);
    file.clear().unwrap();
    let retry = relay
        .consume_ready(state.clone(), "observation", token, generation)
        .await;
    drop(held);
    crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
    assert_eq!(retry.reports_accepted, 1);
    assert!(
        file.read().is_none(),
        "cancelled unlink renewed an already accepted packet's clear authority"
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_actual_pty_input_continues_during_stalled_file_write() {
    actual_pty_input_during_identity_io(false).await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_identity_actual_pty_input_continues_during_stalled_target_read() {
    actual_pty_input_during_identity_io(true).await;
}

#[cfg(unix)]
async fn actual_pty_input_during_identity_io(initial_read: bool) {
    use crate::runtime::session_runtime::RuntimeCommand;
    use std::io::{Read, Write};
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state) = fixture();
    let (_generation, mut commands) = add_runtime(&state);
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut write_pause = (!initial_read)
        .then(|| crate::metadata::hook_observation::testing::hold_next_write(root.clone()));
    let mut read_pause = initial_read
        .then(|| crate::session_application::hook_observation::testing::hold_next(root));
    let app = state.clone();
    let task = tokio::spawn(async move {
        report_harness_session(State(app), Path("observation".into()), Json(request()))
            .await
            .into_response()
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        if let Some(pause) = write_pause.as_mut() {
            pause.entered().await;
        }
        if let Some(pause) = read_pause.as_mut() {
            pause.entered().await;
        }
    })
    .await
    .unwrap();
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
    let mut writer = pair.master.take_writer().unwrap();
    let (seen, observed) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buf = [0; 4096];
        while let Ok(count) = reader.read(&mut buf) {
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..count]);
            if bytes.windows(16).any(|part| part == b"responsive-input") || bytes.len() > 16384 {
                break;
            }
        }
        let _ = seen.send(bytes);
    });
    let delivery = tokio::task::spawn_blocking(move || match commands.blocking_recv().unwrap() {
        RuntimeCommand::Input {
            data,
            accepted: Some(accepted),
        } => {
            let result = writer
                .write_all(data.as_bytes())
                .and_then(|()| writer.flush())
                .map_err(|_| ());
            let _ = accepted.send(result);
        }
        _ => panic!("expected ordinary input"),
    });
    let delivered = tokio::time::timeout(
        Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            "observation",
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
    if let Some(pause) = write_pause {
        pause.finish().await;
    }
    drop(read_pause);
    assert_eq!(task.await.unwrap().status(), axum::http::StatusCode::OK);
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(echoed
        .unwrap()
        .windows(16)
        .any(|part| part == b"responsive-input"));
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_overflow_cannot_renew_epochs_after_clear() {
    use std::os::unix::fs::PermissionsExt;
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state) = fixture();
    let (generation, _commands) = add_runtime(&state);
    let token = "json-observation-overflow-token";
    crate::runtime::terminal_runtime::set_workflow_report_token("observation", token.into());
    let relay =
        Arc::new(crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    struct Restore(std::path::PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
        }
    }
    let _restore = Restore(relay.mailbox_path().to_owned());
    let mut names = (0..64).map(|_| queue_identity(&relay)).collect::<Vec<_>>();
    let file = owner(&state);
    let crate::metadata::hook_observation::Admission::Claimed(held_file) = file.stamp().try_claim()
    else {
        panic!("expected free file");
    };
    // Failed unlink retains actual packets and their original bounded receipts.
    std::fs::set_permissions(relay.mailbox_path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    for _ in 0..12 {
        relay
            .consume_ready(state.clone(), "observation", token, generation)
            .await;
        if crate::runtime::codex_hook_report_relay::read_testing::pending_count(&relay) == 64 {
            break;
        }
    }
    assert_eq!(
        crate::runtime::codex_hook_report_relay::read_testing::pending_count(&relay),
        64
    );
    std::fs::set_permissions(relay.mailbox_path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let extra = queue_identity(&relay);
    names.push(extra.clone());
    std::fs::set_permissions(relay.mailbox_path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let mut held_read = crate::runtime::codex_hook_report_relay::read_testing::hold_next(extra);
    let mut entered = Box::pin(held_read.entered());
    let mut saw_overflow = false;
    for _ in 0..4 {
        let owner = relay.clone();
        let app = state.clone();
        let mut task = tokio::spawn(async move {
            owner
                .consume_ready(app, "observation", token, generation)
                .await
        });
        tokio::select! {
            _ = &mut entered => { saw_overflow = true; task.abort(); let _ = task.await; break; },
            result = &mut task => { result.unwrap(); },
            _ = tokio::time::sleep(Duration::from_secs(5)) => { task.abort(); let _ = task.await; break; },
        }
    }
    drop(entered);
    assert!(
        saw_overflow,
        "overflow packet was never actually read before clear"
    );
    drop(held_file);
    file.clear().unwrap();
    drop(held_read);
    std::fs::set_permissions(relay.mailbox_path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    for _ in 0..12 {
        relay
            .consume_ready(state.clone(), "observation", token, generation)
            .await;
        assert!(
            file.read().is_none(),
            "overflow or failed-unlink retry renewed authority after clear"
        );
        if names
            .iter()
            .all(|name| !relay.mailbox_path().join(name).exists())
        {
            break;
        }
    }
    assert!(names
        .iter()
        .all(|name| !relay.mailbox_path().join(name).exists()));
    let fresh = crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    queue_identity(&fresh);
    assert_eq!(
        fresh
            .consume_ready(state.clone(), "observation", token, generation)
            .await
            .reports_accepted,
        1
    );
    crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
    assert_eq!(file.read().unwrap().fingerprint, "a".repeat(64));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_cancelled_delivery_does_not_adopt_replaced_packet_bytes() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state) = fixture();
    let (generation, _commands) = add_runtime(&state);
    let token = "json-observation-immutable-token";
    crate::runtime::terminal_runtime::set_workflow_report_token("observation", token.into());
    let relay =
        Arc::new(crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    let name = queue_identity(&relay);
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut held = crate::metadata::hook_observation::testing::hold_next_write(root);
    let drain = relay.clone();
    let app = state.clone();
    let task = tokio::spawn(async move {
        drain
            .consume_ready(app, "observation", token, generation)
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), held.entered())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    std::fs::write(relay.mailbox_path().join(&name), serde_json::to_vec(&serde_json::json!({
        "report": {"harnessSessionId":"different-native", "source":"codex_hook", "confidence":0.98, "hookFingerprint":"b".repeat(64)}
    })).unwrap()).unwrap();
    held.finish().await;
    let file = owner(&state);
    file.clear().unwrap();
    let result = relay
        .consume_ready(state.clone(), "observation", token, generation)
        .await;
    crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
    assert_eq!(
        result.reports_accepted, 0,
        "a replacement packet was delivered using an existing filename's receipt"
    );
    assert!(!relay.mailbox_path().join(name).exists());
    assert!(file.read().is_none());
}

#[derive(Clone, Default)]
struct Logs(Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for Logs {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Logs {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_definitive_failure_is_logged_after_waiter_cancellation() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    use tracing::instrument::WithSubscriber;
    let (_dir, state) = fixture();
    let file = owner(&state);
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut pause = crate::metadata::hook_observation::testing::hold_next_write(root.clone());
    let logs = Logs::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(logs.clone())
        .finish();
    let app = state.clone();
    let task = tokio::spawn(
        async move {
            report_harness_session(State(app), Path("observation".into()), Json(request()))
                .await
                .into_response()
        }
        .with_subscriber(subscriber),
    );
    tokio::time::timeout(Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    std::fs::create_dir(root.join("codex-hook-observation.json.tmp")).unwrap();
    pause.finish().await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(
                file.stamp().try_claim(),
                crate::metadata::hook_observation::Admission::Claimed(_)
            ) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    let logged = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert_eq!(
        logged
            .matches("failed to record Codex hook observation")
            .count(),
        1,
        "{logged}"
    );
    assert!(logged.contains("session_id=observation"), "{logged}");
    assert!(file.read().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_observation_authenticated_request_does_not_downgrade_during_initial_read() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state) = fixture();
    let file = owner(&state);
    let token = "json-observation-initial-token";
    crate::runtime::terminal_runtime::set_workflow_report_token("observation", token.into());
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut held = crate::session_application::hook_observation::testing::hold_next(root);
    let watchdog = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let entered = runtime
            .block_on(async { tokio::time::timeout(Duration::from_secs(5), held.entered()).await })
            .is_ok();
        crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
        drop(held);
        entered
    });
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer json-observation-initial-token"),
    );
    let response = report_harness_session_with_headers(
        State(state),
        Path("observation".into()),
        headers,
        Json(request()),
    )
    .await
    .into_response();
    assert!(watchdog.join().unwrap());
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert!(
        file.read().is_none(),
        "an authenticated request silently became bearerless after token revocation"
    );
}

// A stalled first metadata lookup must not retain the workspace guard or
// prevent the async worker from handling unrelated work.
#[tokio::test]
async fn json_identity_target_read_releases_application_guards() {
    let (_dir, state) = fixture();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut held = crate::session_application::hook_observation::testing::hold_next(root);
    let checking = state.clone();
    let (go, started) = tokio::sync::oneshot::channel();
    let (done, heartbeat) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let _ = started.await;
        let _ = done.send(());
    });
    let watchdog = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let entered = runtime
            .block_on(async { tokio::time::timeout(Duration::from_secs(10), held.entered()).await })
            .is_ok();
        let _ = go.send(());
        let executor_available = runtime
            .block_on(async { tokio::time::timeout(Duration::from_secs(1), heartbeat).await })
            .is_ok();
        let workspace_available = checking.workspace.try_lock().is_ok();
        let projection_available = checking.projection_lock.try_lock().is_ok();
        let sessions_available = checking.sessions.try_lock().is_ok();
        drop(held);
        (
            entered,
            workspace_available,
            projection_available,
            sessions_available,
            executor_available,
        )
    });
    let response =
        report_harness_session(State(state), Path("observation".into()), Json(request()))
            .await
            .into_response();
    let (entered, workspace, projection, sessions, executor) = watchdog.join().unwrap();
    assert!(entered, "initial identity read was not reached");
    assert!(executor, "identity read blocked the single Tokio worker");
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert!(workspace, "identity target read retained workspace guard");
    assert!(
        projection && sessions,
        "identity target read retained other application guards"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_identity_target_read_rejects_replaced_original_context() {
    for change in [
        "workspace",
        "owner",
        "lease",
        "runtime",
        "control",
        "creation",
        "record_creation",
        "removal",
        "adoption",
    ] {
        let (_dir, state) = fixture();
        let _commands = (change != "adoption").then(|| add_runtime(&state).1);
        let file = owner(&state);
        let root = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path();
        let mut held = crate::session_application::hook_observation::testing::hold_next(root);
        let app = state.clone();
        let pending = tokio::spawn(async move {
            report_harness_session(State(app), Path("observation".into()), Json(request()))
                .await
                .into_response()
        });
        tokio::time::timeout(Duration::from_secs(5), held.entered())
            .await
            .unwrap();
        assert!(state.workspace.try_lock().is_ok());
        let replacement = tempfile::tempdir().unwrap();
        match change {
            "workspace" => crate::test_support::swap_workspace(&state, replacement.path()),
            "owner" => {
                let mut selected = state.workspace.lock().unwrap();
                let ws = selected.as_mut().unwrap();
                ws.metadata = metadata::MetadataStore::new(&ws.metadata.root_path());
            }
            "lease" => state.workspace.lock().unwrap().as_mut().unwrap().lease = None,
            "runtime" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80)
            }
            "control" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .runtime
                    .control_tx = tokio::sync::mpsc::channel(8).0
            }
            "creation" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .info
                    .created_at = "replacement-created".into()
            }
            "record_creation" => {
                let selected = state.workspace.lock().unwrap();
                let metadata = &selected.as_ref().unwrap().metadata;
                let mut meta = metadata.read_session("observation").unwrap();
                meta.created_at = "replacement-created".into();
                metadata.try_write_session(&meta).unwrap();
            }
            "removal" => {
                state.sessions.lock().unwrap().remove("observation");
            }
            "adoption" => {
                let _ = add_runtime(&state);
            }
            _ => unreachable!(),
        }
        drop(held);
        let response = tokio::time::timeout(Duration::from_secs(5), pending)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::CONFLICT,
            "{change}"
        );
        assert!(file.read().is_none(), "stale {change} wrote observation");
        let ws = state.workspace.lock().unwrap();
        if let Some(meta) = ws.as_ref().unwrap().metadata.read_session("observation") {
            assert!(
                meta.resume.and_then(|r| r.harness_session_id).is_none(),
                "stale {change} merged identity"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_identity_cancelled_read_retains_lease_and_bounded_admission() {
    let (_dir, state) = fixture();
    let (root, reader, lease) = {
        let selected = state.workspace.lock().unwrap();
        let ws = selected.as_ref().unwrap();
        (
            ws.metadata.root_path(),
            ws.metadata.session_reader(),
            Arc::downgrade(ws.lease.as_ref().unwrap()),
        )
    };
    let mut held = crate::session_application::hook_observation::testing::hold_next(root.clone());
    let app = state.clone();
    let pending = tokio::spawn(async move {
        report_harness_session(State(app), Path("observation".into()), Json(request()))
            .await
            .into_response()
    });
    tokio::time::timeout(Duration::from_secs(5), held.entered())
        .await
        .unwrap();
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    let permits: Vec<_> = (0..7).map(|_| reader.try_admit().unwrap()).collect();
    assert!(
        reader.try_admit().is_none(),
        "cancelled read released a slot before file completion"
    );
    let replacement = tempfile::tempdir().unwrap();
    crate::test_support::swap_workspace(&state, replacement.path());
    assert!(
        lease.upgrade().is_some(),
        "old read lost its original workspace lease"
    );
    assert!(crate::workspace_runtime::WorkspaceLease::acquire(&root).is_err());
    drop(held);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(permit) = reader.try_admit() {
                drop(permit);
                if lease.upgrade().is_none() {
                    break;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(lease.upgrade().is_none());
    drop(permits);
}

#[tokio::test]
async fn json_identity_read_overload_is_retryable_without_identity_or_file_writes() {
    let (_dir, state) = fixture();
    let reader = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .session_reader();
    let held: Vec<_> = (0..8).map(|_| reader.try_admit().unwrap()).collect();
    let response = report_harness_session(
        State(state.clone()),
        Path("observation".into()),
        Json(request()),
    )
    .await
    .into_response();
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    assert!(owner(&state).read().is_none());
    let meta = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_session("observation")
        .unwrap();
    assert!(meta.resume.and_then(|r| r.harness_session_id).is_none());
    drop(held);
    let response = report_harness_session(
        State(state.clone()),
        Path("observation".into()),
        Json(request()),
    )
    .await
    .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert!(owner(&state).read().is_some());
}

#[tokio::test]
async fn json_identity_relay_retains_packet_on_target_read_overload() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state) = fixture();
    let (generation, _commands) = add_runtime(&state);
    let token = "identity-read-overload-token";
    crate::runtime::terminal_runtime::set_workflow_report_token("observation", token.into());
    let relay = crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    let path = relay
        .mailbox_path()
        .join(format!("{}.json", uuid::Uuid::new_v4().simple()));
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({"report": {
            "harnessSessionId":"native-observation", "source":"codex_hook", "confidence":0.98,
            "hookFingerprint":"a".repeat(64)
        }}))
        .unwrap(),
    )
    .unwrap();
    let reader = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .session_reader();
    let held: Vec<_> = (0..8).map(|_| reader.try_admit().unwrap()).collect();
    let outcome = relay
        .consume_ready(state.clone(), "observation", token, generation)
        .await;
    assert_eq!(outcome.reports_accepted, 0);
    assert!(
        path.exists(),
        "retryable read overload discarded identity packet"
    );
    owner(&state).clear().unwrap();
    drop(held);
    let outcome = relay
        .consume_ready(state.clone(), "observation", token, generation)
        .await;
    assert_eq!(outcome.reports_accepted, 1);
    assert!(!path.exists());
    assert!(
        owner(&state).read().is_none(),
        "retry renewed the original clear epoch"
    );
    crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_identity_corrupt_read_cannot_quarantine_a_valid_replacement() {
    let (_dir, state) = fixture();
    let (path, mut valid) = {
        let selected = state.workspace.lock().unwrap();
        let metadata = &selected.as_ref().unwrap().metadata;
        (
            metadata.sessions_dir().join("observation.json"),
            metadata.read_session("observation").unwrap(),
        )
    };
    std::fs::write(&path, "{broken").unwrap();
    let mut held = metadata::session_reader::testing::hold_next_quarantine(path.clone());
    let app = state.clone();
    let pending = tokio::spawn(async move {
        report_harness_session(State(app), Path("observation".into()), Json(request()))
            .await
            .into_response()
    });
    tokio::time::timeout(Duration::from_secs(5), held.entered())
        .await
        .unwrap();
    assert!(state.workspace.try_lock().is_ok());
    valid.label = "Valid replacement".into();
    // A repair replaces the corrupt record through the actual atomic writer.
    std::fs::remove_file(&path).unwrap();
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .try_write_session(&valid)
        .unwrap();
    drop(held);
    let response = tokio::time::timeout(Duration::from_secs(5), pending)
        .await
        .unwrap()
        .unwrap();
    assert!(
        path.exists(),
        "stale corrupt read quarantined the replacement valid record"
    );
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session("observation")
            .unwrap()
            .label,
        "Valid replacement"
    );
    assert!(!path.with_extension("json.corrupt").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_identity_quarantine_and_replacement_share_the_existing_write_lane() {
    let (_dir, state) = fixture();
    let (path, mut valid) = {
        let selected = state.workspace.lock().unwrap();
        let metadata = &selected.as_ref().unwrap().metadata;
        (
            metadata.sessions_dir().join("observation.json"),
            metadata.read_session("observation").unwrap(),
        )
    };
    std::fs::write(&path, "{broken").unwrap();
    let mut held = metadata::session_reader::testing::hold_next_rename(&path);
    let app = state.clone();
    let pending = tokio::spawn(async move {
        report_harness_session(State(app), Path("observation".into()), Json(request()))
            .await
            .into_response()
    });
    tokio::time::timeout(Duration::from_secs(5), held.entered())
        .await
        .unwrap();
    valid.label = "Valid replacement".into();
    let app = state.clone();
    let (started, beginning) = tokio::sync::oneshot::channel();
    let mut writer = tokio::task::spawn_blocking(move || {
        let selected = app.workspace.lock().unwrap();
        let metadata = &selected.as_ref().unwrap().metadata;
        let _ = started.send(());
        metadata.try_write_session(&valid)
    });
    beginning.await.unwrap();
    let early = tokio::time::timeout(Duration::from_millis(100), &mut writer).await;
    drop(held);
    assert!(
        early.is_err(),
        "writer did not serialize with the admitted quarantine"
    );
    tokio::time::timeout(Duration::from_secs(5), writer)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let response = tokio::time::timeout(Duration::from_secs(5), pending)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session("observation")
            .unwrap()
            .label,
        "Valid replacement"
    );
    assert!(path.exists());
    assert!(path.with_extension("json.corrupt").exists());
}
