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
    fn validate(&self) -> Result<(), String> {
        fn hash(s: &str) -> bool {
            s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
        }
        if self.purpose != "brain-improvement-v1"
            || self.workspace_path.is_empty()
            || self.context_level == ContextLevel::SessionObservations
            || self.ranges.len() > 32
            || self.pages.len() > 16
            || self.ranges.iter().map(|r| r.bytes).sum::<usize>() > 65536
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
                r.bytes > 8192
                    || !hash(&r.sha256)
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
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Record {
    pub format_version: u8,
    pub id: String,
    pub state: RunState,
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
    }
    fn validate(&self) -> Result<(), String> {
        if self.format_version != 1
            || self.id.is_empty()
            || self.id.len() > 128
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || self
                .error_code
                .as_ref()
                .is_some_and(|s| s.len() > 256 || s.chars().any(char::is_control))
            || (self.state == RunState::Succeeded
                && (self.report.is_none() || self.pending_recommendation.is_some()))
            || (self.state == RunState::PendingRecommendation
                && (self.report.is_none() || self.pending_recommendation.is_none()))
            || (matches!(
                self.state,
                RunState::Queued | RunState::Running | RunState::AccessRedacted
            ) && (self.report.is_some() || self.pending_recommendation.is_some()))
        {
            return Err("Invalid improvement report state".into());
        }
        if let Some(report) = &self.report {
            report.manifest.validate()?;
            if report.repository_evidence.len() > 32 || report.knowledge_evidence.len() > 16 {
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
            if let Some(pending) = &self.pending_recommendation {
                if report.recommendation_id.as_deref() != Some(&pending.id) {
                    return Err("Improvement pending publication identity mismatch".into());
                }
            }
        }
        Ok(())
    }
}

pub(crate) struct ReportStore {
    root: PathBuf,
}
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
