//! Startup cleanup for `~/.orkworks/workspaces/<hash>/` directories whose
//! source path no longer exists (e.g. a removed worktree). `workspace_hash`
//! is one-way, so a directory can only be traced back to its path via the
//! `origin.json` file `workspace_runtime::write_origin_file_if_absent`
//! writes when a workspace is first opened; directories without one predate
//! that file and are left alone rather than guessed at.
//!
//! `NotFound` from `std::fs::metadata` cannot by itself distinguish a deleted
//! source from an unmounted external drive, disconnected network share or
//! locked encrypted volume — and the removal below is irreversible. Removal
//! is therefore justified only when the source's immediate parent (the
//! nearest existing ancestor) is verifiably present, only the leaf is
//! missing, the lease is free, and the directory carries no
//! retention-protected session history. Every removal is logged with its
//! path and durable session count.

use crate::session_archive::dir_has_retention_protected_records;
use crate::workspace_runtime::{orkworks_workspaces_root, read_origin_file, WorkspaceLease};
use std::path::Path;

#[derive(Debug, Default, PartialEq, Eq, Clone, serde::Serialize)]
pub(crate) struct RemovedWorkspace {
    pub path: String,
    /// Number of durable session records (`sessions/` entries, files) the
    /// removed directory held at the moment of deletion, so any loss is
    /// diagnosable.
    pub session_count: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct GcSummary {
    pub removed: Vec<RemovedWorkspace>,
    pub kept: usize,
    pub skipped_active: usize,
    pub skipped_no_origin: usize,
    /// The source reported `NotFound`, but an ancestor needed to verify the
    /// source's hierarchy is missing or not itself a directory: an unmounted
    /// external drive, disconnected network share or locked encrypted volume
    /// looks exactly like a deleted source to `metadata`, so without a
    /// verifiably present immediate parent directory this directory is kept
    /// rather than irreversibly removed.
    pub skipped_unavailable_ancestor: usize,
    /// The directory holds a durable organization record that the
    /// retention-protection classes preserve (archived/active/unreadable),
    /// so its history survives even a verifiably deleted source.
    pub skipped_protected: usize,
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
        // 0052) — never touch it, regardless of what the path checks below
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
        let source_leaves = matches!(
            std::fs::metadata(&origin.canonical_path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound
        );
        if !source_leaves {
            drop(lease);
            summary.kept += 1;
            continue;
        }
        // `NotFound` on the source is also what an unmounted external drive,
        // disconnected network share or locked encrypted volume reports: the
        // medium is unavailable, not the source deleted — and the source may
        // reappear with its media. A deleted source leaves its containing
        // hierarchy intact, so only when the source's immediate parent is
        // verifiably present (the nearest existing ancestor) is the source
        // actually gone. Residual limit: an unavailable network mount whose
        // persistent mount-point directory survives unmounting still
        // satisfies the parent check; a mount-table probe would be needed to
        // close that case and is not adopted here.
        let parent_present = origin.canonical_path.parent().is_some_and(|parent| {
            std::fs::metadata(parent)
                .map(|m| m.is_dir())
                .unwrap_or(false)
        });
        if !parent_present {
            drop(lease);
            summary.skipped_unavailable_ancestor += 1;
            continue;
        }
        // Retention-protected durable history (archived/active/unreadable
        // organization records) survives a verifiably deleted source, the
        // same classes the interactive retention loop protects.
        if dir_has_retention_protected_records(&dir.join("session-state")) {
            drop(lease);
            summary.skipped_protected += 1;
            continue;
        }
        // Hold the lease through removal itself: dropping it first would
        // reopen the window it exists to close, letting a concurrent
        // sidecar acquire the lock and start using the directory right
        // before it's deleted out from under it.
        let session_count = count_durable_session_records(&dir);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {
                let removed = RemovedWorkspace {
                    path: dir.display().to_string(),
                    session_count,
                };
                tracing::info!(
                    path = %removed.path,
                    sessions = removed.session_count,
                    "removed stale workspace metadata directory"
                );
                summary.removed.push(removed);
            }
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

/// Counts the durable session records (`sessions/` entries, files) a
/// workspace directory currently holds, before removal, so the per-directory
/// log can state how much history was deleted. A missing directory counts as
/// zero.
fn count_durable_session_records(workspace_dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(workspace_dir.join("sessions")) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .count()
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
                skipped_unavailable_ancestor = summary.skipped_unavailable_ancestor,
                skipped_protected = summary.skipped_protected,
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
    fn removes_a_deleted_source_whose_parent_directory_still_exists() {
        let root = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let gone = workspace_dir(root.path(), "gone");
        // The source itself was removed; its immediate parent directory is
        // verifiably present, so removal is justified.
        let deleted = source.path().join("removed-child");
        assert!(!deleted.exists());
        write_origin_file_if_absent(&gone, &deleted);
        std::fs::create_dir(gone.join("sessions")).unwrap();
        std::fs::write(gone.join("sessions/one.json"), "{}").unwrap();
        std::fs::write(gone.join("sessions/two.json"), "{}").unwrap();

        let summary = gc_stale_workspaces_once(root.path());

        assert_eq!(
            summary.removed,
            vec![RemovedWorkspace {
                path: gone.display().to_string(),
                session_count: 2,
            }]
        );
        assert!(!gone.exists());
    }

    #[test]
    fn keeps_a_directory_whose_source_parent_is_also_absent() {
        let root = tempfile::tempdir().unwrap();
        let unmounted = workspace_dir(root.path(), "unmounted");
        // An unmounted external drive, disconnected network share or locked
        // encrypted volume returns NotFound for the source *and* for an
        // ancestor of it. Without a verifiably present parent, removal
        // cannot be justified — the source may reappear with its media.
        // Fixture: record a path under a directory that is then removed, so
        // the test is independent of any host mount layout.
        let mount = tempfile::tempdir().unwrap();
        let source = mount.path().join("repo");
        write_origin_file_if_absent(&unmounted, &source);
        drop(mount);

        let summary = gc_stale_workspaces_once(root.path());

        assert!(summary.removed.is_empty());
        assert_eq!(summary.skipped_unavailable_ancestor, 1);
        assert!(unmounted.exists());
    }

    #[test]
    fn keeps_a_directory_containing_an_archived_session_record() {
        let root = tempfile::tempdir().unwrap();
        let kept = workspace_dir(root.path(), "archived");
        let source = tempfile::tempdir().unwrap();
        let deleted = source.path().join("removed-child");
        write_origin_file_if_absent(&kept, &deleted);
        let state = kept.join("session-state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            state.join("one.json"),
            r#"{"version":1,"sessionId":"one","createdAt":"2026-01-01T00:00:00.000000Z","state":"archived","revision":1,"changedAt":"2026-01-02T00:00:00.000000Z"}"#,
        )
        .unwrap();

        let summary = gc_stale_workspaces_once(root.path());

        assert!(summary.removed.is_empty());
        assert_eq!(summary.skipped_protected, 1);
        assert!(kept.exists());
    }

    #[test]
    fn keeps_a_directory_with_an_unreadable_session_record() {
        let root = tempfile::tempdir().unwrap();
        let kept = workspace_dir(root.path(), "unreadable");
        let source = tempfile::tempdir().unwrap();
        let deleted = source.path().join("removed-child");
        write_origin_file_if_absent(&kept, &deleted);
        let state = kept.join("session-state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(state.join("corrupt.json"), "{not json").unwrap();

        let summary = gc_stale_workspaces_once(root.path());

        assert!(summary.removed.is_empty());
        assert_eq!(summary.skipped_protected, 1);
        assert!(kept.exists());
    }

    #[test]
    fn keeps_a_directory_with_a_persisted_active_or_oversized_session_record() {
        let root = tempfile::tempdir().unwrap();
        let kept = workspace_dir(root.path(), "invalid-records");
        let source = tempfile::tempdir().unwrap();
        let deleted = source.path().join("removed-child");
        write_origin_file_if_absent(&kept, &deleted);
        let state = kept.join("session-state");
        std::fs::create_dir_all(&state).unwrap();
        // The store never persists `active`, and an oversized record
        // violates the store's bounded-read rule: this well-formed `closed`
        // record's over-long `changedAt` isolates the size guard, and the
        // `active` record isolates the state rule. Both classes protect
        // (like the owning read path reporting an error), so GC keeps
        // the directory.
        std::fs::write(
            state.join("one.json"),
            r#"{"version":1,"sessionId":"one","createdAt":"2026-01-01T00:00:00.000000Z","state":"active","revision":1,"changedAt":"2026-01-02T00:00:00.000000Z"}"#,
        )
        .unwrap();
        std::fs::write(
            state.join("two.json"),
            format!(
                r#"{{"version":1,"sessionId":"two","createdAt":"2026-01-01T00:00:00.000000Z","state":"closed","revision":1,"changedAt":"{pad}"}}"#,
                pad = "2".repeat(4100),
            ),
        )
        .unwrap();

        let summary = gc_stale_workspaces_once(root.path());

        assert!(summary.removed.is_empty());
        assert_eq!(summary.skipped_protected, 1);
        assert!(kept.exists());
    }

    #[test]
    fn logs_no_session_count_beyond_zero_when_no_sessions_directory_exists() {
        let root = tempfile::tempdir().unwrap();
        let stale = workspace_dir(root.path(), "no-sessions");
        let source = tempfile::tempdir().unwrap();
        let deleted = source.path().join("removed-child");
        write_origin_file_if_absent(&stale, &deleted);

        let summary = gc_stale_workspaces_once(root.path());

        assert_eq!(
            summary.removed,
            vec![RemovedWorkspace {
                path: stale.display().to_string(),
                session_count: 0,
            }]
        );
        assert!(!stale.exists());
    }

    #[test]
    fn still_removes_closed_only_history_with_a_verified_deleted_source() {
        let root = tempfile::tempdir().unwrap();
        let stale = workspace_dir(root.path(), "closed-only");
        let source = tempfile::tempdir().unwrap();
        let deleted = source.path().join("removed-child");
        write_origin_file_if_absent(&stale, &deleted);
        let state = stale.join("session-state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            state.join("one.json"),
            r#"{"version":1,"sessionId":"one","createdAt":"2026-01-01T00:00:00.000000Z","state":"closed","revision":1,"changedAt":"2026-01-02T00:00:00.000000Z"}"#,
        )
        .unwrap();

        let summary = gc_stale_workspaces_once(root.path());

        assert_eq!(summary.removed.len(), 1);
        assert!(!stale.exists());
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
        let source = tempfile::tempdir().unwrap();
        let deleted = source.path().join("removed-child");
        assert!(!deleted.exists());
        write_origin_file_if_absent(&active, &deleted);
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
