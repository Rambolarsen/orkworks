use super::knowledge::{self, KnowledgeIdentity, VerifiedKnowledge};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

mod access;
mod assessment;
mod inference;
#[cfg(test)]
mod knowledge_tests;

// Runtime instances are intentionally cheap views over durable state. This
// process-wide lock serializes the read-modify-write reservation path even
// when independent HTTP handlers construct separate views.
pub(super) static PERSISTENCE_LOCK: Mutex<()> = Mutex::new(());

/// Proof of the existing process/file locking order. Never construct outside `acquire`.
pub(super) struct PersistenceGuard<'a> {
    root: &'a Path,
    // Release the file lease before permitting another in-process acquisition.
    _file: std::fs::File,
    _process: std::sync::MutexGuard<'static, ()>,
}

impl<'a> PersistenceGuard<'a> {
    pub(super) fn acquire(root: &'a Path) -> Result<Self, String> {
        let process = PERSISTENCE_LOCK
            .lock()
            .map_err(|_| "taskmaster persistence lock unavailable")?;
        let file = persistence_file_lock(root)?;
        Ok(Self {
            root,
            _file: file,
            _process: process,
        })
    }
    pub(super) fn protects(&self, root: &Path) -> bool {
        self.root == root
    }
}

#[cfg(test)]
const MAX_PAGES: usize = 256;
#[cfg(test)]
const MAX_PAGE_BYTES: usize = 64 * 1024;
#[cfg(test)]
const MAX_BUNDLE_BYTES: usize = 2 * 1024 * 1024;
const MAX_EXCLUDED_PATHS: usize = 64;
#[cfg(test)]
const MAX_PATH_BYTES: usize = 512;
const MAX_DAILY_EVALUATIONS: u32 = 64;
const MAX_INTERVAL_MINUTES: u32 = 24 * 60;

const BRAIN_KNOWLEDGE_UNAVAILABLE: &str = "Brain analysis is unavailable until verified reference knowledge meets the privacy requirements. Observation recommendations remain available.";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContextLevel {
    SessionObservations,
    WorkflowContext,
    SourceCode,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskmasterSelection {
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ollama_base_url: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "SelectionOverride::is_unchanged")]
    pub selection: SelectionOverride,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_level: Option<ContextLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excluded_paths: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_interval_minutes: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SelectionOverride {
    Unchanged,
    Clear,
    Set(TaskmasterSelection),
}

