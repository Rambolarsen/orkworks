use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

mod inference;

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

const MAX_PAGES: usize = 256;
const MAX_PAGE_BYTES: usize = 64 * 1024;
const MAX_BUNDLE_BYTES: usize = 2 * 1024 * 1024;
const MAX_EXCLUDED_PATHS: usize = 64;
const MAX_PATH_BYTES: usize = 512;
const MAX_DAILY_EVALUATIONS: u32 = 64;
const MAX_INTERVAL_MINUTES: u32 = 24 * 60;

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
    pub settings: TaskmasterSettings,
    pub effective_settings: TaskmasterSettings,
    pub remaining_evaluations: u32,
    pub analysis_status: String,
    pub knowledge_version: Option<String>,
    pub last_evaluated_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvaluationLedger {
    day: String,
    reservations: u32,
    #[serde(default)]
    workspace_last_evaluated: BTreeMap<String, String>,
    #[serde(default)]
    workspace_cache_keys: BTreeMap<String, String>,
    #[serde(default)]
    last_error: Option<String>,
    #[serde(default)]
    generation: u64,
}

struct RuntimeData {
    settings: TaskmasterSettings,
    knowledge: Option<KnowledgeBundle>,
    ledger: EvaluationLedger,
    ledger_readable: bool,
}

#[derive(Clone)]
pub(crate) struct EvaluationSnapshot {
    pub settings: TaskmasterSettings,
    pub knowledge: Option<KnowledgeBundle>,
    pub generation: u64,
    pub custom_inference: Option<inference::CapturedInference>,
    pub native_revision: Option<super::provider_catalog::NativeRevision>,
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
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let file_lock = persistence_file_lock(&root);
        let settings = read_json(root.join("settings.json")).unwrap_or_default();
        let knowledge = read_json(root.join("knowledge.json"))
            .filter(|bundle: &KnowledgeBundle| validate_bundle(bundle).is_ok());
        let ledger = read_json(root.join("evaluations.json"));
        let ledger_readable =
            file_lock.is_ok() && (ledger.is_some() || !root.join("evaluations.json").exists());
        Self {
            root,
            data: Mutex::new(RuntimeData {
                settings,
                knowledge,
                ledger: ledger.unwrap_or_default(),
                ledger_readable,
            }),
        }
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

    pub(crate) fn replace_settings(&self, settings: TaskmasterSettings) -> Result<(), String> {
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
        data.ledger.generation = data.ledger.generation.saturating_add(1);
        data.ledger.workspace_cache_keys.clear();
        write_json(&self.root.join("evaluations.json"), &data.ledger)?;
        write_json(&self.root.join("settings.json"), &settings)?;
        data.settings = settings;
        Ok(())
    }

    pub(crate) fn activate_knowledge(&self, bundle: KnowledgeBundle) -> Result<(), String> {
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let _file_lock = persistence_file_lock(&self.root)?;
        validate_bundle(&bundle)?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable {
            return Err("Taskmaster evaluation ledger is unreadable".into());
        }
        if data.knowledge.as_ref() == Some(&bundle) {
            return Ok(());
        }
        if data
            .knowledge
            .as_ref()
            .is_some_and(|current| bundle.sequence < current.sequence)
        {
            return Err("knowledge bundle sequence is older than the active bundle".into());
        }
        data.ledger.generation = data.ledger.generation.saturating_add(1);
        write_json(&self.root.join("evaluations.json"), &data.ledger)?;
        write_json(&self.root.join("knowledge.json"), &bundle)?;
        data.knowledge = Some(bundle);
        Ok(())
    }

    /// Reservations are durable before a provider call. A failed call consumes
    /// its reservation, so process restarts cannot reset usage accounting.
    pub(crate) fn reserve_evaluation(&self, workspace: &Path, now: &str) -> Result<bool, String> {
        self.reserve_evaluation_for_inputs(workspace, now, None)
    }

    pub(crate) fn reserve_evaluation_for_inputs(
        &self,
        workspace: &Path,
        now: &str,
        cache_key: Option<&str>,
    ) -> Result<bool, String> {
        self.reserve_current(workspace, now, cache_key, None)
    }

    pub(crate) fn reserve_current(
        &self,
        workspace: &Path,
        now: &str,
        cache_key: Option<&str>,
        generation: Option<u64>,
    ) -> Result<bool, String> {
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let _file_lock = persistence_file_lock(&self.root)?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        self.reserve_loaded(&mut data, workspace, now, cache_key, generation)
    }

