use chrono::{DateTime, Duration, Utc};

#[allow(unused_imports)]
use crate::workflow_observations::{Impact, ObservationKind, ObservationSource};

#[allow(unused_imports)]
use super::{
    active_workflow_recommendation, evaluate_workflow_improvements, AuditCleanup,
    AuditCleanupEntry, AuditCriterion, Recommendation, RecommendationConfidence,
    RecommendationStatus, RecommendationType, TargetSurface, WorkflowImprovement,
    WorkflowObservationEvidence,
};

#[allow(unused_imports)]
pub(crate) const STALE_AFTER_DAYS: u32 = 14;

/// Precomputed per-dedupe-key context the classifier needs, built once by the
/// caller so `classify` stays pure and cheap.
#[allow(dead_code)]
pub(crate) struct FamilyContext {
    pub has_newer_proposed_sibling: bool,
    pub has_unextended_terminal_sibling: bool,
}

#[allow(dead_code)]
fn qualifies(evidence: &WorkflowObservationEvidence) -> bool {
    evidence.confidence >= 0.6
        && (evidence.reported_impact != Impact::High || evidence.confidence >= 0.8)
}

#[allow(dead_code)]
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
    let parsed: Vec<_> = recommendation
        .evidence
        .iter()
        .filter_map(|evidence| DateTime::parse_from_rfc3339(&evidence.observed_at).ok())
        .map(|parsed| parsed.with_timezone(&Utc))
        .collect();
    if let Some(oldest) = parsed.iter().min() {
        if now.signed_duration_since(*oldest) >= Duration::days(STALE_AFTER_DAYS.into()) {
            criteria.push(AuditCriterion::Stale);
        }
    }
    criteria
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::*;
    #[allow(unused_imports)]
    use crate::workflow_observations::ObservationSource;

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
    ) -> crate::workflow_observations::WorkflowObservation {
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

    fn evidence_card(evidence: Vec<WorkflowObservationEvidence>) -> Recommendation {
        super_test_recommendation(evidence)
    }

    fn super_test_recommendation(evidence: Vec<WorkflowObservationEvidence>) -> Recommendation {
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
        let card = evidence_card(vec![
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
        let card = evidence_card(vec![
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
        let card = evidence_card(vec![
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
            let card = evidence_card(vec![unscoped_peon(), peer.clone()]);
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
    fn stale_when_oldest_evidence_exceeds_the_window() {
        let card = evidence_card(vec![
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
        assert!(criteria.contains(&AuditCriterion::Stale));
    }

    #[test]
    fn not_stale_inside_the_window() {
        let card = evidence_card(vec![
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
    fn unparseable_or_empty_evidence_never_classifies_stale() {
        for evidence in [
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
                ObservationSource::Agent,
                Some("build"),
            ),
        ] {
            let card = evidence_card(vec![evidence]);
            assert_eq!(
                classify(
                    &card,
                    &FamilyContext {
                        has_newer_proposed_sibling: false,
                        has_unextended_terminal_sibling: false
                    },
                    Utc::now()
                ),
                vec![AuditCriterion::UnderEligible]
            );
        }
    }

    fn healthy_card() -> Recommendation {
        evidence_card(vec![
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
        let card = evidence_card(vec![
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
        let card = evidence_card(vec![
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

    #[test]
    fn cleanup_cards_do_not_block_brain_analyses() {
        let mut proposed = super_test_recommendation(Vec::new());
        proposed.recommendation_type = RecommendationType::Cleanup;
        proposed.dedupe_key = "taskmaster_audit:v1".into();
        proposed.audit = Some(AuditCleanup {
            entries: vec![AuditCleanupEntry {
                id: "sub".into(),
                title: "Sub card".into(),
                criteria: vec![AuditCriterion::Stale],
            }],
            scanned: 1,
            healthy: 0,
            stale_after_days: super::STALE_AFTER_DAYS,
        });
        assert!(active_workflow_recommendation(&[proposed]).is_none());
    }
}
