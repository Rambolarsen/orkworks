use super::*;
use serde_json::json;
const EPOCH: &str = "12345678-1234-4234-8234-123456789abc";
pub(super) fn value() -> serde_json::Value {
    json!({"id":EPOCH,"sequence":1,"sessionId":"s","observedAt":"2026-11-08T00:00:00Z","kind":"obstacle","description":"blocked","evidence":"error","problemArea":null,"reportedImpact":"low","source":"peon","confidence":0.8,"fingerprint":"v1:obstacle:blocked"})
}
pub(super) fn line() -> serde_json::Value {
    let observation: WorkflowObservation = serde_json::from_value(value()).unwrap();
    json!({"record":"sql_observation_v1","observation":value(),"origin":{"schemaVersion":1,"databaseEpoch":EPOCH,"workspaceSha256":"a".repeat(64),"sessionId":"s","createdAt":"original","observationSha256":value_digest(&observation).unwrap()},"idempotencyKeyHash":hash_key("s","key"),"payloadHash":hash_payload(observation.kind,&observation.description,&observation.evidence,observation.reported_impact,None)})
}
#[test]
fn sql_observation_value_has_independent_golden_and_strict_round_trip() {
    let observation: WorkflowObservation = serde_json::from_value(value()).unwrap();
    assert_eq!(
        value_digest(&observation).unwrap(),
        "801cdfb0894ec64177b11875669bda1f45196cd8644e317a241c3e665f4b5116"
    );
    let bytes = serde_json::to_vec(&line()).unwrap();
    let decoded = decode_line(&bytes, "s").unwrap();
    assert_eq!(decoded.observation().unwrap(), &observation);
    assert!(decoded.origin().is_some());
    assert_eq!(
        decode_line(&decoded.encode().unwrap(), "s").unwrap(),
        decoded
    );
}