impl Default for SelectionOverride {
    fn default() -> Self {
        Self::Unchanged
    }
}
impl SelectionOverride {
    fn is_unchanged(value: &Self) -> bool {
        matches!(value, Self::Unchanged)
    }
}
impl Serialize for SelectionOverride {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unchanged | Self::Clear => serializer.serialize_none(),
            Self::Set(value) => value.serialize(serializer),
        }
    }
}
impl<'de> Deserialize<'de> for SelectionOverride {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match Option::<TaskmasterSelection>::deserialize(deserializer)? {
                Some(value) => Self::Set(value),
                None => Self::Clear,
            },
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskmasterSettings {
    pub enabled: bool,
    pub selection: Option<TaskmasterSelection>,
    pub context_level: ContextLevel,
    pub excluded_paths: Vec<String>,
    pub daily_evaluation_limit: u32,
    pub min_interval_minutes: u32,
    pub automatic_knowledge_updates: bool,
    pub workspace_overrides: BTreeMap<String, WorkspaceOverride>,
}

impl Default for TaskmasterSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            selection: None,
            context_level: ContextLevel::WorkflowContext,
            excluded_paths: Vec::new(),
            daily_evaluation_limit: 8,
            min_interval_minutes: 60,
            automatic_knowledge_updates: true,
            workspace_overrides: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgePage {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub page_type: String,
    pub status: String,
    pub content: String,
    pub sha256: String,
    pub related_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeBundle {
    pub format_version: u8,
    pub version: String,
    pub sequence: u64,
    pub published_at: String,
    pub pages: Vec<KnowledgePage>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskmasterStatus {
    pub workspace_path: Option<String>,
    pub settings: TaskmasterSettings,
    pub effective_settings: TaskmasterSettings,
    pub remaining_evaluations: u32,
    pub analysis_status: String,
    pub knowledge_version: Option<String>,
    pub last_evaluated_at: Option<String>,
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskmasterRunStatus {
    pub workspace_path: Option<String>,
    pub active_attempt: Option<TaskmasterRunAttempt>,
    pub latest_outcome: Option<TaskmasterRunOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_manual_request: Option<TaskmasterManualRequest>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskmasterRunAttemptState {
    Queued,
    Running,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskmasterRunOutcomeState {
    Succeeded,
    Failed,
    Interrupted,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskmasterRunTrigger {
    Manual,
    Background,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskmasterRunAttempt {
    pub id: u64,
    pub state: TaskmasterRunAttemptState,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub trigger: TaskmasterRunTrigger,
    pub provider: String,
    pub model: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskmasterRunOutcome {
    pub state: TaskmasterRunOutcomeState,
    pub started_at: String,
    pub completed_at: String,
    pub trigger: TaskmasterRunTrigger,
    pub provider: String,
    pub model: String,
    pub error_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_counts: Option<TaskmasterProposalCounts>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskmasterProposalCounts {
    pub created: u32,
    pub suppressed: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskmasterManualRequestDisposition {
    Cached,
    Skipped,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskmasterManualRequestReason {
    UnchangedInputs,
    ContextChanged,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskmasterManualRequest {
    pub id: String,
    pub requested_at: String,
    pub completed_at: String,
    pub disposition: TaskmasterManualRequestDisposition,
    pub reason: TaskmasterManualRequestReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvaluationReservation {
    Reserved,
    Cached,
    ContextChanged,
    Skipped,
}

impl EvaluationReservation {
    pub(crate) fn is_reserved(self) -> bool {
        self == Self::Reserved
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct WorkspaceRunRecord {
    active_attempt: Option<TaskmasterRunAttempt>,
    latest_outcome: Option<TaskmasterRunOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    latest_manual_request: Option<TaskmasterManualRequest>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvaluationLedger {
    day: String,
    reservations: u32,
    #[serde(default)]
    workspace_last_evaluated: BTreeMap<String, String>,
    // Older versions cached reservations, not accepted results. Ignore those keys.
    #[serde(default, rename = "acceptedWorkspaceCacheKeys")]
    workspace_cache_keys: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "omit_legacy_error")]
    last_error: Option<String>,
    #[serde(default)]
    generation: u64,
    #[serde(default)]
    workspace_runs: BTreeMap<String, WorkspaceRunRecord>,
    #[serde(default)]
    next_run_id: u64,
}

struct RuntimeData {
    settings: TaskmasterSettings,
    knowledge: Option<KnowledgeBundle>,
    verified_knowledge: Option<VerifiedKnowledge>,
    trusted_key_sha256: Option<String>,
    #[cfg(test)]
    test_key: Option<String>,
    ledger: EvaluationLedger,
    ledger_readable: bool,
}

#[derive(Clone)]
pub(crate) struct EvaluationSnapshot {
    pub readiness_rubric: Option<super::readiness_rubric::ReadinessRubric>,
    pub settings: TaskmasterSettings,
    pub knowledge: Option<KnowledgeBundle>,
    pub knowledge_identity: Option<KnowledgeIdentity>,
    pub generation: u64,
    pub custom_inference: Option<inference::CapturedInference>,
    pub native_revision: Option<super::provider_catalog::NativeRevision>,
}

/// Server-owned identity for one bounded semantic rollup evaluation.
///
/// The token is carried only in memory between request construction and result
/// application. It is never accepted from or persisted as model authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RollupEvaluationToken {
    pub workspace_instance: u64,
    pub generation: u64,
    pub provider: String,
    pub model: String,
    pub prompt_version: String,
    pub family_snapshot_hash: String,
}

impl EvaluationSnapshot {
    pub(crate) fn cache_key(&self, prompt: &str) -> Result<String, String> {
        let custom = self
            .custom_inference
            .as_ref()
            .map(inference::CapturedInference::cache_identity)
            .transpose()?;
        let bytes = serde_json::to_vec(&(
            1_u8,
            &self.settings,
            &self.knowledge_identity,
            self.generation,
            custom,
            &self.native_revision,
            prompt,
        ))
        .map_err(|_| "cannot encode Taskmaster evaluation cache key")?;
        Ok(hex::encode(Sha256::digest(bytes)))
    }
}

pub(crate) struct TaskmasterRuntime {
    root: PathBuf,
    data: Mutex<RuntimeData>,
}

impl TaskmasterRuntime {
    pub(crate) fn open(root: PathBuf) -> Self {
        Self::open_inner(
            root,
            #[cfg(test)]
            None,
        )
    }

    #[cfg(test)]
    pub(crate) fn open_with_test_key(root: PathBuf, key: &str) -> Self {
        Self::open_inner(root, Some(key.to_owned()))
    }

    #[cfg(test)]
    pub(crate) fn open_fixture(root: PathBuf) -> Self {
        Self::open_with_test_key(root, knowledge::test_support::TEST_PUBLIC_KEY)
    }

    fn open_inner(root: PathBuf, #[cfg(test)] test_key: Option<String>) -> Self {
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let file_lock = persistence_file_lock(&root);
        let settings = read_json(root.join("settings.json")).unwrap_or_default();
        let trusted_key_sha256 = knowledge::trusted_key_sha256().ok();
        #[cfg(test)]
        let trusted_key_sha256 = test_key.as_deref().map_or(trusted_key_sha256, |key| {
            knowledge::test_key_sha256(key).ok()
        });
        let ledger = read_json(root.join("evaluations.json"));
        let ledger_readable =
            file_lock.is_ok() && (ledger.is_some() || !root.join("evaluations.json").exists());
        let mut data = RuntimeData {
            settings,
            knowledge: None,
            verified_knowledge: None,
            trusted_key_sha256,
            #[cfg(test)]
            test_key,
            ledger: ledger.unwrap_or_default(),
            ledger_readable,
        };
        reload_knowledge(&root, &mut data);
        Self {
            root,
            data: Mutex::new(data),
        }
    }

    /// Re-read the bounded durable proof; memory alone never authorizes analysis.
    pub(crate) fn brain_knowledge_availability(&self) -> Result<(), &'static str> {
        let reason = BRAIN_KNOWLEDGE_UNAVAILABLE;
        let _guard = PersistenceGuard::acquire(&self.root).map_err(|_| reason)?;
        let mut data = self.data.lock().map_err(|_| reason)?;
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable || data.verified_knowledge.is_none() {
            return Err(reason);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn knowledge_path(&self) -> PathBuf {
        proof_path(&self.root, &self.data.lock().unwrap()).unwrap()
    }

    #[cfg(test)]
    pub(crate) fn activate_fixture_bundle(&self, bundle: KnowledgeBundle) -> Result<(), String> {
        let mut value = serde_json::to_value(&bundle).unwrap();
        value["publishedAt"] =
            serde_json::json!(chrono::DateTime::parse_from_rfc3339(&bundle.published_at)
                .unwrap()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        value["privacyPolicyVersion"] = serde_json::json!(1);
        value["capabilities"] = serde_json::json!([]);
        if bundle.pages.is_empty() {
            value["pages"] = serde_json::json!([{"id":"fixture.md","title":"Fixture","type":"concept","status":"active","content":"","sha256":hex::encode(Sha256::digest(b"")),"relatedIds":[]}]);
        }
        self.activate_knowledge(&knowledge::test_support::activation(value, false))
    }

    pub(crate) fn root_path(&self) -> &Path {
        &self.root
    }

    pub(crate) fn status(&self, workspace: Option<&Path>) -> TaskmasterStatus {
        let data = self.data.lock().expect("taskmaster runtime lock poisoned");
        let workspace = workspace.and_then(canonical_workspace_key);
        let effective = workspace.as_deref().map_or_else(
            || data.settings.clone(),
            |key| effective_settings(&data.settings, key),
        );
        let today = utc_day();
        let used = (data.ledger.day == today)
            .then_some(data.ledger.reservations)
            .unwrap_or(0);
        let last_evaluated_at = workspace
            .as_deref()
            .and_then(|key| data.ledger.workspace_last_evaluated.get(key).cloned());
        TaskmasterStatus {
            workspace_path: workspace,
            remaining_evaluations: data
                .ledger_readable
                .then_some(effective.daily_evaluation_limit.saturating_sub(used))
                .unwrap_or(0),
            analysis_status: if data.ledger_readable {
                analysis_status(&effective)
            } else {
                "ledger_unavailable".into()
            },
            knowledge_version: data.knowledge.as_ref().map(|bundle| bundle.version.clone()),
            last_error: data.ledger.last_error.clone(),
            settings: data.settings.clone(),
            effective_settings: effective,
            last_evaluated_at,
        }
    }

    pub(crate) fn run_status(
        &self,
        workspace: Option<&Path>,
    ) -> Result<TaskmasterRunStatus, String> {
        let workspace = workspace.ok_or_else(|| "No workspace is selected".to_string())?;
        let key = canonical_workspace_key(workspace)
            .ok_or_else(|| "Selected workspace is unavailable".to_string())?;
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let _file_lock = persistence_file_lock(&self.root)?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable {
            return Err("Taskmaster evaluation ledger is unreadable".into());
        }
        let record = data.ledger.workspace_runs.get(&key);
        Ok(TaskmasterRunStatus {
            workspace_path: Some(key),
            active_attempt: record.and_then(|record| record.active_attempt.clone()),
            latest_outcome: record.and_then(|record| record.latest_outcome.clone()),
            latest_manual_request: record.and_then(|record| record.latest_manual_request.clone()),
        })
    }

    pub(crate) fn queue_run(
        &self,
        workspace: &Path,
        trigger: TaskmasterRunTrigger,
        provider: &str,
        model: &str,
    ) -> Result<u64, String> {
        let key = canonical_workspace_key(workspace)
            .ok_or_else(|| "Selected workspace is unavailable".to_string())?;
        self.mutate_ledger(|ledger| {
            if ledger
                .workspace_runs
                .get(&key)
                .is_some_and(|record| record.active_attempt.is_some())
            {
                return Err("Taskmaster analysis already has an active attempt".into());
            }
            ledger.next_run_id = ledger.next_run_id.saturating_add(1);
            let id = ledger.next_run_id;
            let record = ledger.workspace_runs.entry(key).or_default();
            if trigger == TaskmasterRunTrigger::Manual {
                record.latest_manual_request = None;
            }
            record.active_attempt = Some(TaskmasterRunAttempt {
                id,
                state: TaskmasterRunAttemptState::Queued,
                queued_at: chrono::Utc::now().to_rfc3339(),
                started_at: None,
                trigger,
                provider: provider.to_string(),
                model: model.to_string(),
            });
            Ok(id)
        })
    }

    pub(crate) fn mark_run_running(
        &self,
        workspace: &Path,
        run_id: u64,
        provider: &str,
        model: &str,
    ) -> Result<bool, String> {
        let key = canonical_workspace_key(workspace)
            .ok_or_else(|| "Selected workspace is unavailable".to_string())?;
        self.mutate_ledger(|ledger| {
            let Some(attempt) = ledger
                .workspace_runs
                .get_mut(&key)
                .and_then(|record| record.active_attempt.as_mut())
                .filter(|attempt| attempt.id == run_id)
            else {
                return Ok(false);
            };
            attempt.state = TaskmasterRunAttemptState::Running;
            attempt.started_at = Some(chrono::Utc::now().to_rfc3339());
            attempt.provider = provider.to_string();
            attempt.model = model.to_string();
            Ok(true)
        })
    }

    pub(crate) fn finish_run(
        &self,
        workspace: &Path,
        run_id: u64,
        state: TaskmasterRunOutcomeState,
        error_summary: Option<&str>,
    ) -> Result<bool, String> {
        self.finish_run_with_counts(workspace, run_id, state, error_summary, None)
    }

    pub(crate) fn finish_run_with_counts(
        &self,
        workspace: &Path,
        run_id: u64,
        state: TaskmasterRunOutcomeState,
        error_summary: Option<&str>,
        proposal_counts: Option<TaskmasterProposalCounts>,
    ) -> Result<bool, String> {
        let key = canonical_workspace_key(workspace)
            .ok_or_else(|| "Selected workspace is unavailable".to_string())?;
        self.mutate_ledger(|ledger| {
            let Some(record) = ledger.workspace_runs.get_mut(&key) else {
                return Ok(false);
            };
            if record.active_attempt.as_ref().map(|attempt| attempt.id) != Some(run_id) {
                return Ok(false);
            }
            let attempt = record
                .active_attempt
                .take()
                .expect("matching active attempt");
            let completed_at = chrono::Utc::now().to_rfc3339();
            let proposal_counts = if state == TaskmasterRunOutcomeState::Succeeded {
                proposal_counts
            } else {
                None
            };
            record.latest_outcome = Some(TaskmasterRunOutcome {
                state,
                started_at: attempt
                    .started_at
                    .unwrap_or_else(|| attempt.queued_at.clone()),
                completed_at,
                trigger: attempt.trigger,
                provider: attempt.provider,
                model: attempt.model,
                error_summary: error_summary.map(bounded_run_error_summary),
                proposal_counts,
            });
            Ok(true)
        })
    }

    #[cfg(test)]
    pub(crate) fn record_manual_request_disposition(
        &self,
        workspace: &Path,
        run_id: u64,
        disposition: TaskmasterManualRequestDisposition,
        reason: TaskmasterManualRequestReason,
    ) -> Result<bool, String> {
        let key = canonical_workspace_key(workspace)
            .ok_or_else(|| "Selected workspace is unavailable".to_string())?;
        self.mutate_ledger(|ledger| {
            let Some(record) = ledger.workspace_runs.get_mut(&key) else {
                return Ok(false);
            };
            let Some((requested_at, request_id)) = record
                .active_attempt
                .as_ref()
                .filter(|attempt| {
                    attempt.id == run_id && attempt.trigger == TaskmasterRunTrigger::Manual
                })
                .map(|attempt| (attempt.queued_at.clone(), attempt.id.to_string()))
            else {
                return Ok(false);
            };
            record.latest_manual_request = Some(TaskmasterManualRequest {
                id: request_id,
                requested_at,
                completed_at: chrono::Utc::now().to_rfc3339(),
                disposition,
                reason,
            });
            Ok(true)
        })
    }

    pub(crate) fn finish_manual_request_without_outcome(
        &self,
        workspace: &Path,
        run_id: u64,
        disposition: TaskmasterManualRequestDisposition,
        reason: TaskmasterManualRequestReason,
    ) -> Result<bool, String> {
        let key = canonical_workspace_key(workspace)
            .ok_or_else(|| "Selected workspace is unavailable".to_string())?;
        self.mutate_ledger(|ledger| {
            let Some(record) = ledger.workspace_runs.get_mut(&key) else {
                return Ok(false);
            };
            let Some((requested_at, request_id)) = record
                .active_attempt
                .as_ref()
                .filter(|attempt| {
                    attempt.id == run_id && attempt.trigger == TaskmasterRunTrigger::Manual
                })
                .map(|attempt| (attempt.queued_at.clone(), attempt.id.to_string()))
            else {
                return Ok(false);
            };
            record.latest_manual_request = Some(TaskmasterManualRequest {
                id: request_id,
                requested_at,
                completed_at: chrono::Utc::now().to_rfc3339(),
                disposition,
                reason,
            });
            record.active_attempt = None;
            Ok(true)
        })
    }

    pub(crate) fn clear_run(&self, workspace: &Path, run_id: u64) -> Result<bool, String> {
        let key = canonical_workspace_key(workspace)
            .ok_or_else(|| "Selected workspace is unavailable".to_string())?;
        self.mutate_ledger(|ledger| {
            let Some(record) = ledger.workspace_runs.get_mut(&key) else {
                return Ok(false);
            };
            if record.active_attempt.as_ref().map(|attempt| attempt.id) != Some(run_id) {
                return Ok(false);
            }
            record.active_attempt = None;
            if record.latest_outcome.is_none() && record.latest_manual_request.is_none() {
                ledger.workspace_runs.remove(&key);
            }
            Ok(true)
        })
    }

    /// Recovery is available only to a caller holding the same installation-wide
    /// lease as an evaluator, so another live instance is never marked interrupted.
    pub(crate) fn recover_workspace_run(
        &self,
        workspace: &Path,
        _analysis_lease: &fs::File,
    ) -> Result<bool, String> {
        let key = canonical_workspace_key(workspace)
            .ok_or_else(|| "Selected workspace is unavailable".to_string())?;
        self.mutate_ledger(|ledger| {
            let Some(record) = ledger.workspace_runs.get_mut(&key) else {
                return Ok(false);
            };
            let Some(attempt) = record.active_attempt.take() else {
                return Ok(false);
            };
            if attempt.state == TaskmasterRunAttemptState::Running {
                record.latest_outcome = Some(TaskmasterRunOutcome {
                    state: TaskmasterRunOutcomeState::Interrupted,
                    started_at: attempt
                        .started_at
                        .unwrap_or_else(|| attempt.queued_at.clone()),
                    completed_at: chrono::Utc::now().to_rfc3339(),
                    trigger: attempt.trigger,
                    provider: attempt.provider,
                    model: attempt.model,
                    error_summary: None,
                    proposal_counts: None,
                });
            }
            if record.latest_outcome.is_none() && record.latest_manual_request.is_none() {
                ledger.workspace_runs.remove(&key);
            }
            Ok(true)
        })
    }

    fn mutate_ledger<T>(
        &self,
        mutate: impl FnOnce(&mut EvaluationLedger) -> Result<T, String>,
    ) -> Result<T, String> {
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let _file_lock = persistence_file_lock(&self.root)?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable {
            return Err("Taskmaster evaluation ledger is unreadable".into());
        }
        let result = mutate(&mut data.ledger)?;
        write_json(&self.root.join("evaluations.json"), &data.ledger)?;
        Ok(result)
    }

    #[cfg(test)]
    pub(crate) fn replace_settings(&self, settings: TaskmasterSettings) -> Result<(), String> {
        self.replace_settings_with_reconcile(settings, |_, _| Ok(()))
    }

    pub(crate) fn replace_settings_with_reconcile(
        &self,
        settings: TaskmasterSettings,
        reconcile: impl FnOnce(&TaskmasterSettings, &TaskmasterSettings) -> Result<(), String>,
    ) -> Result<(), String> {
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let _file_lock = persistence_file_lock(&self.root)?;
        validate_settings(&settings)?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable {
            return Err("Taskmaster evaluation ledger is unreadable".into());
        }
        reconcile(&data.settings, &settings)?;
        data.ledger.generation = data.ledger.generation.saturating_add(1);
        data.ledger.workspace_cache_keys.clear();
        write_json(&self.root.join("evaluations.json"), &data.ledger)?;
        write_json(&self.root.join("settings.json"), &settings)?;
        data.settings = settings;
        Ok(())
    }

    pub(crate) fn activate_knowledge(&self, bytes: &[u8]) -> Result<(), String> {
        let _guard = PersistenceGuard::acquire(&self.root)?;
        let mut data = self
            .data
            .lock()
            .map_err(|_| "taskmaster runtime lock unavailable")?;
        // Verify raw bytes, preserving duplicate-field and lexical validation.
        let verified = verify_knowledge(&data, bytes)?;
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable {
            return Err("Taskmaster evaluation ledger is unreadable".into());
        }
        if let Some(current) = &data.verified_knowledge {
            if verified.identity() == current.identity() {
                return Ok(());
            }
            if verified.identity().sequence < current.identity().sequence {
                // Stable recoverable error consumed by Electron synchronization before its feed check.
                return Err("knowledge bundle sequence is older than the active bundle".into());
            }
            if verified.identity().sequence == current.identity().sequence {
                return Err("knowledge bundle sequence equivocation at the active sequence".into());
            }
        }
        data.ledger.generation = data
            .ledger
            .generation
            .checked_add(1)
            .ok_or("Taskmaster generation exhausted")?;
        data.ledger.workspace_cache_keys.clear();
        // Invalidate old work first. A crash here may discard work, never accept stale work.
        write_json(&self.root.join("evaluations.json"), &data.ledger)?;
        let path = proof_path(&self.root, &data).ok_or("trusted knowledge key unavailable")?;
        write_proof(&path, verified.activation_bytes())?;
        reload_knowledge(&self.root, &mut data);
        if data
            .verified_knowledge
            .as_ref()
            .map(VerifiedKnowledge::identity)
            != Some(verified.identity())
        {
            return Err("knowledge activation could not be confirmed".into());
        }
        Ok(())
    }

    /// Reservations are durable before a provider call. A failed background
    /// call consumes its reservation, so process restarts cannot reset usage
    /// accounting; manual reservations bypass the daily limit and never
    /// increment the shared counter.
    #[cfg(test)]
    pub(crate) fn reserve_current(
        &self,
        workspace: &Path,
        now: &str,
        cache_key: Option<&str>,
        generation: Option<u64>,
    ) -> Result<bool, String> {
        self.reserve_current_with_policy(workspace, now, cache_key, generation, false)
    }

    #[cfg(test)]
    pub(crate) fn reserve_current_manual(
        &self,
        workspace: &Path,
        now: &str,
        cache_key: Option<&str>,
        generation: Option<u64>,
    ) -> Result<bool, String> {
        self.reserve_current_with_policy(workspace, now, cache_key, generation, true)
    }

    #[cfg(test)]
    fn reserve_current_with_policy(
        &self,
        workspace: &Path,
        now: &str,
        cache_key: Option<&str>,
        generation: Option<u64>,
        manual: bool,
    ) -> Result<bool, String> {
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let _file_lock = persistence_file_lock(&self.root)?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        self.reserve_loaded(
            &mut data, workspace, now, cache_key, generation, manual, manual, manual,
        )
        .map(EvaluationReservation::is_reserved)
    }

    /// Caller owns the persistence lease and runtime-data mutex through the write.
    fn reserve_loaded(
        &self,
        data: &mut RuntimeData,
        workspace: &Path,
        now: &str,
        cache_key: Option<&str>,
        generation: Option<u64>,
        bypass_min_interval: bool,
        allow_disabled: bool,
        manual: bool,
    ) -> Result<EvaluationReservation, String> {
        let Some(workspace) = canonical_workspace_key(workspace) else {
            return Ok(EvaluationReservation::Skipped);
        };
        if !data.ledger_readable {
            return Err("Taskmaster evaluation ledger is unreadable".into());
        }
        if generation.is_some_and(|expected| expected != data.ledger.generation) {
            return Ok(EvaluationReservation::ContextChanged);
        }
        let effective = effective_settings(&data.settings, &workspace);
        if (!allow_disabled && !effective.enabled) || effective.selection.is_none() {
            return Ok(EvaluationReservation::Skipped);
        }
        let day = now
            .get(..10)
            .filter(|day| day.len() == 10)
            .map(str::to_string)
            .unwrap_or_else(utc_day);
        if data.ledger.day != day {
            data.ledger.day = day;
            data.ledger.reservations = 0;
        }
        if !manual && data.ledger.reservations >= effective.daily_evaluation_limit {
            return Ok(EvaluationReservation::Skipped);
        }
        if cache_key.is_some_and(|cache_key| {
            data.ledger
                .workspace_cache_keys
                .get(&workspace)
                .is_some_and(|existing| existing == cache_key)
        }) {
            return Ok(if manual {
                EvaluationReservation::Cached
            } else {
                EvaluationReservation::Skipped
            });
        }
        if !bypass_min_interval {
            if let Some(last) = data.ledger.workspace_last_evaluated.get(&workspace) {
                let last = chrono::DateTime::parse_from_rfc3339(last).ok();
                let current = chrono::DateTime::parse_from_rfc3339(now).ok();
                if last.zip(current).is_some_and(|(last, current)| {
                    current.signed_duration_since(last).num_minutes()
                        < i64::from(effective.min_interval_minutes)
                }) {
                    return Ok(EvaluationReservation::Skipped);
                }
            }
        }
        if !manual {
            data.ledger.reservations = data.ledger.reservations.saturating_add(1);
        }
        data.ledger
            .workspace_last_evaluated
            .insert(workspace.clone(), now.to_string());
        write_json(&self.root.join("evaluations.json"), &data.ledger)?;
        Ok(EvaluationReservation::Reserved)
    }

    pub(crate) fn evaluation_snapshot(&self, workspace: &Path) -> Option<EvaluationSnapshot> {
        self.evaluation_snapshot_with_disabled(workspace, false)
    }

    pub(crate) fn manual_evaluation_snapshot(
        &self,
        workspace: &Path,
    ) -> Option<EvaluationSnapshot> {
        self.evaluation_snapshot_with_disabled(workspace, true)
    }

    fn evaluation_snapshot_with_disabled(
        &self,
        workspace: &Path,
        allow_disabled: bool,
    ) -> Option<EvaluationSnapshot> {
        let workspace = canonical_workspace_key(workspace)?;
        let _guard = PersistenceGuard::acquire(&self.root).ok()?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable {
            return None;
        }
        let settings = effective_settings(&data.settings, &workspace);
        ((allow_disabled || settings.enabled) && settings.selection.is_some()).then(|| {
            EvaluationSnapshot {
                readiness_rubric: None,
                settings,
                knowledge: data.knowledge.clone(),
                knowledge_identity: data
                    .verified_knowledge
                    .as_ref()
                    .map(|proof| proof.identity().clone()),
                generation: data.ledger.generation,
                custom_inference: None,
                native_revision: None,
            }
        })
    }

    #[cfg(test)]
    pub(crate) fn record_error(
        &self,
        generation: u64,
        _error: Option<String>,
    ) -> Result<bool, String> {
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let _file_lock = persistence_file_lock(&self.root)?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        // `lastError` is a legacy deserialization-only field. Execution errors
        // are stored in workspace-scoped run outcomes instead.
        if !data.ledger_readable || data.ledger.generation != generation {
            return Ok(false);
        }
        Ok(true)
    }

    /// One application-wide inference lease, held until provider and commit finish.
    pub(crate) fn try_analysis_lease(&self) -> Result<Option<fs::File>, String> {
        fs::create_dir_all(&self.root).map_err(|error| error.to_string())?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join(".analysis.lock"))
            .map_err(|error| error.to_string())?;
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(Some(file)),
            Err(error) if is_analysis_lease_contention(&error) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Holds the configuration lock through applying output, closing the check/write race.
    #[cfg(test)]
    pub(crate) fn with_current_generation(
        &self,
        generation: u64,
        apply: impl FnOnce(),
    ) -> Result<bool, String> {
        let _guard = PersistenceGuard::acquire(&self.root)?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable || data.ledger.generation != generation {
            return Ok(false);
        }
        apply();
        Ok(true)
    }

    /// Revalidates the selected provider/model and generation before applying
    /// a semantic rollup. The caller performs workspace and evidence checks in
    /// the application layer while this guard owns durable Taskmaster config.
    pub(crate) fn with_current_rollup_evaluation(
        &self,
        harnesses: &crate::harness::store::HarnessStore,
        workspace: &Path,
        snapshot: &EvaluationSnapshot,
        token: &RollupEvaluationToken,
        apply: impl FnOnce(),
    ) -> Result<bool, String> {
        let Some(selection) = snapshot.settings.selection.as_ref() else {
            return Ok(false);
        };
        if snapshot.generation != token.generation
            || selection.provider != token.provider
            || selection.model != token.model
        {
            return Ok(false);
        }
        self.with_current_evaluation(harnesses, workspace, snapshot, apply)
    }
}

fn is_analysis_lease_contention(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }
    #[cfg(windows)]
    {
        return error.raw_os_error() == Some(33);
    }
    #[cfg(not(windows))]
    false
}

pub(crate) fn taskmaster_global_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".orkworks").join("taskmaster"))
}

#[cfg(test)]
pub(crate) fn validate_bundle(bundle: &KnowledgeBundle) -> Result<(), String> {
    if bundle.format_version != 1
        || bundle.version.trim().is_empty()
        || bundle
            .published_at
            .parse::<chrono::DateTime<chrono::FixedOffset>>()
            .is_err()
    {
        return Err("knowledge bundle format is invalid".into());
    }
    if bundle.pages.len() > MAX_PAGES {
        return Err("knowledge bundle has too many pages".into());
    }
    let mut total = 0usize;
    let mut ids = std::collections::HashSet::new();
    for page in &bundle.pages {
        if !valid_relative_markdown_path(&page.id)
            || page.title.trim().is_empty()
            || page.content.len() > MAX_PAGE_BYTES
            || page.related_ids.len() > MAX_PAGES
            || !ids.insert(&page.id)
        {
            return Err("knowledge page is out of bounds".into());
        }
        total = total.saturating_add(page.content.len());
        if total > MAX_BUNDLE_BYTES {
            return Err("knowledge bundle is too large".into());
        }
        let digest = hex::encode(Sha256::digest(page.content.as_bytes()));
        if digest != page.sha256 {
            return Err("knowledge page digest does not match content".into());
        }
    }
    if bundle.pages.iter().any(|page| {
        page.related_ids
            .iter()
            .any(|related| !ids.contains(related))
    }) {
        return Err("knowledge page references an excluded page".into());
    }
    Ok(())
}

fn validate_settings(settings: &TaskmasterSettings) -> Result<(), String> {
    if settings.daily_evaluation_limit == 0
        || settings.daily_evaluation_limit > MAX_DAILY_EVALUATIONS
        || settings.min_interval_minutes == 0
        || settings.min_interval_minutes > MAX_INTERVAL_MINUTES
        || settings.excluded_paths.len() > MAX_EXCLUDED_PATHS
    {
        return Err("Taskmaster settings are out of bounds".into());
    }
    for (workspace, override_) in &settings.workspace_overrides {
        if canonical_workspace_key(Path::new(workspace)).as_deref() != Some(workspace.as_str())
            || override_
                .excluded_paths
                .as_ref()
                .is_some_and(|paths| paths.len() > MAX_EXCLUDED_PATHS)
            || override_
                .min_interval_minutes
                .is_some_and(|minutes| minutes == 0 || minutes > MAX_INTERVAL_MINUTES)
        {
            return Err("Taskmaster workspace override is invalid".into());
        }
    }
    if let Some(selection) = &settings.selection {
        validate_selection(selection)?
    }
    for override_ in settings.workspace_overrides.values() {
        if let SelectionOverride::Set(selection) = &override_.selection {
            validate_selection(selection)?;
        }
    }
    Ok(())
}

fn validate_selection(selection: &TaskmasterSelection) -> Result<(), String> {
    if selection.provider.trim().is_empty()
        || !crate::harness::inference::valid_selection_value(&selection.model)
        || selection
            .reasoning_effort
            .as_deref()
            .is_some_and(|effort| !crate::harness::inference::valid_selection_value(effort))
    {
        return Err("Taskmaster selection requires a provider and nonempty model/effort values of at most 256 bytes without control characters".into());
    }
    if selection.provider == "ollama" {
        let url = selection
            .ollama_base_url
            .as_deref()
            .unwrap_or("http://127.0.0.1:11434");
        crate::providers::normalize_ollama_base_url(url)?;
    }
    Ok(())
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    #[test]
    fn settings_status_exposes_the_canonical_workspace_override_key() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().join("runtime"));
        let path = directory.path().join(".");
        let value = serde_json::to_value(runtime.status(Some(&path))).unwrap();
        assert_eq!(
            value["workspacePath"],
            path.canonicalize().unwrap().display().to_string()
        );
        assert!(serde_json::to_value(runtime.status(None)).unwrap()["workspacePath"].is_null());
        let mut settings = TaskmasterSettings::default();
        settings.enabled = true;
        settings.workspace_overrides.insert(
            value["workspacePath"].as_str().unwrap().into(),
            serde_json::from_value(serde_json::json!({"enabled": false})).unwrap(),
        );
        runtime.replace_settings(settings).unwrap();
        assert!(runtime.status(None).effective_settings.enabled);
        assert!(!runtime.status(Some(&path)).effective_settings.enabled);
    }

    #[test]
    fn stored_custom_overrides_keep_opaque_model_ids() {
        for provider in ["codex", "claude-code"] {
            let directory = tempfile::tempdir().unwrap();
            let runtime = TaskmasterRuntime::open(directory.path().into());
            for model in ["model ", "vendor/模型", "model;other"] {
                let mut settings = TaskmasterSettings::default();
                settings.selection = Some(TaskmasterSelection {
                    provider: provider.into(),
                    model: model.into(),
                    reasoning_effort: None,
                    ollama_base_url: None,
                });
                assert!(
                    runtime.replace_settings(settings).is_ok(),
                    "rejected opaque {provider}: {model}"
                );
                assert_eq!(
                    runtime.status(None).settings.selection.unwrap().model,
                    model
                );
            }
        }
    }

    #[test]
    fn selection_rejects_unbounded_or_control_values_without_rewriting_opaque_models() {
        let mut selection = TaskmasterSelection {
            provider: "custom".into(),
            model: " vendor/opaque model ".into(),
            reasoning_effort: None,
            ollama_base_url: None,
        };
        assert!(validate_selection(&selection).is_ok());
        assert_eq!(selection.model, " vendor/opaque model ");
        for model in [
            String::new(),
            "x".repeat(257),
            "é".repeat(129),
            "model\nother".into(),
            "model\u{7f}".into(),
        ] {
            selection.model = model;
            assert!(
                validate_selection(&selection).is_err(),
                "invalid model accepted"
            );
        }
        selection.model = "é".repeat(128);
        assert!(validate_selection(&selection).is_ok());
        for effort in [String::new(), "x".repeat(257), "high\n".into()] {
            selection.reasoning_effort = Some(effort);
            assert!(
                validate_selection(&selection).is_err(),
                "invalid effort accepted"
            );
        }
    }
}

pub(super) fn effective_settings(base: &TaskmasterSettings, workspace: &str) -> TaskmasterSettings {
    let mut effective = base.clone();
    if let Some(override_) = base.workspace_overrides.get(workspace) {
        if let Some(enabled) = override_.enabled {
            effective.enabled = enabled;
        }
        match &override_.selection {
            SelectionOverride::Unchanged => {}
            SelectionOverride::Clear => effective.selection = None,
            SelectionOverride::Set(selection) => effective.selection = Some(selection.clone()),
        }
        if let Some(level) = override_.context_level {
            effective.context_level = level;
        }
        if let Some(paths) = &override_.excluded_paths {
            effective.excluded_paths = paths.clone();
        }
        if let Some(minutes) = override_.min_interval_minutes {
            effective.min_interval_minutes = minutes;
        }
    }
    effective.workspace_overrides.clear();
    effective
}

fn analysis_status(settings: &TaskmasterSettings) -> String {
    if !settings.enabled {
        "disabled".into()
    } else if settings.selection.is_none() {
        "unconfigured".into()
    } else {
        "ready".into()
    }
}

fn canonical_workspace_key(path: &Path) -> Option<String> {
    fs::canonicalize(path)
        .ok()
        .map(|path| path.display().to_string())
}
fn utc_day() -> String {
    chrono::Utc::now().format("%F").to_string()
}
fn omit_legacy_error(_: &Option<String>) -> bool {
    true
}
fn bounded_run_error_summary(error: &str) -> String {
    let normalized = error
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let mut bounded = String::new();
    for character in normalized.trim().chars() {
        if bounded.len() + character.len_utf8() > 512 {
            break;
        }
        bounded.push(character);
    }
    bounded
}
#[cfg(test)]
fn valid_relative_markdown_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_PATH_BYTES
        && path.ends_with(".md")
        && !Path::new(path).is_absolute()
        && !path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
}
fn read_json<T: for<'a> Deserialize<'a>>(path: PathBuf) -> Option<T> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}
fn reload_durable(root: &Path, data: &mut RuntimeData) {
    if let Some(settings) = read_json(root.join("settings.json")) {
        data.settings = settings;
    }
    let ledger = read_json(root.join("evaluations.json"));
    data.ledger_readable = ledger.is_some() || !root.join("evaluations.json").exists();
    if let Some(ledger) = ledger {
        data.ledger = ledger;
    }
    reload_knowledge(root, data);
}
fn proof_path(root: &Path, data: &RuntimeData) -> Option<PathBuf> {
    Some(
        root.join("knowledge")
            .join(data.trusted_key_sha256.as_ref()?)
            .join("activation.json"),
    )
}

fn verify_knowledge(data: &RuntimeData, bytes: &[u8]) -> Result<VerifiedKnowledge, String> {
    #[cfg(test)]
    if let Some(key) = &data.test_key {
        return knowledge::verify_activation_with_key(bytes, key);
    }
    let _ = data;
    knowledge::verify_activation(bytes)
}

fn reload_knowledge(root: &Path, data: &mut RuntimeData) {
    use std::io::Read;
    // Bound allocation as well as parsing, including files enlarged after metadata inspection.
    data.verified_knowledge = proof_path(root, data).and_then(|path| {
        let mut bytes = Vec::new();
        fs::File::open(path)
            .ok()?
            .take((knowledge::MAX_ACTIVATION_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .ok()?;
        verify_knowledge(data, &bytes).ok()
    });
    data.knowledge = data.verified_knowledge.as_ref().and_then(|proof| {
        // Prompt DTO is a projection; the full signed hierarchy/provenance stays in the proof.
        serde_json::from_slice::<KnowledgeBundle>(&serde_json::to_vec(proof.bundle()).ok()?).ok()
    });
}

fn proof_matches(data: &RuntimeData, identity: Option<&KnowledgeIdentity>) -> bool {
    identity.is_some()
        && data
            .verified_knowledge
            .as_ref()
            .map(VerifiedKnowledge::identity)
            == identity
}

fn write_proof(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().ok_or("knowledge path has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    temporary
        .write_all(bytes)
        .map_err(|error| error.to_string())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    let temporary = temporary.into_temp_path();
    crate::harness::integration::atomic_replace(&temporary, path, path.exists())
        .map_err(|error| error.to_string())
}

pub(super) fn persistence_file_lock(root: &Path) -> Result<std::fs::File, String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(root.join(".evaluation.lock"))
        .map_err(|error| error.to_string())?;
    fs2::FileExt::lock_exclusive(&file).map_err(|error| error.to_string())?;
    Ok(file)
}
fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    write_proof(
        path,
        &serde_json::to_vec(value).map_err(|error| error.to_string())?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_knowledge_cannot_be_activated() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(dir.path().to_path_buf());
        let starter =
            include_bytes!("../../../../tests/fixtures/taskmaster-knowledge/unsigned-bundle.json");
        assert!(
            runtime.activate_knowledge(starter).is_err(),
            "unsigned starter must not establish server proof"
        );
        assert!(!dir.path().join("knowledge.json").exists());
    }

    #[test]
    fn settings_default_to_a_safe_unconfigured_runtime() {
        let settings = TaskmasterSettings::default();
        assert!(settings.enabled);
        assert!(settings.selection.is_none());
        assert_eq!(settings.daily_evaluation_limit, 8);
        assert_eq!(settings.min_interval_minutes, 60);
        assert_eq!(settings.context_level, ContextLevel::WorkflowContext);
    }

    #[test]
    fn knowledge_rejects_a_page_with_an_incorrect_digest() {
        let bundle = KnowledgeBundle {
            format_version: 1,
            version: "v1".into(),
            sequence: 1,
            published_at: "2026-09-09T00:00:00Z".into(),
            pages: vec![KnowledgePage {
                id: "guide.md".into(),
                title: "Guide".into(),
                page_type: "concept".into(),
                status: "active".into(),
                content: "bounded content".into(),
                sha256: "0".repeat(64),
                related_ids: vec![],
            }],
        };

        assert!(validate_bundle(&bundle).is_err());
    }

    #[test]
    fn corrupted_ledger_fails_closed_without_resetting_the_daily_budget() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("evaluations.json"), "not json").unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().to_path_buf());

        assert_eq!(
            runtime.status(Some(directory.path())).remaining_evaluations,
            0
        );
        assert!(runtime
            .reserve_current(directory.path(), "2026-09-09T00:00:00Z", None, None)
            .is_err());
    }

    #[test]
    fn null_workspace_selection_clears_the_inherited_selection() {
        let directory = tempfile::tempdir().unwrap();
        let key = directory
            .path()
            .canonicalize()
            .unwrap()
            .display()
            .to_string();
        let settings: TaskmasterSettings = serde_json::from_value(serde_json::json!({
            "enabled": true, "selection": {"provider":"ollama", "model":"llama"},
            "contextLevel":"workflow_context", "excludedPaths": [], "dailyEvaluationLimit": 8,
            "minIntervalMinutes": 60, "automaticKnowledgeUpdates": true,
            "workspaceOverrides": { (key.clone()): { "selection": null } }
        }))
        .unwrap();

        assert!(effective_settings(&settings, &key).selection.is_none());
    }

    fn configured() -> TaskmasterSettings {
        TaskmasterSettings {
            selection: Some(TaskmasterSelection {
                provider: "ollama".into(),
                model: "test".into(),
                reasoning_effort: None,
                ollama_base_url: None,
            }),
            min_interval_minutes: 1,
            ..TaskmasterSettings::default()
        }
    }

    #[test]
    fn corrupt_ledger_cannot_be_repaired_by_settings_or_error_updates() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        fs::write(directory.path().join("evaluations.json"), "corrupt").unwrap();
        assert!(runtime.replace_settings(configured()).is_err());
        assert!(!runtime.record_error(0, None).unwrap());
        assert_eq!(
            fs::read_to_string(directory.path().join("evaluations.json")).unwrap(),
            "corrupt"
        );
    }

    #[test]
    fn taskmaster_run_status_distinguishes_idle_from_missing_or_corrupt_state() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        assert!(runtime
            .run_status(None)
            .unwrap_err()
            .contains("No workspace"));
        let status = runtime.run_status(Some(directory.path())).unwrap();
        assert_eq!(status.active_attempt, None);
        assert_eq!(status.latest_outcome, None);
        let canonical = fs::canonicalize(directory.path()).unwrap();
        assert_eq!(
            status.workspace_path.as_deref(),
            Some(canonical.to_str().unwrap())
        );

        fs::write(directory.path().join("evaluations.json"), "corrupt").unwrap();
        let unreadable = TaskmasterRuntime::open(directory.path().into());
        assert!(unreadable
            .run_status(Some(directory.path()))
            .unwrap_err()
            .contains("unreadable"));
    }

    #[test]
    fn run_status_round_trips_optional_proposal_counts_and_manual_disposition() {
        let value = serde_json::json!({
            "workspacePath": "/workspace",
            "activeAttempt": null,
            "latestOutcome": {
                "state": "succeeded",
                "startedAt": "2026-10-17T10:00:00Z",
                "completedAt": "2026-10-17T10:01:00Z",
                "trigger": "manual",
                "provider": "codex",
                "model": "gpt-6-luna",
                "errorSummary": null,
                "proposalCounts": { "created": 2, "suppressed": 1 }
            },
            "latestManualRequest": {
                "id": "42",
                "requestedAt": "2026-10-17T10:00:00Z",
                "completedAt": "2026-10-17T10:00:01Z",
                "disposition": "cached",
                "reason": "unchanged_inputs"
            }
        });

        let status: TaskmasterRunStatus = serde_json::from_value(value.clone()).unwrap();

        assert_eq!(serde_json::to_value(status).unwrap(), value);
    }

    #[test]
    fn skipped_manual_request_preserves_latest_outcome_and_background_does_not_replace_it() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        let workspace = directory.path();
        let actual = runtime
            .queue_run(workspace, TaskmasterRunTrigger::Manual, "codex", "model")
            .unwrap();
        runtime
            .mark_run_running(workspace, actual, "codex", "model")
            .unwrap();
        runtime
            .finish_run_with_counts(
                workspace,
                actual,
                TaskmasterRunOutcomeState::Succeeded,
                None,
                Some(TaskmasterProposalCounts {
                    created: 1,
                    suppressed: 2,
                }),
            )
            .unwrap();
        let before = runtime
            .run_status(Some(workspace))
            .unwrap()
            .latest_outcome
            .unwrap();

        let manual = runtime
            .queue_run(workspace, TaskmasterRunTrigger::Manual, "codex", "model")
            .unwrap();
        runtime
            .mark_run_running(workspace, manual, "codex", "model")
            .unwrap();
        runtime
            .record_manual_request_disposition(
                workspace,
                manual,
                TaskmasterManualRequestDisposition::Skipped,
                TaskmasterManualRequestReason::ContextChanged,
            )
            .unwrap();
        runtime.clear_run(workspace, manual).unwrap();
        let status = runtime.run_status(Some(workspace)).unwrap();
        assert_eq!(status.latest_outcome, Some(before));
        assert_eq!(status.latest_manual_request.unwrap().id, manual.to_string());

        let background = runtime
            .queue_run(
                workspace,
                TaskmasterRunTrigger::Background,
                "codex",
                "model",
            )
            .unwrap();
        runtime
            .mark_run_running(workspace, background, "codex", "model")
            .unwrap();
        runtime
            .record_manual_request_disposition(
                workspace,
                background,
                TaskmasterManualRequestDisposition::Skipped,
                TaskmasterManualRequestReason::Unavailable,
            )
            .unwrap();
        runtime.clear_run(workspace, background).unwrap();
        assert_eq!(
            runtime
                .run_status(Some(workspace))
                .unwrap()
                .latest_manual_request
                .unwrap()
                .id,
            manual.to_string()
        );

        let next_manual = runtime
            .queue_run(workspace, TaskmasterRunTrigger::Manual, "codex", "model")
            .unwrap();
        assert_eq!(
            runtime
                .run_status(Some(workspace))
                .unwrap()
                .latest_manual_request,
            None
        );
        runtime.clear_run(workspace, next_manual).unwrap();
    }

    #[test]
    fn manual_skip_disposition_and_attempt_clear_are_one_durable_mutation() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("taskmaster");
        let runtime = TaskmasterRuntime::open(root.clone());
        let workspace = directory.path();
        let actual = runtime
            .queue_run(workspace, TaskmasterRunTrigger::Manual, "codex", "model")
            .unwrap();
        runtime
            .mark_run_running(workspace, actual, "codex", "model")
            .unwrap();
        runtime
            .finish_run(
                workspace,
                actual,
                TaskmasterRunOutcomeState::Succeeded,
                None,
            )
            .unwrap();
        let latest_outcome = runtime
            .run_status(Some(workspace))
            .unwrap()
            .latest_outcome
            .unwrap();

        let skipped = runtime
            .queue_run(workspace, TaskmasterRunTrigger::Manual, "codex", "model")
            .unwrap();
        runtime
            .mark_run_running(workspace, skipped, "codex", "model")
            .unwrap();
        assert!(runtime
            .finish_manual_request_without_outcome(
                workspace,
                skipped,
                TaskmasterManualRequestDisposition::Skipped,
                TaskmasterManualRequestReason::ContextChanged,
            )
            .unwrap());

        let reopened = TaskmasterRuntime::open(root);
        let status = reopened.run_status(Some(workspace)).unwrap();
        assert_eq!(status.active_attempt, None);
        assert_eq!(status.latest_outcome, Some(latest_outcome));
        let request = status.latest_manual_request.unwrap();
        assert_eq!(request.id, skipped.to_string());
        assert_eq!(
            request.disposition,
            TaskmasterManualRequestDisposition::Skipped
        );
        assert_eq!(
            request.reason,
            TaskmasterManualRequestReason::ContextChanged
        );
    }

    #[test]
    fn settings_reconciliation_failure_leaves_settings_and_generation_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        let before = runtime.status(Some(directory.path())).settings;
        let generation = runtime.data.lock().unwrap().ledger.generation;
        let mut changed = before.clone();
        changed.context_level = ContextLevel::SourceCode;
        assert!(runtime
            .replace_settings_with_reconcile(changed, |old, new| {
                assert_eq!(old, &before);
                assert_eq!(new.context_level, ContextLevel::SourceCode);
                Err("reconciliation rejected".into())
            })
            .unwrap_err()
            .contains("reconciliation rejected"));
        assert_eq!(runtime.status(Some(directory.path())).settings, before);
        assert_eq!(runtime.data.lock().unwrap().ledger.generation, generation);
    }

    #[test]
    fn legacy_analysis_error_remains_readable_but_is_not_serialized() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(
            directory.path().join("evaluations.json"),
            r#"{"day":"2026-09-30","reservations":2,"lastError":"old failure"}"#,
        )
        .unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        assert_eq!(
            runtime.status(None).last_error.as_deref(),
            Some("old failure")
        );
        assert!(!serde_json::to_string(&runtime.status(None))
            .unwrap()
            .contains("lastError"));
        let encoded = serde_json::to_string(&runtime.data.lock().unwrap().ledger).unwrap();
        assert!(!encoded.contains("lastError"));
        assert!(encoded.contains("workspaceRuns"));
    }

