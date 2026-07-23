//! Durable v2 harness-document storage.
//!
//! Legacy recognition is intentionally limited to the immediate pre-v2 main
//! baseline, `pre-v2-main@f13f460`. Its canonical serde JSON is hashed before
//! comparison. Older or unmatched entries are preserved conservatively with a
//! diagnostic rather than guessed to be stock.
//!
//! The HTTP CRUD and `AppState` catalog wiring are deliberately deferred to
//! Task 4. They must land atomically with runtime/provider migration so this
//! persistence core does not create an interim dual registry.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::definition::{
    BuiltinDocument, HarnessDefinition, HarnessDiagnostic, HarnessPatch, HarnessUserDocument,
    LaunchCapability, ModelCapability, PeonCapability, PeonPatch, VoiceCapability, VoicePatch,
};
use super::integration::atomic_replace;
use super::registry::{resolve_document, HarnessCatalog, ResolvedHarnessRegistry};

pub(crate) struct HarnessStore {
    path: PathBuf,
    builtins: Arc<BuiltinDocument>,
    write_lock: Mutex<()>,
    writer: Arc<dyn AtomicWriter>,
}

pub(crate) fn global_harnesses_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".orkworks")
        .join("harnesses.json")
}

pub(crate) trait AtomicWriter: Send + Sync {
    fn replace_if_revision(
        &self,
        target: &Path,
        expected_revision: Option<[u8; 32]>,
        contents: &[u8],
    ) -> Result<(), HarnessStoreError>;
}

#[derive(Debug)]
pub(crate) enum HarnessStoreError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    RevisionChanged,
    Validation(Vec<HarnessDiagnostic>),
    Mutation(HarnessDiagnostic),
}

impl From<std::io::Error> for HarnessStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<serde_json::Error> for HarnessStoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parse(error)
    }
}

pub(crate) struct LoadedHarnesses {
    pub document: HarnessUserDocument,
    pub registry: Arc<ResolvedHarnessRegistry>,
    pub source_revision: Option<[u8; 32]>,
    pub migrated_from_v1: bool,
    pub migration_diagnostics: Vec<HarnessDiagnostic>,
}

impl HarnessStore {
    pub(crate) fn new(path: PathBuf, builtins: Arc<BuiltinDocument>) -> Self {
        Self {
            path,
            builtins,
            write_lock: Mutex::new(()),
            writer: Arc::new(FileAtomicWriter),
        }
    }

    #[cfg(test)]
    fn with_writer(
        path: PathBuf,
        builtins: Arc<BuiltinDocument>,
        writer: Arc<dyn AtomicWriter>,
    ) -> Self {
        Self {
            path,
            builtins,
            write_lock: Mutex::new(()),
            writer,
        }
    }

    pub(crate) fn load(&self) -> Result<LoadedHarnesses, HarnessStoreError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let revision = bytes.as_deref().map(hash_bytes);
        let (document, migration_diagnostics, migrated_from_v1) = match bytes {
            None => (HarnessUserDocument::default(), Vec::new(), false),
            Some(bytes) => parse_document(&bytes, &self.builtins)?,
        };
        let registry = resolve_document(&self.builtins, &document)
            .map_err(HarnessStoreError::Validation)?
            .with_diagnostics(migration_diagnostics.clone());
        Ok(LoadedHarnesses {
            document,
            registry: Arc::new(registry),
            source_revision: revision,
            migrated_from_v1,
            migration_diagnostics,
        })
    }

    pub(crate) fn mutate<F>(
        &self,
        catalog: &HarnessCatalog,
        change: F,
    ) -> Result<Arc<ResolvedHarnessRegistry>, HarnessStoreError>
    where
        F: FnOnce(&mut HarnessUserDocument) -> Result<(), HarnessDiagnostic>,
    {
        let _guard = self.write_lock.lock().expect("harness store lock poisoned");
        let loaded = self.load()?;
        let mut document = loaded.document;
        change(&mut document).map_err(HarnessStoreError::Mutation)?;
        let registry = Arc::new(
            resolve_document(&self.builtins, &document)
                .map_err(HarnessStoreError::Validation)?
                .with_diagnostics(loaded.migration_diagnostics),
        );
        let serialized = serde_json::to_vec_pretty(&document)?;
        self.writer
            .replace_if_revision(&self.path, loaded.source_revision, &serialized)?;
        *catalog.write().expect("harness catalog lock poisoned") = registry.clone();
        Ok(registry)
    }
}

