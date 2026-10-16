use super::*;
use crate::taskmaster::knowledge::test_support::{activation, fixture, TEST_PUBLIC_KEY};

#[test]
fn signed_proof_persists_and_reverifies_on_restart() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = TaskmasterRuntime::open_with_test_key(dir.path().into(), TEST_PUBLIC_KEY);
    let bytes = fixture("valid-activation.json");
    runtime.activate_knowledge(&bytes).unwrap();
    let identity = runtime
        .data
        .lock()
        .unwrap()
        .verified_knowledge
        .as_ref()
        .unwrap()
        .identity()
        .clone();
    let path = runtime.knowledge_path();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let restarted = TaskmasterRuntime::open_with_test_key(dir.path().into(), TEST_PUBLIC_KEY);
    assert_eq!(
        restarted
            .data
            .lock()
            .unwrap()
            .verified_knowledge
            .as_ref()
            .unwrap()
            .identity(),
        &identity
    );
    fs::write(&path, b"corrupt").unwrap();
    assert!(runtime.brain_knowledge_availability().is_err());
    assert!(runtime.data.lock().unwrap().verified_knowledge.is_none());
    assert!(
        TaskmasterRuntime::open_with_test_key(dir.path().into(), TEST_PUBLIC_KEY)
            .data
            .lock()
            .unwrap()
            .verified_knowledge
            .is_none()
    );
}

#[test]
fn activation_is_monotonic_and_same_bundle_reattestation_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = TaskmasterRuntime::open_with_test_key(dir.path().into(), TEST_PUBLIC_KEY);
    runtime
        .activate_knowledge(&fixture("valid-activation.json"))
        .unwrap();
    let generation = runtime.data.lock().unwrap().ledger.generation;
    let prior = fs::read(runtime.knowledge_path()).unwrap();
    runtime
        .activate_knowledge(&fixture("same-bundle-newer-manifest.json"))
        .unwrap();
    assert_eq!(runtime.data.lock().unwrap().ledger.generation, generation);
    assert_eq!(fs::read(runtime.knowledge_path()).unwrap(), prior);
    let mut equivocal =
        crate::taskmaster::knowledge::test_support::bundle_value("valid-activation.json");
    equivocal["pages"][0]["title"] = serde_json::json!("Changed at equal sequence");
    assert!(runtime
        .activate_knowledge(&activation(equivocal, false))
        .is_err());
    assert_eq!(fs::read(runtime.knowledge_path()).unwrap(), prior);
    for name in [
        "sequence-zero.json",
        "wrong-key.json",
        "duplicate-payload-field.json",
        "duplicate-envelope-field.json",
        "unsigned-bundle.json",
    ] {
        assert!(
            runtime.activate_knowledge(&fixture(name)).is_err(),
            "accepted {name}"
        );
        assert_eq!(fs::read(runtime.knowledge_path()).unwrap(), prior);
        assert_eq!(runtime.data.lock().unwrap().ledger.generation, generation);
        assert_eq!(runtime.data.lock().unwrap().ledger.reservations, 0);
    }
}

#[test]
fn key_namespaces_preserve_legacy_and_foreign_proofs() {
    let dir = tempfile::tempdir().unwrap();
    let alternate = include_str!(
        "../../../../../tests/fixtures/taskmaster-knowledge/test-alternate-public-key.pem"
    );
    let old = TaskmasterRuntime::open_with_test_key(dir.path().into(), TEST_PUBLIC_KEY);
    let new = TaskmasterRuntime::open_with_test_key(dir.path().into(), alternate);
    fs::write(
        dir.path().join("knowledge.json"),
        br#"{"sequence":9007199254740991}"#,
    )
    .unwrap();
    let old_bytes = fixture("valid-activation.json");
    let new_bytes = activation(
        serde_json::json!({"formatVersion":1,"version":"other-key","sequence":0,"publishedAt":"2026-10-10T09:00:00.000Z","privacyPolicyVersion":1,"capabilities":[],"pages":[{"id":"page.md","title":"Page","type":"concept","status":"active","content":"","sha256":hex::encode(Sha256::digest(b"")),"relatedIds":[]}]}),
        true,
    );
    old.activate_knowledge(&old_bytes).unwrap();
    new.activate_knowledge(&new_bytes).unwrap();
    assert_ne!(old.knowledge_path(), new.knowledge_path());
    assert!(old.activate_knowledge(&new_bytes).is_err());
    assert!(new.activate_knowledge(&old_bytes).is_err());
    assert_eq!(fs::read(old.knowledge_path()).unwrap(), old_bytes);
    assert_eq!(fs::read(new.knowledge_path()).unwrap(), new_bytes);
    assert!(dir.path().join("knowledge.json").exists());
    for key in [TEST_PUBLIC_KEY, alternate] {
        let reopened = TaskmasterRuntime::open_with_test_key(dir.path().into(), key);
        assert!(reopened.brain_knowledge_availability().is_ok());
    }
}

