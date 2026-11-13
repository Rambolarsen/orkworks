//! Derive one advisory recommendation and cited report from one validated result.
use super::{
    access::{self, Capture},
    engine::InvestigationResult,
    model::EvidenceStrength,
    store::Report,
};
use crate::taskmaster::{self, KnowledgeEvidence, Recommendation, RepositoryEvidence};
use std::collections::HashSet;

pub(crate) fn report(
    capture: &Capture,
    investigation: InvestigationResult,
) -> Result<Report, String> {
    let manifest = access::manifest(
        capture,
        &investigation.evidence,
        &investigation.supplied_page_ids,
    )?;
    let mut cited = HashSet::new();
    for row in &investigation.result.concepts {
        for id in &row.repository_fact_ids {
            cited.insert(
                manifest
                    .ranges
                    .get(*id)
                    .ok_or("Unknown result fact")?
                    .sha256
                    .as_str(),
            );
        }
    }
    for opportunity in &investigation.result.opportunities {
        cited.extend(
            opportunity
                .repository_fact_hashes
                .iter()
                .map(String::as_str),
        );
    }
    let repository_evidence = investigation
        .evidence
        .facts()
        .iter()
        .filter(|f| cited.contains(f.sha256.as_str()))
        .cloned()
        .collect();
    let pages = capture
        .evaluation
        .knowledge
        .as_ref()
        .ok_or("Guidance unavailable")?;
    let mut knowledge_ids: HashSet<_> = investigation
        .result
        .opportunities
        .iter()
        .flat_map(|o| o.knowledge_page_ids.iter().map(String::as_str))
        .collect();
    knowledge_ids.extend(
        investigation
            .result
            .concepts
            .iter()
            .map(|row| row.rubric_page_id.as_str()),
    );
    if knowledge_ids.iter().any(|id| {
        !investigation
            .supplied_page_ids
            .iter()
            .any(|selected| selected == id)
    }) {
        return Err("Cited rubric page was not supplied".into());
    }
    let knowledge_evidence = pages
        .pages
        .iter()
        .filter(|p| knowledge_ids.contains(p.id.as_str()))
        .map(|p| KnowledgeEvidence {
            page_id: p.id.clone(),
            title: p.title.clone(),
            status: p.status.clone(),
            bundle_version: pages.version.clone(),
            sha256: p.sha256.clone(),
            excerpt: p.content.chars().take(500).collect(),
        })
        .collect();
    Ok(Report {
        manifest,
        result: investigation.result,
        repository_evidence,
        knowledge_evidence,
        recommendation_id: None,
        history_snapshot_id: None,
        history_error: None,
        publication_limitation: None,
    })
}

