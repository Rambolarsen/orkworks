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
        let driver = dir.path().join("driver.py");
        fs::write(&driver, r#"import json,pathlib,sys
response, prompt_path = map(pathlib.Path, sys.argv[1:])
prompt = json.loads(prompt_path.read_text())
args = prompt_path.with_name('provider-args').read_text().splitlines()
stage = 'discovery' if prompt.get('protocol') == 'taskmaster-discovery-v1' else 'final'
schema_path = pathlib.Path(args[args.index('--output-schema') + 1]) if '--output-schema' in args else None
response.with_suffix('.' + stage + '-schema').write_text(schema_path.read_text() if schema_path else 'null')
response.with_suffix('.' + stage + '-schema-path').write_text(str(schema_path) if schema_path else '')
response.with_suffix('.' + stage + '-constraint').write_text(json.dumps(prompt.get('nativeResponseConstraint')))
if prompt.get('protocol') != 'taskmaster-discovery-v1' and response.with_suffix('.block-final').exists():
    import time
    response.with_suffix('.final-started').write_text('started')
    while not response.with_suffix('.final-release').exists(): time.sleep(0.01)
if prompt.get('protocol') == 'taskmaster-discovery-v1':
    override = response.with_suffix('.discovery')
    if override.exists():
        output = override.read_text()
    else:
        page = next(p for p in prompt['knowledgePages'] if p['type'] == 'playbook')
        output = json.dumps({'version':1,'outcome':'inspect','candidate':{'targetSurface':'documentation','title':'Clarify workflow evidence','hypothesis':'The current workflow guide may need more specific validation guidance.'},'repositoryFactHashes':[prompt['repositoryFacts'][0]['sha256']],'knowledgePageIds':[page['id']],'requestedFileIds':[],'reason':None})
else:
    output = response.read_text()
print(json.dumps({'type':'thread.started'}))
print(json.dumps({'type':'turn.started'}))
print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':output}}))
print(json.dumps({'type':'turn.completed'}))
"#).unwrap();
        let args_file = dir.path().join("provider-args").display().to_string();
        let reject_schema = response
            .with_extension("reject-schema")
            .display()
            .to_string();
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'codex-cli 0.153.4\\n'; exit 0; fi\nprintf '%s\\n' \"$@\" > '{args_file}'\ncat > '{}'\nprintf dispatched > '{}'\nprintf x >> '{}.count'\nwhile [ ! -f '{}.release' ]; do sleep 0.01; done\nif [ -f '{reject_schema}' ]; then for arg in \"$@\"; do if [ \"$arg\" = '--output-schema' ]; then printf 'unsupported response schema' >&2; exit 2; fi; done; fi\npython3 '{}' '{}' '{}' 2> '{}.stderr' | tee '{}'\n",
            dir.path().join("provider-prompt").display(), marker.display(), marker.display(), marker.display(),
            driver.display(), response.display(), dir.path().join("provider-prompt").display(),
            dir.path().join("provider-output").display(), dir.path().join("provider-output").display(),
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
                "unverifiedChecks": ["Documentation verification remains unverified until implementation."],
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
        let mut snapshot = capture_current_snapshot(&self.state, &self.runtime_root, None).unwrap();
        let mut repository = snapshot.repository.clone();
        repository.bind_discovery("fixture discovery identity");
        let input = build_workspace_input(
            &snapshot.workspace_path,
            &snapshot.evaluation,
            snapshot.observations.clone(),
            snapshot.recommendations.clone(),
            repository,
        )
        .unwrap();
        snapshot.repository = input.repository;
        snapshot.fingerprint = input.fingerprint;
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
            0,
            "2026-10-17T00:00:00Z",
        );
        AssessmentRecord {
            format_version: 2,
            input_manifest: Some(snapshot.repository.manifest()),
            phase: None,
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
                    AssessmentState::Succeeded
                        | AssessmentState::Failed
                        | AssessmentState::Interrupted
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
    let reloaded = AssessmentStore::new(&fixture.workspace_root())
        .load::<AssessmentRecord>()
        .unwrap()
        .unwrap();
    assert!(reloaded.reused, "cache reuse must survive a report reload");
    let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
    assert_eq!(
        crate::taskmaster::assessment_access::status_for(&fixture.state, &runtime)["reused"],
        true
    );

    assert_eq!(
        fs::read_to_string(fixture.marker.with_extension("count"))
            .unwrap()
            .len(),
        2
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
    // Queued, Running, ReadingEvidence, Assessing, and Pending writes succeed; only the
    // final committed-state write fails once.
    crate::taskmaster::assessment_store::set_save_failures(&root, 5, 1);
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
    crate::taskmaster::assessment_store::set_save_failures(&root, 5, 2);
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
    for change in [
        "cited_fact",
        "uncited_fact",
        "provider",
        "context",
        "workspace",
    ] {
        let fixture = AssessmentFixture::new();
        fixture.write_response("proposed");
        let original_root = fixture.workspace_root();
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
            "workspace" => {
                let next = fixture.dir.path().join("other-workspace");
                fs::create_dir(&next).unwrap();
                crate::test_support::swap_workspace(&fixture.state, &next);
            }
            _ => unreachable!(),
        }
        fixture.release_provider();
        let record = if change == "workspace" {
            let store = AssessmentStore::new(&original_root);
            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            loop {
                let record = store.load::<AssessmentRecord>().unwrap().unwrap();
                if record.state == AssessmentState::Failed {
                    break record;
                }
                assert!(tokio::time::Instant::now() < deadline);
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        } else {
            fixture.wait_for_terminal().await
        };
        assert_eq!(
            fs::read_to_string(fixture.marker.with_extension("count"))
                .unwrap()
                .len(),
            1,
            "stale second dispatch for {change}"
        );
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
    assert_eq!(
        first.state,
        AssessmentState::Succeeded,
        "{:?}",
        first.error_summary
    );
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
            input_manifest: None,
            phase: None,
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn superseded_proposal_remains_eligible_after_an_intervening_no_proposal() {
    let fixture = AssessmentFixture::new();
    fixture.prepare_response("proposed");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let first = fixture.wait_for_terminal().await;
    assert_eq!(
        first.state,
        AssessmentState::Succeeded,
        "{:?}",
        first.error_summary
    );
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
    fixture.write_response("no_proposal");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let intermediate = fixture.wait_for_terminal().await;
    assert_eq!(intermediate.report.unwrap().outcome, "no_proposal");
    let mut settings = runtime.status(Some(&workspace_path)).settings;
    settings.selection.as_mut().unwrap().model = "after-intervening-assessment".into();
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
async fn superseded_proposal_remains_eligible_after_an_intervening_failure() {
    let fixture = AssessmentFixture::new();
    fixture.prepare_response("proposed");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let first = fixture.wait_for_terminal().await;
    assert_eq!(
        first.state,
        AssessmentState::Succeeded,
        "{:?}",
        first.error_summary
    );
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
    fs::write(&fixture.response, "invalid provider output").unwrap();
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let intermediate = fixture.wait_for_terminal().await;
    assert_eq!(intermediate.state, AssessmentState::Failed);
    let mut settings = runtime.status(Some(&workspace_path)).settings;
    settings.selection.as_mut().unwrap().model = "after-intervening-assessment".into();
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
async fn successive_input_invalidations_extend_the_unique_assessment_chain() {
    let fixture = AssessmentFixture::new();
    fixture.prepare_response("proposed");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let first = fixture.wait_for_terminal().await;
    assert_eq!(
        first.state,
        AssessmentState::Succeeded,
        "{:?}",
        first.error_summary
    );
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
    let mut settings = runtime.status(Some(&workspace_path)).settings;
    settings.selection.as_mut().unwrap().model = "third-fixture-model".into();
    runtime.replace_settings(settings).unwrap();
    fixture.write_response("proposed");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let third = fixture.wait_for_terminal().await;
    assert_eq!(third.state, AssessmentState::Succeeded);
    let third_id = third
        .report
        .unwrap()
        .recommendation_id
        .expect("latest invalidated predecessor permits a successor");
    let workspace = fixture.state.workspace.lock().unwrap();
    let store = &workspace.as_ref().unwrap().recommendation_store;
    let third = store.get(&third_id).unwrap().unwrap();
    assert_eq!(store.get(&old_id).unwrap().unwrap().chain_depth, 0);
    assert_eq!(store.get(&successor_id).unwrap().unwrap().chain_depth, 1);
    assert_eq!(third.chain_depth, 2);
    assert_eq!(
        third
            .workflow_improvement
            .supersedes_recommendation_id
            .as_deref(),
        Some(successor_id.as_str())
    );
    assert_ne!(third_id, old_id);
}

#[test]
fn recovery_reconciliation_serializes_with_lifecycle_and_a_second_access_reduction() {
    use crate::taskmaster::runtime::ContextLevel;
    use crate::taskmaster::{RecommendationStatus, RepositoryEvidence};
    let directory = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(directory.path());
    let (workspace_path, metadata_root) = {
        let workspace = state.workspace.lock().unwrap();
        let workspace = workspace.as_ref().unwrap();
        (workspace.path.clone(), workspace.metadata.root_path())
    };
    let runtime_root = directory.path().join("policy-runtime");
    let runtime = Arc::new(TaskmasterRuntime::open(runtime_root));
    let (_, mut recommendation) = crate::taskmaster::assessment_report::tests::proposal();
    recommendation.status = RecommendationStatus::Accepted;
    recommendation.target_session_id = Some("delivered-target".into());
    recommendation.assessment.as_mut().unwrap().context_level = ContextLevel::SourceCode;
    recommendation.repository_evidence.push(RepositoryEvidence {
        path: "src/private.rs".into(),
        sha256: "source-hash".into(),
        excerpt: "private source".into(),
        observed_at: "then".into(),
    });
    let held_workspace = state.workspace.lock().unwrap();
    let store = &held_workspace.as_ref().unwrap().recommendation_store;
    store.put(&recommendation).unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let first_state = state.clone();
    let first_runtime = runtime.clone();
    let first_path = workspace_path.clone();
    let first_root = metadata_root.clone();
    let first = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        let result =
            reconcile_current_workspace(&first_state, &first_runtime, &first_path, &first_root);
        done_tx.send(result).unwrap();
    });
    started_rx.recv().unwrap();
    let finished_while_locked = done_rx.recv_timeout(Duration::from_millis(150)).ok();
    // A delivered lifecycle change under the same lock must not be replaced
    // by a reconciliation clone captured before it.
    store
        .complete_accepted(&recommendation.id, "completed-at".into())
        .unwrap();
    let second_state = state.clone();
    let second_runtime = runtime.clone();
    let second = std::thread::spawn(move || {
        second_runtime.replace_settings_with_reconcile(
            TaskmasterSettings {
                context_level: ContextLevel::SessionObservations,
                ..TaskmasterSettings::default()
            },
            |_, current| {
                crate::taskmaster::assessment_access::reconcile_settings(&second_state, current)
            },
        )
    });
    drop(held_workspace);
    first.join().unwrap();
    second.join().unwrap().unwrap();
    assert!(
        finished_while_locked.is_none(),
        "recovery must wait for the active workspace lock before reading or replacing metadata"
    );
    done_rx.recv().unwrap().unwrap();
    let final_record = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .recommendation_store
        .get(&recommendation.id)
        .unwrap()
        .unwrap();
    assert_eq!(final_record.status, RecommendationStatus::Completed);
    assert!(
        final_record.repository_evidence.is_empty(),
        "the second narrower policy must remain durable"
    );
    assert!(final_record.assessment.unwrap().access_redacted);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovery_no_candidate_uses_one_call_and_malformed_selection_never_retries() {
    for malformed in [false, true] {
        let fixture = AssessmentFixture::new();
        fixture.prepare_response("proposed");
        let mut discovery = serde_json::json!({"version":1,"outcome":"no_candidate","candidate":null,"repositoryFactHashes":[],"knowledgePageIds":[],"requestedFileIds":[],"reason":"Current repository evidence identifies no justified workflow change."});
        if malformed {
            discovery["requestedFileIds"] = serde_json::json!([99999]);
        }
        fs::write(
            fixture.response.with_extension("discovery"),
            discovery.to_string(),
        )
        .unwrap();
        assert_eq!(
            schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
            AssessmentRequestStatus::Scheduled
        );
        let record = fixture.wait_for_terminal().await;
        assert_eq!(
            record.state,
            if malformed {
                AssessmentState::Failed
            } else {
                AssessmentState::Succeeded
            }
        );
        assert_eq!(
            fs::read_to_string(fixture.marker.with_extension("count"))
                .unwrap()
                .len(),
            1
        );
        assert!(fixture
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .list()
            .unwrap()
            .is_empty());
        if !malformed {
            assert_eq!(record.report.unwrap().outcome, "no_proposal");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observation_only_access_dispatches_zero_calls() {
    let fixture = AssessmentFixture::new();
    let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
    let mut settings = runtime.status(None).settings;
    settings.context_level = crate::taskmaster::runtime::ContextLevel::SessionObservations;
    runtime.replace_settings(settings).unwrap();
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Unavailable
    );
    assert!(!fixture.marker.exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn catalog_change_after_publication_fences_uncited_inputs_and_allows_new_assessment() {
    let fixture = AssessmentFixture::new();
    fixture.prepare_response("proposed");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let record = fixture.wait_for_terminal().await;
    assert_eq!(record.state, AssessmentState::Succeeded);
    let id = record.report.unwrap().recommendation_id.unwrap();
    let workspace = fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path
        .clone();
    fs::write(
        workspace.join("UNCITED.md"),
        "New current workflow guidance.",
    )
    .unwrap();
    let application = crate::session_application::SessionApplication::new(fixture.state.clone());
    assert!(application.get_recommendation(&id).unwrap().is_none());
    let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
    assert_eq!(
        crate::taskmaster::assessment_access::status_for(&fixture.state, &runtime)["state"],
        "unavailable"
    );
    fixture.write_response("no_proposal");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    assert_eq!(
        fixture.wait_for_terminal().await.state,
        AssessmentState::Succeeded
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn legacy_accepted_assessments_block_admission_without_mutating_their_state() {
    let fixture = AssessmentFixture::new();
    let mut legacy = fixture.pending_record();
    legacy.format_version = 1;
    legacy.input_manifest = None;
    legacy.state = AssessmentState::Succeeded;
    let mut recommendation = legacy.pending_recommendation.take().unwrap();
    recommendation.status = crate::taskmaster::RecommendationStatus::Accepted;
    recommendation.target_session_id = Some("legacy-session".into());
    let before = recommendation.clone();
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
    AssessmentStore::new(&fixture.workspace_root())
        .save(&legacy)
        .unwrap();
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::ActiveRecommendation
    );
    assert!(!fixture.marker.exists());
    assert_eq!(
        fixture
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .get(&before.id)
            .unwrap()
            .unwrap(),
        before
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn current_accepted_or_executing_assessments_block_admission_after_input_changes() {
    for status in [
        crate::taskmaster::RecommendationStatus::Accepted,
        crate::taskmaster::RecommendationStatus::Executing,
    ] {
        let fixture = AssessmentFixture::new();
        let mut record = fixture.pending_record();
        record.state = AssessmentState::Succeeded;
        let mut recommendation = record.pending_recommendation.take().unwrap();
        recommendation.status = status;
        recommendation.target_session_id = Some("protected-session".into());
        let before = recommendation.clone();
        let workspace_path = {
            let workspace = fixture.state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            workspace.recommendation_store.put(&recommendation).unwrap();
            workspace.path.clone()
        };
        let store = AssessmentStore::new(&fixture.workspace_root());
        store.save(&record).unwrap();
        fs::write(
            workspace_path.join("UNCITED.md"),
            "Changed repository workflow input.",
        )
        .unwrap();
        assert_eq!(
            schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
            AssessmentRequestStatus::ActiveRecommendation
        );
        assert!(!fixture.marker.exists());
        assert_eq!(store.load::<AssessmentRecord>().unwrap().unwrap(), record);
        assert_eq!(
            fixture
                .state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .recommendation_store
                .get(&before.id)
                .unwrap()
                .unwrap(),
            before
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_uncited_followup_reaches_final_prompt_and_fences_consumers_after_change() {
    let fixture = AssessmentFixture::new();
    let workspace = fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path
        .clone();
    fs::create_dir(workspace.join("src")).unwrap();
    fs::write(
        workspace.join("src/coverage.ts"),
        "export const uncitedCoverageMarker = true;",
    )
    .unwrap();
    let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
    let mut settings = runtime.status(None).settings;
    settings.context_level = crate::taskmaster::runtime::ContextLevel::SourceCode;
    runtime.replace_settings(settings).unwrap();
    fixture.prepare_response("proposed");
    let snapshot = capture_current_snapshot(&fixture.state, &fixture.runtime_root, None).unwrap();
    let id = snapshot
        .repository
        .catalog
        .iter()
        .position(|p| p == "src/coverage.ts")
        .unwrap();
    let page = snapshot.prompt_snapshot.knowledge.as_ref().unwrap().pages[0]
        .id
        .clone();
    fs::write(fixture.response.with_extension("discovery"), serde_json::json!({"version":1,"outcome":"inspect","candidate":{"targetSurface":"test","title":"Check unit coverage","hypothesis":"The current unit coverage may need a bounded improvement."},"repositoryFactHashes":[snapshot.facts[0].sha256],"knowledgePageIds":[page],"requestedFileIds":[id],"reason":null}).to_string()).unwrap();
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
    assert_eq!(
        fs::read_to_string(fixture.marker.with_extension("count"))
            .unwrap()
            .len(),
        2
    );
    assert!(
        fs::read_to_string(fixture.dir.path().join("provider-prompt"))
            .unwrap()
            .contains("uncitedCoverageMarker")
    );
    assert!(!serde_json::to_string(&record)
        .unwrap()
        .contains("uncitedCoverageMarker"));
    assert!(record
        .input_manifest
        .as_ref()
        .unwrap()
        .files
        .iter()
        .any(|f| !f.seed && f.path == "src/coverage.ts"));
    let id = record.report.unwrap().recommendation_id.unwrap();
    fs::write(
        workspace.join("src/coverage.ts"),
        "export const changedUncitedCoverage = false;",
    )
    .unwrap();
    let app = crate::session_application::SessionApplication::new(fixture.state.clone());
    assert!(app.get_recommendation(&id).unwrap().is_none());
    assert_eq!(
        crate::taskmaster::assessment_access::status_for(&fixture.state, &runtime)["state"],
        "unavailable"
    );
}

#[test]
fn pending_recovery_after_narrowing_never_restores_source_manifest_paths_or_hashes() {
    let fixture = AssessmentFixture::new();
    let workspace = fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path
        .clone();
    fs::create_dir(workspace.join("src")).unwrap();
    fs::write(
        workspace.join("src/coverage.ts"),
        "export const coverageMarker = true;",
    )
    .unwrap();
    let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
    let mut settings = runtime.status(None).settings;
    settings.context_level = crate::taskmaster::runtime::ContextLevel::SourceCode;
    runtime.replace_settings(settings.clone()).unwrap();
    let snapshot = capture_current_snapshot(&fixture.state, &fixture.runtime_root, None).unwrap();
    let mut context = snapshot.repository.clone();
    let id = context
        .catalog
        .iter()
        .position(|p| p == "src/coverage.ts")
        .unwrap();
    context.follow_up(&[id], "coverage").unwrap();
    context.bind_discovery("validated discovery identity");
    let input = build_workspace_input(
        &workspace,
        &snapshot.evaluation,
        snapshot.observations,
        snapshot.recommendations,
        context,
    )
    .unwrap();
    let source_hash = input
        .repository
        .manifest()
        .files
        .iter()
        .find(|f| !f.seed)
        .unwrap()
        .sha256
        .clone();
    let mut record = fixture.pending_record();
    record.input_manifest = Some(input.repository.manifest());
    record.report.as_mut().unwrap().input_fingerprint = input.fingerprint.clone();
    record
        .pending_recommendation
        .as_mut()
        .unwrap()
        .assessment
        .as_mut()
        .unwrap()
        .input_fingerprint = input.fingerprint;
    let root = fixture.workspace_root();
    let store = AssessmentStore::new(&root);
    store.save(&record).unwrap();
    settings.context_level = crate::taskmaster::runtime::ContextLevel::WorkflowContext;
    runtime.replace_settings(settings).unwrap();
    recover_workspace_record(&fixture.state, &fixture.runtime_root, &root).unwrap();
    let saved = fs::read_to_string(root.join("assessment.json")).unwrap();
    assert!(
        !saved.contains("src/coverage.ts"),
        "recovery restored a disallowed path"
    );
    assert!(
        !saved.contains(&source_hash),
        "recovery restored a disallowed hash"
    );
    assert_eq!(
        store.load::<AssessmentRecord>().unwrap().unwrap().state,
        AssessmentState::Interrupted
    );
    assert!(!fixture.marker.exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn changes_during_final_call_prevent_publication_without_retry() {
    for change in ["file", "settings", "workspace"] {
        let fixture = AssessmentFixture::new();
        fixture.prepare_response("proposed");
        fs::write(fixture.response.with_extension("block-final"), "hold").unwrap();
        let root = fixture.workspace_root();
        assert_eq!(
            schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
            AssessmentRequestStatus::Scheduled
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while !fixture.response.with_extension("final-started").exists() {
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        match change {
            "file" => {
                let path = fixture
                    .state
                    .workspace
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .path
                    .clone();
                fs::write(path.join("README.md"), "changed final evidence").unwrap();
            }
            "settings" => {
                let runtime = TaskmasterRuntime::open(fixture.runtime_root.clone());
                let mut settings = runtime.status(None).settings;
                settings.selection.as_mut().unwrap().model = "changed-model".into();
                runtime.replace_settings(settings).unwrap();
            }
            "workspace" => {
                let next = fixture.dir.path().join("other-workspace");
                fs::create_dir(&next).unwrap();
                crate::test_support::swap_workspace(&fixture.state, &next);
            }
            _ => unreachable!(),
        }
        fs::write(fixture.response.with_extension("final-release"), "release").unwrap();
        let store = AssessmentStore::new(&root);
        loop {
            let record = store.load::<AssessmentRecord>().unwrap().unwrap();
            if record.state == AssessmentState::Failed {
                assert!(record.report.is_none());
                break;
            }
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(
            fs::read_to_string(fixture.marker.with_extension("count"))
                .unwrap()
                .len(),
            2,
            "unexpected retry for {change}"
        );
    }
}

#[test]
fn format_two_queued_and_running_restart_never_replays_discovery() {
    let fixture = AssessmentFixture::new();
    let root = fixture.workspace_root();
    let store = AssessmentStore::new(&root);
    for state in [AssessmentState::Queued, AssessmentState::Running] {
        let mut record = fixture.pending_record();
        record.state = state;
        record.report = None;
        record.pending_recommendation = None;
        record.phase = Some(crate::taskmaster::assessment_report::AssessmentPhase::ReadingEvidence);
        store.save(&record).unwrap();
        recover_workspace_record(&fixture.state, &fixture.runtime_root, &root).unwrap();
        let saved = store.load::<AssessmentRecord>().unwrap().unwrap();
        assert_eq!(saved.state, AssessmentState::Interrupted);
        assert!(saved.report.is_none());
        assert!(saved.input_manifest.is_none());
        assert!(!fixture.marker.exists());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partial_ancestor_or_instruction_only_budget_overflow_publishes_no_proposal() {
    for partial in [false, true] {
        let fixture = AssessmentFixture::new();
        let workspace = fixture
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .path
            .clone();
        if partial {
            for n in 0..2005 {
                fs::write(
                    workspace.join(format!("bounded-{n:04}.md")),
                    "bounded workflow evidence",
                )
                .unwrap();
            }
            fs::write(fixture.response.with_extension("discovery"), serde_json::json!({"version":1,"outcome":"no_candidate","candidate":null,"repositoryFactHashes":[],"knowledgePageIds":[],"requestedFileIds":[],"reason":"The supplied repository coverage is insufficient for a justified candidate."}).to_string()).unwrap();
            fixture.release_provider();
        } else {
            fs::write(workspace.join("AGENTS.md"), "Root guidance. ".repeat(700)).unwrap();
            fs::write(workspace.join("CLAUDE.md"), "Root constraint. ".repeat(700)).unwrap();
            fs::create_dir(workspace.join("scoped")).unwrap();
            fs::write(
                workspace.join("scoped/AGENTS.md"),
                "Scoped guidance. ".repeat(700),
            )
            .unwrap();
            fs::write(
                workspace.join("scoped/CLAUDE.md"),
                "Scoped constraint. ".repeat(700),
            )
            .unwrap();
            fs::write(workspace.join("scoped/README.md"), "Small selected target").unwrap();
            fixture.prepare_response("proposed");
        }
        let snapshot =
            capture_current_snapshot(&fixture.state, &fixture.runtime_root, None).unwrap();
        assert!(!snapshot.repository.manifest().instruction_coverage_complete);
        assert!(
            snapshot
                .facts
                .iter()
                .all(|f| !f.path.starts_with("scoped/")),
            "no target without its full governing closure"
        );
        assert_eq!(
            schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
            AssessmentRequestStatus::Scheduled
        );
        let record = fixture.wait_for_terminal().await;
        assert_eq!(
            record.state,
            AssessmentState::Succeeded,
            "partial={partial}: {:?}",
            record.error_summary
        );
        let report = record.report.unwrap();
        assert_eq!(report.outcome, "no_proposal");
        assert!(report
            .no_proposal_reason
            .unwrap()
            .contains("governing-instruction closure"));
        assert!(report.recommendation_id.is_none());
        assert_eq!(
            fs::read_to_string(fixture.marker.with_extension("count"))
                .unwrap()
                .len(),
            1
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_assessment_binds_static_stage_shapes_and_contract_identity() {
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
    for (stage, property) in [("discovery", "candidate"), ("final", "proposal")] {
        let schema: serde_json::Value = serde_json::from_slice(
            &fs::read(fixture.response.with_extension(format!("{stage}-schema"))).unwrap(),
        )
        .unwrap();
        assert!(
            schema["properties"][property].is_object(),
            "missing {stage} native constraint"
        );
        let path = fs::read_to_string(
            fixture
                .response
                .with_extension(format!("{stage}-schema-path")),
        )
        .unwrap();
        assert!(
            !std::path::Path::new(&path).exists(),
            "schema retained after process cleanup"
        );
        let marker: serde_json::Value = serde_json::from_slice(
            &fs::read(
                fixture
                    .response
                    .with_extension(format!("{stage}-constraint")),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(marker, "taskmaster-response-shapes-v1");
    }
    let snapshot = capture_current_snapshot(&fixture.state, &fixture.runtime_root, None).unwrap();
    let mut legacy_prompt: serde_json::Value = serde_json::from_str(&snapshot.prompt).unwrap();
    assert!(legacy_prompt
        .as_object_mut()
        .unwrap()
        .remove("nativeResponseConstraint")
        .is_some());
    assert_ne!(
        snapshot.evaluation.cache_key(&snapshot.prompt).unwrap(),
        snapshot
            .evaluation
            .cache_key(&legacy_prompt.to_string())
            .unwrap(),
        "the constraint contract must participate in reconstructed input identity"
    );
    assert_eq!(record.format_version, 2);
    assert_eq!(
        record.input_manifest.unwrap().prompt_version,
        "taskmaster-assessment-v2"
    );
    assert_eq!(
        fs::read_to_string(fixture.marker.with_extension("count"))
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_native_response_schema_fails_once_without_unconstrained_retry() {
    let fixture = AssessmentFixture::new();
    fs::write(fixture.response.with_extension("reject-schema"), "reject").unwrap();
    fixture.prepare_response("no_proposal");
    assert_eq!(
        schedule(fixture.state.clone(), fixture.runtime_root.clone()).status,
        AssessmentRequestStatus::Scheduled
    );
    let record = fixture.wait_for_terminal().await;
    assert_eq!(
        record.state,
        AssessmentState::Failed,
        "schema failure did not stop inference"
    );
    assert!(record.report.is_none());
    assert!(record.pending_recommendation.is_none());
    assert_eq!(
        fs::read_to_string(fixture.marker.with_extension("count"))
            .unwrap()
            .len(),
        1
    );
}
