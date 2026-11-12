use super::Manifest;
use crate::taskmaster::{assessment_context::improvement::RangeIdentity, runtime::ContextLevel};

#[test]
fn manifest_rejects_oversized_ranges_without_summing_untrusted_sizes() {
    let manifest = Manifest {
        purpose: "brain-improvement-v1".into(),
        workspace_path: "/workspace".into(),
        workspace_instance: 1,
        context_level: ContextLevel::SourceCode,
        excluded_paths: Vec::new(),
        evaluation_key: "0".repeat(64),
        rubric_digest: "0".repeat(64),
        catalog_digest: "0".repeat(64),
        ranges: [0, 1]
            .map(|index| RangeIdentity {
                path: format!("src/{index}.rs"),
                offset: 0,
                bytes: usize::MAX,
                file_size: u64::MAX,
                sha256: "0".repeat(64),
            })
            .into(),
        pages: Vec::new(),
        recommendation_digest: "0".repeat(64),
    };

    assert!(manifest.validate().is_err());
}

#[test]
fn public_report_projects_row_fact_ids_and_range_completeness() {
    use super::{PageIdentity, Report};
    use crate::taskmaster::assessment_context::improvement::RangeFact;
    let mut fact = RangeFact {
        path: "src/a.rs".into(),
        offset: 0,
        file_size: 4,
        sha256: String::new(),
        text: "abcd".into(),
    };
    fact.sha256 = fact.identity().unwrap();
    let manifest = Manifest {
        purpose: "brain-improvement-v1".into(),
        workspace_path: "/workspace".into(),
        workspace_instance: 1,
        context_level: ContextLevel::SourceCode,
        excluded_paths: vec![],
        evaluation_key: "0".repeat(64),
        rubric_digest: "0".repeat(64),
        catalog_digest: "0".repeat(64),
        ranges: vec![(&fact).into()],
        pages: vec![PageIdentity {
            id: "rubric".into(),
            sha256: "0".repeat(64),
        }],
        recommendation_digest: "0".repeat(64),
    };
    let result = serde_json::from_value(serde_json::json!({"version":1,"summary":"summary","concepts":[{"conceptId":"x","maturity":2,"confidence":"medium","repositoryFactIds":[0],"rubricPageId":"rubric","rationale":"reason","limits":[],"nextTransition":"next","unassessedReason":null}],"opportunities":[],"preferredOpportunityId":null,"noProposalReason":"none","limitations":[]})).unwrap();
    let report = Report {
        manifest,
        result,
        repository_evidence: vec![fact.clone()],
        knowledge_evidence: vec![],
        recommendation_id: None,
        history_snapshot_id: None,
        history_error: None,
        publication_limitation: None,
    };
    let public = serde_json::to_value(report.public().unwrap()).unwrap();
    assert_eq!(
        public["concepts"][0]["repositoryFactHashes"],
        serde_json::json!([fact.sha256])
    );
    assert!(public["concepts"][0].get("repositoryFactIds").is_none());
    assert_eq!(public["repositoryEvidence"][0]["complete"], true);
    assert!(public.get("manifest").is_none());
}

#[test]
fn pending_publication_retries_same_recommendation_after_interrupted_commit() {
    use super::{publish_with, recover_with, Record, RunState};
    let (_, recommendation) = crate::taskmaster::assessment_report::tests::proposal();
    let mut record = Record::queued("run-1", "start");
    record.pending_recommendation = Some(recommendation.clone());
    let mut durable = None;
    let result = publish_with(
        &mut record,
        |record| {
            durable = Some(record.clone());
            Ok(())
        },
        |_| Err("temporary upsert failure".into()),
    );
    assert!(result.is_err());
    let mut recovered = durable.unwrap();
    assert_eq!(recovered.state, RunState::PendingRecommendation);
    let mut writes = Vec::new();
    recover_with(
        &mut recovered,
        |record| {
            writes.push(record.state.clone());
            Ok(())
        },
        |value| {
            assert_eq!(value, &recommendation);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(recovered.state, RunState::Succeeded);
    assert!(recovered.pending_recommendation.is_none());
    assert_eq!(writes, vec![RunState::Succeeded]);
    let mut interrupted = Record::queued("run-2", "start");
    interrupted.pending_recommendation = Some(recommendation);
    let result = publish_with(
        &mut interrupted,
        |value| {
            if value.state == RunState::Succeeded {
                Err("commit failed".into())
            } else {
                Ok(())
            }
        },
        |_| Ok(()),
    );
    assert!(result.is_err());
    assert_eq!(interrupted.state, RunState::PendingRecommendation);
    assert!(interrupted.pending_recommendation.is_some());
}

#[test]
fn persisted_errors_cannot_expose_provider_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    let store = super::ReportStore::new(dir.path());
    let mut record = super::Record::queued("run-1", "now");
    record.state = super::RunState::Failed;
    record.phase = None;
    record.completed_at = Some("now".into());
    record.error_code = Some("provider said private diagnostic".into());
    assert!(store.save(&record).is_err());
    record.error_code = Some("investigation_failed".into());
    assert!(store.save(&record).is_ok());
}