#[test]
fn source_codec_refuses_unknown_duplicate_malformed_and_changed_values() {
    let baseline = line();
    for (pointer, invalid) in [
        ("/origin/schemaVersion", json!(2)),
        ("/origin/databaseEpoch", json!(EPOCH.to_uppercase())),
        ("/origin/workspaceSha256", json!("A".repeat(64))),
        ("/origin/createdAt", json!("")),
        ("/origin/sessionId", json!("other")),
        ("/origin/observationSha256", json!("0".repeat(64))),
        ("/observation/description", json!("changed")),
        ("/observation/sequence", json!(0)),
        ("/observation/confidence", json!(1.1)),
        ("/observation/id", json!("not-uuid")),
        ("/payloadHash", json!("not-hash")),
    ] {
        let mut bad = baseline.clone();
        *bad.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            decode_line(&serde_json::to_vec(&bad).unwrap(), "s").is_err(),
            "{pointer}"
        );
    }
    for pointer in ["", "/origin", "/observation"] {
        let mut bad = baseline.clone();
        bad.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        assert!(decode_line(&serde_json::to_vec(&bad).unwrap(), "s").is_err());
    }
    let encoded = serde_json::to_string(&baseline).unwrap();
    for (from, to) in [
        ("\"record\":", "\"record\":\"observation\",\"record\":"),
        (
            "\"schemaVersion\":",
            "\"schemaVersion\":1,\"schemaVersion\":",
        ),
        (
            "\"description\":",
            "\"description\":\"ignored\",\"description\":",
        ),
    ] {
        assert!(decode_line(encoded.replacen(from, to, 1).as_bytes(), "s").is_err());
    }
    assert!(decode_line(&serde_json::to_vec(&baseline).unwrap(), "wrong-segment").is_err());
}
#[test]
fn origin_and_raw_line_limits_are_independent_and_exact() {
    let mut data = line();
    let long_id = "s".repeat(256);
    data["observation"]["sessionId"] = json!(long_id);
    data["origin"]["sessionId"] = json!(long_id);
    data["origin"]["observationSha256"] =
        json!(value_digest(&serde_json::from_value(data["observation"].clone()).unwrap()).unwrap());
    // Control characters count by UTF-8 in the creation constraint and escaped JSON in the packet cap.
    let mut found = None;
    for controls in 0..=256 {
        for plain in 0..=256 - controls {
            data["origin"]["createdAt"] = json!(format!(
                "{}{}",
                "\u{0001}".repeat(controls),
                "c".repeat(plain)
            ));
            if serde_json::to_vec(&data["origin"]).unwrap().len() == MAX_ORIGIN_BYTES {
                found = Some(data.clone());
                break;
            }
        }
        if found.is_some() {
            break;
        }
    }
    let mut exact = found.expect("exact origin boundary fixture");
    let bytes = serde_json::to_vec(&exact).unwrap();
    decode_line(&bytes, &long_id).unwrap();
    exact["origin"]["createdAt"] = json!(format!(
        "{}c",
        exact["origin"]["createdAt"].as_str().unwrap()
    ));
    assert!(matches!(
        decode_line(&serde_json::to_vec(&exact).unwrap(), &long_id),
        Err(StoreError::TooLarge)
    ));
    let mut raw = serde_json::to_vec(&line()).unwrap();
    raw.resize(MAX_LINE_BYTES, b' ');
    decode_line(&raw, "s").unwrap();
    raw.push(b' ');
    assert!(matches!(decode_line(&raw, "s"), Err(StoreError::TooLarge)));
}
#[test]
fn qualified_tombstone_keeps_exact_origin_and_legacy_never_gains_one() {
    let observation = line();
    let tomb = json!({"record":"sql_tombstone_v1","idempotencyKeyHash":observation["idempotencyKeyHash"],"payloadHash":observation["payloadHash"],"observationId":EPOCH,"sequence":1,"acceptedAt":"2026-11-08T00:00:00Z","origin":observation["origin"]});
    let parsed = decode_line(&serde_json::to_vec(&tomb).unwrap(), "s").unwrap();
    assert!(parsed.observation().is_none());
    assert_eq!(
        serde_json::to_value(parsed.origin().unwrap()).unwrap(),
        observation["origin"]
    );
    assert_eq!(decode_line(&parsed.encode().unwrap(), "s").unwrap(), parsed);
    let mut old = value();
    old["record"] = json!("observation");
    old["idempotencyKeyHash"] = observation["idempotencyKeyHash"].clone();
    old["payloadHash"] = observation["payloadHash"].clone();
    let legacy = decode_line(&serde_json::to_vec(&old).unwrap(), "s").unwrap();
    assert!(legacy.origin().is_none());
    assert_eq!(
        serde_json::to_value(legacy.observation().unwrap()).unwrap(),
        value()
    );
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn strict_inventory_preserves_copied_bytes_and_original_data_across_reopen() {
    use std::time::{Duration, Instant};
    let root = tempfile::tempdir().unwrap();
    let bytes = [serde_json::to_vec(&line()).unwrap(), vec![b'\n']].concat();
    std::fs::write(root.path().join("s.ndjson"), &bytes).unwrap();
    std::fs::write(root.path().join("sequence"), b"7").unwrap();
    let directory = std::fs::File::open(root.path()).unwrap();
    let snapshot = ScanRequest::capture(Instant::now() + Duration::from_secs(2))
        .unwrap()
        .read_at(&directory, EPOCH, &"a".repeat(64))
        .unwrap();
    assert_eq!(snapshot.sequence(), Some(7));
    assert_eq!(snapshot.records().len(), 1);
    assert_eq!(snapshot.records()[0].observation().unwrap().id, EPOCH);
    drop(snapshot);
    drop(directory);
    let reopened = std::fs::File::open(root.path()).unwrap();
    let snapshot = ScanRequest::capture(Instant::now() + Duration::from_secs(2))
        .unwrap()
        .read_at(&reopened, EPOCH, &"a".repeat(64))
        .unwrap();
    assert_eq!(
        snapshot.records()[0].origin().unwrap().created_at(),
        "original"
    );
    assert_eq!(std::fs::read(root.path().join("s.ndjson")).unwrap(), bytes);
    assert_eq!(std::fs::read(root.path().join("sequence")).unwrap(), b"7");
}
