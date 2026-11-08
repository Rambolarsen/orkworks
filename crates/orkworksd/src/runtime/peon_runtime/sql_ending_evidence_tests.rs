use super::sql_reservation_tests::fixture;
use super::*;
use crate::session_application::sql_writes::lifecycle::LifecycleWrite;
use std::time::Duration;
#[tokio::test]
async fn captured_evidence_rejects_deleted_permit_without_recreating_segment() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let runtime = state.sessions.lock().unwrap()["sql-runtime"]
        .runtime
        .identity();
    writes
        .lifecycle(
            "sql-runtime".into(),
            runtime.clone(),
            LifecycleWrite::Transition {
                status: "killed".into(),
            },
        )
        .await
        .unwrap();
    let captured = writes
        .capture_ending_history(&state, "sql-runtime".into(), runtime)
        .await
        .unwrap()
        .with_workflow_evidence()
        .await
        .unwrap();
    let evidence = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .workflow_observations
        .clone();
    evidence.delete_session_observations("sql-runtime").unwrap();
    let candidate = peon::PeonWorkflowObservation {
        kind: crate::workflow_observations::ObservationKind::Obstacle,
        description: "blocked".into(),
        evidence: "failure".into(),
        problem_area: None,
        reported_impact: crate::workflow_observations::Impact::Low,
        confidence: 0.8,
    };
    assert!(matches!(
        captured
            .persist_workflow_observations(1, vec![candidate], vec!["failure".into()])
            .await,
        Err(crate::session_store::StoreError::Conflict)
    ));
    assert_eq!(
        evidence.session_observation_count("sql-runtime").unwrap(),
        0
    );
    store.close().await.unwrap();
}

