use super::{
    assessment_model::validate_prose, runtime::KnowledgePage, RepositoryEvidence, TargetSurface,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const ENVELOPE_FIELDS: [&str; 7] = [
    "version",
    "outcome",
    "candidate",
    "repositoryFactHashes",
    "knowledgePageIds",
    "requestedFileIds",
    "reason",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DiscoveryResponse {
    pub version: u8,
    pub outcome: DiscoveryOutcome,
    pub candidate: Option<DiscoveryCandidate>,
    pub repository_fact_hashes: Vec<String>,
    pub knowledge_page_ids: Vec<String>,
    pub requested_file_ids: Vec<usize>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DiscoveryOutcome {
    Inspect,
    NoCandidate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DiscoveryCandidate {
    pub target_surface: TargetSurface,
    pub title: String,
    pub hypothesis: String,
}

pub(crate) fn parse_discovery_response(
    output: &str,
    facts: &[RepositoryEvidence],
    pages: &[KnowledgePage],
    catalog_len: usize,
) -> Result<DiscoveryResponse, String> {
    if output.len() > MAX_RESPONSE_BYTES {
        return Err("discovery response exceeds the size limit".into());
    }
    let envelope: serde_json::Value =
        serde_json::from_str(output).map_err(|_| "discovery response is not valid JSON")?;
    let object = envelope
        .as_object()
        .ok_or("discovery response must be an object")?;
    if ENVELOPE_FIELDS
        .iter()
        .any(|field| !object.contains_key(*field))
    {
        return Err("discovery response is missing a required field".into());
    }
    // Parsing the original text rejects duplicate keys; the presence check also
    // requires explicit nulls for nullable fields.
    let response: DiscoveryResponse =
        serde_json::from_str(output).map_err(|_| "discovery response has an invalid shape")?;
    if response.version != 1 {
        return Err("unsupported discovery response version".into());
    }
    validate_references(
        &response.repository_fact_hashes,
        32,
        &facts.iter().map(|fact| fact.sha256.as_str()).collect(),
    )?;
    validate_references(
        &response.knowledge_page_ids,
        8,
        &pages.iter().map(|page| page.id.as_str()).collect(),
    )?;
    let mut seen = HashSet::new();
    if response.requested_file_ids.len() > 8
        || response
            .requested_file_ids
            .iter()
            .any(|id| *id >= catalog_len || !seen.insert(*id))
    {
        return Err("discovery contains invalid catalog IDs".into());
    }
    match response.outcome {
        DiscoveryOutcome::Inspect => {
            let candidate = response
                .candidate
                .as_ref()
                .ok_or("inspect requires a candidate")?;
            if response.reason.is_some()
                || response.repository_fact_hashes.is_empty()
                || response.knowledge_page_ids.is_empty()
            {
                return Err("inspect requires citations and a null reason".into());
            }
            validate_prose(&candidate.title, 240, true)?;
            validate_prose(&candidate.hypothesis, 1_000, true)?;
        }
        DiscoveryOutcome::NoCandidate => {
            if response.candidate.is_some() || !response.requested_file_ids.is_empty() {
                return Err("no_candidate requires a null candidate and no file requests".into());
            }
            let reason = response
                .reason
                .as_deref()
                .ok_or("no_candidate requires a reason")?;
            validate_prose(reason, 1_000, true)?;
        }
    }
    Ok(response)
}

/// A readiness evidence plan grants only a bounded read, never proposal authority.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReadinessInvestigation {
    pub version: u8,
    pub concept_page_ids: Vec<String>,
    pub requested_file_ids: Vec<usize>,
}

pub(crate) fn parse_readiness_investigation(
    output: &str,
    signed_concept_page_ids: &[String],
    catalog_len: usize,
) -> Result<ReadinessInvestigation, String> {
    if output.len() > MAX_RESPONSE_BYTES {
        return Err("readiness investigation exceeds the size limit".into());
    }
    let plan: ReadinessInvestigation =
        serde_json::from_str(output).map_err(|_| "readiness investigation has an invalid shape")?;
    if plan.version != 1 || plan.concept_page_ids.is_empty() {
        return Err("readiness investigation requires supported concept guidance".into());
    }
    validate_references(
        &plan.concept_page_ids,
        3,
        &signed_concept_page_ids.iter().map(String::as_str).collect(),
    )?;
    let mut seen = HashSet::new();
    if plan.requested_file_ids.len() > 8
        || plan
            .requested_file_ids
            .iter()
            .any(|id| *id >= catalog_len || *id >= 512 || !seen.insert(*id))
    {
        return Err("readiness investigation contains invalid catalog IDs".into());
    }
    Ok(plan)
}

fn validate_references(
    values: &[String],
    limit: usize,
    supplied: &HashSet<&str>,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    if values.len() > limit
        || values.iter().any(|value| {
            value.trim().is_empty()
                || value.chars().any(char::is_control)
                || !seen.insert(value.as_str())
                || !supplied.contains(value.as_str())
        })
    {
        return Err("discovery contains invalid evidence references".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn readiness_evidence_plans_are_closed_bounded_and_cannot_claim_candidates() {
        let allowed = vec![
            "concepts/verification.md".into(),
            "concepts/context-discovery.md".into(),
            "concepts/feedback-loops.md".into(),
        ];
        let plan = json!({"version":1,"conceptPageIds":[allowed[0]],"requestedFileIds":[0,7]});
        assert_eq!(
            parse_readiness_investigation(&plan.to_string(), &allowed, 8)
                .unwrap()
                .requested_file_ids,
            [0, 7]
        );
        let mut empty = plan.clone();
        empty["requestedFileIds"] = json!([]);
        assert!(parse_readiness_investigation(&empty.to_string(), &allowed, 0).is_ok());
        for (field, value) in [
            ("version", json!(2)),
            ("conceptPageIds", json!([])),
            ("conceptPageIds", json!([allowed[0], allowed[0]])),
            (
                "conceptPageIds",
                json!(["playbooks/agent-readiness-rubric.md"]),
            ),
            (
                "conceptPageIds",
                json!([
                    allowed[0],
                    allowed[1],
                    allowed[2],
                    "concepts/observability.md"
                ]),
            ),
            ("requestedFileIds", json!([8])),
            ("requestedFileIds", json!([0, 0])),
            ("requestedFileIds", json!([-1])),
            ("requestedFileIds", json!([0.5])),
            ("requestedFileIds", json!(["src/test.ts"])),
            ("requestedFileIds", json!([0, 1, 2, 3, 4, 5, 6, 7, 8])),
        ] {
            let mut bad = plan.clone();
            bad[field] = value;
            assert!(
                parse_readiness_investigation(&bad.to_string(), &allowed, 8).is_err(),
                "accepted {bad}"
            );
        }
        for field in ["candidate", "outcome", "reason", "paths", "commands"] {
            let mut bad = plan.clone();
            bad[field] = json!(null);
            assert!(parse_readiness_investigation(&bad.to_string(), &allowed, 8).is_err());
        }
        assert!(parse_readiness_investigation(r#"{"version":1,"version":1,"conceptPageIds":["concepts/verification.md"],"requestedFileIds":[]}"#,&allowed,8).is_err());
        assert!(
            parse_readiness_investigation(&" ".repeat(MAX_RESPONSE_BYTES + 1), &allowed, 8)
                .is_err()
        );
        let mut out_of_bounds = plan.clone();
        out_of_bounds["requestedFileIds"] = json!([512]);
        assert!(parse_readiness_investigation(&out_of_bounds.to_string(), &allowed, 1024).is_err());
        assert!(parse_readiness_investigation(&plan.to_string(), &[], 8).is_err());
        assert!(
            parse_discovery_response(&plan.to_string(), &facts(), &pages(), 8).is_err(),
            "standalone contract accepted readiness plan"
        );
    }

    fn facts() -> Vec<RepositoryEvidence> {
        vec![RepositoryEvidence {
            path: "docs/workflow.md".into(),
            sha256: "a".repeat(64),
            excerpt: "Bounded evidence".into(),
            observed_at: "2026-10-18T10:00:00Z".into(),
        }]
    }
    fn pages() -> Vec<KnowledgePage> {
        vec![KnowledgePage {
            id: "workflow/improvement".into(),
            title: "Workflow".into(),
            page_type: "concept".into(),
            status: "published".into(),
            content: "Guidance".into(),
            sha256: "b".repeat(64),
            related_ids: vec![],
        }]
    }
    fn inspect() -> Value {
        json!({"version":1,"outcome":"inspect","candidate":{"targetSurface":"documentation","title":"Clarify workflow","hypothesis":"The supplied CI configuration omits the documented check."},"repositoryFactHashes":["a".repeat(64)],"knowledgePageIds":["workflow/improvement"],"requestedFileIds":[0,2],"reason":null})
    }
    fn no_candidate() -> Value {
        json!({"version":1,"outcome":"no_candidate","candidate":null,"repositoryFactHashes":[],"knowledgePageIds":[],"requestedFileIds":[],"reason":"The available evidence supports no concrete improvement."})
    }
    fn parse(value: &Value) -> bool {
        parse_discovery_response(&value.to_string(), &facts(), &pages(), 8).is_ok()
    }

    #[test]
    fn accepts_grounded_inspect_and_explained_no_candidate() {
        assert!(parse(&inspect()));
        assert!(parse(&no_candidate()));
        let mut value = inspect();
        value["requestedFileIds"] = json!([]);
        assert!(parse(&value));
    }
    #[test]
    fn requires_all_fields_and_rejects_unknown_and_duplicate_keys() {
        for key in [
            "version",
            "outcome",
            "candidate",
            "repositoryFactHashes",
            "knowledgePageIds",
            "requestedFileIds",
            "reason",
        ] {
            let mut value = inspect();
            value.as_object_mut().unwrap().remove(key);
            assert!(!parse(&value), "missing {key}");
        }
        let mut value = inspect();
        value["extra"] = json!(true);
        assert!(!parse(&value));
        value = inspect();
        value["candidate"]["extra"] = json!(true);
        assert!(!parse(&value));
        for (needle, replacement) in [
            ("\"version\":1", "\"version\":1,\"version\":1"),
            ("\"reason\":null", "\"reason\":null,\"reason\":null"),
            (
                "\"title\":\"Clarify workflow\"",
                "\"title\":\"Clarify workflow\",\"title\":\"Clarify workflow\"",
            ),
        ] {
            let raw = inspect().to_string().replace(needle, replacement);
            assert!(parse_discovery_response(&raw, &facts(), &pages(), 8).is_err());
        }
    }
    #[test]
    fn rejects_invalid_shapes_and_outcome_combinations() {
        for (key, invalid) in [
            ("version", json!(2)),
            ("version", json!(1.0)),
            ("outcome", json!("proposed")),
            ("candidate", Value::Null),
            ("reason", json!("Unexpected reason")),
            ("repositoryFactHashes", json!([])),
            ("knowledgePageIds", json!([])),
            ("requestedFileIds", json!(["0"])),
            ("requestedFileIds", json!([-1])),
            ("requestedFileIds", json!([0.0])),
        ] {
            let mut value = inspect();
            value[key] = invalid;
            assert!(!parse(&value), "invalid {key}: {value}");
        }
        for (key, invalid) in [
            ("candidate", inspect()["candidate"].clone()),
            ("requestedFileIds", json!([0])),
            ("reason", Value::Null),
            ("reason", json!("  ")),
        ] {
            let mut value = no_candidate();
            value[key] = invalid;
            assert!(!parse(&value));
        }
        for raw in ["[]", "null", "```json {} ```", "{} trailing"] {
            assert!(parse_discovery_response(raw, &facts(), &pages(), 8).is_err());
        }
    }
    #[test]
    fn rejects_unknown_duplicate_and_out_of_bounds_references() {
        for (key, invalid) in [
            ("repositoryFactHashes", json!(["unknown"])),
            (
                "repositoryFactHashes",
                json!(["a".repeat(64), "a".repeat(64)]),
            ),
            ("knowledgePageIds", json!(["unknown"])),
            (
                "knowledgePageIds",
                json!(["workflow/improvement", "workflow/improvement"]),
            ),
            ("requestedFileIds", json!([0, 0])),
            ("requestedFileIds", json!([8])),
            ("requestedFileIds", json!([0, 1, 2, 3, 4, 5, 6, 7, 8])),
        ] {
            let mut value = inspect();
            value[key] = invalid;
            assert!(!parse(&value), "invalid {key}");
        }
        let mut value = inspect();
        value["requestedFileIds"] = json!([0]);
        assert!(parse_discovery_response(&value.to_string(), &facts(), &pages(), 0).is_err());
        value["requestedFileIds"] = json!([0, 1, 2, 3, 4, 5, 6, 7]);
        assert!(parse(&value));
    }
    #[test]
    fn accepts_descriptive_node_prose_in_discovery() {
        let text = "Node package management is required; desktop guidance differs.";
        for field in ["title", "hypothesis"] {
            let mut value = inspect();
            value["candidate"][field] = json!(text);
            assert!(parse(&value), "description rejected in {field}");
        }
        let mut value = no_candidate();
        value["reason"] = json!(text);
        assert!(parse(&value));
    }

    #[test]
    fn bounds_prose_by_unicode_scalars_and_rejects_unsafe_text() {
        for (field, limit) in [("title", 240), ("hypothesis", 1000)] {
            let mut value = inspect();
            value["candidate"][field] = json!("界".repeat(limit));
            assert!(parse(&value));
            value["candidate"][field] = json!("界".repeat(limit + 1));
            assert!(!parse(&value));
            for text in [
                "",
                "  ",
                "line\nbreak",
                "Run cargo test",
                "Use widget frobnicate",
                "$(cat /etc/passwd)",
                "All tests passed.",
            ] {
                value["candidate"][field] = json!(text);
                assert!(!parse(&value), "unsafe {field}: {text}");
            }
        }
        let mut value = no_candidate();
        value["reason"] = json!("界".repeat(1000));
        assert!(parse(&value));
        value["reason"] = json!("界".repeat(1001));
        assert!(!parse(&value));
        value["reason"] = json!("Run cargo test");
        assert!(!parse(&value));
        let mut value = inspect();
        value["candidate"]["targetSurface"] = json!("shell");
        assert!(!parse(&value));
        let raw = inspect().to_string();
        let padded = format!("{raw}{}", " ".repeat(65536 - raw.len()));
        assert!(parse_discovery_response(&padded, &facts(), &pages(), 8).is_ok());
        assert!(parse_discovery_response(&(padded + " "), &facts(), &pages(), 8).is_err());
    }
    #[test]
    fn accepts_reference_limits_and_rejects_one_more_supplied_reference() {
        let facts: Vec<_> = (0..33)
            .map(|index| RepositoryEvidence {
                sha256: format!("{index:064x}"),
                ..facts().remove(0)
            })
            .collect();
        let pages: Vec<_> = (0..9)
            .map(|index| KnowledgePage {
                id: format!("workflow/{index}"),
                ..pages().remove(0)
            })
            .collect();
        let mut value = inspect();
        value["repositoryFactHashes"] = json!(facts[..32]
            .iter()
            .map(|fact| &fact.sha256)
            .collect::<Vec<_>>());
        value["knowledgePageIds"] =
            json!(pages[..8].iter().map(|page| &page.id).collect::<Vec<_>>());
        assert!(parse_discovery_response(&value.to_string(), &facts, &pages, 8).is_ok());
        value["repositoryFactHashes"] =
            json!(facts.iter().map(|fact| &fact.sha256).collect::<Vec<_>>());
        assert!(parse_discovery_response(&value.to_string(), &facts, &pages, 8).is_err());
        value["repositoryFactHashes"] = json!([facts[0].sha256]);
        value["knowledgePageIds"] = json!(pages.iter().map(|page| &page.id).collect::<Vec<_>>());
        assert!(parse_discovery_response(&value.to_string(), &facts, &pages, 8).is_err());
    }
}
