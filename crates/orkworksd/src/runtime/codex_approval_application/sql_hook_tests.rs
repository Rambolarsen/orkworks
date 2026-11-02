use super::sql_tests::{fixture_with_root, ID};
use super::*;
use crate::session_application::sql_writes::SqlHookOutcome;
use std::sync::atomic::AtomicBool;
const TOKEN: &str = "sql-hook-report-token";
fn report(event: HookEvent, at: chrono::DateTime<chrono::Utc>) -> NativeHookReport {
    NativeHookReport {
        root_id: "root".into(),
        turn_id: Some("turn".into()),
        tool_use_id: None,
        event,
        observed_at: at.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        hook_fingerprint: expected_fingerprint().unwrap(),
    }
}
#[tokio::test]
async fn sql_hook_relay_activates_then_applies_boundaries_without_replaying_an_old_permission() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.active_work_hook = false;
        let generation = h.runtime.run_generation();
        h.runtime.native_approval = Some(NativeApprovalState::new(generation));
        generation
    };
    super::super::terminal_runtime::set_workflow_report_token(ID, TOKEN.into());
    let at = chrono::Utc::now();
    let cancelled = Arc::new(AtomicBool::new(false));
    for (event, offset, want, revision) in [
        (HookEvent::PermissionRequest, 0, None, "2"),
        (HookEvent::Stop, 1, Some("idle"), "3"),
        (HookEvent::UserPromptSubmit, 2, Some("working"), "4"),
    ] {
        assert_eq!(
            writes
                .report_native_hook(
                    ID.into(),
                    TOKEN.into(),
                    generation,
                    Instant::now(),
                    report(event, at + chrono::Duration::milliseconds(offset)),
                    cancelled.clone()
                )
                .await
                .unwrap(),
            SqlHookOutcome::Accepted
        );
        let record = store.get(ID.into()).await.unwrap().unwrap();
        assert_eq!(record["attention"], serde_json::json!(want));
        assert_eq!(record["metadataRevision"], revision);
        assert_eq!(record["attentionOrigin"], "harness_hook");
        let sessions = state.sessions.lock().unwrap();
        assert!(sessions[ID].active_work_hook);
        assert_eq!(sessions[ID].info.attention.as_deref(), want);
    }
    assert_eq!(
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                Instant::now(),
                report(HookEvent::PermissionRequest, at),
                cancelled
            )
            .await
            .unwrap(),
        SqlHookOutcome::Rejected
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "4"
    );
    assert!(!dir
        .path()
        .join("sessions")
        .join(format!("{ID}.json"))
        .exists());
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_consumes_real_approval_mailboxes_and_defers_identity_without_json() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.active_work_hook = false;
        let generation = h.runtime.run_generation();
        h.runtime.native_approval = Some(NativeApprovalState::new(generation));
        generation
    };
    super::super::terminal_runtime::set_workflow_report_token(ID, TOKEN.into());
    let relay = super::super::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    let report = report(HookEvent::PermissionRequest, chrono::Utc::now());
    let approval = relay
        .mailbox_path()
        .join(format!("{}.json", uuid::Uuid::new_v4().simple()));
    std::fs::write(&approval, serde_json::json!({"approval": {
        "rootId":report.root_id, "turnId":report.turn_id, "toolUseId":null,
        "event":"PermissionRequest", "observedAt":report.observed_at, "hookFingerprint":report.hook_fingerprint,
    }}).to_string()).unwrap();
    let identity = relay
        .mailbox_path()
        .join(format!("{}.json", uuid::Uuid::new_v4().simple()));
    std::fs::write(
        &identity,
        serde_json::json!({"report": {
            "harnessSessionId":"root", "source":"codex_hook", "confidence":0.98,
            "hookFingerprint":expected_fingerprint().unwrap(),
        }})
        .to_string(),
    )
    .unwrap();
    let result = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert_eq!(result.activity.reports_accepted, 1);
    assert_eq!(result.activity.reports_consumed, 1);
    assert_eq!(result.identities_deferred, 1);
    assert!(result.storage_error.is_none());
    assert!(!approval.exists());
    assert!(identity.exists());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert!(!dir
        .path()
        .join("sessions")
        .join(format!("{ID}.json"))
        .exists());
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

