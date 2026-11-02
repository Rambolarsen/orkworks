use super::*;
fn observed(root: &str) -> NativeIdentityReport {
    let mut request = report(root, None);
    request.hook_fingerprint = Some(expected_fingerprint().unwrap());
    request
}
#[tokio::test]
async fn sql_trust_records_the_external_observation_without_session_json_or_metadata_side_effects()
{
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(None).await;
    let fingerprint = expected_fingerprint().unwrap();
    assert_eq!(
        apply(&writes, &state, observed("root-one")).await.unwrap(),
        SqlIdentityOutcome::Accepted(Merge::Accepted)
    );
    let observation = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_codex_hook_observation();
    assert_eq!(
        observation.as_ref().map(|value| value.fingerprint.as_str()),
        Some(fingerprint.as_str())
    );
    assert!(chrono::DateTime::parse_from_rfc3339(&observation.unwrap().observed_at).is_ok());
    let row = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(row["metadataRevision"], "2");
    assert_eq!(row["lastActivity"], CREATED);
    assert!(!dir
        .path()
        .join("sessions")
        .join(format!("{ID}.json"))
        .exists());
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_trust_keeps_ignored_identity_observation_semantics() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    assert_eq!(
        apply(&writes, &state, observed("other-root"))
            .await
            .unwrap(),
        SqlIdentityOutcome::Accepted(Merge::IgnoredIdentityChange)
    );
    assert!(state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_codex_hook_observation()
        .is_some());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    clear_token();
    store.close().await.unwrap();
}

