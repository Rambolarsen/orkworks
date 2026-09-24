//! Startup cleanup for `~/.orkworks/workspaces/<hash>/` directories whose
//! source path no longer exists (e.g. a removed worktree). `workspace_hash`
//! is one-way, so a directory can only be traced back to its path via the
//! `origin.json` file `workspace_runtime::write_origin_file_if_absent`
//! writes when a workspace is first opened; directories without one predate
//! that file and are left alone rather than guessed at.

use crate::workspace_runtime::{orkworks_workspaces_root, read_origin_file, WorkspaceLease};
use std::path::Path;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct GcSummary {
    pub removed: Vec<String>,
    pub kept: usize,
    pub skipped_active: usize,
    pub skipped_no_origin: usize,
}

/// Scans `workspaces_root` once, synchronously. Kept separate from the async
/// task wrapper so it is directly unit-testable.
pub(crate) fn gc_stale_workspaces_once(workspaces_root: &Path) -> GcSummary {
    let mut summary = GcSummary::default();
    let Ok(entries) = std::fs::read_dir(workspaces_root) else {
        return summary;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(origin) = read_origin_file(&dir) else {
            summary.skipped_no_origin += 1;
            continue;
        };
        // A held lease means a sidecar currently owns this workspace (ADR
        // 0052) — never touch it, regardless of what the path check below
        // would say.
        let lease = match WorkspaceLease::acquire(&dir) {
            Ok(lease) => lease,
            Err(_) => {
                summary.skipped_active += 1;
                continue;
            }
        };
        // Only a definitive "not found" counts as stale. Any other error
        // (permission denied, a transient filesystem hiccup, ...) means the
        // question can't be answered right now, so the directory is kept —
        // deletion here is irreversible, and the cleanup scope is sources
        // that no longer exist, not sources that merely failed to stat.
        let source_definitely_gone = matches!(
            std::fs::metadata(&origin.canonical_path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound
        );

        if !source_definitely_gone {
            drop(lease);
            summary.kept += 1;
            continue;
        }
        // Hold the lease through removal itself: dropping it first would
        // reopen the window it exists to close, letting a concurrent
        // sidecar acquire the lock and start using the directory right
        // before it's deleted out from under it.
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => summary.removed.push(dir.display().to_string()),
            Err(error) => {
                tracing::warn!(
                    path = %dir.display(),
                    %error,
                    "failed to remove stale workspace metadata directory"
                );
            }
        }
        drop(lease);
    }
    summary
}

/// Runs once at sidecar startup, before any workspace is opened.
pub(crate) async fn gc_stale_workspaces_task() {
    let Some(root) = orkworks_workspaces_root() else {
        return;
    };
    let summary = tokio::task::spawn_blocking(move || gc_stale_workspaces_once(&root)).await;
    if let Ok(summary) = summary {
        if !summary.removed.is_empty() {
            tracing::info!(
                removed = summary.removed.len(),
                "cleaned up stale workspace metadata directories"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_runtime::write_origin_file_if_absent;

    fn workspace_dir(root: &Path, name: &str) -> std::path::PathBuf {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn removes_a_directory_whose_source_path_no_longer_exists() {
        let root = tempfile::tempdir().unwrap();
        let gone = workspace_dir(root.path(), "gone");
        write_origin_file_if_absent(&gone, Path::new("/definitely/does/not/exist/anywhere"));

        let summary = gc_stale_workspaces_once(root.path());

        assert_eq!(summary.removed, vec![gone.display().to_string()]);
        assert!(!gone.exists());
    }

    #[test]
    fn keeps_a_directory_whose_source_path_still_exists() {
        let root = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let live = workspace_dir(root.path(), "live");
        write_origin_file_if_absent(&live, source.path());

        let summary = gc_stale_workspaces_once(root.path());

        assert_eq!(summary.kept, 1);
        assert!(summary.removed.is_empty());
        assert!(live.exists());
    }

    #[test]
    fn leaves_a_directory_with_no_origin_file_untouched() {
        let root = tempfile::tempdir().unwrap();
        let legacy = workspace_dir(root.path(), "legacy");

        let summary = gc_stale_workspaces_once(root.path());

        assert_eq!(summary.skipped_no_origin, 1);
        assert!(summary.removed.is_empty());
        assert!(legacy.exists());
    }

    #[test]
    fn never_removes_a_directory_whose_lease_is_currently_held() {
        let root = tempfile::tempdir().unwrap();
        let active = workspace_dir(root.path(), "active");
        write_origin_file_if_absent(&active, Path::new("/definitely/does/not/exist/anywhere"));
        let held_lease = WorkspaceLease::acquire(&active).unwrap();

        let summary = gc_stale_workspaces_once(root.path());

        assert_eq!(summary.skipped_active, 1);
        assert!(summary.removed.is_empty());
        assert!(active.exists());
        drop(held_lease);
    }

    #[cfg(unix)]
    #[test]
    fn keeps_a_directory_when_the_source_check_fails_with_something_other_than_not_found() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let locked_parent = tempfile::tempdir().unwrap();
        let unreachable_child = locked_parent.path().join("child");
        std::fs::create_dir_all(&unreachable_child).unwrap();
        // Strip execute permission on the parent so stat'ing the child fails
        // with PermissionDenied, not NotFound — the child genuinely exists.
        std::fs::set_permissions(locked_parent.path(), std::fs::Permissions::from_mode(0o600))
            .unwrap();

        let ambiguous = workspace_dir(root.path(), "ambiguous");
        write_origin_file_if_absent(&ambiguous, &unreachable_child);

        let summary = gc_stale_workspaces_once(root.path());

        std::fs::set_permissions(locked_parent.path(), std::fs::Permissions::from_mode(0o700))
            .unwrap();

        assert_eq!(summary.kept, 1);
        assert!(summary.removed.is_empty());
        assert!(ambiguous.exists());
    }

    #[test]
    fn gc_on_a_missing_root_directory_is_a_harmless_noop() {
        let root = tempfile::tempdir().unwrap();
        let missing_root = root.path().join("does-not-exist");

        let summary = gc_stale_workspaces_once(&missing_root);

        assert_eq!(summary, GcSummary::default());
    }
}
