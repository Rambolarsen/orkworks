//! Final acceptance and cache tests use real local stores, never model calls.
use super::*;
use crate::harness::{
    definition::{BuiltinDocument, EMBEDDED_BUILTINS},
    store::HarnessStore,
};
use crate::taskmaster::{
    inference_approval::{self, ApprovalAction, ApprovalRequest},
    inference_trust::InferenceTrustStore,
    runtime::TaskmasterSettings,
};
use std::{fs, sync::Arc};

struct Fixture {
    dir: tempfile::TempDir,
    state: Arc<AppState>,
    runtime: TaskmasterRuntime,
    trust: InferenceTrustStore,
    snapshot: EvaluationSnapshot,
    facts: Vec<crate::taskmaster::RepositoryEvidence>,
    instance: u64,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("README.md"),
            "Document verification commands.",
        )
        .unwrap();
        let mut state = crate::test_support::test_app_state_with_workspace(dir.path());
        let harness_path = dir.path().join("harnesses.json");
        fs::write(
            &harness_path,
            serde_json::to_vec(&serde_json::json!({
                "version":3,"overrides":{},"custom":[{"id":"custom","name":"Custom",
                "launch":{"kind":"platform-shell","login":false},"inference":{
                    "kind":"command","command":std::env::current_exe().unwrap(),
                    "args":["{model}"],"input":"stdin","output":"result-json-v1"}}]
            }))
            .unwrap(),
        )
        .unwrap();
        Arc::get_mut(&mut state).unwrap().harness_store = Arc::new(HarnessStore::new(
            harness_path,
            Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
        ));
        let root = dir.path().join("runtime");
        let runtime = TaskmasterRuntime::open_fixture(root.clone());
        runtime
            .activate_knowledge(&crate::taskmaster::knowledge::test_support::fixture(
                "valid-activation.json",
            ))
            .unwrap();
        let mut settings = TaskmasterSettings::default();
        settings.enabled = true;
        settings.selection = Some(
            serde_json::from_value(serde_json::json!({
            "provider":"custom","model":"opaque/model"}))
            .unwrap(),
        );
        runtime.replace_settings(settings).unwrap();
        let trust = InferenceTrustStore::new(root);
        approve(&state, &trust);
        let mut snapshot = runtime.evaluation_snapshot(dir.path()).unwrap();
        snapshot.custom_inference = runtime
            .capture_custom_inference(&state.harness_store, dir.path(), &snapshot)
            .unwrap();
        assert!(snapshot.custom_inference.is_some());
        let facts = crate::taskmaster::context::collect_repository_facts(
            dir.path(),
            snapshot.settings.context_level,
            &[],
            "2026-09-11T00:00:00Z",
            "",
        )
        .unwrap();
        let instance = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .workflow_observations
            .instance_id();
        Self {
            dir,
            state,
            runtime,
            trust,
            snapshot,
            facts,
            instance,
        }
    }

    fn apply(&self) -> usize {
        let fact = self
            .facts
            .iter()
            .find(|fact| fact.path == "README.md")
            .unwrap();
        let output = serde_json::json!({"enrichments":[],"proposals":[{
            "targetSurface":"documentation","title":"Document verification",
            "summary":"Experimental improvement","repositoryFactHashes":[fact.sha256],
            "knowledgePageIds":[]}]})
        .to_string();
        apply_model_output(
            &self.state,
            &self.runtime,
            &self.snapshot,
            self.dir.path(),
            self.instance,
            &self.facts,
            &[],
            &output,
        );
        self.state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .list()
            .unwrap()
            .len()
    }
}

fn approve(state: &AppState, trust: &InferenceTrustStore) {
    let view = inference_approval::inspect_adapters(&state.harness_store, trust)
        .unwrap()
        .pop()
        .unwrap();
    inference_approval::change_approval(
        &state.harness_store,
        trust,
        ApprovalRequest {
            harness_id: view.id,
            action: ApprovalAction::Approve,
            expected_revision: view.revision,
        },
    )
    .unwrap();
}

#[test]
fn generated_proposals_reject_terminal_control_characters() {
    for field in ["title", "summary"] {
        for control in ['\r', '\n', '\u{1b}', '\u{0}', '\u{7f}'] {
            let fixture = Fixture::new();
            let mut proposal = serde_json::json!({
                "targetSurface":"documentation", "title":"Document verification",
                "summary":"Experimental improvement",
                "repositoryFactHashes":[fixture.facts[0].sha256], "knowledgePageIds":[]
            });
            proposal[field] = format!("first{control}second").into();
            apply_model_output(
                &fixture.state,
                &fixture.runtime,
                &fixture.snapshot,
                fixture.dir.path(),
                fixture.instance,
                &fixture.facts,
                &[],
                &serde_json::json!({"proposals":[proposal]}).to_string(),
            );
            assert!(
                fixture
                    .state
                    .workspace
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .recommendation_store
                    .list()
                    .unwrap()
                    .is_empty(),
                "accepted {field} {control:?}"
            );
        }
    }
}

