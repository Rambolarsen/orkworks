// These primitives become live when Task 6 wires the concrete handlers. Keep
// their staged boundary explicit without weakening crate-wide linting.
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{
        json_ownership, parse_json_object, ConfigFileTransaction, IntegrationError,
        IntegrationOwnership, ValidatedWorkspaceTarget,
    };

    #[test]
    fn target_validation_accepts_a_missing_leaf_below_an_existing_workspace_directory() {
        let workspace = tempfile::tempdir().unwrap();
        std::fs::create_dir(workspace.path().join(".codex")).unwrap();

        let target =
            ValidatedWorkspaceTarget::new(workspace.path(), Path::new(".codex/hooks.json"))
                .unwrap();

        assert_eq!(target.relative_path(), Path::new(".codex/hooks.json"));
    }

    #[test]
    fn target_validation_rejects_an_absolute_or_parent_relative_path() {
        let workspace = tempfile::tempdir().unwrap();

        for path in [Path::new("../outside.json"), Path::new("/outside.json")] {
            let error = ValidatedWorkspaceTarget::new(workspace.path(), path).unwrap_err();
            assert_eq!(error.code(), "invalid_relative_path");
        }
    }

    #[cfg(unix)]
    #[test]
    fn target_validation_rejects_file_and_directory_symlink_escapes() {
        use std::os::unix::fs::symlink;

        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("hooks.json"), "{}").unwrap();
        symlink(
            outside.path().join("hooks.json"),
            workspace.path().join("file-link"),
        )
        .unwrap();
        symlink(outside.path(), workspace.path().join("directory-link")).unwrap();

        for path in [
            Path::new("file-link"),
            Path::new("directory-link/hooks.json"),
        ] {
            let error = ValidatedWorkspaceTarget::new(workspace.path(), path).unwrap_err();
            assert_eq!(error.code(), "workspace_escape");
        }
    }

    #[test]
    fn transaction_rejects_an_external_edit_before_replace() {
        let workspace = tempfile::tempdir().unwrap();
        let relative = Path::new(".codex/hooks.json");
        fs::create_dir(workspace.path().join(".codex")).unwrap();
        let path = workspace.path().join(relative);
        fs::write(&path, "old").unwrap();
        let target = ValidatedWorkspaceTarget::new(workspace.path(), relative).unwrap();
        let transaction = ConfigFileTransaction::open(target)
            .unwrap()
            .with_before_replace(|path| fs::write(path, "external").unwrap());

        assert!(matches!(
            transaction.commit(b"replacement"),
            Err(IntegrationError::RevisionChanged)
        ));
        assert_eq!(fs::read_to_string(path).unwrap(), "external");
    }

    #[test]
    fn transaction_cleans_up_temp_file_when_replace_fails() {
        let workspace = tempfile::tempdir().unwrap();
        fs::create_dir(workspace.path().join(".codex")).unwrap();
        let target =
            ValidatedWorkspaceTarget::new(workspace.path(), Path::new(".codex/hooks.json"))
                .unwrap();

        let error = ConfigFileTransaction::open(target)
            .unwrap()
            .with_replace(|_, _| Err(std::io::Error::other("injected failure")))
            .commit(b"replacement")
            .unwrap_err();

        assert!(matches!(error, IntegrationError::Io(_)));
        assert!(fs::read_dir(workspace.path().join(".codex"))
            .unwrap()
            .next()
            .is_none());
    }

    #[test]
    fn malformed_json_is_reported_without_a_panic() {
        assert!(matches!(
            parse_json_object(b"{not json"),
            Err(IntegrationError::InvalidConfig(_))
        ));
    }

    #[test]
    fn confirmation_constructor_never_accepts_absolute_or_parent_paths() {
        let error = super::IntegrationConfirmation::new(
            "Codex",
            Path::new("/workspace/demo"),
            "Limited metadata reporting",
            &[Path::new(".codex/hooks.json"), Path::new("../secret")],
            false,
        )
        .unwrap_err();
        assert_eq!(error.code(), "invalid_relative_path");
    }

    #[test]
    fn reporter_asset_resolver_copies_only_code_owned_file_names_to_stable_storage() {
        let source = tempfile::tempdir().unwrap();
        let stable = tempfile::tempdir().unwrap();
        fs::write(source.path().join("report.sh"), "#!/bin/sh\n").unwrap();
        let resolver = super::ReporterAssetResolver {
            source_dir: source.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };

        let resolved = resolver.reconcile("report.sh").unwrap();
        assert_eq!(resolved, stable.path().join("hook-scripts/report.sh"));
        assert_eq!(fs::read_to_string(resolved).unwrap(), "#!/bin/sh\n");
        assert_eq!(
            resolver.reconcile("../report.sh").unwrap_err().code(),
            "invalid_asset_name"
        );
    }

    #[test]
    fn ownership_requires_the_exact_expected_marker() {
        let exact = serde_json::json!({"marker": "orkworks:harness-integration:v2:codex"});
        let different =
            serde_json::json!({"marker": "orkworks:harness-integration:v2:claude-code"});
        let unrelated = serde_json::json!({"marker": "someone-else"});

        assert_eq!(
            json_ownership(&exact, "orkworks:harness-integration:v2:codex"),
            IntegrationOwnership::OrkWorks
        );
        assert_eq!(
            json_ownership(&different, "orkworks:harness-integration:v2:codex"),
            IntegrationOwnership::Ambiguous
        );
        assert_eq!(
            json_ownership(&unrelated, "orkworks:harness-integration:v2:codex"),
            IntegrationOwnership::None
        );
    }

    #[test]
    fn git_safety_rejects_tracked_and_unignored_targets_but_accepts_ignored_local_targets() {
        let workspace = tempfile::tempdir().unwrap();
        let repository = git2::Repository::init(workspace.path()).unwrap();
        fs::create_dir(workspace.path().join(".codex")).unwrap();
        let relative = Path::new(".codex/hooks.json");
        let path = workspace.path().join(relative);
        fs::write(&path, "{}").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(relative).unwrap();
        index.write().unwrap();

        let tracked = ValidatedWorkspaceTarget::new(workspace.path(), relative).unwrap();
        assert_eq!(
            tracked
                .require_local_or_ignored_untracked()
                .unwrap_err()
                .code(),
            "tracked_target"
        );

        index.remove_path(relative).unwrap();
        index.write().unwrap();
        let unignored = ValidatedWorkspaceTarget::new(workspace.path(), relative).unwrap();
        assert_eq!(
            unignored
                .require_local_or_ignored_untracked()
                .unwrap_err()
                .code(),
            "not_ignored_target"
        );

        fs::write(workspace.path().join(".gitignore"), ".codex/hooks.json\n").unwrap();
        let ignored = ValidatedWorkspaceTarget::new(workspace.path(), relative).unwrap();
        ignored.require_local_or_ignored_untracked().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn transaction_rejects_a_workspace_identity_change_before_replace() {
        let parent = tempfile::tempdir().unwrap();
        let workspace = parent.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(workspace.join(".codex")).unwrap();
        let target =
            ValidatedWorkspaceTarget::new(&workspace, Path::new(".codex/hooks.json")).unwrap();
        let transaction = ConfigFileTransaction::open(target).unwrap();
        fs::rename(&workspace, parent.path().join("replaced-workspace")).unwrap();
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(workspace.join(".codex")).unwrap();

        assert_eq!(
            transaction.commit(b"replacement").unwrap_err().code(),
            "workspace_changed"
        );
    }

    #[cfg(windows)]
    #[test]
    fn target_validation_rejects_windows_directory_reparse_escape_when_available() {
        use std::os::windows::fs::symlink_dir;

        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = workspace.path().join(".codex");
        if let Err(error) = symlink_dir(outside.path(), &link) {
            // Developer mode or junction permissions may be unavailable on a
            // local Windows host. The reparse-specific code still compiles in
            // Windows CI, and enabled hosts exercise the real rejection.
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                return;
            }
            panic!("failed to create Windows reparse-point fixture: {error}");
        }
        let error = ValidatedWorkspaceTarget::new(workspace.path(), Path::new(".codex/hooks.json"))
            .unwrap_err();
        assert_eq!(error.code(), "workspace_escape");
    }
}
// Shared safety boundaries for workspace-scoped harness integrations.
// Tool handlers may only mutate a [`ValidatedWorkspaceTarget`] through a
// [`ConfigFileTransaction`]. The final revision check narrows, but cannot
// eliminate the cross-process race between checking a path and replacing it.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::definition::IntegrationBinding;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IntegrationRegistration {
    Unsupported,
    Absent,
    Installed,
    Drifted,
    Error,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IntegrationOwnership {
    None,
    OrkWorks,
    Ambiguous,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IntegrationActivation {
    Active,
    NeedsTrust,
    Disabled,
    Unknown,
    NotApplicable,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IntegrationCoverage {
    Full,
    Limited,
    None,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationStatus {
    pub harness_id: String,
    pub enabled: bool,
    pub tool_detected: bool,
    pub registration: IntegrationRegistration,
    pub ownership: IntegrationOwnership,
    pub activation: IntegrationActivation,
    pub coverage: IntegrationCoverage,
    pub diagnostics: Vec<IntegrationDiagnostic>,
    pub confirmation: Option<IntegrationConfirmation>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationDiagnostic {
    pub code: String,
    pub message: String,
    pub action: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationConfirmation {
    pub tool_name: String,
    pub workspace_label: String,
    pub coverage_summary: String,
    pub relative_paths: Vec<String>,
    pub executable_code_warning: bool,
}

impl IntegrationConfirmation {
    /// Builds renderer-safe confirmation content from code-owned labels and
    /// workspace-relative paths. Absolute paths and handler identifiers are
    /// deliberately excluded from the confirmation surface.
    pub(crate) fn new(
        tool_name: &str,
        workspace: &Path,
        coverage_summary: &str,
        relative_paths: &[&Path],
        executable_code_warning: bool,
    ) -> Result<Self, IntegrationError> {
        let mut paths = Vec::with_capacity(relative_paths.len());
        for relative in relative_paths {
            validate_relative_path(relative)?;
            paths.push(relative.to_string_lossy().into_owned());
        }
        Ok(Self {
            tool_name: sanitized_label(tool_name, "Coding tool"),
            workspace_label: workspace
                .file_name()
                .and_then(OsStr::to_str)
                .map(|label| sanitized_label(label, "Workspace"))
                .unwrap_or_else(|| "Workspace".into()),
            coverage_summary: sanitized_label(coverage_summary, "Integration change"),
            relative_paths: paths,
            executable_code_warning,
        })
    }
}

pub(crate) trait IntegrationHandler: Send + Sync {
    fn status(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError>;
    fn install(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError>;
    fn uninstall(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationStatus, IntegrationError>;
}

pub(crate) fn handler(binding: &IntegrationBinding) -> &'static dyn IntegrationHandler {
    super::integrations::handler(binding)
}

pub(crate) struct IntegrationContext<'a> {
    pub workspace: &'a Path,
    pub orkworks_root: &'a Path,
    pub enabled: bool,
    pub detected_tool: Option<&'a DetectedTool>,
    pub reporter_assets: &'a ReporterAssetResolver,
}

pub(crate) struct DetectedTool {
    pub executable: PathBuf,
    pub version: Option<String>,
    pub compatible: bool,
}

pub(crate) struct ReporterAssetResolver {
    pub source_dir: PathBuf,
    pub stable_dir: PathBuf,
}

impl ReporterAssetResolver {
    /// Copies a code-owned reporter asset to the stable OrkWorks directory.
    ///
    /// Asset names must be a single safe file name; tool handlers cannot use
    /// this resolver to choose arbitrary source or destination paths.
    pub(crate) fn reconcile(&self, asset_name: &str) -> Result<PathBuf, IntegrationError> {
        if !is_safe_asset_name(asset_name) {
            return Err(IntegrationError::UnsafeTarget {
                code: "invalid_asset_name",
                message: "Reporter asset name must be a single relative file name.".into(),
            });
        }
        let source = self.source_dir.join(asset_name);
        let bytes = fs::read(&source)?;
        fs::create_dir_all(&self.stable_dir)?;
        let destination = self.stable_dir.join(asset_name);
        if fs::read(&destination).ok().as_deref() != Some(bytes.as_slice()) {
            write_new_file_atomically(&destination, &bytes)?;
        }
        Ok(destination)
    }
}

#[derive(Debug)]
pub(crate) enum IntegrationError {
    NoWorkspace,
    UnsafeTarget { code: &'static str, message: String },
    InvalidConfig(String),
    OwnershipAmbiguous,
    RevisionChanged,
    Io(std::io::Error),
}

impl IntegrationError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::NoWorkspace => "no_workspace",
            Self::UnsafeTarget { code, .. } => code,
            Self::InvalidConfig(_) => "invalid_config",
            Self::OwnershipAmbiguous => "ownership_ambiguous",
            Self::RevisionChanged => "revision_changed",
            Self::Io(_) => "io_error",
        }
    }
}

impl From<std::io::Error> for IntegrationError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Debug)]
struct WorkspaceIdentity {
    canonical_root: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl WorkspaceIdentity {
    fn capture(workspace: &Path) -> Result<Self, IntegrationError> {
        let canonical_root = fs::canonicalize(workspace).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                IntegrationError::NoWorkspace
            } else {
                IntegrationError::Io(error)
            }
        })?;
        let metadata = fs::metadata(&canonical_root)?;
        if !metadata.is_dir() {
            return Err(IntegrationError::NoWorkspace);
        }
        Ok(Self {
            canonical_root,
            #[cfg(unix)]
            device: std::os::unix::fs::MetadataExt::dev(&metadata),
            #[cfg(unix)]
            inode: std::os::unix::fs::MetadataExt::ino(&metadata),
        })
    }

    fn still_matches(&self, lexical_root: &Path) -> Result<(), IntegrationError> {
        let current = Self::capture(lexical_root)?;
        if current.canonical_root != self.canonical_root || {
            #[cfg(unix)]
            {
                current.device != self.device || current.inode != self.inode
            }
            #[cfg(not(unix))]
            {
                false
            }
        } {
            return Err(IntegrationError::UnsafeTarget {
                code: "workspace_changed",
                message:
                    "Workspace identity changed before integration configuration could be replaced."
                        .into(),
            });
        }
        Ok(())
    }
}

