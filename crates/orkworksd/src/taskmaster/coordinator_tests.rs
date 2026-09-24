use super::coordinator::*;
use super::coordinator_store::{CoordinatorStore, CoordinatorStoreError};
use chrono::{TimeZone, Utc};
use serde::Serialize;
use std::io::Write;

fn fixed_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 24, 12, 0, 0).unwrap()
}

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

fn plan() -> PlanProposalInput {
    PlanProposalInput {
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
    }
}

fn approved_plan(plan: &PlanProposalInput) -> PlanRevision {
    PlanRevision::from_proposal(plan.clone()).unwrap()
}

fn approval(plan: &PlanProposalInput) -> PlanApproval {
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

fn stored_path(root: &std::path::Path, revision: u64) -> std::path::PathBuf {
    root.join("coordinator/plans/plan-1")
        .join(format!("{revision}.json"))
}

#[test]
fn plan_ids_must_be_lowercase_to_avoid_case_insensitive_directory_aliases() {
    let mut input = plan();
    input.plan_id = "Plan-1".into();
    assert!(input.validate().is_err());
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    assert!(matches!(
        store.get("Plan-1", 1),
        Err(CoordinatorStoreError::Invalid(_))
    ));
}

#[test]
fn activation_rejects_approval_after_live_revocation_generation_advances() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let old = approval(&input);
    assert!(matches!(
        store.activate(&old, 1),
        Err(CoordinatorStoreError::Stale)
    ));
    assert_eq!(
        store.get("plan-1", 1).unwrap().unwrap().status,
        PlanStatus::Proposed
    );
}

#[test]
fn store_rejects_foreign_proposals_and_records_on_read_and_reopen() {
    for (instance_id, workspace_id) in
        [("instance-2", "workspace-1"), ("instance-1", "workspace-2")]
    {
        let root = tempfile::tempdir().unwrap();
        let expected =
            CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
        let mut foreign = plan();
        foreign.instance_id = instance_id.into();
        foreign.workspace_id = workspace_id.into();
        foreign.subject.workspace_id = workspace_id.into();
        assert!(matches!(
            expected.put_proposed(&approved_plan(&foreign)),
            Err(CoordinatorStoreError::Invalid(_))
        ));
        let foreign_store =
            CoordinatorStore::open(root.path().to_path_buf(), instance_id, workspace_id).unwrap();
        foreign_store
            .put_proposed(&approved_plan(&foreign))
            .unwrap();
        assert!(matches!(
            expected.get("plan-1", 1),
            Err(CoordinatorStoreError::Invalid(_))
        ));
        assert!(
            CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").is_err()
        );
    }
}

#[test]
fn paused_plan_resumes_only_with_renewed_current_approval() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let original = approval(&input);
    store.activate(&original, 0).unwrap();
    let digest = input.compute_plan_digest().unwrap();
    store
        .transition("plan-1", 1, &digest, PlanStatus::Paused)
        .unwrap();
    assert!(matches!(
        store.transition("plan-1", 1, &digest, PlanStatus::Active),
        Err(CoordinatorStoreError::Stale)
    ));
    assert!(matches!(
        store.resume(&original, 0),
        Err(CoordinatorStoreError::Stale)
    ));
    let mut renewed = original.clone();
    renewed.approval_id = "approval-2".into();
    renewed.approved_at = "2026-09-24T01:00:00Z".into();
    assert!(matches!(
        store.resume(&renewed, 1),
        Err(CoordinatorStoreError::Stale)
    ));
    assert_eq!(
        store.resume(&renewed, 0).unwrap().status,
        PlanStatus::Active
    );
    assert_eq!(
        store.get("plan-1", 1).unwrap().unwrap().approval,
        Some(renewed)
    );
}

#[test]
fn activation_retry_reflushes_record_after_post_publication_sync_failure() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let approval = approval(&input);
    store.fail_after_publication_once();
    assert!(matches!(
        store.activate(&approval, 0),
        Err(CoordinatorStoreError::Io(_))
    ));
    let retry = store.activate(&approval, 0).unwrap();
    assert_eq!(retry.status, PlanStatus::Active);
    assert_eq!(retry.approval, Some(approval));
}

