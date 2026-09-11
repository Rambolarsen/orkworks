use super::*;
use crate::harness::{
    definition::{BuiltinDocument, EMBEDDED_BUILTINS},
    store::HarnessStore,
};
use crate::taskmaster::runtime::{TaskmasterRuntime, TaskmasterSettings};
use crate::taskmaster::{
    inference_approval::{self, ApprovalAction, ApprovalRequest},
    inference_trust::InferenceTrustStore,
};
use serde_json::json;
use std::sync::Arc;

#[cfg(unix)]
mod spawn_tests;

struct Fixture {
    dir: tempfile::TempDir,
    runtime: TaskmasterRuntime,
    harnesses: HarnessStore,
    trust: InferenceTrustStore,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("harnesses.json"), serde_json::to_vec(&json!({"version":3,"overrides":{},"custom":[{
            "id":"custom", "name":"Custom", "launch":{"kind":"platform-shell","login":false},
            "inference":{"kind":"command","command":std::env::current_exe().unwrap(),"args":["{model}"],"input":"stdin","output":"result-json-v1"}
        }]})).unwrap()).unwrap();
        let harnesses = HarnessStore::new(
            dir.path().join("harnesses.json"),
            Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
        );
        let root = dir.path().join("runtime");
        let runtime = TaskmasterRuntime::open(root.clone());
        let mut settings = TaskmasterSettings::default();
        settings.enabled = true;
        settings.selection = Some(
            serde_json::from_value(json!({"provider":"custom","model":"opaque/model"})).unwrap(),
        );
        runtime.replace_settings(settings).unwrap();
        Self {
            dir,
            runtime,
            harnesses,
            trust: InferenceTrustStore::new(root),
        }
    }
    fn approve(&self) {
        let view = inference_approval::inspect_adapters(&self.harnesses, &self.trust)
            .unwrap()
            .pop()
            .unwrap();
        inference_approval::change_approval(
            &self.harnesses,
            &self.trust,
            ApprovalRequest {
                harness_id: view.id,
                action: ApprovalAction::Approve,
                expected_revision: view.revision,
            },
        )
        .unwrap();
    }
    fn capture(&self) -> CapturedInference {
        self.runtime
            .capture_custom_inference(
                &self.harnesses,
                self.dir.path(),
                &self.runtime.evaluation_snapshot(self.dir.path()).unwrap(),
            )
            .unwrap()
            .unwrap()
    }
}

#[test]
fn custom_action_rejects_revocation_and_reapproval_even_with_identical_definition() {
    let fixture = Fixture::new();
    let snapshot = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    assert!(fixture
        .runtime
        .capture_custom_inference(&fixture.harnesses, fixture.dir.path(), &snapshot)
        .unwrap()
        .is_none());
    fixture.approve();
    let captured = fixture.capture();
    let mut accepted = false;
    assert!(fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || accepted = true)
        .unwrap());
    assert!(accepted);
    fixture
        .trust
        .revoke("custom", fixture.trust.generation().unwrap())
        .unwrap();
    assert!(!fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!("revoked action"))
        .unwrap());
    fixture.approve();
    assert!(!fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!("stale generation"))
        .unwrap());
    assert_ne!(
        captured.cache_identity(),
        fixture.capture().cache_identity()
    );
}

#[test]
fn custom_action_rejects_definition_changes_and_corrupt_trust() {
    let fixture = Fixture::new();
    fixture.approve();
    let captured = fixture.capture();
    let path = fixture.dir.path().join("harnesses.json");
    let original = std::fs::read(&path).unwrap();
    let mut document: serde_json::Value = serde_json::from_slice(&original).unwrap();
    document["custom"][0]["inference"]["args"] = json!(["--changed", "{model}"]);
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(!fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!(
            "changed definition"
        ))
        .unwrap());
    std::fs::write(&path, original).unwrap();
    std::fs::write(
        fixture.dir.path().join("runtime/inference-trust.json"),
        b"malformed",
    )
    .unwrap();
    assert!(fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!("corrupt trust"))
        .is_err());
}

