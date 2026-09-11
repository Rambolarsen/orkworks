//! Exercise the production evaluator with local stores and a real fixture child.
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
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture {
    dir: tempfile::TempDir,
    state: Arc<AppState>,
    root: PathBuf,
    trust: InferenceTrustStore,
    marker: PathBuf,
    fallback_calls: Arc<AtomicUsize>,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("README.md"),
            "Document verification commands.",
        )
        .unwrap();
        let root = dir.path().join("runtime");
        fs::create_dir(&root).unwrap();
        let marker = root.join("prompt");
        let response = root.join("response.json");
        use sha2::Digest;
        let hash = hex::encode(sha2::Sha256::digest(b"Document verification commands."));
        let result = json!({"proposals":[{"targetSurface":"documentation",
            "title":"Document verification", "summary":"Experimental improvement",
            "repositoryFactHashes":[hash]}]})
        .to_string();
        fs::write(
            &response,
            if mode == "malformed" {
                "invalid envelope".into()
            } else {
                json!({"version":1,"status":"success","result":result}).to_string()
            },
        )
        .unwrap();
        let command = crate::test_support::native_inference::compile(&root);
        let harness_path = root.join("harnesses.json");
        fs::write(&harness_path, serde_json::to_vec(&json!({"version":3,"overrides":{},"custom":[{
            "id":"custom","name":"Custom fixture","launch":{"kind":"platform-shell","login":false},
            "inference":{"kind":"command","command":command,
                "args":["activation","{model}",marker,response,mode],"input":"stdin","output":"result-json-v1","timeoutSecs":10}
        }]})).unwrap()).unwrap();
        let mut state = crate::test_support::test_app_state_with_workspace(dir.path());
        let fallback_calls = Arc::new(AtomicUsize::new(0));
        Arc::get_mut(&mut state).unwrap().providers = crate::providers::ProviderManager::for_tests(
            crate::providers::ProviderSettingsPayload::default(),
            [
                "ollama",
                "codex",
                "claude-code",
                "copilot",
                "opencode",
                "aider",
            ]
            .into_iter()
            .map(|id| crate::providers::FakeProvider::new(id).with_counter(fallback_calls.clone()))
            .collect(),
        );
        Arc::get_mut(&mut state).unwrap().harness_store = Arc::new(HarnessStore::new(
            harness_path,
            Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
        ));
        let runtime = TaskmasterRuntime::open(root.clone());
        let mut settings = TaskmasterSettings::default();
        settings.enabled = true;
        settings.selection = Some(
            serde_json::from_value(json!({
            "provider":"custom","model":"vendor/opaque model;$(literal)"}))
            .unwrap(),
        );
        runtime.replace_settings(settings).unwrap();
        let trust = InferenceTrustStore::new(root.clone());
        Self {
            dir,
            state,
            root,
            trust,
            marker,
            fallback_calls,
        }
    }

    fn approve(&self) {
        let view = inference_approval::inspect_adapters(&self.state.harness_store, &self.trust)
            .unwrap()
            .pop()
            .unwrap();
        inference_approval::change_approval(
            &self.state.harness_store,
            &self.trust,
            ApprovalRequest {
                harness_id: view.id,
                action: ApprovalAction::Approve,
                expected_revision: view.revision,
            },
        )
        .unwrap();
    }

    fn run(&self) {
        run_model_evaluation_at(self.state.clone(), self.root.clone());
        assert_eq!(
            self.fallback_calls.load(Ordering::SeqCst),
            0,
            "custom selection must not invoke native or Peon alternatives"
        );
    }

    fn recommendations(&self) -> Vec<Recommendation> {
        self.state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .recommendation_store
            .list()
            .unwrap()
    }

    fn remaining(&self) -> u32 {
        TaskmasterRuntime::open(self.root.clone())
            .status(Some(self.dir.path()))
            .remaining_evaluations
    }
}

#[test]
fn unapproved_custom_evaluation_never_starts_a_child_or_spends_usage() {
    let fixture = Fixture::new("success");
    fixture.run();
    assert!(!fixture.marker.exists());
    assert_eq!(fixture.remaining(), 8);
    assert!(fixture.recommendations().is_empty());
}

#[test]
fn activation_requires_approval_then_persists_grounded_custom_output() {
    let fixture = Fixture::new("success");
    fixture.approve();
    fixture.run();
    assert_eq!(fixture.recommendations().len(), 1);
    assert_eq!(fixture.remaining(), 7);
    assert_eq!(
        fs::read_to_string(fixture.marker.with_extension("model")).unwrap(),
        "vendor/opaque model;$(literal)"
    );
    let prompt: serde_json::Value =
        serde_json::from_slice(&fs::read(&fixture.marker).unwrap()).unwrap();
    assert!(prompt["repositoryFacts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|fact| fact["path"] == "README.md"
            && fact["excerpt"] == "Document verification commands."));
    let recommendation = fixture.recommendations().pop().unwrap();
    assert!(recommendation.source_session_ids.is_empty());
    assert_eq!(recommendation.repository_evidence[0].path, "README.md");
}

