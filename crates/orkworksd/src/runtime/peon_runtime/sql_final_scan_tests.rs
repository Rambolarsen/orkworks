use super::sql_reservation_tests::fixture;
use super::*;
use crate::session_application::sql_writes::lifecycle::LifecycleWrite;
const ID: &str = "sql-runtime";
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn sql_runtime_final_scan_persists_provider_and_evidence_before_history() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _commands) = fixture().await;
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
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut(ID)
        .unwrap()
        .output_buffer
        .push("failure".into());
    let owner = crate::runtime::terminal_runtime::final_scan::FinalScanPersistence::sql(
        state.clone(),
        writes,
        ID.into(),
        runtime.run_generation,
    )
    .await
    .unwrap();
    let result = owner.run("ended".into(), |_| async { Some(scan()) }).await;
    assert!(matches!(
        result,
        crate::runtime::terminal_runtime::final_scan::FinalScanCompletion::Sql(Ok(_))
    ));
    let record = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(
        record["modelProviderId"], "final-provider",
        "runtime history completion has no final-scan provider stage"
    );
    assert_eq!(qualified_count(&state), 1);
    assert_eq!(observations(&state).evaluation_generation(), 0);
    store.close().await.unwrap();
}

fn scan() -> crate::providers::ProviderRunResult {
    crate::providers::ProviderRunResult {
        observation:Some(crate::providers::ProviderObservation{provider_id:"final-provider".into(),provider_label:"Final provider".into(),provider_model:Some("final-model".into()),provider_state:"healthy".into()}),
        inference:Some(serde_json::from_value(serde_json::json!({"observedStatus":"blocked","confidence":0.8,"workflowObservations":[{"kind":"obstacle","description":"blocked","evidence":"failure","reportedImpact":"low","confidence":0.8}]})).unwrap()),
        attempts:Vec::new(),runtime:Default::default(),
    }
}