#[test]
fn custom_action_rejects_changed_settings_and_another_runtime_root() {
    let fixture = Fixture::new();
    fixture.approve();
    let captured = fixture.capture();
    let other = TaskmasterRuntime::open(fixture.dir.path().join("other-runtime"));
    assert!(!other
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!("wrong root"))
        .unwrap());
    let snapshot = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    let mut settings = snapshot.settings.clone();
    settings.selection.as_mut().unwrap().model = "changed-model".into();
    // A separate runtime view simulates a settings update from another request/process.
    TaskmasterRuntime::open(fixture.dir.path().join("runtime"))
        .replace_settings(settings)
        .unwrap();
    assert!(!fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!("changed settings"))
        .unwrap());
    assert!(fixture
        .runtime
        .capture_custom_inference(&fixture.harnesses, fixture.dir.path(), &snapshot)
        .unwrap()
        .is_none());
}

#[cfg(unix)]
#[test]
fn custom_action_rejects_changed_executable_resolution() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let fixture = Fixture::new();
    for name in ["first", "second"] {
        let path = fixture.dir.path().join(name);
        std::fs::write(&path, b"not executed").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let link = fixture.dir.path().join("adapter");
    symlink(fixture.dir.path().join("first"), &link).unwrap();
    let path = fixture.dir.path().join("harnesses.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    document["custom"][0]["inference"]["command"] = json!(link);
    std::fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
    fixture.approve();
    let captured = fixture.capture();
    std::fs::remove_file(&link).unwrap();
    symlink(fixture.dir.path().join("second"), &link).unwrap();
    assert!(!fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!(
            "changed resolution"
        ))
        .unwrap());
    fixture.approve();
    assert_ne!(
        captured.cache_identity().unwrap(),
        fixture.capture().cache_identity().unwrap()
    );
}

#[test]
fn custom_action_serializes_trust_and_harness_mutations_until_callback_returns() {
    use std::{
        sync::{mpsc, RwLock},
        time::Duration,
    };
    for change_trust in [true, false] {
        let fixture = Fixture::new();
        fixture.approve();
        let captured = fixture.capture();
        let trust_generation = fixture.trust.generation().unwrap();
        let catalog = Arc::new(RwLock::new(fixture.harnesses.load().unwrap().registry));
        let other_harnesses = HarnessStore::new(
            fixture.dir.path().join("harnesses.json"),
            Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
        );
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (started_tx, started_rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        std::thread::scope(|scope| {
            let fixture_ref = &fixture;
            let captured_ref = &captured;
            let action = scope.spawn(move || {
                fixture_ref.runtime.with_current_custom_inference(
                    &fixture_ref.harnesses,
                    captured_ref,
                    || {
                        entered_tx.send(()).unwrap();
                        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                    },
                )
            });
            entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let mutation = scope.spawn(|| {
                started_tx.send(()).unwrap();
                if change_trust {
                    fixture.trust.revoke("custom", trust_generation).unwrap();
                } else {
                    other_harnesses
                        .mutate(&catalog, |document| {
                            let mut capability = serde_json::to_value(
                                document.custom[0].inference.as_ref().unwrap(),
                            )
                            .unwrap();
                            capability["args"] = json!(["--changed", "{model}"]);
                            document.custom[0].inference =
                                Some(serde_json::from_value(capability).unwrap());
                            Ok(())
                        })
                        .unwrap();
                }
                finished_tx.send(()).unwrap();
            });
            started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let blocked = finished_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err();
            release_tx.send(()).unwrap();
            let applied = action.join().unwrap().unwrap();
            mutation.join().unwrap();
            assert!(blocked, "mutation crossed an active guarded callback");
            assert!(applied);
        });
        assert!(!fixture
            .runtime
            .with_current_custom_inference(&fixture.harnesses, &captured, || panic!(
                "mutation not observed"
            ))
            .unwrap());
    }
}

#[test]
fn custom_capture_binds_workspace_overrides_and_rejects_later_disablement() {
    use crate::taskmaster::runtime::{SelectionOverride, WorkspaceOverride};
    let fixture = Fixture::new();
    fixture.approve();
    let workspace = canonical_workspace_key(fixture.dir.path()).unwrap();
    let mut settings = fixture.runtime.status(None).settings;
    let mut selection = settings.selection.clone().unwrap();
    selection.model = "workspace-model".into();
    settings.workspace_overrides.insert(
        workspace.clone(),
        WorkspaceOverride {
            selection: SelectionOverride::Set(selection),
            ..Default::default()
        },
    );
    fixture.runtime.replace_settings(settings.clone()).unwrap();
    let captured = fixture.capture();
    let snapshot = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    let other_workspace = tempfile::tempdir().unwrap();
    assert!(fixture
        .runtime
        .capture_custom_inference(&fixture.harnesses, other_workspace.path(), &snapshot)
        .unwrap()
        .is_none());
    settings
        .workspace_overrides
        .get_mut(&workspace)
        .unwrap()
        .enabled = Some(false);
    TaskmasterRuntime::open(fixture.dir.path().join("runtime"))
        .replace_settings(settings)
        .unwrap();
    assert!(!fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!(
            "disabled workspace"
        ))
        .unwrap());
}