/// A workspace-relative path which was canonically confined at validation time.
#[derive(Clone, Debug)]
pub(crate) struct ValidatedWorkspaceTarget {
    lexical_workspace: PathBuf,
    identity: WorkspaceIdentity,
    relative: PathBuf,
    target: PathBuf,
}

impl ValidatedWorkspaceTarget {
    pub(crate) fn new(workspace: &Path, relative: &Path) -> Result<Self, IntegrationError> {
        validate_relative_path(relative)?;
        let identity = WorkspaceIdentity::capture(workspace)?;
        let target = identity.canonical_root.join(relative);
        ensure_existing_ancestor_is_confined(&identity.canonical_root, &target)?;
        Ok(Self {
            lexical_workspace: workspace.to_path_buf(),
            identity,
            relative: relative.to_path_buf(),
            target,
        })
    }

    pub(crate) fn require_local_or_ignored_untracked(&self) -> Result<(), IntegrationError> {
        let repository = git2::Repository::discover(&self.identity.canonical_root).map_err(|_| {
            IntegrationError::UnsafeTarget {
                code: "not_git_workspace",
                message: "Workspace integration files require a Git workspace so tracked files are never edited.".into(),
            }
        })?;
        let workdir = repository
            .workdir()
            .ok_or_else(|| IntegrationError::UnsafeTarget {
                code: "not_git_workspace",
                message: "Bare repositories cannot contain workspace integration files.".into(),
            })?;
        if fs::canonicalize(workdir)? != self.identity.canonical_root {
            return Err(IntegrationError::UnsafeTarget {
                code: "workspace_repository_mismatch",
                message: "Workspace root does not match the Git worktree root.".into(),
            });
        }
        let index = repository
            .index()
            .map_err(|error| IntegrationError::InvalidConfig(error.message().into()))?;
        if index.get_path(&self.relative, 0).is_some() {
            return Err(IntegrationError::UnsafeTarget {
                code: "tracked_target",
                message: "Integration configuration is tracked by Git and will not be edited automatically.".into(),
            });
        }
        if !repository
            .status_should_ignore(&self.relative)
            .map_err(|error| IntegrationError::InvalidConfig(error.message().into()))?
        {
            return Err(IntegrationError::UnsafeTarget {
                code: "not_ignored_target",
                message: "Integration configuration is not ignored by Git and will not be edited automatically.".into(),
            });
        }
        Ok(())
    }