use crate::runtime::terminal_runtime::final_scan::{FinalScanCompletion, FinalScanPersistence};
use crate::session_application::ending_completion::WriteResult;
use crate::session_application::sql_writes::lifecycle::{MetadataResult, ProviderWrite};
use crate::session_store::StoreError;
use std::time::Duration;
async fn ending(
    state: &Arc<AppState>,
    writes: &Arc<crate::session_application::sql_writes::SessionWrites>,
    output: bool,
) -> u64 {
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
    if output {
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut(ID)
            .unwrap()
            .output_buffer
            .push("failure".into());
    }
    runtime.run_generation
}
fn files(state: &Arc<AppState>) -> crate::metadata::MetadataStore {
    crate::metadata::MetadataStore::new(
        &state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path(),
    )
}
fn observations(state: &Arc<AppState>) -> crate::workflow_observations::WorkflowObservationStore {
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .workflow_observations
        .clone()
}
fn qualified_count(state: &Arc<AppState>) -> usize {
    let path = files(state)
        .root_path()
        .join(format!("workflow-observations/{ID}.ndjson"));
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return 0,
        Err(e) => panic!("{e}"),
    };
    bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let record = crate::workflow_observations::sql_origin::decode_line(line, ID).unwrap();
            assert!(record.origin().is_some());
            usize::from(record.observation().is_some())
        })
        .sum()
}
async fn stored(store: &crate::session_store::SessionStore) -> crate::metadata::SessionMetadata {
    serde_json::from_value(store.get(ID.into()).await.unwrap().unwrap()).unwrap()
}
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_partial_stages_preserve_history_progress_without_retry() {
    let _guard = diagnostic_test_guard();
    for fault in ["provider", "evidence", "grid", "event"] {
        let (_dir, state, store, writes, _, _, _, _) = fixture().await;
        let generation = ending(&state, &writes, true).await;
        let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
            .await
            .unwrap();
        let files = files(&state);
        let legacy = files.sessions_dir().join(format!("{ID}.json"));
        let legacy_before = std::fs::read(&legacy).unwrap();
        let mut result = scan();
        match fault {
            "provider" => {
                result.observation.as_mut().unwrap().provider_id = "x".repeat(1024 * 1024)
            }
            "evidence" => std::fs::create_dir(
                files
                    .root_path()
                    .join("workflow-observations")
                    .join(format!("{ID}.ndjson")),
            )
            .unwrap(),
            "grid" | "event" => {
                std::fs::create_dir_all(files.root_path().join("events").join(format!(
                    "{ID}.{}",
                    if fault == "grid" {
                        "terminal-size"
                    } else {
                        "ndjson"
                    }
                )))
                .unwrap()
            }
            _ => unreachable!(),
        }
        let FinalScanCompletion::Sql(Ok(stages)) = owner
            .run("ended".into(), |_| async move { Some(result) })
            .await
        else {
            panic!("missing SQL stage report")
        };
        if fault == "provider" {
            assert!(matches!(stages.provider, Some(Err(StoreError::TooLarge))));
        } else {
            assert!(matches!(
                stages.provider,
                Some(Ok(ProviderWrite::Committed))
            ));
        }
        if fault == "evidence" {
            assert!(matches!(stages.evidence, Some(Err(_))));
            assert!(
                !files
                    .root_path()
                    .join("workflow-observations/sequence")
                    .exists(),
                "strict preflight refuses a nonregular segment before the counter"
            );
        } else {
            assert_eq!(qualified_count(&state), 1);
        }
        let history = stages.history.unwrap();
        if fault == "grid" {
            assert!(matches!(history.replay, WriteResult::Failed(_)));
            assert!(matches!(history.metadata, MetadataResult::NotAttempted));
            assert_eq!(stored(&store).await.lifecycle_phase, "ending");
        } else {
            assert!(matches!(
                history.metadata,
                MetadataResult::Completed(
                    crate::session_application::sql_writes::Outcome::Committed
                )
            ));
            assert_eq!(stored(&store).await.lifecycle_phase, "ended");
            if fault == "event" {
                assert!(matches!(history.event, WriteResult::Failed(_)));
            }
        }
        assert_eq!(std::fs::read(legacy).unwrap(), legacy_before, "{fault}");
        store.close().await.unwrap();
    }
}
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_fallback_uses_sql_ending_snapshot_and_skips_empty_inference() {
    let _guard = diagnostic_test_guard();
    for mode in ["empty", "timeout", "no-inference", "no-status"] {
        let (_dir, state, store, writes, _, _, _, _) = fixture().await;
        let generation = ending(&state, &writes, mode != "empty").await;
        let files = files(&state);
        let mut metadata = store.get(ID.into()).await.unwrap().unwrap();
        let expected = crate::metadata::ObservedStatusSnapshotMetadata {
            value: Some("working".into()),
            source: "process".into(),
            confidence: Some(0.7),
            observed_at: Some("2026-11-05T00:00:00Z".into()),
            attention_origin: None,
        };
        metadata["endingObservedStatusSnapshot"] = serde_json::to_value(&expected).unwrap();
        {
            let conn =
                rusqlite::Connection::open(files.root_path().join("session-store/v1.sqlite"))
                    .unwrap();
            conn.execute(
                "UPDATE sessions SET metadata=?1 WHERE id=?2",
                rusqlite::params![serde_json::to_string(&metadata).unwrap(), ID],
            )
            .unwrap();
        }
        let legacy = files.sessions_dir().join(format!("{ID}.json"));
        std::fs::write(&legacy, b"invalid legacy must not provide fallback").unwrap();
        let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
            .await
            .unwrap();
        let called = Arc::new(AtomicBool::new(false));
        let notice = called.clone();
        let report = owner
            .run("ended".into(), move |output| async move {
                notice.store(true, Ordering::SeqCst);
                assert_eq!(output, vec!["failure"]);
                match mode {
                    "timeout" => None,
                    "no-inference" => {
                        let mut result = scan();
                        result.inference = None;
                        Some(result)
                    }
                    "no-status" => {
                        let mut result = scan();
                        let inf = result.inference.as_mut().unwrap();
                        inf.observed_status = None;
                        inf.workflow_observations.clear();
                        Some(result)
                    }
                    _ => panic!("empty output dispatched inference"),
                }
            })
            .await;
        assert_eq!(called.load(Ordering::SeqCst), mode != "empty");
        let FinalScanCompletion::Sql(Ok(stages)) = report else {
            panic!("SQL fallback failed")
        };
        assert!(stages.evidence.is_none());
        assert!(stages.history.is_ok());
        assert_eq!(
            stored(&store).await.final_observed_status_snapshot,
            Some(expected)
        );
        assert_eq!(
            std::fs::read(legacy).unwrap(),
            b"invalid legacy must not provide fallback"
        );
        store.close().await.unwrap();
    }
}
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_original_capture_rejects_replacements_across_inference_wait() {
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
        let generation = ending(&state, &writes, true).await;
        let old_files = files(&state);
        let old_observations = observations(&state);
        let before = store.get(ID.into()).await.unwrap();
        let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
            .await
            .unwrap();
        let (entered, at_inference) = tokio::sync::oneshot::channel();
        let (release, wait) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            owner
                .run("ended".into(), |_| async move {
                    let _ = entered.send(());
                    let _ = wait.await;
                    Some(scan())
                })
                .await
        });
        at_inference.await.unwrap();
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
                        dir.path().join("replacement-workflow"),
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
        release.send(()).unwrap();
        assert!(
            matches!(
                task.await.unwrap(),
                FinalScanCompletion::Sql(Err(StoreError::Conflict))
            ),
            "{change}"
        );
        assert_eq!(store.get(ID.into()).await.unwrap(), before, "{change}");
        assert_eq!(old_observations.session_observation_count(ID).unwrap(), 0);
        assert!(old_files.read_events(ID).is_empty());
        assert!(old_files.read_terminal_size(ID).is_none());
        store.close().await.unwrap();
    }
}
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_missing_ended_or_unavailable_sql_never_uses_valid_json() {
    let _guard = diagnostic_test_guard();
    for fault in ["missing", "ended", "unavailable"] {
        let (_dir, state, store, writes, _, _, _, _) = fixture().await;
        let generation = ending(&state, &writes, true).await;
        let files = files(&state);
        let legacy = files.sessions_dir().join(format!("{ID}.json"));
        let before = std::fs::read(&legacy).unwrap();
        let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
            .await
            .unwrap();
        let (entered, at_inference) = tokio::sync::oneshot::channel();
        let (release, wait) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            owner
                .run("ended".into(), |_| async move {
                    let _ = entered.send(());
                    let _ = wait.await;
                    Some(scan())
                })
                .await
        });
        at_inference.await.unwrap();
        if fault == "unavailable" {
            store.close().await.unwrap();
        } else if fault == "ended" {
            store
                .mutate_internal(
                    ID.into(),
                    "2026-01-01T00:00:00Z".into(),
                    None,
                    crate::metadata::internal_mutations::InternalMutation::Lifecycle(
                        crate::metadata::lifecycle_mutations::LifecycleMutation::Finalize {
                            snapshot: crate::metadata::canonical_null_snapshot("process", None),
                            fallback_status: "ended".into(),
                            timestamp: "2026-11-05T00:00:00Z".into(),
                        },
                    ),
                )
                .await
                .unwrap();
        } else {
            let conn =
                rusqlite::Connection::open(files.root_path().join("session-store/v1.sqlite"))
                    .unwrap();
            conn.execute("DELETE FROM sessions WHERE id=?1", [ID])
                .unwrap();
        }
        release.send(()).unwrap();
        let result = task.await.unwrap();
        assert!(
            matches!(
                result,
                FinalScanCompletion::Sql(Err(StoreError::Missing
                    | StoreError::Conflict
                    | StoreError::Unavailable))
            ),
            "{result:?}"
        );
        assert_eq!(std::fs::read(legacy).unwrap(), before);
        assert!(files.read_events(ID).is_empty());
        assert!(files.read_terminal_size(ID).is_none());
        assert_eq!(qualified_count(&state), 0);
        if fault != "unavailable" {
            store.close().await.unwrap();
        }
    }
}
#[tokio::test]
async fn final_scan_default_json_preserves_legacy_fields_and_leaves_sql_untouched() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, _writes, _, _, _, _) = fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    crate::runtime::terminal_runtime::set_session_status_for_generation(
        &state, ID, generation, "killed",
    )
    .await;
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut(ID)
        .unwrap()
        .output_buffer
        .push("failure".into());
    let before = store.get(ID.into()).await.unwrap();
    let result = FinalScanPersistence::json(state.clone(), ID.into(), generation)
        .run("ended".into(), |_| async { Some(scan()) })
        .await;
    assert!(matches!(result, FinalScanCompletion::Json(Ok(_))));
    let meta = files(&state).read_session(ID).unwrap();
    assert_eq!(meta.lifecycle_phase, "ended");
    assert_eq!(meta.provider_id.as_deref(), Some("final-provider"));
    assert_eq!(
        meta.final_observed_status_snapshot
            .unwrap()
            .value
            .as_deref(),
        Some("blocked")
    );
    assert_eq!(
        observations(&state).session_observation_count(ID).unwrap(),
        1
    );
    assert_eq!(observations(&state).evaluation_generation(), 1);
    assert_eq!(store.get(ID.into()).await.unwrap(), before);
    store.close().await.unwrap();
}
#[tokio::test]
async fn final_scan_json_missing_metadata_completes_live_handle_with_null_recovery() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, _writes, _, _, _, _) = fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    crate::runtime::terminal_runtime::set_session_status_for_generation(
        &state, ID, generation, "killed",
    )
    .await;
    std::fs::remove_file(files(&state).sessions_dir().join(format!("{ID}.json"))).unwrap();
    let result = FinalScanPersistence::json(state.clone(), ID.into(), generation)
        .run("ended".into(), |_| async {
            panic!("empty output dispatched inference")
        })
        .await;
    let FinalScanCompletion::Json(Ok(done)) = result else {
        panic!("JSON recovery failed")
    };
    assert!(done.published);
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ended"
    );
    store.close().await.unwrap();
}