fn observation_root(state: &Arc<AppState>) -> std::path::PathBuf {
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
async fn sql_trust_clear_during_sql_read_cannot_be_undone_by_the_late_identity() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let mut held = crate::session_store::test_worker::hold_nth_get(&store, ID, 1);
    let owner = writes.clone();
    let app = state.clone();
    let pending = tokio::spawn(async move { apply(&owner, &app, observed("root-one")).await });
    held.entered().await;
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .clear_codex_hook_observation();
    drop(held);
    assert_eq!(
        pending.await.unwrap().unwrap(),
        SqlIdentityOutcome::Accepted(Merge::Accepted)
    );
    assert!(!observation_root(&state)
        .join("codex-hook-observation.json")
        .exists());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["resume"]["harnessSessionId"],
        "root-one"
    );
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_trust_mailbox_retains_file_contention_then_retries_the_original_packet() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay = crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    let packet = queue_identity(&relay, "root-one");
    let original = std::fs::read(&packet).unwrap();
    let stamp = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .hook_observation_stamp();
    let crate::metadata::hook_observation::Admission::Claimed(held) = stamp.try_claim() else {
        panic!("expected claim");
    };
    let first = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes.clone(),
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(first.storage_error.is_none());
    assert_eq!(first.activity.reports_consumed, 0);
    assert_eq!(std::fs::read(&packet).unwrap(), original);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    drop(held);
    let second = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(second.storage_error.is_none());
    assert_eq!(second.activity.reports_accepted, 1);
    assert!(!packet.exists());
    assert!(observation_root(&state)
        .join("codex-hook-observation.json")
        .exists());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "3"
    );
    assert!(!dir
        .path()
        .join("sessions")
        .join(format!("{ID}.json"))
        .exists());
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_trust_file_failure_is_typed_after_sql_commit_and_retries_without_json_fallback() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(None).await;
    let root = observation_root(&state);
    let failed = root.join("codex-hook-observation.json.tmp");
    std::fs::create_dir(&failed).unwrap();
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay = crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    let packet = queue_identity(&relay, "root-one");
    let first = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes.clone(),
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(matches!(first.storage_error, Some(StoreError::Io(_))));
    assert!(packet.exists());
    assert_eq!(first.activity.reports_accepted, 0);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert!(!root.join("codex-hook-observation.json").exists());
    std::fs::remove_dir(&failed).unwrap();
    let second = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(second.storage_error.is_none());
    assert_eq!(second.activity.reports_accepted, 1);
    assert!(!packet.exists());
    assert!(root.join("codex-hook-observation.json").exists());
    assert!(!dir
        .path()
        .join("sessions")
        .join(format!("{ID}.json"))
        .exists());
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_trust_bad_or_missing_fingerprint_does_not_write_the_observation_file() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for fingerprint in [None, Some("bad".into()), Some("x".repeat(64))] {
        let (_dir, state, store, writes, _commands) = fixture(None).await;
        let mut request = report("root-one", None);
        request.hook_fingerprint = fingerprint;
        assert_eq!(
            apply(&writes, &state, request).await.unwrap(),
            SqlIdentityOutcome::Accepted(Merge::Accepted)
        );
        assert!(!observation_root(&state)
            .join("codex-hook-observation.json")
            .exists());
        clear_token();
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn sql_trust_original_runtime_and_capability_are_required_after_a_stalled_read() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in ["control", "token", "workspace"] {
        let (_dir, state, store, writes, _commands) = fixture(None).await;
        let root = observation_root(&state);
        let mut held = crate::session_store::test_worker::hold_nth_get(&store, ID, 1);
        let owner = writes.clone();
        let app = state.clone();
        let pending = tokio::spawn(async move { apply(&owner, &app, observed("root-one")).await });
        held.entered().await;
        match change {
            "control" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .runtime
                    .control_tx = tokio::sync::mpsc::channel(4).0
            }
            "token" => crate::runtime::terminal_runtime::set_workflow_report_token(
                ID,
                "replaced-token".into(),
            ),
            "workspace" => {
                state.workspace.lock().unwrap().as_mut().unwrap().path =
                    std::path::PathBuf::from("/other-workspace")
            }
            _ => unreachable!(),
        }
        drop(held);
        assert_eq!(
            pending.await.unwrap().unwrap(),
            SqlIdentityOutcome::Rejected,
            "{change}"
        );
        assert!(!root.join("codex-hook-observation.json").exists());
        clear_token();
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn sql_trust_caller_abort_after_external_claim_keeps_the_definitive_file_result() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let root = observation_root(&state);
    let mut held = crate::metadata::hook_observation::testing::hold_next_write(root.clone());
    let app = state.clone();
    let pending = tokio::spawn(async move { apply(&writes, &app, observed("root-one")).await });
    held.entered().await;
    pending.abort();
    let _ = pending.await;
    held.finish().await;
    assert!(root.join("codex-hook-observation.json").exists());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    clear_token();
    store.close().await.unwrap();
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sql_trust_stalled_file_write_keeps_real_pty_input_available() {
    use crate::runtime::session_runtime::RuntimeCommand;
    use std::io::{Read, Write};
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, mut commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let mut held =
        crate::metadata::hook_observation::testing::hold_next_write(observation_root(&state));
    let pending = tokio::spawn(async move {
        writes
            .report_native_identity(
                ID.into(),
                TOKEN.into(),
                generation,
                observed("root-one"),
                Arc::new(AtomicBool::new(false)),
            )
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
        std::time::Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            ID,
            "responsive-input\r".into(),
        ),
    )
    .await;
    let echoed = observed.recv_timeout(std::time::Duration::from_secs(1));
    let _ = child.kill();
    let _ = child.wait();
    drop(pair.master);
    reading.join().unwrap();
    delivery.await.unwrap();
    held.finish().await;
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(echoed
        .unwrap()
        .unwrap()
        .windows(16)
        .any(|s| s == b"responsive-input"));
    assert_eq!(
        pending.await.unwrap().unwrap(),
        SqlIdentityOutcome::Accepted(Merge::Accepted)
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert!(!state.sessions.lock().unwrap()[ID].active_work_hook);
    assert_eq!(
        state.sessions.lock().unwrap()[ID]
            .info
            .resume
            .as_ref()
            .unwrap()
            .harness_session_id
            .as_deref(),
        Some("root-one")
    );
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_trust_abort_before_sql_claim_cannot_record_an_observation() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let root = observation_root(&state);
    let reservation = store
        .prepare_internal(
            ID.into(),
            CREATED.into(),
            InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let app = state.clone();
    let pending = tokio::spawn(async move { apply(&writes, &app, observed("root-one")).await });
    ready.await.unwrap();
    pending.abort();
    let _ = pending.await;
    drop(lane);
    drop(reservation);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    assert!(!root.join("codex-hook-observation.json").exists());
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_trust_lower_confidence_report_records_execution_without_rebinding() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    let mut request = observed("other-root");
    request.report.confidence = 0.1;
    assert_eq!(
        apply(&writes, &state, request).await.unwrap(),
        SqlIdentityOutcome::Accepted(Merge::IgnoredLowerConfidence)
    );
    assert!(observation_root(&state)
        .join("codex-hook-observation.json")
        .exists());
    let row = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(row["metadataRevision"], "1");
    assert_eq!(row["resume"]["harnessSessionId"], "root-one");
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_trust_failed_replacement_preserves_the_previous_observation_and_releases_its_claim() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    assert_eq!(
        apply(&writes, &state, observed("root-one")).await.unwrap(),
        SqlIdentityOutcome::Accepted(Merge::Accepted)
    );
    let root = observation_root(&state);
    let path = root.join("codex-hook-observation.json");
    let previous = std::fs::read(&path).unwrap();
    let tmp = root.join("codex-hook-observation.json.tmp");
    std::fs::create_dir(&tmp).unwrap();
    let mut request = observed("root-one");
    request.hook_fingerprint = Some("b".repeat(64));
    assert!(matches!(
        apply(&writes, &state, request.clone()).await,
        Err(StoreError::Io(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), previous);
    std::fs::remove_dir(&tmp).unwrap();
    assert_eq!(
        apply(&writes, &state, request).await.unwrap(),
        SqlIdentityOutcome::Accepted(Merge::IgnoredUnchanged)
    );
    let current: crate::metadata::CodexHookObservation =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(current.fingerprint, "b".repeat(64));
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_trust_mailbox_retry_cannot_renew_an_observation_cleared_after_first_receipt() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let root = observation_root(&state);
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay = crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    let packet = queue_identity(&relay, "root-one");
    let stamp = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .hook_observation_stamp();
    let crate::metadata::hook_observation::Admission::Claimed(held) = stamp.try_claim() else {
        panic!("expected claim");
    };
    let first = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes.clone(),
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert_eq!(first.activity.reports_consumed, 0);
    assert!(packet.exists());
    drop(held);
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .clear_codex_hook_observation();
    let second = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes.clone(),
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert_eq!(second.activity.reports_accepted, 1);
    assert!(!packet.exists());
    assert!(
        !root.join("codex-hook-observation.json").exists(),
        "retry must retain the original clear authority"
    );
    queue_identity(&relay, "root-one");
    let third = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert_eq!(third.activity.reports_accepted, 1);
    assert!(root.join("codex-hook-observation.json").exists());
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_trust_stamp_cannot_target_a_different_workspace_file_owner() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let other = tempfile::tempdir().unwrap();
    let foreign = crate::metadata::MetadataStore::new(other.path());
    let mut request = observed("root-one");
    request.observation_stamp = Some(foreign.hook_observation_stamp());
    assert_eq!(
        apply(&writes, &state, request).await.unwrap(),
        SqlIdentityOutcome::Rejected
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    assert!(!other.path().join("codex-hook-observation.json").exists());
    assert!(!observation_root(&state)
        .join("codex-hook-observation.json")
        .exists());
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_trust_detached_identity_cannot_record_a_live_execution_observation() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let root = observation_root(&state);
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let app = state.clone();
    let pending = tokio::spawn(async move { apply(&writes, &app, observed("root-one")).await });
    held.entered().await;
    crate::runtime::terminal_runtime::set_workflow_report_token(ID, "replacement-token".into());
    drop(held);
    assert_eq!(
        pending.await.unwrap().unwrap(),
        SqlIdentityOutcome::CommittedDetached
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert!(!root.join("codex-hook-observation.json").exists());
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_trust_cancelled_mailbox_drain_retains_original_clear_authority_on_retry() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let root = observation_root(&state);
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay =
        Arc::new(crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    let packet = queue_identity(&relay, "root-one");
    let original = std::fs::read(&packet).unwrap();
    let mut held = crate::session_store::test_worker::hold_nth_get(&store, ID, 1);
    let owner = relay.clone();
    let app = state.clone();
    let sql = writes.clone();
    let pending = tokio::spawn(async move {
        owner
            .consume_ready_sql(
                app,
                ID,
                TOKEN,
                generation,
                sql,
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    held.entered().await;
    pending.abort();
    let _ = pending.await;
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .clear_codex_hook_observation();
    drop(held);
    assert_eq!(std::fs::read(&packet).unwrap(), original);
    let retry = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(retry.storage_error.is_none());
    assert_eq!(retry.activity.reports_accepted, 1);
    assert!(!packet.exists());
    assert!(!root.join("codex-hook-observation.json").exists());
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_trust_replaced_file_owner_under_the_same_lease_rejects_old_capture() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let root = observation_root(&state);
    let mut held = crate::session_store::test_worker::hold_nth_get(&store, ID, 1);
    let owner = writes.clone();
    let app = state.clone();
    let pending = tokio::spawn(async move { apply(&owner, &app, observed("root-one")).await });
    held.entered().await;
    state.workspace.lock().unwrap().as_mut().unwrap().metadata =
        crate::metadata::MetadataStore::new(&root);
    drop(held);
    assert_eq!(
        pending.await.unwrap().unwrap(),
        SqlIdentityOutcome::Accepted(Merge::Accepted)
    );
    assert!(!root.join("codex-hook-observation.json").exists());
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_trust_cancellation_during_mailbox_read_cannot_renew_clear_authority() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let root = observation_root(&state);
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay =
        Arc::new(crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    let packet = queue_identity(&relay, "root-one");
    let mut held = crate::runtime::codex_hook_report_relay::read_testing::hold_next(
        packet.file_name().unwrap().to_owned(),
    );
    let owner = relay.clone();
    let app = state.clone();
    let sql = writes.clone();
    let pending = tokio::spawn(async move {
        owner
            .consume_ready_sql(
                app,
                ID,
                TOKEN,
                generation,
                sql,
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    held.entered().await;
    pending.abort();
    let _ = pending.await;
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .clear_codex_hook_observation();
    let retry = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    drop(held);
    assert!(retry.storage_error.is_none());
    assert_eq!(retry.activity.reports_accepted, 1);
    assert!(!packet.exists());
    assert!(
        !root.join("codex-hook-observation.json").exists(),
        "cancelled mailbox read must retain its admitted observation epoch"
    );
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_trust_receipt_overflow_cannot_renew_observations_after_clear() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let root = observation_root(&state);
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay =
        Arc::new(crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    let mut packets = (0..64)
        .map(|_| queue_identity(&relay, "root-one"))
        .collect::<Vec<_>>();
    let stamp = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .hook_observation_stamp();
    let crate::metadata::hook_observation::Admission::Claimed(held) = stamp.try_claim() else {
        panic!("expected claim");
    };
    for _ in 0..12 {
        let result = relay
            .consume_ready_sql(
                state.clone(),
                ID,
                TOKEN,
                generation,
                writes.clone(),
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert!(result.storage_error.is_none());
        assert_eq!(result.activity.reports_consumed, 0);
        if crate::runtime::codex_hook_report_relay::read_testing::pending_count(&relay) == 64 {
            break;
        }
    }
    assert_eq!(
        crate::runtime::codex_hook_report_relay::read_testing::pending_count(&relay),
        64,
        "overflow fixture must fill every receipt slot"
    );
    packets.push(queue_identity(&relay, "root-one"));
    let mut reading = crate::runtime::codex_hook_report_relay::read_testing::hold_next(
        packets.last().unwrap().file_name().unwrap().to_owned(),
    );
    let owner = relay.clone();
    let app = state.clone();
    let sql = writes.clone();
    let overflow = tokio::spawn(async move {
        for _ in 0..8 {
            let result = owner
                .consume_ready_sql(
                    app.clone(),
                    ID,
                    TOKEN,
                    generation,
                    sql.clone(),
                    Arc::new(AtomicBool::new(false)),
                )
                .await;
            assert!(result.storage_error.is_none());
            assert_eq!(result.activity.reports_consumed, 0);
        }
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), reading.entered())
        .await
        .unwrap();
    drop(reading);
    overflow.await.unwrap();
    drop(held);
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .clear_codex_hook_observation();
    for _ in 0..8 {
        let result = relay
            .consume_ready_sql(
                state.clone(),
                ID,
                TOKEN,
                generation,
                writes.clone(),
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert!(result.storage_error.is_none());
        if packets.iter().all(|path| !path.exists()) {
            break;
        }
    }
    assert!(
        packets.iter().all(|path| !path.exists()),
        "identity packets must remain deliverable"
    );
    assert!(
        !root.join("codex-hook-observation.json").exists(),
        "overflow retry must not renew cleared observation authority"
    );
    queue_identity(&relay, "root-one");
    relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes.clone(),
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(
        !root.join("codex-hook-observation.json").exists(),
        "overflow floor is conservative for the rest of this relay"
    );
    let fresh = crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    queue_identity(&fresh, "root-one");
    let result = fresh
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(result.storage_error.is_none());
    assert_eq!(result.activity.reports_accepted, 1);
    assert!(root.join("codex-hook-observation.json").exists());
    clear_token();
    store.close().await.unwrap();
}
