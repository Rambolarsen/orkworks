use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::PathBuf;

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

    #[test]
    fn hook_observed_at_requires_utc_microsecond_precision() {
        assert!(parse_hook_observed_at("2026-07-21T08:00:00.123456Z").is_ok());
        assert!(parse_hook_observed_at("2026-07-21T08:00:00Z").is_err());
        assert!(parse_hook_observed_at("2026-07-21T08:00:00.123Z").is_err());
        assert!(parse_hook_observed_at("2026-07-21T08:00:00.123456+00:00").is_err());
    }
}