struct FileAtomicWriter;

impl AtomicWriter for FileAtomicWriter {
    fn replace_if_revision(
        &self,
        target: &Path,
        expected_revision: Option<[u8; 32]>,
        contents: &[u8],
    ) -> Result<(), HarnessStoreError> {
        let actual = match fs::read(target) {
            Ok(bytes) => Some(hash_bytes(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if actual != expected_revision {
            return Err(HarnessStoreError::RevisionChanged);
        }
        let parent = target.parent().ok_or_else(|| {
            HarnessStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "harness path has no parent",
            ))
        })?;
        fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(
            ".{}.{}.tmp",
            target
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("harnesses"),
            uuid::Uuid::new_v4()
        ));
        let write_result = (|| -> Result<(), HarnessStoreError> {
            let mut file = File::options()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(contents)?;
            file.sync_all()?;
            Ok(())
        })();
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        let actual = match fs::read(target) {
            Ok(bytes) => Some(hash_bytes(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                return Err(error.into());
            }
        };
        if actual != expected_revision {
            let _ = fs::remove_file(&temporary);
            return Err(HarnessStoreError::RevisionChanged);
        }
        match atomic_replace(&temporary, target, expected_revision.is_some()) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(error.into())
            }
        }
    }
}

fn parse_document(
    bytes: &[u8],
    builtins: &BuiltinDocument,
) -> Result<(HarnessUserDocument, Vec<HarnessDiagnostic>, bool), HarnessStoreError> {
    if let Ok(document) = serde_json::from_slice::<HarnessUserDocument>(bytes) {
        return Ok((document, Vec::new(), false));
    }
    let legacy = serde_json::from_slice::<Vec<serde_json::Value>>(bytes)?;
    let (document, diagnostics) = migrate_v1(legacy, builtins);
    Ok((document, diagnostics, true))
}

fn migrate_v1(
    legacy: Vec<serde_json::Value>,
    builtins: &BuiltinDocument,
) -> (HarnessUserDocument, Vec<HarnessDiagnostic>) {
    let baseline = legacy_baselines(builtins);
    let mut document = HarnessUserDocument::default();
    let mut diagnostics = Vec::new();
    for (index, raw_entry) in legacy.into_iter().enumerate() {
        let entry = match serde_json::from_value::<LegacyHarnessConfig>(raw_entry) {
            Ok(entry) => entry,
            Err(error) => {
                diagnostics.push(HarnessDiagnostic {
                    harness_id: None,
                    code: "invalid_legacy_entry".into(),
                    message: format!("Legacy harness entry {index} was skipped: {error}"),
                });
                continue;
            }
        };
        let snapshot = baseline.get(&entry.id);
        let stock = snapshot.is_some_and(|candidate| candidate == &entry);
        if stock {
            continue;
        }
        if snapshot.is_some_and(|candidate| entry.attention != candidate.attention) {
            diagnostics.push(HarnessDiagnostic::for_id(
                &entry.id,
                "legacy_attention_unrepresentable",
                "Legacy active-work-hook configuration cannot be represented without granting a custom definition compiled authority.",
            ));
        }
        if entry.id == "gh-copilot" {
            let id = unique_legacy_id("gh-copilot-legacy", &document, builtins);
            let safe_adapter = safe_adapter_definition(builtins, &entry.harness);
            document
                .custom
                .push(legacy_definition(entry, id, safe_adapter));
            diagnostics.push(HarnessDiagnostic::for_id(
                "gh-copilot",
                "legacy_gh_copilot_custom",
                "Customized legacy gh-copilot was preserved as a custom harness; it does not override interactive Copilot.",
            ));
        } else if let Some(snapshot) = snapshot {
            if entry.harness != snapshot.harness {
                let id = unique_legacy_id(&format!("{}-legacy", entry.id), &document, builtins);
                let safe_adapter = safe_adapter_definition(builtins, &entry.harness);
                let original_id = entry.id.clone();
                document
                    .custom
                    .push(legacy_definition(entry, id, safe_adapter));
                diagnostics.push(HarnessDiagnostic::for_id(
                    &original_id,
                    "legacy_harness_binding_custom",
                    "Legacy adapter binding changed, so the harness was preserved as a custom declarative definition.",
                ));
            } else if builtins
                .builtins
                .iter()
                .any(|definition| definition.id == entry.id)
            {
                document
                    .overrides
                    .insert(entry.id.clone(), legacy_patch(&entry, snapshot));
            } else {
                let id = unique_legacy_id(&entry.id, &document, builtins);
                let safe_adapter = safe_adapter_definition(builtins, &entry.harness);
                let original_id = entry.id.clone();
                document
                    .custom
                    .push(legacy_definition(entry, id, safe_adapter));
                diagnostics.push(HarnessDiagnostic::for_id(
                    &original_id,
                    "unmatched_legacy_harness",
                    "Legacy harness did not match the current built-ins and was frozen as a custom definition.",
                ));
            }
        } else {
            let id = unique_legacy_id(&format!("{}-legacy", entry.id), &document, builtins);
            let safe_adapter = safe_adapter_definition(builtins, &entry.harness);
            let original_id = entry.id.clone();
            document
                .custom
                .push(legacy_definition(entry, id, safe_adapter));
            diagnostics.push(HarnessDiagnostic::for_id(
                &original_id,
                "unmatched_legacy_harness",
                "Legacy harness did not match the captured pre-v2 baseline and was frozen as a custom definition.",
            ));
        }
    }
    (document, diagnostics)
}