async fn inactive_fixture() -> super::sql_tests::Fixture {
    let fixture = fixture_with_root("root").await;
    {
        let mut sessions = fixture.1.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.active_work_hook = false;
        h.runtime.native_approval = Some(NativeApprovalState::new(h.runtime.run_generation()));
    }
    super::super::terminal_runtime::set_workflow_report_token(ID, TOKEN.into());
    fixture
}
fn queue_permission(
    relay: &super::super::codex_hook_report_relay::CodexHookReportRelay,
) -> std::path::PathBuf {
    let report = report(HookEvent::PermissionRequest, chrono::Utc::now());
    let path = relay
        .mailbox_path()
        .join(format!("{}.json", uuid::Uuid::new_v4().simple()));
    std::fs::write(&path, serde_json::json!({"approval": {
        "rootId":report.root_id, "turnId":report.turn_id, "toolUseId":null,
        "event":"PermissionRequest", "observedAt":report.observed_at, "hookFingerprint":report.hook_fingerprint,
    }}).to_string()).unwrap();
    path
}
#[tokio::test]
async fn sql_hook_relay_storage_retry_retains_the_first_receipt_instead_of_renewing_grace() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay = super::super::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    let path = queue_permission(&relay);
    store.close().await.unwrap();
    let result = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(matches!(
        result.storage_error,
        Some(crate::session_store::StoreError::Unavailable)
    ));
    assert_eq!(result.activity.reports_consumed, 0);
    assert!(path.exists());
    assert!(!state.sessions.lock().unwrap()[ID].active_work_hook);
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    let (root, workspace) = store.location();
    let reopened = Arc::new(
        crate::session_store::SessionStore::open(root.into(), workspace.into())
            .await
            .unwrap(),
    );
    let writes = crate::session_application::sql_writes::SessionWrites::bind(
        state.clone(),
        reopened.clone(),
    )
    .unwrap();
    let retried = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(retried.storage_error.is_none());
    assert_eq!(retried.activity.reports_accepted, 1);
    assert_eq!(retried.activity.reports_consumed, 1);
    assert!(!path.exists());
    let doc = reopened.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(
        doc["attention"], "needs_you",
        "retry must use the original receipt's elapsed grace"
    );
    assert_eq!(doc["metadataRevision"], "3");
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    reopened.close().await.unwrap();
}
#[tokio::test]
async fn sql_hook_relay_counter_exhaustion_persists_and_protects_a_conservative_wait() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.active_work_hook = true;
        h.runtime.native_approval.as_mut().unwrap().hook_revision = None;
        h.runtime.run_generation()
    };
    assert_eq!(
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                Instant::now(),
                report(HookEvent::PermissionRequest, chrono::Utc::now()),
                Arc::new(AtomicBool::new(false))
            )
            .await
            .unwrap(),
        SqlHookOutcome::Accepted
    );
    let doc = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(doc["attention"], "needs_you");
    assert_eq!(doc["attentionOrigin"], "harness_hook");
    assert_eq!(doc["metadataRevision"], "2");
    let sessions = state.sessions.lock().unwrap();
    let h = &sessions[ID];
    assert!(h
        .runtime
        .native_approval
        .as_ref()
        .unwrap()
        .protects_permission_wait(&h.runtime.attention_owner, &h.info));
    drop(sessions);
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_retries_transition_contention_without_discarding_a_valid_report() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let original = report(HookEvent::PermissionRequest, chrono::Utc::now());
    let receipt = Instant::now();
    let cancelled = Arc::new(AtomicBool::new(false));
    let transition = super::super::prompt_authority::transition_lock()
        .lock()
        .unwrap();
    assert_eq!(
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                receipt,
                original.clone(),
                cancelled.clone()
            )
            .await
            .unwrap(),
        SqlHookOutcome::Retry
    );
    drop(transition);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    assert!(!state.sessions.lock().unwrap()[ID].active_work_hook);
    assert_eq!(
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                receipt,
                original,
                cancelled
            )
            .await
            .unwrap(),
        SqlHookOutcome::Accepted
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_rejects_stale_context_after_a_stalled_metadata_read() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "input",
        "control",
        "hook",
        "tracker",
        "attention",
        "creation",
        "root",
        "ending",
        "token",
        "cancel",
        "workspace",
    ] {
        let (dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
        let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation = cancelled.clone();
        let paused = crate::session_store::test_worker::pause(&store);
        let pending = tokio::spawn(async move {
            writes
                .report_native_hook(
                    ID.into(),
                    TOKEN.into(),
                    generation,
                    Instant::now(),
                    report(HookEvent::PermissionRequest, chrono::Utc::now()),
                    cancellation,
                )
                .await
        });
        crate::session_store::test_worker::wait_for_get(&store, ID).await;
        assert!(state.projection_lock.try_lock().is_ok());
        assert!(state.workspace.try_lock().is_ok());
        {
            let mut sessions = state
                .sessions
                .try_lock()
                .expect("hook metadata read retained the session guard");
            let h = sessions.get_mut(ID).unwrap();
            match change {
                "input" => h.runtime.input_generation += 1,
                "control" => h.runtime.control_tx = tokio::sync::mpsc::channel(1).0,
                "hook" => {
                    let t = h.runtime.native_approval.as_mut().unwrap();
                    t.hook_revision = t.hook_revision.map(|r| r + 1);
                }
                "tracker" => h.runtime.native_approval = Some(NativeApprovalState::new(generation)),
                "attention" => {
                    h.runtime.attention_owner.accepted_write();
                }
                "creation" => h.info.created_at = "replacement".into(),
                "root" => {
                    h.info.resume.as_mut().unwrap().harness_session_id = Some("replacement".into())
                }
                "ending" => h.info.lifecycle_phase = "ending".into(),
                "cancel" => cancelled.store(true, std::sync::atomic::Ordering::Release),
                "token" | "workspace" => {}
                _ => unreachable!(),
            }
        }
        if change == "token" {
            super::super::terminal_runtime::set_workflow_report_token(
                ID,
                "replacement-token".into(),
            );
        }
        if change == "workspace" {
            crate::test_support::swap_workspace(&state, dir.path());
        }
        drop(paused);
        assert_eq!(
            pending.await.unwrap().unwrap(),
            if change == "workspace" {
                SqlHookOutcome::Retry
            } else {
                SqlHookOutcome::Rejected
            },
            "{change}"
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "1",
            "{change}"
        );
        assert!(
            !state.sessions.lock().unwrap()[ID].active_work_hook,
            "{change}"
        );
        super::super::terminal_runtime::clear_workflow_report_token(ID);
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn sql_hook_relay_committed_write_cannot_publish_into_a_replacement_controller() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let pending = tokio::spawn(async move {
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                Instant::now(),
                report(HookEvent::PermissionRequest, chrono::Utc::now()),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    held.entered().await;
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    state
        .sessions
        .try_lock()
        .unwrap()
        .get_mut(ID)
        .unwrap()
        .runtime
        .control_tx = tokio::sync::mpsc::channel(1).0;
    drop(held);
    assert_eq!(
        pending.await.unwrap().unwrap(),
        SqlHookOutcome::CommittedDetached
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert!(!state.sessions.lock().unwrap()[ID].active_work_hook);
    assert_eq!(
        state.sessions.lock().unwrap()[ID]
            .info
            .provenance
            .attention_origin,
        None
    );
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_hook_relay_caller_cancellation_after_claim_retains_the_definitive_application() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let pending = tokio::spawn(async move {
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                Instant::now(),
                report(HookEvent::PermissionRequest, chrono::Utc::now()),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    held.entered().await;
    pending.abort();
    assert!(matches!(pending.await, Err(error) if error.is_cancelled()));
    drop(held);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !state.sessions.lock().unwrap()[ID].active_work_hook {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert_eq!(
        state.sessions.lock().unwrap()[ID]
            .info
            .provenance
            .attention_origin
            .as_deref(),
        Some("harness_hook")
    );
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_hook_relay_caller_cancellation_before_claim_removes_queued_admission() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let reservation = store
        .prepare_internal(
            ID.into(),
            super::sql_tests::CREATED.into(),
            crate::metadata::internal_mutations::InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let before = state.sessions.lock().unwrap()[ID]
        .runtime
        .native_approval
        .as_ref()
        .unwrap()
        .hook_application
        .clone();
    let pending = tokio::spawn(async move {
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                Instant::now(),
                report(HookEvent::PermissionRequest, chrono::Utc::now()),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let prepared = !Arc::ptr_eq(
                &before,
                &state.sessions.lock().unwrap()[ID]
                    .runtime
                    .native_approval
                    .as_ref()
                    .unwrap()
                    .hook_application,
            );
            if prepared {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    pending.abort();
    assert!(matches!(pending.await, Err(error) if error.is_cancelled()));
    drop(lane);
    drop(reservation);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    assert!(!state.sessions.lock().unwrap()[ID].active_work_hook);
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_retries_contention_at_execution_claim() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let reservation = store
        .prepare_internal(
            ID.into(),
            super::sql_tests::CREATED.into(),
            crate::metadata::internal_mutations::InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let original = report(HookEvent::PermissionRequest, chrono::Utc::now());
    let retry_report = original.clone();
    let receipt = Instant::now();
    let owner = writes.clone();
    let pending = tokio::spawn(async move {
        owner
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                receipt,
                original,
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    ready.await.unwrap();
    let transition = super::super::prompt_authority::transition_lock()
        .lock()
        .unwrap();
    drop(lane);
    drop(reservation);
    assert_eq!(pending.await.unwrap().unwrap(), SqlHookOutcome::Retry);
    drop(transition);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    assert!(!state.sessions.lock().unwrap()[ID].active_work_hook);
    assert_eq!(
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                receipt,
                retry_report,
                Arc::new(AtomicBool::new(false))
            )
            .await
            .unwrap(),
        SqlHookOutcome::Accepted
    );
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_equal_reports_at_exhaustion_cannot_adopt_an_older_application() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.active_work_hook = true;
        h.runtime.native_approval.as_mut().unwrap().hook_revision = None;
        h.runtime.run_generation()
    };
    let reservation = store
        .prepare_internal(
            ID.into(),
            super::sql_tests::CREATED.into(),
            crate::metadata::internal_mutations::InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let report = report(HookEvent::PermissionRequest, chrono::Utc::now());
    let receipt = Instant::now();
    let first_ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let first_owner = writes.clone();
    let first_report = report.clone();
    let first = tokio::spawn(async move {
        first_owner
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                receipt,
                first_report,
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    first_ready.await.unwrap();
    let second_ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let second = tokio::spawn(async move {
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                receipt,
                report,
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    second_ready.await.unwrap();
    drop(lane);
    drop(reservation);
    assert!(matches!(
        first.await.unwrap(),
        Err(crate::session_store::StoreError::Conflict)
    ));
    assert_eq!(second.await.unwrap().unwrap(), SqlHookOutcome::Accepted);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    let sessions = state.sessions.lock().unwrap();
    assert_eq!(sessions[ID].info.attention.as_deref(), Some("needs_you"));
    assert!(sessions[ID]
        .runtime
        .native_approval
        .as_ref()
        .unwrap()
        .hook_revision
        .is_none());
    drop(sessions);
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_hook_relay_preparing_a_boundary_revokes_an_earlier_emitted_native_effect() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    super::super::terminal_runtime::set_workflow_report_token(ID, TOKEN.into());
    let reservation = store
        .prepare_internal(
            ID.into(),
            super::sql_tests::CREATED.into(),
            crate::metadata::internal_mutations::InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let owner = writes.clone();
    let boundary = tokio::spawn(async move {
        owner
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                Instant::now(),
                report(HookEvent::Stop, chrono::Utc::now()),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    ready.await.unwrap();
    assert!(
        matches!(
            tokio::time::timeout(
                std::time::Duration::from_secs(1),
                writes.native(ID.into(), effect, Arc::new(AtomicBool::new(false)))
            )
            .await
            .unwrap(),
            Err(crate::session_store::StoreError::Conflict)
        ),
        "old effect must be rejected before waiting for the held lane"
    );
    boundary.abort();
    assert!(boundary.await.unwrap_err().is_cancelled());
    drop(lane);
    drop(reservation);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sql_hook_relay_stalled_execution_leaves_real_pty_input_delivery_available() {
    use crate::runtime::session_runtime::RuntimeCommand;
    use std::io::{Read, Write};
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, mut commands) = inactive_fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let pending = tokio::spawn(async move {
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                Instant::now(),
                report(HookEvent::PermissionRequest, chrono::Utc::now()),
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
    drop(held);
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(echoed
        .unwrap()
        .unwrap()
        .windows(16)
        .any(|s| s == b"responsive-input"));
    assert_eq!(
        pending.await.unwrap().unwrap(),
        SqlHookOutcome::CommittedDetached
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert!(!state.sessions.lock().unwrap()[ID].active_work_hook);
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_review_unknown_live_root_retains_valid_approval() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.info.resume.as_mut().unwrap().harness_session_id = None;
        h.runtime.run_generation()
    };
    let relay = super::super::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    let path = queue_permission(&relay);
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
    assert_eq!(result.activity.reports_consumed, 0);
    assert!(path.exists());
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut(ID)
        .unwrap()
        .info
        .resume
        .as_mut()
        .unwrap()
        .harness_session_id = Some("other".into());
    let result = relay
        .consume_ready_sql(
            state,
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert_eq!(result.activity.reports_consumed, 1);
    assert!(!path.exists());
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_review_cancelled_drain_preserves_original_receipt() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay =
        Arc::new(super::super::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    let path = queue_permission(&relay);
    let paused = crate::session_store::test_worker::pause(&store);
    let owner = relay.clone();
    let application = writes.clone();
    let runtime = state.clone();
    let pending = tokio::spawn(async move {
        owner
            .consume_ready_sql(
                runtime,
                ID,
                TOKEN,
                generation,
                application,
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    crate::session_store::test_worker::wait_for_get(&store, ID).await;
    pending.abort();
    assert!(matches!(pending.await, Err(error) if error.is_cancelled()));
    drop(paused);
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    let result = relay
        .consume_ready_sql(
            state,
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(result.storage_error.is_none());
    assert_eq!(result.activity.reports_accepted, 1);
    assert!(!path.exists());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["attention"],
        "needs_you"
    );
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_review_contention_after_activation_commit_retains_packet() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay =
        Arc::new(super::super::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    let path = queue_permission(&relay);
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let owner = relay.clone();
    let application = writes.clone();
    let runtime = state.clone();
    let pending = tokio::spawn(async move {
        owner
            .consume_ready_sql(
                runtime,
                ID,
                TOKEN,
                generation,
                application,
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    held.entered().await;
    let transition = super::super::prompt_authority::transition_lock()
        .lock()
        .unwrap();
    drop(held);
    let result = pending.await.unwrap();
    drop(transition);
    assert_eq!(result.activity.reports_consumed, 0);
    assert_eq!(result.committed_detached, 0);
    assert!(result.storage_error.is_none());
    assert!(path.exists());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert!(!state.sessions.lock().unwrap()[ID].active_work_hook);
    let result = relay
        .consume_ready_sql(
            state,
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert_eq!(result.activity.reports_accepted, 1);
    assert!(!path.exists());
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_review_native_effect_cannot_publish_after_token_replacement() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
    let generation = {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.active_work_hook = true;
        h.runtime.run_generation()
    };
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let pending = tokio::spawn(async move {
        writes
            .report_native_hook(
                ID.into(),
                TOKEN.into(),
                generation,
                Instant::now() - std::time::Duration::from_secs(3),
                report(HookEvent::PermissionRequest, chrono::Utc::now()),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    held.entered().await;
    super::super::terminal_runtime::set_workflow_report_token(ID, "replacement-token".into());
    drop(held);
    assert_eq!(
        pending.await.unwrap().unwrap(),
        SqlHookOutcome::CommittedDetached
    );
    assert_eq!(state.sessions.lock().unwrap()[ID].info.attention, None);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["attention"],
        "needs_you"
    );
    super::super::terminal_runtime::clear_workflow_report_token(ID);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_hook_relay_review_native_claim_revalidates_capability_and_retries_contention() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for replacement in [false, true] {
        let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
        let generation = {
            let mut sessions = state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            h.active_work_hook = true;
            h.runtime.run_generation()
        };
        let reservation = store
            .prepare_internal(
                ID.into(),
                super::sql_tests::CREATED.into(),
                crate::metadata::internal_mutations::InternalMutation::UserTitle { title: None },
            )
            .unwrap();
        let lane = reservation.lane().unwrap().lock_owned().await;
        let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
        let pending = tokio::spawn(async move {
            writes
                .report_native_hook(
                    ID.into(),
                    TOKEN.into(),
                    generation,
                    Instant::now() - std::time::Duration::from_secs(3),
                    report(HookEvent::PermissionRequest, chrono::Utc::now()),
                    Arc::new(AtomicBool::new(false)),
                )
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), ready)
            .await
            .unwrap()
            .unwrap();
        let transition = if replacement {
            super::super::terminal_runtime::set_workflow_report_token(
                ID,
                "replacement-token".into(),
            );
            None
        } else {
            Some(
                super::super::prompt_authority::transition_lock()
                    .lock()
                    .unwrap(),
            )
        };
        drop(lane);
        drop(reservation);
        let result = pending.await.unwrap();
        drop(transition);
        if replacement {
            assert!(matches!(
                result,
                Err(crate::session_store::StoreError::Conflict)
            ));
        } else {
            assert_eq!(result.unwrap(), SqlHookOutcome::Retry);
        }
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "1"
        );
        assert_eq!(state.sessions.lock().unwrap()[ID].info.attention, None);
        super::super::terminal_runtime::clear_workflow_report_token(ID);
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_hook_relay_review_post_commit_contention_retries_native_and_fallback() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for fallback in [false, true] {
        let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
        let generation = {
            let mut sessions = state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            h.active_work_hook = true;
            if fallback {
                h.runtime.native_approval.as_mut().unwrap().hook_revision = None;
            }
            h.runtime.run_generation()
        };
        let original = report(HookEvent::PermissionRequest, chrono::Utc::now());
        let retry_report = original.clone();
        let receipt = Instant::now() - std::time::Duration::from_secs(3);
        let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
        let owner = writes.clone();
        let pending = tokio::spawn(async move {
            owner
                .report_native_hook(
                    ID.into(),
                    TOKEN.into(),
                    generation,
                    receipt,
                    original,
                    Arc::new(AtomicBool::new(false)),
                )
                .await
        });
        held.entered().await;
        let transition = super::super::prompt_authority::transition_lock()
            .lock()
            .unwrap();
        drop(held);
        assert_eq!(pending.await.unwrap().unwrap(), SqlHookOutcome::Retry);
        drop(transition);
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["attention"],
            "needs_you"
        );
        assert_eq!(state.sessions.lock().unwrap()[ID].info.attention, None);
        assert_eq!(
            writes
                .report_native_hook(
                    ID.into(),
                    TOKEN.into(),
                    generation,
                    receipt,
                    retry_report,
                    Arc::new(AtomicBool::new(false))
                )
                .await
                .unwrap(),
            SqlHookOutcome::Accepted
        );
        assert_eq!(
            state.sessions.lock().unwrap()[ID].info.attention.as_deref(),
            Some("needs_you"),
            "a fresh retry must restore live permission attention before acceptance"
        );
        assert!(
            state.sessions.lock().unwrap()[ID]
                .runtime
                .native_approval
                .as_ref()
                .unwrap()
                .sql_ownership
                .is_none(),
            "a retry must not adopt the durable proof from a skipped publication"
        );
        super::super::terminal_runtime::clear_workflow_report_token(ID);
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_hook_relay_review_expiry_drains_resources_while_claim_guards_are_blocked() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for native in [false, true] {
        let (_dir, state, store, writes, _effect, _commands) = inactive_fixture().await;
        let generation = {
            let mut sessions = state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            h.active_work_hook = native;
            h.runtime.run_generation()
        };
        let reservation = store
            .prepare_internal(
                ID.into(),
                super::sql_tests::CREATED.into(),
                crate::metadata::internal_mutations::InternalMutation::UserTitle { title: None },
            )
            .unwrap();
        let lane_key = reservation.lane().unwrap();
        let lane = lane_key.clone().lock_owned().await;
        let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
        let mut pending = tokio::spawn(async move {
            writes
                .report_native_hook(
                    ID.into(),
                    TOKEN.into(),
                    generation,
                    Instant::now() - std::time::Duration::from_secs(3),
                    report(HookEvent::PermissionRequest, chrono::Utc::now()),
                    Arc::new(AtomicBool::new(false)),
                )
                .await
        });
        ready.await.unwrap();
        let projection = state.projection_lock.lock().unwrap();
        drop(lane);
        drop(reservation);
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(2800), &mut pending).await;
        // Ticket expiry and the worker's deadline retirement race. Require the
        // lane to become available while validation is still blocked, allowing
        // the worker to deliver/drop its rejected completion.
        let lane_available =
            tokio::time::timeout(std::time::Duration::from_millis(300), lane_key.lock_owned())
                .await
                .is_ok();
        drop(projection);
        if result.is_err() {
            let _ = pending.await;
        }
        assert!(
            matches!(
                result,
                Ok(Ok(Err(crate::session_store::StoreError::Expired)))
            ),
            "expiry must finish even while the application guard is unavailable"
        );
        assert!(
            lane_available,
            "expired completion must release its session lane before guard release"
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "1"
        );
        super::super::terminal_runtime::clear_workflow_report_token(ID);
        store.close().await.unwrap();
    }
}
