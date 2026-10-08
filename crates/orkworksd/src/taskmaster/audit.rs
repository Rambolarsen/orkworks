use chrono::{DateTime, Duration, Utc};

use crate::workflow_observations::{Impact, ObservationSource};

use super::{
    stable_id, AuditCleanup, AuditCleanupEntry, AuditCriterion, Recommendation,
    RecommendationConfidence, RecommendationStatus, RecommendationType, TargetSurface,
    WorkflowImprovement, WorkflowObservationEvidence,
};

pub(crate) const STALE_AFTER_DAYS: u32 = 14;

/// Precomputed per-dedupe-key context the classifier needs, built once by the
/// caller so `classify` stays pure and cheap.
pub(crate) struct FamilyContext {
    pub has_newer_proposed_sibling: bool,
    pub has_unextended_terminal_sibling: bool,
}

fn qualifies(evidence: &WorkflowObservationEvidence) -> bool {
    evidence.confidence >= 0.6
        && (evidence.reported_impact != Impact::High || evidence.confidence >= 0.8)
}

pub(crate) fn classify(
    recommendation: &Recommendation,
    family: &FamilyContext,
    now: DateTime<Utc>,
) -> Vec<AuditCriterion> {
    let mut criteria = Vec::new();
    if !recommendation.rollup_member_ids.is_empty() {
        return criteria;
    }
    let qualifying_ids: std::collections::HashSet<&str> = recommendation
        .evidence
        .iter()
        .filter(|evidence| qualifies(evidence))
        .map(|evidence| evidence.observation_id.as_str())
        .collect();
    if qualifying_ids.len() < 2 {
        criteria.push(AuditCriterion::UnderEligible);
    }
    if !recommendation.evidence.is_empty()
        && recommendation.evidence.iter().all(|evidence| {
            evidence.source == ObservationSource::Peon && evidence.problem_area.is_none()
        })
    {
        criteria.push(AuditCriterion::Noise);
    }
    if family.has_newer_proposed_sibling || family.has_unextended_terminal_sibling {
        criteria.push(AuditCriterion::Duplicate);
    }
    let parseable: Option<Vec<_>> = recommendation
        .evidence
        .iter()
        .map(|evidence| {
            DateTime::parse_from_rfc3339(&evidence.observed_at)
                .ok()
                .map(|parsed| parsed.with_timezone(&Utc))
        })
        .collect();
    if let Some(newest) = parseable
        .as_deref()
        .and_then(|timestamps| timestamps.iter().max())
    {
        if now.signed_duration_since(*newest) > Duration::days(STALE_AFTER_DAYS.into()) {
            criteria.push(AuditCriterion::Stale);
        }
    }
    criteria
}

fn is_terminal(status: RecommendationStatus) -> bool {
    matches!(
        status,
        RecommendationStatus::Accepted
            | RecommendationStatus::Completed
            | RecommendationStatus::Dismissed
            | RecommendationStatus::Superseded
            | RecommendationStatus::Expired
            | RecommendationStatus::Failed
    )
}

fn supersedes_chain_contains(
    recommendations: &[Recommendation],
    start: &Recommendation,
    target: &str,
) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut cursor = start
        .workflow_improvement
        .supersedes_recommendation_id
        .clone();
    while let Some(id) = cursor {
        if id == target {
            return true;
        }
        if !visited.insert(id.clone()) {
            return false;
        }
        cursor = recommendations
            .iter()
            .find(|recommendation| recommendation.id == id)
            .and_then(|recommendation| {
                recommendation
                    .workflow_improvement
                    .supersedes_recommendation_id
                    .clone()
            });
    }
    false
}

