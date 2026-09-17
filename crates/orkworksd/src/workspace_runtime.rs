use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectoryIdentity {
    #[cfg(unix)]
    Unix { device: u64, inode: u64 },
    #[cfg(windows)]
    Windows { volume: u32, file_index: u64 },
    #[cfg(not(any(unix, windows)))]
    Unsupported,
}

/// Canonical path plus native directory identity captured for one open
/// attempt. Keeping the handle open lets the caller revalidate the same
/// directory object after an Electron-side path check.
#[derive(Debug)]
pub(crate) struct WorkspaceIdentity {
    canonical_path: PathBuf,
    directory_identity: DirectoryIdentity,
    _directory: File,
}

impl WorkspaceIdentity {
    pub(crate) fn resolve(path: &Path) -> io::Result<Self> {
        let canonical_path = std::fs::canonicalize(path)?;
        let directory = open_directory(&canonical_path)?;
        let metadata = directory.metadata()?;
        if !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "workspace identity is not a directory",
            ));
        }
        Ok(Self {
            canonical_path,
            directory_identity: native_directory_identity(&metadata),
            _directory: directory,
        })
    }

    pub(crate) fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    pub(crate) fn matches(&self, path: &Path) -> bool {
        Self::resolve(path).is_ok_and(|candidate| candidate == *self)
    }
}

impl PartialEq for WorkspaceIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.canonical_path == other.canonical_path
            && self.directory_identity == other.directory_identity
    }
}

impl Eq for WorkspaceIdentity {}

#[cfg(windows)]
fn open_directory(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;

    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(not(windows))]
fn open_directory(path: &Path) -> io::Result<File> {
    File::open(path)
}

fn native_directory_identity(metadata: &std::fs::Metadata) -> DirectoryIdentity {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return DirectoryIdentity::Unix {
            device: metadata.dev(),
            inode: metadata.ino(),
        };
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        return DirectoryIdentity::Windows {
            volume: metadata.volume_serial_number().unwrap_or_default(),
            file_index: metadata.file_index().unwrap_or_default(),
        };
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = metadata;
        DirectoryIdentity::Unsupported
    }
}

/// Exclusive OS-level ownership of one workspace's metadata directory.
///
/// The lock file is intentionally retained on disk. File existence is not the
/// ownership signal; the advisory lock held by this open file is. That means a
/// crashed sidecar cannot leave a stale PID marker blocking the next owner.
pub(crate) struct WorkspaceLease {
    file: File,
}

impl WorkspaceLease {
    pub(crate) fn acquire(global_dir: &std::path::Path) -> io::Result<Self> {
        std::fs::create_dir_all(global_dir)?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(global_dir.join(".sidecar.lock"))?;
        file.try_lock_exclusive()?;
        Ok(Self { file })
    }
}

impl Drop for WorkspaceLease {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

pub(crate) fn iso_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub(crate) fn parse_hook_observed_at(raw: &str) -> Result<chrono::DateTime<chrono::Utc>, ()> {
    let Some((_, fraction_and_z)) = raw.rsplit_once('.') else {
        return Err(());
    };
    let Some(fraction) = fraction_and_z.strip_suffix('Z') else {
        return Err(());
    };
    if fraction.len() != 6 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(());
    }
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
        .map_err(|_| ())
}

pub(crate) fn workspace_hash(path: &std::path::Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

pub(crate) fn orkworks_global_dir(workspace_path: &std::path::Path) -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join(".orkworks")
            .join("workspaces")
            .join(workspace_hash(workspace_path))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_lease_is_exclusive_and_reusable() {
        let dir = tempfile::tempdir().unwrap();
        let first = WorkspaceLease::acquire(dir.path()).unwrap();
        assert!(WorkspaceLease::acquire(dir.path()).is_err());

        drop(first);
        assert!(WorkspaceLease::acquire(dir.path()).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn equivalent_directory_aliases_share_one_identity() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let alias = root.path().join("alias");
        symlink(root.path(), &alias).unwrap();

        let direct = WorkspaceIdentity::resolve(root.path()).unwrap();
        let through_alias = WorkspaceIdentity::resolve(&alias).unwrap();

        assert_eq!(direct, through_alias);
        assert_eq!(workspace_hash(root.path()), workspace_hash(&alias));
    }

    #[test]
    fn distinct_git_worktrees_have_distinct_identities() {
        let root = tempfile::tempdir().unwrap();
        let repository = git2::Repository::init(root.path()).unwrap();
        std::fs::write(root.path().join("README"), "workspace").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("README")).unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = git2::Signature::now("OrkWorks test", "test@example.invalid").unwrap();
        repository
            .commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
            .unwrap();

        let worktree = root.path().join("linked");
        let _worktree = repository.worktree("linked", &worktree, None).unwrap();

        let first = WorkspaceIdentity::resolve(root.path()).unwrap();
        let second = WorkspaceIdentity::resolve(&worktree).unwrap();

        assert_ne!(first, second);
        assert_ne!(workspace_hash(root.path()), workspace_hash(&worktree));
    }

    #[cfg(unix)]
    #[test]
    fn a_directory_replacement_invalidates_the_retained_identity() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let alias = root.path().join("workspace");
        symlink(first.path(), &alias).unwrap();

        let identity = WorkspaceIdentity::resolve(&alias).unwrap();
        std::fs::remove_file(&alias).unwrap();
        symlink(second.path(), &alias).unwrap();

        assert!(!identity.matches(&alias));
    }

    #[test]
    fn hook_observed_at_requires_utc_microsecond_precision() {
        assert!(parse_hook_observed_at("2026-07-21T08:00:00.123456Z").is_ok());
        assert!(parse_hook_observed_at("2026-07-21T08:00:00Z").is_err());
        assert!(parse_hook_observed_at("2026-07-21T08:00:00.123Z").is_err());
        assert!(parse_hook_observed_at("2026-07-21T08:00:00.123456+00:00").is_err());
    }
}