#[test]
fn evaluation_identity_rejects_stale_custom_output_at_recommendation_commit() {
    for change in [
        "none",
        "revoke",
        "reapprove",
        "definition",
        "trust",
        "facts",
        "workspace",
        "snapshot",
        "missing_identity",
    ] {
        let mut fixture = Fixture::new();
        match change {
            "revoke" | "reapprove" => {
                fixture
                    .trust
                    .revoke("custom", fixture.trust.generation().unwrap())
                    .unwrap();
                if change == "reapprove" {
                    approve(&fixture.state, &fixture.trust);
                }
            }
            "definition" => {
                let path = fixture.dir.path().join("harnesses.json");
                let mut document: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                document["custom"][0]["inference"]["args"] =
                    serde_json::json!(["--changed", "{model}"]);
                fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
            }
            "trust" => {
                fs::write(
                    fixture.dir.path().join("runtime/inference-trust.json"),
                    "broken",
                )
                .unwrap();
            }
            "facts" => {
                fs::write(fixture.dir.path().join("README.md"), "Changed fact").unwrap();
            }
            "workspace" => {
                fixture.instance += 1;
            }
            "snapshot" => {
                fixture.snapshot.settings.selection.as_mut().unwrap().model =
                    "different-model".into();
            }
            "missing_identity" => {
                fixture.snapshot.custom_inference = None;
            }
            "none" => {}
            _ => unreachable!(),
        }
        assert_eq!(
            fixture.apply(),
            usize::from(change == "none"),
            "change={change}"
        );
    }
}

#[test]
fn evaluation_identity_cache_changes_on_reapproval_with_identical_prompt_and_model() {
    let fixture = Fixture::new();
    let first = fixture.snapshot.cache_key("same prompt").unwrap();
    assert_eq!(first, fixture.snapshot.cache_key("same prompt").unwrap());
    assert_ne!(first, fixture.snapshot.cache_key("changed prompt").unwrap());
    fixture
        .trust
        .revoke("custom", fixture.trust.generation().unwrap())
        .unwrap();
    approve(&fixture.state, &fixture.trust);
    let mut fresh = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    fresh.custom_inference = fixture
        .runtime
        .capture_custom_inference(&fixture.state.harness_store, fixture.dir.path(), &fresh)
        .unwrap();
    assert_ne!(first, fresh.cache_key("same prompt").unwrap());
}

#[test]
fn manual_evaluation_discards_a_request_after_workspace_switch() {
    let fixture = Fixture::new();
    let requested_workspace = fixture.dir.path().join("different-workspace");

    run_model_evaluation_with_context_and_workspace(
        fixture.state.clone(),
        Arc::new(TaskmasterRuntime::open_fixture(
            fixture.dir.path().join("runtime"),
        )),
        |_, _, _, _, _| panic!("repository context must not be collected for a stale workspace"),
        Some(requested_workspace),
        None,
        None,
    );
}

#[test]
fn scheduled_run_clears_queued_skip_and_records_context_failure() {
    let fixture = Fixture::new();
    let root = fixture.dir.path().join("runtime");
    let skipped = fixture
        .runtime
        .queue_run(
            fixture.dir.path(),
            crate::taskmaster::runtime::TaskmasterRunTrigger::Manual,
            "custom",
            "opaque/model",
        )
        .unwrap();
    run_model_evaluation_with_context_and_workspace(
        fixture.state.clone(),
        Arc::new(TaskmasterRuntime::open_fixture(root.clone())),
        |_, _, _, _, _| panic!("a stale manual request must not collect context"),
        Some(fixture.dir.path().join("different-workspace")),
        None,
        Some(ScheduledRun {
            id: skipped,
            workspace_path: fixture.dir.path().to_path_buf(),
            root: root.clone(),
        }),
    );
    let status = fixture
        .runtime
        .run_status(Some(fixture.dir.path()))
        .unwrap();
    assert_eq!(status.active_attempt, None);
    assert_eq!(status.latest_outcome, None);

    let failed = fixture
        .runtime
        .queue_run(
            fixture.dir.path(),
            crate::taskmaster::runtime::TaskmasterRunTrigger::Background,
            "custom",
            "opaque/model",
        )
        .unwrap();
    run_model_evaluation_with_context_and_workspace(
        fixture.state.clone(),
        Arc::new(TaskmasterRuntime::open_fixture(root.clone())),
        |_, _, _, _, _| Err("context collection failed".into()),
        None,
        None,
        Some(ScheduledRun {
            id: failed,
            workspace_path: fixture.dir.path().to_path_buf(),
            root,
        }),
    );
    let status = fixture
        .runtime
        .run_status(Some(fixture.dir.path()))
        .unwrap();
    assert_eq!(status.active_attempt, None);
    let outcome = status.latest_outcome.unwrap();
    assert_eq!(
        outcome.state,
        crate::taskmaster::runtime::TaskmasterRunOutcomeState::Failed
    );
    assert_eq!(
        outcome.error_summary.as_deref(),
        Some("context collection failed")
    );
}

