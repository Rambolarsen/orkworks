use super::coordinator::*;
use serde::Serialize;

#[test]
fn canonical_json_sorts_objects_but_preserves_arrays() {
    let left = serde_json::json!({"b": 2, "a": [3, 1]});
    let right = serde_json::json!({"a": [3, 1], "b": 2});
    let bytes = canonical_json_bytes(&left).unwrap();
    assert_eq!(bytes, br#"{"a":[3,1],"b":2}"#);
    assert_eq!(bytes, canonical_json_bytes(&right).unwrap());
    assert_eq!(
        sha256_hex(&bytes),
        sha256_hex(&canonical_json_bytes(&right).unwrap())
    );
    assert_ne!(
        bytes,
        canonical_json_bytes(&serde_json::json!({"a": [1, 3], "b": 2})).unwrap()
    );
}

#[test]
fn canonical_json_rejects_non_finite_numbers() {
    #[derive(Serialize)]
    struct NonFinite {
        value: f64,
    }
    assert!(canonical_json_bytes(&NonFinite { value: f64::NAN }).is_err());
}

fn plan() -> PlanRevision {
    PlanRevision {
        version: 1,
        instance_id: "instance-1".into(),
        workspace_id: "workspace-1".into(),
        plan_id: "plan-1".into(),
        revision: 1,
        revocation_generation: 0,
        supersedes: None,
        subject: WorkspaceChangeSubject {
            workspace_id: "workspace-1".into(),
            repository_revision: "abc123".into(),
            dirty_paths: vec!["src/lib.rs".into()],
            attribution: Attribution::Conclusive,
        },
        evidence: PlanEvidence {
            version: 1,
            subject: "issue-604".into(),
            references: vec!["spec".into()],
        },
        nodes: vec![PlanNode {
            id: "node-1".into(),
            parent_id: None,
            task: "implement domain".into(),
            success_criteria: vec!["tests pass".into()],
            prompt_context: "context".into(),
            prompt_context_digest: sha256_hex(b"context"),
            output_contract: "report".into(),
            scope: vec!["src/lib.rs".into()],
            role: Role::Implementation,
            allowed_tools: vec!["read_file".into()],
            provider_allowlist: vec!["codex".into()],
            dependencies: vec![],
            budget_units: 1,
            retry_limit: 0,
            concurrency_class: "default".into(),
        }],
        total_budget_units: 1,
        max_concurrency: 1,
        plan_digest: None,
        evidence_digest: None,
    }
}

fn approval(plan: &PlanRevision) -> PlanApproval {
    PlanApproval {
        version: 1,
        approval_id: "approval-1".into(),
        user_id: "user-1".into(),
        instance_id: plan.instance_id.clone(),
        workspace_id: plan.workspace_id.clone(),
        plan_id: plan.plan_id.clone(),
        revision: plan.revision,
        plan_digest: plan.compute_plan_digest().unwrap(),
        evidence_digest: plan.compute_evidence_digest().unwrap(),
        approved_at: "2026-09-24T00:00:00Z".into(),
        expires_at: "2099-01-01T00:00:00Z".into(),
        revocation_generation: 0,
        supersedes: None,
    }
}

#[test]
fn plan_rejects_supplied_digest_and_inconclusive_subject() {
    let mut value = plan();
    value.plan_digest = Some("0".repeat(64));
    assert!(value.validate().is_err());
    value.plan_digest = None;
    value.evidence_digest = Some("0".repeat(64));
    assert!(value.validate().is_err());
    value.evidence_digest = None;
    value.subject.attribution = Attribution::Inconclusive;
    assert!(value.validate().is_err());
}

#[test]
fn approved_plan_rejects_digest_or_scope_changes() {
    let original = plan();
    original.validate().unwrap();
    let approval = approval(&original);
    approval.validate_against(&original).unwrap();
    let mut changed = original.clone();
    changed.nodes[0].task.push('!');
    assert_ne!(
        original.compute_plan_digest().unwrap(),
        changed.compute_plan_digest().unwrap()
    );
    assert!(approval.validate_against(&changed).is_err());
    let mut changed = original.clone();
    changed.nodes[0].scope.push("another.rs".into());
    assert!(approval.validate_against(&changed).is_err());
    let mut changed = original.clone();
    changed.evidence.references.push("other".into());
    assert!(approval.validate_against(&changed).is_err());
    for field in ["instance", "workspace", "plan", "evidence", "generation"] {
        let mut wrong = approval.clone();
        match field {
            "instance" => wrong.instance_id = "other".into(),
            "workspace" => wrong.workspace_id = "other".into(),
            "plan" => wrong.plan_digest = "0".repeat(64),
            "evidence" => wrong.evidence_digest = "0".repeat(64),
            _ => wrong.revocation_generation = 1,
        }
        assert!(wrong.validate_against(&original).is_err(), "{field}");
    }
}

#[test]
fn lifecycle_is_explicit_and_review_is_nonterminal() {
    assert!(PlanStatus::Draft.allows_transition(PlanStatus::Proposed));
    assert!(PlanStatus::Proposed.allows_transition(PlanStatus::Active));
    assert!(PlanStatus::Active.allows_transition(PlanStatus::Completed));
    assert!(!PlanStatus::Active.allows_transition(PlanStatus::Proposed));
    assert!(!PlanStatus::ReadyForUserReview.is_terminal_success());
    for status in [
        PlanStatus::Expired,
        PlanStatus::Revoked,
        PlanStatus::Failed,
        PlanStatus::Cancelled,
        PlanStatus::RecoveryRequired,
    ] {
        assert!(!status.can_launch());
    }
}

#[test]
fn activation_requires_matching_approval_and_current_generation() {
    let original = plan();
    let valid = approval(&original);
    assert!(PlanStatus::Proposed
        .can_activate(&valid, &original, 0)
        .is_ok());
    assert!(PlanStatus::Draft
        .can_activate(&valid, &original, 0)
        .is_err());
    assert!(PlanStatus::Active
        .can_activate(&valid, &original, 0)
        .is_err());
    assert!(PlanStatus::Proposed
        .can_activate(&valid, &original, 1)
        .is_err());
    let mut expired = valid;
    expired.expires_at = "2020-01-01T00:00:00Z".into();
    assert!(PlanStatus::Proposed
        .can_activate(&expired, &original, 0)
        .is_err());
}

#[test]
fn plan_rejects_noncanonical_scope_and_unknown_node_fields() {
    let mut value = plan();
    value.nodes[0].scope = vec!["../outside".into()];
    assert!(value.validate().is_err());
    value.nodes[0].scope = vec!["src//lib.rs".into()];
    assert!(value.validate().is_err());
    let json = serde_json::to_value(&plan().nodes[0]).unwrap();
    let mut map = json.as_object().unwrap().clone();
    map.insert("unknown".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<PlanNode>(serde_json::Value::Object(map)).is_err());
}

#[test]
fn hard_denied_capabilities_never_become_grantable() {
    for kind in [
        CapabilityKind::GitMutation,
        CapabilityKind::Credentials,
        CapabilityKind::PermissionChange,
        CapabilityKind::DestructiveCommand,
        CapabilityKind::MergeApproval,
        CapabilityKind::ProviderSubstitution,
        CapabilityKind::ScopeExpansion,
        CapabilityKind::BudgetExpansion,
        CapabilityKind::RetryExpansion,
        CapabilityKind::ConcurrencyExpansion,
    ] {
        assert!(kind.is_hard_denied());
        assert!(CapabilityDecision::direct(kind, Role::Implementation).is_err());
        assert!(CapabilityDecision::direct(kind, Role::Custom("special".into())).is_err());
        let mut request = valid_request();
        request.kind = kind;
        assert!(CapabilityDecision::from_request(&request).is_err());
    }
}

fn valid_request() -> CapabilityRequest {
    CapabilityRequest {
        version: 1,
        node_id: "node-1".into(),
        contract_reference: "contract-1".into(),
        current_capability_digest: "a".repeat(64),
        kind: CapabilityKind::ReadFile,
        scope: vec!["src/lib.rs".into()],
        tool_id: "read_file".into(),
        expected_cost: 1,
        reason: "need to inspect source".into(),
        evidence: vec!["missing context".into()],
    }
}

#[test]
fn capability_request_keeps_structured_evidence_but_rejects_unknown_tools() {
    let request = valid_request();
    assert!(matches!(
        CapabilityDecision::from_request(&request),
        Ok(CapabilityDecision::RequiresParentReview)
    ));
    let mut bad = request.clone();
    bad.tool_id = "unknown_tool".into();
    assert!(CapabilityDecision::from_request(&bad).is_err());
    bad.tool_id = "x".repeat(300);
    assert!(CapabilityDecision::from_request(&bad).is_err());
}
