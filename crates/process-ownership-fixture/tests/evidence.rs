use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

const PLATFORMS: &[&str] = &["windows", "macos", "linux"];
const EXPECTED_ROW_COUNT: usize = 48;
const EXPECTED_SCENARIO_COUNT: usize = 16;
const EXPECTED_PLATFORM_COUNT: usize = 3;
const EXPECTED_PASS_ROWS: usize = 5;
const EXPECTED_ACCEPTED_ROWS: usize = 9;
const EXPECTED_UNRESOLVED_ROWS: usize = 1;
const EXPECTED_UNSUPPORTED_ROWS: usize = 33;

const SCENARIOS: &[&str] = &[
    "normal-owner-loss",
    "sidecar-crashes-first",
    "registration-or-containment-failure",
    "admission-races-with-owner-loss",
    "pty-separate-session-group",
    "descendant-topology",
    "graceful-ignore-escalation",
    "supervisor-dies-with-live-descendants",
    "foreign-owner-survives",
    "ticket-replay-and-cross-generation-rejection",
    "late-obsolete-generation-event",
    "immediate-relaunch",
    "forced-electron-termination",
    "two-open-generations",
    "inference-admission-while-older-root-survives",
    "production-launch-seam-audit",
];

const REQUIRED_FIELDS: &[&str] = &[
    "scenario",
    "platform",
    "mechanism",
    "host",
    "architecture",
    "build_mode",
    "test_command",
    "forced_parent_termination",
    "native_identities",
    "containment_observation",
    "cleanup_latency_ms",
    "result",
    "survivors",
    "evidence_ref",
];

fn evidence_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("evidence/matrix.json")
}

fn non_empty_string(row: &Value, field: &str, row_key: &str) -> String {
    let value = row
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{row_key}: `{field}` must be a string"));
    assert!(
        !value.trim().is_empty(),
        "{row_key}: `{field}` must not be empty"
    );
    value.to_owned()
}

#[test]
fn evidence_matrix_covers_every_platform_and_requires_proof_for_passes() {
    let path = evidence_path();
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let document: Value = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let rows = document
        .get("rows")
        .and_then(Value::as_array)
        .expect("evidence matrix must contain a rows array");
    assert_eq!(
        document.get("scenario_count").and_then(Value::as_u64),
        Some(EXPECTED_SCENARIO_COUNT as u64)
    );
    assert_eq!(
        document.get("platform_count").and_then(Value::as_u64),
        Some(EXPECTED_PLATFORM_COUNT as u64)
    );
    let declared_counts = document
        .get("result_counts")
        .and_then(Value::as_object)
        .expect("evidence matrix must declare result_counts");

    let expected_count = SCENARIOS.len() * PLATFORMS.len();
    assert_eq!(expected_count, EXPECTED_ROW_COUNT);
    assert_eq!(
        rows.len(),
        EXPECTED_ROW_COUNT,
        "evidence matrix must contain one row per scenario/platform"
    );

    let mut keys = HashSet::new();
    let mut result_counts = HashMap::new();
    for row in rows {
        let object = row.as_object().expect("evidence rows must be JSON objects");
        for field in REQUIRED_FIELDS {
            assert!(object.contains_key(*field), "row is missing `{field}`");
        }

        let scenario = non_empty_string(row, "scenario", "evidence row");
        let platform = non_empty_string(row, "platform", &scenario);
        assert!(
            SCENARIOS.contains(&scenario.as_str()),
            "unknown evidence scenario `{scenario}`"
        );
        assert!(
            PLATFORMS.contains(&platform.as_str()),
            "unknown evidence platform `{platform}`"
        );
        let row_key = format!("{scenario}/{platform}");
        assert!(
            keys.insert(row_key.clone()),
            "duplicate evidence row {row_key}"
        );

        for field in REQUIRED_FIELDS {
            if *field != "cleanup_latency_ms" && *field != "scenario" && *field != "platform" {
                non_empty_string(row, field, &row_key);
            }
        }
        non_empty_string(row, "evidence_ref", &row_key);
        let latency = row
            .get("cleanup_latency_ms")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| {
                panic!("{row_key}: `cleanup_latency_ms` must be a non-negative integer")
            });
        assert!(
            latency <= 10_000,
            "{row_key}: cleanup latency exceeds the 10 s fixture bound"
        );

        let result = non_empty_string(row, "result", &row_key);
        *result_counts.entry(result.clone()).or_insert(0usize) += 1;
        if scenario == "production-launch-seam-audit" {
            assert_eq!(
                row.get("evidence_ref").and_then(Value::as_str),
                Some("Task 6 report"),
                "{row_key}: production seam rows must cite Task 6 report"
            );
        }
        match result.as_str() {
            "pass" => {
                let observation = non_empty_string(row, "containment_observation", &row_key);
                assert!(
                    observation.to_ascii_lowercase().contains("independent"),
                    "{row_key}: pass requires independent observation"
                );
                let forced_parent = non_empty_string(row, "forced_parent_termination", &row_key);
                let forced_parent = forced_parent.to_ascii_lowercase();
                assert!(
                    forced_parent.contains("proof")
                        && (forced_parent.contains("terminateprocess")
                            || forced_parent.contains("sigkill")),
                    "{row_key}: pass requires forced-parent proof"
                );
                let identities = non_empty_string(row, "native_identities", &row_key);
                assert!(
                    !identities.to_ascii_lowercase().contains("unavailable"),
                    "{row_key}: pass cannot have unavailable native identities"
                );
            }
            "unresolved" | "unsupported" => {
                let reason = non_empty_string(row, "reason", &row_key);
                assert!(
                    reason.len() >= 20 && reason.chars().any(|character| character.is_alphabetic()),
                    "{row_key}: unresolved/unsupported rows require a concrete reason"
                );
            }
            "accepted" => {}
            other => panic!("{row_key}: unsupported evidence result `{other}`"),
        }
    }

    assert_eq!(result_counts.get("pass"), Some(&EXPECTED_PASS_ROWS));
    assert_eq!(result_counts.get("accepted"), Some(&EXPECTED_ACCEPTED_ROWS));
    assert_eq!(
        result_counts.get("unresolved"),
        Some(&EXPECTED_UNRESOLVED_ROWS)
    );
    assert_eq!(
        result_counts.get("unsupported"),
        Some(&EXPECTED_UNSUPPORTED_ROWS)
    );
    for (result, expected) in [
        ("pass", EXPECTED_PASS_ROWS),
        ("accepted", EXPECTED_ACCEPTED_ROWS),
        ("unresolved", EXPECTED_UNRESOLVED_ROWS),
        ("unsupported", EXPECTED_UNSUPPORTED_ROWS),
    ] {
        assert_eq!(
            declared_counts.get(result).and_then(Value::as_u64),
            Some(expected as u64),
            "declared result count for `{result}` must match the validated rows"
        );
    }

    for scenario in SCENARIOS {
        for platform in PLATFORMS {
            assert!(
                keys.contains(&format!("{scenario}/{platform}")),
                "missing evidence row {scenario}/{platform}"
            );
        }
    }
}