#[test]
fn manual_dispatch_revalidation_rejects_a_workspace_replaced_after_admission() {
    let fixture = Fixture::new();
    let expected_path = fixture.dir.path().to_path_buf();
    let expected_instance = fixture.instance;
    let replacement = tempfile::tempdir().unwrap();
    SessionApplication::new(fixture.state.clone())
        .open_workspace(replacement.path().to_path_buf())
        .unwrap();

    let mut dispatched = false;
    let mut start = || {
        dispatched = true;
        Ok(())
    };
    assert!(manual_workspace_dispatch_gate(
        &fixture.state,
        &expected_path,
        expected_instance,
        &mut start,
    )
    .is_none());
    assert!(!dispatched);
}

#[test]
fn manual_dispatch_runs_its_start_action_while_workspace_is_locked() {
    let fixture = Fixture::new();
    let mut ran_under_lock = false;
    let mut start = || {
        ran_under_lock = fixture.state.workspace.try_lock().is_err();
        Ok(())
    };
    assert!(manual_workspace_dispatch_gate(
        &fixture.state,
        fixture.dir.path(),
        fixture.instance,
        &mut start,
    )
    .unwrap()
    .is_ok());

    assert!(ran_under_lock);
    assert!(fixture.state.workspace.try_lock().is_ok());
}

fn codex_snapshot(fixture: &Fixture) -> EvaluationSnapshot {
    let mut settings = fixture.runtime.status(None).settings;
    settings.selection.as_mut().unwrap().provider = "codex".into();
    fixture.runtime.replace_settings(settings).unwrap();
    let mut snapshot = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    let transport =
        super::super::provider_catalog::inspect(&fixture.state.harness_store, Some(&fixture.trust))
            .unwrap()
            .into_iter()
            .find(|provider| provider.id == "codex")
            .unwrap()
            .transport;
    assert!(bind_evaluation_transport(
        &fixture.runtime,
        &fixture.state.harness_store,
        fixture.dir.path(),
        &mut snapshot,
        transport,
    ));
    snapshot
}

#[test]
fn native_manual_dispatch_rejects_changed_settings_knowledge_and_harness() {
    for change in ["selection", "disabled", "knowledge", "harness"] {
        let fixture = Fixture::new();
        let snapshot = codex_snapshot(&fixture);
        match change {
            "selection" => {
                let mut settings = fixture.runtime.status(None).settings;
                settings.selection.as_mut().unwrap().provider = "claude-code".into();
                fixture.runtime.replace_settings(settings).unwrap();
            }
            "disabled" => {
                let mut settings = fixture.runtime.status(None).settings;
                settings.enabled = false;
                fixture.runtime.replace_settings(settings).unwrap();
            }
            "knowledge" => fixture
                .runtime
                .activate_fixture_bundle(crate::taskmaster::runtime::KnowledgeBundle {
                    format_version: 1,
                    version: "changed".into(),
                    sequence: 2,
                    published_at: "2026-09-26T00:00:00Z".into(),
                    pages: Vec::new(),
                })
                .unwrap(),
            "harness" => {
                let path = fixture.dir.path().join("harnesses.json");
                let mut document: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                document["overrides"]["codex"] = serde_json::json!({"inference":{
                    "kind":"command","command":"codex","args":["{model}"],
                    "input":"stdin","output":"result-json-v1"}});
                fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
            }
            _ => unreachable!(),
        }

        let mut dispatched = false;
        let mut start = || {
            dispatched = true;
            Ok(())
        };
        assert!(
            native_workspace_dispatch_gate(
                &fixture.state,
                &fixture.runtime,
                &snapshot,
                fixture.dir.path(),
                fixture.instance,
                true,
                &mut start,
            )
            .is_none(),
            "stale native {change} must reject provider dispatch"
        );
        assert!(!dispatched, "stale native {change} reached the provider");
    }
}

#[test]
fn native_manual_dispatch_keeps_snapshot_and_workspace_guards_through_start() {
    let fixture = Fixture::new();
    let snapshot = codex_snapshot(&fixture);
    let mut ran_under_workspace_lock = false;
    let mut start = || {
        ran_under_workspace_lock = fixture.state.workspace.try_lock().is_err();
        Ok(())
    };
    assert!(native_workspace_dispatch_gate(
        &fixture.state,
        &fixture.runtime,
        &snapshot,
        fixture.dir.path(),
        fixture.instance,
        true,
        &mut start,
    )
    .unwrap()
    .is_ok());
    assert!(ran_under_workspace_lock);
    assert!(fixture.state.workspace.try_lock().is_ok());
}

