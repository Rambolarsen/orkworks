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
    pub preferred_opportunity_id: Option<String>,
    pub no_proposal_reason: Option<String>,
    pub limitations: Vec<String>,
}
impl FinalResult {
    pub(crate) fn decode(
        text: &str,
        facts: &[String],
        pages: &[String],
        rubric: &crate::taskmaster::readiness_rubric::ReadinessRubric,
    ) -> Result<Self, String> {
        use crate::taskmaster::assessment_model::{validate_prose, validate_prospective_prose};
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
        validate_prose(&result.summary, 1000, true)?;
        for limit in &result.limitations {
            validate_prose(limit, 500, true)?;
        }
        let readiness =
            serde_json::json!({"version":1,"summary":result.summary,"concepts":value["concepts"]});
        // Same returned fact indices and same signed rubric as the preferred opportunity.
        crate::taskmaster::readiness_model::decode(&readiness.to_string(), facts.len(), rubric)?;
        let mut ids = HashSet::new();
        for opportunity in &result.opportunities {
            if opportunity.id.is_empty()
                || opportunity.id.len() > 64
                || !opportunity
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || !ids.insert(opportunity.id.as_str())
                || opportunity.repository_fact_hashes.is_empty()
                || opportunity.repository_fact_hashes.len() > 32
                || opportunity.knowledge_page_ids.is_empty()
                || opportunity.knowledge_page_ids.len() > 16
                || opportunity
                    .repository_fact_hashes
                    .iter()
                    .any(|id| !facts.contains(id))
                || opportunity
                    .knowledge_page_ids
                    .iter()
                    .any(|id| !pages.contains(id))
                || opportunity
                    .repository_fact_hashes
                    .iter()
                    .collect::<HashSet<_>>()
                    .len()
                    != opportunity.repository_fact_hashes.len()
                || opportunity
                    .knowledge_page_ids
                    .iter()
                    .collect::<HashSet<_>>()
                    .len()
                    != opportunity.knowledge_page_ids.len()
            {
                return Err("Improvement opportunity has invalid identity or citations".into());
            }
            validate_prose(&opportunity.title, 240, true)?;
            for text in [
                &opportunity.summary,
                &opportunity.mechanism,
                &opportunity.benefit,
                &opportunity.rollback,
            ] {
                validate_prospective_prose(text, 1000, true)?;
            }
            validate_prospective_prose(&opportunity.validation, 500, true)?;
        }
        match (&result.preferred_opportunity_id, &result.no_proposal_reason) {
            (Some(id), None) if ids.contains(id.as_str()) => {}
            (None, Some(reason)) => validate_prose(reason, 1000, true)?,
            _ => {
                return Err("Improvement preferred step or no-result reason is inconsistent".into())
            }
        }
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
