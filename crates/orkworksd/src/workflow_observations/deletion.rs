//! Durable deletion through the original workflow store and revocation owner.

use super::*;

#[cfg(unix)]
pub(super) struct Anchor {
    root: std::fs::File,
    directory: std::fs::File,
}
#[cfg(unix)]
impl Anchor {
    pub(super) fn checked_directory(
        &self,
        lease: &crate::workspace_runtime::WorkspaceLease,
    ) -> std::io::Result<std::fs::File> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::MetadataExt,
        };
        lease.validate_directory(&self.root)?;
        let name = std::ffi::CString::new("workflow-observations").unwrap();
        // SAFETY: retained root FD, fixed NUL-terminated component, newly owned result.
        let fd = unsafe {
            libc::openat(
                self.root.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: successful openat returned a fresh descriptor owned exactly once.
        let current = unsafe { std::fs::File::from_raw_fd(fd) };
        let original = self.directory.metadata()?;
        let selected = current.metadata()?;
        if original.dev() != selected.dev() || original.ino() != selected.ino() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "original workflow directory replaced",
            ));
        }
        self.directory.try_clone()
    }
    // Retain original directories for the shared store lifetime, even if their
    // paths are renamed/replaced. An error disables only this new durable path.
    pub(super) fn open(root: &std::path::Path) -> std::io::Result<Self> {
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::fs::OpenOptionsExt;
        let root = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root)?;
        let name = std::ffi::CString::new("workflow-observations").unwrap();
        // SAFETY: live directory FD and fixed NUL-terminated basename.
        let fd = unsafe {
            libc::openat(
                root.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: openat returned a new owned FD; File closes it exactly once.
        Ok(Self {
            root,
            directory: unsafe { std::fs::File::from_raw_fd(fd) },
        })
    }
}

