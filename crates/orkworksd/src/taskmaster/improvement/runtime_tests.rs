use super::*;
use crate::taskmaster::{
    knowledge::test_support::{activation, bundle_value},
    readiness_rubric::CONCEPT_IDS,
    runtime::{ContextLevel, TaskmasterSettings},
};
use std::sync::mpsc;

fn fixture() -> (
    tempfile::TempDir,
    Arc<crate::AppState>,
    Arc<TaskmasterRuntime>,
) {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(workspace.join("AGENTS.md"), "Fixture repository guidance.").unwrap();
    let state = crate::test_support::test_app_state_with_workspace(&workspace);
    let runtime = Arc::new(TaskmasterRuntime::open_fixture(dir.path().join("runtime")));
    let mut bundle = bundle_value("valid-activation.json");
    bundle["formatVersion"] = serde_json::json!(2);
    bundle["capabilities"] = serde_json::json!([
        "taskmaster-improvement-v1",
        "taskmaster-readiness-v1",
        "taskmaster-assessment-v1"
    ]);
    let page_id = bundle["pages"][0]["id"].as_str().unwrap().to_owned();
    let page_sha = bundle["pages"][0]["sha256"].as_str().unwrap().to_owned();
    bundle["readinessRubric"] = serde_json::json!({"id":"brain-agent-readiness","revision":1,"concepts":CONCEPT_IDS.iter().map(|id|serde_json::json!({"conceptId":id,"pageId":page_id,"pageSha256":page_sha,"levels":["Implicit","Discoverable","Usable","Integrated","Adaptive"]})).collect::<Vec<_>>()});
    let mut entry = bundle["pages"][0].clone();
    entry["id"] = serde_json::json!("playbooks/taskmaster-improvement.md");
    entry["relatedIds"] = serde_json::json!([page_id]);
    bundle["pages"].as_array_mut().unwrap().push(entry);
    runtime
        .activate_knowledge(&activation(bundle, false))
        .unwrap();
    let mut settings = TaskmasterSettings::default();
    settings.context_level = ContextLevel::SourceCode;
    settings.selection = Some(
        serde_json::from_value(serde_json::json!({"provider":"codex","model":"fixture"})).unwrap(),
    );
    runtime.replace_settings(settings).unwrap();
    (dir, state, runtime)
}