use crate::session_application::sql_writes::lifecycle::{CandidateResult, CapturedEndingEvidence};
use crate::session_store::StoreError;
const ID: &str = "sql-runtime";
fn candidate() -> peon::PeonWorkflowObservation {
    peon::PeonWorkflowObservation {
        kind: crate::workflow_observations::ObservationKind::Obstacle,
        description: "blocked".into(),
        evidence: "failure".into(),
        problem_area: None,
        reported_impact: crate::workflow_observations::Impact::Low,
        confidence: 0.8,
    }
}
async fn capture(
    writes: &Arc<crate::session_application::sql_writes::SessionWrites>,
    state: &Arc<AppState>,
) -> CapturedEndingEvidence {
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    writes
        .lifecycle(
            ID.into(),
            runtime.clone(),
            LifecycleWrite::Transition {
                status: "killed".into(),
            },
        )
        .await
        .unwrap();
    writes
        .capture_ending_history(state, ID.into(), runtime)
        .await
        .unwrap()
        .with_workflow_evidence()
        .await
        .unwrap()
}
fn evidence(state: &Arc<AppState>) -> crate::workflow_observations::WorkflowObservationStore {
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .workflow_observations
        .clone()
}
#[tokio::test]
async fn evidence_exact_indexes_duplicates_partial_errors_and_metadata_preservation() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let capture = capture(&writes, &state).await;
    captured_provider(&capture).await;
    let before = store.get(ID.into()).await.unwrap();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let legacy = root.join("sessions").join(format!("{ID}.json"));
    let legacy_before = std::fs::read(&legacy).unwrap();
    let mut ungrounded = candidate();
    ungrounded.evidence = "not present".into();
    let mut invalid = candidate();
    invalid.confidence = f64::NAN;
    let result = capture
        .persist_workflow_observations(
            7,
            vec![ungrounded, candidate(), invalid],
            vec!["failure".into()],
        )
        .await
        .unwrap();
    assert!(!result.detached);
    assert!(result.evaluation_scheduled);
    assert!(matches!(result.candidates[0], CandidateResult::Ungrounded));
    let CandidateResult::Recorded(Ok(crate::workflow_observations::RecordOutcome::Accepted(
        ref first,
    ))) = result.candidates[1]
    else {
        panic!("expected accepted")
    };
    assert!(matches!(
        result.candidates[2],
        CandidateResult::Recorded(Err(
            crate::workflow_observations::RecordError::ConfidenceOutOfRange
        ))
    ));
    let repeated = capture
        .persist_workflow_observations(
            7,
            vec![
                {
                    let mut c = candidate();
                    c.evidence = "not present".into();
                    c
                },
                candidate(),
            ],
            vec!["failure".into()],
        )
        .await
        .unwrap();
    assert!(!repeated.evaluation_scheduled);
    assert!(
        matches!(&repeated.candidates[1],CandidateResult::Recorded(Ok(crate::workflow_observations::RecordOutcome::Duplicate{observation_id,sequence,..})) if observation_id==&first.id && sequence==&first.sequence)
    );
    assert_eq!(store.get(ID.into()).await.unwrap(), before);
    assert_eq!(std::fs::read(&legacy).unwrap(), legacy_before);
    let snapshot = crate::metadata::ObservedStatusSnapshotMetadata {
        value: None,
        source: "process".into(),
        confidence: None,
        observed_at: None,
        attention_origin: None,
    };
    capture.complete(snapshot, "ended".into()).await.unwrap();
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["lifecyclePhase"],
        "ended"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn evidence_original_owner_replacements_cannot_capture_or_publish() {
    let _guard = diagnostic_test_guard();
    for change in [
        "workspace",
        "reader",
        "lease",
        "store",
        "runtime",
        "control",
        "creation",
        "source",
        "attention",
        "phase",
    ] {
        let (dir, state, store, writes, _, _, _, _) = fixture().await;
        let captured = capture(&writes, &state).await;
        let old = evidence(&state);
        match change {
            "workspace" => crate::test_support::replace_sql_workspace_context(&state, dir.path()),
            "reader" => {
                let mut ws = state.workspace.lock().unwrap();
                let ws = ws.as_mut().unwrap();
                ws.metadata = crate::metadata::MetadataStore::new(&ws.metadata.root_path());
            }
            "lease" => state.workspace.lock().unwrap().as_mut().unwrap().lease = None,
            "store" => {
                let mut ws = state.workspace.lock().unwrap();
                let ws = ws.as_mut().unwrap();
                ws.workflow_observations =
                    crate::workflow_observations::WorkflowObservationStore::open(
                        ws.metadata.root_path(),
                    )
                    .unwrap();
            }
            _ => {
                let mut sessions = state.sessions.lock().unwrap();
                let h = sessions.get_mut(ID).unwrap();
                match change {
                    "runtime" => {
                        h.runtime =
                            crate::runtime::session_runtime::SessionRuntime::detached(24, 80)
                    }
                    "control" => h.runtime.control_tx = tokio::sync::mpsc::channel(2).0,
                    "creation" => h.info.created_at = "replacement".into(),
                    "source" => h.info.metadata_source = Some("user".into()),
                    "attention" => {
                        h.runtime.attention_owner.accepted_write();
                    }
                    "phase" => h.info.lifecycle_phase = "active".into(),
                    _ => unreachable!(),
                }
            }
        }
        assert!(
            matches!(
                captured
                    .persist_workflow_observations(1, vec![candidate()], vec!["failure".into()])
                    .await,
                Err(StoreError::Conflict)
            ),
            "{change}"
        );
        assert_eq!(old.session_observation_count(ID).unwrap(), 0, "{change}");
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn evidence_rejects_size_expiry_and_queued_cancel_without_late_effects() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = capture(&writes, &state).await;
    assert!(matches!(
        captured
            .persist_workflow_observations(1, vec![candidate(); 6], vec![])
            .await,
        Err(StoreError::TooLarge)
    ));
    assert!(matches!(
        captured
            .persist_workflow_observations(1, vec![candidate()], vec!["x".repeat(1024 * 1024)])
            .await,
        Err(StoreError::TooLarge)
    ));
    let lane = store.runtime_lane(ID).unwrap();
    let held = lane.clone().lock_owned().await;
    assert!(matches!(
        captured
            .persist_workflow_observations(1, vec![candidate()], vec!["failure".into()])
            .await,
        Err(StoreError::Expired)
    ));
    let job = tokio::spawn(async move {
        captured
            .persist_workflow_observations(1, vec![candidate()], vec!["failure".into()])
            .await
    });
    tokio::task::yield_now().await;
    job.abort();
    let _ = job.await;
    drop(held);
    let drained = tokio::time::timeout(Duration::from_secs(3), lane.lock_owned())
        .await
        .unwrap();
    drop(drained);
    assert_eq!(evidence(&state).session_observation_count(ID).unwrap(), 0);
    store.close().await.unwrap();
}
#[tokio::test]
async fn evidence_persist_failure_is_not_retried_or_collapsed_into_success() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = capture(&writes, &state).await;
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    std::fs::create_dir(
        root.join("workflow-observations")
            .join(format!("{ID}.ndjson")),
    )
    .unwrap();
    let result = captured
        .persist_workflow_observations(1, vec![candidate()], vec!["failure".into()])
        .await
        .unwrap();
    assert!(matches!(
        result.candidates[0],
        CandidateResult::Recorded(Err(
            crate::workflow_observations::RecordError::PersistFailed
        ))
    ));
    assert!(!result.evaluation_scheduled);
    assert_eq!(
        std::fs::read_to_string(root.join("workflow-observations/sequence"))
            .unwrap()
            .trim(),
        "1"
    );
    assert_eq!(evidence(&state).session_observation_count(ID).unwrap(), 0);
    store.close().await.unwrap();
}
#[tokio::test]
async fn admitted_evidence_retains_lane_and_lease_after_caller_cancellation() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = capture(&writes, &state).await;
    let old = evidence(&state);
    let (gate, release) = crate::workflow_observations::captured_testing::hold(&old);
    let job = tokio::spawn(async move {
        captured
            .persist_workflow_observations(1, vec![candidate()], vec!["failure".into()])
            .await
    });
    tokio::time::timeout(Duration::from_secs(3), gate.entered())
        .await
        .unwrap();
    let lease = {
        let mut ws = state.workspace.lock().unwrap();
        Arc::downgrade(&ws.as_mut().unwrap().lease.take().unwrap())
    };
    job.abort();
    let _ = job.await;
    assert!(lease.upgrade().is_some());
    let lane = store.runtime_lane(ID).unwrap();
    assert!(lane.try_lock().is_err());
    release.send(()).unwrap();
    let drained = tokio::time::timeout(Duration::from_secs(3), lane.lock_owned())
        .await
        .unwrap();
    drop(drained);
    tokio::time::timeout(Duration::from_secs(2), async {
        while lease.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(old.session_observation_count(ID).unwrap(), 1);
    assert_eq!(old.evaluation_generation(), 0);
    store.close().await.unwrap();
}
#[test]
fn evidence_owner_does_not_recursively_wait_on_single_blocking_thread() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let _guard = diagnostic_test_guard();
        let (_dir, state, store, writes, _, _, _, _) = fixture().await;
        let captured = capture(&writes, &state).await;
        assert!(
            !tokio::time::timeout(
                Duration::from_secs(5),
                captured.persist_workflow_observations(
                    1,
                    vec![candidate()],
                    vec!["failure".into()]
                )
            )
            .await
            .unwrap()
            .unwrap()
            .detached
        );
        store.close().await.unwrap();
    });
}