impl WorkflowObservationStore {
    /// Blocking-only; revoke under the shared mutex and sync before forgetting.
    #[cfg(unix)]
    pub(crate) fn delete_session_observations_durable(&self, id: &str) -> std::io::Result<()> {
        self.delete_session_observations_with(id, std::fs::File::sync_all)
    }
    #[cfg(not(unix))]
    pub(crate) fn delete_session_observations_durable(&self, _id: &str) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "durable workflow cleanup is not qualified on this platform",
        ))
    }
    #[cfg(unix)]
    fn delete_session_observations_with(
        &self,
        id: &str,
        sync: impl Fn(&std::fs::File) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        use std::os::fd::AsRawFd;
        let _operation = self.owner.legacy()?;
        if id.is_empty()
            || id.len() > 256
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid workflow cleanup identity",
            ));
        }
        let mut inner = self.inner.lock().unwrap();
        revoke_session(&mut inner, id);
        let anchor = self
            .cleanup_anchor
            .as_ref()
            .as_ref()
            .map_err(|error| std::io::Error::new(error.kind(), error.to_string()))?;
        let dir = &anchor.directory;
        // SAFETY: live directory FD and valid component-limit query.
        let limit = unsafe { libc::fpathconf(dir.as_raw_fd(), libc::_PC_NAME_MAX) };
        let limit = usize::try_from(limit)
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "workflow directory component limit unavailable",
                )
            })?;
        let name = std::ffi::CString::new(format!("{id}.ndjson")).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid workflow basename",
            )
        })?;
        // Never resolve an impossible exact basename or a truncated alias.
        if name.as_bytes().len() <= limit {
            // SAFETY: live directory FD, validated single basename, flags0
            // removes only the file/link rather than following a link/tree.
            if unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), 0) } < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() != std::io::ErrorKind::NotFound {
                    return Err(error);
                }
            }
        }
        // Retries repeat sync even if a previous unlink already succeeded.
        sync(dir)?;
        sync(&anchor.root)?;
        forget_session(&mut inner, id);
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::*;
    fn seeded() -> (tempfile::TempDir, WorkflowObservationStore, CapturedWriter) {
        let root = tempfile::tempdir().unwrap();
        let store = WorkflowObservationStore::open(root.path().into()).unwrap();
        let writer = store.capture_writer("s".into(), "created".into());
        store
            .record_observation(
                "s",
                ObservationOrigin::Agent,
                "key",
                ObservationCandidate {
                    kind: ObservationKind::Obstacle,
                    description: "blocked".into(),
                    evidence: "failure".into(),
                    problem_area: None,
                    reported_impact: Impact::Low,
                    confidence: None,
                },
            )
            .unwrap();
        (root, store, writer)
    }
    #[test]
    fn durable_delete_retries_sync_after_unlink_and_retains_cache_until_success() {
        for fail_at in [1, 2] {
            let (_root, store, writer) = seeded();
            let calls = std::cell::Cell::new(0);
            assert!(store
                .delete_session_observations_with("s", |file| {
                    calls.set(calls.get() + 1);
                    if calls.get() == fail_at {
                        Err(std::io::Error::other("injected sync failure"))
                    } else {
                        file.sync_all()
                    }
                })
                .is_err());
            assert!(!store.segment_path("s").exists());
            assert_eq!(store.session_observation_count("s").unwrap(), 1);
            assert!(!writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok());
            calls.set(0);
            store
                .delete_session_observations_with("s", |file| {
                    calls.set(calls.get() + 1);
                    file.sync_all()
                })
                .unwrap();
            assert_eq!(calls.get(), 2);
            assert_eq!(store.session_observation_count("s").unwrap(), 0);
            let inner = store.inner.lock().unwrap();
            assert!(inner.idempotency.is_empty());
            assert_eq!(inner.last_issued_sequence, 1);
        }
    }
    #[test]
    fn durable_delete_failed_unlink_revokes_permits_without_clearing_cache() {
        let (_root, store, writer) = seeded();
        fs::remove_file(store.segment_path("s")).unwrap();
        fs::create_dir(store.segment_path("s")).unwrap();
        assert!(store.delete_session_observations_durable("s").is_err());
        assert_eq!(store.session_observation_count("s").unwrap(), 1);
        assert!(!writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok());
        fs::remove_dir(store.segment_path("s")).unwrap();
        store.delete_session_observations_durable("s").unwrap();
        assert_eq!(store.session_observation_count("s").unwrap(), 0);
    }
    #[test]
    fn durable_delete_rejects_invalid_identity_before_revocation_or_path_resolution() {
        let (_root, store, writer) = seeded();
        for id in ["", "../s", "s/other", "s\0x", &"x".repeat(257)] {
            assert_eq!(
                store
                    .delete_session_observations_durable(id)
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::InvalidInput
            );
        }
        assert!(writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok());
        assert_eq!(store.session_observation_count("s").unwrap(), 1);
    }
    #[test]
    fn legacy_directory_link_refuses_and_original_root_anchor_stays_nonfollowing() {
        for root_link in [true, false] {
            let outer = tempfile::tempdir().unwrap();
            let actual = outer.path().join("actual");
            fs::create_dir(&actual).unwrap();
            let root = outer.path().join("workspace");
            if root_link {
                std::os::unix::fs::symlink(&actual, &root).unwrap();
            } else {
                fs::create_dir(&root).unwrap();
                std::os::unix::fs::symlink(&actual, root.join("workflow-observations")).unwrap();
            }
            if !root_link {
                // The accepted SQL-tag refusal floor cannot inspect through a
                // workflow-directory link. It refuses before any legacy effect.
                fs::write(actual.join("s.ndjson"), b"keep").unwrap();
                assert!(WorkflowObservationStore::open(root).is_err());
                assert_eq!(fs::read(actual.join("s.ndjson")).unwrap(), b"keep");
                continue;
            }
            let store = WorkflowObservationStore::open(root).unwrap();
            fs::write(store.segment_path("s"), b"keep").unwrap();
            let writer = store.capture_writer("s".into(), "creation".into());
            assert!(store.delete_session_observations_durable("s").is_err());
            assert_eq!(fs::read(store.segment_path("s")).unwrap(), b"keep");
            assert!(writer.record_batch(&FileClaim::new(), 1, vec![]).is_err());
            // No new durable fallback was introduced; legacy's explicit API is unchanged.
            store.delete_session_observations("s").unwrap();
            assert!(!store.segment_path("s").exists());
        }
    }
    #[test]
    fn durable_delete_never_targets_replacement_workflow_directory() {
        let (root, store, _writer) = seeded();
        let original = root.path().join("original-workflow");
        fs::rename(store.dir.as_ref(), &original).unwrap();
        fs::create_dir(store.dir.as_ref()).unwrap();
        fs::write(store.segment_path("s"), b"replacement").unwrap();
        store.delete_session_observations_durable("s").unwrap();
        assert_eq!(fs::read(store.segment_path("s")).unwrap(), b"replacement");
        assert!(!original.join("s.ndjson").exists());
    }
    #[test]
    fn durable_delete_never_targets_replacement_workspace_root() {
        let outer = tempfile::tempdir().unwrap();
        let root = outer.path().join("workspace");
        let store = WorkflowObservationStore::open(root.clone()).unwrap();
        fs::write(store.segment_path("s"), b"original").unwrap();
        let original = outer.path().join("original-workspace");
        fs::rename(&root, &original).unwrap();
        fs::create_dir_all(root.join("workflow-observations")).unwrap();
        fs::write(store.segment_path("s"), b"replacement").unwrap();
        store.delete_session_observations_durable("s").unwrap();
        assert_eq!(fs::read(store.segment_path("s")).unwrap(), b"replacement");
        assert!(!original.join("workflow-observations/s.ndjson").exists());
    }
    #[test]
    fn durable_delete_handles_maximum_ids_and_absent_directory_without_aliasing() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkflowObservationStore::open(root.path().into()).unwrap();
        for length in [248, 256] {
            let id = "x".repeat(length);
            let alias = store.segment_path(&"x".repeat(240));
            fs::write(&alias, b"keep").unwrap();
            if length == 248 {
                fs::write(store.segment_path(&id), b"remove").unwrap();
            }
            store.delete_session_observations_durable(&id).unwrap();
            assert_eq!(fs::read(&alias).unwrap(), b"keep");
        }
        fs::remove_dir_all(store.dir.as_ref()).unwrap();
        store.delete_session_observations_durable("s").unwrap();
    }
    #[test]
    fn durable_delete_ignores_replacement_directory_and_unlinks_segment_symlink_only() {
        let (_root, store, writer) = seeded();
        let foreign = tempfile::tempdir().unwrap();
        fs::write(foreign.path().join("s.ndjson"), b"keep").unwrap();
        fs::remove_dir_all(store.dir.as_ref()).unwrap();
        std::os::unix::fs::symlink(foreign.path(), store.dir.as_ref()).unwrap();
        store.delete_session_observations_durable("s").unwrap();
        assert!(!writer.record_batch(&FileClaim::new(), 2, vec![]).is_ok());
        assert_eq!(fs::read(foreign.path().join("s.ndjson")).unwrap(), b"keep");
        fs::remove_file(store.dir.as_ref()).unwrap();
        fs::create_dir(store.dir.as_ref()).unwrap();
        let store = WorkflowObservationStore::open(_root.path().into()).unwrap();
        std::os::unix::fs::symlink(foreign.path().join("s.ndjson"), store.segment_path("s"))
            .unwrap();
        store.delete_session_observations_durable("s").unwrap();
        assert_eq!(fs::read(foreign.path().join("s.ndjson")).unwrap(), b"keep");
    }
}

#[cfg(all(test, not(unix)))]
mod unsupported_tests {
    use super::*;
    #[test]
    fn durable_delete_unqualified_platform_has_no_effects() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkflowObservationStore::open(root.path().into()).unwrap();
        let writer = store.capture_writer("s".into(), "created".into());
        fs::write(store.segment_path("s"), b"keep").unwrap();
        assert_eq!(
            store
                .delete_session_observations_durable("s")
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::Unsupported
        );
        assert_eq!(fs::read(store.segment_path("s")).unwrap(), b"keep");
        assert!(writer.record_batch(&FileClaim::new(), 1, vec![]).is_ok());
    }
}