    /// Caller owns the persistence lease and runtime-data mutex through the write.
    fn reserve_loaded(
        &self,
        data: &mut RuntimeData,
        workspace: &Path,
        now: &str,
        cache_key: Option<&str>,
        generation: Option<u64>,
    ) -> Result<bool, String> {
        let Some(workspace) = canonical_workspace_key(workspace) else {
            return Ok(false);
        };
        if !data.ledger_readable {
            return Err("Taskmaster evaluation ledger is unreadable".into());
        }
        if generation.is_some_and(|expected| expected != data.ledger.generation) {
            return Ok(false);
        }
        let effective = effective_settings(&data.settings, &workspace);
        if !effective.enabled || effective.selection.is_none() {
            return Ok(false);
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
        if data.ledger.reservations >= effective.daily_evaluation_limit {
            return Ok(false);
        }
        if cache_key.is_some_and(|cache_key| {
            data.ledger
                .workspace_cache_keys
                .get(&workspace)
                .is_some_and(|existing| existing == cache_key)
        }) {
            return Ok(false);
        }
        if let Some(last) = data.ledger.workspace_last_evaluated.get(&workspace) {
            let last = chrono::DateTime::parse_from_rfc3339(last).ok();
            let current = chrono::DateTime::parse_from_rfc3339(now).ok();
            if last.zip(current).is_some_and(|(last, current)| {
                current.signed_duration_since(last).num_minutes()
                    < i64::from(effective.min_interval_minutes)
            }) {
                return Ok(false);
            }
        }
        data.ledger.reservations = data.ledger.reservations.saturating_add(1);
        data.ledger
            .workspace_last_evaluated
            .insert(workspace.clone(), now.to_string());
        if let Some(cache_key) = cache_key {
            data.ledger
                .workspace_cache_keys
                .insert(workspace.clone(), cache_key.to_string());
        }
        write_json(&self.root.join("evaluations.json"), &data.ledger)?;
        Ok(true)
    }

    pub(crate) fn knowledge_pages(&self) -> Vec<KnowledgePage> {
        self.data
            .lock()
            .expect("taskmaster runtime lock poisoned")
            .knowledge
            .as_ref()
            .map(|bundle| bundle.pages.clone())
            .unwrap_or_default()
    }

    pub(crate) fn evaluation_snapshot(&self, workspace: &Path) -> Option<EvaluationSnapshot> {
        let workspace = canonical_workspace_key(workspace)?;
        let data = self.data.lock().expect("taskmaster runtime lock poisoned");
        if !data.ledger_readable {
            return None;
        }
        let settings = effective_settings(&data.settings, &workspace);
        (settings.enabled && settings.selection.is_some()).then(|| EvaluationSnapshot {
            settings,
            knowledge: data.knowledge.clone(),
            generation: data.ledger.generation,
            custom_inference: None,
            native_revision: None,
        })
    }

    pub(crate) fn record_error(
        &self,
        generation: u64,
        error: Option<String>,
    ) -> Result<bool, String> {
        let _persistence = PERSISTENCE_LOCK
            .lock()
            .expect("taskmaster persistence lock poisoned");
        let _file_lock = persistence_file_lock(&self.root)?;
        let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable || data.ledger.generation != generation {
            return Ok(false);
        }
        data.ledger.last_error = error;
        write_json(&self.root.join("evaluations.json"), &data.ledger)?;
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
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Holds the configuration lock through applying output, closing the check/write race.
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
}

pub(crate) fn taskmaster_global_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".orkworks").join("taskmaster"))
}

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

fn effective_settings(base: &TaskmasterSettings, workspace: &str) -> TaskmasterSettings {
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
    if let Some(knowledge) = read_json(root.join("knowledge.json"))
        .filter(|bundle: &KnowledgeBundle| validate_bundle(bundle).is_ok())
    {
        data.knowledge = Some(knowledge);
    }
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
    fs::create_dir_all(path.parent().ok_or("settings path has no parent")?)
        .map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(
        &temporary,
        serde_json::to_vec(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .reserve_evaluation(directory.path(), "2026-09-09T00:00:00Z")
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
        assert!(!reopened
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
                "2026-09-09T01:00:00Z",
                Some("same"),
                Some(generation)
            )
            .unwrap());
        let ledger: EvaluationLedger =
            read_json(directory.path().join("evaluations.json")).unwrap();
        assert_eq!(ledger.reservations, 2);
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