#[test]
fn resume_retry_reflushes_published_approval_after_sync_failure() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let original = approval(&input);
    store.activate(&original, 0).unwrap();
    store
        .transition(
            "plan-1",
            1,
            &input.compute_plan_digest().unwrap(),
            PlanStatus::Paused,
        )
        .unwrap();
    let mut renewed = original;
    renewed.approval_id = "approval-2".into();
    renewed.approved_at = "2026-09-24T01:00:00Z".into();

    store.fail_after_publication_once();
    assert!(matches!(
        store.resume(&renewed, 0),
        Err(CoordinatorStoreError::Io(_))
    ));
    assert!(matches!(
        store.resume(&renewed, 1),
        Err(CoordinatorStoreError::Stale)
    ));
    let mut expired_retry = renewed.clone();
    expired_retry.expires_at = "2026-09-24T02:00:00Z".into();
    assert!(matches!(
        store.resume(&expired_retry, 0),
        Err(CoordinatorStoreError::Stale)
    ));
    let retry = store.resume(&renewed, 0).unwrap();
    assert_eq!(retry.status, PlanStatus::Active);
    assert_eq!(retry.approval, Some(renewed));
}

#[test]
fn resume_retry_waits_for_recovery_reflush_barrier() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let original = approval(&input);
    store.activate(&original, 0).unwrap();
    store
        .transition(
            "plan-1",
            1,
            &input.compute_plan_digest().unwrap(),
            PlanStatus::Paused,
        )
        .unwrap();
    let mut renewed = original;
    renewed.approval_id = "approval-2".into();
    renewed.approved_at = "2026-09-24T01:00:00Z".into();

    store.fail_after_publication_once();
    assert!(matches!(
        store.resume(&renewed, 0),
        Err(CoordinatorStoreError::Io(_))
    ));
    store.fail_recovery_reflush_once();
    assert!(matches!(
        store.resume(&renewed, 0),
        Err(CoordinatorStoreError::Io(_))
    ));
    assert_eq!(
        store.resume(&renewed, 0).unwrap().status,
        PlanStatus::Active
    );
}

#[test]
fn renewed_approvals_round_trip_as_immutable_history() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let original = approval(&input);
    store.activate(&original, 0).unwrap();
    let digest = input.compute_plan_digest().unwrap();
    store
        .transition("plan-1", 1, &digest, PlanStatus::Paused)
        .unwrap();
    let mut first_renewal = original.clone();
    first_renewal.approval_id = "approval-2".into();
    first_renewal.approved_at = "2026-09-24T01:00:00Z".into();
    store.resume(&first_renewal, 0).unwrap();
    store
        .transition("plan-1", 1, &digest, PlanStatus::Paused)
        .unwrap();
    let mut second_renewal = original.clone();
    second_renewal.approval_id = "approval-3".into();
    second_renewal.approved_at = "2026-09-24T02:00:00Z".into();
    store.resume(&second_renewal, 0).unwrap();

    let reopened =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let record = reopened.get("plan-1", 1).unwrap().unwrap();
    assert_eq!(record.approval, Some(second_renewal.clone()));
    let disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(stored_path(root.path(), 1)).unwrap()).unwrap();
    assert_eq!(
        disk["approval_history"],
        serde_json::json!([original, first_renewal, second_renewal])
    );
}

#[test]
fn legacy_record_without_history_gains_original_approval_on_resume() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let original = approval(&input);
    store.activate(&original, 0).unwrap();
    let path = stored_path(root.path(), 1);
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    legacy.as_object_mut().unwrap().remove("approval_history");
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let reopened =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    reopened
        .transition(
            "plan-1",
            1,
            &input.compute_plan_digest().unwrap(),
            PlanStatus::Paused,
        )
        .unwrap();
    let mut renewed = original.clone();
    renewed.approval_id = "approval-2".into();
    renewed.approved_at = "2026-09-24T01:00:00Z".into();
    reopened.resume(&renewed, 0).unwrap();
    let disk: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(
        disk["approval_history"],
        serde_json::json!([original, renewed])
    );
}