#[test]
fn custom_evaluation_failures_spend_one_reservation_without_recommendations() {
    for mode in ["malformed", "nonzero"] {
        let fixture = Fixture::new(mode);
        fixture.approve();
        fixture.run();
        assert!(fixture.marker.exists(), "{mode}: fixture must have run");
        assert_eq!(fixture.remaining(), 7, "{mode}");
        assert!(fixture.recommendations().is_empty(), "{mode}");
        let runtime = TaskmasterRuntime::open(fixture.root.clone());
        let error = runtime.status(Some(fixture.dir.path())).last_error.unwrap();
        assert!(
            !error.contains("PRIVATE_PROVIDER_ERROR"),
            "stderr must be redacted"
        );
    }
}

#[test]
fn custom_evaluation_rechecks_readiness_before_context_and_reservation() {
    for change in [
        "unapproved",
        "revoke",
        "disabled",
        "effort",
        "unknown",
        "trust",
        "definition",
    ] {
        let fixture = Fixture::new("success");
        if change != "unapproved" {
            fixture.approve();
        }
        match change {
            "revoke" => fixture
                .trust
                .revoke("custom", fixture.trust.generation().unwrap())
                .unwrap(),
            "unapproved" => {}
            "disabled" | "effort" | "unknown" => {
                let runtime = TaskmasterRuntime::open(fixture.root.clone());
                let mut settings = runtime.status(None).settings;
                if change == "disabled" {
                    settings.enabled = false;
                } else if change == "unknown" {
                    settings.selection.as_mut().unwrap().provider = "unknown-provider".into();
                } else {
                    settings.selection.as_mut().unwrap().reasoning_effort = Some("high".into());
                }
                runtime.replace_settings(settings).unwrap();
            }
            "trust" => fs::write(fixture.root.join("inference-trust.json"), "corrupt").unwrap(),
            "definition" => {
                let path = fixture.root.join("harnesses.json");
                let mut document: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                document["custom"][0]["inference"]["timeoutSecs"] = json!(11);
                fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
            }
            _ => unreachable!(),
        }
        run_model_evaluation_with_context(
            fixture.state.clone(),
            fixture.root.clone(),
            |_, _, _, _| {
                panic!("{change}: must not collect repository context before readiness");
            },
        );
        assert_eq!(fixture.fallback_calls.load(Ordering::SeqCst), 0, "{change}");
        assert!(!fixture.marker.exists(), "{change}");
        assert_eq!(fixture.remaining(), 8, "{change}");
        assert!(fixture.recommendations().is_empty(), "{change}");
        assert!(
            TaskmasterRuntime::open(fixture.root.clone())
                .status(Some(fixture.dir.path()))
                .last_error
                .is_none(),
            "{change}"
        );
    }
}

#[test]
fn custom_evaluation_discards_output_revoked_while_child_is_running() {
    let fixture = Fixture::new("wait");
    fixture.approve();
    let state = fixture.state.clone();
    let root = fixture.root.clone();
    let evaluation = std::thread::spawn(move || run_model_evaluation_at(state, root));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !fixture.marker.exists()
        && !evaluation.is_finished()
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let started = fixture.marker.exists();
    fixture
        .trust
        .revoke("custom", fixture.trust.generation().unwrap())
        .unwrap();
    fs::write(fixture.marker.with_extension("release"), "release").unwrap();
    evaluation.join().unwrap();
    assert!(
        started,
        "fixture must reach the invocation before revocation"
    );
    assert_eq!(fixture.remaining(), 7);
    assert!(
        fixture.marker.with_extension("finished").exists(),
        "child must finish producing valid output, not merely time out"
    );
    assert!(fixture.recommendations().is_empty());
    fs::write(&fixture.marker, "not invoked again").unwrap();
    fixture.run();
    assert_eq!(
        fs::read_to_string(&fixture.marker).unwrap(),
        "not invoked again"
    );
    assert_eq!(fixture.remaining(), 7);
}

#[test]
fn custom_evaluation_recovers_after_reapproval_or_reenable() {
    for change in ["reapprove", "reenable"] {
        let fixture = Fixture::new("success");
        fixture.approve();
        let runtime = TaskmasterRuntime::open(fixture.root.clone());
        let settings = runtime.status(None).settings;
        if change == "reapprove" {
            fixture
                .trust
                .revoke("custom", fixture.trust.generation().unwrap())
                .unwrap();
        } else {
            let mut disabled = settings.clone();
            disabled.enabled = false;
            runtime.replace_settings(disabled).unwrap();
        }
        fixture.run();
        assert!(!fixture.marker.exists());
        assert_eq!(fixture.remaining(), 8);
        if change == "reapprove" {
            fixture.approve();
        } else {
            runtime.replace_settings(settings).unwrap();
        }
        fixture.run();
        assert!(fixture.marker.with_extension("finished").exists());
        assert_eq!(fixture.remaining(), 7);
        assert_eq!(fixture.recommendations().len(), 1);
    }
}