fn legacy_patch(entry: &LegacyHarnessConfig, baseline: &LegacyHarnessConfig) -> HarnessPatch {
    let launch_changed = entry.command != baseline.command
        || entry.args != baseline.args
        || entry.model_prefix != baseline.model_prefix;
    let legacy_models_changed = entry.peon.as_ref().and_then(legacy_models)
        != baseline.peon.as_ref().and_then(legacy_models);
    HarnessPatch {
        name: (entry.name != baseline.name).then(|| entry.name.clone()),
        launch: launch_changed.then(|| super::definition::LaunchPatch {
            kind: None,
            command: (entry.command != baseline.command).then(|| entry.command.clone()),
            args: (entry.args != baseline.args).then(|| entry.args.clone()),
            model_prefix: (entry.model_prefix != baseline.model_prefix)
                .then(|| (!entry.model_prefix.is_empty()).then(|| entry.model_prefix.clone())),
            login: None,
        }),
        default_model: (entry.default_model != baseline.default_model)
            .then(|| (!entry.default_model.is_empty()).then(|| entry.default_model.clone())),
        resume: None,
        models: legacy_models_changed.then(|| entry.peon.as_ref().and_then(legacy_models)),
        peon: legacy_peon_patch(entry.peon.as_ref(), baseline.peon.as_ref()),
        capacity: None,
        session_signals: None,
        integration: None,
        voice: legacy_voice_patch(&entry.capabilities, &baseline.capabilities),
    }
}

fn legacy_peon_patch(
    entry: Option<&LegacyPeonConfig>,
    baseline: Option<&LegacyPeonConfig>,
) -> Option<Option<PeonPatch>> {
    match (entry, baseline) {
        (None, None) => None,
        (None, Some(_)) => Some(None),
        (Some(entry), None) => Some(Some(PeonPatch {
            command_override: Some(entry.command_override.clone()),
            args: Some(entry.args.clone()),
            model_arg_template: Some(entry.model_arg_template.clone()),
            supports_model: Some(entry.supports_model),
            timeout_secs: Some(entry.timeout_secs),
        })),
        (Some(entry), Some(baseline)) => {
            let patch = PeonPatch {
                command_override: (entry.command_override != baseline.command_override)
                    .then(|| entry.command_override.clone()),
                args: (entry.args != baseline.args).then(|| entry.args.clone()),
                model_arg_template: (entry.model_arg_template != baseline.model_arg_template)
                    .then(|| entry.model_arg_template.clone()),
                supports_model: (entry.supports_model != baseline.supports_model)
                    .then_some(entry.supports_model),
                timeout_secs: (entry.timeout_secs != baseline.timeout_secs)
                    .then_some(entry.timeout_secs),
            };
            (patch != PeonPatch::default()).then_some(Some(patch))
        }
    }
}