#[test]
fn expired_historical_approval_remains_readable_but_tampering_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let original = approval(&input);
    store.activate(&original, 0).unwrap();
    store
        .transition(
            "plan-1",
            1,
            &input.compute_plan_digest().unwrap(),
            PlanStatus::Paused,
        )
        .unwrap();
    let mut renewed = original;
    renewed.approval_id = "approval-2".into();
    renewed.approved_at = "2026-09-24T01:00:00Z".into();
    store.resume(&renewed, 0).unwrap();
    let path = stored_path(root.path(), 1);
    let mut disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    disk["approval_history"][0]["expires_at"] = "2026-09-24T00:30:00Z".into();
    std::fs::write(&path, serde_json::to_vec(&disk).unwrap()).unwrap();
    assert!(CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").is_ok());

    for mutation in ["plan_digest", "approved_at", "approval_id", "unknown_field"] {
        let mut changed = disk.clone();
        match mutation {
            "plan_digest" => changed["approval_history"][0]["plan_digest"] = "0".repeat(64).into(),
            "approved_at" => changed["approval_history"][0]["approved_at"] = "nonsense".into(),
            "approval_id" => changed["approval_history"][0]["approval_id"] = "approval-2".into(),
            _ => changed["approval_history"][0]["unknown_field"] = true.into(),
        }
        let bytes = serde_json::to_vec(&changed).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(
            CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn transition_retry_reflushes_record_after_post_publication_sync_failure() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    store.activate(&approval(&input), 0).unwrap();
    let digest = input.compute_plan_digest().unwrap();
    store.fail_after_publication_once();
    assert!(matches!(
        store.transition("plan-1", 1, &digest, PlanStatus::Paused),
        Err(CoordinatorStoreError::Io(_))
    ));
    assert_eq!(
        store
            .transition("plan-1", 1, &digest, PlanStatus::Paused)
            .unwrap()
            .status,
        PlanStatus::Paused
    );
}

#[test]
fn coordinator_store_rejects_stale_activation() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    let revision = approved_plan(&input);
    store.put_proposed(&revision).unwrap();
    let mut stale = approval(&input);
    stale.plan_digest = "0".repeat(64);
    assert!(matches!(
        store.activate(&stale, 0),
        Err(CoordinatorStoreError::Stale)
    ));
    assert_eq!(
        store.get("plan-1", 1).unwrap().unwrap().status,
        PlanStatus::Proposed
    );
    assert_eq!(
        store.activate(&approval(&input), 0).unwrap().status,
        PlanStatus::Active
    );
    assert_eq!(
        store.activate(&approval(&input), 0).unwrap().status,
        PlanStatus::Active
    );
}

#[test]
fn coordinator_store_round_trips_and_rejects_conflicting_revision() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    let revision = approved_plan(&input);
    store.put_proposed(&revision).unwrap();
    assert!(stored_path(root.path(), 1).exists());
    assert_eq!(store.get("missing", 1).unwrap(), None);
    let record = store.get("plan-1", 1).unwrap().unwrap();
    assert_eq!(record.plan, revision);
    let before = std::fs::read(stored_path(root.path(), 1)).unwrap();
    let mut other = input;
    other.nodes[0].task.push('!');
    assert!(matches!(
        store.put_proposed(&approved_plan(&other)),
        Err(CoordinatorStoreError::Stale)
    ));
    assert_eq!(std::fs::read(stored_path(root.path(), 1)).unwrap(), before);
}

#[test]
fn coordinator_store_rejects_corrupt_records_without_deleting_them() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    store.put_proposed(&approved_plan(&plan())).unwrap();
    let path = stored_path(root.path(), 1);
    let original = std::fs::read(&path).unwrap();
    for corrupt in [b"{bad json".to_vec(), vec![b' '; 2 * 1024 * 1024 + 8193], {
        let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        value["unknown"] = true.into();
        serde_json::to_vec(&value).unwrap()
    }] {
        std::fs::write(&path, &corrupt).unwrap();
        assert!(
            CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").is_err()
        );
        assert!(store.get("plan-1", 1).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
    }
}

#[test]
fn coordinator_store_rejects_inconsistent_approval_state() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    store.activate(&approval(&input), 0).unwrap();
    let path = stored_path(root.path(), 1);
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["status"] = "proposed".into();
    let corrupt = serde_json::to_vec(&value).unwrap();
    std::fs::write(&path, &corrupt).unwrap();
    assert!(
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").is_err()
    );
    assert!(store.activate(&approval(&input), 0).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), corrupt);
}

#[test]
fn coordinator_store_recovers_abandoned_temp_without_publishing_it() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    store.put_proposed(&approved_plan(&plan())).unwrap();
    let path = stored_path(root.path(), 1);
    let before = std::fs::read(&path).unwrap();
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, b"unfinished").unwrap();
    store.recover().unwrap();
    assert!(!temp.exists());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        store.get("plan-1", 1).unwrap().unwrap().status,
        PlanStatus::Proposed
    );
}

