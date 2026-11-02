use super::root_bootstrap::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
fn catalog() -> Value {
    serde_json::from_str(include_str!(
        "../../../../docs/validation/root-bootstrap-vectors.json"
    ))
    .unwrap()
}
fn canon(v: &Value) -> Vec<u8> {
    super::coordinator::canonical_json_bytes(v).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn fixture() -> (Value, Value, Vec<(Vec<u8>, String)>, String) {
    let c = catalog();
    let b = serde_json::from_str(
        c["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == "complete-unverified-root")
            .unwrap()["canonical"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let a = c["retainedArtifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["content"].as_str().unwrap().as_bytes().to_vec(),
                a["contentDigest"].as_str().unwrap().into(),
            )
        })
        .collect();
    (b, c["run"].clone(), a, c["goal"].as_str().unwrap().into())
}
fn validate(
    bytes: &[u8],
    run: &Value,
    artifacts: &[(Vec<u8>, String)],
    goal: &str,
) -> std::result::Result<ValidatedUnverifiedRootBootstrap, RootBootstrapError> {
    let run_bytes = canon(run);
    let retained: Vec<_> = artifacts
        .iter()
        .map(|(bytes, d)| RetainedArtifact {
            bytes,
            content_digest: d,
        })
        .collect();
    validate_unverified_root_bootstrap_v2(
        bytes,
        &RootBootstrapContext {
            run_bytes: &run_bytes,
            goal,
            retained_artifacts: &retained,
        },
    )
}
#[test]
fn root_bootstrap_rejects_noncanonical_and_malformed_wire() {
    let (_, run, a, goal) = fixture();
    for bytes in [
        b"{\"schemaVersion\":2,\"schemaVersion\":2}".as_slice(),
        b"{\"x\":{\"a\":0,\"a\":0}}",
        b"{\"x\":1.0}",
        b"{\"x\":1e0}",
        b"{\"x\":-0}",
        b"{\"x\":\"\\ud800\"}",
        b"{} ",
        b"{}{}",
        b"\xef\xbb\xbf{}",
        b"\xff",
    ] {
        assert_eq!(
            validate(bytes, &run, &a, &goal).unwrap_err().0,
            "canonical wire"
        );
    }
}
#[test]
fn root_bootstrap_explicitly_rejects_other_versions_and_child_family() {
    let (mut b, run, a, goal) = fixture();
    for version in [0, 1, 3, 99] {
        b["schemaVersion"] = json!(version);
        assert_eq!(
            validate(&canon(&b), &run, &a, &goal).unwrap_err().0,
            "root v2 envelope"
        );
    }
    b["schemaVersion"] = json!(2);
    b["configurationId"] = json!("child");
    b.as_object_mut().unwrap().remove("bootstrapId");
    assert_eq!(
        validate(&canon(&b), &run, &a, &goal).unwrap_err().0,
        "root v2 envelope"
    );
}

fn rejects(b: &Value, r: &Value, a: &[(Vec<u8>, String)], g: &str, reason: &str) {
    assert_eq!(validate(&canon(b), r, a, g).unwrap_err().0, reason);
}
#[test]
fn root_bootstrap_accepts_independent_complete_root_without_promoting_support() {
    let (b, r, a, g) = fixture();
    let result = validate(&canon(&b), &r, &a, &g).unwrap();
    assert_eq!(
        result.canonical_bytes(),
        catalog()["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == "complete-unverified-root")
            .unwrap()["canonical"]
            .as_str()
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        result.bootstrap_digest(),
        "8a8002146717dcb41d6066a29fcfb9c67d243bd58e24a0e47ee2b1fa53ed9be8"
    );
    assert_eq!(result.classification(), "unverified-source-bytes-only");
}
#[test]
fn root_bootstrap_rejects_closed_shape_and_forbidden_root_effects() {
    for pointer in [
        "/permissions/requested/readPaths",
        "/permissions/effective/writePaths",
        "/permissions/requested/commands",
        "/permissions/effective/externalSources",
        "/permissions/requested/connectors",
    ] {
        let (mut b, r, a, g) = fixture();
        *b.pointer_mut(pointer).unwrap() = json!(["forbidden"]);
        rejects(&b, &r, &a, &g, "empty root access");
    }
    let (mut b, r, a, g) = fixture();
    b["grant"] = Value::Null;
    rejects(&b, &r, &a, &g, "closed fields");
    let (mut b, r, a, g) = fixture();
    b["permissions"]["requested"]["tools"]
        .as_array_mut()
        .unwrap()
        .push(json!("shell"));
    rejects(&b, &r, &a, &g, "root tools");
    let (mut b, r, a, g) = fixture();
    b["permissions"]["effective"]["coordinationActions"][0]["runId"] = json!("foreign");
    rejects(&b, &r, &a, &g, "action owner");
    let (mut b, r, a, g) = fixture();
    b["outputContract"]["maxSnapshotBytes"] = json!(1);
    rejects(&b, &r, &a, &g, "empty output caps");
}
#[test]
fn root_bootstrap_rejects_snapshot_manifest_and_render_drift() {
    let (mut b, r, a, g) = fixture();
    b["roleTemplate"]["instructions"] = json!("changed");
    rejects(&b, &r, &a, &g, "snapshot digest");
    let (mut b, r, a, g) = fixture();
    b["skills"][0]["content"] = json!("changed");
    rejects(&b, &r, &a, &g, "snapshot digest");
    let (mut b, r, a, g) = fixture();
    b["skillResources"][0]["skillId"] = json!("missing");
    rejects(&b, &r, &a, &g, "resource closure");
    let (mut b, r, a, g) = fixture();
    b["requirementManifests"][0]["sourceDigest"] = json!("a".repeat(64));
    rejects(&b, &r, &a, &g, "manifest binding");
    let (mut b, r, a, g) = fixture();
    b["skills"][0]["requirementSources"] = json!([]);
    rejects(&b, &r, &a, &g, "requirement sources");
    let (mut b, r, a, g) = fixture();
    b["rules"][0]["precedence"] = json!(1);
    rejects(&b, &r, &a, &g, "rule precedence");
    let (mut b, r, a, g) = fixture();
    b["renderedInstructions"] = json!("changed");
    rejects(&b, &r, &a, &g, "root rendering");
}
#[test]
fn root_bootstrap_rejects_unretained_or_unverified_identity_joins() {
    let (mut b, r, a, g) = fixture();
    b["capabilityEvidence"]["decision"] = json!("verified");
    rejects(&b, &r, &a, &g, "unverified slice");
    let (mut b, r, a, g) = fixture();
    b["capabilityEvidence"]["checks"][0]["referenceIds"] = json!(["absent"]);
    rejects(&b, &r, &a, &g, "evidence reference");
    let (mut b, r, a, g) = fixture();
    b["capabilityEvidence"]["assessedAt"] = json!("2026-02-30T00:00:00Z");
    rejects(&b, &r, &a, &g, "UTC timestamp");
    let (mut b, r, a, g) = fixture();
    b["harness"]["platform"]["architecture"] = json!("x64");
    rejects(&b, &r, &a, &g, "harness binding");
    let (b, r, mut a, g) = fixture();
    a.pop();
    rejects(&b, &r, &a, &g, "artifact closure");
    let (b, r, mut a, g) = fixture();
    a[0].0.push(b'x');
    rejects(&b, &r, &a, &g, "artifact digest");
    let (b, r, mut a, g) = fixture();
    a.push((b"extra".to_vec(), hash(b"extra")));
    rejects(&b, &r, &a, &g, "artifact closure");
    let (b, r, a, mut g) = fixture();
    g.push('x');
    rejects(&b, &r, &a, &g, "goal binding");
}

fn seal(x: &mut Value, key: &str, domain: &str) {
    let mut body = x.clone();
    body.as_object_mut().unwrap().remove(key);
    let mut raw = domain.as_bytes().to_vec();
    raw.push(b'\n');
    raw.extend(canon(&body));
    x[key] = json!(hash(&raw));
}
fn seal_bootstrap(b: &mut Value, r: &mut Value) {
    seal(
        b,
        "bootstrapConfigurationDigest",
        "orkworks.orchestrator-bootstrap.v2",
    );
    r["bootstrapConfigurationDigest"] = b["bootstrapConfigurationDigest"].clone();
    seal(r, "runDefinitionDigest", "orkworks.orchestration-run.v1");
}
fn refresh_role(b: &mut Value, r: &mut Value) {
    let mut body = b["roleTemplate"].clone();
    body.as_object_mut().unwrap().remove("contentDigest");
    b["roleTemplate"]["contentDigest"] = json!(hash(&canon(&body)));
    let role_id = b["roleTemplate"]["id"].clone();
    let role_digest = b["roleTemplate"]["contentDigest"].clone();
    for m in b["requirementManifests"].as_array_mut().unwrap() {
        if m["sourceId"] == role_id {
            m["sourceDigest"] = role_digest.clone();
        }
    }
    let manifests = b["requirementManifests"].as_array().unwrap().clone();
    for skill in b["skills"].as_array_mut().unwrap() {
        skill["requirementSources"] = json!(if skill["requirement"] == "mandatory" {
            manifests.iter().filter(|m|m["requiredSkillIds"].as_array().unwrap().contains(&skill["id"])).map(|m|json!({"sourceId":m["sourceId"],"sourceDigest":m["sourceDigest"],"sourceLocations":m["sourceLocations"]})).collect::<Vec<_>>()
        } else {
            vec![]
        });
    }
    seal_bootstrap(b, r);
}
fn replace_role_text(b: &mut Value, r: &mut Value, content: &str) {
    let old = b["roleTemplate"]["instructions"].as_str().unwrap();
    let id = b["roleTemplate"]["id"].as_str().unwrap();
    let old_frame = format!("role {id} {}\n{old}\n", old.len());
    let new_frame = format!("role {id} {}\n{content}\n", content.len());
    let rendered = b["renderedInstructions"]
        .as_str()
        .unwrap()
        .replace(&old_frame, &new_frame);
    b["roleTemplate"]["instructions"] = json!(content);
    b["roleTemplate"]["instructionsDigest"] = json!(hash(content.as_bytes()));
    b["renderedInstructionsDigest"] = json!(hash(rendered.as_bytes()));
    b["renderedInstructions"] = json!(rendered);
    refresh_role(b, r);
}
fn replace_artifact(
    b: &mut Value,
    r: &mut Value,
    a: &mut [(Vec<u8>, String)],
    index: usize,
    bytes: Vec<u8>,
) {
    let old = a[index].1.clone();
    let new = hash(&bytes);
    a[index] = (bytes, new.clone());
    for reference in b["capabilityEvidence"]["references"]
        .as_array_mut()
        .unwrap()
    {
        if reference["contentDigest"] == old {
            reference["contentDigest"] = json!(new);
        }
    }
    for owner in ["harness", "capabilityEvidence"] {
        if b[owner]["effectiveSettingsDigest"] == old {
            b[owner]["effectiveSettingsDigest"] = json!(new);
        }
    }
    if b["model"]["policyDigest"] == old {
        b["model"]["policyDigest"] = json!(new);
    }
    seal(
        &mut b["capabilityEvidence"],
        "evidenceDigest",
        "orkworks.capability-evidence.v1",
    );
    b["harness"]["evidenceDigest"] = b["capabilityEvidence"]["evidenceDigest"].clone();
    seal_bootstrap(b, r);
}
#[test]
fn root_bootstrap_independent_run_and_all_catalog_fingerprints_remain_exact() {
    let (b, _, a, g) = fixture();
    let bytes = include_bytes!("../../tests/fixtures/root-bootstrap/run.json");
    let artifacts: Vec<_> = a
        .iter()
        .map(|(bytes, d)| RetainedArtifact {
            bytes,
            content_digest: d,
        })
        .collect();
    assert!(validate_unverified_root_bootstrap_v2(
        &canon(&b),
        &RootBootstrapContext {
            run_bytes: bytes,
            goal: &g,
            retained_artifacts: &artifacts
        }
    )
    .is_ok());
    for v in catalog()["vectors"].as_array().unwrap() {
        let bytes = v["canonical"].as_str().unwrap().as_bytes();
        assert_eq!(bytes.len() as u64, v["bytes"].as_u64().unwrap());
        assert_eq!(hash(bytes), v["sha256"].as_str().unwrap());
    }
}
#[test]
fn root_bootstrap_preserves_large_string_counters() {
    for seq in ["9007199254740993", "9223372036854775807"] {
        let (mut b, mut r, a, g) = fixture();
        let old = b["runId"].clone();
        let new = json!(format!("{}-{seq}", r["admissionEpoch"].as_str().unwrap()));
        b["runId"] = new.clone();
        r["runId"] = new.clone();
        r["runSequence"] = json!(seq);
        for owner in ["requested", "effective"] {
            for action in b["permissions"][owner]["coordinationActions"]
                .as_array_mut()
                .unwrap()
            {
                assert_eq!(action["runId"], old);
                action["runId"] = new.clone();
            }
        }
        b["capabilityEvidence"]["requestedProfileDigest"] =
            json!(hash(&canon(&b["permissions"]["requested"])));
        b["capabilityEvidence"]["effectiveProfileDigest"] =
            json!(hash(&canon(&b["permissions"]["effective"])));
        seal(
            &mut b["capabilityEvidence"],
            "evidenceDigest",
            "orkworks.capability-evidence.v1",
        );
        b["harness"]["evidenceDigest"] = b["capabilityEvidence"]["evidenceDigest"].clone();
        seal_bootstrap(&mut b, &mut r);
        assert!(validate(&canon(&b), &r, &a, &g).is_ok());
        r["runSequence"] = json!(1);
        rejects(&b, &r, &a, &g, "string");
    }
    let (b, mut r, a, g) = fixture();
    r["runSequence"] = json!("9223372036854775808");
    rejects(&b, &r, &a, &g, "run sequence");
}
#[test]
fn root_bootstrap_rejects_settings_and_policy_mismatches_with_fresh_artifact_hashes() {
    for (key, value, reason) in [
        ("harnessId", json!("other"), "settings binding"),
        (
            "deliveryOrder",
            json!([{"kind":"supplied"},{"kind":"supplied"}]),
            "delivery order",
        ),
        (
            "discoveredInstructions",
            json!([{"snapshotKind":"rule","snapshotId":"absent","contentDigest":"a".repeat(64),"precedence":0}]),
            "automatic snapshot",
        ),
    ] {
        let (mut b, mut r, mut a, g) = fixture();
        let mut settings: Value = serde_json::from_slice(&a[0].0).unwrap();
        settings[key] = value;
        replace_artifact(&mut b, &mut r, &mut a, 0, canon(&settings));
        rejects(&b, &r, &a, &g, reason);
    }
    let (mut b, mut r, mut a, g) = fixture();
    let mut p: Value = serde_json::from_slice(&a[1].0).unwrap();
    p["unknownModelAllowed"] = json!(true);
    replace_artifact(&mut b, &mut r, &mut a, 1, canon(&p));
    rejects(&b, &r, &a, &g, "pinned policy");
    let (mut b, r, a, g) = fixture();
    b["model"]["policyId"] = json!("other");
    rejects(&b, &r, &a, &g, "model binding");
    let (mut b, mut r, mut a, g) = fixture();
    let p: Value = serde_json::from_str(
        catalog()["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == "tool-managed-policy")
            .unwrap()["canonical"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    b["model"]["mode"] = p["mode"].clone();
    b["model"]["modelId"] = Value::Null;
    b["model"]["policyId"] = p["policyId"].clone();
    replace_artifact(&mut b, &mut r, &mut a, 1, canon(&p));
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
}
#[test]
fn root_bootstrap_revision_and_retained_byte_caps_are_inclusive() {
    let (mut b, mut r, a, g) = fixture();
    b["roleTemplate"]["version"] = json!(2147483647);
    refresh_role(&mut b, &mut r);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    b["roleTemplate"]["version"] = json!(2147483648u64);
    refresh_role(&mut b, &mut r);
    rejects(&b, &r, &a, &g, "integer");
    let (mut b, mut r, mut a, g) = fixture();
    replace_artifact(&mut b, &mut r, &mut a, 3, vec![b'x'; 65536]);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    replace_artifact(&mut b, &mut r, &mut a, 3, vec![b'x'; 65537]);
    rejects(&b, &r, &a, &g, "artifact bytes");
    let (b, r, mut a, g) = fixture();
    a[3].0 = vec![0xff];
    a[3].1 = hash(&a[3].0);
    rejects(&b, &r, &a, &g, "artifact UTF-8");
    let (b, r, mut a, g) = fixture();
    let duplicate = a[0].clone();
    a.push(duplicate);
    rejects(&b, &r, &a, &g, "artifact closure");
}
#[test]
fn root_bootstrap_exact_rendered_utf8_cap_includes_frame_bytes() {
    let (mut b, mut r, a, g) = fixture();
    let old = b["roleTemplate"]["instructions"].as_str().unwrap();
    let base =
        b["renderedInstructions"].as_str().unwrap().len() - old.len() - old.len().to_string().len();
    let n = (1..262144)
        .find(|n| base + n + n.to_string().len() == 262144)
        .unwrap();
    let mut content = "é".repeat(n / 2);
    if n % 2 != 0 {
        content.push('x');
    }
    replace_role_text(&mut b, &mut r, &content);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    content.push('x');
    replace_role_text(&mut b, &mut r, &content);
    rejects(&b, &r, &a, &g, "text bound");
}
#[test]
fn root_bootstrap_descriptor_cap_counts_json_escaping() {
    let (mut b, mut r, a, g) = fixture();
    let mut low = 1;
    let mut high = 100000;
    while low < high {
        let mid = (low + high + 1) / 2;
        let (mut candidate, mut run, _, _) = fixture();
        replace_role_text(&mut candidate, &mut run, &"\0".repeat(mid));
        if canon(&candidate).len() <= 1048576 {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    let mut content = "\0".repeat(low);
    replace_role_text(&mut b, &mut r, &content);
    while canon(&b).len() + 2 <= 1048576 {
        content.push('x');
        replace_role_text(&mut b, &mut r, &content);
    }
    if canon(&b).len() < 1048576 {
        b["taskCategory"] = json!(format!("{}x", b["taskCategory"].as_str().unwrap()));
        seal_bootstrap(&mut b, &mut r);
    }
    assert_eq!(canon(&b).len(), 1048576);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    b["taskCategory"] = json!(format!("{}x", b["taskCategory"].as_str().unwrap()));
    seal_bootstrap(&mut b, &mut r);
    assert_eq!(canon(&b).len(), 1048577);
    rejects(&b, &r, &a, &g, "bootstrap bytes");
}

#[test]
fn root_bootstrap_native_path_bounds_preserve_encoding_units() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    for (encoding, bytes) in [
        ("unix-bytes", vec![b'/', 0xff]),
        (
            "windows-utf16le",
            [67u16, 58, 92, 0xd800, 120]
                .iter()
                .flat_map(|n| n.to_le_bytes())
                .collect(),
        ),
    ] {
        let (mut b, mut r, a, g) = fixture();
        b["rules"][0]["provenance"]["sourcePath"] =
            json!({"encoding":encoding,"base64":STANDARD.encode(bytes)});
        seal_bootstrap(&mut b, &mut r);
        assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    }
    let (mut b, mut r, a, g) = fixture();
    b["rules"][0]["provenance"]["sourcePath"]["base64"] =
        json!(STANDARD.encode(format!("/{}", "x".repeat(2047))));
    seal_bootstrap(&mut b, &mut r);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    b["rules"][0]["provenance"]["sourcePath"]["base64"] =
        json!(STANDARD.encode(format!("/{}", "x".repeat(2048))));
    rejects(&b, &r, &a, &g, "native path bytes");
    for bytes in [vec![b'/', 0], vec![b'r', b'e', b'l']] {
        let (mut b, r, a, g) = fixture();
        b["rules"][0]["provenance"]["sourcePath"]["base64"] = json!(STANDARD.encode(bytes));
        rejects(&b, &r, &a, &g, "native Unix path");
    }
    let (mut b, r, a, g) = fixture();
    b["rules"][0]["provenance"]["sourcePath"] = json!({"encoding":"windows-utf16le","base64":STANDARD.encode([67u16,58,92,0].iter().flat_map(|n|n.to_le_bytes()).collect::<Vec<_>>())});
    rejects(&b, &r, &a, &g, "native Windows path");
}
#[test]
fn root_bootstrap_counts_inline_evidence_descriptor_bytes() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let (mut b, mut r, a, g) = fixture();
    let mut found = false;
    b["capabilityEvidence"]["executableIdentity"]["kind"] = json!("wrapped");
    for size in 1000..1500 {
        b["capabilityEvidence"]["executableIdentity"]["components"]=json!((0..32).map(|i|json!({"kind":"package-file","path":{"encoding":"unix-bytes","base64":STANDARD.encode(format!("/{i}{}","x".repeat(size)))},"contentDigest":"a".repeat(64)})).collect::<Vec<_>>());
        if canon(&b["capabilityEvidence"]["executableIdentity"]).len() <= 65536
            && canon(&b["capabilityEvidence"]).len() > 65536
        {
            found = true;
            break;
        }
    }
    assert!(found);
    let oversized = b["capabilityEvidence"].clone();
    rejects(&b, &r, &a, &g, "evidence bytes");
    // Shrink one native path until the descriptor is just below its own cap.
    while canon(&b["capabilityEvidence"]).len() > 65536 {
        let encoded = b["capabilityEvidence"]["executableIdentity"]["components"][0]["path"]
            ["base64"]
            .as_str()
            .unwrap();
        let mut bytes = STANDARD.decode(encoded).unwrap();
        bytes.truncate(bytes.len() - 3);
        b["capabilityEvidence"]["executableIdentity"]["components"][0]["path"]["base64"] =
            json!(STANDARD.encode(bytes));
    }
    let gap = 65536 - canon(&b["capabilityEvidence"]).len();
    let old = b["capabilityEvidence"]["assessorIdentity"]
        .as_str()
        .unwrap();
    b["capabilityEvidence"]["assessorIdentity"] = json!(format!("{old}{}", "x".repeat(gap)));
    seal(
        &mut b["capabilityEvidence"],
        "evidenceDigest",
        "orkworks.capability-evidence.v1",
    );
    b["harness"]["executableIdentity"] = b["capabilityEvidence"]["executableIdentity"].clone();
    b["harness"]["evidenceDigest"] = b["capabilityEvidence"]["evidenceDigest"].clone();
    seal_bootstrap(&mut b, &mut r);
    assert_eq!(canon(&b["capabilityEvidence"]).len(), 65536);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    b["capabilityEvidence"] = oversized;
    rejects(&b, &r, &a, &g, "evidence bytes");
}
#[test]
fn root_bootstrap_closed_records_and_common_leaf_bounds_are_enforced() {
    for (pointer, max) in [
        ("/taskCategory", 512),
        ("/assignment/label", 512),
        ("/assignment/description", 4096),
        ("/assignment/expectedOutput", 2048),
        ("/rules/0/sourcePath", 2048),
        ("/skills/0/version", 128),
        ("/skills/0/selectionReason", 2048),
        ("/skillResources/0/sourceReference", 2048),
        ("/requirementManifests/0/applicabilityReason", 2048),
        ("/requirementManifests/0/sourceLocations/0", 512),
        ("/rubric/dimensions/0/description", 2048),
    ] {
        let (mut b, r, a, g) = fixture();
        *b.pointer_mut(pointer).unwrap() = json!("x".repeat(max + 1));
        rejects(&b, &r, &a, &g, "text bound");
    }
    for pointer in [
        "/assignment",
        "/rules/0",
        "/skills/0",
        "/rubric",
        "/permissions/requested",
        "/capabilityEvidence",
    ] {
        let (mut b, r, a, g) = fixture();
        b.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!("PRIVATE SENTINEL"));
        rejects(&b, &r, &a, &g, "closed fields");
    }
    for (pointer, max) in [
        ("/skills", 16),
        ("/rules", 32),
        ("/skillResources", 32),
        ("/requirementManifests", 33),
        ("/rubric/dimensions", 16),
        ("/assignment/inputs", 32),
        ("/assignment/criteria", 32),
        ("/outputContract/declarations", 32),
    ] {
        let (mut b, r, a, g) = fixture();
        let item = b
            .pointer(pointer)
            .unwrap()
            .as_array()
            .unwrap()
            .first()
            .cloned()
            .unwrap_or(json!({}));
        *b.pointer_mut(pointer).unwrap() = json!(vec![item; max + 1]);
        rejects(&b, &r, &a, &g, "array bound");
    }
}

fn assignment_bytes(b: &Value) -> Vec<u8> {
    canon(
        &json!({"assignment":b["assignment"],"outputContract":b["outputContract"],"rubric":b["rubric"]}),
    )
}
fn refresh_assignment(b: &mut Value, r: &mut Value, old: &[u8]) {
    let new = assignment_bytes(b);
    let id = b["rootAssignmentId"].as_str().unwrap();
    let before = format!(
        "assignment {id} {}\n{}\n",
        old.len(),
        std::str::from_utf8(old).unwrap()
    );
    let after = format!(
        "assignment {id} {}\n{}\n",
        new.len(),
        std::str::from_utf8(&new).unwrap()
    );
    let render = b["renderedInstructions"]
        .as_str()
        .unwrap()
        .replace(&before, &after);
    b["renderedInstructionsDigest"] = json!(hash(render.as_bytes()));
    b["renderedInstructions"] = json!(render);
    seal_bootstrap(b, r);
}
#[test]
fn root_bootstrap_rubric_inputs_criteria_and_output_maxima_are_inclusive() {
    let (mut b, mut r, a, g) = fixture();
    let old = assignment_bytes(&b);
    b["rubric"]["version"] = json!(2147483647);
    refresh_assignment(&mut b, &mut r, &old);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    b["rubric"]["version"] = json!(2147483648u64);
    rejects(&b, &r, &a, &g, "integer");
    let (mut b, mut r, a, g) = fixture();
    let old = assignment_bytes(&b);
    let content = "x".repeat(4096);
    b["assignment"]["inputs"]=json!((0..32).map(|i|json!({"kind":"inline","id":format!("input{i}"),"content":content,"contentDigest":hash(content.as_bytes())})).collect::<Vec<_>>());
    refresh_assignment(&mut b, &mut r, &old);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    b["assignment"]["inputs"][0]["content"] = json!("x".repeat(4097));
    rejects(&b, &r, &a, &g, "text bound");
    let (mut b, mut r, a, g) = fixture();
    let old = assignment_bytes(&b);
    b["assignment"]["criteria"]=json!((0..32).map(|i|json!({"id":format!("criterion{i}"),"requirement":"required","description":"é".repeat(1024)})).collect::<Vec<_>>());
    refresh_assignment(&mut b, &mut r, &old);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    for c in b["assignment"]["criteria"].as_array_mut().unwrap() {
        c["requirement"] = json!("optional");
    }
    rejects(&b, &r, &a, &g, "required criterion");
    let (mut b, mut r, a, g) = fixture();
    let old = assignment_bytes(&b);
    b["outputContract"] = json!({"schemaVersion":1,"declarations":(0..32).map(|i|json!({"id":format!("out{i:02}"),"kind":"root_report","maxBytes":524288,"source":{"kind":"server-artifact"}})).collect::<Vec<_>>(),"maxSnapshotBytes":16777216,"maxRehashBytes":67108864});
    for mode in ["requested", "effective"] {
        let tools = b["permissions"][mode]["tools"].as_array_mut().unwrap();
        tools.push(json!("orkworks.result.receipt.read.v1"));
        tools.push(json!("orkworks.result.submit.v1"));
    }
    b["capabilityEvidence"]["requestedProfileDigest"] =
        json!(hash(&canon(&b["permissions"]["requested"])));
    b["capabilityEvidence"]["effectiveProfileDigest"] =
        json!(hash(&canon(&b["permissions"]["effective"])));
    seal(
        &mut b["capabilityEvidence"],
        "evidenceDigest",
        "orkworks.capability-evidence.v1",
    );
    b["harness"]["evidenceDigest"] = b["capabilityEvidence"]["evidenceDigest"].clone();
    refresh_assignment(&mut b, &mut r, &old);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    let valid = b.clone();
    for (key, value) in [
        ("maxSnapshotBytes", 16777217u64),
        ("maxRehashBytes", 67108865),
    ] {
        b = valid.clone();
        b["outputContract"][key] = json!(value);
        rejects(&b, &r, &a, &g, "integer");
    }
    b = valid.clone();
    b["outputContract"]["declarations"][0]["maxBytes"] = json!(524289);
    rejects(&b, &r, &a, &g, "integer");
    b = valid.clone();
    b["outputContract"]["declarations"][0]["kind"] = json!("assignment_evaluation");
    rejects(&b, &r, &a, &g, "root output");
    b = valid;
    b["permissions"]["effective"]["tools"]
        .as_array_mut()
        .unwrap()
        .pop();
    rejects(&b, &r, &a, &g, "root tools");
}
#[test]
fn root_bootstrap_retained_settings_bounds_and_sorted_sets_are_enforced() {
    let (mut b, mut r, mut a, g) = fixture();
    let mut settings: Value = serde_json::from_slice(&a[0].0).unwrap();
    settings["argv"] = json!(vec![""; 64]);
    settings["nonsecretBindings"] = json!((0..32)
        .map(|i| json!({"name":format!("binding{i:02}{}","x".repeat(119)),"value":""}))
        .collect::<Vec<_>>());
    replace_artifact(&mut b, &mut r, &mut a, 0, canon(&settings));
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    settings["argv"].as_array_mut().unwrap().push(json!(""));
    replace_artifact(&mut b, &mut r, &mut a, 0, canon(&settings));
    rejects(&b, &r, &a, &g, "array bound");
    settings["argv"] = json!(["é".repeat(1024)]);
    settings["nonsecretBindings"] = json!([{"name":"x".repeat(128),"value":"é".repeat(1024)}]);
    replace_artifact(&mut b, &mut r, &mut a, 0, canon(&settings));
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    settings["nonsecretBindings"][0]["value"] = json!("é".repeat(1024) + "x");
    replace_artifact(&mut b, &mut r, &mut a, 0, canon(&settings));
    rejects(&b, &r, &a, &g, "text bound");
    let (mut b, r, a, g) = fixture();
    b["skills"].as_array_mut().unwrap().reverse();
    rejects(&b, &r, &a, &g, "sorted unique set");
    let (mut b, r, a, g) = fixture();
    b["rules"][0]["sourcePath"] = json!("../outside");
    rejects(&b, &r, &a, &g, "rule path");
}

#[test]
fn root_bootstrap_retained_context_and_run_limits_are_inclusive() {
    let (mut b, mut r, mut a, g) = fixture();
    for i in 0..12 {
        let bytes = format!("extra artifact {i}").into_bytes();
        let d = hash(&bytes);
        b["capabilityEvidence"]["references"].as_array_mut().unwrap().push(json!({"id":format!("extra-{i:02}"),"kind":"fixture","sourceReference":"synthetic","contentDigest":d}));
        a.push((bytes, d));
    }
    b["capabilityEvidence"]["references"]
        .as_array_mut()
        .unwrap()
        .sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let ids: Vec<_> = b["capabilityEvidence"]["references"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].clone())
        .collect();
    b["capabilityEvidence"]["checks"][0]["referenceIds"] = json!(ids);
    seal(
        &mut b["capabilityEvidence"],
        "evidenceDigest",
        "orkworks.capability-evidence.v1",
    );
    b["harness"]["evidenceDigest"] = b["capabilityEvidence"]["evidenceDigest"].clone();
    seal_bootstrap(&mut b, &mut r);
    assert_eq!(a.len(), 16);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    b["capabilityEvidence"]["checks"][0]["referenceIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("seventeenth"));
    rejects(&b, &r, &a, &g, "array bound");
    a.push((b"extra".to_vec(), hash(b"extra")));
    rejects(&b, &r, &a, &g, "artifact count");
    let (mut b, mut r, a, _) = fixture();
    r["goal"] = json!("");
    let n = 16384 - canon(&r).len();
    let mut g = "x".repeat(n);
    r["goal"] = json!(g);
    r["goalDigest"] = json!(hash(g.as_bytes()));
    b["goalDigest"] = r["goalDigest"].clone();
    seal_bootstrap(&mut b, &mut r);
    assert_eq!(canon(&r).len(), 16384);
    assert!(validate(&canon(&b), &r, &a, &g).is_ok());
    g.push('x');
    r["goal"] = json!(g);
    r["goalDigest"] = json!(hash(g.as_bytes()));
    b["goalDigest"] = r["goalDigest"].clone();
    seal_bootstrap(&mut b, &mut r);
    assert_eq!(canon(&r).len(), 16385);
    rejects(&b, &r, &a, &g, "run bytes");
}