    #[test]
    fn taskmaster_run_transitions_preserve_latest_and_reject_stale_attempts() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        let run_id = runtime
            .queue_run(
                directory.path(),
                TaskmasterRunTrigger::Manual,
                "codex",
                "gpt-x",
            )
            .unwrap();
        assert!(runtime
            .mark_run_running(directory.path(), run_id, "codex", "gpt-x")
            .unwrap());
        assert!(!runtime
            .finish_run(
                directory.path(),
                run_id + 1,
                TaskmasterRunOutcomeState::Failed,
                Some("stale")
            )
            .unwrap());
        assert!(runtime
            .finish_run(
                directory.path(),
                run_id,
                TaskmasterRunOutcomeState::Failed,
                Some(&format!("{}\n", "x".repeat(600)))
            )
            .unwrap());
        let status = runtime.run_status(Some(directory.path())).unwrap();
        assert_eq!(status.active_attempt, None);
        let outcome = status.latest_outcome.unwrap();
        assert_eq!(outcome.state, TaskmasterRunOutcomeState::Failed);
        assert_eq!(outcome.error_summary.as_ref().unwrap().len(), 512);
        assert!(!outcome.error_summary.as_ref().unwrap().contains('\n'));
        let second = runtime
            .queue_run(
                directory.path(),
                TaskmasterRunTrigger::Background,
                "ollama",
                "llama",
            )
            .unwrap();
        assert!(runtime.clear_run(directory.path(), second).unwrap());
        assert_eq!(
            runtime
                .run_status(Some(directory.path()))
                .unwrap()
                .latest_outcome
                .unwrap(),
            outcome
        );
    }

    #[test]
    fn running_attempt_records_the_selection_used_by_the_evaluator() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        let run_id = runtime
            .queue_run(
                directory.path(),
                TaskmasterRunTrigger::Background,
                "codex",
                "old-model",
            )
            .unwrap();

        assert!(runtime
            .mark_run_running(directory.path(), run_id, "ollama", "llama3")
            .unwrap());
        let status = runtime.run_status(Some(directory.path())).unwrap();
        let attempt = status.active_attempt.unwrap();
        assert_eq!(attempt.provider, "ollama");
        assert_eq!(attempt.model, "llama3");
    }

    #[test]
    fn recovery_needs_analysis_lease_and_marks_only_running_attempt_interrupted() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        let run_id = runtime
            .queue_run(
                directory.path(),
                TaskmasterRunTrigger::Background,
                "codex",
                "gpt-x",
            )
            .unwrap();
        let lease = runtime.try_analysis_lease().unwrap().unwrap();
        let other_instance = TaskmasterRuntime::open(directory.path().into());
        assert!(other_instance.try_analysis_lease().unwrap().is_none());
        assert!(runtime
            .recover_workspace_run(directory.path(), &lease)
            .unwrap());
        assert_eq!(
            runtime
                .run_status(Some(directory.path()))
                .unwrap()
                .latest_outcome,
            None
        );
        assert_eq!(
            runtime
                .run_status(Some(directory.path()))
                .unwrap()
                .active_attempt,
            None
        );

        let next_id = runtime
            .queue_run(
                directory.path(),
                TaskmasterRunTrigger::Manual,
                "codex",
                "gpt-x",
            )
            .unwrap();
        assert!(runtime
            .mark_run_running(directory.path(), next_id, "ollama", "llama")
            .unwrap());
        assert!(runtime
            .recover_workspace_run(directory.path(), &lease)
            .unwrap());
        let outcome = runtime
            .run_status(Some(directory.path()))
            .unwrap()
            .latest_outcome
            .unwrap();
        assert_eq!(outcome.state, TaskmasterRunOutcomeState::Interrupted);
        assert_eq!(outcome.trigger, TaskmasterRunTrigger::Manual);
        assert_ne!(run_id, next_id);
    }

    #[test]
    fn reservation_budget_cache_and_generation_survive_reopening() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        runtime.replace_settings(configured()).unwrap();
        let generation = runtime
            .evaluation_snapshot(directory.path())
            .unwrap()
            .generation;
        assert!(runtime
            .reserve_current(
                directory.path(),
                "2026-09-09T00:00:00Z",
                Some("same"),
                Some(generation)
            )
            .unwrap());
        let reopened = TaskmasterRuntime::open(directory.path().into());
        assert!(reopened
            .reserve_current(
                directory.path(),
                "2026-09-09T01:00:00Z",
                Some("same"),
                Some(generation)
            )
            .unwrap());
        let mut narrowed = configured();
        narrowed.context_level = ContextLevel::SessionObservations;
        reopened.replace_settings(narrowed).unwrap();
        assert!(!runtime
            .reserve_current(
                directory.path(),
                "2026-09-09T01:00:00Z",
                Some("new"),
                Some(generation)
            )
            .unwrap());
        let generation = reopened
            .evaluation_snapshot(directory.path())
            .unwrap()
            .generation;
        assert!(reopened
            .reserve_current(
                directory.path(),
                "2026-09-09T02:00:00Z",
                Some("same"),
                Some(generation)
            )
            .unwrap());
        let ledger: EvaluationLedger =
            read_json(directory.path().join("evaluations.json")).unwrap();
        assert_eq!(ledger.reservations, 3);
    }

    #[test]
    fn manual_snapshot_allows_disabled_background_discovery() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        let mut settings = configured();
        settings.enabled = false;
        runtime.replace_settings(settings).unwrap();

        assert!(runtime.evaluation_snapshot(directory.path()).is_none());
        let snapshot = runtime
            .manual_evaluation_snapshot(directory.path())
            .expect("manual analysis should work while background discovery is disabled");
        assert!(runtime
            .reserve_current_manual(
                directory.path(),
                "2026-09-09T00:00:00Z",
                Some("manual"),
                Some(snapshot.generation),
            )
            .unwrap());
    }

    #[test]
    fn manual_reservation_bypasses_cooldown_and_daily_limits_but_keeps_cache() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        let mut settings = configured();
        settings.daily_evaluation_limit = 1;
        runtime.replace_settings(settings).unwrap();
        let generation = runtime
            .evaluation_snapshot(directory.path())
            .unwrap()
            .generation;

        assert!(runtime
            .reserve_current(
                directory.path(),
                "2026-09-09T00:00:00Z",
                Some("first"),
                Some(generation)
            )
            .unwrap());
        let workspace_key = directory
            .path()
            .canonicalize()
            .unwrap()
            .display()
            .to_string();
        let mut ledger: EvaluationLedger =
            read_json(directory.path().join("evaluations.json")).unwrap();
        ledger
            .workspace_cache_keys
            .insert(workspace_key, "accepted".into());
        write_json(&directory.path().join("evaluations.json"), &ledger).unwrap();
        assert!(!runtime
            .reserve_current(
                directory.path(),
                "2026-09-09T00:00:30Z",
                Some("second"),
                Some(generation)
            )
            .unwrap());
        assert!(!runtime
            .reserve_current_manual(
                directory.path(),
                "2026-09-09T00:00:30Z",
                Some("accepted"),
                Some(generation)
            )
            .unwrap());
        assert!(runtime
            .reserve_current_manual(
                directory.path(),
                "2026-09-09T00:00:30Z",
                Some("second"),
                Some(generation)
            )
            .unwrap());
        assert!(runtime
            .reserve_current_manual(
                directory.path(),
                "2026-09-09T00:02:00Z",
                Some("third"),
                Some(generation)
            )
            .unwrap());
        let ledger: EvaluationLedger =
            read_json(directory.path().join("evaluations.json")).unwrap();
        assert_eq!(
            ledger.reservations, 1,
            "manual runs must not consume the background discovery allowance"
        );
    }

    #[test]
    fn legacy_reservation_cache_does_not_suppress_a_retry_or_reset_usage() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        runtime.replace_settings(configured()).unwrap();
        let key = directory
            .path()
            .canonicalize()
            .unwrap()
            .display()
            .to_string();
        fs::write(
            directory.path().join("evaluations.json"),
            serde_json::json!({
                "day":"2026-09-12", "reservations":3, "generation":1,
                "workspaceCacheKeys":{key:"old-attempt"},
                "workspaceLastEvaluated":{}
            })
            .to_string(),
        )
        .unwrap();
        assert!(runtime
            .reserve_current(
                directory.path(),
                "2026-09-12T12:00:00Z",
                Some("old-attempt"),
                None
            )
            .unwrap());
        let ledger: EvaluationLedger =
            read_json(directory.path().join("evaluations.json")).unwrap();
        assert_eq!(ledger.reservations, 4);
    }

    #[test]
    fn analysis_lease_contention_errors_are_classified() {
        assert!(is_analysis_lease_contention(&std::io::Error::from(
            std::io::ErrorKind::WouldBlock,
        )));
        #[cfg(windows)]
        assert!(is_analysis_lease_contention(
            &std::io::Error::from_raw_os_error(33),
        ));
        assert!(!is_analysis_lease_contention(
            &std::io::Error::from_raw_os_error(5),
        ));
    }

    #[test]
    fn inference_lease_excludes_independent_runtime_instances_until_drop() {
        let directory = tempfile::tempdir().unwrap();
        let first = TaskmasterRuntime::open(directory.path().into());
        let second = TaskmasterRuntime::open(directory.path().into());
        let lease = first.try_analysis_lease().unwrap().unwrap();
        assert!(second.try_analysis_lease().unwrap().is_none());
        drop(lease);
        assert!(second.try_analysis_lease().unwrap().is_some());
    }

    #[test]
    fn conditional_commit_rejects_a_durably_changed_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        runtime.replace_settings(configured()).unwrap();
        let generation = runtime
            .evaluation_snapshot(directory.path())
            .unwrap()
            .generation;
        TaskmasterRuntime::open(directory.path().into())
            .replace_settings(TaskmasterSettings::default())
            .unwrap();
        assert!(!runtime
            .with_current_generation(generation, || panic!("stale output applied"))
            .unwrap());
    }
}