#[test]
fn coordinator_store_stale_transition_preserves_external_record() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let path = stored_path(root.path(), 1);
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["status"] = "recovery_required".into();
    let external = serde_json::to_vec(&value).unwrap();
    std::fs::write(&path, &external).unwrap();
    assert!(matches!(
        store.transition(
            "plan-1",
            1,
            &input.compute_plan_digest().unwrap(),
            PlanStatus::Active
        ),
        Err(CoordinatorStoreError::Stale)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), external);
}

#[test]
fn coordinator_store_superseded_approval_and_revoked_transition_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let first = plan();
    store.put_proposed(&approved_plan(&first)).unwrap();
    let mut second = first.clone();
    second.revision = 2;
    second.supersedes = Some("approval-1".into());
    store.put_proposed(&approved_plan(&second)).unwrap();
    let mut old = approval(&first);
    old.revision = 1;
    assert!(matches!(
        store.activate(&old, 0),
        Err(CoordinatorStoreError::Stale)
    ));
    let mut current = approval(&second);
    current.supersedes = second.supersedes.clone();
    store.activate(&current, 0).unwrap();
    let digest = second.compute_plan_digest().unwrap();
    store
        .transition("plan-1", 2, &digest, PlanStatus::Revoked)
        .unwrap();
    let before = std::fs::read(stored_path(root.path(), 2)).unwrap();
    assert!(matches!(
        store.transition("plan-1", 2, &digest, PlanStatus::Active),
        Err(CoordinatorStoreError::Stale)
    ));
    assert_eq!(std::fs::read(stored_path(root.path(), 2)).unwrap(), before);
}

#[test]
fn coordinator_store_rejects_superseded_progress_but_allows_cancellation() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let first = plan();
    store.put_proposed(&approved_plan(&first)).unwrap();
    store.activate(&approval(&first), 0).unwrap();
    let mut second = first.clone();
    second.revision = 2;
    second.supersedes = Some("approval-1".into());
    store.put_proposed(&approved_plan(&second)).unwrap();
    let path = stored_path(root.path(), 1);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        store.transition(
            "plan-1",
            1,
            &first.compute_plan_digest().unwrap(),
            PlanStatus::Completed
        ),
        Err(CoordinatorStoreError::Stale)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        store.get("plan-1", 1).unwrap().unwrap().status,
        PlanStatus::Active
    );
    assert_eq!(
        store
            .transition(
                "plan-1",
                1,
                &first.compute_plan_digest().unwrap(),
                PlanStatus::Cancelled
            )
            .unwrap()
            .status,
        PlanStatus::Cancelled
    );
}

#[test]
fn coordinator_store_rejects_expired_approval_progress_but_allows_expiry() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    store.activate(&approval(&input), 0).unwrap();
    let path = stored_path(root.path(), 1);
    let mut record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    record["approval"]["approved_at"] = "2019-01-01T00:00:00Z".into();
    record["approval"]["expires_at"] = "2020-01-01T00:00:00Z".into();
    record["approval_history"][0] = record["approval"].clone();
    let before = serde_json::to_vec(&record).unwrap();
    std::fs::write(&path, &before).unwrap();
    let digest = input.compute_plan_digest().unwrap();
    assert!(matches!(
        store.transition("plan-1", 1, &digest, PlanStatus::Completed),
        Err(CoordinatorStoreError::Stale)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        store.get("plan-1", 1).unwrap().unwrap().status,
        PlanStatus::Active
    );
    assert_eq!(
        store
            .transition("plan-1", 1, &digest, PlanStatus::Expired)
            .unwrap()
            .status,
        PlanStatus::Expired
    );
}

#[test]
fn coordinator_store_recovery_waits_for_external_writer_lease() {
    let root = tempfile::tempdir().unwrap();
    let first =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let second =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    first.put_proposed(&approved_plan(&plan())).unwrap();
    let temp = stored_path(root.path(), 1).with_extension("json.tmp");
    std::fs::write(&temp, b"incomplete publication").unwrap();
    let lease_path = root.path().join("coordinator/.coordinator.lock");
    let mut lease = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&lease_path)
        .unwrap();
    lease.write_all(b"").unwrap();
    fs2::FileExt::lock_exclusive(&lease).unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        done_tx.send(second.recover()).unwrap();
    });
    started_rx.recv().unwrap();
    assert!(done_rx
        .recv_timeout(std::time::Duration::from_millis(100))
        .is_err());
    assert!(temp.exists());
    fs2::FileExt::unlock(&lease).unwrap();
    assert!(done_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap()
        .is_ok());
    worker.join().unwrap();
    assert!(!temp.exists());
}

