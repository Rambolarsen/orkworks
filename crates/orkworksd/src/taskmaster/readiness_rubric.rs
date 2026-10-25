//! Signed, complete Brain readiness rubric; never synthesized from retrieval.
use super::knowledge::KnowledgePage;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const CONCEPT_IDS: [&str; 14] = [
    "context-discovery",
    "progressive-disclosure",
    "instruction-scoping",
    "architecture-awareness",
    "domain-knowledge",
    "environment-reproducibility",
    "tool-accessibility",
    "task-decomposition",
    "verification",
    "observability",
    "change-safety",
    "feedback-loops",
    "durable-memory",
    "human-oversight",
];
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReadinessRubric {
    pub id: String,
    pub revision: u64,
    pub concepts: Vec<RubricConcept>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RubricConcept {
    pub concept_id: String,
    pub page_id: String,
    pub page_sha256: String,
    pub levels: Vec<String>,
}
impl ReadinessRubric {
    pub(crate) fn validate(&self, pages: &[KnowledgePage]) -> Result<(), String> {
        if self.id != "brain-agent-readiness"
            || self.revision == 0
            || self.revision > 9_007_199_254_740_991
            || self.concepts.len() != 14
            || self.canonical_bytes()?.len() > 40 * 1024
        {
            return Err("Readiness rubric identity or bounds are invalid".into());
        }
        for (concept, id) in self.concepts.iter().zip(CONCEPT_IDS) {
            if concept.concept_id != id
                || concept.levels.len() != 5
                || !pages
                    .iter()
                    .any(|page| page.id == concept.page_id && page.sha256 == concept.page_sha256)
                || concept.levels.iter().any(|text| {
                    text.trim().is_empty() || text.len() > 500 || text.chars().any(char::is_control)
                })
            {
                return Err("Readiness rubric mapping or levels are invalid".into());
            }
        }
        Ok(())
    }
    fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        // All object keys are ASCII and numbers are safe positive integers;
        // serde_json's ordered Value map matches RFC 8785 for this closed shape.
        let value = serde_json::to_value(self).map_err(|_| "Readiness rubric cannot be encoded")?;
        serde_json::to_vec(&value).map_err(|_| "Readiness rubric cannot be encoded".into())
    }
    pub(crate) fn digest(&self) -> Result<String, String> {
        Ok(hex::encode(Sha256::digest(self.canonical_bytes()?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::knowledge::KnowledgePage;
    fn page() -> KnowledgePage {
        serde_json::from_value(serde_json::json!({"id":"playbooks/rubric.md","title":"Rubric","type":"playbook","status":"active","content":"rubric","sha256":"a".repeat(64),"relatedIds":[]})).unwrap()
    }
    fn rubric() -> serde_json::Value {
        serde_json::json!({"id":"brain-agent-readiness","revision":1,"concepts":CONCEPT_IDS.iter().map(|id| serde_json::json!({"conceptId":id,"pageId":"playbooks/rubric.md","pageSha256":"a".repeat(64),"levels":["Implicit","Discoverable","Usable","Integrated","Adaptive"]})).collect::<Vec<_>>()})
    }
    #[test]
    fn readiness_rubric_requires_all_ordered_concepts_and_exact_signed_pages() {
        let value = rubric();
        let valid: ReadinessRubric = serde_json::from_value(value.clone()).unwrap();
        assert!(valid.validate(&[page()]).is_ok());
        let mut invalid = value.clone();
        invalid["concepts"][0]["pageSha256"] = serde_json::json!("b".repeat(64));
        let bad: ReadinessRubric = serde_json::from_value(invalid).unwrap();
        assert!(bad.validate(&[page()]).is_err());
        let mut invalid = value.clone();
        invalid["concepts"].as_array_mut().unwrap().pop();
        let bad: ReadinessRubric = serde_json::from_value(invalid).unwrap();
        assert!(bad.validate(&[page()]).is_err());
        let mut invalid = value;
        invalid["concepts"][0]["levels"] = serde_json::json!(["Implicit"]);
        let bad: ReadinessRubric = serde_json::from_value(invalid).unwrap();
        assert!(bad.validate(&[page()]).is_err());
    }
    #[test]
    fn readiness_rubric_rejects_unknown_fields() {
        let mut value = rubric();
        value["extra"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ReadinessRubric>(value).is_err());
    }
}