#[test]
fn maximum_activation_roundtrips_and_rejected_records_do_not_mutate() {
    use base64::Engine as _;
    let dir = tempfile::tempdir().unwrap();
    let runtime = TaskmasterRuntime::open_with_test_key(dir.path().into(), TEST_PUBLIC_KEY);
    let mut bytes = serde_json::to_vec(&serde_json::json!({"activationFormatVersion":1,
        "bundleEnvelopeBase64":base64::engine::general_purpose::STANDARD.encode(fixture("maximum-bundle-envelope.json")),
        "manifestEnvelopeBase64":base64::engine::general_purpose::STANDARD.encode(fixture("maximum-manifest-envelope.json"))})).unwrap();
    assert_eq!(bytes.len(), 5_592_491);
    bytes.resize(6 * 1024 * 1024, b' ');
    runtime.activate_knowledge(&bytes).unwrap();
    let restarted = TaskmasterRuntime::open_with_test_key(dir.path().into(), TEST_PUBLIC_KEY);
    assert_eq!(
        restarted.status(None).knowledge_version.as_deref(),
        Some("maximum-envelope")
    );
    let prior = fs::read(runtime.knowledge_path()).unwrap();
    bytes.push(b' ');
    assert!(runtime.activate_knowledge(&bytes).is_err());
    assert_eq!(fs::read(runtime.knowledge_path()).unwrap(), prior);
}

#[test]
fn generation_advance_without_publication_invalidates_old_work_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = TaskmasterRuntime::open_fixture(dir.path().into());
    runtime
        .activate_knowledge(&fixture("valid-activation.json"))
        .unwrap();
    let mut settings = TaskmasterSettings::default();
    settings.selection = Some(
        serde_json::from_value(serde_json::json!({"provider":"codex","model":"test"})).unwrap(),
    );
    runtime.replace_settings(settings).unwrap();
    let mut old = runtime.evaluation_snapshot(dir.path()).unwrap();
    let (_, harnesses) = crate::test_support::test_harness_components();
    old.native_revision = Some(crate::taskmaster::provider_catalog::NativeRevision {
        document_revision: harnesses.snapshot().unwrap().document_revision,
        profile: crate::providers::native_inference::NativeProfile::Codex,
    });
    {
        let mut data = runtime.data.lock().unwrap();
        data.ledger.generation += 1;
        write_json(&dir.path().join("evaluations.json"), &data.ledger).unwrap();
    }
    let restarted = TaskmasterRuntime::open_fixture(dir.path().into());
    let fresh = restarted.evaluation_snapshot(dir.path()).unwrap();
    assert!(fresh.generation > old.generation);
    assert_eq!(fresh.knowledge_identity, old.knowledge_identity);
    assert!(!restarted
        .with_current_native_evaluation(&harnesses, dir.path(), &old, || panic!(
            "stale work accepted"
        ))
        .unwrap());
}

#[test]
fn signed_hierarchy_applicability_and_provenance_remain_in_verified_proof() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = TaskmasterRuntime::open_fixture(dir.path().into());
    let mut bundle =
        crate::taskmaster::knowledge::test_support::bundle_value("valid-activation.json");
    bundle["pages"][0]["applicability"] = serde_json::json!(["Rust"]);
    bundle["pages"][0]["provenance"] =
        serde_json::json!([{"title":"Source","url":"https://example.test/source"}]);
    let mut child = bundle["pages"][0].clone();
    child["id"] = serde_json::json!("child.md");
    child["parentId"] = bundle["pages"][0]["id"].clone();
    bundle["pages"].as_array_mut().unwrap().push(child);
    let bytes = activation(bundle, false);
    runtime.activate_knowledge(&bytes).unwrap();
    let restarted = TaskmasterRuntime::open_fixture(dir.path().into());
    let data = restarted.data.lock().unwrap();
    let proof = data.verified_knowledge.as_ref().unwrap();
    assert_eq!(proof.activation_bytes(), bytes);
    assert_eq!(
        proof.bundle().pages[1].parent_id.as_deref(),
        Some("concepts/reliable-changes.md")
    );
    assert_eq!(
        proof.bundle().pages[1].applicability.as_ref().unwrap(),
        &["Rust"]
    );
    assert_eq!(
        proof.bundle().pages[1].provenance.as_ref().unwrap()[0].url,
        "https://example.test/source"
    );
}

#[test]
fn production_starter_admits_offline_analysis_and_corruption_revokes_it() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = TaskmasterRuntime::open(dir.path().into());
    assert!(runtime.brain_knowledge_availability().is_err());
    let starter = include_bytes!("../../../../../apps/desktop/resources/knowledge/starter.json");
    runtime.activate_knowledge(starter).unwrap();
    assert!(runtime.brain_knowledge_availability().is_ok());
    assert_eq!(runtime.data.lock().unwrap().ledger.reservations, 0);
    let restarted = TaskmasterRuntime::open(dir.path().into());
    assert!(restarted.brain_knowledge_availability().is_ok());
    fs::write(runtime.knowledge_path(), b"corrupt").unwrap();
    assert!(runtime.brain_knowledge_availability().is_err());
    assert!(restarted.brain_knowledge_availability().is_err());
    assert_eq!(runtime.data.lock().unwrap().ledger.reservations, 0);
}