#[test]
fn coordinator_store_revalidates_after_temp_sync_before_publication() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    let input = plan();
    store.put_proposed(&approved_plan(&input)).unwrap();
    let path = stored_path(root.path(), 1);
    let original = std::fs::read(&path).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    value["status"] = "recovery_required".into();
    let external = serde_json::to_vec(&value).unwrap();
    let external_for_hook = external.clone();
    let path_for_hook = path.clone();
    store.set_before_publication_hook(Box::new(move || {
        std::fs::write(&path_for_hook, &external_for_hook).unwrap();
    }));
    assert!(matches!(
        store.activate(&approval(&input), 0),
        Err(CoordinatorStoreError::Stale)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), external);
}

#[test]
fn coordinator_store_rejects_duplicate_plan_and_node_fields_without_mutation() {
    let root = tempfile::tempdir().unwrap();
    let store =
        CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
    store.put_proposed(&approved_plan(&plan())).unwrap();
    let path = stored_path(root.path(), 1);
    let original = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
    for duplicate in [
        original.replacen(
            "\"plan_id\":\"plan-1\"",
            "\"plan_id\":\"plan-1\",\"plan_id\":\"plan-1\"",
            1,
        ),
        original.replacen(
            "\"task\":\"implement domain\"",
            "\"task\":\"implement domain\",\"task\":\"implement domain\"",
            1,
        ),
    ] {
        assert_ne!(duplicate, original);
        std::fs::write(&path, duplicate.as_bytes()).unwrap();
        assert!(
            CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").is_err()
        );
        assert!(store.get("plan-1", 1).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), duplicate.as_bytes());
    }
}

#[test]
fn coordinator_store_terminal_and_recovery_states_cannot_reactivate() {
    for terminal in [
        PlanStatus::Completed,
        PlanStatus::Failed,
        PlanStatus::Cancelled,
        PlanStatus::Revoked,
        PlanStatus::RecoveryRequired,
    ] {
        let root = tempfile::tempdir().unwrap();
        let store =
            CoordinatorStore::open(root.path().to_path_buf(), "instance-1", "workspace-1").unwrap();
        let input = plan();
        store.put_proposed(&approved_plan(&input)).unwrap();
        let approval = approval(&input);
        store.activate(&approval, 0).unwrap();
        let digest = input.compute_plan_digest().unwrap();
        store.transition("plan-1", 1, &digest, terminal).unwrap();
        let before = std::fs::read(stored_path(root.path(), 1)).unwrap();
        assert!(matches!(
            store.activate(&approval, 0),
            Err(CoordinatorStoreError::Stale)
        ));
        assert!(matches!(
            store.transition("plan-1", 1, &digest, PlanStatus::Active),
            Err(CoordinatorStoreError::Stale)
        ));
        assert_eq!(std::fs::read(stored_path(root.path(), 1)).unwrap(), before);
    }
}

#[test]
fn plan_rejects_supplied_digest_and_inconclusive_subject() {
    let mut value = plan();
    let mut json = serde_json::to_value(&value).unwrap();
    json["plan_digest"] = value.compute_plan_digest().unwrap().into();
    assert!(serde_json::from_value::<PlanProposalInput>(json).is_err());
    let mut json = serde_json::to_value(&value).unwrap();
    json["evidence_digest"] = value.compute_evidence_digest().unwrap().into();
    assert!(serde_json::from_value::<PlanProposalInput>(json).is_err());
    value.subject.attribution = Attribution::Inconclusive;
    assert!(value.validate().is_err());
}

#[test]
fn approved_plan_rejects_digest_or_scope_changes() {
    let original = plan();
    original.validate().unwrap();
    let approval = approval(&original);
    approval
        .validate_against(&approved_plan(&original))
        .unwrap();
    let mut changed = original.clone();
    changed.nodes[0].task.push('!');
    assert_ne!(
        original.compute_plan_digest().unwrap(),
        changed.compute_plan_digest().unwrap()
    );
    assert!(approval.validate_against(&approved_plan(&changed)).is_err());
    let mut changed = original.clone();
    changed.nodes[0].scope.push("another.rs".into());
    assert!(approval.validate_against(&approved_plan(&changed)).is_err());
    let mut changed = original.clone();
    changed.evidence.references.push("other".into());
    assert!(approval.validate_against(&approved_plan(&changed)).is_err());
    for field in ["instance", "workspace", "plan", "evidence", "generation"] {
        let mut wrong = approval.clone();
        match field {
            "instance" => wrong.instance_id = "other".into(),
            "workspace" => wrong.workspace_id = "other".into(),
            "plan" => wrong.plan_digest = "0".repeat(64),
            "evidence" => wrong.evidence_digest = "0".repeat(64),
            _ => wrong.revocation_generation = 1,
        }
        assert!(
            wrong.validate_against(&approved_plan(&original)).is_err(),
            "{field}"
        );
    }
}