fn legacy_voice_patch(
    entry: &LegacyVoiceCapabilities,
    baseline: &LegacyVoiceCapabilities,
) -> Option<Option<VoicePatch>> {
    let patch = VoicePatch {
        native_voice: (entry.native_voice != baseline.native_voice).then_some(entry.native_voice),
        requires_microphone_permission: (entry.requires_microphone_permission
            != baseline.requires_microphone_permission)
            .then_some(entry.requires_microphone_permission),
        orkworks_dictation: (entry.orkworks_dictation != baseline.orkworks_dictation)
            .then_some(entry.orkworks_dictation),
        orkworks_voice_commands: (entry.orkworks_voice_commands
            != baseline.orkworks_voice_commands)
            .then_some(entry.orkworks_voice_commands),
    };
    (patch != VoicePatch::default()).then_some(Some(patch))
}

fn legacy_models(peon: &LegacyPeonConfig) -> Option<ModelCapability> {
    if peon.http_list_models {
        Some(ModelCapability::Http)
    } else if let Some(command) = &peon.list_models_command {
        Some(ModelCapability::Command {
            command: command.clone(),
            args: peon.list_models_args.clone(),
        })
    } else if !peon.static_models.is_empty() {
        Some(ModelCapability::Static {
            models: peon.static_models.clone(),
        })
    } else {
        None
    }
}

fn safe_adapter_definition<'a>(
    builtins: &'a BuiltinDocument,
    legacy_harness: &str,
) -> Option<&'a HarnessDefinition> {
    builtins
        .builtins
        .iter()
        .find(|definition| definition.id == legacy_harness)
}

fn legacy_definition(
    entry: LegacyHarnessConfig,
    id: String,
    safe_adapter: Option<&HarnessDefinition>,
) -> HarnessDefinition {
    HarnessDefinition {
        id,
        name: entry.name,
        launch: LaunchCapability::CommandTemplate {
            command: entry.command,
            args: entry.args,
            model_prefix: (!entry.model_prefix.is_empty()).then_some(entry.model_prefix),
        },
        default_model: (!entry.default_model.is_empty()).then_some(entry.default_model),
        resume: safe_adapter.and_then(|definition| definition.resume.clone()),
        models: entry.peon.as_ref().and_then(legacy_models),
        peon: entry.peon.map(|peon| PeonCapability {
            command_override: peon.command_override,
            args: peon.args,
            model_arg_template: peon.model_arg_template,
            supports_model: peon.supports_model,
            timeout_secs: peon.timeout_secs,
        }),
        capacity: safe_adapter.and_then(|definition| definition.capacity.clone()),
        session_signals: None,
        integration: None,
        voice: legacy_voice(&entry.capabilities),
    }
}

fn legacy_voice(capabilities: &LegacyVoiceCapabilities) -> Option<VoiceCapability> {
    (capabilities.native_voice
        || capabilities.requires_microphone_permission
        || capabilities.orkworks_dictation
        || capabilities.orkworks_voice_commands)
        .then_some(VoiceCapability {
            native_voice: capabilities.native_voice,
            requires_microphone_permission: capabilities.requires_microphone_permission,
            orkworks_dictation: capabilities.orkworks_dictation,
            orkworks_voice_commands: capabilities.orkworks_voice_commands,
        })
}

fn legacy_baselines(
    builtins: &BuiltinDocument,
) -> std::collections::HashMap<String, LegacyHarnessConfig> {
    builtins
        .legacy_snapshots
        .iter()
        .filter_map(|snapshot| {
            snapshot
                .definition
                .as_ref()
                .and_then(|definition| serde_json::from_value(definition.clone()).ok())
                .map(|definition| (snapshot.harness_id.clone(), definition))
        })
        .collect()
}