    pub(crate) fn relative_path(&self) -> &Path {
        &self.relative
    }

    fn revalidate(&self) -> Result<(), IntegrationError> {
        self.identity.still_matches(&self.lexical_workspace)?;
        ensure_existing_ancestor_is_confined(&self.identity.canonical_root, &self.target)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileRevision {
    hash: Option<[u8; 32]>,
    len: Option<u64>,
    modified: Option<SystemTime>,
}

impl FileRevision {
    fn read(path: &Path) -> Result<(Self, Vec<u8>), IntegrationError> {
        match fs::read(path) {
            Ok(bytes) => {
                let metadata = fs::metadata(path)?;
                Ok((
                    Self {
                        hash: Some(Sha256::digest(&bytes).into()),
                        len: Some(metadata.len()),
                        modified: metadata.modified().ok(),
                    },
                    bytes,
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((
                Self {
                    hash: None,
                    len: None,
                    modified: None,
                },
                Vec::new(),
            )),
            Err(error) => Err(error.into()),
        }
    }
}

/// A write transaction which only publishes after final containment and
/// optimistic revision checks. It does not provide portable CAS semantics
/// against another process that changes the file after that final check.
pub(crate) struct ConfigFileTransaction {
    target: ValidatedWorkspaceTarget,
    original: FileRevision,
    current_bytes: Vec<u8>,
    retain_backup: bool,
    replace: fn(&Path, &Path) -> std::io::Result<()>,
    #[cfg(test)]
    before_replace: Option<fn(&Path)>,
}

impl ConfigFileTransaction {
    pub(crate) fn open(target: ValidatedWorkspaceTarget) -> Result<Self, IntegrationError> {
        target.revalidate()?;
        let (original, current_bytes) = FileRevision::read(&target.target)?;
        Ok(Self {
            target,
            original,
            current_bytes,
            retain_backup: false,
            replace: atomic_replace,
            #[cfg(test)]
            before_replace: None,
        })
    }

    pub(crate) fn current_bytes(&self) -> &[u8] {
        &self.current_bytes
    }

    /// Enables a same-directory recovery copy for an eligible local-only file.
    pub(crate) fn retain_recoverable_backup(mut self) -> Self {
        self.retain_backup = true;
        self
    }

    #[cfg(test)]
    fn with_before_replace(mut self, callback: fn(&Path)) -> Self {
        self.before_replace = Some(callback);
        self
    }

    #[cfg(test)]
    fn with_replace(mut self, replace: fn(&Path, &Path) -> std::io::Result<()>) -> Self {
        self.replace = replace;
        self
    }

    pub(crate) fn commit(self, replacement: &[u8]) -> Result<(), IntegrationError> {
        self.target.revalidate()?;
        let parent = self
            .target
            .target
            .parent()
            .ok_or_else(|| IntegrationError::UnsafeTarget {
                code: "invalid_target",
                message: "Integration target has no parent directory.".into(),
            })?;
        fs::create_dir_all(parent)?;
        self.target.revalidate()?;
        let temporary = temporary_path(&self.target.target);
        let result = (|| -> Result<(), IntegrationError> {
            let mut file = File::options()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(replacement)?;
            file.flush()?;
            file.sync_all()?;
            drop(file);

            #[cfg(test)]
            if let Some(callback) = self.before_replace {
                callback(&self.target.target);
            }
            self.target.revalidate()?;
            let (current, _) = FileRevision::read(&self.target.target)?;
            if current != self.original {
                return Err(IntegrationError::RevisionChanged);
            }
            if self.retain_backup && self.original.hash.is_some() {
                write_new_file_atomically(&backup_path(&self.target.target), &self.current_bytes)?;
            }
            (self.replace)(&temporary, &self.target.target)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

pub(crate) fn atomic_replace(source: &Path, target: &Path) -> std::io::Result<()> {
    #[cfg(not(windows))]
    {
        fs::rename(source, target)
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };

        let source = wide_path(source.as_os_str());
        let target = wide_path(target.as_os_str());
        // SAFETY: both buffers are nul-terminated UTF-16 paths and remain
        // alive for the duration of the Windows API call.
        let result = unsafe {
            MoveFileExW(
                source.as_ptr(),
                target.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if result == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
fn wide_path(path: &OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.encode_wide().chain(std::iter::once(0)).collect()
}

fn validate_relative_path(relative: &Path) -> Result<(), IntegrationError> {
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(IntegrationError::UnsafeTarget {
            code: "invalid_relative_path",
            message: "Integration configuration path must be a non-empty relative path without parent traversal.".into(),
        });
    }
    Ok(())
}

fn ensure_existing_ancestor_is_confined(
    root: &Path,
    target: &Path,
) -> Result<(), IntegrationError> {
    let mut ancestor = target;
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or_else(|| IntegrationError::UnsafeTarget {
                code: "workspace_escape",
                message: "Integration target has no existing ancestor inside the workspace.".into(),
            })?;
    }
    let canonical = fs::canonicalize(ancestor)?;
    if !canonical.starts_with(root) {
        return Err(IntegrationError::UnsafeTarget {
            code: "workspace_escape",
            message: "Integration target resolves outside the workspace.".into(),
        });
    }
    Ok(())
}

fn temporary_path(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .unwrap_or_else(|| OsStr::new("integration"));
    target.with_file_name(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        uuid::Uuid::new_v4()
    ))
}

fn backup_path(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .unwrap_or_else(|| OsStr::new("integration"));
    target.with_file_name(format!(".{}.orkworks-backup", name.to_string_lossy()))
}

fn is_safe_asset_name(asset_name: &str) -> bool {
    let path = Path::new(asset_name);
    path.components().count() == 1 && matches!(path.components().next(), Some(Component::Normal(_)))
}

fn sanitized_label(value: &str, fallback: &str) -> String {
    let label: String = value
        .chars()
        .filter(|character| !character.is_control() && *character != '/' && *character != '\\')
        .take(120)
        .collect();
    let label = label.trim();
    if label.is_empty() {
        fallback.into()
    } else {
        label.into()
    }
}

/// Parses an integration configuration as an object, preserving unrelated
/// keys for a handler to merge without accepting malformed document shapes.
pub(crate) fn parse_json_object(
    bytes: &[u8],
) -> Result<serde_json::Map<String, serde_json::Value>, IntegrationError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| IntegrationError::InvalidConfig(error.to_string()))?;
    value.as_object().cloned().ok_or_else(|| {
        IntegrationError::InvalidConfig("Integration configuration must be a JSON object.".into())
    })
}

/// Classifies a handler-owned JSON fragment by its exact OrkWorks marker.
/// A different OrkWorks integration marker is ambiguous and must not be
/// removed by the calling handler.
pub(crate) fn json_ownership(value: &serde_json::Value, marker: &str) -> IntegrationOwnership {
    let mut markers = Vec::new();
    collect_markers(value, &mut markers);
    if markers.contains(&marker) {
        IntegrationOwnership::OrkWorks
    } else if markers
        .iter()
        .any(|candidate| candidate.starts_with("orkworks:harness-integration:"))
    {
        IntegrationOwnership::Ambiguous
    } else {
        IntegrationOwnership::None
    }
}

fn collect_markers<'a>(value: &'a serde_json::Value, markers: &mut Vec<&'a str>) {
    match value {
        serde_json::Value::String(string) => markers.push(string),
        serde_json::Value::Array(values) => {
            for value in values {
                collect_markers(value, markers);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                collect_markers(value, markers);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}

fn write_new_file_atomically(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let temporary = temporary_path(path);
    let result = (|| -> std::io::Result<()> {
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(contents)?;
        file.flush()?;
        file.sync_all()?;
        atomic_replace(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