#[test]
fn lifecycle_is_explicit_and_review_is_nonterminal() {
    assert!(PlanStatus::Draft.allows_transition(PlanStatus::Proposed));
    assert!(!PlanStatus::Proposed.allows_transition(PlanStatus::Active));
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
    let original = approved_plan(&original);
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

#[test]
fn matching_caller_digests_are_rejected_at_proposal_boundary() {
    let original = plan();
    let mut json = serde_json::to_value(&original).unwrap();
    json["plan_digest"] = original.compute_plan_digest().unwrap().into();
    json["evidence_digest"] = original.compute_evidence_digest().unwrap().into();
    assert!(serde_json::from_value::<PlanProposalInput>(json).is_err());
}

#[test]
fn multiline_prose_is_valid_but_control_bytes_are_not() {
    let mut value = plan();
    value.nodes[0].task = "Implement\nDomain".into();
    value.nodes[0].prompt_context = "First line\nSecond line\tindented".into();
    value.nodes[0].prompt_context_digest = sha256_hex(value.nodes[0].prompt_context.as_bytes());
    value.nodes[0].output_contract = "Summary\nTests\nRisks".into();
    assert!(value.validate().is_ok());
    value.nodes[0].output_contract.push('\0');
    assert!(value.validate().is_err());
}

#[test]
fn graph_rejects_self_reference_and_cycles() {
    let mut value = plan();
    value.nodes[0].parent_id = Some("node-1".into());
    assert!(value.validate().is_err());
    value.nodes[0].parent_id = None;
    value.nodes[0].dependencies = vec!["node-1".into()];
    assert!(value.validate().is_err());
    value.nodes[0].dependencies.clear();
    let mut second = value.nodes[0].clone();
    second.id = "node-2".into();
    value.nodes.push(second);
    value.total_budget_units = 2;
    value.nodes[0].parent_id = Some("node-2".into());
    value.nodes[1].dependencies = vec!["node-1".into()];
    assert!(value.validate().is_err());
    value.nodes[0].parent_id = None;
    value.nodes[0].dependencies = vec!["node-2".into()];
    assert!(value.validate().is_err());
}

#[test]
fn capability_kind_must_match_tool() {
    let mut request = valid_request();
    request.tool_id = "write_file".into();
    assert!(request.validate().is_err());
    request.kind = CapabilityKind::WriteFile;
    assert!(request.validate().is_ok());
}

#[test]
fn drive_and_network_scopes_are_rejected_on_every_host() {
    for path in [
        "C:/outside",
        "c:relative",
        "//server/share",
        "\\\\server\\share",
    ] {
        let mut value = plan();
        value.nodes[0].scope = vec![path.into()];
        assert!(value.validate().is_err(), "{path}");
    }
}

#[test]
fn approval_time_is_bounded_by_server_time() {
    let original = plan();
    let mut valid = approval(&original);
    let original = approved_plan(&original);
    assert!(valid.validate_against_at(&original, fixed_now()).is_ok());
    valid.approved_at = "2026-09-25T00:00:00Z".into();
    assert!(valid.validate_against_at(&original, fixed_now()).is_err());
    valid.approved_at = "2026-09-24T00:00:00Z".into();
    valid.expires_at = "2026-09-24T12:00:00Z".into();
    assert!(valid.validate_against_at(&original, fixed_now()).is_err());
}

#[test]
fn transition_table_has_one_approval_gated_entry_to_active() {
    use PlanStatus::*;
    let states = [
        Draft,
        Proposed,
        Active,
        ReadyForUserReview,
        Paused,
        RecoveryRequired,
        Completed,
        Failed,
        Cancelled,
        Expired,
        Revoked,
    ];
    let allowed = [
        (Draft, Proposed),
        (Proposed, Paused),
        (Proposed, RecoveryRequired),
        (Active, ReadyForUserReview),
        (Active, Paused),
        (Active, RecoveryRequired),
        (Active, Completed),
        (Active, Failed),
        (Active, Cancelled),
        (Active, Expired),
        (Active, Revoked),
        (ReadyForUserReview, Failed),
        (ReadyForUserReview, Cancelled),
        (ReadyForUserReview, Expired),
        (ReadyForUserReview, Revoked),
        (Paused, Cancelled),
        (Paused, Expired),
        (Paused, Revoked),
    ];
    for from in states {
        for to in states {
            assert_eq!(
                from.allows_transition(to),
                allowed.contains(&(from, to)),
                "{from:?} -> {to:?}"
            );
        }
    }
    let original = plan();
    let valid = approval(&original);
    let original = approved_plan(&original);
    assert!(Proposed
        .can_activate_at(&valid, &original, 0, fixed_now())
        .is_ok());
    for state in states {
        if state != Proposed {
            assert!(state
                .can_activate_at(&valid, &original, 0, fixed_now())
                .is_err());
        }
    }
}

#[test]
fn approved_revision_requires_new_revision_for_edits() {
    let original = plan();
    let approval = approval(&original);
    let active =
        ApprovedPlanRevision::activate(approved_plan(&original), approval, 0, fixed_now()).unwrap();
    let old_digest = active.plan_digest().to_owned();
    let mut draft = active.propose_revision().unwrap();
    draft.nodes[0].task = "revised task".into();
    let revised = PlanRevision::from_proposal(draft).unwrap();
    assert_eq!(active.plan_digest(), old_digest);
    assert_eq!(revised.revision(), active.revision() + 1);
    assert_ne!(revised.compute_plan_digest().unwrap(), old_digest);
}

#[test]
fn every_node_contract_field_is_required() {
    let node = serde_json::to_value(&plan().nodes[0]).unwrap();
    for field in [
        "id",
        "parent_id",
        "task",
        "success_criteria",
        "prompt_context",
        "prompt_context_digest",
        "output_contract",
        "scope",
        "role",
        "allowed_tools",
        "provider_allowlist",
        "dependencies",
        "budget_units",
        "retry_limit",
        "concurrency_class",
    ] {
        let mut missing = node.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<PlanNode>(missing).is_err(),
            "missing {field}"
        );
    }
}

