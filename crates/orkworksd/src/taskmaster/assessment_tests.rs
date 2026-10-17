use super::*;
use crate::taskmaster::{
    assessment_report::{AssessmentRecord, AssessmentState},
    assessment_store::AssessmentStore,
    runtime::{TaskmasterRuntime, TaskmasterSettings},
};
use std::{fs, sync::Arc, time::Duration};

pub(crate) struct AssessmentFixture {
    pub(crate) dir: tempfile::TempDir,
    pub(crate) state: Arc<crate::AppState>,
    pub(crate) runtime_root: std::path::PathBuf,
    marker: std::path::PathBuf,
    response: std::path::PathBuf,
    _home: crate::test_support::FakeHome,
    _path: crate::test_support::FakePath,
}

impl AssessmentFixture {
    pub(crate) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        let runtime_root = workspace.join(".orkworks-test").join("taskmaster");
        let fake_bin = dir.path().join("bin");
        let home = dir.path().join("home");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&fake_bin).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(workspace.join("README.md"), "Fixture workflow evidence.\n").unwrap();
        let marker = dir.path().join("provider-dispatched");
        let response = dir.path().join("provider-response.json");
        let codex = fake_bin.join("codex");
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'codex-cli 0.153.4\\n'; exit 0; fi\ncat > '{}'\nprintf dispatched > '{}'\nprintf x >> '{}.count'\nwhile [ ! -f '{}.release' ]; do sleep 0.01; done\npython3 -c 'import json,pathlib,sys; print(json.dumps({{\"type\":\"thread.started\"}})); print(json.dumps({{\"type\":\"turn.started\"}})); print(json.dumps({{\"type\":\"item.completed\",\"item\":{{\"type\":\"agent_message\",\"text\":pathlib.Path(sys.argv[1]).read_text()}}}})); print(json.dumps({{\"type\":\"turn.completed\"}}))' '{}' 2> '{}.stderr' | tee '{}'\n",
            dir.path().join("provider-prompt").display(),
            marker.display(),
            marker.display(),
            marker.display(),
            response.display(),
            dir.path().join("provider-output").display(),
            dir.path().join("provider-output").display(),
        );
        fs::write(&codex, script).unwrap();
        crate::test_support::make_test_executable(&codex);
        let path_guard = crate::test_support::FakePath::prepend(&fake_bin);
        let home_guard = crate::test_support::FakeHome::set(&home);
        let state = crate::test_support::test_app_state_with_workspace(&workspace);
        let runtime = TaskmasterRuntime::open(runtime_root.clone());
        runtime
            .activate_knowledge(include_bytes!(
                "../../../../apps/desktop/resources/knowledge/starter.json"
            ))
            .unwrap();
        let mut settings = TaskmasterSettings::default();
        settings.enabled = true;
        settings.daily_evaluation_limit = 1;
        settings.min_interval_minutes = 24 * 60;
        settings.selection = Some(
            serde_json::from_value(serde_json::json!({
                "provider": "codex",
                "model": "fixture-model"
            }))
            .unwrap(),
        );
        runtime.replace_settings(settings).unwrap();
        Self {
            dir,
            state,
            runtime_root,
            marker,
            response,
            _home: home_guard,
            _path: path_guard,
        }
    }

    pub(crate) fn workspace_root(&self) -> std::path::PathBuf {
        self.state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path()
    }

    pub(crate) fn write_response(&self, outcome: &str) {
        let snapshot = capture_current_snapshot(&self.state, &self.runtime_root, None).unwrap();
        let fact_hash = snapshot.facts.first().unwrap().sha256.clone();
        let page_id = snapshot
            .prompt_snapshot
            .knowledge
            .as_ref()
            .unwrap()
            .pages
            .iter()
            .find(|page| page.page_type == "playbook")
            .unwrap()
            .id
            .clone();
        let body = match outcome {
            "proposed" => serde_json::json!({
                "version": 1,
                "outcome": "proposed",
                "summary": "The supplied evidence supports a bounded documentation improvement.",
                "repositoryFactHashes": [fact_hash],
                "knowledgePageIds": [page_id],
                "unverifiedChecks": ["Run the documentation check after implementation."],
                "proposal": {
                    "targetSurface": "documentation",
                    "title": "Clarify the workflow guide",
                    "summary": "Add a concise explanation of the repository verification steps."
                },
                "noProposalReason": null
            }),
            "no_proposal" => serde_json::json!({
                "version": 1,
                "outcome": "no_proposal",
                "summary": "The supplied evidence does not identify a justified workflow change.",
                "repositoryFactHashes": [],
                "knowledgePageIds": [],
                "unverifiedChecks": [],
                "proposal": null,
                "noProposalReason": "The current workflow is adequately supported by the supplied evidence."
            }),
            other => panic!("unknown test outcome {other}"),
        };
        fs::write(&self.response, body.to_string()).unwrap();
    }

    pub(crate) fn prepare_response(&self, outcome: &str) {
        self.write_response(outcome);
        fs::write(self.marker.with_extension("release"), "go").unwrap();
    }

    fn pending_record(&self) -> AssessmentRecord {
        let snapshot = capture_current_snapshot(&self.state, &self.runtime_root, None).unwrap();
        let fact = snapshot.facts.first().unwrap().clone();
        let page = snapshot
            .prompt_snapshot
            .knowledge
            .as_ref()
            .unwrap()
            .pages
            .iter()
            .find(|page| page.page_type == "playbook")
            .unwrap()
            .clone();
        let response = serde_json::json!({
            "version": 1,
            "outcome": "proposed",
            "summary": "The supplied evidence supports a bounded documentation improvement.",
            "repositoryFactHashes": [fact.sha256],
            "knowledgePageIds": [page.id],
            "unverifiedChecks": [],
            "proposal": {
                "targetSurface": "documentation",
                "title": "Clarify the workflow guide",
                "summary": "Add a concise explanation of the repository verification steps."
            },
            "noProposalReason": null
        });
        let parsed = crate::taskmaster::assessment_model::parse_assessment_response(
            &response.to_string(),
            &snapshot.facts,
            &snapshot.prompt_snapshot.knowledge.as_ref().unwrap().pages,
        )
        .unwrap();
        let evidence = vec![fact];
        let knowledge_evidence = vec![crate::taskmaster::KnowledgeEvidence {
            page_id: page.id.clone(),
            title: page.title.clone(),
            status: page.status.clone(),
            bundle_version: snapshot
                .prompt_snapshot
                .knowledge
                .as_ref()
                .unwrap()
                .version
                .clone(),
            sha256: page.sha256,
            excerpt: page.content.chars().take(500).collect(),
        }];
        let id = "assessment-recovery-fixture";
        let dedupe = format!("proactive:v1:documentation:{}", evidence[0].sha256);
        let rec_id = "recommendation-recovery-fixture";
        let recommendation = build_assessment_recommendation(
            &snapshot,
            &parsed,
            &evidence,
            &knowledge_evidence,
            &dedupe,
            rec_id,
            id,
            None,
            "2026-10-17T00:00:00Z",
        );
        AssessmentRecord {
            format_version: 1,
            id: id.into(),
            state: AssessmentState::PendingRecommendation,
            started_at: "2026-10-17T00:00:00Z".into(),
            completed_at: Some("2026-10-17T00:01:00Z".into()),
            report: Some(crate::taskmaster::assessment_report::AssessmentReport {
                id: id.into(),
                workspace_path: snapshot.workspace_path.to_string_lossy().into_owned(),
                workspace_instance: snapshot.workspace_instance,
                observed_at: "2026-10-17T00:01:00Z".into(),
                provider: "codex".into(),
                model: "fixture-model".into(),
                bundle_version: snapshot
                    .prompt_snapshot
                    .knowledge
                    .as_ref()
                    .unwrap()
                    .version
                    .clone(),
                knowledge_identity: serde_json::to_value(&snapshot.evaluation.knowledge_identity)
                    .unwrap(),
                input_fingerprint: snapshot.fingerprint,
                context_level: snapshot.evaluation.settings.context_level,
                excluded_paths: snapshot.evaluation.settings.excluded_paths.clone(),
                outcome: "proposed".into(),
                summary: parsed.summary,
                proposal: Some(crate::taskmaster::assessment_report::AssessmentProposal {
                    target_surface: parsed.proposal.as_ref().unwrap().target_surface,
                    title: parsed.proposal.as_ref().unwrap().title.clone(),
                    summary: parsed.proposal.as_ref().unwrap().summary.clone(),
                }),
                no_proposal_reason: None,
                unverified_checks: parsed.unverified_checks,
                repository_evidence: evidence,
                knowledge_evidence,
                recommendation_id: Some(rec_id.into()),
            }),
            pending_recommendation: Some(recommendation),
            error_summary: None,
            reused: false,
        }
    }

    pub(crate) async fn wait_for_terminal(&self) -> AssessmentRecord {
        let store = AssessmentStore::new(&self.workspace_root());
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(record) = store.load::<AssessmentRecord>().unwrap() {
                if matches!(
                    record.state,
                    AssessmentState::Succeeded | AssessmentState::Failed
                ) {
                    return record;
                }
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "assessment did not finish"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    fn release_provider(&self) {
        fs::write(self.marker.with_extension("release"), "go").unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scheduled_assessment_uses_signed_native_fixture_and_keeps_ordinary_quota_untouched() {
    let fixture = AssessmentFixture::new();
    let workspace_path = fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path
        .clone();
    let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
    let evaluation = runtime.evaluation_snapshot(&workspace_path).unwrap();
    assert!(runtime
        .reserve_current(
            &workspace_path,
            &chrono::Utc::now().to_rfc3339(),
            None,
            Some(evaluation.generation),
        )
        .unwrap());
    fixture.prepare_response("proposed");
    let ordinary_before = TaskmasterRuntime::open(fixture.runtime_root.clone())
        .status(Some(&fixture.workspace_root()))
        .remaining_evaluations;
    assert_eq!(
        ordinary_before, 0,
        "ordinary quota is exhausted and cooldown is active"
    );

    let scheduled = schedule(fixture.state.clone(), fixture.runtime_root.clone());
    assert_eq!(scheduled.status, AssessmentRequestStatus::Scheduled);
    let record = fixture.wait_for_terminal().await;
    assert_eq!(
        record.state,
        AssessmentState::Succeeded,
        "{:?}; dispatched={}; prompt={:?}; stdout={:?}; stderr={:?}",
        record.error_summary,
        fixture.marker.exists(),
        fs::read_to_string(fixture.dir.path().join("provider-prompt")).ok(),
        fs::read_to_string(fixture.dir.path().join("provider-output")).ok(),
        fs::read_to_string(fixture.dir.path().join("provider-output.stderr")).ok()
    );
    let report = record.report.unwrap();
    assert_eq!(report.outcome, "proposed");
    let recommendation_id = report.recommendation_id.clone().unwrap();
    assert!(record.pending_recommendation.is_none());
    assert!(fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .recommendation_store
        .get(&recommendation_id)
        .unwrap()
        .is_some());
    let application = crate::session_application::SessionApplication::new(fixture.state.clone());
    let (listed, _) = application.list_recommendations().unwrap();
    assert!(listed
        .iter()
        .any(|recommendation| recommendation.id == recommendation_id));
    assert_eq!(
        application
            .get_recommendation(&recommendation_id)
            .unwrap()
            .unwrap()
            .id,
        recommendation_id
    );
    assert!(fixture.dir.path().join("provider-prompt").exists());

    let ordinary_after = TaskmasterRuntime::open(fixture.runtime_root.clone())
        .status(Some(&fixture.workspace_root()))
        .remaining_evaluations;
    assert_eq!(ordinary_after, ordinary_before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scheduled_assessment_publishes_a_grounded_no_proposal_report() {
    let fixture = AssessmentFixture::new();
    fixture.prepare_response("no_proposal");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let record = fixture.wait_for_terminal().await;
    assert_eq!(
        record.state,
        AssessmentState::Succeeded,
        "{:?}",
        record.error_summary
    );
    let report = record.report.unwrap();
    assert_eq!(report.outcome, "no_proposal");
    assert!(report.recommendation_id.is_none());
    assert_eq!(
        report.no_proposal_reason.as_deref(),
        Some("The current workflow is adequately supported by the supplied evidence.")
    );
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Cached,
        "a matching durable report should be reused without another provider dispatch"
    );
    assert_eq!(
        fs::read_to_string(fixture.marker.with_extension("count"))
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_assessment_remains_available_when_background_analysis_is_disabled() {
    let fixture = AssessmentFixture::new();
    let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
    let mut settings = TaskmasterSettings::default();
    settings.enabled = false;
    settings.selection = Some(
        serde_json::from_value(serde_json::json!({
            "provider": "codex",
            "model": "fixture-model"
        }))
        .unwrap(),
    );
    runtime.replace_settings(settings).unwrap();
    fixture.prepare_response("no_proposal");

    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let record = fixture.wait_for_terminal().await;
    assert_eq!(
        record.state,
        AssessmentState::Succeeded,
        "{:?}",
        record.error_summary
    );
    assert_eq!(record.report.unwrap().outcome, "no_proposal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn final_commit_failure_is_reconciled_before_the_analysis_lease_is_released() {
    let fixture = AssessmentFixture::new();
    fixture.prepare_response("proposed");
    let root = fixture.workspace_root();
    // Queued, Running, and PendingRecommendation writes succeed; only the
    // final committed-state write fails once.
    crate::taskmaster::assessment_store::set_save_failures(&root, 3, 1);
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let record = fixture.wait_for_terminal().await;
    crate::taskmaster::assessment_store::set_save_failures(&root, 0, 0);
    assert_eq!(
        record.state,
        AssessmentState::Succeeded,
        "{:?}",
        record.error_summary
    );
    assert!(record.pending_recommendation.is_none());
    let report = record.report.unwrap();
    let recommendation = fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .recommendation_store
        .get(report.recommendation_id.as_ref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(recommendation.id, report.recommendation_id.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repeated_final_commit_failure_keeps_pending_publication_for_later_recovery() {
    let fixture = AssessmentFixture::new();
    fixture.prepare_response("proposed");
    let root = fixture.workspace_root();
    // The final commit and the immediate in-lease reconciliation commit fail.
    crate::taskmaster::assessment_store::set_save_failures(&root, 3, 2);
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let store = AssessmentStore::new(&root);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let pending = loop {
        if let Some(record) = store.load::<AssessmentRecord>().unwrap() {
            if matches!(
                record.state,
                AssessmentState::PendingRecommendation
                    | AssessmentState::Succeeded
                    | AssessmentState::Failed
            ) {
                break record;
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "pending publication was not persisted"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    assert_eq!(pending.state, AssessmentState::PendingRecommendation);
    let expected_recommendation = pending.pending_recommendation.clone().unwrap();
    assert_eq!(
        fixture
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .get(&expected_recommendation.id)
            .unwrap(),
        Some(expected_recommendation.clone()),
        "upsert must have happened before the failed final commit"
    );

    let before_read = fs::read(root.join("assessment.json")).unwrap();
    let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
    let status = crate::taskmaster::assessment_access::status_for(&fixture.state, &runtime);
    assert_eq!(status["state"], "unavailable");
    assert!(status["report"].is_null());
    assert!(!status.to_string().contains("Clarify the workflow guide"));
    let application = crate::session_application::SessionApplication::new(fixture.state.clone());
    let (listed, _) = application.list_recommendations().unwrap();
    assert!(!listed
        .iter()
        .any(|recommendation| recommendation.id == expected_recommendation.id));
    assert!(application
        .get_recommendation(&expected_recommendation.id)
        .unwrap()
        .is_none());
    assert_eq!(
        fs::read(root.join("assessment.json")).unwrap(),
        before_read,
        "read-only status/list/get must not recover or rewrite a pending report"
    );

    crate::taskmaster::assessment_store::set_save_failures(&root, 0, 0);
    recover_workspace_record(&fixture.state, &fixture.runtime_root, &root).unwrap();
    let committed = store.load::<AssessmentRecord>().unwrap().unwrap();
    assert_eq!(committed.state, AssessmentState::Succeeded);
    assert!(committed.pending_recommendation.is_none());
    assert_eq!(
        fixture
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .get(&expected_recommendation.id)
            .unwrap(),
        Some(expected_recommendation)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fact_provider_and_context_changes_before_apply_reject_the_provider_result() {
    for change in ["cited_fact", "uncited_fact", "provider", "context"] {
        let fixture = AssessmentFixture::new();
        fixture.write_response("proposed");
        assert_eq!(
            schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
            AssessmentRequestStatus::Scheduled,
            "{change}"
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !fixture.marker.exists() {
            assert!(
                tokio::time::Instant::now() < deadline,
                "provider did not dispatch"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let workspace = fixture.state.workspace.lock().unwrap();
        let workspace_path = workspace.as_ref().unwrap().path.clone();
        drop(workspace);
        match change {
            "cited_fact" => {
                fs::write(workspace_path.join("README.md"), "Changed cited fact.\n").unwrap()
            }
            "uncited_fact" => fs::write(
                workspace_path.join("OTHER.md"),
                "Changed uncited repository fact.\n",
            )
            .unwrap(),
            "provider" => {
                let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
                let mut settings = runtime.status(Some(&workspace_path)).settings;
                settings.selection.as_mut().unwrap().model = "changed-fixture-model".into();
                runtime.replace_settings(settings).unwrap();
            }
            "context" => {
                let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
                let mut settings = runtime.status(Some(&workspace_path)).settings;
                settings.context_level =
                    crate::taskmaster::runtime::ContextLevel::SessionObservations;
                runtime.replace_settings(settings).unwrap();
            }
            _ => unreachable!(),
        }
        fixture.release_provider();
        let record = fixture.wait_for_terminal().await;
        assert_eq!(
            record.state,
            AssessmentState::Failed,
            "{change}: {:?}",
            record.error_summary
        );
        assert!(
            record.report.is_none(),
            "stale report published for {change}"
        );
        assert!(
            fixture
                .state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .recommendation_store
                .list()
                .unwrap()
                .is_empty(),
            "stale recommendation published for {change}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn changed_assessment_inputs_create_a_provenance_linked_successor() {
    let fixture = AssessmentFixture::new();
    fixture.prepare_response("proposed");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let first = fixture.wait_for_terminal().await;
    assert_eq!(first.state, AssessmentState::Succeeded);
    let old_id = first.report.unwrap().recommendation_id.unwrap();

    let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
    let workspace_path = fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path
        .clone();
    let mut settings = runtime.status(Some(&workspace_path)).settings;
    settings.selection.as_mut().unwrap().model = "successor-fixture-model".into();
    runtime.replace_settings(settings).unwrap();
    fixture.write_response("proposed");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let second = fixture.wait_for_terminal().await;
    assert_eq!(
        second.state,
        AssessmentState::Succeeded,
        "{:?}",
        second.error_summary
    );
    let successor_id = second.report.unwrap().recommendation_id.unwrap();
    assert_ne!(successor_id, old_id);
    let workspace = fixture.state.workspace.lock().unwrap();
    let store = &workspace.as_ref().unwrap().recommendation_store;
    let old = store.get(&old_id).unwrap().unwrap();
    let successor = store.get(&successor_id).unwrap().unwrap();
    assert_eq!(
        old.status,
        crate::taskmaster::RecommendationStatus::Superseded
    );
    assert_eq!(
        successor
            .workflow_improvement
            .supersedes_recommendation_id
            .as_deref(),
        Some(old_id.as_str())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn matching_ordinary_dismissed_recommendation_suppresses_duplicate_proposal() {
    let fixture = AssessmentFixture::new();
    let pending = fixture.pending_record();
    let mut ordinary_duplicate = pending.pending_recommendation.unwrap();
    ordinary_duplicate.assessment = None;
    ordinary_duplicate.status = crate::taskmaster::RecommendationStatus::Dismissed;
    ordinary_duplicate.id = "ordinary-duplicate".into();
    ordinary_duplicate
        .workflow_improvement
        .supersedes_recommendation_id = None;
    fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .recommendation_store
        .put(&ordinary_duplicate)
        .unwrap();
    fixture.prepare_response("proposed");

    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let record = fixture.wait_for_terminal().await;
    assert_eq!(
        record.state,
        AssessmentState::Succeeded,
        "{:?}",
        record.error_summary
    );
    let report = record.report.unwrap();
    assert_eq!(report.outcome, "no_proposal");
    assert!(report.recommendation_id.is_none());
    assert_eq!(
        report.no_proposal_reason.as_deref(),
        Some("A matching recommendation already exists for these facts.")
    );
    assert_eq!(
        fixture
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .list()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn recovery_under_another_instances_lease_preserves_queued_and_running_records() {
    for state in [AssessmentState::Queued, AssessmentState::Running] {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let app = crate::test_support::test_app_state_with_workspace(&workspace);
        let runtime_root = dir.path().join("runtime");
        let runtime = TaskmasterRuntime::open(runtime_root.clone());
        let lease = runtime.try_analysis_lease().unwrap().unwrap();
        let root = app
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path();
        let store = AssessmentStore::new(&root);
        let record = AssessmentRecord {
            format_version: 1,
            id: format!("assessment-{state:?}"),
            state,
            started_at: "2026-10-17T00:00:00Z".into(),
            completed_at: None,
            report: None,
            pending_recommendation: None,
            error_summary: None,
            reused: false,
        };
        store.save(&record).unwrap();
        let before = fs::read(root.join("assessment.json")).unwrap();

        assert!(recover_workspace_record(&app, &runtime_root, &root).is_err());
        assert_eq!(fs::read(root.join("assessment.json")).unwrap(), before);
        drop(lease);
    }
}

#[test]
fn transient_recovery_capture_failure_preserves_the_exact_pending_record() {
    // Recovery must fail before making any durable mutation when context is unavailable.
    let fixture = AssessmentFixture::new();
    let app = fixture.state.clone();
    let runtime_root = fixture.runtime_root.clone();
    let root = fixture.workspace_root();
    let store = AssessmentStore::new(&root);
    let record = fixture.pending_record();
    store.save(&record).unwrap();
    let before = fs::read(root.join("assessment.json")).unwrap();
    let runtime = TaskmasterRuntime::open(runtime_root.clone());
    let knowledge_path = runtime.knowledge_path();
    let valid_proof = fs::read(&knowledge_path).unwrap();
    fs::write(&knowledge_path, b"temporary proof read failure").unwrap();
    assert!(recover_workspace_record(&app, &runtime_root, &root).is_err());
    assert_eq!(fs::read(root.join("assessment.json")).unwrap(), before);
    fs::write(knowledge_path, valid_proof).unwrap();
}

#[test]
fn pending_crash_replay_is_idempotent_before_and_after_recommendation_upsert() {
    for already_upserted in [false, true] {
        let fixture = AssessmentFixture::new();
        let root = fixture.workspace_root();
        let store = AssessmentStore::new(&root);
        let record = fixture.pending_record();
        let recommendation = record.pending_recommendation.as_ref().unwrap().clone();
        store.save(&record).unwrap();
        if already_upserted {
            fixture
                .state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .recommendation_store
                .put(&recommendation)
                .unwrap();
        }

        recover_workspace_record(&fixture.state, &fixture.runtime_root, &root).unwrap();
        let recovered = store.load::<AssessmentRecord>().unwrap().unwrap();
        assert_eq!(recovered.state, AssessmentState::Succeeded);
        assert!(recovered.pending_recommendation.is_none());
        let published = fixture
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .get(&recommendation.id)
            .unwrap()
            .unwrap();
        assert_eq!(published, recommendation);
        let committed_bytes = fs::read(root.join("assessment.json")).unwrap();
        recover_workspace_record(&fixture.state, &fixture.runtime_root, &root).unwrap();
        assert_eq!(
            fs::read(root.join("assessment.json")).unwrap(),
            committed_bytes
        );
    }
}
