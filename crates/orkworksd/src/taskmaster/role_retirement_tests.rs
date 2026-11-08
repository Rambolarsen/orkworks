use super::role_retirement::*;
use super::root_bootstrap::wire::{canonical, hash, raw};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};

fn fixture(name: &str) -> &'static [u8] {
    match name {
        "role" => include_bytes!("../../tests/fixtures/role-retirement/role.json"),
        "skill" => include_bytes!("../../tests/fixtures/role-retirement/skill.json"),
        "resource-skill" => {
            include_bytes!("../../tests/fixtures/role-retirement/resource-skill.json")
        }
        _ => panic!("unknown fixture"),
    }
}
fn value(name: &str) -> Value {
    serde_json::from_slice(fixture(name)).unwrap()
}
fn validate(x: &Value) -> Result<ValidatedRetirementArtifact, RetirementArtifactError> {
    validate_retirement_artifact_v1(&canonical(x).unwrap())
}
fn seal(x: &mut Value) {
    let role = x["artifactKind"] == "role-template";
    let s = &mut x["snapshot"];
    if role {
        s["instructionsDigest"] = json!(hash(s["instructions"].as_str().unwrap()));
        s.as_object_mut().unwrap().remove("contentDigest");
        s["contentDigest"] = json!(raw(s).unwrap());
    } else {
        s["contentDigest"] = json!(hash(s["content"].as_str().unwrap()));
    }
    for r in x["resources"].as_array_mut().unwrap() {
        r["contentDigest"] = json!(hash(r["content"].as_str().unwrap()));
    }
}
#[test]
fn role_retirement_independent_goldens_are_valid() {
    let expected: Value = serde_json::from_slice(include_bytes!(
        "../../tests/fixtures/role-retirement/expected.json"
    ))
    .unwrap();
    for name in ["role", "skill", "resource-skill"] {
        let result = validate_retirement_artifact_v1(fixture(name)).unwrap();
        assert_eq!(result.canonical_bytes(), fixture(name));
        let e = &expected[name];
        assert_eq!(
            result.snapshot_digest(),
            e["snapshotDigest"].as_str().unwrap()
        );
        assert_eq!(
            result.key().source_identity(),
            e["sourceIdentity"].as_str().unwrap()
        );
        assert_eq!(
            result.key().artifact_id(),
            e["artifactId"].as_str().unwrap()
        );
        match result.key() {
            RetirementArtifactKey::RoleTemplate { version, .. } => {
                assert_eq!(json!(version), e["version"])
            }
            RetirementArtifactKey::Skill { version, .. } => {
                assert_eq!(json!(version), e["version"])
            }
        }
    }
}
#[test]
fn role_retirement_rejects_noncanonical_wire() {
    let bytes = fixture("role");
    let text = std::str::from_utf8(bytes).unwrap();
    let bad = [
        format!("{text}\n"),
        text.replacen("{", "{\"artifactKind\":\"role-template\",", 1),
        text.replace("\"version\":1", "\"version\":1.0"),
        text.replace("\"version\":1", "\"version\":1e0"),
        text.replace("\"version\":1", "\"version\":-0"),
        text.replace("café", "caf\\u00e9"),
    ];
    for b in bad {
        assert!(
            validate_retirement_artifact_v1(b.as_bytes()).is_err(),
            "{b}"
        );
    }
    assert!(validate_retirement_artifact_v1(b"\xff").is_err());
    assert!(validate_retirement_artifact_v1(&vec![b' '; 1048577]).is_err());
}
#[test]
fn role_retirement_rejects_closed_field_and_variant_violations() {
    for (name, pointer, bad) in [
        ("role", "/artifactKind", json!("other")),
        ("role", "/snapshot/role", json!("worker")),
        ("role", "/snapshot/version", json!("1")),
        ("role", "/snapshot/version", json!(0)),
        ("role", "/snapshot/version", json!(-1)),
        ("role", "/snapshot/version", json!(2147483648u64)),
        ("role", "/snapshot/id", json!("bad/id")),
        (
            "role",
            "/snapshot/instructionsDigest",
            json!("A".repeat(64)),
        ),
        ("role", "/snapshot/contentDigest", json!("0".repeat(64))),
        ("role", "/resources", json!([{}])),
        ("skill", "/snapshot/version", json!(1)),
        ("skill", "/snapshot/version", json!("")),
        ("skill", "/snapshot/contentDigest", json!("0".repeat(64))),
        (
            "resource-skill",
            "/resources/0/contentDigest",
            json!("0".repeat(64)),
        ),
    ] {
        let mut x = value(name);
        *x.pointer_mut(pointer).unwrap() = bad;
        assert!(validate(&x).is_err(), "{pointer}: {x}");
    }
    for (name, pointer) in [
        ("role", ""),
        ("role", "/snapshot"),
        ("skill", "/snapshot"),
        ("resource-skill", "/resources/0"),
    ] {
        let mut x = value(name);
        x.pointer_mut(pointer).unwrap()["unknown"] = json!(true);
        assert!(validate(&x).is_err());
    }
    for field in ["requirement", "selectionReason", "requirementSources"] {
        let mut x = value("skill");
        x["snapshot"][field] = json!("assignment-only");
        assert!(validate(&x).is_err());
    }
    for role in [
        "orchestrator",
        "research",
        "implementation",
        "review",
        "verification",
        "remediation",
    ] {
        let mut x = value("role");
        x["snapshot"]["role"] = json!(role);
        seal(&mut x);
        assert!(validate(&x).is_ok());
    }
}
#[test]
fn role_retirement_provenance_is_closed_and_lexical() {
    for p in [
        json!({}),
        json!({"schemaVersion":2,"kind":"nonlocal","sourceKind":"bundled","sourceIdentity":"a","sourceRevision":"b"}),
        json!({"schemaVersion":1,"kind":"nonlocal","sourceKind":"repository","sourceIdentity":"a","sourceRevision":"b"}),
        json!({"schemaVersion":1,"kind":"remote","sourceKind":"bundled","sourceIdentity":"a","sourceRevision":"b"}),
        json!({"schemaVersion":1,"kind":"nonlocal","sourceKind":"unknown","sourceIdentity":"a","sourceRevision":"b"}),
    ] {
        let mut x = value("skill");
        x["snapshot"]["provenance"] = p;
        assert!(validate(&x).is_err());
    }
    for (pointer, bad) in [
        (
            "/resources/0/provenance/sourcePath/base64",
            json!(STANDARD.encode(b"relative")),
        ),
        (
            "/resources/0/provenance/sourcePath/base64",
            json!(STANDARD.encode(b"/null\0")),
        ),
        ("/resources/0/provenance/sourcePath/base64", json!("!!")),
        ("/resources/0/provenance/sourcePath/encoding", json!("utf8")),
        ("/resources/0/provenance/sourcePath", json!("/path")),
        ("/resources/0/provenance/sourceIdentity", json!("")),
    ] {
        let mut x = value("resource-skill");
        *x.pointer_mut(pointer).unwrap() = bad;
        assert!(validate(&x).is_err());
    }
    let mut x = value("skill");
    x["snapshot"]["provenance"]["sourcePath"] = json!({});
    assert!(validate(&x).is_err());
}
#[test]
fn role_retirement_requires_exact_sorted_resource_closure() {
    for mode in 0..7 {
        let mut x = value("resource-skill");
        match mode {
            0 => x["resources"] = json!([]),
            1 => x["snapshot"]["resourceIds"] = json!([]),
            2 => x["resources"][0]["skillId"] = json!("other"),
            3 => {
                let r = x["resources"][0].clone();
                x["resources"].as_array_mut().unwrap().push(r);
            }
            4 => x["snapshot"]["resourceIds"] = json!(["res-a", "res-a"]),
            5 => x["snapshot"]["resourceIds"] = json!(["res-z", "res-a"]),
            _ => x["resources"][0]["id"] = json!("missing"),
        }
        assert!(validate(&x).is_err(), "mode {mode}");
    }
    let mut x = resource_count(2);
    x["resources"].as_array_mut().unwrap().reverse();
    assert!(validate(&x).is_err());
}
fn resource_count(n: usize) -> Value {
    let mut x = value("resource-skill");
    let resource = x["resources"][0].clone();
    x["resources"] = json!((0..n)
        .map(|i| {
            let mut r = resource.clone();
            r["id"] = json!(format!("res-{i:02}"));
            r
        })
        .collect::<Vec<_>>());
    x["snapshot"]["resourceIds"] = json!((0..n).map(|i| format!("res-{i:02}")).collect::<Vec<_>>());
    x
}
#[test]
fn role_retirement_caps_resources_and_aggregate_content() {
    assert!(validate(&resource_count(32)).is_ok());
    assert!(validate(&resource_count(33)).is_err());
    for role in [false, true] {
        let mut x = value(if role { "role" } else { "skill" });
        let field = if role { "instructions" } else { "content" };
        x["snapshot"][field] = json!("é".repeat(131072));
        seal(&mut x);
        assert!(validate(&x).is_ok());
        x["snapshot"][field] = json!(format!("{}x", "é".repeat(131072)));
        seal(&mut x);
        assert!(validate(&x).is_err());
    }
    let mut x = value("resource-skill");
    x["snapshot"]["content"] = json!("é".repeat(131071));
    x["resources"][0]["content"] = json!("é");
    seal(&mut x);
    assert!(validate(&x).is_ok());
    x["resources"][0]["content"] = json!("éx");
    seal(&mut x);
    assert!(validate(&x).is_err());
}
#[test]
fn role_retirement_text_and_revision_boundaries() {
    for (pointer, max, multibyte) in [
        ("/snapshot/version", 128, true),
        ("/snapshot/id", 128, false),
        ("/snapshot/provenance/sourceIdentity", 512, true),
        ("/snapshot/provenance/sourceRevision", 512, true),
        ("/resources/0/sourceReference", 2048, true),
    ] {
        let mut x = value("resource-skill");
        let exact = if multibyte {
            "é".repeat(max / 2)
        } else {
            "a".repeat(max)
        };
        *x.pointer_mut(pointer).unwrap() = json!(exact);
        if pointer == "/snapshot/id" {
            x["resources"][0]["skillId"] = x["snapshot"]["id"].clone();
        }
        assert!(validate(&x).is_ok(), "{pointer}");
        *x.pointer_mut(pointer).unwrap() = json!(format!("{exact}x"));
        assert!(validate(&x).is_err(), "{pointer}");
    }
    let mut x = value("resource-skill");
    for size in [2048, 2049] {
        x["resources"][0]["provenance"]["sourcePath"]["base64"] =
            json!(STANDARD.encode(format!("/{}", "x".repeat(size - 1))));
        assert_eq!(validate(&x).is_ok(), size == 2048);
    }
    let mut x = value("role");
    x["snapshot"]["version"] = json!(2147483647);
    seal(&mut x);
    assert!(validate(&x).is_ok());
    let mut a = value("skill");
    let mut b = a.clone();
    a["snapshot"]["version"] = json!("é");
    b["snapshot"]["version"] = json!("e\u{301}");
    assert_ne!(validate(&a).unwrap().key(), validate(&b).unwrap().key());
}
#[test]
fn role_retirement_payload_byte_cap_is_inclusive() {
    let mut x = value("skill");
    x["snapshot"]["content"] = json!("");
    seal(&mut x);
    let overhead = canonical(&x).unwrap().len();
    for size in [1048576, 1048577] {
        let remaining = size - overhead;
        x["snapshot"]["content"] = json!(
            format!(
                "{}{}",
                "\0".repeat(remaining / 6),
                "é".repeat(remaining % 6 / 2)
            ) + if remaining % 2 == 1 { "x" } else { "" }
        );
        seal(&mut x);
        let bytes = canonical(&x).unwrap();
        assert_eq!(bytes.len(), size);
        assert_eq!(
            validate_retirement_artifact_v1(&bytes).is_ok(),
            size == 1048576
        );
    }
}
#[test]
fn role_retirement_key_excludes_digest_and_revision_but_includes_source() {
    let x = value("role");
    let original = validate(&x).unwrap();
    let mut changed = x.clone();
    changed["snapshot"]["provenance"]["sourceRevision"] = json!("new");
    seal(&mut changed);
    let revision = validate(&changed).unwrap();
    assert_eq!(original.key(), revision.key());
    assert_ne!(original.snapshot_digest(), revision.snapshot_digest());
    changed["snapshot"]["provenance"]["sourceIdentity"] = json!("other");
    seal(&mut changed);
    assert_ne!(original.key(), validate(&changed).unwrap().key());
    let raw_digest = hash(fixture("role"));
    assert_ne!(original.snapshot_digest(), raw_digest);
    for domain in [
        "orkworks.configuration.v1",
        "orkworks.root-bootstrap.v2",
        "orkworks.retirement-artifact-snapshot.v1\\n",
    ] {
        assert_ne!(
            original.snapshot_digest(),
            hash([domain.as_bytes(), b"\n", fixture("role")].concat())
        );
    }
}