#[test]
fn every_approved_plan_field_changes_digest_or_fails_validation() {
    let mut original = plan();
    let mut sibling = original.nodes[0].clone();
    sibling.id = "node-2".into();
    original.nodes.push(sibling);
    original.total_budget_units = 3;
    let base = original.compute_plan_digest().unwrap();
    for field in [
        "version",
        "instance_id",
        "workspace_id",
        "plan_id",
        "revision",
        "revocation_generation",
        "supersedes",
        "repository_revision",
        "dirty_paths",
        "attribution",
        "evidence_version",
        "evidence_subject",
        "evidence_references",
        "node_id",
        "parent_id",
        "task",
        "success_criteria",
        "prompt_context",
        "prompt_context_digest",
        "output_contract",
        "scope",
        "role",
        "allowed_tools",
        "provider_allowlist",
        "dependencies",
        "budget_units",
        "retry_limit",
        "concurrency_class",
        "total_budget_units",
        "max_concurrency",
    ] {
        let mut value = serde_json::to_value(&original).unwrap();
        match field {
            "version" => value["version"] = 2.into(),
            "instance_id" => value["instance_id"] = "other".into(),
            "workspace_id" => {
                value["workspace_id"] = "other".into();
                value["subject"]["workspace_id"] = "other".into();
            }
            "plan_id" => value["plan_id"] = "other".into(),
            "revision" => value["revision"] = 2.into(),
            "revocation_generation" => value["revocation_generation"] = 1.into(),
            "supersedes" => value["supersedes"] = "prior".into(),
            "repository_revision" => value["subject"]["repository_revision"] = "def456".into(),
            "dirty_paths" => value["subject"]["dirty_paths"] = serde_json::json!(["other.rs"]),
            "attribution" => value["subject"]["attribution"] = "inconclusive".into(),
            "evidence_version" => value["evidence"]["version"] = 2.into(),
            "evidence_subject" => value["evidence"]["subject"] = "other issue".into(),
            "evidence_references" => value["evidence"]["references"] = serde_json::json!(["other"]),
            "node_id" => value["nodes"][0]["id"] = "node-3".into(),
            "parent_id" => value["nodes"][0]["parent_id"] = "node-2".into(),
            "task" => value["nodes"][0]["task"] = "other task".into(),
            "success_criteria" => {
                value["nodes"][0]["success_criteria"] = serde_json::json!(["other success"])
            }
            "prompt_context" | "prompt_context_digest" => {
                value["nodes"][0]["prompt_context"] = "other context".into();
                value["nodes"][0]["prompt_context_digest"] = sha256_hex(b"other context").into();
            }
            "output_contract" => value["nodes"][0]["output_contract"] = "other report".into(),
            "scope" => value["nodes"][0]["scope"] = serde_json::json!(["other.rs"]),
            "role" => value["nodes"][0]["role"] = "review".into(),
            "allowed_tools" => {
                value["nodes"][0]["allowed_tools"] = serde_json::json!(["write_file"])
            }
            "provider_allowlist" => {
                value["nodes"][0]["provider_allowlist"] = serde_json::json!(["claude"])
            }
            "dependencies" => value["nodes"][0]["dependencies"] = serde_json::json!(["node-2"]),
            "budget_units" => value["nodes"][0]["budget_units"] = 2.into(),
            "retry_limit" => value["nodes"][0]["retry_limit"] = 1.into(),
            "concurrency_class" => value["nodes"][0]["concurrency_class"] = "serial".into(),
            "total_budget_units" => value["total_budget_units"] = 4.into(),
            "max_concurrency" => value["max_concurrency"] = 2.into(),
            _ => unreachable!(),
        }
        let changed: PlanProposalInput = serde_json::from_value(value).unwrap();
        match changed.compute_plan_digest() {
            Ok(digest) => assert_ne!(digest, base, "{field}"),
            Err(_) => assert!(
                matches!(field, "version" | "attribution" | "evidence_version"),
                "{field}"
            ),
        }
    }
}

