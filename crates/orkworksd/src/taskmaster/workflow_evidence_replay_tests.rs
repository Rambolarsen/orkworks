//! Offline current-policy boundary replay. No provider, AppState or live store.

use crate::workflow_observations::WorkflowObservation;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::Path;

const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Packet {
    schema_version: u8,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Case {
    id: String,
    declared_origin: DeclaredOrigin,
    workspace_id: String,
    as_of: String,
    #[serde(default)]
    captured_candidates: Vec<Candidate>,
    #[serde(default)]
    stored_observations: Option<Vec<WorkflowObservation>>,
    #[serde(default)]
    existing_recommendations: Option<Vec<super::Recommendation>>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum DeclaredOrigin {
    Synthetic,
    Observed,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Candidate {
    id: String,
    evidence: String,
    reassembled_rows: Option<Vec<String>>,
}

fn replay_packet(raw: &[u8]) -> Result<Value, String> {
    if raw.len() > MAX_BYTES {
        return Err("replay packet exceeds byte limit".into());
    }
    let packet: Packet = serde_json::from_slice(raw).map_err(|_| "invalid replay packet")?;
    if packet.schema_version != 1 {
        return Err("unsupported replay version".into());
    }
    if packet.cases.len() > 24 {
        return Err("replay case limit exceeded".into());
    }
    let mut case_ids = HashSet::new();
    let mut results = Vec::new();
    for case in packet.cases {
        if case.id.trim().is_empty() || !case_ids.insert(case.id.clone()) {
            return Err("invalid case IDs".into());
        }
        let as_of = chrono::DateTime::parse_from_rfc3339(&case.as_of)
            .map_err(|_| "invalid evaluation instant")?
            .with_timezone(&chrono::Utc);
        if case.workspace_id.trim().is_empty()
            || case
                .existing_recommendations
                .iter()
                .flatten()
                .any(|item| item.workspace_id != case.workspace_id)
        {
            return Err("recommendation workspace mismatch".into());
        }
        if case
            .stored_observations
            .as_ref()
            .is_some_and(|rows| rows.len() > 10_000)
            || case
                .existing_recommendations
                .as_ref()
                .is_some_and(|rows| rows.len() > 10_000)
        {
            return Err("replay record limit exceeded".into());
        }
        let mut observation_ids = HashSet::new();
        let mut previous_sequence = 0;
        for row in case.stored_observations.as_deref().unwrap_or(&[]) {
            if row.id.trim().is_empty()
                || !observation_ids.insert(&row.id)
                || row.sequence <= previous_sequence
            {
                return Err("observations must have unique IDs and increasing sequences".into());
            }
            previous_sequence = row.sequence;
        }
        let mut candidate_ids = HashSet::new();
        let mut grounding = Vec::new();
        for candidate in &case.captured_candidates {
            if candidate.id.trim().is_empty() || !candidate_ids.insert(&candidate.id) {
                return Err("invalid candidate IDs".into());
            }
            let decision = match candidate.reassembled_rows.as_ref() {
                None => "unavailable",
                Some(rows) if crate::peon::evidence_is_grounded(&candidate.evidence, rows) => {
                    "grounded"
                }
                Some(_) => "ungrounded",
            };
            grounding.push(json!({"candidateId":candidate.id,"decision":decision}));
        }
        // Captured candidates and retained rows are separate boundary inputs.
        // Never retroactively apply new-admission grounding to stored history.
        let proposals = match (
            case.stored_observations.as_deref(),
            case.existing_recommendations.as_deref(),
        ) {
            (Some(observations), Some(existing)) => {
                let mut proposals = super::evaluate_workflow_improvements(
                    observations,
                    existing,
                    &case.workspace_id,
                    &case.as_of,
                );
                proposals.sort_by(|left, right| left.id.cmp(&right.id));
                Some(proposals.iter().map(|item| json!({
                    "id":item.id,"status":item.status,"priority":item.priority,"confidence":item.confidence,
                    "observationIds":item.workflow_improvement.observation_ids,
                    "recurrenceCount":item.workflow_improvement.recurrence_count,
                    "supersedesRecommendationId":item.workflow_improvement.supersedes_recommendation_id,
                })).collect::<Vec<_>>())
            }
            _ => None,
        };
        let selected = case.stored_observations.as_ref().map(|rows| {
            super::context::recent_workflow_observations(rows, as_of)
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>()
        });
        results.push(json!({"id":case.id,"declaredOrigin":case.declared_origin,"asOf":case.as_of,
            "captureGrounding":grounding,"exactFamilyProposals":proposals,"ordinaryObservationIds":selected}));
    }
    Ok(json!({"schemaVersion":1,"mode":"current-policy-boundaries",
        "inputSha256":hex::encode(Sha256::digest(raw)),"inputProvenance":"unverified",
        "semanticQuality":"not-assessed","baselineCompared":false,"cases":results}))
}

fn replay_files(input: &Path, output: &Path) -> Result<(), String> {
    let mut raw = Vec::new();
    std::fs::File::open(input)
        .map_err(|_| "cannot read replay input")?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(|_| "cannot read replay input")?;
    let report = replay_packet(&raw)?;
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| "cannot create replay report")?;
    serde_json::to_writer_pretty(&mut temporary, &report)
        .map_err(|_| "cannot write replay report")?;
    temporary
        .write_all(b"\n")
        .map_err(|_| "cannot write replay report")?;
    temporary
        .persist_noclobber(output)
        .map_err(|_| "report output exists or cannot be published")?;
    Ok(())
}

#[test]
#[ignore = "offline explicit-input replay; no provider, application or session access"]
fn replay_workflow_evidence_packet_from_files() {
    let input = std::env::var_os("ORKWORKS_WORKFLOW_REPLAY_INPUT").expect("set replay input path");
    let output =
        std::env::var_os("ORKWORKS_WORKFLOW_REPLAY_OUTPUT").expect("set new replay output path");
    replay_files(Path::new(&input), Path::new(&output)).expect("offline replay failed");
}

const SYNTHETIC: &[u8] =
    include_bytes!("../../tests/fixtures/workflow-evidence-replay/synthetic.json");

#[test]
fn replay_grounding_distinguishes_missing_capture_and_semantic_limits() {
    let report = replay_packet(SYNTHETIC).unwrap();
    let decisions = report["cases"][0]["captureGrounding"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["decision"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        decisions,
        [
            "ungrounded",
            "grounded",
            "grounded",
            "ungrounded",
            "unavailable"
        ]
    );
    assert_eq!(report["semanticQuality"], "not-assessed");
    assert_eq!(report["baselineCompared"], false);
    assert_eq!(report["cases"][0]["declaredOrigin"], "synthetic");
    assert!(!report.to_string().contains("SYNTHETIC_PRIVATE_TEXT"));
    assert!(!report.to_string().contains("fatal: cannot open build.lock"));
}

#[test]
fn replay_uses_actual_exact_family_and_distinct_ordinary_selection_owners() {
    let report = replay_packet(SYNTHETIC).unwrap();
    let result = &report["cases"][1];
    assert_eq!(
        result["ordinaryObservationIds"],
        serde_json::json!(["weak", "now", "edge"])
    );
    let proposals = result["exactFamilyProposals"].as_array().unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(
        proposals[0]["observationIds"],
        serde_json::json!(["edge", "now"])
    );
    assert_eq!(proposals[0]["recurrenceCount"], 2);
    assert_eq!(proposals[0]["priority"], "low");
}

#[test]
fn replay_does_not_retroactively_ground_retained_reports() {
    let report = replay_packet(SYNTHETIC).unwrap();
    let result = &report["cases"][2];
    assert_eq!(result["captureGrounding"][0]["decision"], "ungrounded");
    assert_eq!(
        result["exactFamilyProposals"][0]["observationIds"],
        serde_json::json!(["history-a", "history-b"])
    );
}

#[test]
fn replay_preserves_existing_id_updates_after_age_out() {
    let mut packet: serde_json::Value = serde_json::from_slice(SYNTHETIC).unwrap();
    let case = &mut packet["cases"][2];
    for row in case["storedObservations"].as_array_mut().unwrap() {
        row["observedAt"] = serde_json::json!("2026-08-20T00:00:00Z");
    }
    let observations: Vec<crate::workflow_observations::WorkflowObservation> =
        serde_json::from_value(case["storedObservations"].clone()).unwrap();
    let prior = super::evaluate_workflow_improvements(
        &observations,
        &[],
        "synthetic-workspace",
        "2026-08-21T00:00:00Z",
    )
    .remove(0);
    case["existingRecommendations"] = serde_json::json!([prior]);
    let report = replay_packet(&serde_json::to_vec(&packet).unwrap()).unwrap();
    let result = &report["cases"][2];
    assert_eq!(result["ordinaryObservationIds"], serde_json::json!([]));
    assert_eq!(result["exactFamilyProposals"][0]["id"], prior.id);
    assert_eq!(
        result["exactFamilyProposals"][0]["observationIds"],
        serde_json::json!(["history-a", "history-b"])
    );
}

#[test]
fn replay_rejects_invalid_time_and_out_of_order_or_duplicate_rows() {
    for scenario in ["time", "order", "duplicate"] {
        let mut packet: serde_json::Value = serde_json::from_slice(SYNTHETIC).unwrap();
        let case = &mut packet["cases"][1];
        if scenario == "time" {
            case["asOf"] = serde_json::json!("invalid");
        }
        if scenario == "order" {
            case["storedObservations"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1);
        }
        if scenario == "duplicate" {
            case["storedObservations"][1]["id"] = serde_json::json!("old");
        }
        let error = replay_packet(&serde_json::to_vec(&packet).unwrap()).unwrap_err();
        assert_eq!(
            error,
            if scenario == "time" {
                "invalid evaluation instant"
            } else {
                "observations must have unique IDs and increasing sequences"
            }
        );
    }
}

#[test]
fn replay_validates_the_whole_packet_before_publishing_and_never_overwrites() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.json");
    let output = directory.path().join("report.json");
    std::fs::write(&input, SYNTHETIC).unwrap();
    replay_files(&input, &output).unwrap();
    let original = std::fs::read(&output).unwrap();
    assert_eq!(std::fs::read(&input).unwrap(), SYNTHETIC);
    assert!(replay_files(&input, &output).is_err());
    assert_eq!(std::fs::read(&output).unwrap(), original);
    let second_output = directory.path().join("invalid-report.json");
    let mut invalid: serde_json::Value = serde_json::from_slice(SYNTHETIC).unwrap();
    invalid["cases"][2]["asOf"] = serde_json::json!("invalid");
    std::fs::write(&input, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(replay_files(&input, &second_output).is_err());
    assert!(!second_output.exists());
}

#[test]
fn replay_binds_full_input_bytes_without_echoing_prose() {
    let original = replay_packet(SYNTHETIC).unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(SYNTHETIC).unwrap();
    changed["cases"][1]["storedObservations"][0]["description"] =
        serde_json::json!("PRIVATE_DESCRIPTION");
    let changed = replay_packet(&serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_ne!(original["inputSha256"], changed["inputSha256"]);
    assert_eq!(original["cases"], changed["cases"]);
    assert!(!changed.to_string().contains("PRIVATE_DESCRIPTION"));
}

#[test]
fn replay_rejects_oversized_duplicate_and_cross_workspace_envelopes() {
    assert_eq!(
        replay_packet(&vec![b' '; 8 * 1024 * 1024 + 1]).unwrap_err(),
        "replay packet exceeds byte limit"
    );
    let source: serde_json::Value = serde_json::from_slice(SYNTHETIC).unwrap();
    let many = serde_json::json!({"schemaVersion":1,"cases":vec![source["cases"][0].clone();25]});
    assert_eq!(
        replay_packet(&serde_json::to_vec(&many).unwrap()).unwrap_err(),
        "replay case limit exceeded"
    );
    let mut duplicate = source.clone();
    duplicate["cases"][1]["id"] = duplicate["cases"][0]["id"].clone();
    assert_eq!(
        replay_packet(&serde_json::to_vec(&duplicate).unwrap()).unwrap_err(),
        "invalid case IDs"
    );
    let observations: Vec<crate::workflow_observations::WorkflowObservation> =
        serde_json::from_value(source["cases"][2]["storedObservations"].clone()).unwrap();
    let prior = super::evaluate_workflow_improvements(
        &observations,
        &[],
        "foreign-workspace",
        "2026-11-24T12:00:00Z",
    )
    .remove(0);
    let mut foreign = source;
    foreign["cases"][2]["existingRecommendations"] = serde_json::json!([prior]);
    assert_eq!(
        replay_packet(&serde_json::to_vec(&foreign).unwrap()).unwrap_err(),
        "recommendation workspace mismatch"
    );
}

#[test]
fn replay_keeps_missing_store_inputs_unknown_and_declarations_unverified() {
    let report = replay_packet(SYNTHETIC).unwrap();
    assert!(report["cases"][0]["exactFamilyProposals"].is_null());
    assert!(report["cases"][0]["ordinaryObservationIds"].is_null());
    let mut packet: Value = serde_json::from_slice(SYNTHETIC).unwrap();
    packet["cases"][0]["storedObservations"] = json!([]);
    packet["cases"][0]["existingRecommendations"] = json!([]);
    packet["cases"][1]
        .as_object_mut()
        .unwrap()
        .remove("existingRecommendations");
    packet["cases"][1]["declaredOrigin"] = json!("observed");
    let report = replay_packet(&serde_json::to_vec(&packet).unwrap()).unwrap();
    assert_eq!(report["cases"][0]["exactFamilyProposals"], json!([]));
    assert_eq!(report["cases"][0]["ordinaryObservationIds"], json!([]));
    assert!(report["cases"][1]["exactFamilyProposals"].is_null());
    assert_eq!(
        report["cases"][1]["ordinaryObservationIds"],
        json!(["weak", "now", "edge"])
    );
    assert_eq!(report["inputProvenance"], "unverified");
    assert_eq!(report["semanticQuality"], "not-assessed");
}
