use super::sql_tests::{fixture_with_root, ID};
use super::*;
use crate::session_store::StoreError;
use std::sync::atomic::{AtomicBool, Ordering};
fn result(
) -> Result<super::super::codex_native::NativeObservation, super::super::codex_native::NativeError>
{
    Ok(super::super::codex_native::NativeObservation {
        status: super::super::codex_approval::NativeStatus::Active,
        complete_singleton_root: true,
        started_at: Instant::now(),
    })
}
#[tokio::test]
async fn sql_observer_pipeline_reads_sql_and_commits_only_current_rpc_effects() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let cancelled = Arc::new(AtomicBool::new(false));
    // Tick publishes the due wait first; a fresh loop captures the subsequent RPC.
    assert!(writes
        .begin_native_observation(ID.into(), generation, cancelled.clone())
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["attention"],
        "needs_you"
    );
    let snapshot = writes
        .begin_native_observation(ID.into(), generation, cancelled)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.root(), "root");
    assert!(writes
        .finish_native_observation_coordinated_for_test(snapshot, result())
        .await
        .unwrap());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["attention"],
        "working"
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "3"
    );
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.attention.as_deref(),
        Some("working")
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_observer_pipeline_discards_a_hook_obsolete_rpc_result() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let cancelled = Arc::new(AtomicBool::new(false));
    writes
        .begin_native_observation(ID.into(), generation, cancelled.clone())
        .await
        .unwrap();
    let snapshot = writes
        .begin_native_observation(ID.into(), generation, cancelled)
        .await
        .unwrap()
        .unwrap();
    {
        let mut sessions = state.sessions.lock().unwrap();
        let t = sessions
            .get_mut(ID)
            .unwrap()
            .runtime
            .native_approval
            .as_mut()
            .unwrap();
        t.hook_revision = t.hook_revision.map(|r| r + 1);
    }
    assert!(matches!(
        writes
            .finish_native_observation_coordinated_for_test(snapshot, result())
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["attention"],
        "needs_you"
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_pipeline_discards_all_obsolete_rpc_contexts_before_reduction() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "input",
        "sequence",
        "hook",
        "tracker",
        "runtime",
        "control",
        "creation",
        "attention",
        "root",
        "harness",
        "ending",
        "inactive",
        "cancel",
        "workspace",
    ] {
        let (dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
        let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
        let cancelled = Arc::new(AtomicBool::new(false));
        writes
            .begin_native_observation(ID.into(), generation, cancelled.clone())
            .await
            .unwrap();
        let snapshot = writes
            .begin_native_observation(ID.into(), generation, cancelled.clone())
            .await
            .unwrap()
            .unwrap();
        let diagnostic = Arc::new(std::sync::Mutex::new(DiagnosticMetrics::default()));
        {
            let mut sessions = state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            h.runtime.native_approval.as_mut().unwrap().diagnostic = Some(diagnostic.clone());
            match change {
                "input" => h.runtime.input_generation += 1,
                "sequence" => h.runtime.committed_input_sequence += 1,
                "hook" => {
                    let t = h.runtime.native_approval.as_mut().unwrap();
                    t.hook_revision = t.hook_revision.map(|r| r + 1);
                }
                "tracker" => {
                    let mut t = NativeApprovalState::new(generation);
                    t.root = Some("root".into());
                    t.diagnostic = Some(diagnostic.clone());
                    h.runtime.native_approval = Some(t);
                }
                "runtime" => {
                    h.runtime = super::super::session_runtime::SessionRuntime::detached(24, 80)
                }
                "control" => h.runtime.control_tx = tokio::sync::mpsc::channel(1).0,
                "creation" => h.info.created_at = "new creation".into(),
                "attention" => {
                    h.runtime.attention_owner.accepted_write();
                }
                "root" => h.info.resume.as_mut().unwrap().harness_session_id = Some("other".into()),
                "harness" => h.info.harness = Some("claude-code".into()),
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
        assert!(
            matches!(
                writes
                    .finish_native_observation_coordinated_for_test(snapshot, result())
                    .await,
                Err(StoreError::Conflict)
            ),
            "{change}"
        );
        assert_eq!(
            diagnostic.lock().unwrap().observations,
            0,
            "{change}: obsolete RPC must not enter the reducer"
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
            "2",
            "{change}"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn sql_observer_pipeline_snapshot_cannot_move_to_a_rebound_application_owner() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let cancelled = Arc::new(AtomicBool::new(false));
    writes
        .begin_native_observation(ID.into(), generation, cancelled.clone())
        .await
        .unwrap();
    let snapshot = writes
        .begin_native_observation(ID.into(), generation, cancelled)
        .await
        .unwrap()
        .unwrap();
    let other =
        crate::session_application::sql_writes::SessionWrites::bind(state, store.clone()).unwrap();
    assert!(matches!(
        other
            .finish_native_observation_coordinated_for_test(snapshot, result())
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_pipeline_read_stall_cannot_adopt_a_replacement_tracker() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let paused = crate::session_store::test_worker::pause(&store);
    let reader = tokio::spawn(async move {
        writes
            .begin_native_observation(ID.into(), generation, Arc::new(AtomicBool::new(false)))
            .await
    });
    crate::session_store::test_worker::wait_for_get(&store, ID).await;
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    {
        let mut sessions = state
            .sessions
            .try_lock()
            .expect("SQL read retained the session guard");
        let h = sessions.get_mut(ID).unwrap();
        let mut t = NativeApprovalState::new(generation);
        t.root = Some("root".into());
        h.runtime.native_approval = Some(t);
    }
    drop(paused);
    assert!(matches!(reader.await.unwrap(), Err(StoreError::Conflict)));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_pipeline_metadata_read_time_cannot_refresh_an_old_rpc() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let cancelled = Arc::new(AtomicBool::new(false));
    writes
        .begin_native_observation(ID.into(), generation, cancelled.clone())
        .await
        .unwrap();
    let snapshot = writes
        .begin_native_observation(ID.into(), generation, cancelled)
        .await
        .unwrap()
        .unwrap();
    let response = result();
    let paused = crate::session_store::test_worker::pause(&store);
    let finisher = tokio::spawn(async move {
        writes
            .finish_native_observation_coordinated_for_test(snapshot, response)
            .await
    });
    crate::session_store::test_worker::wait_for_get(&store, ID).await;
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    tokio::time::sleep(std::time::Duration::from_millis(310)).await;
    drop(paused);
    assert!(finisher.await.unwrap().unwrap());
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    let sessions = state.sessions.lock().unwrap();
    let h = &sessions[ID];
    assert_eq!(h.info.attention.as_deref(), Some("needs_you"));
    assert!(h
        .runtime
        .native_approval
        .as_ref()
        .unwrap()
        .protects_permission_wait(&h.runtime.attention_owner, &h.info));
    drop(sessions);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_pipeline_cancelled_metadata_read_never_publishes_a_tick() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let paused = crate::session_store::test_worker::pause(&store);
    let reader = tokio::spawn(async move {
        writes
            .begin_native_observation(ID.into(), generation, Arc::new(AtomicBool::new(false)))
            .await
    });
    crate::session_store::test_worker::wait_for_get(&store, ID).await;
    reader.abort();
    assert!(matches!(reader.await, Err(error) if error.is_cancelled()));
    drop(paused);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    assert_ne!(
        state.sessions.lock().unwrap()[ID].info.attention.as_deref(),
        Some("needs_you")
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_pipeline_discards_rpc_when_durable_native_identity_changes() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let cancelled = Arc::new(AtomicBool::new(false));
    writes
        .begin_native_observation(ID.into(), generation, cancelled.clone())
        .await
        .unwrap();
    let snapshot = writes
        .begin_native_observation(ID.into(), generation, cancelled)
        .await
        .unwrap()
        .unwrap();
    let response = result();
    store
        .mutate_internal(
            ID.into(),
            super::sql_tests::CREATED.into(),
            None,
            crate::metadata::internal_mutations::InternalMutation::HarnessReport {
                report: crate::metadata::HarnessSessionReport {
                    harness_session_id: "other-root".into(),
                    source: "codex_hook".into(),
                    confidence: 1.0,
                },
                timestamp: super::sql_tests::CREATED.into(),
                allow_codex_identity_replacement: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["resume"]["harnessSessionId"],
        "other-root"
    );
    assert!(matches!(
        writes
            .finish_native_observation_coordinated_for_test(snapshot, response)
            .await,
        Err(StoreError::Conflict)
    ));
    let row = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(row["metadataRevision"], "3");
    assert_eq!(row["attention"], "needs_you");
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_pipeline_preserves_new_work_recorded_during_rpc() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let cancelled = Arc::new(AtomicBool::new(false));
    writes
        .begin_native_observation(ID.into(), generation, cancelled.clone())
        .await
        .unwrap();
    let snapshot = writes
        .begin_native_observation(ID.into(), generation, cancelled)
        .await
        .unwrap()
        .unwrap();
    let response = result();
    store
        .mutate_internal(
            ID.into(),
            super::sql_tests::CREATED.into(),
            None,
            crate::metadata::internal_mutations::InternalMutation::UserTitle {
                title: Some("newer work".into()),
            },
        )
        .await
        .unwrap();
    assert!(writes
        .finish_native_observation_coordinated_for_test(snapshot, response)
        .await
        .unwrap());
    let row = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(row["label"], "newer work");
    assert_eq!(row["metadataRevision"], "4");
    assert_eq!(row["attention"], "working");
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_pipeline_rejects_a_response_requested_before_its_snapshot() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let cancelled = Arc::new(AtomicBool::new(false));
    writes
        .begin_native_observation(ID.into(), generation, cancelled.clone())
        .await
        .unwrap();
    let old_response = result();
    let snapshot = writes
        .begin_native_observation(ID.into(), generation, cancelled)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        writes
            .finish_native_observation_coordinated_for_test(snapshot, old_response)
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_pipeline_retains_post_rpc_metadata_read_deadline_errors() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let cancelled = Arc::new(AtomicBool::new(false));
    writes
        .begin_native_observation(ID.into(), generation, cancelled.clone())
        .await
        .unwrap();
    let snapshot = writes
        .begin_native_observation(ID.into(), generation, cancelled)
        .await
        .unwrap()
        .unwrap();
    let response = result();
    let paused = crate::session_store::test_worker::pause(&store);
    let finish = tokio::spawn(async move {
        writes
            .finish_native_observation_coordinated_for_test(snapshot, response)
            .await
    });
    crate::session_store::test_worker::wait_for_get(&store, ID).await;
    let deadline = tokio::time::timeout(std::time::Duration::from_secs(4), finish).await;
    drop(paused);
    assert!(matches!(
        deadline.unwrap().unwrap(),
        Err(StoreError::Expired)
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "2"
    );
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.attention.as_deref(),
        Some("needs_you")
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_recovers_rpc_polling_after_detached_wait_without_adopting_proof() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let writer = writes.clone();
    let cancellation = cancelled.clone();
    let pending = tokio::spawn(async move {
        writer
            .begin_native_observation(ID.into(), generation, cancellation)
            .await
    });
    held.entered().await;
    let transition = super::super::prompt_authority::transition_lock()
        .lock()
        .unwrap();
    drop(held);
    assert!(pending.await.unwrap().unwrap().is_none());
    drop(transition);
    assert_ne!(
        state.sessions.lock().unwrap()[ID].info.attention.as_deref(),
        Some("needs_you")
    );
    let mut snapshot = None;
    for _ in 0..2 {
        snapshot = writes
            .begin_native_observation(ID.into(), generation, cancelled.clone())
            .await
            .unwrap();
        if snapshot.is_some() {
            break;
        }
    }
    let snapshot = snapshot.expect("polling must resume after transition contention clears");
    assert!(writes
        .finish_native_observation_coordinated_for_test(snapshot, result())
        .await
        .unwrap());
    let record = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(record["attention"], "needs_you");
    assert_eq!(record["metadataRevision"], "2");
    assert!(state.sessions.lock().unwrap()[ID]
        .runtime
        .native_approval
        .as_ref()
        .unwrap()
        .sql_ownership
        .is_none());
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_observer_rejected_tick_cleanup_cannot_invalidate_a_replacement() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for replacement in ["hook", "attention", "tracker", "cancel"] {
        let (_dir, state, store, _writes, issued, _commands) = fixture_with_root("root").await;
        {
            let mut sessions = state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            let generation = h.runtime.run_generation();
            let cancelled = Arc::new(AtomicBool::new(false));
            let origin =
                sql_context::ObservationContext::capture(h, generation, cancelled.clone(), None)
                    .unwrap();
            match replacement {
                "hook" => {
                    let t = h.runtime.native_approval.as_mut().unwrap();
                    t.hook_revision = t.hook_revision.map(|r| r + 1);
                }
                "attention" => {
                    h.runtime.attention_owner.accepted_write();
                }
                "tracker" => {
                    h.runtime.native_approval.as_mut().unwrap().instance = Arc::new(());
                }
                "cancel" => cancelled.store(true, Ordering::Release),
                _ => unreachable!(),
            }
            assert!(
                !sql_context::reject_sql_observation_effect(h, &origin, &issued, &None),
                "{replacement}"
            );
            assert!(
                h.runtime
                    .native_approval
                    .as_ref()
                    .unwrap()
                    .reducer
                    .accepts(&issued.effect),
                "replacement candidate was invalidated: {replacement}"
            );
        }
        store.close().await.unwrap();
    }
}
