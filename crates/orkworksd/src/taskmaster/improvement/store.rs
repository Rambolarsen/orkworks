//! Separate bounded report codec; old assessment records are never reinterpreted.
use super::model::FinalResult;
use crate::taskmaster::{
    assessment_context::improvement::{RangeFact, RangeIdentity},
    runtime::{ContextLevel, TaskmasterSettings},
    KnowledgeEvidence, Recommendation,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub(crate) const FILE: &str = "improvement-assessment.json";
pub(crate) const MAX_REPORT_BYTES: usize = 128 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunState {
    Queued,
    Running,
    PendingRecommendation,
    Succeeded,
    Failed,
    Interrupted,
    AccessRedacted,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PageIdentity {
    pub id: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Manifest {
    pub purpose: String,
    pub workspace_path: String,
    pub workspace_instance: u64,
    pub context_level: ContextLevel,
    pub excluded_paths: Vec<String>,
    pub evaluation_key: String,
    pub rubric_digest: String,
    pub catalog_digest: String,
    pub ranges: Vec<RangeIdentity>,
    pub pages: Vec<PageIdentity>,
    pub recommendation_digest: String,
}
impl Manifest {
    pub(crate) fn fingerprint(&self) -> Result<String, String> {
        crate::taskmaster::readiness_store::hash(self)
    }
    pub(crate) fn validate(&self) -> Result<(), String> {
        fn hash(s: &str) -> bool {
            s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
        }
        if crate::taskmaster::advice_text::TextPolicy::for_improvement_purpose(&self.purpose)
            .is_err()
            || self.workspace_path.is_empty()
            || self.context_level == ContextLevel::SessionObservations
            || self.ranges.len() > 32
            || self.pages.len() > 16
            || self.ranges.iter().any(|r| r.bytes > 8192)
            || self
                .ranges
                .iter()
                .try_fold(0usize, |total, range| total.checked_add(range.bytes))
                .is_none_or(|total| total > 65536)
            || self
                .ranges
                .iter()
                .map(|r| &r.path)
                .collect::<HashSet<_>>()
                .len()
                > 32
            || self
                .pages
                .iter()
                .map(|p| &p.id)
                .collect::<HashSet<_>>()
                .len()
                != self.pages.len()
            || [
                &self.evaluation_key,
                &self.rubric_digest,
                &self.catalog_digest,
                &self.recommendation_digest,
            ]
            .iter()
            .any(|s| !hash(s))
            || self.ranges.iter().any(|r| {
                !hash(&r.sha256)
                    || r.offset
                        .checked_add(r.bytes as u64)
                        .is_none_or(|end| end > r.file_size)
                    || !crate::taskmaster::context::fact_is_permitted(
                        &r.path,
                        self.context_level,
                        &self.excluded_paths,
                    )
            })
            || self.pages.iter().any(|p| !hash(&p.sha256))
        {
            return Err("Invalid improvement input manifest".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Report {
    pub manifest: Manifest,
    pub result: FinalResult,
    pub repository_evidence: Vec<RangeFact>,
    pub knowledge_evidence: Vec<KnowledgeEvidence>,
    pub recommendation_id: Option<String>,
    pub history_snapshot_id: Option<String>,
    pub history_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication_limitation: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicRange<'a> {
    #[serde(flatten)]
    fact: &'a RangeFact,
    complete: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicReport<'a> {
    summary: &'a str,
    concepts: Vec<crate::taskmaster::readiness_store::StoredRow>,
    opportunities: &'a [super::model::Opportunity],
    preferred_opportunity_id: &'a Option<String>,
    no_proposal_reason: &'a Option<String>,
    limitations: Vec<&'a str>,
    coverage: usize,
    overall_score: Option<u8>,
    recommendation_id: &'a Option<String>,
    history_snapshot_id: &'a Option<String>,
    history_error: &'a Option<String>,
    repository_evidence: Vec<PublicRange<'a>>,
    knowledge_evidence: &'a [KnowledgeEvidence],
}
impl Report {
    pub(crate) fn public(&self) -> Result<PublicReport<'_>, String> {
        let concepts = self
            .result
            .concepts
            .iter()
            .map(|row| {
                Ok(crate::taskmaster::readiness_store::StoredRow {
                    concept_id: row.concept_id.clone(),
                    maturity: row.maturity,
                    confidence: row.confidence.clone(),
                    repository_fact_hashes: row
                        .repository_fact_ids
                        .iter()
                        .map(|id| {
                            self.manifest
                                .ranges
                                .get(*id)
                                .map(|range| range.sha256.clone())
                                .ok_or("Improvement row citation unavailable")
                        })
                        .collect::<Result<_, _>>()?,
                    rubric_page_id: row.rubric_page_id.clone(),
                    rationale: row.rationale.clone(),
                    limits: row.limits.clone(),
                    next_transition: row.next_transition.clone(),
                    unassessed_reason: row.unassessed_reason.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let (coverage, _, overall_score) = self.result.readiness().totals();
        Ok(PublicReport {
            summary: &self.result.summary,
            concepts,
            opportunities: &self.result.opportunities,
            preferred_opportunity_id: &self.result.preferred_opportunity_id,
            no_proposal_reason: &self.result.no_proposal_reason,
            limitations: self
                .result
                .limitations
                .iter()
                .map(String::as_str)
                .chain(self.publication_limitation.as_deref())
                .collect(),
            coverage,
            overall_score,
            recommendation_id: &self.recommendation_id,
            history_snapshot_id: &self.history_snapshot_id,
            history_error: &self.history_error,
            repository_evidence: self
                .repository_evidence
                .iter()
                .map(|fact| PublicRange {
                    fact,
                    complete: fact.complete(),
                })
                .collect(),
            knowledge_evidence: &self.knowledge_evidence,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    CollectingEvidence,
    ReadingEvidence,
    Assessing,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Record {
    pub format_version: u8,
    pub id: String,
    pub state: RunState,
    #[serde(default)]
    pub phase: Option<Phase>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub report: Option<Report>,
    pub pending_recommendation: Option<Recommendation>,
    pub error_code: Option<String>,
}
impl Record {
    pub(crate) fn queued(id: &str, started: &str) -> Self {
        Self {
            format_version: 1,
            id: id.into(),
            state: RunState::Queued,
            phase: Some(Phase::CollectingEvidence),
            started_at: started.into(),
            completed_at: None,
            report: None,
            pending_recommendation: None,
            error_code: None,
        }
    }
    pub(crate) fn redact(&mut self) {
        self.report = None;
        self.pending_recommendation = None;
        self.error_code = Some("access_changed".into());
        self.state = RunState::AccessRedacted;
        self.phase = None;
    }
    fn validate(&self) -> Result<(), String> {
        if self.format_version != 1
            || self.id.is_empty()
            || self.id.len() > 128
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || self.error_code.as_deref().is_some_and(|code| {
                !matches!(
                    code,
                    "access_changed"
                        | "cancelled"
                        | "inputs_changed"
                        | "deadline"
                        | "investigation_failed"
                        | "interrupted"
                )
            })
            || self.started_at.is_empty()
            || self.started_at.len() > 40
            || self.started_at.chars().any(char::is_control)
            || self
                .completed_at
                .as_ref()
                .is_some_and(|s| s.len() > 40 || s.chars().any(char::is_control))
            || match self.state {
                RunState::Queued | RunState::Running => {
                    self.phase.is_none()
                        || self.completed_at.is_some()
                        || self.error_code.is_some()
                        || self.report.is_some()
                        || self.pending_recommendation.is_some()
                }
                RunState::PendingRecommendation => {
                    self.phase.is_some()
                        || self.completed_at.is_none()
                        || self.error_code.is_some()
                        || self.report.is_none()
                        || self.pending_recommendation.is_none()
                }
                RunState::Succeeded => {
                    self.phase.is_some()
                        || self.completed_at.is_none()
                        || self.error_code.is_some()
                        || self.report.is_none()
                        || self.pending_recommendation.is_some()
                }
                RunState::Failed | RunState::Interrupted => {
                    self.phase.is_some()
                        || self.completed_at.is_none()
                        || self.error_code.is_none()
                        || self.report.is_some()
                        || self.pending_recommendation.is_some()
                }
                RunState::AccessRedacted => {
                    self.phase.is_some()
                        || self.report.is_some()
                        || self.pending_recommendation.is_some()
                        || self.error_code.as_deref() != Some("access_changed")
                }
            }
        {
            return Err("Invalid improvement report state".into());
        }
        if let Some(report) = &self.report {
            report.manifest.validate()?;
            let policy = crate::taskmaster::advice_text::TextPolicy::for_improvement_purpose(
                &report.manifest.purpose,
            )?;
            report.result.validate_text(policy)?;
            report.result.validate_citations(
                &report
                    .manifest
                    .ranges
                    .iter()
                    .map(|r| r.sha256.clone())
                    .collect::<Vec<_>>(),
                &report
                    .manifest
                    .pages
                    .iter()
                    .map(|p| p.id.clone())
                    .collect::<Vec<_>>(),
            )?;
            if report
                .publication_limitation
                .as_deref()
                .is_some_and(|s| s != "An existing recommendation prevented a new card.")
                || report
                    .history_error
                    .as_deref()
                    .is_some_and(|s| !matches!(s, "history_pending" | "history_unavailable"))
                || report.repository_evidence.len() > 32
                || report.knowledge_evidence.len() > 16
            {
                return Err("Improvement cited evidence exceeds limits".into());
            }
            for fact in &report.repository_evidence {
                if fact.identity()? != fact.sha256
                    || !report.manifest.ranges.contains(&RangeIdentity::from(fact))
                {
                    return Err("Improvement cited range identity mismatch".into());
                }
            }
            for page in &report.knowledge_evidence {
                if !report
                    .manifest
                    .pages
                    .iter()
                    .any(|p| p.id == page.page_id && p.sha256 == page.sha256)
                {
                    return Err("Improvement cited page identity mismatch".into());
                }
            }
            if report.recommendation_id.is_some() && report.result.preferred().is_none() {
                return Err("Improvement recommendation lacks a preferred opportunity".into());
            }
            if let Some(pending) = &self.pending_recommendation {
                if !super::publication::join_valid(report, &self.id, pending) {
                    return Err("Improvement pending publication identity mismatch".into());
                }
            }
        }
        Ok(())
    }
}

/// Persist the pending marker before publishing a recommendation. A failure
/// keeps the exact pending value available for idempotent recovery.
pub(crate) fn publish_with(
    record: &mut Record,
    mut save: impl FnMut(&Record) -> Result<(), String>,
    upsert: impl FnMut(&Recommendation) -> Result<(), String>,
) -> Result<(), String> {
    if record.pending_recommendation.is_some() {
        let mut pending = record.clone();
        pending.state = RunState::PendingRecommendation;
        pending.phase = None;
        save(&pending)?;
        *record = pending;
        return recover_with(record, save, upsert);
    }
    let mut completed = record.clone();
    completed.state = RunState::Succeeded;
    completed.phase = None;
    save(&completed)?;
    *record = completed;
    Ok(())
}

pub(crate) fn recover_with(
    record: &mut Record,
    mut save: impl FnMut(&Record) -> Result<(), String>,
    mut upsert: impl FnMut(&Recommendation) -> Result<(), String>,
) -> Result<(), String> {
    if record.state != RunState::PendingRecommendation {
        return Err("Improvement publication is not pending".into());
    }
    let recommendation = record
        .pending_recommendation
        .as_ref()
        .ok_or("Improvement recommendation is missing")?;
    upsert(recommendation)?;
    let mut completed = record.clone();
    completed.pending_recommendation = None;
    completed.state = RunState::Succeeded;
    completed.phase = None;
    save(&completed)?;
    *record = completed;
    Ok(())
}

pub(crate) struct ReportStore {
    root: PathBuf,
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
impl ReportStore {
    pub(crate) fn new(root: &Path) -> Self {
        Self { root: root.into() }
    }
    pub(crate) fn load(&self) -> Result<Option<Record>, String> {
        let record: Option<Record> =
            crate::taskmaster::readiness_store::ReadinessStore::new(&self.root)
                .read(FILE, MAX_REPORT_BYTES)?;
        if let Some(record) = &record {
            record.validate()?;
        }
        Ok(record)
    }
    pub(crate) fn save(&self, record: &Record) -> Result<(), String> {
        record.validate()?;
        crate::taskmaster::readiness_store::ReadinessStore::new(&self.root).write(
            FILE,
            record,
            MAX_REPORT_BYTES,
        )
    }
    pub(crate) fn reconcile(&self, settings: &TaskmasterSettings) -> Result<(), String> {
        if let Some(mut record) = self.load()? {
            if record.report.as_ref().is_some_and(|r| {
                crate::taskmaster::assessment_access::access_narrowed(
                    r.manifest.context_level,
                    &r.manifest.excluded_paths,
                    settings,
                )
            }) {
                record.redact();
                self.save(&record)?;
            }
        }
        Ok(())
    }
}
