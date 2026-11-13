//! Strict, inert evidence requests and the finite invocation budget.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

pub(crate) const MAX_CALLS: usize = 6;
pub(crate) const MAX_READS: usize = 8;
pub(crate) const MAX_RANGE_BYTES: usize = 8 * 1024;
pub(crate) const RUN_TIMEOUT: Duration = Duration::from_secs(480);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepositoryRead {
    pub file_id: usize,
    pub offset: u64,
    pub length: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InvestigationStep {
    pub version: u8,
    pub repository_reads: Vec<RepositoryRead>,
    pub knowledge_page_ids: Vec<String>,
    pub finish: bool,
}
impl InvestigationStep {
    pub(crate) fn decode(text: &str, files: usize, pages: &[String]) -> Result<Self, String> {
        let step: Self = crate::harness::definition::parse_strict_json(text.as_bytes(), 64 * 1024)
            .map_err(|_| "Invalid improvement evidence response")?;
        if step.version != 1
            || step.repository_reads.len() + step.knowledge_page_ids.len() > MAX_READS
            || step.repository_reads.iter().any(|r| {
                r.file_id >= files
                    || r.length == 0
                    || r.length > MAX_RANGE_BYTES
                    || r.offset.checked_add(r.length as u64).is_none()
            })
            || step.repository_reads.iter().collect::<HashSet<_>>().len()
                != step.repository_reads.len()
            || step.knowledge_page_ids.iter().collect::<HashSet<_>>().len()
                != step.knowledge_page_ids.len()
            || step.knowledge_page_ids.iter().any(|id| !pages.contains(id))
            || (step.finish
                && (!step.repository_reads.is_empty() || !step.knowledge_page_ids.is_empty()))
            || (!step.finish
                && step.repository_reads.is_empty()
                && step.knowledge_page_ids.is_empty())
        {
            return Err("Invalid improvement evidence selection".into());
        }
        Ok(step)
    }
}

pub(crate) struct CallBudget {
    started: Instant,
    calls: usize,
    finalized: bool,
}
impl CallBudget {
    pub(crate) fn new(started: Instant) -> Self {
        Self {
            started,
            calls: 0,
            finalized: false,
        }
    }
    pub(crate) fn reserve(&mut self, now: Instant, final_call: bool) -> Result<(), String> {
        if self.finalized
            || now.saturating_duration_since(self.started) >= RUN_TIMEOUT
            || self.calls >= MAX_CALLS
            || (!final_call && self.calls >= MAX_CALLS - 1)
        {
            return Err("Improvement investigation call or time limit reached".into());
        }
        self.calls += 1;
        self.finalized = final_call;
        Ok(())
    }
    pub(crate) fn remaining(&self, now: Instant) -> Duration {
        RUN_TIMEOUT.saturating_sub(now.saturating_duration_since(self.started))
    }
    pub(crate) fn final_due(&self) -> bool {
        self.calls >= MAX_CALLS - 1
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceStrength {
    ObservedProblem,
    SupportedHypothesis,
    InvestigationNeeded,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Ranking {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Opportunity {
    pub id: String,
    pub target_surface: crate::taskmaster::TargetSurface,
    pub title: String,
    pub summary: String,
    pub mechanism: String,
    pub benefit: String,
    pub evidence_strength: EvidenceStrength,
    pub effort: Ranking,
    pub risk: Ranking,
    pub reversibility: Ranking,
    pub repository_fact_hashes: Vec<String>,
    pub knowledge_page_ids: Vec<String>,
    pub validation: String,
    pub rollback: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FinalResult {
    pub version: u8,
    pub summary: String,
    pub concepts: Vec<crate::taskmaster::readiness_model::ReadinessRow>,
    pub opportunities: Vec<Opportunity>,
    #[serde(deserialize_with = "crate::taskmaster::readiness_model::required_nullable")]
    pub preferred_opportunity_id: Option<String>,
    #[serde(deserialize_with = "crate::taskmaster::readiness_model::required_nullable")]
    pub no_proposal_reason: Option<String>,
    pub limitations: Vec<String>,
}
impl FinalResult {
    pub(crate) fn validate_text(
        &self,
        policy: crate::taskmaster::advice_text::TextPolicy,
    ) -> Result<(), String> {
        if self.version != 1
            || self.opportunities.len() > 5
            || self.limitations.len() > 8
            || self.concepts.len() != 14
        {
            return Err("Improvement result exceeds its bounds".into());
        }
        policy.validate(&self.summary, 1000, true, false)?;
        for text in &self.limitations {
            policy.validate(text, 500, true, false)?;
        }
        if let Some(text) = &self.no_proposal_reason {
            policy.validate(text, 1000, true, false)?;
        }
        for row in &self.concepts {
            policy.validate(&row.rationale, 500, true, false)?;
            policy.validate(&row.next_transition, 500, false, true)?;
            if row.limits.len() > 4 {
                return Err("Invalid readiness limits".into());
            }
            for text in &row.limits {
                policy.validate(text, 200, true, false)?;
            }
            if let Some(text) = &row.unassessed_reason {
                policy.validate(text, 500, true, false)?;
            }
        }
        for item in &self.opportunities {
            policy.validate(&item.title, 240, true, false)?;
            for text in [
                &item.summary,
                &item.mechanism,
                &item.benefit,
                &item.rollback,
            ] {
                policy.validate(text, 1000, true, true)?;
            }
            policy.validate(&item.validation, 500, true, true)?;
        }
        Ok(())
    }

    /// Persisted reports lack the full rubric, but every citation must still
    /// belong to their captured manifest. Publication also checks the signed rubric.
    pub(crate) fn validate_citations(
        &self,
        facts: &[String],
        pages: &[String],
    ) -> Result<(), String> {
        use crate::taskmaster::readiness_rubric::CONCEPT_IDS;
        if self.concepts.len() != CONCEPT_IDS.len() {
            return Err("Invalid readiness coverage".into());
        }
        for (row, id) in self.concepts.iter().zip(CONCEPT_IDS) {
            if row.concept_id != id
                || !pages.contains(&row.rubric_page_id)
                || row.repository_fact_ids.len() > 8
                || row.repository_fact_ids.iter().any(|id| *id >= facts.len())
                || row.repository_fact_ids.iter().collect::<HashSet<_>>().len()
                    != row.repository_fact_ids.len()
            {
                return Err("Invalid readiness citations or concept".into());
            }
            match (&row.maturity, &row.confidence, &row.unassessed_reason) {
                (Some(0..=4), Some(_), None) if !row.repository_fact_ids.is_empty() => {}
                (None, None, Some(reason)) if !reason.trim().is_empty() => {}
                _ => return Err("Invalid readiness rating or unassessed reason".into()),
            }
        }
        let mut ids = HashSet::new();
        for item in &self.opportunities {
            if item.id.is_empty()
                || item.id.len() > 64
                || !item
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
                || !ids.insert(item.id.as_str())
                || item.repository_fact_hashes.is_empty()
                || item.repository_fact_hashes.len() > 32
                || item.knowledge_page_ids.is_empty()
                || item.knowledge_page_ids.len() > 16
                || item
                    .repository_fact_hashes
                    .iter()
                    .any(|id| !facts.contains(id))
                || item.knowledge_page_ids.iter().any(|id| !pages.contains(id))
                || item
                    .repository_fact_hashes
                    .iter()
                    .collect::<HashSet<_>>()
                    .len()
                    != item.repository_fact_hashes.len()
                || item.knowledge_page_ids.iter().collect::<HashSet<_>>().len()
                    != item.knowledge_page_ids.len()
            {
                return Err("Improvement opportunity has invalid identity or citations".into());
            }
        }
        match (&self.preferred_opportunity_id, &self.no_proposal_reason) {
            (Some(id), None) if ids.contains(id.as_str()) => Ok(()),
            (None, Some(_)) => Ok(()),
            _ => Err("Improvement preferred step or no-result reason is inconsistent".into()),
        }
    }

    pub(crate) fn decode(
        text: &str,
        facts: &[String],
        pages: &[String],
        rubric: &crate::taskmaster::readiness_rubric::ReadinessRubric,
    ) -> Result<Self, String> {
        Self::decode_with_policy(
            text,
            facts,
            pages,
            rubric,
            crate::taskmaster::advice_text::TextPolicy::InertAdvice,
        )
    }

    pub(crate) fn decode_with_policy(
        text: &str,
        facts: &[String],
        pages: &[String],
        rubric: &crate::taskmaster::readiness_rubric::ReadinessRubric,
        policy: crate::taskmaster::advice_text::TextPolicy,
    ) -> Result<Self, String> {
        let value: serde_json::Value =
            crate::harness::definition::parse_strict_json(text.as_bytes(), 64 * 1024)
                .map_err(|_| "Invalid improvement result JSON")?;
        if ["preferredOpportunityId", "noProposalReason"]
            .iter()
            .any(|key| value.get(*key).is_none())
        {
            return Err("Missing improvement result fields".into());
        }
        let result: Self =
            crate::harness::definition::parse_strict_json(text.as_bytes(), 64 * 1024)
                .map_err(|_| "Invalid improvement result shape")?;
        if result.version != 1 || result.opportunities.len() > 5 || result.limitations.len() > 8 {
            return Err("Improvement result exceeds its bounds".into());
        }
        result.validate_text(policy)?;
        result.validate_citations(facts, pages)?;
        let readiness =
            serde_json::json!({"version":1,"summary":result.summary,"concepts":value["concepts"]});
        crate::taskmaster::readiness_model::decode_with_policy(
            &readiness.to_string(),
            facts.len(),
            rubric,
            policy,
        )?;
        Ok(result)
    }
    pub(crate) fn readiness(&self) -> crate::taskmaster::readiness_model::ReadinessResponse {
        crate::taskmaster::readiness_model::ReadinessResponse {
            version: 1,
            summary: self.summary.clone(),
            concepts: self.concepts.clone(),
        }
    }
    pub(crate) fn preferred(&self) -> Option<&Opportunity> {
        self.preferred_opportunity_id
            .as_deref()
            .and_then(|id| self.opportunities.iter().find(|o| o.id == id))
    }
}