fn unique_legacy_id(
    preferred: &str,
    document: &HarnessUserDocument,
    builtins: &BuiltinDocument,
) -> String {
    let mut candidate = preferred.to_owned();
    let mut suffix = 2;
    while builtins
        .builtins
        .iter()
        .any(|definition| definition.id == candidate)
        || candidate == "gh-copilot"
        || document
            .custom
            .iter()
            .any(|definition| definition.id == candidate)
    {
        candidate = format!("{preferred}-{suffix}");
        suffix += 1;
    }
    candidate
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct LegacyHarnessConfig {
    id: String,
    name: String,
    harness: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    default_model: String,
    #[serde(default)]
    model_prefix: String,
    #[serde(default)]
    capabilities: LegacyVoiceCapabilities,
    #[serde(default)]
    attention: LegacyAttentionCapabilities,
    #[serde(default)]
    is_builtin: bool,
    #[serde(default)]
    peon: Option<LegacyPeonConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct LegacyVoiceCapabilities {
    native_voice: bool,
    requires_microphone_permission: bool,
    orkworks_dictation: bool,
    orkworks_voice_commands: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct LegacyAttentionCapabilities {
    active_work_hook: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct LegacyPeonConfig {
    command_override: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    model_arg_template: Option<String>,
    #[serde(default)]
    supports_model: bool,
    #[serde(default = "default_legacy_peon_timeout")]
    timeout_secs: u64,
    list_models_command: Option<String>,
    #[serde(default)]
    list_models_args: Vec<String>,
    #[serde(default)]
    static_models: Vec<String>,
    #[serde(default)]
    http_list_models: bool,
}

fn default_legacy_peon_timeout() -> u64 {
    30
}
fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, RwLock};

    use super::*;
    use crate::harness::definition::EMBEDDED_BUILTINS;

    fn captured_legacy_entry(id: &str) -> LegacyHarnessConfig {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let raw = builtins
            .legacy_snapshots
            .iter()
            .find(|snapshot| snapshot.harness_id == id)
            .and_then(|snapshot| snapshot.definition.clone())
            .expect("captured legacy entry");
        serde_json::from_value(raw).unwrap()
    }

    fn captured_legacy_values() -> Vec<serde_json::Value> {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        builtins
            .legacy_snapshots
            .into_iter()
            .filter_map(|snapshot| snapshot.definition)
            .collect()
    }

    struct FailingWriter {
        fail: AtomicBool,
    }
    impl FailingWriter {
        fn new() -> Self {
            Self {
                fail: AtomicBool::new(false),
            }
        }
        fn fail_next_replace(&self) {
            self.fail.store(true, Ordering::SeqCst);
        }
    }
    impl AtomicWriter for FailingWriter {
        fn replace_if_revision(
            &self,
            target: &Path,
            expected: Option<[u8; 32]>,
            contents: &[u8],
        ) -> Result<(), HarnessStoreError> {
            if self.fail.swap(false, Ordering::SeqCst) {
                return Err(HarnessStoreError::RevisionChanged);
            }
            FileAtomicWriter.replace_if_revision(target, expected, contents)
        }
    }

    struct StoreFixture {
        _dir: tempfile::TempDir,
        path: PathBuf,
        store: HarnessStore,
        catalog: HarnessCatalog,
        writer: Arc<FailingWriter>,
    }
    impl StoreFixture {
        fn v2() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("harnesses.json");
            let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
            let registry =
                Arc::new(resolve_document(&builtins, &HarnessUserDocument::default()).unwrap());
            let writer = Arc::new(FailingWriter::new());
            Self {
                _dir: dir,
                path: path.clone(),
                store: HarnessStore::with_writer(path, builtins, writer.clone()),
                catalog: Arc::new(RwLock::new(registry)),
                writer,
            }
        }
        fn read_document(&self) -> HarnessUserDocument {
            serde_json::from_slice(
                &fs::read(&self.path).unwrap_or_else(|_| {
                    serde_json::to_vec(&HarnessUserDocument::default()).unwrap()
                }),
            )
            .unwrap()
        }
    }

    #[test]
    fn failed_replace_leaves_disk_and_live_catalog_unchanged() {
        let fixture = StoreFixture::v2();
        let before = fixture.catalog.read().unwrap().clone();
        fixture.writer.fail_next_replace();
        assert!(fixture
            .store
            .mutate(&fixture.catalog, |document| {
                document.overrides.entry("codex".into()).or_default().name = Some("Changed".into());
                Ok(())
            })
            .is_err());
        assert!(Arc::ptr_eq(&before, &fixture.catalog.read().unwrap()));
        assert_eq!(
            fixture.read_document().overrides,
            HarnessUserDocument::default().overrides
        );
    }

    #[test]
    fn missing_file_loads_an_empty_v2_document() {
        let fixture = StoreFixture::v2();
        let loaded = fixture.store.load().unwrap();
        assert!(!loaded.migrated_from_v1);
        assert_eq!(loaded.document.version, 2);
    }

    #[test]
    fn exact_pre_v2_stock_is_unmodified_and_gh_copilot_is_replaced() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let (migrated, diagnostics) = migrate_v1(captured_legacy_values(), &builtins);
        assert!(migrated.overrides.is_empty());
        assert!(migrated.custom.is_empty());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn command_only_legacy_builtin_override_inherits_new_v2_capabilities() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let mut legacy = captured_legacy_entry("codex");
        legacy.command = "codex-wrapper".into();

        let (migrated, _) = migrate_v1(vec![serde_json::to_value(legacy).unwrap()], &builtins);
        let resolved = resolve_document(&builtins, &migrated).unwrap();
        let codex = &resolved.get("codex").expect("resolved codex").definition;

        assert_eq!(
            codex.launch,
            LaunchCapability::CommandTemplate {
                command: "codex-wrapper".into(),
                args: vec![],
                model_prefix: None,
            }
        );
        assert!(
            codex.models.is_some(),
            "current model discovery must remain"
        );
        assert!(
            codex.session_signals.is_some(),
            "current signals must remain"
        );
        assert!(
            codex.integration.is_some(),
            "current integration must remain"
        );
        assert!(codex.peon.is_some(), "current Peon config must remain");
    }

    #[test]
    fn changed_legacy_adapter_binding_freezes_a_known_builtin_as_custom() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let mut legacy = captured_legacy_entry("codex");
        legacy.harness = "claude-code".into();

        let (document, diagnostics) =
            migrate_v1(vec![serde_json::to_value(legacy).unwrap()], &builtins);
        let custom = document.custom.first().expect("frozen custom definition");

        assert!(document.overrides.is_empty());
        assert_eq!(custom.id, "codex-legacy");
        assert!(custom.resume.is_some());
        assert!(custom.capacity.is_some());
        assert!(custom.session_signals.is_none());
        assert!(custom.integration.is_none());
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "legacy_harness_binding_custom"));
    }

    #[test]
    fn unmatched_legacy_entry_inherits_safe_adapter_resume_and_capacity() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let mut legacy = captured_legacy_entry("codex");
        legacy.id = "old-wrapper".into();
        legacy.harness = "claude-code".into();

        let (document, _) = migrate_v1(vec![serde_json::to_value(legacy).unwrap()], &builtins);
        let custom = document.custom.first().expect("frozen custom definition");

        assert!(custom.resume.is_some());
        assert!(custom.capacity.is_some());
        assert!(custom.session_signals.is_none());
        assert!(custom.integration.is_none());
    }

    #[test]
    fn changed_legacy_attention_is_reported_without_granting_authority() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let mut legacy = captured_legacy_entry("codex");
        legacy.attention.active_work_hook = true;

        let (document, diagnostics) =
            migrate_v1(vec![serde_json::to_value(legacy).unwrap()], &builtins);

        assert!(document.custom.is_empty());
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "legacy_attention_unrepresentable"));
    }

    #[test]
    fn legacy_peon_execution_change_does_not_replace_unchanged_model_discovery() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let mut legacy = captured_legacy_entry("opencode");
        legacy
            .peon
            .as_mut()
            .expect("legacy Peon config")
            .args
            .push("--quiet".into());
        let baseline = legacy_baselines(&builtins)
            .remove("opencode")
            .expect("captured baseline");

        let patch = legacy_patch(&legacy, &baseline);

        assert!(patch.models.is_none());
        assert!(patch.peon.is_some());
    }

    #[test]
    fn customized_legacy_gh_copilot_becomes_a_custom_harness() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let mut legacy = captured_legacy_entry("gh-copilot");
        legacy.command = "gh-wrapper".into();

        let (migrated, diagnostics) =
            migrate_v1(vec![serde_json::to_value(legacy).unwrap()], &builtins);

        assert!(!migrated.overrides.contains_key("copilot"));
        assert!(migrated
            .custom
            .iter()
            .any(|definition| definition.id == "gh-copilot-legacy"));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "legacy_gh_copilot_custom"));
    }

    #[test]
    fn invalid_legacy_entry_does_not_block_a_valid_one() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let valid = captured_legacy_entry("codex");
        let bytes = serde_json::to_vec(&vec![
            serde_json::json!({"id": 7}),
            serde_json::to_value(valid).unwrap(),
        ])
        .unwrap();

        let (document, diagnostics, migrated) =
            parse_document(&bytes, &builtins).expect("partial migration");

        assert!(migrated);
        assert!(document.overrides.is_empty());
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "invalid_legacy_entry"));
    }

    #[test]
    fn unmatched_legacy_entry_freezes_models_peon_and_voice_with_a_diagnostic() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let mut legacy = captured_legacy_entry("codex");
        legacy.id = "historic-codex".into();
        legacy.name = "Historic Codex".into();
        legacy.capabilities.native_voice = true;

        let (document, diagnostics) =
            migrate_v1(vec![serde_json::to_value(legacy).unwrap()], &builtins);
        let custom = document.custom.first().expect("frozen custom definition");

        assert_eq!(custom.id, "historic-codex-legacy");
        assert!(matches!(
            custom.models,
            Some(ModelCapability::Static { .. })
        ));
        assert!(custom.peon.is_some());
        assert_eq!(
            custom.voice.as_ref().map(|voice| voice.native_voice),
            Some(true)
        );
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "unmatched_legacy_harness"));
    }

    #[test]
    fn legacy_all_false_voice_is_not_frozen_as_voice_support() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let mut legacy = captured_legacy_entry("codex");
        legacy.id = "voice-less-legacy".into();

        let (document, _) = migrate_v1(vec![serde_json::to_value(legacy).unwrap()], &builtins);

        assert!(document
            .custom
            .first()
            .expect("custom definition")
            .voice
            .is_none());
    }

    #[test]
    fn environment_dependent_generic_shell_is_never_treated_as_stock() {
        let builtins = Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap());
        let mut legacy = captured_legacy_entry("codex");
        legacy.id = "generic-shell".into();
        legacy.name = "Shell".into();
        legacy.harness = "generic-shell".into();
        legacy.command = "/bin/zsh".into();
        legacy.args = vec!["-i".into(), "-l".into()];
        legacy.peon = None;

        let (document, diagnostics) =
            migrate_v1(vec![serde_json::to_value(legacy).unwrap()], &builtins);

        assert!(document
            .custom
            .iter()
            .any(|definition| definition.id == "generic-shell-legacy"));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "unmatched_legacy_harness"));
    }

    #[test]
    fn published_registry_includes_transient_migration_diagnostics() {
        let fixture = StoreFixture::v2();
        fs::write(
            &fixture.path,
            serde_json::to_vec(&vec![serde_json::json!({"id": 7})]).unwrap(),
        )
        .unwrap();

        let loaded = fixture.store.load().expect("legacy file loads");

        assert!(loaded
            .registry
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code == "invalid_legacy_entry" }));
        assert_eq!(loaded.document.version, 2);
        assert!(!serde_json::to_string(&loaded.document)
            .unwrap()
            .contains("invalid_legacy_entry"));
    }
}