#[tokio::test]
async fn cancellation_keeps_lease_until_invocation_settles_and_discards_late_output() {
    let (_dir, state, runtime) = fixture();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let invoke = Box::new(move |_: &str, _: Duration| {
        entered_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        Ok(r#"{"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}"#.into())
    });
    let admission = schedule_qualified(state.clone(), runtime.clone(), invoke);
    assert_eq!(admission.outcome, "scheduled");
    let id = admission.run_id.unwrap();
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(cancel(&state, &id).outcome, "cancel_requested");
    assert!(runtime.try_analysis_lease().unwrap().is_none());
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    assert_eq!(
        ReportStore::new(&root).load().unwrap().unwrap().state,
        RunState::Running
    );
    release_tx.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let record = ReportStore::new(&root).load().unwrap().unwrap();
        if record.state == RunState::Interrupted {
            assert_eq!(record.error_code.as_deref(), Some("cancelled"));
            assert!(record.report.is_none());
            break;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    while runtime.try_analysis_lease().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn status_recovers_abandoned_run_without_inference_and_keeps_production_disabled() {
    let (_dir, state, runtime) = fixture();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut record = Record::queued("improvement-stale", "2026-01-01T00:00:00Z");
    record.state = RunState::Running;
    ReportStore::new(&root).save(&record).unwrap();
    let status = status_with_runtime(&state, &runtime, false);
    assert!(!status.available);
    assert_eq!(status.reason_code, Some("qualification_disabled"));
    let run = status.run.unwrap();
    assert_eq!(run.state, RunState::Interrupted);
    assert_eq!(run.error_code.as_deref(), Some("interrupted"));
    assert!(run.report.is_none());
}

fn preferred_invoke() -> FixtureInvoke {
    Box::new(|prompt: &str, _: Duration| {
        let prompt: serde_json::Value = serde_json::from_str(prompt).unwrap();
        if prompt["phase"] == "evidence" {
            return Ok(serde_json::json!({"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}).to_string());
        }
        let hash = prompt["repositoryFacts"][0]["sha256"].clone();
        let page = prompt["knowledgePages"][0]["id"].clone();
        let concepts = prompt["rubric"]["concepts"].as_array().unwrap().iter().map(|c| serde_json::json!({"conceptId":c["conceptId"],"maturity":2,"confidence":"medium","repositoryFactIds":[0],"rubricPageId":c["pageId"],"rationale":"Documented guidance","limits":["Behavior remains unverified"],"nextTransition":"Evidence of workflow integration","unassessedReason":null})).collect::<Vec<_>>();
        Ok(serde_json::json!({"version":1,"summary":"Scoped guidance offers a practical next improvement.","concepts":concepts,
            "opportunities":[{"id":"scoped-guidance","targetSurface":"instructions","title":"Scoped guidance","summary":"The entry point would link the existing scoped instructions.","mechanism":"The inspected entry point contains only general guidance.","benefit":"The relevant scoped constraints would be discoverable.","evidenceStrength":"supported_hypothesis","effort":"low","risk":"low","reversibility":"high","repositoryFactHashes":[hash],"knowledgePageIds":[page],"validation":"Whether scoped constraints are discoverable remains unverified.","rollback":"The prior entry point would be restored."}],
            "preferredOpportunityId":"scoped-guidance","noProposalReason":null,"limitations":["Behavior remains unverified"]}).to_string())
    })
}

#[tokio::test]
async fn qualified_investigation_publishes_one_shared_current_report_and_fences_changed_evidence() {
    let (_dir, state, runtime) = fixture();
    let invoke = preferred_invoke();
    let admission = schedule_qualified(state.clone(), runtime.clone(), invoke);
    assert_eq!(admission.outcome, "scheduled");
    let id = admission.run_id.unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let status = status_with_runtime(&state, &runtime, true);
        if status.run.as_ref().is_some_and(|run| {
            matches!(
                run.state,
                RunState::Succeeded | RunState::Failed | RunState::Interrupted
            )
        }) {
            let run = status.run.unwrap();
            assert_eq!(
                run.state,
                RunState::Succeeded,
                "error: {:?}",
                run.error_code
            );
            assert_eq!(run.id, id);
            let report = run.report.unwrap();
            assert_eq!(report["coverage"], 14);
            assert_eq!(report["overallScore"], 50);
            assert!(report["recommendationId"].is_string());
            assert_eq!(
                report["concepts"][0]["repositoryFactHashes"][0],
                report["repositoryEvidence"][0]["sha256"]
            );
            assert_eq!(report["repositoryEvidence"][0]["complete"], true);
            break;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    while runtime.try_analysis_lease().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let history_root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let history = crate::taskmaster::readiness_store::ReadinessStore::new(&history_root)
        .history()
        .unwrap();
    assert_eq!(history.snapshots.len(), 1);
    let history_wire = serde_json::to_value(&history).unwrap();
    assert_eq!(
        history_wire["snapshots"][0]["data"]["inputManifest"]["purpose"],
        "brain-improvement-v1"
    );
    let corrupt_root = tempfile::tempdir().unwrap();
    let mut unsupported = history_wire;
    unsupported["snapshots"][0]["data"]["inputManifest"]["collector"]["version"] =
        serde_json::json!(2);
    std::fs::write(
        corrupt_root.path().join("readiness-history.json"),
        unsupported.to_string(),
    )
    .unwrap();
    assert!(
        crate::taskmaster::readiness_store::ReadinessStore::new(corrupt_root.path())
            .history()
            .is_err()
    );
    let second = schedule_qualified(
        state.clone(),
        runtime.clone(),
        Box::new(|_, _| panic!("existing recommendation must prevent dispatch")),
    );
    assert_eq!(second.outcome, "unavailable");
    assert_eq!(second.reason_code, Some("active_recommendation"));
    let visible = crate::taskmaster::assessment_access::with_recommendations_using_runtime(
        &state,
        &runtime,
        true,
        |e| e,
        |_, items| Ok(items),
    )
    .unwrap()
    .unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].assessment.as_ref().unwrap().id, id);
    let report_store = ReportStore::new(&history_root);
    let mut pending = report_store.load().unwrap().unwrap();
    let dismissed = crate::taskmaster::store::dismissed_record(
        visible[0].clone(),
        "2026-01-01T00:00:00Z",
        None,
    );
    assert!(publication::join_valid(
        pending.report.as_ref().unwrap(),
        &id,
        &dismissed
    ));
    let mut altered = visible[0].clone();
    altered.suggested_model = Some("unapproved-model".into());
    assert!(!publication::join_valid(
        pending.report.as_ref().unwrap(),
        &id,
        &altered
    ));
    pending.state = RunState::PendingRecommendation;
    pending.pending_recommendation = Some(altered);
    assert!(report_store.save(&pending).is_err());
    pending.pending_recommendation = Some(visible[0].clone());
    report_store.save(&pending).unwrap();
    let recovered = status_with_runtime(&state, &runtime, true).run.unwrap();
    assert_eq!(recovered.state, RunState::Succeeded);
    assert!(recovered.report.is_some());
    assert_eq!(
        crate::taskmaster::readiness_store::ReadinessStore::new(&history_root)
            .history()
            .unwrap()
            .snapshots
            .len(),
        1
    );
    let closed_root = tempfile::tempdir().unwrap();
    let closed_metadata = closed_root.path().join("closed");
    std::fs::create_dir(&closed_metadata).unwrap();
    let workspace_path = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path
        .clone();
    crate::workspace_runtime::write_origin_file_if_absent(
        &closed_metadata,
        &workspace_path.canonicalize().unwrap(),
    );
    for name in ["improvement-assessment.json", "readiness-history.json"] {
        std::fs::copy(history_root.join(name), closed_metadata.join(name)).unwrap();
    }
    let closed_recommendations =
        crate::taskmaster::store::RecommendationStore::open(closed_metadata.clone()).unwrap();
    closed_recommendations.put(&visible[0]).unwrap();
    closed_recommendations
        .dismiss(&visible[0].id, "2026-01-01T00:00:00Z".into(), None)
        .unwrap();
    let narrowed = TaskmasterSettings {
        context_level: ContextLevel::SessionObservations,
        ..TaskmasterSettings::default()
    };
    crate::taskmaster::assessment_access::reconcile_known_workspaces(
        closed_root.path(),
        &narrowed,
        None,
    )
    .unwrap();
    assert_eq!(
        ReportStore::new(&closed_metadata)
            .load()
            .unwrap()
            .unwrap()
            .state,
        RunState::AccessRedacted
    );
    assert!(
        crate::taskmaster::readiness_store::ReadinessStore::new(&closed_metadata)
            .history()
            .unwrap()
            .snapshots[0]
            .redacted
    );
    let sanitized = closed_recommendations.get(&visible[0].id).unwrap().unwrap();
    assert_eq!(
        sanitized.status,
        crate::taskmaster::RecommendationStatus::Dismissed
    );
    assert!(sanitized.assessment.unwrap().access_redacted);
    assert!(sanitized.repository_evidence.is_empty());
    assert!(sanitized.knowledge_evidence.is_empty());
    let mut stale_pending = report_store.load().unwrap().unwrap();
    stale_pending.state = RunState::PendingRecommendation;
    stale_pending.pending_recommendation = Some(visible[0].clone());
    report_store.save(&stale_pending).unwrap();
    let path = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path
        .join("AGENTS.md");
    std::fs::write(path, "Changed repository guidance.").unwrap();
    let stale_status = status_with_runtime(&state, &runtime, true).run.unwrap();
    assert_eq!(stale_status.state, RunState::Interrupted);
    assert_eq!(stale_status.error_code.as_deref(), Some("inputs_changed"));
    assert!(stale_status.report.is_none());
    let stale = crate::taskmaster::assessment_access::with_recommendations_using_runtime(
        &state,
        &runtime,
        true,
        |e| e,
        |_, items| Ok(items),
    )
    .unwrap()
    .unwrap();
    assert!(stale.is_empty());
}

#[tokio::test]
async fn access_change_signals_inflight_invocation_and_suppresses_publication() {
    let (_dir, state, runtime) = fixture();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let invoke = Box::new(move |_: &str, _: Duration| {
        entered_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        Ok(r#"{"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}"#.into())
    });
    let admission = schedule_qualified(state.clone(), runtime.clone(), invoke);
    assert_eq!(admission.outcome, "scheduled");
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut settings = TaskmasterSettings::default();
    settings.context_level = ContextLevel::SessionObservations;
    settings.selection = Some(
        serde_json::from_value(serde_json::json!({"provider":"codex","model":"fixture"})).unwrap(),
    );
    runtime.replace_settings(settings).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if active()
            .lock()
            .unwrap()
            .get(&root)
            .is_some_and(|run| run.cancelled.load(Ordering::SeqCst))
        {
            break;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(runtime.try_analysis_lease().unwrap().is_none());
    release_tx.send(()).unwrap();
    loop {
        let record = ReportStore::new(&root).load().unwrap().unwrap();
        if record.state == RunState::Interrupted {
            assert_eq!(record.error_code.as_deref(), Some("inputs_changed"));
            assert!(record.report.is_none());
            break;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn no_proposal_invoke() -> FixtureInvoke {
    Box::new(|prompt: &str, _: Duration| {
        let prompt: serde_json::Value = serde_json::from_str(prompt).unwrap();
        if prompt["phase"] == "evidence" {
            return Ok(serde_json::json!({"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}).to_string());
        }
        let rows = prompt["rubric"]["concepts"].as_array().unwrap().iter().map(|c| serde_json::json!({"conceptId":c["conceptId"],"maturity":2,"confidence":"medium","repositoryFactIds":[0],"rubricPageId":c["pageId"],"rationale":"Documented guidance","limits":["Behavior remains unverified"],"nextTransition":"Evidence of workflow integration","unassessedReason":null})).collect::<Vec<_>>();
        Ok(serde_json::json!({"version":1,"summary":"The supplied evidence leaves no preferred opportunity.","concepts":rows,"opportunities":[],"preferredOpportunityId":null,"noProposalReason":"No supported next step was identified.","limitations":["Behavior remains unverified"]}).to_string())
    })
}

#[tokio::test]
async fn no_proposal_report_retains_cited_signed_rubric_snapshots() {
    let (_dir, state, runtime) = fixture();
    let invoke = no_proposal_invoke();
    let admission = schedule_qualified(state.clone(), runtime.clone(), invoke);
    assert_eq!(admission.outcome, "scheduled");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(run) = status_with_runtime(&state, &runtime, true).run {
            if matches!(
                run.state,
                RunState::Succeeded | RunState::Failed | RunState::Interrupted
            ) {
                assert_eq!(run.state, RunState::Succeeded, "{:?}", run.error_code);
                let report = run.report.unwrap();
                assert!(report["recommendationId"].is_null());
                let rubric = report["concepts"][0]["rubricPageId"].as_str().unwrap();
                assert!(report["knowledgeEvidence"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|page| page["pageId"] == rubric));
                break;
            }
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn history_failure_is_reported_without_hiding_committed_assessment() {
    let (_dir, state, runtime) = fixture();
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    std::fs::write(root.join("readiness-history.json"), "corrupt").unwrap();
    let admission = schedule_qualified(state.clone(), runtime.clone(), no_proposal_invoke());
    assert_eq!(admission.outcome, "scheduled");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(run) = status_with_runtime(&state, &runtime, true).run {
            if matches!(
                run.state,
                RunState::Succeeded | RunState::Failed | RunState::Interrupted
            ) {
                assert_eq!(run.state, RunState::Succeeded, "{:?}", run.error_code);
                let report = run.report.unwrap();
                assert_eq!(report["historyError"], "history_unavailable");
                assert!(report["historySnapshotId"].is_null());
                assert_eq!(report["coverage"], 14);
                break;
            }
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn dismissed_matching_recommendation_is_not_recreated_by_an_identical_investigation() {
    let (_dir, state, runtime) = fixture();
    let wait = |state: Arc<crate::AppState>, runtime: Arc<TaskmasterRuntime>| async move {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(run) = status_with_runtime(&state, &runtime, true).run {
                if matches!(
                    run.state,
                    RunState::Succeeded | RunState::Failed | RunState::Interrupted
                ) {
                    assert_eq!(run.state, RunState::Succeeded, "{:?}", run.error_code);
                    while runtime.try_analysis_lease().unwrap().is_none() {
                        assert!(Instant::now() < deadline);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    break run.report.unwrap();
                }
            }
            assert!(Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    assert_eq!(
        schedule_qualified(state.clone(), runtime.clone(), preferred_invoke()).outcome,
        "scheduled"
    );
    let first = wait(state.clone(), runtime.clone()).await;
    let id = first["recommendationId"].as_str().unwrap().to_owned();
    {
        let lock = state.workspace.lock().unwrap();
        let workspace = lock.as_ref().unwrap();
        workspace
            .recommendation_store
            .dismiss(&id, "2026-01-01T00:00:00Z".into(), None)
            .unwrap();
    }
    let dismissed = crate::taskmaster::assessment_access::with_recommendations_using_runtime(
        &state,
        &runtime,
        false,
        |error| error,
        |_, items| Ok(items),
    )
    .unwrap()
    .unwrap();
    assert_eq!(dismissed.len(), 1);
    assert_eq!(
        dismissed[0].status,
        crate::taskmaster::RecommendationStatus::Dismissed
    );
    assert_eq!(
        schedule_qualified(state.clone(), runtime.clone(), preferred_invoke()).outcome,
        "scheduled"
    );
    let second = wait(state.clone(), runtime.clone()).await;
    assert!(second["recommendationId"].is_null());
    assert!(second["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|text| text == "An existing recommendation prevented a new card."));
    let lock = state.workspace.lock().unwrap();
    let cards = lock.as_ref().unwrap().recommendation_store.list().unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(
        cards[0].status,
        crate::taskmaster::RecommendationStatus::Dismissed
    );
}

#[test]
fn custom_authority_gate_and_cancel_have_one_lock_order_and_no_late_dispatch() {
    let (_dir, state, runtime) = fixture();
    let capture = access::capture(&state, &runtime, None).unwrap();
    let evidence =
        crate::taskmaster::assessment_context::improvement::ProgressiveEvidence::collect(
            &capture.path,
            capture.evaluation.settings.context_level,
            &capture.evaluation.settings.excluded_paths,
            "now",
        )
        .unwrap();
    let selected =
        vec![crate::taskmaster::runtime::assessment::IMPROVEMENT_ENTRY_PAGE_ID.to_string()];
    let manifest = access::manifest(&capture, &evidence, &selected).unwrap();
    let entry = Active {
        id: "improvement-lock-test".into(),
        instance: capture.instance,
        cancelled: Arc::new(AtomicBool::new(false)),
        user_cancelled: Arc::new(AtomicBool::new(false)),
        inputs_changed: Arc::new(AtomicBool::new(false)),
        settled: Arc::new(AtomicBool::new(false)),
        dispatch: Arc::new(Mutex::new(())),
    };
    active()
        .lock()
        .unwrap()
        .insert(capture.root.clone(), entry.clone());
    let _owner = ActiveGuard {
        root: capture.root.clone(),
        id: entry.id.clone(),
    };
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let state_for_gate = state.clone();
    let runtime_for_gate = runtime.clone();
    let capture_for_gate = capture.clone();
    let manifest_for_gate = manifest.clone();
    let entry_for_gate = entry.clone();
    let worker = std::thread::spawn(move || {
        let valid = runtime_for_gate
            .with_current_evaluation(
                &state_for_gate.harness_store,
                &capture_for_gate.path,
                &capture_for_gate.evaluation,
                || {
                    authorized_dispatch(
                        &state_for_gate,
                        &runtime_for_gate,
                        &capture_for_gate,
                        &manifest_for_gate,
                        &entry_for_gate,
                        Instant::now() + Duration::from_secs(5),
                        true,
                        |_| {
                            entered_tx.send(()).unwrap();
                            release_rx.recv().unwrap();
                            Ok(())
                        },
                    )
                    .unwrap();
                },
            )
            .unwrap();
        assert!(valid);
        done_tx.send(()).unwrap();
    });
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let (cancel_started_tx, cancel_started_rx) = mpsc::channel();
    let (cancel_done_tx, cancel_done_rx) = mpsc::channel();
    let state_for_cancel = state.clone();
    let id = entry.id.clone();
    let canceller = std::thread::spawn(move || {
        cancel_started_tx.send(()).unwrap();
        cancel_done_tx
            .send(cancel(&state_for_cancel, &id).outcome)
            .unwrap();
    });
    cancel_started_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    release_tx.send(()).unwrap();
    done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        cancel_done_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        "cancel_requested"
    );
    worker.join().unwrap();
    canceller.join().unwrap();
    let mut dispatched = false;
    assert!(authorized_dispatch(
        &state,
        &runtime,
        &capture,
        &manifest,
        &entry,
        Instant::now() + Duration::from_secs(5),
        false,
        |_| {
            dispatched = true;
            Ok(())
        }
    )
    .is_err());
    assert!(!dispatched);
    assert_eq!(
        settle_failure(&entry, Instant::now() + Duration::from_secs(5)),
        ("cancelled", true)
    );
    assert_eq!(cancel(&state, &entry.id).outcome, "not_running");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn qualified_run_uses_controlled_http_transport_against_local_fixture() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (_dir, state, runtime) = fixture();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut invoke = no_proposal_invoke();
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let (header_end, content_length) = loop {
                let mut chunk = [0u8; 4096];
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&chunk[..n]);
                assert!(request.len() < 256 * 1024);
                if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                    let head = std::str::from_utf8(&request[..end]).unwrap();
                    let length = head
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while request.len() < header_end + content_length {
                let mut chunk = [0u8; 4096];
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&chunk[..n]);
            }
            let body: serde_json::Value =
                serde_json::from_slice(&request[header_end..header_end + content_length]).unwrap();
            let output = invoke(body["prompt"].as_str().unwrap(), Duration::from_secs(5)).unwrap();
            let response = serde_json::json!({"response":output,"done":true}).to_string();
            let head = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.len());
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let mut settings = TaskmasterSettings::default();
    settings.context_level = ContextLevel::SourceCode;
    settings.selection = Some(serde_json::from_value(serde_json::json!({"provider":"ollama","model":"fixture","ollamaBaseUrl":format!("http://{address}")})).unwrap());
    runtime.replace_settings(settings).unwrap();
    let admission = schedule_inner(state.clone(), runtime.clone(), None);
    assert_eq!(admission.outcome, "scheduled");
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(run) = status_with_runtime(&state, &runtime, true).run {
            if matches!(
                run.state,
                RunState::Succeeded | RunState::Failed | RunState::Interrupted
            ) {
                assert_eq!(run.state, RunState::Succeeded, "{:?}", run.error_code);
                assert_eq!(run.report.unwrap()["coverage"], 14);
                break;
            }
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