#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_duplicate_evidence_keeps_original_identity_without_rescheduling() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let generation = ending(&state, &writes, true).await;
    let evidence = observations(&state);
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    let captured = writes
        .capture_ending_history(&state, ID.into(), runtime)
        .await
        .unwrap()
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let receipt = captured
        .persist_workflow_observations(
            generation,
            scan().inference.unwrap().workflow_observations,
            vec!["failure".into()],
        )
        .await
        .unwrap();
    let crate::session_application::sql_writes::lifecycle::SqlCandidateResult::Recorded(Ok(
        crate::workflow_observations::RecordOutcome::Accepted(original),
    )) = receipt.candidates.into_iter().next().unwrap()
    else {
        panic!("fixture not accepted")
    };
    drop(captured);
    let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
    let FinalScanCompletion::Sql(Ok(stages)) =
        owner.run("ended".into(), |_| async { Some(scan()) }).await
    else {
        panic!("duplicate failed")
    };
    let evidence_result = stages.evidence.unwrap().unwrap();
    assert!(
        matches!(&evidence_result.candidates[0],crate::session_application::sql_writes::lifecycle::SqlCandidateResult::Recorded(Ok(crate::workflow_observations::RecordOutcome::Duplicate{observation_id,sequence,..})) if observation_id==&original.id && sequence==&original.sequence)
    );
    assert_eq!(evidence.evaluation_generation(), 0);
    assert_eq!(qualified_count(&state), 1);
    store.close().await.unwrap();
}
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_stage_order_is_visible_before_history_admission() {
    use crate::session_application::ending_completion::testing::{hold, Stage};
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let generation = ending(&state, &writes, true).await;
    let files = files(&state);
    let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
    let mut pause = hold(&state, Stage::BeforeHistory);
    let task =
        tokio::spawn(async move { owner.run("ended".into(), |_| async { Some(scan()) }).await });
    tokio::time::timeout(Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    let metadata = stored(&store).await;
    assert_eq!(metadata.provider_id.as_deref(), Some("final-provider"));
    assert_eq!(metadata.lifecycle_phase, "ending");
    assert_eq!(qualified_count(&state), 1);
    assert_eq!(observations(&state).evaluation_generation(), 0);
    assert!(files.read_terminal_size(ID).is_none());
    assert!(files.read_events(ID).is_empty());
    drop(pause);
    let FinalScanCompletion::Sql(Ok(stages)) = task.await.unwrap() else {
        panic!("history failed")
    };
    assert!(stages.history.is_ok());
    let metadata = stored(&store).await;
    let snapshot = metadata.final_observed_status_snapshot.unwrap();
    assert_eq!(snapshot.value.as_deref(), Some("blocked"));
    assert_eq!(snapshot.source, "peon");
    assert_eq!(snapshot.confidence, Some(0.8));
    assert!(snapshot.observed_at.is_some());
    let events = files.read_events(ID);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].observed_status.as_deref(), Some("blocked"));
    store.close().await.unwrap();
}
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_cancelled_inference_releases_capture_without_persistence() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let generation = ending(&state, &writes, true).await;
    let before = store.get(ID.into()).await.unwrap();
    let files = files(&state);
    let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let job = tokio::spawn(async move {
        owner
            .run("ended".into(), |_| async move {
                let _ = entered.send(());
                std::future::pending().await
            })
            .await
    });
    waiting.await.unwrap();
    let lease = {
        let mut ws = state.workspace.lock().unwrap();
        Arc::downgrade(&ws.as_mut().unwrap().lease.take().unwrap())
    };
    let lane = store.runtime_lane(ID).unwrap();
    assert!(
        lane.try_lock().is_ok(),
        "inference retained the mutation lane"
    );
    job.abort();
    let _ = job.await;
    assert!(lease.upgrade().is_none());
    assert_eq!(store.get(ID.into()).await.unwrap(), before);
    assert_eq!(qualified_count(&state), 0);
    assert!(files.read_events(ID).is_empty());
    assert!(files.read_terminal_size(ID).is_none());
    store.close().await.unwrap();
}
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_cancelled_admitted_provider_keeps_definitive_stage_without_later_effects() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let generation = ending(&state, &writes, true).await;
    let files = files(&state);
    let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
    let mut pause = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let job =
        tokio::spawn(async move { owner.run("ended".into(), |_| async { Some(scan()) }).await });
    tokio::time::timeout(Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    let lease = {
        let mut ws = state.workspace.lock().unwrap();
        Arc::downgrade(&ws.as_mut().unwrap().lease.take().unwrap())
    };
    let lane = store.runtime_lane(ID).unwrap();
    job.abort();
    let _ = job.await;
    assert!(lane.try_lock().is_err());
    assert!(lease.upgrade().is_some());
    drop(pause);
    let released = tokio::time::timeout(Duration::from_secs(5), lane.lock_owned())
        .await
        .unwrap();
    drop(released);
    tokio::time::timeout(Duration::from_secs(2), async {
        while lease.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let metadata = stored(&store).await;
    assert_eq!(metadata.provider_id.as_deref(), Some("final-provider"));
    assert_eq!(metadata.lifecycle_phase, "ending");
    assert_eq!(qualified_count(&state), 0);
    assert!(files.read_events(ID).is_empty());
    assert!(files.read_terminal_size(ID).is_none());
    store.close().await.unwrap();
}
#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_explicit_sql_runtime_entry_finishes_empty_output_without_provider_dispatch() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let generation = ending(&state, &writes, false).await;
    let result = crate::runtime::terminal_runtime::finalize_session_ending_sql(
        state.clone(),
        writes,
        ID.into(),
        generation,
        "ended".into(),
    )
    .await
    .unwrap();
    let FinalScanCompletion::Sql(Ok(stages)) = result else {
        panic!("explicit SQL runtime entry failed")
    };
    assert!(stages.provider.is_none());
    assert!(stages.evidence.is_none());
    assert!(stages.history.is_ok());
    assert_eq!(stored(&store).await.lifecycle_phase, "ended");
    store.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_full_pipeline_stall_leaves_other_runtime_input_and_guards_available() {
    use crate::runtime::session_runtime::RuntimeCommand;
    let _guard = diagnostic_test_guard();
    let (dir, state, store, writes, _, _, _, _commands) = fixture().await;
    let generation = ending(&state, &writes, true).await;
    let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
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

    let mut pause = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (progress_tx, progress_rx) = tokio::sync::oneshot::channel();
    let watchdog = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let progressed = runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), pause.entered())
                .await
                .unwrap();
            let _ = entered_tx.send(());
            tokio::time::timeout(Duration::from_secs(2), progress_rx)
                .await
                .is_ok_and(|r| r.is_ok())
        });
        drop(pause);
        progressed
    });
    let writer =
        tokio::spawn(async move { owner.run("ended".into(), |_| async { Some(scan()) }).await });
    entered_rx.await.unwrap();
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
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
        "final-scan pipeline blocked the sole Tokio worker until OS watchdog release"
    );
    delivery_result.unwrap().unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    let FinalScanCompletion::Sql(Ok(stages)) = writer.await.unwrap() else {
        panic!("pipeline failed")
    };
    assert!(stages.history.is_ok());
    assert_eq!(qualified_count(&state), 1);
    assert_eq!(stored(&store).await.lifecycle_phase, "ended");
    store.close().await.unwrap();
}
#[test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn final_scan_pipeline_finishes_with_only_one_blocking_worker() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        let _guard = diagnostic_test_guard();
        let (_dir, state, store, writes, _, _, _, _) = fixture().await;
        let generation = ending(&state, &writes, true).await;
        let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
            .await
            .unwrap();
        let completed = tokio::time::timeout(
            Duration::from_secs(5),
            owner.run("ended".into(), |_| async { Some(scan()) }),
        )
        .await;
        store.close().await.unwrap();
        completed
    });
    runtime.shutdown_timeout(Duration::from_secs(2));
    assert!(matches!(result, Ok(FinalScanCompletion::Sql(Ok(_)))));
}
#[tokio::test]
async fn final_scan_production_entry_stays_json_without_activating_sql() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, _writes, _, _, _, _) = fixture().await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    crate::runtime::terminal_runtime::set_session_status_for_generation(
        &state, ID, generation, "killed",
    )
    .await;
    let before = store.get(ID.into()).await.unwrap();
    crate::runtime::terminal_runtime::finalize_session_ending(
        state.clone(),
        ID.into(),
        generation,
        "ended".into(),
    )
    .await;
    assert_eq!(
        files(&state).read_session(ID).unwrap().lifecycle_phase,
        "ended"
    );
    assert_eq!(store.get(ID.into()).await.unwrap(), before);
    store.close().await.unwrap();
}

