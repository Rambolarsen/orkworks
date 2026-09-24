use super::completion::{
    CompletionAction, CompletionActionRole, CompletionApproval, CompletionChangeSubject,
    CompletionEvidenceIssue, CompletionEvidenceIssueKind, CompletionEvidenceProvenance,
    CompletionPacket, CompletionReview, CompletionReviewOutcome, CompletionVerification,
    CompletionVerificationResult, PacketAttribution, PacketReadiness,
};

pub(crate) fn test_packet() -> CompletionPacket {
    let mut packet = CompletionPacket {
        schema_version: 1,
        evidence_version: 1,
        packet_id: "packet-1".into(),
        source_session_id: "session-source".into(),
        observed_at: "2026-09-24T10:00:00Z".into(),
        provenance: CompletionEvidenceProvenance {
            source_session_id: "session-source".into(),
            workspace_id: "workspace-1".into(),
            workspace_snapshot_id: "snapshot-1".into(),
            observed_at: "2026-09-24T10:00:00Z".into(),
        },
        subject: CompletionChangeSubject {
            workspace_id: "workspace-1".into(),
            scope: "src/taskmaster".into(),
            snapshot_id: "snapshot-1".into(),
            attribution: PacketAttribution::Unambiguous,
            changed_paths: vec!["src/taskmaster/mod.rs".into()],
        },
        verification: Some(CompletionVerification {
            command: "cargo test --manifest-path crates/orkworksd/Cargo.toml".into(),
            result: CompletionVerificationResult::Passed,
            applicable_revision: "snapshot-1".into(),
            observed_at: "2026-09-24T10:01:00Z".into(),
        }),
        review: None,
        missing_evidence: Vec::new(),
        conflicting_evidence: Vec::new(),
        action: CompletionAction {
            prompt: "Inspect the packet".into(),
            model: Some("review-model".into()),
            scope: "src/taskmaster".into(),
            role: CompletionActionRole::Review,
        },
        readiness: PacketReadiness::ReviewReady,
        revision: 1,
        evidence_fingerprint: String::new(),
        approval: None,
        supersedes_packet_id: None,
        lineage: Vec::new(),
        completion_idempotency_key: None,
    };
    packet.evidence_fingerprint = packet.computed_evidence_fingerprint();
    packet
}

#[test]
fn readiness_requires_verified_evidence_and_tracks_review_outcome() {
    let mut packet = test_packet();
    assert_eq!(packet.derived_readiness(), PacketReadiness::ReviewReady);

    packet.review = Some(CompletionReview {
        reviewer_session_id: "session-review".into(),
        reviewer_identity: "codex:gpt-5".into(),
        independent: true,
        outcome: CompletionReviewOutcome::NoFindings,
        applicable_revision: "snapshot-1".into(),
        observed_at: "2026-09-24T10:02:00Z".into(),
    });
    assert_eq!(
        packet.derived_readiness(),
        PacketReadiness::ReadyForUserReview
    );

    packet.review.as_mut().unwrap().outcome = CompletionReviewOutcome::Findings;
    assert_eq!(packet.derived_readiness(), PacketReadiness::FindingsNeedFix);

    packet.review.as_mut().unwrap().reviewer_session_id = packet.source_session_id.clone();
    packet.review.as_mut().unwrap().outcome = CompletionReviewOutcome::NoFindings;
    assert_eq!(
        packet.derived_readiness(),
        PacketReadiness::VerificationNeeded
    );
}

#[test]
fn inconclusive_attribution_cannot_claim_review_readiness() {
    let mut packet = test_packet();
    packet.subject.attribution = PacketAttribution::Inconclusive;
    assert_eq!(
        packet.derived_readiness(),
        PacketReadiness::VerificationNeeded
    );
}

#[test]
fn evidence_fingerprint_ignores_projection_and_approval_fields() {
    let first = test_packet();
    let mut second = first.clone();
    second.readiness = PacketReadiness::ReviewReady;
    second.revision = 7;
    second.approval = Some(CompletionApproval {
        approved_at: "2026-09-24T10:03:00Z".into(),
        approver: "user".into(),
        revision: 7,
        evidence_fingerprint: first.evidence_fingerprint.clone(),
        action_fingerprint: first.action_fingerprint(),
        idempotency_key: "approve-1".into(),
    });
    assert_eq!(
        first.computed_evidence_fingerprint(),
        second.computed_evidence_fingerprint()
    );
    first.validate("workspace-1").unwrap();
    second.evidence_fingerprint = second.computed_evidence_fingerprint();
    second.validate("workspace-1").unwrap();
}

#[test]
fn action_fingerprint_changes_for_prompt_model_scope_or_role_edits() {
    let packet = test_packet();
    let original = packet.action_fingerprint();
    for mutate in [
        |action: &mut CompletionAction| action.prompt.push('!'),
        |action: &mut CompletionAction| action.model = Some("other-model".into()),
        |action: &mut CompletionAction| action.scope.push_str("/extra"),
        |action: &mut CompletionAction| action.role = CompletionActionRole::Fix,
    ] {
        let mut changed = packet.clone();
        mutate(&mut changed.action);
        assert_ne!(original, changed.action_fingerprint());
    }
}

#[test]
fn validation_rejects_cross_workspace_and_tampered_evidence() {
    let mut cross_workspace = test_packet();
    cross_workspace.subject.workspace_id = "workspace-2".into();
    assert!(cross_workspace.validate("workspace-1").is_err());

    let mut missing_evidence = test_packet();
    missing_evidence
        .missing_evidence
        .push(CompletionEvidenceIssue {
            kind: CompletionEvidenceIssueKind::Missing,
            detail: "independent review".into(),
        });
    assert!(missing_evidence.validate("workspace-1").is_err());

    let mut forged = test_packet();
    forged.evidence_fingerprint = "forged".into();
    assert!(forged.validate("workspace-1").is_err());
}

#[test]
fn mutation_requests_require_a_sha256_fingerprint_and_safe_idempotency_key() {
    let packet = test_packet();
    let mut mutation = super::completion::CompletionMutationRequest {
        packet_revision: packet.revision,
        evidence_fingerprint: packet.evidence_fingerprint,
        idempotency_key: "completion-1".into(),
    };
    mutation.validate().unwrap();
    mutation.evidence_fingerprint = "not-a-fingerprint".into();
    assert!(mutation.validate().is_err());
    mutation.evidence_fingerprint = test_packet().evidence_fingerprint;
    mutation.idempotency_key = "contains whitespace".into();
    assert!(mutation.validate().is_err());
}
