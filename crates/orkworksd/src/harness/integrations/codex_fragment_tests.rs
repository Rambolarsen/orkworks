use super::{
    codex, codex_fragment::*, portable_reporter_invocation_from_path,
    reporter_invocation_for_platform, ReporterPlatform,
};
use serde_json::{json, Value};
use std::path::Path;

const SENTINEL: &str = "UNRELATED_PRIVATE_SENTINEL";
fn vectors() -> Value {
    serde_json::from_str(include_str!("codex_fragment_vectors.json")).unwrap()
}
fn valid_carrier() -> Value {
    serde_json::from_str(vectors()["posix"]["canonical"].as_str().unwrap()).unwrap()
}
fn deny(bytes: &[u8], expected: CodexFragmentError) {
    let error = project_standard_posix_codex_fragment(bytes)
        .err()
        .expect("must deny");
    assert_eq!(error, expected);
    assert!(!format!("{error:?}").contains(SENTINEL));
}

// Catches stream/order/quoting/fingerprint drift in actual shared source rendering.
#[test]
fn actual_command_bundles_match_independent_posix_and_windows_literals() {
    let vectors = vectors();
    let posix = portable_reporter_invocation_from_path(
        Path::new("$HOME/.orkworks/hook-scripts/report-harness-event.sh"),
        "orkworks:harness-integration:v2:codex",
    );
    let windows = reporter_invocation_for_platform(
        ReporterPlatform::WindowsPowerShell,
        Path::new(r"C:\fixture user's\report-harness-event.ps1"),
        "orkworks:harness-integration:v2:codex",
    );
    for (key, platform, base) in [
        ("posix", ReporterPlatform::Posix, posix),
        ("windows", ReporterPlatform::WindowsPowerShell, windows),
    ] {
        assert_eq!(base.shell_command, vectors[key]["base"].as_str().unwrap());
        let (fingerprint, commands) = codex::command_bundle(&base, platform);
        assert_eq!(fingerprint, vectors[key]["fingerprint"].as_str().unwrap());
        for (index, command) in commands.iter().enumerate() {
            assert_eq!(
                command.shell_command,
                vectors[key]["commands"][index].as_str().unwrap()
            );
        }
    }
}

// Unrelated input must never leak into canonical output, even around every owned group.
#[test]
fn exact_projection_reconstructs_only_the_owned_literal_bytes() {
    let mut carrier = valid_carrier();
    carrier["user"] = json!({"private":SENTINEL});
    for groups in carrier["hooks"].as_object_mut().unwrap().values_mut() {
        let groups = groups.as_array_mut().unwrap();
        groups.insert(0, json!({"hooks":[{"type":"command","command":SENTINEL}]}));
        groups.push(json!({"anything":{"private":SENTINEL}}));
    }
    let bytes = serde_json::to_vec(&carrier).unwrap();
    let fragment = project_standard_posix_codex_fragment(&bytes)
        .unwrap()
        .unwrap();
    let expected = vectors();
    assert_eq!(
        fragment.as_bytes(),
        expected["posix"]["canonical"].as_str().unwrap().as_bytes()
    );
    assert!(fragment.as_bytes().len() <= 16 * 1024);
    assert!(!String::from_utf8_lossy(fragment.as_bytes()).contains(SENTINEL));
    let escaped = vectors()["posix"]["canonical"]
        .as_str()
        .unwrap()
        .replace("orkworks:", r"\u006frkworks:");
    let decoded = project_standard_posix_codex_fragment(escaped.as_bytes())
        .unwrap()
        .unwrap();
    assert_eq!(
        decoded.as_bytes(),
        expected["posix"]["canonical"].as_str().unwrap().as_bytes()
    );
}

#[test]
fn no_namespace_is_logical_absence_only() {
    for bytes in [
        br#"{}"#.as_slice(),
        br#"{"hooks":{}}"#,
        br#"{"user":"UNRELATED_PRIVATE_SENTINEL"}"#,
        br#"{"hooks":{"FutureEvent":[{"anything":"UNRELATED_PRIVATE_SENTINEL"}]}}"#,
    ] {
        assert!(project_standard_posix_codex_fragment(bytes)
            .unwrap()
            .is_none());
    }
}

#[test]
fn malformed_carriers_and_decoded_duplicate_keys_never_project() {
    use CodexFragmentError::MalformedCarrier;
    for bytes in [
        b"".as_slice(),
        b"null",
        b"[]",
        b"{",
        b"{}{}",
        b"\xff",
        b"\xef\xbb\xbf{}",
        br#"{"hooks":17}"#,
        br#"{"hooks":{"Stop":17}}"#,
        br#"{"hooks":{},"\u0068ooks":{}}"#,
        br#"{"user":{"private":1,"\u0070rivate":2}}"#,
        br#"{"UNRELATED_PRIVATE_SENTINEL":1,"UNRELATED_PRIVATE_SENTINEL":2}"#,
    ] {
        deny(bytes, MalformedCarrier);
    }
    let duplicated = vectors()["posix"]["canonical"].as_str().unwrap().replace(
        "\"type\":\"command\"",
        "\"type\":\"command\",\"type\":\"command\"",
    );
    deny(duplicated.as_bytes(), MalformedCarrier);
    let too_deep = format!("{{\"user\":{}null{}}}", "[".repeat(140), "]".repeat(140));
    deny(too_deep.as_bytes(), MalformedCarrier);
}

