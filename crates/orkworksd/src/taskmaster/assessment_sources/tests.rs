use super::*;
use serde_json::json;
const EPOCH: &str = "12345678-1234-4234-8234-123456789abc";
fn envelope() -> serde_json::Value {
    json!({"schemaVersion":1,"databaseEpoch":EPOCH,"workspaceSha256":"c52ddf65534b7b46035084358ab7902be4bfef220bdb503ac7039cc861905b05","observations":[],"recommendations":[],"readinessFocus":null})
}
#[test]
fn empty_original_sources_have_fixed_canonical_digest_and_round_trip() {
    let original = OriginalInputs::fixture("assessment-1", envelope()).unwrap();
    let sources = original.into_sources();
    assert_eq!(
        sources.input_digest(),
        "1297869707c0641d0165eeab56a4d3b5b39b985806f796c3374cadb2195a9148"
    );
    let bytes = serde_json::to_vec(&sources).unwrap();
    assert_eq!(
        serde_json::from_slice::<SqlAssessmentSources>(&bytes).unwrap(),
        sources
    );
    let mut value = serde_json::to_value(&sources).unwrap();
    value["databaseEpoch"] = json!("12345678-1234-4234-8234-123456789ABC");
    assert!(serde_json::from_value::<SqlAssessmentSources>(value).is_err());
}

