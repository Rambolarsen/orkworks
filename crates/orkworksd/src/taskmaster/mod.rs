pub(crate) mod evaluator;
pub(crate) mod store;

use crate::workflow_observations::{Impact, ObservationKind, ObservationSource};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecommendationType {
    ImproveWorkflow,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecommendationStatus {
    Proposed,
    Accepted,
    Executing,
    Completed,
    Dismissed,
    Superseded,
    Expired,
    Failed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecommendationConfidence {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetSurface {
    Instructions,
    Skill,
    Test,
    Tooling,
    Documentation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DismissalWatermark {
    pub dismissed_at: String,
    pub dismissed_through_sequence: u64,
    pub observation_ids: Vec<String>,
    pub qualifying_count: usize,
    pub highest_impact: Impact,
    pub affected_session_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowObservationEvidence {
    pub observation_id: String,
    pub sequence: u64,
    pub session_id: String,
    pub kind: ObservationKind,
    pub description: String,
    pub evidence: String,
    pub reported_impact: Impact,
    pub source: ObservationSource,
    pub confidence: f64,
    pub observed_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Recommendation {
    pub id: String,
    pub workspace_id: String,
    pub chain_id: String,
    pub chain_depth: u32,
    #[serde(rename = "type")]
    pub recommendation_type: RecommendationType,
    pub status: RecommendationStatus,
    pub priority: Impact,
    pub title: String,
    pub summary: String,
    pub reason: Vec<String>,
    pub evidence: Vec<WorkflowObservationEvidence>,
    pub source_session_ids: Vec<String>,
    pub target_session_id: Option<String>,
    pub suggested_harness_id: Option<String>,
    pub suggested_model: Option<String>,
    pub suggested_working_directory: Option<String>,
    pub suggested_prompt: Option<String>,
    pub confidence: RecommendationConfidence,
    pub requires_approval: bool,
    pub dedupe_key: String,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
    pub workflow_improvement: WorkflowImprovement,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowImprovement {
    pub proposed_improvement: String,
    pub target_surface: TargetSurface,
    pub observation_ids: Vec<String>,
    pub recurrence_count: usize,
    pub affected_session_ids: Vec<String>,
    pub impact: Impact,
    pub expected_benefit: String,
    pub supersedes_recommendation_id: Option<String>,
    pub dismissal_watermark: Option<DismissalWatermark>,
}

/// Deterministically turns retained workflow evidence into passive proposals.
/// Persistence is deliberately left to `RecommendationStore` so an evaluator
/// failure cannot mutate observation history.
pub(crate) fn evaluate_workflow_improvements(
    observations: &[crate::workflow_observations::WorkflowObservation],
    existing: &[Recommendation],
    workspace_id: &str,
    now: &str,
) -> Vec<Recommendation> {
    let mut groups: HashMap<String, Vec<_>> = HashMap::new();
    for observation in observations {
        if observation.confidence < 0.6 {
            continue;
        }
        if observation.reported_impact == Impact::High && observation.confidence < 0.8 {
            continue;
        }
        groups
            .entry(observation.fingerprint.clone())
            .or_default()
            .push(observation);
    }

    let mut proposals = Vec::new();
    for (fingerprint, mut qualifying) in groups {
        qualifying.sort_by_key(|observation| observation.sequence);
        let high_impact_single = qualifying.len() == 1
            && qualifying[0].reported_impact == Impact::High
            && qualifying[0].confidence >= 0.8;
        if qualifying.len() < 2 && !high_impact_single {
            continue;
        }

        let target_surface = target_surface(qualifying[0].kind);
        let dedupe_key = format!(
            "improve_workflow:v1:{}:{}",
            target_surface_name(target_surface),
            fingerprint
        );
        let prior = existing
            .iter()
            .filter(|recommendation| recommendation.dedupe_key == dedupe_key)
            .max_by(|left, right| left.updated_at.cmp(&right.updated_at));

        let mut evidence: Vec<WorkflowObservationEvidence> = prior
            .filter(|recommendation| recommendation.status == RecommendationStatus::Proposed)
            .map(|recommendation| recommendation.evidence.clone())
            .unwrap_or_default();
        for observation in qualifying {
            if !evidence
                .iter()
                .any(|item| item.observation_id == observation.id)
            {
                evidence.push(WorkflowObservationEvidence {
                    observation_id: observation.id.clone(),
                    sequence: observation.sequence,
                    session_id: observation.session_id.clone(),
                    kind: observation.kind,
                    description: observation.description.clone(),
                    evidence: observation.evidence.clone(),
                    reported_impact: observation.reported_impact,
                    source: observation.source,
                    confidence: observation.confidence,
                    observed_at: observation.observed_at.clone(),
                });
            }
        }
        evidence.sort_by_key(|item| item.sequence);

        if let Some(dismissed) =
            prior.filter(|recommendation| recommendation.status == RecommendationStatus::Dismissed)
        {
            let watermark = dismissed.workflow_improvement.dismissal_watermark.as_ref();
            let later = evidence.iter().filter(|item| {
                watermark.is_some_and(|mark| item.sequence > mark.dismissed_through_sequence)
            });
            let later: Vec<_> = later.collect();
            let impact_increased = watermark.is_some_and(|mark| {
                evidence
                    .iter()
                    .any(|item| item.reported_impact > mark.highest_impact)
            });
            let new_session = watermark.is_some_and(|mark| {
                later
                    .iter()
                    .any(|item| !mark.affected_session_ids.contains(&item.session_id))
            });
            if !impact_increased && !(later.len() >= 2 && new_session) {
                continue;
            }
        }

        let impact = evidence
            .iter()
            .map(|item| item.reported_impact)
            .max()
            .unwrap_or(Impact::Low);
        let confidence = if evidence.iter().all(|item| item.confidence >= 0.8) {
            RecommendationConfidence::High
        } else {
            RecommendationConfidence::Medium
        };
        let affected_session_ids = unique_session_ids(&evidence);
        let observation_ids = evidence
            .iter()
            .map(|item| item.observation_id.clone())
            .collect::<Vec<_>>();
        let id = prior
            .filter(|recommendation| recommendation.status == RecommendationStatus::Proposed)
            .map(|recommendation| recommendation.id.clone())
            .unwrap_or_else(|| {
                format!(
                    "recommendation-{}",
                    stable_id(&dedupe_key, &observation_ids)
                )
            });
        let supersedes = prior
            .filter(|recommendation| recommendation.status == RecommendationStatus::Dismissed)
            .map(|recommendation| recommendation.id.clone());
        let title = format!("Improve {}", target_surface_name(target_surface));
        let description = evidence[0].description.clone();
        let proposed_improvement =
            format!("{}: {}", improvement_prefix(evidence[0].kind), description);
        let summary = proposed_improvement.clone();
        let reason =
            format!(
            "{} qualifying observation{} across {} session{}; highest impact {:?}; sources: {}.",
            evidence.len(),
            if evidence.len() == 1 { "" } else { "s" },
            affected_session_ids.len(),
            if affected_session_ids.len() == 1 { "" } else { "s" },
            impact,
            source_mix(&evidence)
        );
        let created_at = match prior {
            Some(recommendation) if recommendation.status == RecommendationStatus::Proposed => {
                recommendation.created_at.clone()
            }
            _ => now.to_string(),
        };
        proposals.push(Recommendation {
            id,
            workspace_id: workspace_id.to_string(),
            chain_id: prior
                .map(|recommendation| recommendation.chain_id.clone())
                .unwrap_or_else(|| dedupe_key.clone()),
            chain_depth: prior
                .map(|recommendation| recommendation.chain_depth)
                .unwrap_or(0),
            recommendation_type: RecommendationType::ImproveWorkflow,
            status: RecommendationStatus::Proposed,
            priority: impact,
            title,
            summary,
            reason: vec![reason],
            evidence: evidence.clone(),
            source_session_ids: affected_session_ids.clone(),
            target_session_id: None,
            suggested_harness_id: None,
            suggested_model: None,
            suggested_working_directory: None,
            suggested_prompt: None,
            confidence,
            requires_approval: false,
            dedupe_key,
            created_at,
            updated_at: now.to_string(),
            expires_at: None,
            workflow_improvement: WorkflowImprovement {
                proposed_improvement,
                target_surface,
                observation_ids,
                recurrence_count: evidence.len(),
                affected_session_ids,
                impact,
                expected_benefit: expected_benefit(target_surface).into(),
                supersedes_recommendation_id: supersedes,
                dismissal_watermark: None,
            },
        });
    }
    proposals.sort_by(|left, right| left.dedupe_key.cmp(&right.dedupe_key));
    proposals
}

fn target_surface(kind: ObservationKind) -> TargetSurface {
    match kind {
        ObservationKind::MissingContext
        | ObservationKind::Assumption
        | ObservationKind::Correction => TargetSurface::Instructions,
        ObservationKind::VerificationGap => TargetSurface::Test,
        ObservationKind::Repetition | ObservationKind::Obstacle | ObservationKind::Workaround => {
            TargetSurface::Tooling
        }
    }
}

fn target_surface_name(surface: TargetSurface) -> &'static str {
    match surface {
        TargetSurface::Instructions => "instructions",
        TargetSurface::Skill => "skill",
        TargetSurface::Test => "test",
        TargetSurface::Tooling => "tooling",
        TargetSurface::Documentation => "documentation",
    }
}

fn improvement_prefix(kind: ObservationKind) -> &'static str {
    match kind {
        ObservationKind::Repetition => "Automate or remove repeated work",
        ObservationKind::Obstacle => "Remove or document the obstacle",
        ObservationKind::MissingContext => "Add missing repository context",
        ObservationKind::Assumption => "Make the required assumption explicit",
        ObservationKind::Correction => "Prevent this recurring correction",
        ObservationKind::Workaround => "Replace the workaround with a supported path",
        ObservationKind::VerificationGap => "Add reliable verification for",
    }
}

fn expected_benefit(target: TargetSurface) -> &'static str {
    match target {
        TargetSurface::Instructions => "Agents receive the required context before acting.",
        TargetSurface::Skill => "Agents follow one repeatable workflow for this task.",
        TargetSurface::Test => "The workflow gains repeatable verification.",
        TargetSurface::Tooling => "Agents spend less time on avoidable manual recovery.",
        TargetSurface::Documentation => "The supported workflow becomes discoverable.",
    }
}

fn source_mix(evidence: &[WorkflowObservationEvidence]) -> String {
    let has_agent = evidence
        .iter()
        .any(|item| item.source == ObservationSource::Agent);
    let has_peon = evidence
        .iter()
        .any(|item| item.source == ObservationSource::Peon);
    match (has_agent, has_peon) {
        (true, true) => "agent and peon".into(),
        (true, false) => "agent".into(),
        (false, true) => "peon".into(),
        (false, false) => "unknown".into(),
    }
}

fn unique_session_ids(evidence: &[WorkflowObservationEvidence]) -> Vec<String> {
    let mut ids = evidence
        .iter()
        .map(|item| item.session_id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn stable_id(dedupe_key: &str, observation_ids: &[String]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    dedupe_key.hash(&mut hasher);
    observation_ids.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        id: &str,
        sequence: u64,
        session_id: &str,
        confidence: f64,
        impact: Impact,
    ) -> crate::workflow_observations::WorkflowObservation {
        crate::workflow_observations::WorkflowObservation {
            id: id.into(),
            sequence,
            session_id: session_id.into(),
            observed_at: format!("2026-08-20T12:00:{sequence:02}Z"),
            kind: ObservationKind::Obstacle,
            description: "The setup blocks progress".into(),
            evidence: format!("failure {sequence}"),
            reported_impact: impact,
            source: ObservationSource::Peon,
            confidence,
            fingerprint: "v1:obstacle:the setup blocks progress".into(),
        }
    }

    #[test]
    fn proposes_one_deduplicated_improvement_for_two_qualifying_observations() {
        let observations = vec![
            observation("one", 1, "session-a", 0.6, Impact::Low),
            observation("two", 2, "session-b", 0.8, Impact::Medium),
        ];
        let proposals = evaluate_workflow_improvements(
            &observations,
            &[],
            "workspace-1",
            "2026-08-21T12:00:00Z",
        );

        assert_eq!(proposals.len(), 1);
        assert_eq!(
            proposals[0].recommendation_type,
            RecommendationType::ImproveWorkflow
        );
        assert_eq!(
            proposals[0].workflow_improvement.target_surface,
            TargetSurface::Tooling
        );
        assert_eq!(proposals[0].workflow_improvement.recurrence_count, 2);
        assert_eq!(proposals[0].confidence, RecommendationConfidence::Medium);
        assert!(!proposals[0].requires_approval);
        assert_eq!(proposals[0].target_session_id, None);
    }

    #[test]
    fn proposes_a_high_impact_single_observation_but_ignores_weak_evidence() {
        let high = observation("high", 1, "session-a", 0.8, Impact::High);
        let weak = observation("weak", 2, "session-b", 0.59, Impact::High);
        let proposals =
            evaluate_workflow_improvements(&[high], &[], "workspace-1", "2026-08-21T12:00:00Z");
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].priority, Impact::High);

        assert!(evaluate_workflow_improvements(
            &[weak],
            &[],
            "workspace-1",
            "2026-08-21T12:00:00Z",
        )
        .is_empty());
    }

    #[test]
    fn promotes_repeated_evidence_from_one_session() {
        let proposals = evaluate_workflow_improvements(
            &[
                observation("one", 1, "session-a", 0.8, Impact::Medium),
                observation("two", 2, "session-a", 0.8, Impact::Medium),
            ],
            &[],
            "workspace-1",
            "2026-08-21T12:00:00Z",
        );

        assert_eq!(proposals.len(), 1);
    }

    #[test]
    fn updates_the_existing_proposed_family_without_creating_a_duplicate() {
        let first = observation("one", 1, "session-a", 0.8, Impact::Low);
        let second = observation("two", 2, "session-b", 0.8, Impact::Low);
        let existing = evaluate_workflow_improvements(
            &[first.clone(), second.clone()],
            &[],
            "workspace-1",
            "2026-08-21T12:00:00Z",
        );
        let updated = evaluate_workflow_improvements(
            &[
                first,
                second,
                observation("three", 3, "session-c", 0.8, Impact::Low),
            ],
            &existing,
            "workspace-1",
            "2026-08-21T12:01:00Z",
        );
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].id, existing[0].id);
        assert_eq!(updated[0].evidence.len(), 3);
    }
}
