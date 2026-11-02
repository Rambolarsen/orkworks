use super::*;
use crate::metadata::internal_mutations::InternalMutation;
use crate::metadata::{HarnessSessionMergeResult as Merge, HarnessSessionReport};
use crate::runtime::codex_approval_application::NativeApprovalState;
use crate::session_application::sql_writes::{
    NativeIdentityReport, SessionWrites, SqlIdentityOutcome,
};
use crate::session_store::StoreError;
use std::sync::atomic::AtomicBool;
const ID: &str = "sql-native-identity";
const TOKEN: &str = "identity-report-token";
const CREATED: &str = "2026-01-01T00:00:00Z";
type Fixture = (
    tempfile::TempDir,
    Arc<AppState>,
    Arc<crate::session_store::SessionStore>,
    Arc<SessionWrites>,
    tokio::sync::mpsc::Receiver<crate::runtime::session_runtime::RuntimeCommand>,
);
async fn fixture(root_id: Option<&str>) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let (root, path) = {
        let mut workspace = state.workspace.lock().unwrap();
        let ws = workspace.as_mut().unwrap();
        ws.lease = Some(Arc::new(
            crate::workspace_runtime::WorkspaceLease::acquire(&ws.metadata.root_path()).unwrap(),
        ));
        (
            ws.metadata.root_path(),
            ws.path.to_string_lossy().into_owned(),
        )
    };
    let mut info =
        crate::test_support::test_session_info(ID, "identity", path.clone(), "running", CREATED);
    info.harness = Some("codex".into());
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    info.resume = root_id.map(|root_id| crate::harness::ResumeMemory {
        state: crate::harness::ResumeState::Available,
        preferred_strategy: crate::harness::ResumeStrategy::Exact,
        harness_session_id: Some(root_id.into()),
        latest_fallback: false,
        last_seen_at: None,
    });
    let (mut runtime, commands) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let mut tracker = NativeApprovalState::new(runtime.run_generation());
    tracker.root = root_id.map(str::to_owned);
    runtime.native_approval = Some(tracker);
    let mut metadata = crate::test_support::test_session_metadata(
        ID,
        "identity",
        path.clone(),
        "running",
        CREATED,
        CREATED,
    );
    metadata.harness = "codex".into();
    metadata.lifecycle = "alive".into();
    metadata.lifecycle_phase = "active".into();
    metadata.resume = info.resume.clone();
    if root_id.is_some() {
        metadata.harness_session_id_source = Some("codex_hook".into());
        metadata.harness_session_id_confidence = Some(0.9);
    }
    state.sessions.lock().unwrap().insert(
        ID.into(),
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
    let store = Arc::new(
        crate::session_store::SessionStore::initialize(root, path)
            .await
            .unwrap(),
    );
    let mut document = serde_json::to_value(metadata).unwrap();
    document["futureExtension"] = serde_json::json!({"opaque": [1,2,3]});
    store.create(document).await.unwrap();
    let writes = SessionWrites::bind(state.clone(), store.clone()).unwrap();
    crate::runtime::terminal_runtime::set_workflow_report_token(ID, TOKEN.into());
    (dir, state, store, writes, commands)
}
fn report(root: &str, lifecycle: Option<(&str, &str)>) -> NativeIdentityReport {
    NativeIdentityReport {
        report: HarnessSessionReport {
            harness_session_id: root.into(),
            source: "codex_hook".into(),
            confidence: 0.98,
        },
        session_start_source: lifecycle.map(|(source, _)| source.into()),
        session_start_event: lifecycle.map(|(_, event)| event.into()),
    }
}
async fn apply(
    writes: &Arc<SessionWrites>,
    state: &Arc<AppState>,
    report: NativeIdentityReport,
) -> Result<SqlIdentityOutcome, StoreError> {
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    writes
        .report_native_identity(
            ID.into(),
            TOKEN.into(),
            generation,
            report,
            Arc::new(AtomicBool::new(false)),
        )
        .await
}
fn clear_token() {
    crate::runtime::terminal_runtime::clear_workflow_report_token(ID);
    crate::codex_session_store::clear_native_label_refresh_block(ID);
}
#[tokio::test]
async fn sql_identity_binds_and_publishes_without_touching_attention_or_json() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(None).await;
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let old_epoch = state.sessions.lock().unwrap()[ID]
        .runtime
        .native_approval
        .as_ref()
        .unwrap()
        .hook_application
        .clone();
    assert_eq!(
        apply(
            &writes,
            &state,
            report("root-one", Some(("startup", "SessionStart")))
        )
        .await
        .unwrap(),
        SqlIdentityOutcome::Accepted(Merge::Accepted)
    );
    let after = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(after["metadataRevision"], "2");
    assert_eq!(after["resume"]["harnessSessionId"], "root-one");
    assert_eq!(after["harnessSessionIdSource"], "codex_hook");
    assert_eq!(after["futureExtension"], before["futureExtension"]);
    for key in [
        "attention",
        "observedStatus",
        "lastActivity",
        "needsUserInput",
        "attentionUpdatedAt",
    ] {
        assert_eq!(after[key], before[key], "{key}");
    }
    let sessions = state.sessions.lock().unwrap();
    let handle = &sessions[ID];
    assert_eq!(
        handle
            .info
            .resume
            .as_ref()
            .unwrap()
            .harness_session_id
            .as_deref(),
        Some("root-one")
    );
    assert!(!Arc::ptr_eq(
        &old_epoch,
        &handle
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .hook_application
    ));
    assert!(handle
        .runtime
        .native_approval
        .as_ref()
        .unwrap()
        .sql_ownership
        .is_none());
    drop(sessions);
    assert!(!dir
        .path()
        .join("sessions")
        .join(format!("{ID}.json"))
        .exists());
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_identity_replacement_requires_authenticated_recorded_root_clear() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    assert_eq!(
        apply(&writes, &state, report("root-two", None))
            .await
            .unwrap(),
        SqlIdentityOutcome::Accepted(Merge::IgnoredIdentityChange)
    );
    crate::codex_session_store::block_native_label_refresh(ID, "root-one");
    assert_eq!(
        apply(
            &writes,
            &state,
            report("root-two", Some(("startup", "SessionStart")))
        )
        .await
        .unwrap(),
        SqlIdentityOutcome::Accepted(Merge::IgnoredIdentityChange)
    );
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    assert_eq!(
        writes
            .report_native_identity(
                ID.into(),
                "wrong-token".into(),
                generation,
                report("root-two", Some(("clear", "SessionStart"))),
                Arc::new(AtomicBool::new(false))
            )
            .await
            .unwrap(),
        SqlIdentityOutcome::Rejected
    );
    let mut invalid = report("root-two", Some(("clear", "SessionStart")));
    invalid.session_start_event = None;
    assert_eq!(
        apply(&writes, &state, invalid).await.unwrap(),
        SqlIdentityOutcome::Rejected
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    assert_eq!(
        apply(
            &writes,
            &state,
            report("root-two", Some(("clear", "SessionStart")))
        )
        .await
        .unwrap(),
        SqlIdentityOutcome::Accepted(Merge::Accepted)
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["resume"]["harnessSessionId"],
        "root-two"
    );
    assert!(!crate::codex_session_store::native_identity_reset_is_authorized(ID, "root-one"));
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_identity_binding_preserves_intervening_input_and_user_attention() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let paused = crate::session_store::test_worker::pause(&store);
    let pending = tokio::spawn(async move {
        writes
            .report_native_identity(
                ID.into(),
                TOKEN.into(),
                generation,
                report("root-one", None),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    crate::session_store::test_worker::wait_for_get(&store, ID).await;
    {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(ID).unwrap();
        handle.runtime.input_generation += 1;
        handle.runtime.committed_input_sequence += 1;
        handle.runtime.attention_owner.accepted_write();
        handle.info.attention = Some("needs_you".into());
        handle.info.observed_status = Some("waiting_for_input".into());
        handle.info.needs_user_input = Some(true);
        handle.info.provenance.attention_source = Some("user".into());
    }
    drop(paused);
    assert_eq!(
        pending.await.unwrap().unwrap(),
        SqlIdentityOutcome::Accepted(Merge::Accepted)
    );
    let sessions = state.sessions.lock().unwrap();
    let handle = &sessions[ID];
    assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
    assert_eq!(handle.info.needs_user_input, Some(true));
    assert_eq!(
        handle.info.provenance.attention_source.as_deref(),
        Some("user")
    );
    assert_eq!(
        handle
            .info
            .resume
            .as_ref()
            .unwrap()
            .harness_session_id
            .as_deref(),
        Some("root-one")
    );
    drop(sessions);
    clear_token();
    store.close().await.unwrap();
}

fn queue_identity(
    relay: &crate::runtime::codex_hook_report_relay::CodexHookReportRelay,
    root: &str,
) -> std::path::PathBuf {
    let path = relay
        .mailbox_path()
        .join(format!("{}.json", uuid::Uuid::new_v4().simple()));
    std::fs::write(
        &path,
        serde_json::json!({"report": {
            "harnessSessionId":root, "source":"codex_hook", "confidence":0.98,
            "hookFingerprint":expected_fingerprint().unwrap(),
            "sessionStartSource":"startup", "sessionStartEvent":"SessionStart"
        }})
        .to_string(),
    )
    .unwrap();
    path
}
#[tokio::test]
async fn sql_identity_mailbox_binds_before_retrying_the_original_unbound_approval() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay = crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    let approval = relay
        .mailbox_path()
        .join(format!("{}.json", uuid::Uuid::new_v4().simple()));
    std::fs::write(
        &approval,
        serde_json::json!({"approval": {
            "rootId":"root-one", "turnId":"turn", "toolUseId":null, "event":"PermissionRequest",
            "observedAt":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            "hookFingerprint":expected_fingerprint().unwrap()
        }})
        .to_string(),
    )
    .unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let first = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes.clone(),
            cancelled.clone(),
        )
        .await;
    assert_eq!(first.activity.reports_consumed, 0);
    assert!(approval.exists());
    let identity = queue_identity(&relay, "root-one");
    let second = relay
        .consume_ready_sql(
            state.clone(),
            ID,
            TOKEN,
            generation,
            writes.clone(),
            cancelled.clone(),
        )
        .await;
    assert_eq!(second.activity.reports_accepted, 1);
    assert!(!identity.exists());
    assert!(approval.exists());
    assert!(second.storage_error.is_none());
    let third = relay
        .consume_ready_sql(state.clone(), ID, TOKEN, generation, writes, cancelled)
        .await;
    assert_eq!(third.activity.reports_accepted, 1);
    assert!(!approval.exists());
    assert!(state.sessions.lock().unwrap()[ID].active_work_hook);
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
async fn sql_identity_rejects_obsolete_owner_after_a_stalled_read() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "control",
        "runtime",
        "creation",
        "root",
        "tracker",
        "ending",
        "token",
        "cancel",
        "workspace",
        "identity_epoch",
    ] {
        let (_dir, state, store, writes, _commands) = fixture(None).await;
        if change == "identity_epoch" {
            state
                .sessions
                .lock()
                .unwrap()
                .get_mut(ID)
                .unwrap()
                .runtime
                .native_approval
                .as_mut()
                .unwrap()
                .hook_revision = None;
        }
        let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation = cancelled.clone();
        let paused = crate::session_store::test_worker::pause(&store);
        let pending = tokio::spawn(async move {
            writes
                .report_native_identity(
                    ID.into(),
                    TOKEN.into(),
                    generation,
                    report("root-one", None),
                    cancellation,
                )
                .await
        });
        crate::session_store::test_worker::wait_for_get(&store, ID).await;
        assert!(state.projection_lock.try_lock().is_ok());
        assert!(state.workspace.try_lock().is_ok());
        {
            let mut sessions = state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            match change {
                "control" => h.runtime.control_tx = tokio::sync::mpsc::channel(1).0,
                "runtime" => {
                    h.runtime = crate::runtime::session_runtime::SessionRuntime::live(24, 80).0
                }
                "creation" => h.info.created_at = "replacement-creation".into(),
                "root" => {
                    h.info.resume = Some(crate::harness::ResumeMemory {
                        state: crate::harness::ResumeState::Available,
                        preferred_strategy: crate::harness::ResumeStrategy::Exact,
                        harness_session_id: Some("replacement".into()),
                        latest_fallback: false,
                        last_seen_at: None,
                    })
                }
                "tracker" => h.runtime.native_approval = Some(NativeApprovalState::new(generation)),
                "ending" => h.info.lifecycle_phase = "ending".into(),
                "token" => crate::runtime::terminal_runtime::set_workflow_report_token(
                    ID,
                    "replacement-token".into(),
                ),
                "cancel" => cancelled.store(true, std::sync::atomic::Ordering::Release),
                "identity_epoch" => h
                    .runtime
                    .native_approval
                    .as_mut()
                    .unwrap()
                    .identity_changed(),
                "workspace" => {}
                _ => unreachable!(),
            }
        }
        if change == "workspace" {
            state.workspace.lock().unwrap().take();
        }
        drop(paused);
        assert_eq!(
            pending.await.unwrap().unwrap(),
            SqlIdentityOutcome::Rejected,
            "{change}"
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "1",
            "{change}"
        );
        clear_token();
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_identity_retains_confidence_rules_and_refreshes_only_identity_scope() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    let mut lower = report("root-two", None);
    lower.report.confidence = 0.1;
    assert_eq!(
        apply(&writes, &state, lower).await.unwrap(),
        SqlIdentityOutcome::Accepted(Merge::IgnoredLowerConfidence)
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    let mut same = report("root-one", None);
    same.report.confidence = 0.9;
    assert_eq!(
        apply(&writes, &state, same).await.unwrap(),
        SqlIdentityOutcome::Accepted(Merge::IgnoredUnchanged)
    );
    let row = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(row["metadataRevision"], "2");
    assert!(row["resume"]["lastSeenAt"].is_string());
    assert_eq!(row["lastActivity"], CREATED);
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_identity_publication_contention_retries_with_a_fresh_durable_write() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let owner = writes.clone();
    let pending = tokio::spawn(async move {
        owner
            .report_native_identity(
                ID.into(),
                TOKEN.into(),
                generation,
                report("root-one", None),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    held.entered().await;
    let transition = crate::runtime::prompt_authority::transition_lock()
        .lock()
        .unwrap();
    drop(held);
    assert_eq!(pending.await.unwrap().unwrap(), SqlIdentityOutcome::Retry);
    drop(transition);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert!(state.sessions.lock().unwrap()[ID].info.resume.is_none());
    assert_eq!(
        apply(&writes, &state, report("root-one", None))
            .await
            .unwrap(),
        SqlIdentityOutcome::Accepted(Merge::IgnoredUnchanged)
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "3"
    );
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
async fn sql_identity_after_claim_never_publishes_into_replaced_control_or_token() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for token in [false, true] {
        let (_dir, state, store, writes, _commands) = fixture(None).await;
        let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
        let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
        let pending = tokio::spawn(async move {
            writes
                .report_native_identity(
                    ID.into(),
                    TOKEN.into(),
                    generation,
                    report("root-one", None),
                    Arc::new(AtomicBool::new(false)),
                )
                .await
        });
        held.entered().await;
        if token {
            crate::runtime::terminal_runtime::set_workflow_report_token(
                ID,
                "replacement-token".into(),
            );
        } else {
            state
                .sessions
                .lock()
                .unwrap()
                .get_mut(ID)
                .unwrap()
                .runtime
                .control_tx = tokio::sync::mpsc::channel(1).0;
        }
        drop(held);
        assert_eq!(
            pending.await.unwrap().unwrap(),
            SqlIdentityOutcome::CommittedDetached
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["resume"]["harnessSessionId"],
            "root-one"
        );
        assert!(state.sessions.lock().unwrap()[ID].info.resume.is_none());
        clear_token();
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_identity_caller_cancellation_obeys_the_execution_cutoff() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let reservation = store
        .prepare_internal(
            ID.into(),
            CREATED.into(),
            InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let owner = writes.clone();
    let pending = tokio::spawn(async move {
        owner
            .report_native_identity(
                ID.into(),
                TOKEN.into(),
                generation,
                report("root-one", None),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    ready.await.unwrap();
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    drop(lane);
    drop(reservation);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let owner = writes.clone();
    let pending = tokio::spawn(async move {
        owner
            .report_native_identity(
                ID.into(),
                TOKEN.into(),
                generation,
                report("root-one", None),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    held.entered().await;
    let published = writes.test_before_callback();
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    drop(held);
    tokio::time::timeout(std::time::Duration::from_secs(2), published)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
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
async fn sql_identity_rechecks_recorded_clear_authority_at_execution_claim() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    crate::codex_session_store::block_native_label_refresh(ID, "root-one");
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let reservation = store
        .prepare_internal(
            ID.into(),
            CREATED.into(),
            InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let pending = tokio::spawn(async move {
        writes
            .report_native_identity(
                ID.into(),
                TOKEN.into(),
                generation,
                report("root-two", Some(("clear", "SessionStart"))),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    ready.await.unwrap();
    crate::codex_session_store::clear_native_label_refresh_block(ID);
    drop(lane);
    drop(reservation);
    assert!(matches!(pending.await.unwrap(), Err(StoreError::Conflict)));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
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
async fn sql_identity_conflicting_revision_is_not_replayed_or_acknowledged() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let reservation = store
        .prepare_internal(
            ID.into(),
            CREATED.into(),
            InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let pending = tokio::spawn(async move {
        writes
            .report_native_identity(
                ID.into(),
                TOKEN.into(),
                generation,
                report("root-one", None),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    ready.await.unwrap();
    store
        .mutate_internal(
            ID.into(),
            CREATED.into(),
            None,
            InternalMutation::UserTitle {
                title: Some("newer title".into()),
            },
        )
        .await
        .unwrap();
    drop(lane);
    drop(reservation);
    assert!(matches!(pending.await.unwrap(), Err(StoreError::Conflict)));
    let row = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(row["metadataRevision"], "2");
    assert_eq!(row["label"], "newer title");
    assert!(row["resume"].is_null());
    assert!(state.sessions.lock().unwrap()[ID].info.resume.is_none());
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_identity_blocked_claim_expires_and_releases_its_lane_before_guard_recovery() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let reservation = store
        .prepare_internal(
            ID.into(),
            CREATED.into(),
            InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let key = reservation.lane().unwrap();
    let lane = key.clone().lock_owned().await;
    let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let mut pending = tokio::spawn(async move {
        writes
            .report_native_identity(
                ID.into(),
                TOKEN.into(),
                generation,
                report("root-one", None),
                Arc::new(AtomicBool::new(false)),
            )
            .await
    });
    ready.await.unwrap();
    let projection = state.projection_lock.lock().unwrap();
    drop(lane);
    drop(reservation);
    let result = tokio::time::timeout(std::time::Duration::from_millis(2800), &mut pending).await;
    let available = tokio::time::timeout(std::time::Duration::from_millis(300), key.lock_owned())
        .await
        .is_ok();
    drop(projection);
    if result.is_err() {
        let _ = pending.await;
    }
    assert!(matches!(result, Ok(Ok(Err(StoreError::Expired)))));
    assert!(available);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_identity_mailbox_storage_retry_reopens_without_json_fallback() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay = crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap();
    let file = queue_identity(&relay, "root-one");
    store.close().await.unwrap();
    let failed = relay
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
        failed.storage_error,
        Some(StoreError::Unavailable)
    ));
    assert!(file.exists());
    let (root, path) = store.location();
    let reopened = Arc::new(
        crate::session_store::SessionStore::open(root.into(), path.into())
            .await
            .unwrap(),
    );
    let writes = SessionWrites::bind(state.clone(), reopened.clone()).unwrap();
    let retry = relay
        .consume_ready_sql(
            state,
            ID,
            TOKEN,
            generation,
            writes,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
    assert!(retry.storage_error.is_none());
    assert_eq!(retry.activity.reports_accepted, 1);
    assert!(!file.exists());
    assert_eq!(
        reopened.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert!(!dir
        .path()
        .join("sessions")
        .join(format!("{ID}.json"))
        .exists());
    clear_token();
    reopened.close().await.unwrap();
}

#[tokio::test]
async fn sql_identity_cancelled_drain_retains_original_bytes_and_rejects_changed_packet() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let relay =
        Arc::new(crate::runtime::codex_hook_report_relay::CodexHookReportRelay::new().unwrap());
    let file = queue_identity(&relay, "root-one");
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
    let changed = std::fs::read_to_string(&file)
        .unwrap()
        .replace("root-one", "root-two");
    std::fs::write(&file, changed).unwrap();
    drop(paused);
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
    assert_eq!(result.activity.reports_accepted, 0);
    assert_eq!(result.activity.reports_consumed, 1);
    assert!(!file.exists());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_identity_invalid_private_reports_are_rejected_before_storage() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(None).await;
    store.close().await.unwrap();
    for change in [
        "short",
        "long",
        "space",
        "nan",
        "negative",
        "over",
        "foreign",
        "source_only",
        "event_only",
        "bad_event",
        "bad_lifecycle",
    ] {
        let mut request = report("root-one", None);
        match change {
            "short" => request.report.harness_session_id = "x".into(),
            "long" => request.report.harness_session_id = "x".repeat(513),
            "space" => request.report.harness_session_id = "root one".into(),
            "nan" => request.report.confidence = f64::NAN,
            "negative" => request.report.confidence = -0.1,
            "over" => request.report.confidence = 1.1,
            "foreign" => request.report.source = "claude_hook".into(),
            "source_only" => request.session_start_source = Some("clear".into()),
            "event_only" => request.session_start_event = Some("SessionStart".into()),
            "bad_event" => request = report("root-one", Some(("clear", "Stop"))),
            "bad_lifecycle" => request = report("root-one", Some(("unknown", "SessionStart"))),
            _ => unreachable!(),
        }
        assert_eq!(
            apply(&writes, &state, request).await.unwrap(),
            SqlIdentityOutcome::Rejected,
            "{change}"
        );
    }
    clear_token();
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sql_identity_stalled_execution_keeps_real_pty_input_and_identity_publication_available() {
    use crate::runtime::session_runtime::RuntimeCommand;
    use std::io::{Read, Write};
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, mut commands) = fixture(None).await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let pending = tokio::spawn(async move {
        writes
            .report_native_identity(
                ID.into(),
                TOKEN.into(),
                generation,
                report("root-one", None),
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

#[path = "sql_label_tests.rs"]
mod label_tests;
