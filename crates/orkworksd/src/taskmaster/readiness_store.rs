//! Bounded atomic history and separate restart-safe readiness attempt state.
use super::readiness_model::{Confidence, ReadinessResponse};
use super::{
    assessment_context::InputManifest, runtime::ContextLevel, KnowledgeEvidence, RepositoryEvidence,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub(crate) const MAX_SNAPSHOT: usize = 64 * 1024;
pub(crate) const MAX_HISTORY: usize = 8 * 1024 * 1024;
pub(crate) fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
pub(crate) fn hash(value: &impl Serialize) -> Result<String, String> {
    let value = serde_json::to_value(value).map_err(|_| "Readiness identity cannot be encoded")?;
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(&value).map_err(|_| "Readiness identity cannot be encoded")?,
    )))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GitIdentity {
    pub head_oid: Option<String>,
    pub dirty: Option<bool>,
}
impl GitIdentity {
    pub(crate) fn read(path: &Path) -> Self {
        let Ok(repo) = git2::Repository::discover(path) else {
            return Self {
                head_oid: None,
                dirty: None,
            };
        };
        let head_oid = repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .map(|oid| oid.to_string());
        let mut options = git2::StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);
        let dirty = repo
            .statuses(Some(&mut options))
            .ok()
            .map(|statuses| !statuses.is_empty());
        Self { head_oid, dirty }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Manifest {
    pub version: u8,
    pub purpose: String,
    pub collector: InputManifest,
    pub observations_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InputIdentity {
    pub workspace_path: String,
    pub workspace_instance: u64,
    pub git: GitIdentity,
    pub input_fingerprint: String,
    pub comparison_fingerprint: String,
    pub input_manifest: Manifest,
    pub inference_identity: serde_json::Value,
    pub knowledge_identity: serde_json::Value,
    pub rubric_digest: String,
    pub versions: serde_json::Value,
    pub context_level: ContextLevel,
    pub excluded_paths: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StoredRow {
    pub concept_id: String,
    pub maturity: Option<u8>,
    pub confidence: Option<Confidence>,
    pub repository_fact_hashes: Vec<String>,
    pub rubric_page_id: String,
    pub rationale: String,
    pub limits: Vec<String>,
    pub next_transition: String,
    pub unassessed_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SavedFact {
    pub fact_id: String,
    #[serde(flatten)]
    pub fact: RepositoryEvidence,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SnapshotData {
    #[serde(flatten)]
    pub identity: InputIdentity,
    pub summary: String,
    pub concepts: Vec<StoredRow>,
    pub repository_evidence: Vec<SavedFact>,
    pub knowledge_evidence: Vec<KnowledgeEvidence>,
    pub coverage: usize,
    pub raw_total: Option<u16>,
    pub overall_score: Option<u8>,
}
impl SnapshotData {
    pub(crate) fn from_response(
        identity: InputIdentity,
        response: ReadinessResponse,
        facts: Vec<RepositoryEvidence>,
        knowledge: Vec<KnowledgeEvidence>,
    ) -> Result<Self, String> {
        let (coverage, raw_total, overall_score) = response.totals();
        let concepts = response
            .concepts
            .into_iter()
            .map(|row| {
                Ok(StoredRow {
                    concept_id: row.concept_id,
                    maturity: row.maturity,
                    confidence: row.confidence,
                    repository_fact_hashes: row
                        .repository_fact_ids
                        .into_iter()
                        .map(|id| fact_id(facts.get(id).ok_or("Unknown readiness fact")?))
                        .collect::<Result<_, String>>()?,
                    rubric_page_id: row.rubric_page_id,
                    rationale: row.rationale,
                    limits: row.limits,
                    next_transition: row.next_transition,
                    unassessed_reason: row.unassessed_reason,
                })
            })
            .collect::<Result<_, String>>()?;
        let repository_evidence = facts
            .into_iter()
            .map(|fact| {
                Ok(SavedFact {
                    fact_id: fact_id(&fact)?,
                    fact,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            identity,
            summary: response.summary,
            concepts,
            repository_evidence,
            knowledge_evidence: knowledge,
            coverage,
            raw_total,
            overall_score,
        })
    }
}
pub(crate) fn fact_id(fact: &RepositoryEvidence) -> Result<String, String> {
    hash(&(&fact.path, &fact.sha256, &fact.excerpt))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub id: String,
    pub started_at: String,
    pub completed_at: String,
    pub redacted: bool,
    pub data: Option<SnapshotData>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct History {
    pub format_version: u8,
    pub snapshots: Vec<Snapshot>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    CollectingEvidence,
    ReadingEvidence,
    ScoringReadiness,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Run {
    pub format_version: u8,
    pub id: String,
    pub state: RunState,
    pub phase: Option<Phase>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub input_identity: Option<InputIdentity>,
    pub error_code: Option<String>,
}
impl Run {
    pub(crate) fn queued() -> Self {
        Self {
            format_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            state: RunState::Queued,
            phase: Some(Phase::CollectingEvidence),
            started_at: now(),
            completed_at: None,
            input_identity: None,
            error_code: None,
        }
    }
    pub(crate) fn finish(&mut self, state: RunState, code: Option<&str>) {
        self.state = state;
        self.phase = None;
        self.completed_at = Some(now());
        self.error_code = code.map(str::to_owned);
        self.input_identity = None;
    }
}
pub(crate) struct ReadinessStore {
    root: PathBuf,
}
impl ReadinessStore {
    pub(crate) fn new(root: &Path) -> Self {
        Self { root: root.into() }
    }
    fn read<T: DeserializeOwned>(&self, name: &str, cap: usize) -> Result<Option<T>, String> {
        let path = self.root.join(name);
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("Readiness metadata is unavailable".into()),
        };
        let mut bytes = Vec::new();
        file.take((cap + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "Readiness metadata is unavailable")?;
        crate::harness::definition::parse_strict_json(&bytes, cap)
            .map(Some)
            .map_err(|_| "Readiness metadata is invalid".into())
    }
    fn write(&self, name: &str, value: &impl Serialize, cap: usize) -> Result<(), String> {
        let bytes =
            serde_json::to_vec(value).map_err(|_| "Readiness metadata cannot be encoded")?;
        if bytes.len() > cap {
            return Err("Readiness metadata exceeds its size limit".into());
        }
        let mut tmp = tempfile::NamedTempFile::new_in(&self.root)
            .map_err(|_| "Readiness metadata could not be staged")?;
        tmp.write_all(&bytes)
            .and_then(|_| tmp.as_file().sync_all())
            .map_err(|_| "Readiness metadata could not be staged")?;
        let path = self.root.join(name);
        crate::harness::integration::atomic_replace(tmp.path(), &path, path.exists())
            .map_err(|_| "Readiness metadata could not be committed")?;
        #[cfg(not(windows))]
        std::fs::File::open(&self.root)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| "Readiness metadata could not be synced")?;
        Ok(())
    }
    pub(crate) fn history(&self) -> Result<History, String> {
        let value = self
            .read::<History>("readiness-history.json", MAX_HISTORY)?
            .unwrap_or(History {
                format_version: 1,
                snapshots: vec![],
            });
        if value.format_version != 1
            || value.snapshots.len() > 100
            || value
                .snapshots
                .iter()
                .map(|s| &s.id)
                .collect::<HashSet<_>>()
                .len()
                != value.snapshots.len()
        {
            return Err("Readiness history is invalid".into());
        }
        for snapshot in &value.snapshots {
            validate_snapshot(snapshot)?;
        }
        Ok(value)
    }
    pub(crate) fn append(&self, snapshot: Snapshot) -> Result<(), String> {
        validate_snapshot(&snapshot)?;
        let mut history = self.history()?;
        if let Some(prior) = history.snapshots.iter().find(|s| s.id == snapshot.id) {
            return if prior == &snapshot {
                Ok(())
            } else {
                Err("Conflicting readiness completion".into())
            };
        }
        history.snapshots.push(snapshot);
        while history.snapshots.len() > 100
            || serde_json::to_vec(&history)
                .map_err(|_| "Readiness history cannot be encoded")?
                .len()
                > MAX_HISTORY
        {
            history.snapshots.remove(0);
        }
        self.write("readiness-history.json", &history, MAX_HISTORY)
    }
    pub(crate) fn run(&self) -> Result<Option<Run>, String> {
        let run = self.read::<Run>("readiness-run.json", MAX_SNAPSHOT)?;
        if let Some(run) = &run {
            validate_run(run)?;
        }
        Ok(run)
    }
    pub(crate) fn save_run(&self, run: &Run) -> Result<(), String> {
        validate_run(run)?;
        self.write("readiness-run.json", run, MAX_SNAPSHOT)
    }
    pub(crate) fn recover(&self) -> Result<(), String> {
        let history = self.history()?;
        if let Some(mut run) = self.run()? {
            if matches!(run.state, RunState::Queued | RunState::Running) {
                if history.snapshots.iter().any(|s| s.id == run.id) {
                    run.finish(RunState::Succeeded, None)
                } else {
                    run.finish(RunState::Interrupted, Some("interrupted"))
                }
                self.save_run(&run)?;
            }
        }
        Ok(())
    }
    pub(crate) fn reconcile(
        &self,
        settings: &super::runtime::TaskmasterSettings,
    ) -> Result<(), String> {
        let mut history = self.history()?;
        let mut changed = false;
        for snapshot in &mut history.snapshots {
            if snapshot.data.as_ref().is_some_and(|d| {
                super::assessment_access::access_narrowed(
                    d.identity.context_level,
                    &d.identity.excluded_paths,
                    settings,
                )
            }) {
                snapshot.data = None;
                snapshot.redacted = true;
                changed = true;
            }
        }
        if changed {
            self.write("readiness-history.json", &history, MAX_HISTORY)?;
        }
        if let Some(mut run) = self.run()? {
            if run.input_identity.as_ref().is_some_and(|d| {
                super::assessment_access::access_narrowed(
                    d.context_level,
                    &d.excluded_paths,
                    settings,
                )
            }) {
                run.finish(RunState::Interrupted, Some("access_changed"));
                self.save_run(&run)?;
            }
        }
        Ok(())
    }
}
fn validate_run(run: &Run) -> Result<(), String> {
    if run.format_version != 1
        || uuid::Uuid::parse_str(&run.id).is_err()
        || chrono::DateTime::parse_from_rfc3339(&run.started_at).is_err()
    {
        return Err("Invalid readiness attempt".into());
    }
    let active = matches!(run.state, RunState::Queued | RunState::Running);
    if active != (run.phase.is_some()) || active == run.completed_at.is_some() {
        return Err("Invalid readiness attempt state".into());
    }
    Ok(())
}
fn validate_snapshot(s: &Snapshot) -> Result<(), String> {
    if uuid::Uuid::parse_str(&s.id).is_err()
        || chrono::DateTime::parse_from_rfc3339(&s.started_at).is_err()
        || chrono::DateTime::parse_from_rfc3339(&s.completed_at).is_err()
        || s.redacted != s.data.is_none()
        || serde_json::to_vec(s)
            .map_err(|_| "Invalid readiness snapshot")?
            .len()
            > MAX_SNAPSHOT
    {
        return Err("Invalid readiness snapshot".into());
    }
    if let Some(data) = &s.data {
        if data.concepts.len() != 14
            || data.repository_evidence.len() > 32
            || data.input_manifest_invalid()
        {
            return Err("Invalid readiness snapshot data".into());
        }
        let coverage = data
            .concepts
            .iter()
            .filter(|r| r.maturity.is_some())
            .count();
        let raw = (coverage == 14).then(|| {
            data.concepts
                .iter()
                .filter_map(|r| r.maturity)
                .map(u16::from)
                .sum::<u16>()
        });
        if data.coverage != coverage
            || data.raw_total != raw
            || data.overall_score != raw.map(|total| ((100 * total + 28) / 56) as u8)
        {
            return Err("Invalid readiness totals".into());
        }
        let facts = data
            .repository_evidence
            .iter()
            .map(|fact| {
                let expected = fact_id(&fact.fact)?;
                if fact.fact_id != expected
                    || !data
                        .identity
                        .input_manifest
                        .collector
                        .files
                        .iter()
                        .any(|file| {
                            file.path == fact.fact.path
                                && file.sha256 == fact.fact.sha256
                                && file.excerpt_digest
                                    == hex::encode(Sha256::digest(fact.fact.excerpt.as_bytes()))
                        })
                {
                    return Err("Invalid saved readiness evidence identity".into());
                }
                Ok(expected)
            })
            .collect::<Result<HashSet<_>, String>>()?;
        for (row, id) in data
            .concepts
            .iter()
            .zip(super::readiness_rubric::CONCEPT_IDS)
        {
            if row.concept_id != id
                || row.repository_fact_hashes.len() > 8
                || row
                    .repository_fact_hashes
                    .iter()
                    .collect::<HashSet<_>>()
                    .len()
                    != row.repository_fact_hashes.len()
                || row
                    .repository_fact_hashes
                    .iter()
                    .any(|id| !facts.contains(id))
                || !data
                    .knowledge_evidence
                    .iter()
                    .any(|page| page.page_id == row.rubric_page_id)
            {
                return Err("Invalid saved readiness rating".into());
            }
            match (&row.maturity, &row.confidence, &row.unassessed_reason) {
                (Some(0..=4), Some(_), None) if !row.repository_fact_hashes.is_empty() => {}
                (None, None, Some(reason)) => {
                    super::assessment_model::validate_prose(reason, 500, true)?
                }
                _ => return Err("Invalid saved readiness score state".into()),
            }
            super::assessment_model::validate_prose(&row.rationale, 500, true)?;
            super::assessment_model::validate_prospective_prose(&row.next_transition, 500, false)?;
            if row.limits.len() > 4 {
                return Err("Invalid saved readiness limits".into());
            }
            for limit in &row.limits {
                super::assessment_model::validate_prose(limit, 200, true)?;
            }
        }
    }
    Ok(())
}
impl SnapshotData {
    fn input_manifest_invalid(&self) -> bool {
        self.identity.input_manifest.version != 1
            || self.identity.input_manifest.purpose != "readiness"
            || self.identity.input_manifest.collector.validate().is_err()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_history_is_bounded_and_corruption_never_becomes_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = ReadinessStore::new(dir.path());
        assert!(store.history().unwrap().snapshots.is_empty());
        for n in 0..101 {
            store
                .append(Snapshot {
                    id: uuid::Uuid::new_v4().to_string(),
                    started_at: format!("2026-10-25T00:00:{:02}Z", n % 60),
                    completed_at: "2026-10-25T00:00:00Z".into(),
                    redacted: true,
                    data: None,
                })
                .unwrap();
        }
        assert_eq!(store.history().unwrap().snapshots.len(), 100);
        let snapshot = store.history().unwrap().snapshots[0].clone();
        store.append(snapshot).unwrap();
        assert_eq!(store.history().unwrap().snapshots.len(), 100);
        std::fs::write(dir.path().join("readiness-history.json"), "corrupt").unwrap();
        assert!(store.history().is_err());
        assert!(store
            .append(Snapshot {
                id: uuid::Uuid::new_v4().to_string(),
                started_at: "now".into(),
                completed_at: "now".into(),
                redacted: true,
                data: None
            })
            .is_err());
    }
    #[test]
    fn interrupted_attempts_are_not_replayed_and_committed_ids_reconcile() {
        let dir = tempfile::tempdir().unwrap();
        let store = ReadinessStore::new(dir.path());
        let mut run = Run::queued();
        let id = run.id.clone();
        store.save_run(&run).unwrap();
        store.recover().unwrap();
        assert_eq!(store.run().unwrap().unwrap().state, RunState::Interrupted);
        run.state = RunState::Running;
        store.save_run(&run).unwrap();
        store
            .append(Snapshot {
                id,
                started_at: run.started_at.clone(),
                completed_at: now(),
                redacted: true,
                data: None,
            })
            .unwrap();
        store.recover().unwrap();
        assert_eq!(store.run().unwrap().unwrap().state, RunState::Succeeded);
    }
}