#[test]
fn persisted_revision_round_trips_through_validated_decoder() {
    let original = approved_plan(&plan());
    let bytes = serde_json::to_vec(&original).unwrap();
    let loaded = PlanRevision::from_persisted_bytes(&bytes).unwrap();
    assert_eq!(loaded, original);
    assert_eq!(
        loaded.compute_plan_digest().unwrap(),
        original.compute_plan_digest().unwrap()
    );
    assert_eq!(
        loaded.compute_evidence_digest().unwrap(),
        original.compute_evidence_digest().unwrap()
    );
    assert!(serde_json::from_slice::<PlanProposalInput>(&bytes).is_err());
}

#[test]
fn persisted_revision_rejects_tampering_unknown_fields_and_bad_version() {
    let original = serde_json::to_value(approved_plan(&plan())).unwrap();
    for field in [
        "plan_digest",
        "evidence_digest",
        "nodes",
        "version",
        "unknown",
    ] {
        let mut changed = original.clone();
        match field {
            "plan_digest" | "evidence_digest" => changed[field] = "0".repeat(64).into(),
            "nodes" => changed["nodes"][0]["task"] = "tampered".into(),
            "version" => changed["version"] = 2.into(),
            _ => changed[field] = true.into(),
        }
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(
            PlanRevision::from_persisted_bytes(&bytes).is_err(),
            "{field}"
        );
    }
    for missing in ["plan_digest", "evidence_digest"] {
        let mut changed = original.clone();
        changed.as_object_mut().unwrap().remove(missing);
        assert!(
            PlanRevision::from_persisted_bytes(&serde_json::to_vec(&changed).unwrap()).is_err(),
            "missing {missing}"
        );
    }
}

fn many_nodes(count: usize) -> PlanProposalInput {
    let mut input = plan();
    let template = input.nodes[0].clone();
    input.nodes = (0..count)
        .map(|index| {
            let mut node = template.clone();
            node.id = format!("node-{index}");
            node.task = "x".repeat(16 * 1024);
            node
        })
        .collect();
    input.total_budget_units = count as u64;
    input
}

#[test]
fn valid_large_revision_round_trips_below_record_limit() {
    let revision = PlanRevision::from_proposal(many_nodes(100)).unwrap();
    let bytes = serde_json::to_vec(&revision).unwrap();
    assert!(bytes.len() > 1_000_000, "{}", bytes.len());
    assert!(bytes.len() <= 2 * 1024 * 1024, "{}", bytes.len());
    assert_eq!(
        PlanRevision::from_persisted_bytes(&bytes).unwrap(),
        revision
    );
}

#[test]
fn oversized_revision_is_rejected_at_creation() {
    let input = many_nodes(128);
    let encoded_proposal = serde_json::to_vec(&input).unwrap();
    assert!(
        encoded_proposal.len() > 2 * 1024 * 1024,
        "{}",
        encoded_proposal.len()
    );
    assert!(PlanRevision::from_proposal(input).is_err());
}