#[tokio::test]
async fn evidence_sql_missing_ended_and_creation_mismatch_never_fall_back_to_json() {
    let _guard = diagnostic_test_guard();
    for change in ["missing", "ended", "creation", "unavailable"] {
        let (_dir, state, store, writes, _, _, _, _) = fixture().await;
        let captured = capture(&writes, &state).await;
        let root = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path();
        let legacy = root.join("sessions").join(format!("{ID}.json"));
        let before = std::fs::read(&legacy).unwrap();
        if change == "unavailable" {
            store.close().await.unwrap();
        } else {
            let conn = rusqlite::Connection::open(root.join("session-store/v1.sqlite")).unwrap();
            match change {
                "missing" => {
                    conn.execute("DELETE FROM sessions WHERE id=?1", [ID])
                        .unwrap();
                }
                _ => {
                    let mut record = store.get(ID.into()).await.unwrap().unwrap();
                    let (created, phase) = if change == "creation" {
                        ("reused", "ending")
                    } else {
                        ("2026-01-01T00:00:00Z", "ended")
                    };
                    record["createdAt"] = serde_json::json!(created);
                    record["lifecyclePhase"] = serde_json::json!(phase);
                    conn.execute(
                        "UPDATE sessions SET created_at=?1,metadata=?2 WHERE id=?3",
                        rusqlite::params![created, record.to_string(), ID],
                    )
                    .unwrap();
                }
            }
        }
        let result = captured
            .persist_workflow_observations(1, vec![candidate()], vec!["failure".into()])
            .await;
        assert!(
            matches!(
                result,
                Err(StoreError::Missing | StoreError::Conflict | StoreError::Unavailable)
            ),
            "{change}: {result:?}"
        );
        assert_eq!(evidence(&state).session_observation_count(ID).unwrap(), 0);
        assert_eq!(std::fs::read(legacy).unwrap(), before);
        if change != "unavailable" {
            store.close().await.unwrap();
        }
    }
}
#[tokio::test]
async fn evidence_admitted_old_owner_returns_detached_without_scheduling_replacement() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = capture(&writes, &state).await;
    let old = evidence(&state);
    let (gate, release) = crate::workflow_observations::captured_testing::hold(&old);
    let job = tokio::spawn(async move {
        captured
            .persist_workflow_observations(1, vec![candidate()], vec!["failure".into()])
            .await
    });
    tokio::time::timeout(Duration::from_secs(3), gate.entered())
        .await
        .unwrap();
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut(ID)
        .unwrap()
        .runtime
        .control_tx = tokio::sync::mpsc::channel(2).0;
    release.send(()).unwrap();
    let receipt = job.await.unwrap().unwrap();
    assert!(receipt.detached);
    assert!(!receipt.evaluation_scheduled);
    assert!(matches!(
        receipt.candidates[0],
        CandidateResult::Recorded(Ok(crate::workflow_observations::RecordOutcome::Accepted(_)))
    ));
    assert_eq!(old.evaluation_generation(), 0);
    store.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn evidence_stall_leaves_sql_executor_guards_and_other_input_available() {
    use crate::runtime::session_runtime::RuntimeCommand;
    let _guard = diagnostic_test_guard();
    let (dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let captured = capture(&writes, &state).await;
    let input_id = "provider-input";
    let (runtime, mut input_commands) =
        crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let mut info = crate::test_support::test_session_info(
        input_id,
        "Input",
        dir.path().display().to_string(),
        "running",
        "2026-01-01T00:00:00Z",
    );
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    state.sessions.lock().unwrap().insert(
        input_id.into(),
        crate::SessionHandle {
            info,
            runtime,
            kill_tx: tokio::sync::watch::channel(false).0,
            output_buffer: peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
            active_work_hook: false,
        },
    );
    let meta = crate::test_support::test_session_metadata(
        input_id,
        "Input",
        dir.path().display().to_string(),
        "running",
        "2026-01-01T00:00:00Z",
        "2026-01-02T00:00:00Z",
    );
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .try_write_session(&meta)
        .unwrap();

    let old = evidence(&state);
    let (gate, release) = crate::workflow_observations::captured_testing::hold(&old);
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (progress_tx, progress_rx) = tokio::sync::oneshot::channel();
    let watchdog = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let progressed = rt.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), gate.entered())
                .await
                .unwrap();
            let _ = entered_tx.send(());
            tokio::time::timeout(Duration::from_secs(2), progress_rx)
                .await
                .is_ok_and(|r| r.is_ok())
        });
        let _ = release.send(());
        progressed
    });
    let writer = tokio::spawn(async move {
        captured
            .persist_workflow_observations(1, vec![candidate()], vec!["failure".into()])
            .await
    });
    entered_rx.await.unwrap();
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(
        tokio::time::timeout(Duration::from_secs(1), store.get(ID.into()))
            .await
            .unwrap()
            .unwrap()
            .is_some()
    );
    let delivery = tokio::spawn(async move {
        match input_commands.recv().await.unwrap() {
            RuntimeCommand::Input {
                data,
                accepted: Some(accepted),
            } => {
                assert_eq!(data, "independent-input\r");
                let _ = accepted.send(Ok(()));
            }
            _ => panic!("expected input delivery"),
        }
    });
    let delivered = tokio::time::timeout(
        Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            input_id,
            "independent-input\r".into(),
        ),
    )
    .await;
    let delivery_result = tokio::time::timeout(Duration::from_secs(1), delivery).await;
    let _ = progress_tx.send(());
    assert!(
        watchdog.join().unwrap(),
        "file owner blocked the Tokio worker until watchdog release"
    );
    delivery_result.unwrap().unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(!writer.await.unwrap().unwrap().detached);
    store.close().await.unwrap();
}

