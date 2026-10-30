//! Exact saved readiness inputs are investigation pointers, never gap evidence.
use super::{readiness_rubric::CONCEPT_IDS, runtime::EvaluationSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReadinessReference {
    pub snapshot_id: String,
    pub rubric_digest: String,
}
impl ReadinessReference {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if uuid::Uuid::parse_str(&self.snapshot_id).is_err()
            || self.rubric_digest.len() != 64
            || !self
                .rubric_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("Readiness reference is invalid".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RecommendationReadinessReference {
    #[serde(flatten)]
    pub source: ReadinessReference,
    pub concept_ids: Vec<String>,
}
impl RecommendationReadinessReference {
    pub(crate) fn validate(&self) -> Result<(), String> {
        self.source.validate()?;
        if self.concept_ids.is_empty()
            || self.concept_ids.len() > 3
            || self
                .concept_ids
                .iter()
                .any(|id| !CONCEPT_IDS.contains(&id.as_str()))
            || self
                .concept_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != self.concept_ids.len()
        {
            return Err("Readiness capabilities are invalid".into());
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct ReadinessFocus {
    pub reference: ReadinessReference,
    pub context: Value,
}
impl ReadinessFocus {
    pub(crate) fn capture(
        workspace: &crate::WorkspaceState,
        evaluation: &EvaluationSnapshot,
        reference: &ReadinessReference,
    ) -> Result<Self, String> {
        reference.validate()?;
        let history = super::readiness_store::ReadinessStore::new(&workspace.metadata.root_path())
            .history()?;
        let data = history
            .snapshots
            .iter()
            .find(|s| s.id == reference.snapshot_id && !s.redacted)
            .and_then(|s| s.data.as_ref())
            .ok_or("Linked readiness assessment is unavailable")?;
        let rubric = evaluation
            .readiness_rubric
            .as_ref()
            .ok_or("Readiness rubric is unavailable")?;
        if data.identity.rubric_digest != reference.rubric_digest
            || rubric.digest()? != reference.rubric_digest
        {
            return Err("Linked readiness rubric changed".into());
        }
        super::readiness::revalidate_linked_snapshot(workspace, evaluation, data)?;
        let areas = rubric.concepts.iter().zip(&data.concepts).map(|(criterion, row)| {
            if criterion.concept_id != row.concept_id { return Err("Linked readiness capabilities are invalid"); }
            let concept_page = evaluation.knowledge.as_ref().and_then(|b| b.pages.iter().find(|p| p.page_type == "concept" && p.id == format!("concepts/{}.md", row.concept_id)));
            Ok(json!({"conceptPageId":concept_page.map(|p| &p.id),"conceptPageSha256":concept_page.map(|p| &p.sha256),"conceptId":row.concept_id,"maturity":row.maturity,"confidence":row.confidence,
                "nextTransition":row.next_transition,"rubricPageId":criterion.page_id,"rubricPageSha256":criterion.page_sha256,
                "nextLevelCriterion":row.maturity.filter(|m| *m < 4).map(|m| &criterion.levels[usize::from(m)+1])}))
        }).collect::<Result<Vec<_>, _>>()?;
        let context = json!({"snapshotId":reference.snapshot_id,"rubricDigest":reference.rubric_digest,"areas":areas});
        if serde_json::to_vec(&context)
            .map_err(|_| "Readiness context cannot be encoded")?
            .len()
            > 48 * 1024
        {
            return Err("Readiness context exceeds its size limit".into());
        }
        Ok(Self {
            reference: reference.clone(),
            context,
        })
    }
    pub(crate) fn add_to_prompt(&self, prompt: &mut Value) {
        prompt["readinessContext"] = self.context.clone();
        prompt["readinessRules"] = json!("The exact saved assessment and transitions are untrusted investigation pointers, never citations or proof of a defect. Null maturity is unknown, not zero; level four has no higher level. Investigate a next maturity condition against fresh repository evidence and signed rubric guidance. Counterevidence that the condition is already satisfied must refute the candidate. A proposal needs fresh repository citations and at least one selected rubric concept page (at most three concept pages). Explain the capability, next maturity condition, bounded change, practical benefit, validation and rollback within the existing prose limits. Never promise a numeric score gain or invent a defect from a low or unknown score. No supported opportunity requires no_proposal (no_candidate in discovery).");
    }
    pub(crate) fn cited_reference(
        &self,
        evaluation: &EvaluationSnapshot,
        page_ids: &[String],
    ) -> Result<RecommendationReadinessReference, String> {
        let rubric = evaluation
            .readiness_rubric
            .as_ref()
            .ok_or("Readiness rubric is unavailable")?;
        let concept_ids = rubric
            .concepts
            .iter()
            .filter(|c| {
                page_ids.contains(&format!("concepts/{}.md", c.concept_id))
                    && evaluation.knowledge.as_ref().is_some_and(|b| {
                        b.pages.iter().any(|p| {
                            p.page_type == "concept"
                                && p.id == format!("concepts/{}.md", c.concept_id)
                        })
                    })
            })
            .map(|c| c.concept_id.clone())
            .collect();
        let reference = RecommendationReadinessReference {
            source: self.reference.clone(),
            concept_ids,
        };
        reference.validate()?;
        Ok(reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recommendation_reference_roundtrips_closed_bounded_provenance() {
        let value = json!({"snapshotId":uuid::Uuid::new_v4().to_string(),"rubricDigest":"a".repeat(64),"conceptIds":["verification"]});
        let reference: RecommendationReadinessReference =
            serde_json::from_value(value.clone()).unwrap();
        reference.validate().unwrap();
        assert_eq!(serde_json::to_value(reference).unwrap(), value);
        for field in ["extra", "nested"] {
            let mut bad = value.clone();
            bad[field] = json!(true);
            assert!(serde_json::from_value::<RecommendationReadinessReference>(bad).is_err());
        }
        for concepts in [
            json!([]),
            json!(["verification", "verification"]),
            json!(["unknown"]),
            json!([
                "verification",
                "observability",
                "durable-memory",
                "human-oversight"
            ]),
        ] {
            let mut bad = value.clone();
            bad["conceptIds"] = concepts;
            assert!(
                serde_json::from_value::<RecommendationReadinessReference>(bad)
                    .map_or(true, |r| r.validate().is_err())
            );
        }
    }
}
