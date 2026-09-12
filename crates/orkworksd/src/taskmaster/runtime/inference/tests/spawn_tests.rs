//! Only locally authored fixture programs are executed; no provider CLI is used.
use super::*;
use crate::providers::ProviderOperationErrorCode;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};

fn executable_fixture(wait: bool) -> Fixture {
    let fixture = Fixture::new();
    let executable = fixture.dir.path().join("fixture-adapter");
    fs::write(
        &executable,
        r#"#!/bin/sh
printf started > "$1"
/bin/cat > "$2"
if [ "$3" != now ]; then
  count=0
  while [ ! -f "$3" ]; do
    count=$((count + 1))
    if [ "$count" -gt 400 ]; then exit 2; fi
    /bin/sleep 0.01
  done
fi
printf '%s' '{"version":1,"status":"success","result":"{\"enrichments\":[],\"proposals\":[]}"}'
"#,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let args = json!([
        fixture.dir.path().join("started"),
        fixture.dir.path().join("input"),
        if wait {
            fixture.dir.path().join("release")
        } else {
            PathBuf::from("now")
        },
        "{model}"
    ]);
    let path = fixture.dir.path().join("harnesses.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    document["custom"][0]["inference"] = json!({
        "kind":"command","command":executable,"args":args,
        "input":"stdin","output":"result-json-v1","timeoutSecs":10
    });
    fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
    fixture.approve();
    fixture
}

#[test]
fn custom_spawn_rejects_revoked_identity_without_starting_process() {
    let fixture = executable_fixture(false);
    let captured = fixture.capture();
    fixture
        .trust
        .revoke("custom", fixture.trust.generation().unwrap())
        .unwrap();
    let error = fixture
        .runtime
        .invoke_custom_inference(&fixture.harnesses, &captured, "fixture context".into())
        .unwrap_err();
    assert_eq!(error.code, ProviderOperationErrorCode::StaleGeneration);
    assert!(!fixture.dir.path().join("started").exists());
}

#[test]
fn custom_spawn_rechecks_definition_and_unreadable_trust() {
    for corrupt in [false, true] {
        let fixture = executable_fixture(false);
        let captured = fixture.capture();
        if corrupt {
            fs::write(fixture.runtime.root.join("inference-trust.json"), "broken").unwrap();
        } else {
            let path = fixture.dir.path().join("harnesses.json");
            let mut document: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            document["custom"][0]["inference"] = serde_json::Value::Null;
            fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
        }
        let error = fixture
            .runtime
            .invoke_custom_inference(&fixture.harnesses, &captured, "fixture context".into())
            .unwrap_err();
        assert_eq!(
            error.code,
            if corrupt {
                ProviderOperationErrorCode::VerificationRequired
            } else {
                ProviderOperationErrorCode::StaleGeneration
            }
        );
        assert!(!fixture.dir.path().join("started").exists());
    }
}

#[test]
fn custom_spawn_releases_trust_locks_while_process_runs() {
    let fixture = executable_fixture(true);
    let captured = fixture.capture();
    std::thread::scope(|scope| {
        let invocation = scope.spawn(|| {
            fixture.runtime.invoke_custom_inference(
                &fixture.harnesses,
                &captured,
                "literal fixture context".into(),
            )
        });
        let started = fixture.dir.path().join("started");
        let deadline = Instant::now() + Duration::from_secs(3);
        while !started.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let trust = &fixture.trust;
        let revocation = scope.spawn(move || {
            let result = trust
                .generation()
                .and_then(|generation| trust.revoke("custom", generation));
            tx.send(result).unwrap();
        });
        let revoked = rx.recv_timeout(Duration::from_secs(2));
        let (harness_tx, harness_rx) = std::sync::mpsc::channel();
        let harnesses = &fixture.harnesses;
        let path = fixture.dir.path().join("harnesses.json");
        let harness_probe = scope.spawn(move || {
            // Check the original store mutex and lease, then a separate store's
            // lease. Either retained lock would prevent this from completing.
            let result = harnesses.with_locked_snapshot(|_| ()).and_then(|()| {
                HarnessStore::new(
                    path,
                    Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
                )
                .with_locked_snapshot(|_| ())
            });
            harness_tx.send(result).unwrap();
        });
        let harness_unlocked = harness_rx.recv_timeout(Duration::from_secs(2));
        // Always unblock the process before asserting or joining scoped threads.
        fs::write(fixture.dir.path().join("release"), "release").unwrap();
        revocation.join().unwrap();
        harness_probe.join().unwrap();
        let result = invocation.join().unwrap();
        assert!(started.exists(), "fixture did not start");
        revoked
            .expect("revocation was blocked by inference waiting")
            .unwrap();
        harness_unlocked
            .expect("harness lock was held during inference waiting")
            .unwrap();
        assert_eq!(result.unwrap(), r#"{"enrichments":[],"proposals":[]}"#);
        assert_eq!(
            fs::read_to_string(fixture.dir.path().join("input")).unwrap(),
            "literal fixture context"
        );
    });
    assert!(!fixture
        .runtime
        .with_current_custom_inference(&fixture.harnesses, &captured, || panic!(
            "revoked output accepted"
        ))
        .unwrap());
}