#[test]
fn manual_evaluation_rechecks_active_recommendations_at_its_admission_point() {
    let fixture = Fixture::new();
    let facts = fixture.facts.clone();
    let evidence_state = fixture.state.clone();
    let evidence_runtime = &fixture.runtime;
    let evidence_dir = fixture.dir.path().to_path_buf();
    let evidence_instance = fixture.instance;
    let evidence_snapshot = fixture.snapshot.clone();
    run_model_evaluation_with_context_and_workspace(
        fixture.state.clone(),
        Arc::new(TaskmasterRuntime::open_fixture(
            fixture.dir.path().join("runtime"),
        )),
        move |_, _, _, _, _| {
            let fact = facts.iter().find(|fact| fact.path == "README.md").unwrap();
            let output = serde_json::json!({"enrichments":[],"proposals":[{
                "targetSurface":"documentation","title":"Document verification",
                "summary":"Experimental improvement","repositoryFactHashes":[fact.sha256],
                "knowledgePageIds":[]}]})
            .to_string();
            apply_model_output(
                &evidence_state,
                &evidence_runtime,
                &evidence_snapshot,
                &evidence_dir,
                evidence_instance,
                &facts,
                &[],
                &output,
            );
            Ok(facts)
        },
        Some(fixture.dir.path().to_path_buf()),
        None,
        None,
    );

    let ledger: serde_json::Value = serde_json::from_slice(
        &fs::read(fixture.dir.path().join("runtime/evaluations.json")).unwrap(),
    )
    .unwrap();
    assert!(
        ledger["workspaceLastEvaluated"]
            .as_object()
            .is_none_or(|entries| entries.is_empty()),
        "the manual evaluator must not reserve after a recommendation becomes active"
    );
}

