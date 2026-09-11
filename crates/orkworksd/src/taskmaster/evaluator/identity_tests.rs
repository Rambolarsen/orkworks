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
        let runtime = TaskmasterRuntime::open(root.clone());
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
fn evaluation_identity_stale_output_cannot_change_current_error_status() {
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
    assert_eq!(
        fixture
            .runtime
            .status(Some(fixture.dir.path()))
            .last_error
            .as_deref(),
        Some("current diagnostic")
    );
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
    assert_eq!(
        fixture
            .runtime
            .status(Some(fixture.dir.path()))
            .last_error
            .as_deref(),
        Some("current diagnostic")
    );
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
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    document["overrides"]["codex"] =
        serde_json::json!({"inference":document["custom"][0]["inference"]});
    fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
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