pub(crate) fn recommendation(
    workspace: &crate::WorkspaceState,
    report: &Report,
    run_id: &str,
    now: &str,
) -> Result<Option<Recommendation>, String> {
    use taskmaster::{
        AssessmentProvenance, ImprovementIntent, ImprovementReference, RecommendationConfidence,
        RecommendationStatus, RecommendationType, WorkflowImprovement,
    };
    let Some(preferred) = report.result.preferred() else {
        return Ok(None);
    };
    let all = workspace
        .recommendation_store
        .list()
        .map_err(|_| "Recommendations unavailable")?;
    if taskmaster::active_workflow_recommendation(&all).is_some() {
        return Ok(None);
    }
    let mut hashes = preferred.repository_fact_hashes.clone();
    hashes.sort();
    let key = format!(
        "proactive:v1:{}:{}",
        taskmaster::assessment::target_surface_key(preferred.target_surface),
        hashes.join(":")
    );
    let existing = all
        .iter()
        .filter(|item| item.dedupe_key == key)
        .collect::<Vec<_>>();
    let fingerprint = report.manifest.fingerprint()?;
    let decision = taskmaster::assessment::proposal_dedupe_decision(
        &existing
            .iter()
            .map(|item| {
                let stale = item
                    .assessment
                    .as_ref()
                    .is_some_and(|p| p.input_fingerprint != fingerprint || p.access_redacted);
                (
                    item.status,
                    stale,
                    item.id.as_str(),
                    item.workflow_improvement
                        .supersedes_recommendation_id
                        .as_deref(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let supersedes = match decision {
        taskmaster::assessment::ProposalDedupeDecision::Suppress => return Ok(None),
        taskmaster::assessment::ProposalDedupeDecision::Create => None,
        taskmaster::assessment::ProposalDedupeDecision::Successor(id) => Some(id),
    };
    let material = supersedes.as_ref().map_or_else(
        || format!("{key}:{run_id}"),
        |id| format!("{key}:{id}:{run_id}"),
    );
    let id = format!(
        "recommendation-{}",
        taskmaster::assessment::short_hash(&material)
    );
    let depth = supersedes
        .as_ref()
        .and_then(|id| existing.iter().find(|item| &item.id == id))
        .map_or(0, |item| item.chain_depth.saturating_add(1));
    let repo = report
        .repository_evidence
        .iter()
        .filter(|fact| preferred.repository_fact_hashes.contains(&fact.sha256))
        .map(|fact| RepositoryEvidence {
            path: fact.path.clone(),
            sha256: fact.sha256.clone(),
            excerpt: fact.text.chars().take(500).collect(),
            observed_at: now.into(),
        })
        .collect();
    let knowledge = report
        .knowledge_evidence
        .iter()
        .filter(|page| preferred.knowledge_page_ids.contains(&page.page_id))
        .cloned()
        .collect();
    Ok(Some(Recommendation {
        assessment: Some(AssessmentProvenance {
            improvement_reference: Some(ImprovementReference {
                version: 2,
                intent: if preferred.evidence_strength == EvidenceStrength::InvestigationNeeded {
                    ImprovementIntent::Investigation
                } else {
                    ImprovementIntent::Implementation
                },
            }),
            sql_sources: None,
            readiness_reference: None,
            access_redacted: false,
            id: run_id.into(),
            input_fingerprint: fingerprint,
            context_level: report.manifest.context_level,
            excluded_paths: report.manifest.excluded_paths.clone(),
        }),
        id,
        workspace_id: report.manifest.workspace_path.clone(),
        chain_id: key.clone(),
        chain_depth: depth,
        recommendation_type: RecommendationType::ImproveWorkflow,
        status: RecommendationStatus::Proposed,
        priority: crate::workflow_observations::Impact::Low,
        title: preferred.title.clone(),
        summary: preferred.summary.clone(),
        reason: vec![preferred.mechanism.clone()],
        evidence: vec![],
        repository_evidence: repo,
        knowledge_evidence: knowledge,
        source_session_ids: vec![],
        target_session_id: None,
        suggested_harness_id: None,
        suggested_model: None,
        suggested_working_directory: None,
        suggested_prompt: None,
        confidence: RecommendationConfidence::Low,
        requires_approval: false,
        dedupe_key: key,
        created_at: now.into(),
        updated_at: now.into(),
        expires_at: None,
        workflow_improvement: WorkflowImprovement {
            proposed_improvement: preferred.summary.clone(),
            target_surface: preferred.target_surface,
            observation_ids: vec![],
            recurrence_count: 0,
            affected_session_ids: vec![],
            impact: crate::workflow_observations::Impact::Low,
            expected_benefit: preferred.benefit.clone(),
            supersedes_recommendation_id: supersedes,
            dismissal_watermark: None,
        },
        completion_packet: None,
        audit: None,
        rollup_member_ids: vec![],
        rollup_member_dedupe_keys: vec![],
        rollup_generation: None,
        rolled_up_by: None,
        proposed_change: None,
    }))
}

pub(crate) fn current_report(report: &Report, capture: &Capture) -> Result<(), String> {
    report.manifest.validate()?;
    if report.manifest.purpose != "brain-improvement-v2"
        || report.manifest.evaluation_key != capture.evaluation_key
    {
        return Err("Improvement producer changed".into());
    }
    let facts = report
        .manifest
        .ranges
        .iter()
        .map(|r| r.sha256.clone())
        .collect::<Vec<_>>();
    let pages = report
        .manifest
        .pages
        .iter()
        .map(|p| p.id.clone())
        .collect::<Vec<_>>();
    let text = serde_json::to_string(&report.result).map_err(|_| "Invalid improvement result")?;
    super::model::FinalResult::decode_with_policy(
        &text,
        &facts,
        &pages,
        &capture.rubric,
        crate::taskmaster::advice_text::TextPolicy::for_improvement_purpose(
            &report.manifest.purpose,
        )?,
    )?;
    for row in &report.result.concepts {
        if !report
            .knowledge_evidence
            .iter()
            .any(|page| page.page_id == row.rubric_page_id)
        {
            return Err("Rubric snapshot unavailable".into());
        }
        for id in &row.repository_fact_ids {
            let hash = facts.get(*id).ok_or("Result fact unavailable")?;
            if !report
                .repository_evidence
                .iter()
                .any(|fact| &fact.sha256 == hash)
            {
                return Err("Result fact snapshot unavailable".into());
            }
        }
    }
    let bundle = capture
        .evaluation
        .knowledge
        .as_ref()
        .ok_or("Guidance unavailable")?;
    for opportunity in &report.result.opportunities {
        if opportunity.repository_fact_hashes.iter().any(|hash| {
            !report
                .repository_evidence
                .iter()
                .any(|fact| &fact.sha256 == hash)
        }) || opportunity.knowledge_page_ids.iter().any(|id| {
            !report
                .knowledge_evidence
                .iter()
                .any(|page| &page.page_id == id)
        }) {
            return Err("Opportunity snapshot unavailable".into());
        }
    }
    for saved in &report.knowledge_evidence {
        let page = bundle
            .pages
            .iter()
            .find(|page| page.id == saved.page_id)
            .ok_or("Guidance changed")?;
        if page.sha256 != saved.sha256
            || page.title != saved.title
            || page.status != saved.status
            || bundle.version != saved.bundle_version
            || page.content.chars().take(500).collect::<String>() != saved.excerpt
        {
            return Err("Guidance snapshot changed".into());
        }
    }
    report.public()?;
    Ok(())
}

/// A stored card cannot acquire authority merely by naming a current report.
pub(crate) fn join_valid(report: &Report, run_id: &str, recommendation: &Recommendation) -> bool {
    let Some(preferred) = report.result.preferred() else {
        return false;
    };
    let Some(provenance) = recommendation.assessment.as_ref() else {
        return false;
    };
    let Some(reference) = provenance.improvement_reference.as_ref() else {
        return false;
    };
    let expected_intent = if preferred.evidence_strength == EvidenceStrength::InvestigationNeeded {
        taskmaster::ImprovementIntent::Investigation
    } else {
        taskmaster::ImprovementIntent::Implementation
    };
    let expected_facts = report
        .repository_evidence
        .iter()
        .filter(|fact| preferred.repository_fact_hashes.contains(&fact.sha256))
        .map(|fact| {
            (
                &fact.path,
                &fact.sha256,
                fact.text.chars().take(500).collect::<String>(),
            )
        })
        .collect::<Vec<_>>();
    let actual_facts = recommendation
        .repository_evidence
        .iter()
        .map(|fact| (&fact.path, &fact.sha256, fact.excerpt.clone()))
        .collect::<Vec<_>>();
    let expected_pages = report
        .knowledge_evidence
        .iter()
        .filter(|page| preferred.knowledge_page_ids.contains(&page.page_id))
        .collect::<Vec<_>>();
    let actual_pages = recommendation.knowledge_evidence.iter().collect::<Vec<_>>();
    let mut hashes = preferred.repository_fact_hashes.clone();
    hashes.sort();
    let key = format!(
        "proactive:v1:{}:{}",
        taskmaster::assessment::target_surface_key(preferred.target_surface),
        hashes.join(":")
    );
    let predecessor = recommendation
        .workflow_improvement
        .supersedes_recommendation_id
        .as_ref();
    let id_material = predecessor.map_or_else(
        || format!("{key}:{run_id}"),
        |id| format!("{key}:{id}:{run_id}"),
    );
    let expected_id = format!(
        "recommendation-{}",
        taskmaster::assessment::short_hash(&id_material)
    );
    let watermark_valid = match (
        &recommendation.status,
        &recommendation.workflow_improvement.dismissal_watermark,
    ) {
        (taskmaster::RecommendationStatus::Dismissed, Some(watermark)) => {
            watermark.dismissed_through_sequence == 0
                && watermark.observation_ids.is_empty()
                && watermark.qualifying_count == 0
                && watermark.highest_impact == recommendation.priority
                && watermark.affected_session_ids.is_empty()
        }
        (taskmaster::RecommendationStatus::Dismissed, None) => false,
        (_, None) => true,
        (_, Some(_)) => false,
    };
    report.recommendation_id.as_deref() == Some(recommendation.id.as_str())
        && provenance.id == run_id
        && report.manifest.fingerprint().ok().as_deref() == Some(&provenance.input_fingerprint)
        && reference.version
            == if report.manifest.purpose == "brain-improvement-v2" {
                2
            } else {
                1
            }
        && reference.intent == expected_intent
        && !provenance.access_redacted
        && provenance.context_level == report.manifest.context_level
        && provenance.excluded_paths == report.manifest.excluded_paths
        && recommendation.workspace_id == report.manifest.workspace_path
        && recommendation.recommendation_type == taskmaster::RecommendationType::ImproveWorkflow
        && recommendation.title == preferred.title
        && recommendation.summary == preferred.summary
        && recommendation.reason == vec![preferred.mechanism.clone()]
        && recommendation.workflow_improvement.target_surface == preferred.target_surface
        && recommendation.workflow_improvement.proposed_improvement == preferred.summary
        && recommendation.workflow_improvement.expected_benefit == preferred.benefit
        && provenance.sql_sources.is_none()
        && provenance.readiness_reference.is_none()
        && recommendation.id == expected_id
        && recommendation.dedupe_key == key
        && recommendation.chain_id == key
        && recommendation.suggested_prompt.is_none()
        && recommendation.proposed_change.is_none()
        && recommendation.completion_packet.is_none()
        && recommendation.suggested_harness_id.is_none()
        && recommendation.suggested_model.is_none()
        && recommendation.suggested_working_directory.is_none()
        && recommendation.source_session_ids.is_empty()
        && recommendation.evidence.is_empty()
        && recommendation.audit.is_none()
        && recommendation.rollup_member_ids.is_empty()
        && recommendation.rollup_member_dedupe_keys.is_empty()
        && recommendation.rollup_generation.is_none()
        && recommendation.rolled_up_by.is_none()
        && recommendation
            .workflow_improvement
            .observation_ids
            .is_empty()
        && recommendation
            .workflow_improvement
            .affected_session_ids
            .is_empty()
        && recommendation.workflow_improvement.recurrence_count == 0
        && watermark_valid
        && recommendation.priority == crate::workflow_observations::Impact::Low
        && recommendation.confidence == taskmaster::RecommendationConfidence::Low
        && !recommendation.requires_approval
        && expected_facts == actual_facts
        && expected_pages == actual_pages
}