/// Audits the workspace's proposed recommendations and, when any card is
/// under-eligible/noise/duplicate/stale, builds the cleanup card that
/// proposes dismissing them. Pure: persistence stays with the caller.
pub(crate) fn build_cleanup_card(
    recommendations: &[Recommendation],
    workspace_id: &str,
    now: &str,
    prior: Option<&Recommendation>,
) -> Option<Recommendation> {
    let now_parsed = DateTime::parse_from_rfc3339(now)
        .map(|parsed| parsed.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let proposed: Vec<&Recommendation> = recommendations
        .iter()
        .filter(|recommendation| {
            recommendation.status == RecommendationStatus::Proposed
                && recommendation.recommendation_type != RecommendationType::Cleanup
                && recommendation.rollup_member_ids.is_empty()
        })
        .collect();
    let mut entries = Vec::new();
    let mut under_eligible = 0;
    let mut noise = 0;
    let mut duplicate = 0;
    let mut stale = 0;
    for candidate in &proposed {
        let family = FamilyContext {
            has_newer_proposed_sibling: proposed.iter().any(|sibling| {
                sibling.id != candidate.id
                    && sibling.dedupe_key == candidate.dedupe_key
                    && (sibling.created_at.as_str(), sibling.id.as_str())
                        > (candidate.created_at.as_str(), candidate.id.as_str())
            }),
            has_unextended_terminal_sibling: recommendations.iter().any(|sibling| {
                sibling.dedupe_key == candidate.dedupe_key
                    && is_terminal(sibling.status)
                    && !supersedes_chain_contains(recommendations, candidate, &sibling.id)
            }),
        };
        let criteria = classify(candidate, &family, now_parsed);
        if criteria.is_empty() {
            continue;
        }
        for criterion in &criteria {
            match criterion {
                AuditCriterion::UnderEligible => under_eligible += 1,
                AuditCriterion::Noise => noise += 1,
                AuditCriterion::Duplicate => duplicate += 1,
                AuditCriterion::Stale => stale += 1,
            }
        }
        entries.push(AuditCleanupEntry {
            id: candidate.id.clone(),
            title: candidate.title.clone(),
            criteria,
        });
    }
    if entries.is_empty() {
        return None;
    }
    let healthy = proposed.len() - entries.len();
    let terminal_cleanup = recommendations
        .iter()
        .filter(|recommendation| {
            recommendation.recommendation_type == RecommendationType::Cleanup
                && is_terminal(recommendation.status)
        })
        .max_by(|left, right| {
            left.updated_at
                .cmp(&right.updated_at)
                .then(left.id.cmp(&right.id))
        });
    let (id, chain_depth, created_at, supersedes) = if let Some(prior) = prior {
        (
            prior.id.clone(),
            prior.chain_depth,
            prior.created_at.clone(),
            prior
                .workflow_improvement
                .supersedes_recommendation_id
                .clone(),
        )
    } else if let Some(terminal) = terminal_cleanup {
        (
            format!(
                "recommendation-{}",
                stable_id("cleanup:v1", &[terminal.id.clone()])
            ),
            terminal.chain_depth.saturating_add(1),
            now.to_string(),
            Some(terminal.id.clone()),
        )
    } else {
        (
            format!("recommendation-{}", stable_id("cleanup:v1", &[])),
            0,
            now.to_string(),
            None,
        )
    };
    let proposed_improvement = format!(
        "Dismiss {} proposed recommendations audited as under-eligible/noise/duplicate/stale.",
        entries.len()
    );
    Some(Recommendation {
        id,
        workspace_id: workspace_id.to_string(),
        chain_id: "cleanup:v1".into(),
        chain_depth,
        recommendation_type: RecommendationType::Cleanup,
        status: RecommendationStatus::Proposed,
        priority: Impact::Medium,
        title: "Recommendation cleanup".into(),
        summary: proposed_improvement.clone(),
        reason: vec![format!(
            "{} proposed scanned; {} healthy; criteria counts: under_eligible {}, noise {}, duplicate {}, stale {}.",
            proposed.len(),
            healthy,
            under_eligible,
            noise,
            duplicate,
            stale
        )],
        evidence: Vec::new(),
        repository_evidence: Vec::new(),
        knowledge_evidence: Vec::new(),
        source_session_ids: Vec::new(),
        target_session_id: None,
        suggested_harness_id: None,
        suggested_model: None,
        suggested_working_directory: None,
        suggested_prompt: None,
        confidence: RecommendationConfidence::Medium,
        requires_approval: true,
        dedupe_key: "cleanup:v1".into(),
        created_at,
        updated_at: now.to_string(),
        expires_at: None,
        workflow_improvement: WorkflowImprovement {
            proposed_improvement,
            target_surface: TargetSurface::Documentation,
            observation_ids: Vec::new(),
            recurrence_count: 0,
            affected_session_ids: Vec::new(),
            impact: Impact::Medium,
            expected_benefit: "Fewer stale cards, less noise in the recommendations panel.".into(),
            supersedes_recommendation_id: supersedes,
            dismissal_watermark: None,
        },
        completion_packet: None,
        audit: Some(AuditCleanup {
            entries,
            scanned: proposed.len(),
            healthy,
            stale_after_days: STALE_AFTER_DAYS,
        }),
        rollup_member_ids: Vec::new(),
        rollup_member_dedupe_keys: Vec::new(),
        rollup_generation: None,
        rolled_up_by: None,
        proposed_change: None,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use super::{build_cleanup_card, classify, stable_id, FamilyContext, STALE_AFTER_DAYS};
    use crate::taskmaster::{
        active_workflow_recommendation, evaluate_workflow_improvements, AuditCleanup,
        AuditCleanupEntry, AuditCriterion, Recommendation, RecommendationConfidence,
        RecommendationStatus, RecommendationType, TargetSurface, WorkflowImprovement,
        WorkflowObservationEvidence,
    };
    use crate::workflow_observations::{
        Impact, ObservationKind, ObservationSource, WorkflowObservation,
    };

    const WORKSPACE: &str = "workspace-1";
    const NOW: &str = "2026-10-08T12:00:00Z";
    const FINGERPRINT: &str = "v2:obstacle:the setup blocks progress";

    #[allow(clippy::too_many_arguments)]
    fn observation(
        id: &str,
        sequence: u64,
        session_id: &str,
        observed_at: &str,
        confidence: f64,
        impact: Impact,
        source: ObservationSource,
        problem_area: Option<&str>,
    ) -> WorkflowObservation {
        crate::workflow_observations::WorkflowObservation {
            id: id.into(),
            sequence,
            session_id: session_id.into(),
            observed_at: observed_at.into(),
            kind: ObservationKind::Obstacle,
            description: "The setup blocks progress".into(),
            evidence: format!("failure {id}"),
            problem_area: problem_area.map(Into::into),
            reported_impact: impact,
            source,
            confidence,
            fingerprint: FINGERPRINT.into(),
        }
    }

    fn proposed_card(evidence: Vec<WorkflowObservationEvidence>) -> Recommendation {
        let observation_ids: Vec<String> = evidence
            .iter()
            .map(|item| item.observation_id.clone())
            .collect();
        Recommendation {
            id: "recommendation-1".into(),
            workspace_id: WORKSPACE.into(),
            chain_id: "chain-1".into(),
            chain_depth: 0,
            recommendation_type: RecommendationType::ImproveWorkflow,
            status: RecommendationStatus::Proposed,
            priority: Impact::Medium,
            title: "Improve tooling".into(),
            summary: "Fix the thing".into(),
            reason: vec!["two qualifying observations".into()],
            evidence,
            repository_evidence: Vec::new(),
            knowledge_evidence: Vec::new(),
            source_session_ids: vec!["session-a".into()],
            target_session_id: None,
            suggested_harness_id: None,
            suggested_model: None,
            suggested_working_directory: None,
            suggested_prompt: None,
            confidence: RecommendationConfidence::Medium,
            requires_approval: false,
            dedupe_key: "improve_workflow:v1:tooling:v2:obstacle".into(),
            created_at: NOW.into(),
            updated_at: NOW.into(),
            expires_at: None,
            workflow_improvement: WorkflowImprovement {
                proposed_improvement: "Fix the thing".into(),
                target_surface: TargetSurface::Tooling,
                observation_ids,
                recurrence_count: 2,
                affected_session_ids: vec!["session-a".into()],
                impact: Impact::Medium,
                expected_benefit: "Fewer blockers".into(),
                supersedes_recommendation_id: None,
                dismissal_watermark: None,
            },
            completion_packet: None,
            audit: None,
            rollup_member_ids: Vec::new(),
            rollup_member_dedupe_keys: Vec::new(),
            rollup_generation: None,
            rolled_up_by: None,
            proposed_change: None,
        }
    }

    fn evidence(
        id: &str,
        observed_at: &str,
        confidence: f64,
        impact: Impact,
        source: ObservationSource,
        problem_area: Option<&str>,
    ) -> WorkflowObservationEvidence {
        WorkflowObservationEvidence {
            observation_id: id.into(),
            sequence: 1,
            session_id: "session-a".into(),
            kind: ObservationKind::Obstacle,
            description: "The setup blocks progress".into(),
            evidence: format!("failure {id}"),
            problem_area: problem_area.map(Into::into),
            reported_impact: impact,
            source,
            confidence,
            observed_at: observed_at.into(),
        }
    }

    fn recent() -> String {
        (Utc::now() - Duration::days(2)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    fn stale_observed_at(days_before_now: i64) -> String {
        (Utc::now() - Duration::days(days_before_now))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    #[test]
    fn under_eligible_fires_below_the_two_qualifying_citation_floor() {
        let card = proposed_card(vec![
            evidence(
                "one",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
            evidence(
                "two",
                &recent(),
                0.59,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert_eq!(criteria, vec![AuditCriterion::UnderEligible]);
    }

    #[test]
    fn not_under_eligible_when_two_citations_qualify() {
        let card = proposed_card(vec![
            evidence(
                "one",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert!(!criteria.contains(&AuditCriterion::UnderEligible));
    }

    #[test]
    fn noise_when_every_citation_is_unscoped_peon() {
        let card = proposed_card(vec![
            evidence(
                "one",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                None,
            ),
            evidence(
                "two",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                None,
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert!(criteria.contains(&AuditCriterion::Noise));
    }

    #[test]
    fn noise_excluded_when_any_citation_is_scoped_or_agent() {
        let unscoped_peon = || {
            evidence(
                "one",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                None,
            )
        };
        let scoped = evidence(
            "two",
            &recent(),
            0.8,
            Impact::Medium,
            ObservationSource::Peon,
            Some("build"),
        );
        let agent = evidence(
            "two",
            &recent(),
            0.8,
            Impact::Medium,
            ObservationSource::Agent,
            None,
        );
        for peer in [scoped, agent] {
            let card = proposed_card(vec![unscoped_peon(), peer.clone()]);
            let criteria = classify(
                &card,
                &FamilyContext {
                    has_newer_proposed_sibling: false,
                    has_unextended_terminal_sibling: false,
                },
                Utc::now(),
            );
            assert!(!criteria.contains(&AuditCriterion::Noise));
        }
    }

    #[test]
    fn duplicate_newer_sibling() {
        let card = healthy_card();
        let family = FamilyContext {
            has_newer_proposed_sibling: true,
            has_unextended_terminal_sibling: false,
        };
        assert!(classify(&card, &family, Utc::now()).contains(&AuditCriterion::Duplicate));
    }

    #[test]
    fn duplicate_unextended_terminal_sibling() {
        let card = healthy_card();
        let family = FamilyContext {
            has_newer_proposed_sibling: false,
            has_unextended_terminal_sibling: true,
        };
        assert!(classify(&card, &family, Utc::now()).contains(&AuditCriterion::Duplicate));
    }

    #[test]
    fn stale_when_newest_evidence_exceeds_the_window() {
        let stale_observed = stale_observed_at(20);
        let card = proposed_card(vec![
            evidence(
                "one",
                &stale_observed,
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                &stale_observed,
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert!(criteria.contains(&AuditCriterion::Stale));
    }

    #[test]
    fn mixed_freshness_card_with_a_recent_citation_is_not_stale() {
        let card = proposed_card(vec![
            evidence(
                "one",
                &stale_observed_at(20),
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                &stale_observed_at(2),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert!(!criteria.contains(&AuditCriterion::Stale));
    }

    #[test]
    fn stale_window_boundary_is_exclusive() {
        let now = Utc.with_ymd_and_hms(2026, 10, 8, 12, 0, 0).unwrap();
        let boundary_observed =
            (now - Duration::days(14)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let card = proposed_card(vec![
            evidence(
                "one",
                &boundary_observed,
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                &boundary_observed,
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            now,
        );
        assert!(!criteria.contains(&AuditCriterion::Stale));
    }

    #[test]
    fn not_stale_inside_the_window() {
        let card = proposed_card(vec![
            evidence(
                "one",
                &stale_observed_at(5),
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                &stale_observed_at(3),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert!(!criteria.contains(&AuditCriterion::Stale));
    }

    #[test]
    fn any_unparseable_timestamp_blocks_stale_classification() {
        let ancient_observed = stale_observed_at(20);
        let card = proposed_card(vec![
            evidence(
                "one",
                &ancient_observed,
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                "not-a-timestamp",
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert!(!criteria.contains(&AuditCriterion::Stale));
    }

    #[test]
    fn all_unparseable_timestamps_block_stale_classification() {
        let card = proposed_card(vec![
            evidence(
                "one",
                "not-a-timestamp",
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                "",
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert!(criteria.is_empty());
    }

    #[test]
    fn empty_evidence_never_classifies_stale() {
        let card = proposed_card(Vec::new());
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert!(!criteria.contains(&AuditCriterion::Stale));
    }

    #[test]
    fn two_citations_sharing_one_observation_id_count_once_for_the_floor() {
        let card = proposed_card(vec![
            evidence(
                "one",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "one",
                &recent(),
                0.9,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert_eq!(criteria, vec![AuditCriterion::UnderEligible]);
    }

    #[test]
    fn high_impact_below_point_eight_confidence_fails_qualification() {
        let card = proposed_card(vec![
            evidence(
                "one",
                &recent(),
                0.7,
                Impact::High,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert_eq!(criteria, vec![AuditCriterion::UnderEligible]);
    }

    #[test]
    fn high_impact_at_point_eight_confidence_qualifies() {
        let card = proposed_card(vec![
            evidence(
                "one",
                &recent(),
                0.8,
                Impact::High,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ]);
        let criteria = classify(
            &card,
            &FamilyContext {
                has_newer_proposed_sibling: false,
                has_unextended_terminal_sibling: false,
            },
            Utc::now(),
        );
        assert!(!criteria.contains(&AuditCriterion::UnderEligible));
    }

    fn healthy_card() -> Recommendation {
        proposed_card(vec![
            evidence(
                "one",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            evidence(
                "two",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ])
    }

    #[test]
    fn healthy_card_matches_nothing() {
        let card = proposed_card(vec![
            WorkflowObservationEvidence {
                sequence: 10,
                ..evidence(
                    "one",
                    &recent(),
                    0.8,
                    Impact::Medium,
                    ObservationSource::Agent,
                    Some("build"),
                )
            },
            WorkflowObservationEvidence {
                sequence: 11,
                ..evidence(
                    "two",
                    &recent(),
                    0.8,
                    Impact::Medium,
                    ObservationSource::Peon,
                    Some("build"),
                )
            },
        ]);
        assert_eq!(
            classify(
                &card,
                &FamilyContext {
                    has_newer_proposed_sibling: false,
                    has_unextended_terminal_sibling: false
                },
                Utc::now()
            ),
            Vec::<AuditCriterion>::new()
        );
    }

    #[test]
    fn criteria_multiply_in_canonical_order() {
        let card = proposed_card(vec![
            evidence(
                "one",
                &stale_observed_at(20),
                0.59,
                Impact::Medium,
                ObservationSource::Peon,
                None,
            ),
            evidence(
                "two",
                &stale_observed_at(20),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                None,
            ),
        ]);
        let family = FamilyContext {
            has_newer_proposed_sibling: true,
            has_unextended_terminal_sibling: false,
        };
        assert_eq!(
            classify(&card, &family, Utc::now()),
            vec![
                AuditCriterion::UnderEligible,
                AuditCriterion::Noise,
                AuditCriterion::Duplicate,
                AuditCriterion::Stale,
            ]
        );
    }

    #[test]
    fn rollup_parents_never_classified() {
        let mut card = healthy_card();
        card.status = RecommendationStatus::Proposed;
        card.rollup_member_ids = vec!["member-1".into(), "member-2".into()];
        assert_eq!(
            classify(
                &card,
                &FamilyContext {
                    has_newer_proposed_sibling: true,
                    has_unextended_terminal_sibling: true
                },
                Utc::now()
            ),
            Vec::<AuditCriterion>::new()
        );
    }

    #[test]
    fn mixed_qualifying_sources_build_a_healthy_card_through_the_evaluator() {
        let observations = vec![
            observation(
                "one",
                1,
                "session-a",
                &stale_observed_at(2),
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            ),
            observation(
                "two",
                2,
                "session-b",
                &stale_observed_at(1),
                0.8,
                Impact::Medium,
                ObservationSource::Peon,
                Some("build"),
            ),
        ];
        let cards = evaluate_workflow_improvements(&observations, &[], WORKSPACE, NOW);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].workflow_improvement.dismissal_watermark, None);
        let family = FamilyContext {
            has_newer_proposed_sibling: false,
            has_unextended_terminal_sibling: false,
        };
        assert!(classify(&cards[0], &family, Utc::now()).is_empty());
    }

    fn cleanup_proposed_card(dedupe_key: &str) -> Recommendation {
        let mut proposed = proposed_card(Vec::new());
        proposed.recommendation_type = RecommendationType::Cleanup;
        proposed.dedupe_key = dedupe_key.into();
        proposed.audit = Some(AuditCleanup {
            entries: vec![AuditCleanupEntry {
                id: "sub".into(),
                title: "Sub card".into(),
                criteria: vec![AuditCriterion::Stale],
            }],
            scanned: 1,
            healthy: 0,
            stale_after_days: STALE_AFTER_DAYS,
        });
        proposed
    }

    #[test]
    fn cleanup_cards_do_not_block_brain_analyses() {
        assert!(active_workflow_recommendation(&[cleanup_proposed_card("cleanup:v1")]).is_none());
    }

    #[test]
    fn cleanup_type_gate_survives_a_brain_dedupe_key() {
        assert!(
            active_workflow_recommendation(&[cleanup_proposed_card("proactive:v1:test")]).is_none()
        );
    }

    fn flagged(mut card: Recommendation, id: &str, dedupe_key: &str) -> Recommendation {
        card.id = id.into();
        card.dedupe_key = dedupe_key.into();
        card
    }

    #[test]
    fn builds_cleanup_card_with_entries_and_counts() {
        let under_eligible = flagged(
            proposed_card(Vec::new()),
            "card-under",
            "improve_workflow:v1:tooling:under",
        );
        let noise = flagged(
            proposed_card(vec![
                evidence(
                    "one",
                    &recent(),
                    0.8,
                    Impact::Medium,
                    ObservationSource::Peon,
                    None,
                ),
                evidence(
                    "two",
                    &recent(),
                    0.8,
                    Impact::Medium,
                    ObservationSource::Peon,
                    None,
                ),
            ]),
            "card-noise",
            "improve_workflow:v1:tooling:noise",
        );
        let healthy = flagged(
            healthy_card(),
            "card-healthy",
            "improve_workflow:v1:tooling:healthy",
        );
        let mut terminal = flagged(
            healthy_card(),
            "card-terminal",
            "improve_workflow:v1:tooling:terminal",
        );
        terminal.status = RecommendationStatus::Dismissed;

        let card = build_cleanup_card(
            &[under_eligible, noise, healthy, terminal],
            WORKSPACE,
            NOW,
            None,
        )
        .unwrap();

        let audit = card.audit.as_ref().unwrap();
        assert_eq!(audit.entries.len(), 2);
        assert_eq!(audit.entries[0].id, "card-under");
        assert_eq!(
            audit.entries[0].criteria,
            vec![AuditCriterion::UnderEligible]
        );
        assert_eq!(audit.entries[1].id, "card-noise");
        assert_eq!(audit.entries[1].criteria, vec![AuditCriterion::Noise]);
        assert_eq!(audit.scanned, 3);
        assert_eq!(audit.healthy, 1);
        assert_eq!(audit.stale_after_days, STALE_AFTER_DAYS);
        assert!(card.requires_approval);
        assert_eq!(card.priority, Impact::Medium);
        assert_eq!(card.confidence, RecommendationConfidence::Medium);
        assert!(card.evidence.is_empty());
        assert_eq!(card.workflow_improvement.recurrence_count, 0);
        assert_eq!(
            card.workflow_improvement.target_surface,
            TargetSurface::Documentation
        );
        assert!(card
            .workflow_improvement
            .proposed_improvement
            .starts_with("Dismiss 2 proposed recommendations"));
        assert_eq!(card.dedupe_key, "cleanup:v1");
        assert_eq!(card.chain_depth, 0);
        assert_eq!(
            card.id,
            format!("recommendation-{}", stable_id("cleanup:v1", &[]))
        );
        assert!(card.reason[0].contains("3 proposed scanned; 1 healthy;"));
        assert!(card.reason[0].contains("under_eligible 1, noise 1, duplicate 0, stale 0"));
    }

    #[test]
    fn replaces_in_place_keeping_prior_identity() {
        let mut prior = cleanup_proposed_card("cleanup:v1");
        prior.id = "cleanup-card-1".into();
        prior.chain_depth = 2;
        prior.created_at = "2026-10-01T00:00:00Z".into();
        prior.workflow_improvement.supersedes_recommendation_id = Some("cleanup-card-0".into());
        let under_eligible = flagged(
            proposed_card(Vec::new()),
            "card-under",
            "improve_workflow:v1:tooling:under",
        );

        let card = build_cleanup_card(&[under_eligible], WORKSPACE, NOW, Some(&prior)).unwrap();

        assert_eq!(card.id, "cleanup-card-1");
        assert_eq!(card.created_at, "2026-10-01T00:00:00Z");
        assert_eq!(card.updated_at, NOW);
        assert_eq!(card.chain_depth, 2);
        assert_eq!(
            card.workflow_improvement.supersedes_recommendation_id,
            Some("cleanup-card-0".into())
        );
        assert_eq!(card.audit.as_ref().unwrap().entries.len(), 1);
    }

    #[test]
    fn regenerates_fresh_card_after_terminal_prior() {
        let mut terminal = cleanup_proposed_card("cleanup:v1");
        terminal.id = "cleanup-card-1".into();
        terminal.status = RecommendationStatus::Dismissed;
        terminal.chain_depth = 3;
        let under_eligible = flagged(
            proposed_card(Vec::new()),
            "card-under",
            "improve_workflow:v1:tooling:under",
        );

        let card =
            build_cleanup_card(&[under_eligible, terminal.clone()], WORKSPACE, NOW, None).unwrap();

        assert_ne!(card.id, terminal.id);
        assert_eq!(card.created_at, NOW);
        assert_eq!(card.chain_depth, 4);
        assert_eq!(
            card.workflow_improvement.supersedes_recommendation_id,
            Some("cleanup-card-1".into())
        );
    }

    #[test]
    fn returns_none_when_every_proposed_card_is_healthy() {
        let healthy = flagged(
            healthy_card(),
            "card-healthy",
            "improve_workflow:v1:tooling:healthy",
        );
        let card = build_cleanup_card(&[healthy], WORKSPACE, NOW, None);
        assert!(card.is_none());
    }

    #[test]
    fn unextended_terminal_sibling_flags_duplicate_but_superseded_chain_does_not() {
        let mut terminal = flagged(
            proposed_card(vec![evidence(
                "one",
                &recent(),
                0.8,
                Impact::Medium,
                ObservationSource::Agent,
                Some("build"),
            )]),
            "card-terminal",
            "improve_workflow:v1:tooling:shared",
        );
        terminal.status = RecommendationStatus::Completed;
        let mut extended = flagged(
            healthy_card(),
            "card-extended",
            "improve_workflow:v1:tooling:shared",
        );
        extended.workflow_improvement.supersedes_recommendation_id = Some("card-terminal".into());
        let mut unextended = flagged(
            healthy_card(),
            "card-unextended",
            "improve_workflow:v1:tooling:shared",
        );
        unextended.created_at = "2026-10-07T00:00:00Z".into();

        let card = build_cleanup_card(
            &[terminal.clone(), extended, unextended],
            WORKSPACE,
            NOW,
            None,
        )
        .unwrap();

        let audit = card.audit.as_ref().unwrap();
        assert_eq!(audit.scanned, 2);
        assert_eq!(audit.entries.len(), 1);
        assert_eq!(audit.entries[0].id, "card-unextended");
        assert_eq!(audit.entries[0].criteria, vec![AuditCriterion::Duplicate]);
    }
}
