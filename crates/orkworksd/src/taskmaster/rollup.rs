use super::{Recommendation, RecommendationStatus, TargetSurface, WorkflowObservationEvidence};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_ROLLUP_FAMILIES: usize = 32;
pub(crate) const MAX_ROLLUP_OBSERVATIONS: usize = 96;
pub(crate) const MAX_ROLLUP_SOURCE_SESSIONS: usize = 8;
pub(crate) const MAX_ROLLUP_INPUT_BYTES: usize = 128 * 1024;
pub(crate) const MAX_ROLLUP_RESPONSE_BYTES: usize = 64 * 1024;
pub(crate) const MAX_ROLLUP_CLUSTERS: usize = 8;
pub(crate) const MIN_ROLLUP_MEMBERS: usize = 2;
pub(crate) const MAX_ROLLUP_MEMBERS: usize = 8;
pub(crate) const MAX_ROLLUP_TITLE_CHARS: usize = 240;
pub(crate) const MAX_ROLLUP_SUMMARY_CHARS: usize = 1_000;
pub(crate) const MAX_PARENT_EVIDENCE_ENTRIES: usize = 64;
pub(crate) const MAX_PARENT_EVIDENCE_BYTES: usize = 128 * 1024;

const MAX_SNAPSHOT_PROBLEM_AREA_CHARS: usize = 120;
const MAX_EVIDENCE_ID_CHARS: usize = 256;
const MAX_EVIDENCE_SESSION_ID_CHARS: usize = 256;
const MAX_EVIDENCE_DESCRIPTION_CHARS: usize = 500;
const MAX_EVIDENCE_TEXT_CHARS: usize = 2_000;
const MAX_EVIDENCE_TIMESTAMP_CHARS: usize = 64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RollupFamilySnapshot {
    pub recommendation_id: String,
    pub dedupe_key: String,
    pub target_surface: TargetSurface,
    pub problem_area: Option<String>,
    pub title: String,
    pub summary: String,
    pub representative_evidence: Vec<WorkflowObservationEvidence>,
    pub source_session_ids: Vec<String>,
    #[serde(default)]
    pub active_parent_id: Option<String>,
    #[serde(default)]
    pub active_parent_member_ids: Vec<String>,
    pub evidence_snapshot_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RollupCluster {
    pub member_recommendation_ids: Vec<String>,
    pub target_surface: TargetSurface,
    pub title: String,
    pub summary: String,
}

pub(crate) type RollupModelCluster = RollupCluster;
pub(crate) type ValidatedRollupCluster = RollupCluster;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RollupValidationError {
    TooManyFamilies,
    TooManyObservations,
    InputTooLarge,
    ResponseTooLarge,
    UnknownMemberId(String),
    EmptyCluster,
    DuplicateCluster,
    DuplicateMemberId(String),
    TooFewMembers,
    TooManyMembers,
    TooManyClusters,
    OverlappingMember(String),
    MismatchedTargetSurface,
    NotProposed(String),
    GeneratedTextOutOfBounds { field: &'static str, limit: usize },
}

impl RollupFamilySnapshot {
    pub(crate) fn from_recommendation(
        recommendation: &Recommendation,
    ) -> Result<Self, RollupValidationError> {
        Self::from_recommendation_with_active_parent(recommendation, None)
    }

    pub(crate) fn from_recommendation_with_active_parent(
        recommendation: &Recommendation,
        active_parent: Option<&Recommendation>,
    ) -> Result<Self, RollupValidationError> {
        if recommendation.status != RecommendationStatus::Proposed {
            return Err(RollupValidationError::NotProposed(
                recommendation.id.clone(),
            ));
        }

        let representative_evidence = representative_evidence(&recommendation.evidence);
        let mut source_session_ids = recommendation.source_session_ids.clone();
        source_session_ids.sort();
        source_session_ids.dedup();
        source_session_ids.truncate(MAX_ROLLUP_SOURCE_SESSIONS);
        let active_parent_id = active_parent.map(|parent| parent.id.clone());
        let mut active_parent_member_ids = active_parent
            .map(|parent| parent.rollup_member_ids.clone())
            .unwrap_or_default();
        active_parent_member_ids.sort();
        active_parent_member_ids.dedup();
        let mut snapshot = Self {
            recommendation_id: recommendation.id.clone(),
            dedupe_key: recommendation.dedupe_key.clone(),
            target_surface: recommendation.workflow_improvement.target_surface,
            problem_area: representative_evidence
                .iter()
                .find_map(|item| item.problem_area.clone())
                .map(|value| truncate_chars(&value, MAX_SNAPSHOT_PROBLEM_AREA_CHARS)),
            title: truncate_chars(&recommendation.title, MAX_ROLLUP_TITLE_CHARS),
            summary: truncate_chars(&recommendation.summary, MAX_ROLLUP_SUMMARY_CHARS),
            representative_evidence,
            source_session_ids,
            active_parent_id,
            active_parent_member_ids,
            evidence_snapshot_hash: String::new(),
        };
        snapshot.evidence_snapshot_hash = snapshot_hash(&snapshot);
        Ok(snapshot)
    }
}

pub(crate) fn build_rollup_family_snapshots(
    recommendations: &[Recommendation],
) -> Result<Vec<RollupFamilySnapshot>, RollupValidationError> {
    build_rollup_family_snapshots_with_offset(recommendations, 0)
}

/// Builds a deterministic batch of exact families. `batch_offset` rotates the
/// family groups between periodic evaluations while keeping every active
/// parent's complete member set together.
pub(crate) fn build_rollup_family_snapshots_with_offset(
    recommendations: &[Recommendation],
    batch_offset: usize,
) -> Result<Vec<RollupFamilySnapshot>, RollupValidationError> {
    let active_parents = recommendations
        .iter()
        .filter(|recommendation| {
            recommendation.status == RecommendationStatus::Proposed
                && !recommendation.rollup_member_ids.is_empty()
                && recommendation.rolled_up_by.is_none()
        })
        .map(|recommendation| (recommendation.id.as_str(), recommendation))
        .collect::<BTreeMap<_, _>>();
    let mut candidates = recommendations
        .iter()
        .filter_map(|recommendation| {
            if recommendation.status == RecommendationStatus::Proposed
                && recommendation.rollup_member_ids.is_empty()
                && recommendation.rolled_up_by.is_none()
                && is_exact_family(recommendation)
            {
                return Some((recommendation.clone(), None));
            }
            let parent_id = recommendation.rolled_up_by.as_deref()?;
            let parent = active_parents.get(parent_id)?;
            (parent.rollup_member_ids.contains(&recommendation.id)
                && is_exact_family(recommendation))
            .then(|| {
                let mut comparable = recommendation.clone();
                comparable.status = RecommendationStatus::Proposed;
                comparable.rolled_up_by = None;
                (comparable, Some((*parent).clone()))
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|(left, _), (right, _)| left.id.cmp(&right.id));

    let mut groups: Vec<(String, Vec<(Recommendation, Option<Recommendation>)>)> = Vec::new();
    for candidate in candidates {
        let key = candidate
            .1
            .as_ref()
            .map(|parent| parent.id.clone())
            .unwrap_or_else(|| candidate.0.id.clone());
        if let Some((_, group)) = groups.iter_mut().find(|(group_key, _)| *group_key == key) {
            group.push(candidate);
        } else {
            groups.push((key, vec![candidate]));
        }
    }
    groups.sort_by(|left, right| left.0.cmp(&right.0));

    let mut snapshots = Vec::new();
    if groups.is_empty() {
        return Ok(snapshots);
    }
    let start = batch_offset % groups.len();
    for group_index in 0..groups.len() {
        let (_, group) = &groups[(start + group_index) % groups.len()];
        if snapshots.len() + group.len() > MAX_ROLLUP_FAMILIES {
            continue;
        }
        let group_snapshots = group
            .iter()
            .map(|(recommendation, active_parent)| {
                RollupFamilySnapshot::from_recommendation_with_active_parent(
                    recommendation,
                    active_parent.as_ref(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut candidate = snapshots.clone();
        candidate.extend(group_snapshots);
        if serialized_size(&candidate) <= MAX_ROLLUP_INPUT_BYTES {
            snapshots = candidate;
        }
    }
    Ok(snapshots)
}

fn is_exact_family(recommendation: &Recommendation) -> bool {
    !recommendation.evidence.is_empty()
        && !recommendation
            .workflow_improvement
            .observation_ids
            .is_empty()
}

pub(crate) fn validate_rollup_clusters(
    supplied_snapshots: &[RollupFamilySnapshot],
    model_clusters: &[RollupCluster],
) -> Result<Vec<RollupCluster>, RollupValidationError> {
    if supplied_snapshots.len() > MAX_ROLLUP_FAMILIES {
        return Err(RollupValidationError::TooManyFamilies);
    }
    let supplied_observations = supplied_snapshots
        .iter()
        .map(|snapshot| snapshot.representative_evidence.len())
        .sum::<usize>();
    if supplied_observations > MAX_ROLLUP_OBSERVATIONS {
        return Err(RollupValidationError::TooManyObservations);
    }
    if serialized_size(supplied_snapshots) > MAX_ROLLUP_INPUT_BYTES {
        return Err(RollupValidationError::InputTooLarge);
    }
    if model_clusters.len() > MAX_ROLLUP_CLUSTERS {
        return Err(RollupValidationError::TooManyClusters);
    }
    if serialized_size(model_clusters) > MAX_ROLLUP_RESPONSE_BYTES {
        return Err(RollupValidationError::ResponseTooLarge);
    }

    let by_id = supplied_snapshots
        .iter()
        .map(|snapshot| (snapshot.recommendation_id.as_str(), snapshot))
        .collect::<BTreeMap<_, _>>();
    let mut seen_members = BTreeSet::new();
    let mut seen_clusters = BTreeSet::new();
    let mut normalized = Vec::with_capacity(model_clusters.len());

    for cluster in model_clusters {
        validate_generated_text("title", &cluster.title, MAX_ROLLUP_TITLE_CHARS)?;
        validate_generated_text("summary", &cluster.summary, MAX_ROLLUP_SUMMARY_CHARS)?;
        if cluster.member_recommendation_ids.is_empty() {
            return Err(RollupValidationError::EmptyCluster);
        }
        if cluster.member_recommendation_ids.len() < MIN_ROLLUP_MEMBERS {
            return Err(RollupValidationError::TooFewMembers);
        }
        if cluster.member_recommendation_ids.len() > MAX_ROLLUP_MEMBERS {
            return Err(RollupValidationError::TooManyMembers);
        }

        let mut member_ids = cluster.member_recommendation_ids.clone();
        member_ids.sort();
        for pair in member_ids.windows(2) {
            if pair[0] == pair[1] {
                return Err(RollupValidationError::DuplicateMemberId(pair[0].clone()));
            }
        }
        let member_set = member_ids.iter().cloned().collect::<BTreeSet<_>>();
        if !seen_clusters.insert(member_set) {
            return Err(RollupValidationError::DuplicateCluster);
        }
        for member_id in &member_ids {
            let Some(snapshot) = by_id.get(member_id.as_str()) else {
                return Err(RollupValidationError::UnknownMemberId(member_id.clone()));
            };
            if snapshot.target_surface != cluster.target_surface {
                return Err(RollupValidationError::MismatchedTargetSurface);
            }
            if !seen_members.insert(member_id.clone()) {
                return Err(RollupValidationError::OverlappingMember(member_id.clone()));
            }
        }

        normalized.push(RollupCluster {
            member_recommendation_ids: member_ids,
            target_surface: cluster.target_surface,
            title: cluster.title.clone(),
            summary: cluster.summary.clone(),
        });
    }
    let mut normalized = normalized;
    normalized.sort_by(|left, right| {
        left.member_recommendation_ids
            .cmp(&right.member_recommendation_ids)
    });
    Ok(normalized)
}

pub(crate) fn stable_rollup_id(member_recommendation_ids: &[String]) -> String {
    let mut sorted = member_recommendation_ids.to_vec();
    sorted.sort();
    let mut hasher = Sha256::new();
    for member_id in sorted {
        hasher.update(member_id.as_bytes());
        hasher.update([0]);
    }
    format!("rollup:{:x}", hasher.finalize())
}

pub(crate) fn project_parent_evidence(
    evidence: &[WorkflowObservationEvidence],
) -> Vec<WorkflowObservationEvidence> {
    let mut unique = BTreeMap::new();
    for item in evidence {
        let bounded = bounded_evidence(item);
        unique
            .entry(bounded.observation_id.clone())
            .and_modify(|existing| {
                if evidence_order(&bounded, existing).is_lt() {
                    *existing = bounded.clone();
                }
            })
            .or_insert(bounded);
    }
    let mut candidates = unique.into_values().collect::<Vec<_>>();
    candidates.sort_by(evidence_order);

    let mut projection = Vec::new();
    for candidate in candidates {
        if projection.len() == MAX_PARENT_EVIDENCE_ENTRIES {
            break;
        }
        let mut next = projection.clone();
        next.push(candidate.clone());
        if serialized_size(&next) <= MAX_PARENT_EVIDENCE_BYTES {
            projection.push(candidate);
        }
    }
    projection
}

pub(crate) fn serialized_size<T: Serialize + ?Sized>(value: &T) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |json| json.len())
}

fn representative_evidence(
    evidence: &[WorkflowObservationEvidence],
) -> Vec<WorkflowObservationEvidence> {
    if evidence.is_empty() {
        return Vec::new();
    }
    let mut ordered = evidence.iter().map(bounded_evidence).collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.sequence
            .cmp(&right.sequence)
            .then(left.observation_id.cmp(&right.observation_id))
    });
    let earliest = ordered[0].clone();
    let latest = ordered[ordered.len() - 1].clone();
    let highest_impact = ordered
        .iter()
        .max_by(|left, right| {
            left.reported_impact
                .cmp(&right.reported_impact)
                .then(left.sequence.cmp(&right.sequence))
                .then(right.observation_id.cmp(&left.observation_id))
        })
        .expect("non-empty evidence")
        .clone();
    let mut selected = vec![earliest];
    for candidate in [latest, highest_impact] {
        if !selected
            .iter()
            .any(|item| item.observation_id == candidate.observation_id)
        {
            selected.push(candidate);
        }
    }
    selected
}

fn bounded_evidence(item: &WorkflowObservationEvidence) -> WorkflowObservationEvidence {
    let mut bounded = item.clone();
    bounded.observation_id = truncate_chars(&bounded.observation_id, MAX_EVIDENCE_ID_CHARS);
    bounded.session_id = truncate_chars(&bounded.session_id, MAX_EVIDENCE_SESSION_ID_CHARS);
    bounded.description = truncate_chars(&bounded.description, MAX_EVIDENCE_DESCRIPTION_CHARS);
    bounded.evidence = truncate_chars(&bounded.evidence, MAX_EVIDENCE_TEXT_CHARS);
    bounded.problem_area = bounded
        .problem_area
        .as_deref()
        .map(|value| truncate_chars(value, MAX_SNAPSHOT_PROBLEM_AREA_CHARS));
    bounded.observed_at = truncate_chars(&bounded.observed_at, MAX_EVIDENCE_TIMESTAMP_CHARS);
    bounded
}

fn evidence_order(
    left: &WorkflowObservationEvidence,
    right: &WorkflowObservationEvidence,
) -> std::cmp::Ordering {
    right
        .reported_impact
        .cmp(&left.reported_impact)
        .then(right.sequence.cmp(&left.sequence))
        .then(left.observation_id.cmp(&right.observation_id))
}

fn snapshot_hash(snapshot: &RollupFamilySnapshot) -> String {
    let mut content = snapshot.clone();
    content.evidence_snapshot_hash.clear();
    let serialized = serde_json::to_vec(&content).expect("rollup snapshots are serializable");
    format!("{:x}", Sha256::digest(serialized))
}

fn validate_generated_text(
    field: &'static str,
    value: &str,
    limit: usize,
) -> Result<(), RollupValidationError> {
    if value.trim().is_empty()
        || value.chars().count() > limit
        || value.chars().any(char::is_control)
    {
        return Err(RollupValidationError::GeneratedTextOutOfBounds { field, limit });
    }
    Ok(())
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::{
        RecommendationConfidence, RecommendationStatus, RecommendationType, WorkflowImprovement,
    };
    use crate::workflow_observations::{Impact, ObservationKind, ObservationSource};

    fn evidence(
        id: &str,
        sequence: u64,
        session_id: &str,
        impact: Impact,
    ) -> WorkflowObservationEvidence {
        WorkflowObservationEvidence {
            observation_id: id.into(),
            sequence,
            session_id: session_id.into(),
            kind: ObservationKind::Obstacle,
            description: format!("Description for {id}"),
            evidence: format!("Evidence for {id}"),
            problem_area: Some("Model detection".into()),
            reported_impact: impact,
            source: ObservationSource::Peon,
            confidence: 0.9,
            observed_at: format!("2026-09-13T00:00:{sequence:02}Z"),
        }
    }

    fn recommendation(
        id: &str,
        target_surface: TargetSurface,
        evidence: Vec<WorkflowObservationEvidence>,
    ) -> Recommendation {
        let observation_ids = evidence
            .iter()
            .map(|item| item.observation_id.clone())
            .collect();
        let source_session_ids = evidence
            .iter()
            .map(|item| item.session_id.clone())
            .collect();
        Recommendation {
            id: id.into(),
            workspace_id: "workspace-1".into(),
            chain_id: format!("chain-{id}"),
            chain_depth: 0,
            recommendation_type: RecommendationType::ImproveWorkflow,
            status: RecommendationStatus::Proposed,
            priority: Impact::Medium,
            title: format!("Improve {target_surface:?}"),
            summary: format!("Summary for {id}"),
            reason: vec![format!("Reason for {id}")],
            evidence,
            repository_evidence: vec![],
            knowledge_evidence: vec![],
            source_session_ids,
            target_session_id: None,
            suggested_harness_id: None,
            suggested_model: None,
            suggested_working_directory: None,
            suggested_prompt: None,
            confidence: RecommendationConfidence::Medium,
            requires_approval: false,
            dedupe_key: format!("improve_workflow:v1:{id}"),
            created_at: "2026-09-13T00:00:00Z".into(),
            updated_at: "2026-09-13T00:00:00Z".into(),
            expires_at: None,
            workflow_improvement: WorkflowImprovement {
                proposed_improvement: format!("Improve {id}"),
                target_surface,
                observation_ids,
                recurrence_count: 2,
                affected_session_ids: vec!["session-a".into(), "session-b".into()],
                impact: Impact::Medium,
                expected_benefit: "Less repeated work".into(),
                supersedes_recommendation_id: None,
                dismissal_watermark: None,
            },
            rollup_member_ids: Vec::new(),
            rollup_member_dedupe_keys: Vec::new(),
            rollup_generation: None,
            rolled_up_by: None,
        }
    }

    fn snapshot(id: &str, target_surface: TargetSurface) -> RollupFamilySnapshot {
        RollupFamilySnapshot::from_recommendation(&recommendation(
            id,
            target_surface,
            vec![evidence("observation-a", 1, "session-a", Impact::Low)],
        ))
        .unwrap()
    }

    fn cluster(ids: &[&str], target_surface: TargetSurface) -> RollupCluster {
        RollupCluster {
            member_recommendation_ids: ids.iter().map(|id| (*id).into()).collect(),
            target_surface,
            title: "Related workflow issue".into(),
            summary: "These exact families share one bounded problem.".into(),
        }
    }

    #[test]
    fn legacy_recommendation_json_defaults_rollup_fields_and_problem_area() {
        let json = serde_json::json!({
            "id":"recommendation-1", "workspaceId":"workspace-1", "chainId":"chain-1",
            "chainDepth":0, "type":"improve_workflow", "status":"proposed",
            "priority":"medium", "title":"Improve tooling", "summary":"Summary",
            "reason":["Reason"], "evidence":[{
                "observationId":"observation-1", "sequence":1, "sessionId":"session-1",
                "kind":"obstacle", "description":"Description", "evidence":"Evidence",
                "reportedImpact":"medium", "source":"peon", "confidence":0.9,
                "observedAt":"2026-09-13T00:00:00Z"
            }], "sourceSessionIds":["session-1"], "targetSessionId":null,
            "suggestedHarnessId":null, "suggestedModel":null, "suggestedWorkingDirectory":null,
            "suggestedPrompt":null, "confidence":"medium", "requiresApproval":false,
            "dedupeKey":"improve_workflow:v1:tooling:v1:obstacle:description",
            "createdAt":"2026-09-13T00:00:00Z", "updatedAt":"2026-09-13T00:00:00Z",
            "expiresAt":null, "workflowImprovement":{
                "proposedImprovement":"Improve tooling", "targetSurface":"tooling",
                "observationIds":["observation-1"], "recurrenceCount":1,
                "affectedSessionIds":["session-1"], "impact":"medium",
                "expectedBenefit":"Less repeated work", "supersedesRecommendationId":null,
                "dismissalWatermark":null
            }
        });
        let parsed: Recommendation = serde_json::from_value(json).unwrap();
        assert!(parsed.rollup_member_ids.is_empty());
        assert!(parsed.rollup_member_dedupe_keys.is_empty());
        assert_eq!(parsed.rollup_generation, None);
        assert_eq!(parsed.rolled_up_by, None);
        assert_eq!(parsed.evidence[0].problem_area, None);
    }

    #[test]
    fn rolled_up_status_serializes_and_relationship_fields_round_trip() {
        let mut recommendation = recommendation(
            "recommendation-1",
            TargetSurface::Tooling,
            vec![evidence("observation-1", 1, "session-1", Impact::Medium)],
        );
        recommendation.status = RecommendationStatus::RolledUp;
        recommendation.rollup_member_ids = vec!["recommendation-2".into()];
        recommendation.rollup_member_dedupe_keys = vec!["dedupe-2".into()];
        recommendation.rollup_generation = Some(7);
        recommendation.rolled_up_by = Some("rollup:parent".into());

        let json = serde_json::to_value(&recommendation).unwrap();
        assert_eq!(json["status"], "rolled_up");
        assert_eq!(
            json["rollupMemberIds"],
            serde_json::json!(["recommendation-2"])
        );
        assert_eq!(
            json["rollupMemberDedupeKeys"],
            serde_json::json!(["dedupe-2"])
        );
        assert_eq!(json["rollupGeneration"], 7);
        assert_eq!(json["rolledUpBy"], "rollup:parent");
        assert_eq!(
            serde_json::from_value::<Recommendation>(json).unwrap(),
            recommendation
        );
    }

    #[test]
    fn unknown_recommendation_status_remains_strictly_rejected() {
        let mut json = serde_json::to_value(recommendation(
            "recommendation-1",
            TargetSurface::Tooling,
            vec![evidence("observation-1", 1, "session-1", Impact::Medium)],
        ))
        .unwrap();
        json["status"] = "future_status".into();
        assert!(serde_json::from_value::<Recommendation>(json).is_err());
    }

    #[test]
    fn stable_rollup_id_is_order_independent_and_uses_only_member_ids() {
        let first = stable_rollup_id(&["recommendation-b".into(), "recommendation-a".into()]);
        let second = stable_rollup_id(&["recommendation-a".into(), "recommendation-b".into()]);
        let different_prose =
            stable_rollup_id(&["recommendation-a".into(), "recommendation-b".into()]);
        assert_eq!(first, second);
        assert_eq!(first, different_prose);
        assert!(first.starts_with("rollup:"));
    }

    #[test]
    fn valid_same_target_cluster_is_normalized() {
        let snapshots = vec![
            snapshot("recommendation-a", TargetSurface::Tooling),
            snapshot("recommendation-b", TargetSurface::Tooling),
        ];
        let clusters = validate_rollup_clusters(
            &snapshots,
            &[cluster(
                &["recommendation-b", "recommendation-a"],
                TargetSurface::Tooling,
            )],
        )
        .unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(
            clusters[0].member_recommendation_ids,
            vec!["recommendation-a", "recommendation-b"]
        );
        assert_eq!(clusters[0].target_surface, TargetSurface::Tooling);
    }

    #[test]
    fn reversed_model_cluster_order_has_identical_normalized_output() {
        let snapshots = vec![
            snapshot("recommendation-a", TargetSurface::Tooling),
            snapshot("recommendation-b", TargetSurface::Tooling),
            snapshot("recommendation-c", TargetSurface::Tooling),
            snapshot("recommendation-d", TargetSurface::Tooling),
        ];
        let first = validate_rollup_clusters(
            &snapshots,
            &[
                cluster(
                    &["recommendation-c", "recommendation-d"],
                    TargetSurface::Tooling,
                ),
                cluster(
                    &["recommendation-a", "recommendation-b"],
                    TargetSurface::Tooling,
                ),
            ],
        )
        .unwrap();
        let second = validate_rollup_clusters(
            &snapshots,
            &[
                cluster(
                    &["recommendation-a", "recommendation-b"],
                    TargetSurface::Tooling,
                ),
                cluster(
                    &["recommendation-c", "recommendation-d"],
                    TargetSurface::Tooling,
                ),
            ],
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_unknown_member_ids() {
        let result = validate_rollup_clusters(
            &[snapshot("recommendation-a", TargetSurface::Tooling)],
            &[cluster(
                &["recommendation-a", "unknown"],
                TargetSurface::Tooling,
            )],
        );
        assert!(matches!(
            result,
            Err(RollupValidationError::UnknownMemberId(_))
        ));
    }

    #[test]
    fn rejects_empty_duplicate_and_short_clusters() {
        let snapshots = vec![
            snapshot("recommendation-a", TargetSurface::Tooling),
            snapshot("recommendation-b", TargetSurface::Tooling),
        ];
        for ids in [
            &[][..],
            &["recommendation-a"][..],
            &["recommendation-a", "recommendation-a"][..],
        ] {
            let result =
                validate_rollup_clusters(&snapshots, &[cluster(ids, TargetSurface::Tooling)]);
            assert!(result.is_err(), "accepted invalid member set {ids:?}");
        }
    }

    #[test]
    fn rejects_too_many_clusters_and_out_of_bound_cluster_sizes() {
        let snapshots: Vec<_> = (0..33)
            .map(|index| snapshot(&format!("recommendation-{index}"), TargetSurface::Tooling))
            .collect();
        let too_many_families = validate_rollup_clusters(
            &snapshots[..32],
            &(0..9)
                .map(|_index| {
                    cluster(
                        &["recommendation-0", "recommendation-1"],
                        TargetSurface::Tooling,
                    )
                })
                .collect::<Vec<_>>(),
        );
        assert!(matches!(
            too_many_families,
            Err(RollupValidationError::TooManyClusters)
        ));

        let too_many_members: Vec<_> = (0..9)
            .map(|index| format!("recommendation-{index}"))
            .collect();
        let too_many_members = validate_rollup_clusters(
            &snapshots[..9],
            &[cluster(
                &too_many_members
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                TargetSurface::Tooling,
            )],
        );
        assert!(matches!(
            too_many_members,
            Err(RollupValidationError::TooManyMembers)
        ));
    }

    #[test]
    fn rejects_overlapping_families_and_mismatched_target_surface() {
        let snapshots = vec![
            snapshot("recommendation-a", TargetSurface::Tooling),
            snapshot("recommendation-b", TargetSurface::Tooling),
            snapshot("recommendation-c", TargetSurface::Test),
        ];
        let overlapping = validate_rollup_clusters(
            &snapshots,
            &[
                cluster(
                    &["recommendation-a", "recommendation-b"],
                    TargetSurface::Tooling,
                ),
                cluster(
                    &["recommendation-b", "recommendation-c"],
                    TargetSurface::Tooling,
                ),
            ],
        );
        assert!(matches!(
            overlapping,
            Err(RollupValidationError::OverlappingMember(_))
        ));

        let mismatched = validate_rollup_clusters(
            &snapshots[..2],
            &[cluster(
                &["recommendation-a", "recommendation-b"],
                TargetSurface::Test,
            )],
        );
        assert!(matches!(
            mismatched,
            Err(RollupValidationError::MismatchedTargetSurface)
        ));
    }

    #[test]
    fn rejects_duplicate_cluster_sets() {
        let snapshots = vec![
            snapshot("recommendation-a", TargetSurface::Tooling),
            snapshot("recommendation-b", TargetSurface::Tooling),
        ];
        let result = validate_rollup_clusters(
            &snapshots,
            &[
                cluster(
                    &["recommendation-a", "recommendation-b"],
                    TargetSurface::Tooling,
                ),
                cluster(
                    &["recommendation-b", "recommendation-a"],
                    TargetSurface::Tooling,
                ),
            ],
        );
        assert!(matches!(
            result,
            Err(RollupValidationError::DuplicateCluster)
        ));
    }

    #[test]
    fn rejects_generated_text_outside_bounds() {
        let snapshots = vec![
            snapshot("recommendation-a", TargetSurface::Tooling),
            snapshot("recommendation-b", TargetSurface::Tooling),
        ];
        for (title, summary) in [
            ("x".repeat(241), "ok".into()),
            ("ok".into(), "x".repeat(1_001)),
            ("bad\nline".into(), "ok".into()),
            ("ok".into(), "bad\u{1b}text".into()),
        ] {
            let result = validate_rollup_clusters(
                &snapshots,
                &[RollupCluster {
                    member_recommendation_ids: vec![
                        "recommendation-a".into(),
                        "recommendation-b".into(),
                    ],
                    target_surface: TargetSurface::Tooling,
                    title,
                    summary,
                }],
            );
            assert!(matches!(
                result,
                Err(RollupValidationError::GeneratedTextOutOfBounds { .. })
            ));
        }
    }

    #[test]
    fn snapshots_select_earliest_latest_and_highest_impact_distinct_evidence() {
        let mut recommendation = recommendation(
            "recommendation-a",
            TargetSurface::Tooling,
            (0..5)
                .map(|index| {
                    evidence(
                        &format!("observation-{index}"),
                        index + 1,
                        "session-a",
                        if index == 2 {
                            Impact::High
                        } else {
                            Impact::Low
                        },
                    )
                })
                .collect(),
        );
        recommendation.evidence.reverse();
        let snapshot = RollupFamilySnapshot::from_recommendation(&recommendation).unwrap();
        assert_eq!(snapshot.representative_evidence.len(), 3);
        assert_eq!(snapshot.representative_evidence[0].sequence, 1);
        assert_eq!(snapshot.representative_evidence[1].sequence, 5);
        assert_eq!(snapshot.representative_evidence[2].sequence, 3);
    }

    #[test]
    fn snapshots_and_parent_projection_obey_size_bounds() {
        let recommendations: Vec<_> = (0..33)
            .map(|index| {
                recommendation(
                    &format!("recommendation-{index}"),
                    TargetSurface::Tooling,
                    (0..100)
                        .map(|observation| {
                            evidence(
                                &format!("observation-{index}-{observation}"),
                                observation,
                                "session-a",
                                Impact::High,
                            )
                        })
                        .collect(),
                )
            })
            .collect();
        let snapshots = build_rollup_family_snapshots(&recommendations).unwrap();
        assert_eq!(snapshots.len(), 32);
        assert!(
            snapshots
                .iter()
                .map(|item| item.representative_evidence.len())
                .sum::<usize>()
                <= 96
        );
        assert!(snapshots
            .iter()
            .all(|item| item.source_session_ids.len() <= 8));
        assert!(serialized_size(&snapshots) <= MAX_ROLLUP_INPUT_BYTES);

        let evidence: Vec<_> = recommendations
            .iter()
            .flat_map(|item| item.evidence.clone())
            .collect();
        let projection = project_parent_evidence(&evidence);
        assert!(projection.len() <= MAX_PARENT_EVIDENCE_ENTRIES);
        assert!(serialized_size(&projection) <= MAX_PARENT_EVIDENCE_BYTES);
    }

    #[test]
    fn rotates_bounded_batches_without_splitting_active_parents() {
        let recommendations: Vec<_> = (0..40)
            .map(|index| {
                recommendation(
                    &format!("recommendation-{index:02}"),
                    TargetSurface::Tooling,
                    vec![evidence(
                        &format!("observation-{index}"),
                        index as u64,
                        "session-a",
                        Impact::Medium,
                    )],
                )
            })
            .collect();

        let first = build_rollup_family_snapshots_with_offset(&recommendations, 0).unwrap();
        let second = build_rollup_family_snapshots_with_offset(&recommendations, 32).unwrap();

        assert_eq!(first.len(), MAX_ROLLUP_FAMILIES);
        assert_eq!(second.len(), MAX_ROLLUP_FAMILIES);
        assert!(second
            .iter()
            .any(|snapshot| snapshot.recommendation_id == "recommendation-39"));
        assert_ne!(first, second);
    }

    #[test]
    fn rejects_whitespace_only_generated_text() {
        let snapshots = [
            snapshot("recommendation-a", TargetSurface::Tooling),
            snapshot("recommendation-b", TargetSurface::Tooling),
        ];
        let mut invalid = cluster(
            &["recommendation-a", "recommendation-b"],
            TargetSurface::Tooling,
        );
        invalid.title = "   ".into();

        assert!(matches!(
            validate_rollup_clusters(&snapshots, &[invalid]),
            Err(RollupValidationError::GeneratedTextOutOfBounds { field: "title", .. })
        ));
    }

    #[test]
    fn rejects_oversized_serialized_input() {
        let mut supplied = vec![snapshot("recommendation-a", TargetSurface::Tooling)];
        supplied[0].summary = "x".repeat(MAX_ROLLUP_INPUT_BYTES);
        let result = validate_rollup_clusters(&supplied, &[]);
        assert!(matches!(result, Err(RollupValidationError::InputTooLarge)));
    }

    #[test]
    fn rejects_oversized_serialized_response() {
        let result = validate_rollup_clusters(
            &[],
            &[RollupCluster {
                member_recommendation_ids: vec!["x".repeat(10_000); 8],
                target_surface: TargetSurface::Tooling,
                title: "Valid title".into(),
                summary: "Valid summary".into(),
            }],
        );
        assert!(matches!(
            result,
            Err(RollupValidationError::ResponseTooLarge)
        ));
    }

    #[test]
    fn rejects_excessive_supplied_families() {
        let supplied: Vec<_> = (0..=MAX_ROLLUP_FAMILIES)
            .map(|index| snapshot(&format!("recommendation-{index}"), TargetSurface::Tooling))
            .collect();
        let result = validate_rollup_clusters(&supplied, &[]);
        assert!(matches!(
            result,
            Err(RollupValidationError::TooManyFamilies)
        ));
    }

    #[test]
    fn rejects_excessive_supplied_representative_observations() {
        let mut supplied = vec![snapshot("recommendation-a", TargetSurface::Tooling)];
        supplied[0].representative_evidence = (0..=MAX_ROLLUP_OBSERVATIONS)
            .map(|index| {
                evidence(
                    &format!("observation-{index}"),
                    index as u64,
                    "session-a",
                    Impact::Low,
                )
            })
            .collect();
        let result = validate_rollup_clusters(&supplied, &[]);
        assert!(matches!(
            result,
            Err(RollupValidationError::TooManyObservations)
        ));
    }

    #[test]
    fn caps_representative_evidence_and_source_ids_per_family() {
        let recommendation = recommendation(
            "recommendation-a",
            TargetSurface::Tooling,
            (0..12)
                .map(|index| {
                    evidence(
                        &format!("observation-{index}"),
                        index as u64,
                        &format!("session-{index}"),
                        if index == 6 {
                            Impact::High
                        } else {
                            Impact::Low
                        },
                    )
                })
                .collect(),
        );
        let snapshot = RollupFamilySnapshot::from_recommendation(&recommendation).unwrap();
        assert_eq!(snapshot.representative_evidence.len(), 3);
        assert_eq!(
            snapshot.source_session_ids.len(),
            MAX_ROLLUP_SOURCE_SESSIONS
        );
    }
}