#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_stale_output_capture_never_dispatches_inference() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let generation = ending(&state, &writes, true).await;
    let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
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
    let result = owner
        .run("ended".into(), |_| async {
            panic!("stale captured owner dispatched inference")
        })
        .await;
    assert!(matches!(
        result,
        FinalScanCompletion::Sql(Err(StoreError::Conflict))
    ));
    assert_eq!(stored(&store).await.lifecycle_phase, "ending");
    assert!(stored(&store).await.provider_id.is_none());
    store.close().await.unwrap();
}

#[tokio::test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
async fn final_scan_legacy_retry_collision_refuses_without_upgrade_or_evaluation() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let generation = ending(&state, &writes, true).await;
    let workflow = observations(&state);
    let candidate = scan().inference.unwrap().workflow_observations.remove(0);
    workflow
        .record_observation(
            ID,
            crate::workflow_observations::ObservationOrigin::Peon,
            &format!("final-scan:{generation}:0"),
            crate::workflow_observations::ObservationCandidate {
                kind: candidate.kind,
                description: candidate.description,
                evidence: candidate.evidence,
                problem_area: candidate.problem_area,
                reported_impact: candidate.reported_impact,
                confidence: Some(candidate.confidence),
            },
        )
        .unwrap();
    let path = files(&state)
        .root_path()
        .join(format!("workflow-observations/{ID}.ndjson"));
    let before = std::fs::read(&path).unwrap();
    let owner = FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation)
        .await
        .unwrap();
    let FinalScanCompletion::Sql(Ok(stages)) =
        owner.run("ended".into(), |_| async { Some(scan()) }).await
    else {
        panic!("missing stages")
    };
    assert!(matches!(
        &stages.evidence.unwrap().unwrap().candidates[0],
        crate::session_application::sql_writes::lifecycle::SqlCandidateResult::Recorded(Err(
            crate::workflow_observations::sql_origin::WriteError::Storage(StoreError::Conflict)
        ))
    ));
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert_eq!(workflow.evaluation_generation(), 0);
    assert!(stages.history.is_ok());
    store.close().await.unwrap();
}

#[tokio::test]
#[cfg(not(unix))]
async fn private_sql_final_scan_refuses_unsupported_root_backend() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let generation = ending(&state, &writes, true).await;
    let before = store.get(ID.into()).await.unwrap();
    assert!(matches!(
        FinalScanPersistence::sql(state.clone(), writes, ID.into(), generation).await,
        Err(StoreError::Unsupported)
    ));
    assert_eq!(store.get(ID.into()).await.unwrap(), before);
    assert!(!files(&state)
        .root_path()
        .join("workflow-observations/sequence")
        .exists());
    store.close().await.unwrap();
}
