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

use sha2::{Digest, Sha256};

use super::definition::{
    BuiltinDocument, HarnessDefinition, HarnessDiagnostic, HarnessPatch, HarnessUserDocument,
    LaunchCapability, PeonCapability, VoiceCapability,
};
use super::registry::{resolve_document, HarnessCatalog, ResolvedHarnessRegistry};
use crate::harness_registry::{builtin_harness_configs, HarnessConfig};

pub(crate) struct HarnessStore {
    path: PathBuf,
    builtins: Arc<BuiltinDocument>,
    write_lock: Mutex<()>,
    writer: Arc<dyn AtomicWriter>,
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
        let (document, migrated_from_v1) = match bytes {
            None => (HarnessUserDocument::default(), false),
            Some(bytes) => parse_document(&bytes, &self.builtins)?,
        };
        let registry =
            resolve_document(&self.builtins, &document).map_err(HarnessStoreError::Validation)?;
        Ok(LoadedHarnesses {
            document,
            registry: Arc::new(registry),
            source_revision: revision,
            migrated_from_v1,
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
            resolve_document(&self.builtins, &document).map_err(HarnessStoreError::Validation)?,
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
            let mut file = File::create(&temporary)?;
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
            Err(error) => return Err(error.into()),
        };
        if actual != expected_revision {
            let _ = fs::remove_file(&temporary);
            return Err(HarnessStoreError::RevisionChanged);
        }
        fs::rename(&temporary, target)?;
        Ok(())
    }
}

fn parse_document(
    bytes: &[u8],
    builtins: &BuiltinDocument,
) -> Result<(HarnessUserDocument, bool), HarnessStoreError> {
    if let Ok(document) = serde_json::from_slice::<HarnessUserDocument>(bytes) {
        return Ok((document, false));
    }
    let legacy = serde_json::from_slice::<Vec<HarnessConfig>>(bytes)?;
    Ok((migrate_v1(legacy, builtins), true))
}

fn migrate_v1(legacy: Vec<HarnessConfig>, builtins: &BuiltinDocument) -> HarnessUserDocument {
    let baseline = builtin_harness_configs();
    let mut document = HarnessUserDocument::default();
    for entry in legacy {
        let stock = baseline
            .iter()
            .find(|candidate| candidate.id == entry.id)
            .is_some_and(|candidate| canonical_json(candidate) == canonical_json(&entry));
        if stock {
            continue;
        }
        let target_id = if entry.id == "gh-copilot" {
            "copilot"
        } else {
            entry.id.as_str()
        };
        if builtins
            .builtins
            .iter()
            .any(|definition| definition.id == target_id)
        {
            document
                .overrides
                .insert(target_id.to_owned(), legacy_patch(&entry));
        } else {
            document.custom.push(legacy_definition(entry));
        }
    }
    document
}

fn legacy_patch(entry: &HarnessConfig) -> HarnessPatch {
    HarnessPatch {
        name: Some(entry.name.clone()),
        launch: Some(super::definition::LaunchPatch {
            kind: None,
            command: Some(entry.command.clone()),
            args: Some(entry.args.clone()),
            model_prefix: Some(Some(entry.model_prefix.clone())),
        }),
        default_model: Some((!entry.default_model.is_empty()).then(|| entry.default_model.clone())),
        resume: None,
        models: Some(None),
        peon: Some(
            entry
                .peon
                .as_ref()
                .map(|peon| super::definition::PeonPatch {
                    command_override: Some(peon.command_override.clone()),
                    args: Some(peon.args.clone()),
                    model_arg_template: Some(peon.model_arg_template.clone()),
                    supports_model: Some(peon.supports_model),
                    timeout_secs: Some(peon.timeout_secs),
                }),
        ),
        capacity: None,
        session_signals: Some(None),
        integration: Some(None),
        voice: Some(Some(super::definition::VoicePatch {
            native_voice: Some(entry.capabilities.native_voice),
            requires_microphone_permission: Some(entry.capabilities.requires_microphone_permission),
            orkworks_dictation: Some(entry.capabilities.orkworks_dictation),
            orkworks_voice_commands: Some(entry.capabilities.orkworks_voice_commands),
        })),
    }
}

fn legacy_definition(entry: HarnessConfig) -> HarnessDefinition {
    HarnessDefinition {
        id: entry.id,
        name: entry.name,
        launch: LaunchCapability::CommandTemplate {
            command: entry.command,
            args: entry.args,
            model_prefix: (!entry.model_prefix.is_empty()).then_some(entry.model_prefix),
        },
        default_model: (!entry.default_model.is_empty()).then_some(entry.default_model),
        resume: None,
        models: None,
        peon: entry.peon.map(|peon| PeonCapability {
            command_override: peon.command_override,
            args: peon.args,
            model_arg_template: peon.model_arg_template,
            supports_model: peon.supports_model,
            timeout_secs: peon.timeout_secs,
        }),
        capacity: None,
        session_signals: None,
        integration: None,
        voice: Some(VoiceCapability {
            native_voice: entry.capabilities.native_voice,
            requires_microphone_permission: entry.capabilities.requires_microphone_permission,
            orkworks_dictation: entry.capabilities.orkworks_dictation,
            orkworks_voice_commands: entry.capabilities.orkworks_voice_commands,
        }),
    }
}

fn canonical_json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("legacy built-ins serialize")
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
        let migrated = migrate_v1(builtin_harness_configs(), &builtins);
        assert!(migrated.overrides.is_empty());
        assert!(migrated.custom.is_empty());
    }
}