#[tokio::test]
async fn evidence_registration_preserves_original_owner_and_expires_outside_guards() {
    let _guard = diagnostic_test_guard();
    for expired in [false, true] {
        let (_dir, state, store, writes, _, _, _, _) = fixture().await;
        let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
        writes
            .lifecycle(
                ID.into(),
                runtime.clone(),
                LifecycleWrite::Transition {
                    status: "killed".into(),
                },
            )
            .await
            .unwrap();
        let captured = writes
            .capture_ending_history(&state, ID.into(), runtime)
            .await
            .unwrap();
        let old = evidence(&state);
        let (gate, release, thread, registration_reached) =
            crate::workflow_observations::captured_testing::hold_registration(&old);
        tokio::time::timeout(Duration::from_secs(3), gate.entered())
            .await
            .unwrap();
        let job = tokio::spawn(async move { captured.with_workflow_evidence().await });
        // Wait until initial bounded memory validation has released its guards.
        // A scheduler yield alone can race that legitimate validation.
        tokio::time::timeout(Duration::from_secs(3), registration_reached)
            .await
            .unwrap()
            .unwrap();
        assert!(state.projection_lock.try_lock().is_ok());
        assert!(state.workspace.try_lock().is_ok());
        assert!(state.sessions.try_lock().is_ok());
        if !expired {
            state
                .sessions
                .lock()
                .unwrap()
                .get_mut(ID)
                .unwrap()
                .runtime
                .control_tx = tokio::sync::mpsc::channel(2).0;
            release.send(()).unwrap();
            assert!(matches!(job.await.unwrap(), Err(StoreError::Conflict)));
        } else {
            assert!(matches!(
                tokio::time::timeout(Duration::from_secs(3), job)
                    .await
                    .unwrap()
                    .unwrap(),
                Err(StoreError::Expired)
            ));
            release.send(()).unwrap();
        }
        thread.join().unwrap();
        assert_eq!(old.session_observation_count(ID).unwrap(), 0);
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn evidence_registration_rejects_missing_sql_even_with_valid_legacy_json() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    writes
        .lifecycle(
            ID.into(),
            runtime.clone(),
            LifecycleWrite::Transition {
                status: "killed".into(),
            },
        )
        .await
        .unwrap();
    let captured = writes
        .capture_ending_history(&state, ID.into(), runtime)
        .await
        .unwrap();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    rusqlite::Connection::open(root.join("session-store/v1.sqlite"))
        .unwrap()
        .execute("DELETE FROM sessions WHERE id=?1", [ID])
        .unwrap();
    assert!(matches!(
        captured.with_workflow_evidence().await,
        Err(StoreError::Missing)
    ));
    assert!(root.join("sessions").join(format!("{ID}.json")).exists());
    assert_eq!(evidence(&state).session_observation_count(ID).unwrap(), 0);
    store.close().await.unwrap();
}

#[tokio::test]
async fn evidence_composition_cannot_adopt_store_replaced_since_original_capture() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    writes
        .lifecycle(
            ID.into(),
            runtime.clone(),
            LifecycleWrite::Transition {
                status: "killed".into(),
            },
        )
        .await
        .unwrap();
    let captured = writes
        .capture_ending_history(&state, ID.into(), runtime)
        .await
        .unwrap();
    {
        let mut ws = state.workspace.lock().unwrap();
        let ws = ws.as_mut().unwrap();
        ws.workflow_observations =
            crate::workflow_observations::WorkflowObservationStore::open(ws.metadata.root_path())
                .unwrap();
    }
    assert!(matches!(
        captured.with_workflow_evidence().await,
        Err(StoreError::Conflict)
    ));
    store.close().await.unwrap();
}

async fn captured_provider(capture: &CapturedEndingEvidence) {
    capture
        .persist_provider_context(crate::providers::ProviderObservation {
            provider_id: "evidence-provider".into(),
            provider_label: "Evidence provider".into(),
            provider_model: None,
            provider_state: "healthy".into(),
        })
        .await
        .unwrap();
}
