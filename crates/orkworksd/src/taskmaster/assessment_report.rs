//! Assessment report state and the recoverable recommendation publication boundary.
use super::runtime::ContextLevel;
use super::{KnowledgeEvidence, Recommendation, RepositoryEvidence, TargetSurface};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AssessmentState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
    PendingRecommendation,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssessmentProposal {
    pub target_surface: TargetSurface,
    pub title: String,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssessmentReport {
    pub id: String,
    pub workspace_path: String,
    pub workspace_instance: u64,
    pub observed_at: String,
    pub provider: String,
    pub model: String,
    pub bundle_version: String,
    pub knowledge_identity: serde_json::Value,
    pub input_fingerprint: String,
    pub context_level: ContextLevel,
    pub excluded_paths: Vec<String>,
    pub outcome: String,
    pub summary: String,
    pub proposal: Option<AssessmentProposal>,
    pub no_proposal_reason: Option<String>,
    pub unverified_checks: Vec<String>,
    pub repository_evidence: Vec<RepositoryEvidence>,
    pub knowledge_evidence: Vec<KnowledgeEvidence>,
    pub recommendation_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssessmentRecord {
    pub format_version: u8,
    pub id: String,
    pub state: AssessmentState,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub report: Option<AssessmentReport>,
    pub pending_recommendation: Option<Recommendation>,
    pub error_summary: Option<String>,
    pub reused: bool,
}

/// Stages the complete capped report before the recommendation, then commits it.
/// The caller owns the shared analysis lease and current-context write guards.
pub(crate) fn publish_with(
    record: &mut AssessmentRecord,
    mut save: impl FnMut(&AssessmentRecord) -> Result<(), String>,
    upsert: impl FnMut(&Recommendation) -> Result<(), String>,
) -> Result<(), String> {
    validate_publication(record)?;
    if record.pending_recommendation.is_some() {
        let mut pending = record.clone();
        pending.state = AssessmentState::PendingRecommendation;
        save(&pending)?;
        *record = pending;
        recover_with(record, save, upsert)
    } else {
        let mut committed = record.clone();
        committed.state = AssessmentState::Succeeded;
        save(&committed)?;
        *record = committed;
        Ok(())
    }
}

/// Idempotently replays only a validated pending publication.
pub(crate) fn recover_with(
    record: &mut AssessmentRecord,
    mut save: impl FnMut(&AssessmentRecord) -> Result<(), String>,
    mut upsert: impl FnMut(&Recommendation) -> Result<(), String>,
) -> Result<(), String> {
    if record.state != AssessmentState::PendingRecommendation {
        if record.pending_recommendation.is_some() {
            return Err("Assessment publication state is invalid".into());
        }
        return Ok(());
    }
    validate_publication(record)?;
    let recommendation = record
        .pending_recommendation
        .as_ref()
        .ok_or("Assessment pending recommendation is missing")?;
    upsert(recommendation)?;
    let mut committed = record.clone();
    committed.pending_recommendation = None;
    committed.state = AssessmentState::Succeeded;
    save(&committed)?;
    *record = committed;
    Ok(())
}

fn validate_publication(record: &AssessmentRecord) -> Result<(), String> {
    use super::{RecommendationStatus, RecommendationType};
    let report = record
        .report
        .as_ref()
        .ok_or("Assessment publication report is missing")?;
    if record.format_version != 1 || record.id != report.id {
        return Err("Assessment publication identity is invalid".into());
    }
    if let Some(recommendation) = &record.pending_recommendation {
        let provenance = recommendation
            .assessment
            .as_ref()
            .ok_or("Assessment recommendation provenance is missing")?;
        if report.outcome != "proposed"
            || report.proposal.is_none()
            || report.recommendation_id.as_deref() != Some(recommendation.id.as_str())
            || provenance.id != record.id
            || provenance.input_fingerprint != report.input_fingerprint
            || provenance.context_level != report.context_level
            || provenance.excluded_paths != report.excluded_paths
            || recommendation.recommendation_type != RecommendationType::ImproveWorkflow
            || recommendation.status != RecommendationStatus::Proposed
            || !recommendation.dedupe_key.starts_with("proactive:v1:")
        {
            return Err("Assessment pending recommendation identity is invalid".into());
        }
    } else if report.recommendation_id.is_some() {
        return Err("Assessment publication recommendation is missing".into());
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::taskmaster::{
        AssessmentProvenance, RecommendationConfidence, RecommendationStatus, RecommendationType,
        WorkflowImprovement,
    };
    use crate::workflow_observations::Impact;
    use std::cell::RefCell;

    pub(crate) fn proposal() -> (AssessmentRecord, Recommendation) {
        let mut recommendation = Recommendation {
            assessment: Some(AssessmentProvenance {
                access_redacted: false,
                id: "assessment-1".into(),
                input_fingerprint: "fingerprint".into(),
                context_level: ContextLevel::WorkflowContext,
                excluded_paths: vec![],
            }),
            id: "recommendation-1".into(),
            workspace_id: "workspace".into(),
            chain_id: "chain".into(),
            chain_depth: 0,
            recommendation_type: RecommendationType::ImproveWorkflow,
            status: RecommendationStatus::Proposed,
            priority: Impact::Low,
            title: "Improve the guide".into(),
            summary: "Clarify verification coverage".into(),
            reason: vec!["Grounded workflow hypothesis".into()],
            evidence: vec![],
            repository_evidence: vec![],
            knowledge_evidence: vec![],
            source_session_ids: vec![],
            target_session_id: None,
            suggested_harness_id: None,
            suggested_model: None,
            suggested_working_directory: None,
            suggested_prompt: None,
            confidence: RecommendationConfidence::Low,
            requires_approval: false,
            dedupe_key: "proactive:v1:documentation:hash".into(),
            created_at: "now".into(),
            updated_at: "now".into(),
            expires_at: None,
            workflow_improvement: WorkflowImprovement {
                proposed_improvement: "Clarify verification coverage".into(),
                target_surface: TargetSurface::Documentation,
                observation_ids: vec![],
                recurrence_count: 0,
                affected_session_ids: vec![],
                impact: Impact::Low,
                expected_benefit: "Clearer verification expectations".into(),
                supersedes_recommendation_id: None,
                dismissal_watermark: None,
            },
            completion_packet: None,
            audit: None,
            rollup_member_ids: vec![],
            rollup_member_dedupe_keys: vec![],
            rollup_generation: None,
            rolled_up_by: None,
            proposed_change: None,
        };
        recommendation.repository_evidence.push(RepositoryEvidence {
            path: "README.md".into(),
            sha256: "hash".into(),
            excerpt: "guide".into(),
            observed_at: "now".into(),
        });
        let record = AssessmentRecord {
            format_version: 1,
            id: "assessment-1".into(),
            state: AssessmentState::Succeeded,
            started_at: "then".into(),
            completed_at: Some("now".into()),
            error_summary: None,
            reused: false,
            pending_recommendation: Some(recommendation.clone()),
            report: Some(AssessmentReport {
                id: "assessment-1".into(),
                workspace_path: "workspace".into(),
                workspace_instance: 1,
                observed_at: "now".into(),
                provider: "codex".into(),
                model: "fixture".into(),
                bundle_version: "v1".into(),
                knowledge_identity: serde_json::json!({"version":"v1"}),
                input_fingerprint: "fingerprint".into(),
                context_level: ContextLevel::WorkflowContext,
                excluded_paths: vec![],
                outcome: "proposed".into(),
                summary: "Grounded hypothesis".into(),
                proposal: Some(AssessmentProposal {
                    target_surface: TargetSurface::Documentation,
                    title: "Improve the guide".into(),
                    summary: "Clarify verification coverage".into(),
                }),
                no_proposal_reason: None,
                unverified_checks: vec![],
                repository_evidence: recommendation.repository_evidence.clone(),
                knowledge_evidence: vec![],
                recommendation_id: Some("recommendation-1".into()),
            }),
        };
        (record, recommendation)
    }

    #[test]
    fn assessment_publication_orders_pending_upsert_and_commit() {
        let (mut record, expected) = proposal();
        let events = RefCell::new(Vec::new());
        publish_with(
            &mut record,
            |value| {
                events.borrow_mut().push(format!("save:{:?}", value.state));
                Ok(())
            },
            |value| {
                assert_eq!(value, &expected);
                events.borrow_mut().push("upsert".into());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            *events.borrow(),
            ["save:PendingRecommendation", "upsert", "save:Succeeded"]
        );
        assert_eq!(record.state, AssessmentState::Succeeded);
        assert!(record.pending_recommendation.is_none());
    }

    #[test]
    fn assessment_publication_failure_before_pending_never_upserts() {
        let (mut record, _) = proposal();
        let upserts = RefCell::new(0);
        assert!(publish_with(
            &mut record,
            |_| Err("disk unavailable".into()),
            |_| {
                *upserts.borrow_mut() += 1;
                Ok(())
            }
        )
        .is_err());
        assert_eq!(*upserts.borrow(), 0);
    }

    #[test]
    fn assessment_pending_upsert_and_final_commit_failures_are_recoverable() {
        for fail_upsert in [true, false] {
            let (mut record, expected) = proposal();
            let durable = RefCell::new(None);
            let publications = RefCell::new(Vec::new());
            let writes = RefCell::new(0);
            let result = publish_with(
                &mut record,
                |value| {
                    *writes.borrow_mut() += 1;
                    if *writes.borrow() == 2 {
                        return Err("commit unavailable".into());
                    }
                    *durable.borrow_mut() = Some(value.clone());
                    Ok(())
                },
                |value| {
                    if fail_upsert {
                        return Err("upsert unavailable".into());
                    }
                    publications.borrow_mut().push(value.id.clone());
                    Ok(())
                },
            );
            assert!(result.is_err());
            let mut pending = durable
                .borrow_mut()
                .take()
                .expect("pending record must be durable");
            assert_eq!(pending.state, AssessmentState::PendingRecommendation);
            recover_with(
                &mut pending,
                |value| {
                    *durable.borrow_mut() = Some(value.clone());
                    Ok(())
                },
                |value| {
                    assert_eq!(value, &expected);
                    if !publications.borrow().contains(&value.id) {
                        publications.borrow_mut().push(value.id.clone());
                    }
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(*publications.borrow(), ["recommendation-1"]);
            assert_eq!(pending.state, AssessmentState::Succeeded);
            assert!(pending.pending_recommendation.is_none());
            recover_with(
                &mut pending,
                |_| panic!("committed recovery must be read-only"),
                |_| panic!("must not duplicate an upsert"),
            )
            .unwrap();
        }
    }

    #[test]
    fn assessment_recovery_rejects_a_mismatched_pending_identity() {
        let (mut record, _) = proposal();
        record.state = AssessmentState::PendingRecommendation;
        record.pending_recommendation.as_mut().unwrap().id = "wrong".into();
        assert!(recover_with(
            &mut record,
            |_| panic!("must not publish corruption"),
            |_| panic!("must not upsert corruption")
        )
        .is_err());
    }
}
