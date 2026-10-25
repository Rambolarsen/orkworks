//! Strict readiness wire decoding and deterministic, equal-weight totals.
use super::readiness_rubric::{ReadinessRubric, CONCEPT_IDS};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReadinessResponse {
    pub version: u8,
    pub summary: String,
    pub concepts: Vec<ReadinessRow>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReadinessRow {
    pub concept_id: String,
    pub maturity: Option<u8>,
    pub confidence: Option<Confidence>,
    pub repository_fact_ids: Vec<usize>,
    pub rubric_page_id: String,
    pub rationale: String,
    pub limits: Vec<String>,
    pub next_transition: String,
    pub unassessed_reason: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Confidence {
    Low,
    Medium,
    High,
}
impl ReadinessResponse {
    pub(crate) fn totals(&self) -> (usize, Option<u16>, Option<u8>) {
        let coverage = self
            .concepts
            .iter()
            .filter(|r| r.maturity.is_some())
            .count();
        if coverage != 14 {
            return (coverage, None, None);
        }
        let total = self
            .concepts
            .iter()
            .filter_map(|r| r.maturity)
            .map(u16::from)
            .sum::<u16>();
        (coverage, Some(total), Some(((100 * total + 28) / 56) as u8))
    }
}
fn parse<T: serde::de::DeserializeOwned>(output: &str) -> Result<T, String> {
    crate::harness::definition::parse_strict_json(output.as_bytes(), 64 * 1024)
        .map_err(|_| "Invalid readiness response shape".into())
}
pub(crate) fn decode(
    output: &str,
    fact_count: usize,
    rubric: &ReadinessRubric,
) -> Result<ReadinessResponse, String> {
    let value: serde_json::Value = parse(output)?;
    // Nullable properties are required, not implicitly absent Option defaults.
    if value["concepts"].as_array().is_none_or(|rows| {
        rows.iter().any(|row| {
            ["maturity", "confidence", "unassessedReason"]
                .iter()
                .any(|key| row.get(key).is_none())
        })
    }) {
        return Err("Missing readiness fields".into());
    }
    let response: ReadinessResponse = parse(output)?;
    if response.version != 1 || response.concepts.len() != 14 {
        return Err("Invalid readiness version or coverage".into());
    }
    super::assessment_model::validate_prose(&response.summary, 1000, true)?;
    for ((row, id), definition) in response
        .concepts
        .iter()
        .zip(CONCEPT_IDS)
        .zip(&rubric.concepts)
    {
        if row.concept_id != id
            || row.rubric_page_id != definition.page_id
            || row.repository_fact_ids.len() > 8
            || row.repository_fact_ids.iter().any(|id| *id >= fact_count)
            || row.repository_fact_ids.iter().collect::<HashSet<_>>().len()
                != row.repository_fact_ids.len()
            || row.limits.len() > 4
        {
            return Err("Invalid readiness citations or concept".into());
        }
        match (&row.maturity, &row.confidence, &row.unassessed_reason) {
            (Some(0..=4), Some(_), None) if !row.repository_fact_ids.is_empty() => {}
            (None, None, Some(reason)) => {
                super::assessment_model::validate_prose(reason, 500, true)?
            }
            _ => return Err("Invalid readiness rating or unassessed reason".into()),
        }
        super::assessment_model::validate_prose(&row.rationale, 500, true)?;
        super::assessment_model::validate_prose(&row.next_transition, 500, false)?;
        for limit in &row.limits {
            super::assessment_model::validate_prose(limit, 200, true)?;
        }
    }
    Ok(response)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Discovery {
    version: u8,
    requested_file_ids: Vec<usize>,
}
pub(crate) fn decode_discovery(output: &str, catalog_size: usize) -> Result<Vec<usize>, String> {
    let response: Discovery = parse(output)?;
    if response.version != 1
        || response.requested_file_ids.len() > 8
        || response
            .requested_file_ids
            .iter()
            .any(|id| *id >= catalog_size)
        || response
            .requested_file_ids
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != response.requested_file_ids.len()
    {
        return Err("Invalid readiness evidence selection".into());
    }
    Ok(response.requested_file_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::readiness_rubric::{ReadinessRubric, CONCEPT_IDS};
    fn rubric() -> ReadinessRubric {
        serde_json::from_value(serde_json::json!({"id":"brain-agent-readiness","revision":1,"concepts":CONCEPT_IDS.iter().map(|id|serde_json::json!({"conceptId":id,"pageId":"playbooks/rubric.md","pageSha256":"a".repeat(64),"levels":["a","b","c","d","e"]})).collect::<Vec<_>>()})).unwrap()
    }
    fn result(level: u8) -> serde_json::Value {
        serde_json::json!({"version":1,"summary":"Observed repository guidance","concepts":CONCEPT_IDS.iter().map(|id|serde_json::json!({"conceptId":id,"maturity":level,"confidence":"medium","repositoryFactIds":[0],"rubricPageId":"playbooks/rubric.md","rationale":"Documented guidance","limits":["Runtime behavior remains unverified"],"nextTransition":"Evidence of workflow integration","unassessedReason":null})).collect::<Vec<_>>()})
    }
    #[test]
    fn score_is_equal_weight_complete_and_sidecar_computed() {
        for (level, expected) in [(0, 0), (2, 50), (3, 75), (4, 100)] {
            let parsed = decode(&result(level).to_string(), 1, &rubric()).unwrap();
            assert_eq!(
                parsed.totals(),
                (14, Some(level as u16 * 14), Some(expected))
            );
        }
    }
    #[test]
    fn partial_evidence_never_becomes_zero_or_inflates_total() {
        let mut v = result(3);
        let row = &mut v["concepts"][0];
        row["maturity"] = serde_json::Value::Null;
        row["confidence"] = serde_json::Value::Null;
        row["unassessedReason"] = serde_json::json!("Required evidence is excluded");
        row["repositoryFactIds"] = serde_json::json!([]);
        let parsed = decode(&v.to_string(), 1, &rubric()).unwrap();
        assert_eq!(parsed.totals(), (13, None, None));
    }
    #[test]
    fn scoring_rejects_forged_refs_ratings_missing_rows_and_extra_score() {
        for mutate in [0, 1, 2, 3, 4, 5, 6, 7] {
            let mut v = result(3);
            match mutate {
                0 => {
                    v["concepts"][0]["repositoryFactIds"] = serde_json::json!([99]);
                }
                1 => {
                    v["concepts"][0]["maturity"] = serde_json::json!(5);
                }
                2 => {
                    v["concepts"].as_array_mut().unwrap().pop();
                }
                3 => {
                    v["overallScore"] = serde_json::json!(100);
                }
                4 => {
                    v["concepts"][0]["rubricPageId"] = serde_json::json!("other.md");
                }
                5 => {
                    v["concepts"][0]["maturity"] = serde_json::Value::Null;
                }
                6 => {
                    v["concepts"][0]["rationale"] = serde_json::json!("cargo test");
                }
                _ => {
                    v["concepts"][0]["rationale"] = serde_json::json!("All tests passed");
                }
            }
            assert!(
                decode(&v.to_string(), 1, &rubric()).is_err(),
                "case {mutate}"
            );
        }
    }
    #[test]
    fn strict_discovery_limits_selection_and_has_no_candidate_shortcut() {
        assert_eq!(
            decode_discovery(r#"{"version":1,"requestedFileIds":[0,2]}"#, 3).unwrap(),
            vec![0, 2]
        );
        for output in [
            r#"{"version":1,"requestedFileIds":[0,0]}"#,
            r#"{"version":1,"requestedFileIds":[3]}"#,
            r#"{"version":1,"version":1,"requestedFileIds":[]}"#,
            r#"{"version":1,"requestedFileIds":[],"outcome":"no_candidate"}"#,
        ] {
            assert!(decode_discovery(output, 3).is_err());
        }
    }
}