// Cap is inclusive and applies before even otherwise-valid no-owned data is parsed.
#[test]
fn carrier_limit_is_inclusive() {
    let mut bytes = b"{}".to_vec();
    bytes.resize(256 * 1024, b' ');
    assert!(project_standard_posix_codex_fragment(&bytes)
        .unwrap()
        .is_none());
    bytes.push(b' ');
    deny(&bytes, CodexFragmentError::InputTooLarge);
    let vectors = vectors();
    let mut owned = vectors["posix"]["canonical"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    owned.resize(256 * 1024, b' ');
    let fragment = project_standard_posix_codex_fragment(&owned)
        .unwrap()
        .unwrap();
    assert_eq!(
        fragment.as_bytes(),
        vectors["posix"]["canonical"].as_str().unwrap().as_bytes()
    );
    owned.push(b' ');
    deny(&owned, CodexFragmentError::InputTooLarge);
}

#[test]
fn partial_duplicate_and_customized_owned_groups_deny_the_whole_projection() {
    let mut cases = Vec::new();
    let mut value = valid_carrier();
    value["hooks"].as_object_mut().unwrap().remove("Stop");
    cases.push(value);
    let mut value = valid_carrier();
    let duplicate = value["hooks"]["Stop"][0].clone();
    value["hooks"]["Stop"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    cases.push(value);
    for (pointer, extra) in [
        ("/hooks/Stop/0/matcher", json!("*")),
        ("/hooks/Stop/0/timeout", json!(5)),
        ("/hooks/Stop/0/hooks/0/async", json!(true)),
        ("/hooks/Stop/0/hooks/0/type", json!("prompt")),
        (
            "/hooks/Stop/0/hooks/0/command",
            json!("orkworks:harness-integration:v1:codex"),
        ),
        (
            "/hooks/Stop/0/hooks/0/command",
            json!("orkworks:harness-integration:v2:foreign"),
        ),
    ] {
        let mut value = valid_carrier();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        value
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(key.into(), extra);
        cases.push(value);
    }
    let mut value = valid_carrier();
    value["hooks"]["Stop"][0]["hooks"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"command","command":SENTINEL}));
    cases.push(value);
    let mut value = valid_carrier();
    let wrong = value["hooks"]["Stop"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .replace(
            vectors()["posix"]["fingerprint"].as_str().unwrap(),
            &"0".repeat(64),
        );
    value["hooks"]["Stop"][0]["hooks"][0]["command"] = json!(wrong);
    cases.push(value);
    let mut value = valid_carrier();
    value["hooks"]["Stop"][0] = value["hooks"]["SessionStart"][0].clone();
    cases.push(value);
    for value in cases {
        deny(
            &serde_json::to_vec(&value).unwrap(),
            CodexFragmentError::UnsupportedFragment,
        );
    }
}

// Catches owned mentions missed by a six-known-event-only scan (including key names).
#[test]
fn namespace_anywhere_outside_exact_command_slots_denies() {
    deny(
        br#"{"user":"\u006frkworks:harness-integration:v2:codex"}"#,
        CodexFragmentError::UnsupportedFragment,
    );
    let mut cases = Vec::new();
    let mut value = valid_carrier();
    value["user"] = json!({"note":"mentions orkworks:harness-integration:v2:codex"});
    cases.push(value);
    let mut value = valid_carrier();
    value.as_object_mut().unwrap().insert(
        "orkworks:harness-integration:private".into(),
        json!(SENTINEL),
    );
    cases.push(value);
    let mut value = valid_carrier();
    value["hooks"]["orkworks:harness-integration:event"] = json!([]);
    cases.push(value);
    let mut value = valid_carrier();
    value["hooks"]["FutureEvent"] =
        json!([{"nested":[{"name":"orkworks:harness-integration:v2:codex"}]}]);
    cases.push(value);
    let mut value = valid_carrier();
    value["hooks"]["Stop"]
        .as_array_mut()
        .unwrap()
        .push(json!({"orkworks:harness-integration:unknown":null}));
    cases.push(value);
    let mut value = json!({"user":[{"x":["orkworks:harness-integration:v2:codex"]}]});
    cases.push(value.take());
    for value in cases {
        deny(
            &serde_json::to_vec(&value).unwrap(),
            CodexFragmentError::UnsupportedFragment,
        );
    }
}