#[test]
fn evaluation_identity_custom_capture_failure_never_becomes_native() {
    let mut fixture = Fixture::new();
    assert!(bind_evaluation_transport(
        &fixture.runtime,
        &fixture.state.harness_store,
        fixture.dir.path(),
        &mut fixture.snapshot,
        super::super::provider_catalog::Transport::Custom
    ));
    assert!(fixture.snapshot.custom_inference.is_some());
    fixture
        .trust
        .revoke("custom", fixture.trust.generation().unwrap())
        .unwrap();
    assert!(!bind_evaluation_transport(
        &fixture.runtime,
        &fixture.state.harness_store,
        fixture.dir.path(),
        &mut fixture.snapshot,
        super::super::provider_catalog::Transport::Custom
    ));
    let path = fixture.dir.path().join("harnesses.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    document["custom"][0]["inference"] = serde_json::Value::Null;
    fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(!bind_evaluation_transport(
        &fixture.runtime,
        &fixture.state.harness_store,
        fixture.dir.path(),
        &mut fixture.snapshot,
        super::super::provider_catalog::Transport::Custom
    ));
}

#[test]
fn evaluation_identity_reservation_checks_trust_and_uses_bound_cache_key() {
    let fixture = Fixture::new();
    let key = fixture.snapshot.cache_key("prompt").unwrap();
    for missing in [false, true] {
        let mut invalid = fixture.snapshot.clone();
        if missing {
            invalid.custom_inference = None;
        } else {
            invalid.settings.selection.as_mut().unwrap().model = "different-model".into();
        }
        assert!(!fixture
            .runtime
            .reserve_snapshot(
                &fixture.state.harness_store,
                fixture.dir.path(),
                "2026-09-11T00:00:00Z",
                &key,
                &invalid
            )
            .unwrap());
    }
    fixture
        .trust
        .revoke("custom", fixture.trust.generation().unwrap())
        .unwrap();
    assert!(!fixture
        .runtime
        .reserve_snapshot(
            &fixture.state.harness_store,
            fixture.dir.path(),
            "2026-09-11T00:00:00Z",
            &key,
            &fixture.snapshot
        )
        .unwrap());
    let ledger: serde_json::Value = serde_json::from_slice(
        &fs::read(fixture.dir.path().join("runtime/evaluations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(ledger["reservations"], 0);
    approve(&fixture.state, &fixture.trust);
    let mut fresh = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    fresh.custom_inference = fixture
        .runtime
        .capture_custom_inference(&fixture.state.harness_store, fixture.dir.path(), &fresh)
        .unwrap();
    let fresh_key = fresh.cache_key("prompt").unwrap();
    assert!(fixture
        .runtime
        .reserve_snapshot(
            &fixture.state.harness_store,
            fixture.dir.path(),
            "2026-09-11T01:00:00Z",
            &fresh_key,
            &fresh
        )
        .unwrap());
    assert!(fixture
        .runtime
        .record_evaluation_success(
            &fixture.state.harness_store,
            fixture.dir.path(),
            &fresh,
            &fresh_key,
        )
        .unwrap());
    assert!(!fixture
        .runtime
        .reserve_snapshot(
            &fixture.state.harness_store,
            fixture.dir.path(),
            "2026-09-11T03:00:00Z",
            &fresh_key,
            &fresh
        )
        .unwrap());
    let ledger: serde_json::Value = serde_json::from_slice(
        &fs::read(fixture.dir.path().join("runtime/evaluations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(ledger["reservations"], 1);
}

#[test]
fn evaluation_identity_stale_output_does_not_write_legacy_global_error() {
    let fixture = Fixture::new();
    fixture
        .runtime
        .record_error(
            fixture.snapshot.generation,
            Some("current diagnostic".into()),
        )
        .unwrap();
    fixture
        .trust
        .revoke("custom", fixture.trust.generation().unwrap())
        .unwrap();
    assert_eq!(fixture.apply(), 0);
    assert!(fixture
        .runtime
        .status(Some(fixture.dir.path()))
        .last_error
        .is_none());
    let ledger: serde_json::Value = serde_json::from_slice(
        &fs::read(fixture.dir.path().join("runtime/evaluations.json")).unwrap(),
    )
    .unwrap();
    assert!(ledger.get("lastError").is_none());
    apply_model_output(
        &fixture.state,
        &fixture.runtime,
        &fixture.snapshot,
        fixture.dir.path(),
        fixture.instance,
        &fixture.facts,
        &[],
        "invalid JSON",
    );
    assert!(fixture
        .runtime
        .status(Some(fixture.dir.path()))
        .last_error
        .is_none());
}

#[test]
fn evaluation_identity_native_to_custom_change_invalidates_binding_reservation_and_commit() {
    let mut fixture = Fixture::new();
    let mut settings = fixture.snapshot.settings.clone();
    settings.selection.as_mut().unwrap().provider = "codex".into();
    fixture.runtime.replace_settings(settings).unwrap();
    fixture.snapshot = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    let transport =
        super::super::provider_catalog::inspect(&fixture.state.harness_store, Some(&fixture.trust))
            .unwrap()
            .into_iter()
            .find(|p| p.id == "codex")
            .unwrap()
            .transport;
    assert!(bind_evaluation_transport(
        &fixture.runtime,
        &fixture.state.harness_store,
        fixture.dir.path(),
        &mut fixture.snapshot,
        transport.clone()
    ));
    let key = fixture.snapshot.cache_key("prompt").unwrap();
    let mut changed = fixture.snapshot.clone();
    changed.generation += 1;
    assert_ne!(key, changed.cache_key("prompt").unwrap());
    let mut changed = fixture.snapshot.clone();
    changed.settings.selection.as_mut().unwrap().model = "another-model".into();
    assert_ne!(key, changed.cache_key("prompt").unwrap());
    let path = fixture.dir.path().join("harnesses.json");
    assert!(fixture
        .runtime
        .reserve_snapshot(
            &fixture.state.harness_store,
            fixture.dir.path(),
            "2026-09-11T00:00:00Z",
            &key,
            &fixture.snapshot
        )
        .unwrap());
    assert!(fixture
        .runtime
        .record_evaluation_success(
            &fixture.state.harness_store,
            fixture.dir.path(),
            &fixture.snapshot,
            &key
        )
        .unwrap());
    assert!(!fixture
        .runtime
        .reserve_snapshot(
            &fixture.state.harness_store,
            fixture.dir.path(),
            "2026-09-11T01:00:00Z",
            &key,
            &fixture.snapshot
        )
        .unwrap());
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    document["overrides"]["codex"] =
        serde_json::json!({"inference":document["custom"][0]["inference"]});
    fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(!fixture
        .runtime
        .record_evaluation_success(
            &fixture.state.harness_store,
            fixture.dir.path(),
            &fixture.snapshot,
            "stale-key"
        )
        .unwrap());
    assert!(!fixture
        .runtime
        .reserve_snapshot(
            &fixture.state.harness_store,
            fixture.dir.path(),
            "2026-09-11T01:00:00Z",
            &key,
            &fixture.snapshot
        )
        .unwrap());
    assert_eq!(fixture.apply(), 0);
    let mut new_snapshot = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    assert!(!bind_evaluation_transport(
        &fixture.runtime,
        &fixture.state.harness_store,
        fixture.dir.path(),
        &mut new_snapshot,
        transport
    ));
}

#[test]
fn same_generation_durable_corruption_blocks_all_native_and_custom_continuations() {
    for native in [false, true] {
        for corruption in ["missing", "malformed", "replaced"] {
            let mut fixture = Fixture::new();
            if native {
                fixture.snapshot = codex_snapshot(&fixture);
            }
            let snapshot = &fixture.snapshot;
            let generation = snapshot.generation;
            let key = snapshot.cache_key("same bounded prompt").unwrap();
            let path = fixture.runtime.knowledge_path();
            match corruption {
                "missing" => fs::remove_file(path).unwrap(),
                "malformed" => fs::write(path, b"broken").unwrap(),
                "replaced" => fs::write(
                    path,
                    crate::taskmaster::knowledge::test_support::fixture("sequence-zero.json"),
                )
                .unwrap(),
                _ => unreachable!(),
            }
            assert_eq!(
                fixture
                    .runtime
                    .evaluation_snapshot(fixture.dir.path())
                    .unwrap()
                    .generation,
                generation
            );
            assert!(!fixture
                .runtime
                .reserve_snapshot(
                    &fixture.state.harness_store,
                    fixture.dir.path(),
                    "2026-10-10T10:00:00Z",
                    &key,
                    snapshot
                )
                .unwrap());
            assert!(!fixture
                .runtime
                .reserve_manual_snapshot(
                    &fixture.state.harness_store,
                    fixture.dir.path(),
                    "2026-10-10T10:00:00Z",
                    &key,
                    snapshot
                )
                .unwrap());
            assert!(!fixture
                .runtime
                .with_current_evaluation(
                    &fixture.state.harness_store,
                    fixture.dir.path(),
                    snapshot,
                    || panic!("corrupt proof accepted")
                )
                .unwrap());
            assert!(!fixture
                .runtime
                .record_evaluation_success(
                    &fixture.state.harness_store,
                    fixture.dir.path(),
                    snapshot,
                    &key
                )
                .unwrap());
            let selection = snapshot.settings.selection.as_ref().unwrap();
            let token = crate::taskmaster::runtime::RollupEvaluationToken {
                workspace_instance: fixture.instance,
                generation,
                provider: selection.provider.clone(),
                model: selection.model.clone(),
                prompt_version: ROLLUP_PROMPT_VERSION.into(),
                family_snapshot_hash: "family".into(),
            };
            assert!(!fixture
                .runtime
                .with_current_rollup_evaluation(
                    &fixture.state.harness_store,
                    fixture.dir.path(),
                    snapshot,
                    &token,
                    || panic!("corrupt rollup accepted")
                )
                .unwrap());
            assert_eq!(fixture.apply(), 0);
            assert_eq!(
                fixture
                    .runtime
                    .status(Some(fixture.dir.path()))
                    .remaining_evaluations,
                8
            );
        }
    }
}

#[test]
fn full_proof_identity_survives_prompt_page_selection() {
    let mut fixture = Fixture::new();
    fixture.snapshot = codex_snapshot(&fixture);
    let mut prompt = fixture.snapshot.clone();
    prompt.knowledge.as_mut().unwrap().pages.clear();
    assert_eq!(
        prompt.knowledge_identity,
        fixture.snapshot.knowledge_identity
    );
    assert!(fixture
        .runtime
        .with_current_evaluation(
            &fixture.state.harness_store,
            fixture.dir.path(),
            &prompt,
            || {}
        )
        .unwrap());
    assert_eq!(
        fixture.snapshot.cache_key("prompt").unwrap(),
        prompt.cache_key("prompt").unwrap()
    );
}

#[test]
fn native_http_pending_poll_revalidates_signed_proof_before_each_poll() {
    use std::task::{Context, Poll, Wake, Waker};
    struct NoopWake;
    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }
    for change in ["corrupt", "configuration", "activation"] {
        let fixture = Fixture::new();
        let snapshot = codex_snapshot(&fixture);
        let polls = std::cell::Cell::new(0);
        let mut send = Box::pin(std::future::poll_fn(|_| {
            polls.set(polls.get() + 1);
            Poll::<Result<reqwest::Response, reqwest::Error>>::Pending
        }));
        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        let mut gate = |start: &mut dyn FnMut() -> std::io::Result<()>| {
            native_workspace_dispatch_gate(
                &fixture.state,
                &fixture.runtime,
                &snapshot,
                fixture.dir.path(),
                fixture.instance,
                true,
                start,
            )
        };
        assert!(matches!(
            crate::providers::poll_http_send_for_test(send.as_mut(), &mut context, &mut gate),
            Poll::Pending
        ));
        match change {
            "corrupt" => fs::write(fixture.runtime.knowledge_path(), b"broken").unwrap(),
            "configuration" => {
                let mut settings = fixture.runtime.status(None).settings;
                settings.enabled = false;
                fs::write(
                    fixture.runtime.root_path().join("settings.json"),
                    serde_json::to_vec(&settings).unwrap(),
                )
                .unwrap();
            }
            "activation" => {
                let mut bundle = crate::taskmaster::knowledge::test_support::bundle_value(
                    "valid-activation.json",
                );
                bundle["sequence"] = serde_json::json!(2);
                fixture
                    .runtime
                    .activate_knowledge(&crate::taskmaster::knowledge::test_support::activation(
                        bundle, false,
                    ))
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(matches!(
            crate::providers::poll_http_send_for_test(send.as_mut(), &mut context, &mut gate),
            Poll::Ready(Err(()))
        ));
        assert_eq!(
            polls.get(),
            1,
            "stale request must never be polled again for {change}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_http_endpoint_requires_current_signed_proof_and_discards_post_dispatch_corruption()
{
    use std::sync::atomic::{AtomicUsize, Ordering};
    for mode in ["valid", "before", "after"] {
        let mut fixture = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let proof_path = fixture.runtime.knowledge_path();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let router = axum::Router::new().route("/api/generate", axum::routing::post(move || {
            let count = count.clone(); let proof_path = proof_path.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                if mode == "after" { fs::write(proof_path, b"broken").unwrap(); }
                axum::Json(serde_json::json!({"response":"{\"enrichments\":[],\"proposals\":[]}","done":true}))
            }
        }));
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let mut settings = fixture.runtime.status(None).settings;
        settings.selection = Some(
            serde_json::from_value(
                serde_json::json!({"provider":"ollama","model":"fixture","ollamaBaseUrl":url}),
            )
            .unwrap(),
        );
        fixture.runtime.replace_settings(settings).unwrap();
        let mut snapshot = fixture
            .runtime
            .evaluation_snapshot(fixture.dir.path())
            .unwrap();
        let transport = super::super::provider_catalog::inspect(
            &fixture.state.harness_store,
            Some(&fixture.trust),
        )
        .unwrap()
        .into_iter()
        .find(|entry| entry.id == "ollama")
        .unwrap()
        .transport;
        assert!(bind_evaluation_transport(
            &fixture.runtime,
            &fixture.state.harness_store,
            fixture.dir.path(),
            &mut snapshot,
            transport
        ));
        fixture.snapshot = snapshot;
        let (fixture, result) = tokio::task::spawn_blocking(move || {
            let providers = crate::providers::ProviderManager::new();
            let snapshot = &fixture.snapshot;
            let selection = snapshot.settings.selection.as_ref().unwrap();
            let mut first = true;
            let mut gate = |start: &mut dyn FnMut() -> std::io::Result<()>| {
                if first && mode == "before" {
                    fs::write(fixture.runtime.knowledge_path(), b"broken").unwrap();
                }
                first = false;
                native_workspace_dispatch_gate(
                    &fixture.state,
                    &fixture.runtime,
                    snapshot,
                    fixture.dir.path(),
                    fixture.instance,
                    true,
                    start,
                )
            };
            let result = providers.invoke_native_taskmaster_prompt_with_dispatch_gate(
                snapshot.native_revision.as_ref().unwrap().profile,
                &selection.model,
                None,
                selection.ollama_base_url.as_deref(),
                "bounded fixture context".into(),
                &mut gate,
            );
            (fixture, result)
        })
        .await
        .unwrap();
        if mode == "valid" {
            assert!(result.is_ok(), "{result:?}");
            assert_eq!(fixture.apply(), 1);
        } else {
            assert!(result.is_err());
            assert_eq!(fixture.apply(), 0);
            assert!(!fixture
                .runtime
                .record_evaluation_success(
                    &fixture.state.harness_store,
                    fixture.dir.path(),
                    &fixture.snapshot,
                    "stale HTTP output"
                )
                .unwrap());
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            if mode == "before" { 0 } else { 1 }
        );
        server.abort();
    }
}

#[test]
fn native_process_marker_requires_current_signed_proof_at_actual_spawn() {
    let program_directory = tempfile::tempdir().unwrap();
    let executable = crate::test_support::native_inference::compile(program_directory.path());
    for mode in ["valid", "corrupt", "activation", "configuration"] {
        let fixture = Fixture::new();
        let snapshot = codex_snapshot(&fixture);
        let marker = fixture.dir.path().join("native-started");
        let response = fixture.dir.path().join("response");
        fs::write(&response, "fixture output").unwrap();
        let mut command = std::process::Command::new(&executable);
        command
            .args(["activation", "fixture"])
            .arg(&marker)
            .arg(&response)
            .arg("success");
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut gate = |start: &mut dyn FnMut() -> std::io::Result<()>| {
            match mode {
                "corrupt" => fs::write(fixture.runtime.knowledge_path(), b"broken").unwrap(),
                "activation" => {
                    let mut bundle = crate::taskmaster::knowledge::test_support::bundle_value(
                        "valid-activation.json",
                    );
                    bundle["sequence"] = serde_json::json!(2);
                    fixture
                        .runtime
                        .activate_knowledge(
                            &crate::taskmaster::knowledge::test_support::activation(bundle, false),
                        )
                        .unwrap();
                }
                "configuration" => {
                    let mut settings = fixture.runtime.status(None).settings;
                    settings.enabled = false;
                    fs::write(
                        fixture.runtime.root_path().join("settings.json"),
                        serde_json::to_vec(&settings).unwrap(),
                    )
                    .unwrap();
                }
                _ => {}
            }
            native_workspace_dispatch_gate(
                &fixture.state,
                &fixture.runtime,
                &snapshot,
                fixture.dir.path(),
                fixture.instance,
                true,
                start,
            )
        };
        let result = crate::providers::run_native_process_for_test(
            &mut command,
            "bounded context",
            &mut gate,
        );
        assert_eq!(result.is_ok(), mode == "valid", "{mode}: {result:?}");
        assert_eq!(marker.exists(), mode == "valid");
        if mode == "valid" {
            assert_eq!(fs::read_to_string(marker).unwrap(), "bounded context");
        }
    }
}

#[test]
fn native_process_output_is_not_accepted_after_proof_corruption_during_invocation() {
    let mut fixture = Fixture::new();
    fixture.snapshot = codex_snapshot(&fixture);
    let executable = crate::test_support::native_inference::compile(fixture.dir.path());
    let marker = fixture.dir.path().join("native-started");
    let response = fixture.dir.path().join("response");
    fs::write(&response, "fixture output").unwrap();
    let fixture = Arc::new(fixture);
    let running = fixture.clone();
    let child_marker = marker.clone();
    let invocation = std::thread::spawn(move || {
        let mut command = std::process::Command::new(executable);
        command
            .args(["activation", "fixture"])
            .arg(&child_marker)
            .arg(response)
            .arg("wait");
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut gate = |start: &mut dyn FnMut() -> std::io::Result<()>| {
            native_workspace_dispatch_gate(
                &running.state,
                &running.runtime,
                &running.snapshot,
                running.dir.path(),
                running.instance,
                true,
                start,
            )
        };
        crate::providers::run_native_process_for_test(&mut command, "bounded context", &mut gate)
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !marker.exists() && !invocation.is_finished() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let started = marker.exists();
    fs::write(fixture.runtime.knowledge_path(), b"broken").unwrap();
    fs::write(marker.with_extension("release"), b"release").unwrap();
    assert_eq!(invocation.join().unwrap().unwrap(), "fixture output");
    assert!(
        started,
        "native marker child must run before proof corruption"
    );
    assert_eq!(fixture.apply(), 0);
    assert!(!fixture
        .runtime
        .record_evaluation_success(
            &fixture.state.harness_store,
            fixture.dir.path(),
            &fixture.snapshot,
            "native output"
        )
        .unwrap());
}

#[test]
fn provider_output_counts_created_and_duplicate_suppressed_proposals() {
    let fixture = Fixture::new();
    let fact = fixture
        .facts
        .iter()
        .find(|fact| fact.path == "README.md")
        .unwrap();
    let output = serde_json::json!({
        "enrichments": [],
        "proposals": [{
            "targetSurface": "documentation",
            "title": "Document verification",
            "summary": "Describe the existing verification workflow.",
            "repositoryFactHashes": [fact.sha256],
            "knowledgePageIds": []
        }]
    })
    .to_string();

    let first = super::apply_provider_output_with_diagnostic(
        &fixture.state,
        &fixture.runtime,
        &fixture.snapshot,
        fixture.dir.path(),
        fixture.instance,
        &fixture.facts,
        &[],
        None,
        &output,
    )
    .unwrap();
    assert_eq!(first.proposal_counts.created, 1);
    assert_eq!(first.proposal_counts.suppressed, 0);

    let recommendations = fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .recommendation_store
        .list()
        .unwrap();
    let second = super::apply_provider_output_with_diagnostic(
        &fixture.state,
        &fixture.runtime,
        &fixture.snapshot,
        fixture.dir.path(),
        fixture.instance,
        &fixture.facts,
        &recommendations,
        None,
        &output,
    )
    .unwrap();
    assert_eq!(second.proposal_counts.created, 0);
    assert_eq!(second.proposal_counts.suppressed, 1);
}

#[test]
fn repeated_proposals_in_one_provider_response_count_once_as_created() {
    let fixture = Fixture::new();
    let fact = fixture
        .facts
        .iter()
        .find(|fact| fact.path == "README.md")
        .unwrap();
    let proposal = serde_json::json!({
        "targetSurface": "documentation",
        "title": "Document verification",
        "summary": "Describe the existing verification workflow.",
        "repositoryFactHashes": [fact.sha256],
        "knowledgePageIds": []
    });
    let output = serde_json::json!({
        "enrichments": [],
        "proposals": [proposal.clone(), proposal]
    })
    .to_string();

    let disposition = super::apply_provider_output_with_diagnostic(
        &fixture.state,
        &fixture.runtime,
        &fixture.snapshot,
        fixture.dir.path(),
        fixture.instance,
        &fixture.facts,
        &[],
        None,
        &output,
    )
    .unwrap();

    assert_eq!(disposition.proposal_counts.created, 1);
    assert_eq!(disposition.proposal_counts.suppressed, 1);
}