#[test]
fn custom_capture_fails_closed_for_missing_executable_and_corrupt_trust() {
    let fixture = Fixture::new();
    fixture.approve();
    let snapshot = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    let path = fixture.dir.path().join("harnesses.json");
    let original = std::fs::read(&path).unwrap();
    let mut document: serde_json::Value = serde_json::from_slice(&original).unwrap();
    document["custom"][0]["inference"]["command"] =
        json!(fixture.dir.path().join("missing-executable"));
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(fixture
        .runtime
        .capture_custom_inference(&fixture.harnesses, fixture.dir.path(), &snapshot)
        .unwrap()
        .is_none());
    std::fs::write(path, original).unwrap();
    std::fs::write(
        fixture.dir.path().join("runtime/inference-trust.json"),
        b"malformed",
    )
    .unwrap();
    assert!(fixture
        .runtime
        .capture_custom_inference(&fixture.harnesses, fixture.dir.path(), &snapshot)
        .is_err());
}

#[test]
fn retired_harness_cannot_reuse_a_historical_grant_at_capture() {
    let fixture = Fixture::new();
    let path = fixture.dir.path().join("harnesses.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let capability = document["custom"][0]["inference"].clone();
    document["overrides"]["gemini"] = json!({"inference":capability});
    std::fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
    let registry = fixture.harnesses.load().unwrap().registry;
    let harness = registry.get("gemini").unwrap();
    assert!(harness.definition.retired);
    // Model a grant saved before this coding tool was retired by an app update.
    let identity = AdapterIdentity::resolve(
        "gemini",
        harness.origin,
        harness.definition.inference.as_ref().unwrap(),
        None,
    )
    .unwrap();
    fixture
        .trust
        .approve(
            &identity,
            &fixture.trust.inspect(&identity).unwrap().revision,
        )
        .unwrap();
    let mut settings = fixture.runtime.status(None).settings;
    settings.selection.as_mut().unwrap().provider = "gemini".into();
    fixture.runtime.replace_settings(settings).unwrap();
    let snapshot = fixture
        .runtime
        .evaluation_snapshot(fixture.dir.path())
        .unwrap();
    assert!(fixture
        .runtime
        .capture_custom_inference(&fixture.harnesses, fixture.dir.path(), &snapshot)
        .unwrap()
        .is_none());
}

#[test]
fn custom_cache_identity_is_stable_and_covers_selection_workspace_and_definition() {
    let fixture = Fixture::new();
    fixture.approve();
    let captured = fixture.capture();
    let key = captured.cache_identity().unwrap();
    assert_eq!(key, fixture.capture().cache_identity().unwrap());
    // Vary one captured input at a time, without a generation change masking an omission.
    for field in ["model", "effort", "workspace", "definition"] {
        let mut changed = captured.clone();
        match field {
            "model" => changed.settings.selection.as_mut().unwrap().model = "other-model".into(),
            "effort" => {
                changed
                    .settings
                    .selection
                    .as_mut()
                    .unwrap()
                    .reasoning_effort = Some("high".into())
            }
            "workspace" => changed.workspace.push_str("/different"),
            "definition" => {
                let mut raw = serde_json::to_value(&changed.capability).unwrap();
                raw["args"] = json!(["--changed", "{model}"]);
                changed.capability = serde_json::from_value(raw).unwrap();
            }
            _ => unreachable!(),
        }
        assert_ne!(key, changed.cache_identity().unwrap(), "omitted {field}");
    }
}

#[test]
#[ignore = "subprocess helper for cross-process harness mutation exclusion"]
fn harness_mutation_child() {
    use std::sync::RwLock;
    let root = PathBuf::from(std::env::var_os("ORKWORKS_TEST_GUARD_ROOT").unwrap());
    let store = HarnessStore::new(
        root.join("harnesses.json"),
        Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
    );
    let catalog = Arc::new(RwLock::new(store.load().unwrap().registry));
    std::fs::write(root.join("child-ready"), b"ready").unwrap();
    store
        .mutate(&catalog, |document| {
            document.custom[0].name = "Changed by child".into();
            Ok(())
        })
        .unwrap();
}

#[test]
fn custom_action_excludes_harness_mutation_from_another_process() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let fixture = Fixture::new();
    fixture.approve();
    let captured = fixture.capture();
    let mut child = None;
    let mut saw_ready = false;
    let mut blocked = false;
    let result =
        fixture
            .runtime
            .with_current_custom_inference(&fixture.harnesses, &captured, || {
                let process = Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "taskmaster::runtime::inference::tests::harness_mutation_child",
                        "--ignored",
                    ])
                    .env("ORKWORKS_TEST_GUARD_ROOT", fixture.dir.path())
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap();
                child = Some(process);
                let deadline = Instant::now() + Duration::from_secs(10);
                while !fixture.dir.path().join("child-ready").exists() && Instant::now() < deadline
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                saw_ready = fixture.dir.path().join("child-ready").exists();
                std::thread::sleep(Duration::from_millis(100));
                blocked = child.as_mut().unwrap().try_wait().unwrap().is_none();
            });
    let mut child = child.expect("guarded action should spawn the local test helper");
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= deadline {
            break None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if status.is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    assert!(
        result.unwrap() && saw_ready && blocked,
        "child mutation crossed the guarded action"
    );
    assert!(
        status.is_some_and(|status| status.success()),
        "child did not finish after guard release"
    );
    assert_eq!(
        fixture
            .harnesses
            .load()
            .unwrap()
            .registry
            .get("custom")
            .unwrap()
            .definition
            .name,
        "Changed by child"
    );
}

#[cfg(unix)]
#[test]
fn custom_guard_rejects_a_symlinked_harness_lock_without_touching_target() {
    let fixture = Fixture::new();
    fixture.approve();
    let captured = fixture.capture();
    let target = fixture.dir.path().join("external-lock-target");
    std::fs::write(&target, b"unchanged").unwrap();
    let lock = fixture.dir.path().join("harnesses.json.lock");
    std::fs::remove_file(&lock).unwrap();
    std::os::unix::fs::symlink(&target, lock).unwrap();
    assert!(fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!("unsafe lock"))
        .is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"unchanged");
}