pub(crate) fn original(epoch: &str, workspace: &str, pairs: &[(&str, &str)]) -> OriginalInputs {
    let mut input = envelope();
    input["databaseEpoch"] = json!(epoch);
    input["workspaceSha256"] = json!(hex::encode(Sha256::digest(workspace.as_bytes())));
    input["observations"] = json!([{"value":{"id":"original-observation"}, "sources":pairs.iter().map(|(id,created)|json!({"sessionId":id,"createdAt":created})).collect::<Vec<_>>()}]);
    OriginalInputs::fixture("assessment-1", input).unwrap()
}
#[test]
fn original_dependency_closure_unions_sources_and_pins_input_identity() {
    let packet = OriginalInputs::fixture("assessment-1", envelope())
        .unwrap()
        .into_sources();
    assert_eq!(
        packet
            .input_fingerprint(&json!({"z":2,"a":{"b":1}}))
            .unwrap(),
        "611f281cd08b3354a5c9326440d584337c742bdad06e830904f00b9a3443e5e1"
    );
    let mut a = envelope();
    a["observations"] =
        json!([{"value":{"z":2,"a":1},"sources":[{"sessionId":"s","createdAt":"old"}]}]);
    a["recommendations"] = a["observations"].clone();
    let one = OriginalInputs::fixture("assessment-1", a.clone())
        .unwrap()
        .into_sources();
    assert_eq!(one.sources().len(), 1);
    a["recommendations"][0]["sources"][0]["createdAt"] = json!("reused");
    assert!(OriginalInputs::fixture("assessment-1", a).is_err());
    let mut b = envelope();
    b["observations"] =
        json!([{"sources":[{"createdAt":"old","sessionId":"s"}],"value":{"a":1,"z":2}}]);
    b["recommendations"] = b["observations"].clone();
    assert_eq!(
        one,
        OriginalInputs::fixture("assessment-1", b)
            .unwrap()
            .into_sources()
    );
}
#[test]
fn source_codec_rejects_noncanonical_unknown_duplicate_and_invalid_shapes() {
    let packet = original(EPOCH, "/workspace", &[("a", "one"), ("b", "two")]).into_sources();
    let value = serde_json::to_value(&packet).unwrap();
    for (pointer, invalid) in [
        ("/schemaVersion", json!(2)),
        ("/databaseEpoch", json!(EPOCH.to_uppercase())),
        ("/workspaceSha256", json!("A".repeat(64))),
        ("/inputDependenciesSha256", json!("short")),
        ("/assessmentId", json!("bad/id")),
        ("/assessmentId", json!("x".repeat(257))),
        ("/sources/0/sessionId", json!("ø")),
        ("/sources/0/createdAt", json!("")),
        ("/sources/0/createdAt", json!("c".repeat(257))),
        ("/sources", json!(null)),
    ] {
        let mut changed = value.clone();
        *changed.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            serde_json::from_value::<SqlAssessmentSources>(changed).is_err(),
            "{pointer}"
        );
    }
    for nested in [false, true] {
        let mut changed = value.clone();
        if nested {
            changed["sources"][0]["unknown"] = json!(true);
        } else {
            changed["unknown"] = json!(true);
        }
        assert!(serde_json::from_value::<SqlAssessmentSources>(changed).is_err());
    }
    let mut reordered = value.clone();
    reordered["sources"].as_array_mut().unwrap().reverse();
    assert!(serde_json::from_value::<SqlAssessmentSources>(reordered).is_err());
    let mut duplicate = value.clone();
    duplicate["sources"][1] = duplicate["sources"][0].clone();
    assert!(serde_json::from_value::<SqlAssessmentSources>(duplicate).is_err());
    let bytes = serde_json::to_string(&packet).unwrap();
    assert!(
        serde_json::from_str::<SqlAssessmentSources>(&bytes.replacen(
            "\"schemaVersion\":1",
            "\"schemaVersion\":1,\"schemaVersion\":1",
            1
        ))
        .is_err()
    );
    assert!(
        serde_json::from_str::<SqlAssessmentSources>(&bytes.replacen(
            "\"createdAt\":\"one\"",
            "\"createdAt\":\"one\",\"createdAt\":\"one\"",
            1
        ))
        .is_err()
    );
    assert!(serde_json::from_str::<serde_json::Value>("{\"number\":NaN}").is_err());
}
#[test]
fn cardinality_and_compact_encoding_limits_are_independent_and_exact() {
    let ids = (0..MAX_SOURCES)
        .map(|i| format!("s{i:03}"))
        .collect::<Vec<_>>();
    let pairs = ids.iter().map(|id| (id.as_str(), "c")).collect::<Vec<_>>();
    let mut packet = original(EPOCH, "/workspace", &pairs).into_sources();
    assert_eq!(packet.sources.len(), MAX_SOURCES);
    packet.validate().unwrap();
    let mut too_many = packet.clone();
    too_many.sources.push(SourceCreation {
        session_id: "z".into(),
        created_at: "c".into(),
    });
    assert_eq!(too_many.validate(), Err(SourceError::TooLarge));
    let mut remaining = MAX_SOURCE_BYTES - serde_json::to_vec(&packet).unwrap().len();
    for source in &mut packet.sources {
        let add = remaining.min(255);
        source.created_at.push_str(&"x".repeat(add));
        remaining -= add;
    }
    assert_eq!(remaining, 0);
    assert_eq!(serde_json::to_vec(&packet).unwrap().len(), MAX_SOURCE_BYTES);
    packet.validate().unwrap();
    let source = packet
        .sources
        .iter_mut()
        .find(|s| s.created_at.len() < 256)
        .unwrap();
    source.created_at.push('x');
    assert_eq!(packet.validate(), Err(SourceError::TooLarge));
    assert!(
        serde_json::from_slice::<SqlAssessmentSources>(&serde_json::to_vec(&packet).unwrap())
            .is_err()
    );
}

#[test]
fn original_one_and_maximal_short_source_sets_match_independent_golden_digests() {
    for (count, digest) in [
        (
            1,
            "6f728b1ad63fcebb8175de4c2fad55611892b8634ecee9dc968ffcd72c67c58f",
        ),
        (
            128,
            "e6959d6c1bcb8f1175f6e45afa3f78c01ddccb7254761e1a4ff34b2330012fdb",
        ),
    ] {
        let ids: Vec<String> = (0..count).map(|i| format!("s{i:03}")).collect();
        let pairs: Vec<(&str, &str)> = ids.iter().map(|id| (id.as_str(), "created")).collect();
        assert_eq!(
            original(EPOCH, "/workspace", &pairs)
                .sources()
                .input_digest(),
            digest
        );
    }
}
