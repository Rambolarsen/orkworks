//! Narrow durable replay owner; no metadata/organization deletion authority.
#![cfg_attr(not(test), allow(dead_code))]
use super::*;

pub(crate) struct ReplayCleanup {
    root: PathBuf,
    id: String,
    checkpoints: Arc<Mutex<HashMap<String, SummaryCheckpointCacheEntry>>>,
}
impl MetadataStore {
    // Memory only: safe to capture under application ownership guards.
    pub(crate) fn capture_replay_cleanup(&self, id: &str) -> std::io::Result<ReplayCleanup> {
        if id.is_empty()
            || id.len() > 256
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid replay identity",
            ));
        }
        Ok(ReplayCleanup {
            root: self.root.clone(),
            id: id.into(),
            checkpoints: self.summary_checkpoints.clone(),
        })
    }
}
pub(crate) struct AnchoredReplayCleanup {
    #[cfg(unix)]
    root: std::fs::File,
    #[cfg(unix)]
    events: Option<std::fs::File>,
    id: String,
    checkpoints: Arc<Mutex<HashMap<String, SummaryCheckpointCacheEntry>>>,
}
impl ReplayCleanup {
    // Blocking-only, read-only binding outside all application guards.
    #[cfg(unix)]
    pub(crate) fn bind(
        self,
        lease: &crate::workspace_runtime::WorkspaceLease,
    ) -> std::io::Result<AnchoredReplayCleanup> {
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::fs::OpenOptionsExt;
        let root = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.root)?;
        lease.validate_directory(&root)?;
        let name = std::ffi::CString::new("events").unwrap();
        // SAFETY: a live directory FD and a valid NUL-terminated fixed basename.
        let fd = unsafe {
            libc::openat(
                root.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        let events = if fd < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(error);
            }
            None
        } else {
            // SAFETY: openat returned a new owned FD; File closes it exactly once.
            Some(unsafe { std::fs::File::from_raw_fd(fd) })
        };

        Ok(AnchoredReplayCleanup {
            root,
            events,
            id: self.id,
            checkpoints: self.checkpoints,
        })
    }
    #[cfg(not(unix))]
    pub(crate) fn bind(
        self,
        _lease: &crate::workspace_runtime::WorkspaceLease,
    ) -> std::io::Result<AnchoredReplayCleanup> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "durable replay cleanup is not qualified on this platform",
        ))
    }
}
impl AnchoredReplayCleanup {
    #[cfg(unix)]
    pub(crate) fn remove(&self) -> std::io::Result<()> {
        self.remove_with(std::fs::File::sync_all)
    }
    #[cfg(not(unix))]
    pub(crate) fn remove(&self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "durable replay cleanup is not qualified on this platform",
        ))
    }
    #[cfg(unix)]
    fn remove_with(
        &self,
        sync: impl Fn(&std::fs::File) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        use std::os::fd::AsRawFd;
        if let Some(events) = &self.events {
            // Reject unknown limits before effects. Never ask a filesystem to
            // resolve an overlong basename: some profiles may truncate it.
            // SAFETY: the FD is live and _PC_NAME_MAX is a valid query.
            let limit = unsafe { libc::fpathconf(events.as_raw_fd(), libc::_PC_NAME_MAX) };
            let limit = usize::try_from(limit)
                .ok()
                .filter(|limit| *limit > 0)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "replay directory component limit unavailable",
                    )
                })?;
            for suffix in ["ndjson", "terminal", "terminal-size"] {
                let name =
                    std::ffi::CString::new(format!("{}.{suffix}", self.id)).map_err(|_| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "invalid replay basename",
                        )
                    })?;
                // No directory entry can have this exact basename. Preserve
                // the full SQL ID and do not resolve a truncated alias.
                if name.as_bytes().len() > limit {
                    continue;
                }
                // SAFETY: a live directory FD and a validated single basename;
                // unlinkat flags=0 removes a file/link, never a directory tree.
                if unsafe { libc::unlinkat(events.as_raw_fd(), name.as_ptr(), 0) } < 0 {
                    let error = std::io::Error::last_os_error();
                    if error.kind() != std::io::ErrorKind::NotFound {
                        return Err(error);
                    }
                }
            }
            // Always repeat sync, including retries whose previous unlink
            // succeeded but sync failed. Only then may SQL record progress.
            sync(events)?;
        }
        if self.events.is_none() {
            let name = std::ffi::CString::new("events").unwrap();
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            // SAFETY: live root FD, fixed C basename, valid stat output pointer;
            // AT_SYMLINK_NOFOLLOW observes a later alias without following it.
            if unsafe {
                libc::fstatat(
                    self.root.as_raw_fd(),
                    name.as_ptr(),
                    stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } == 0
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "events directory appeared after replay capture",
                ));
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(error);
            }
        }
        sync(&self.root)?;

        self.checkpoints.lock().unwrap().remove(&self.id);
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn replay_cleanup_retries_directory_sync_even_after_files_are_absent() {
        for fail_at in [1, 2] {
            let dir = tempfile::tempdir().unwrap();
            let store = MetadataStore::new(dir.path());
            fs::create_dir_all(store.events_dir()).unwrap();
            for ext in ["ndjson", "terminal", "terminal-size"] {
                fs::write(store.events_dir().join(format!("s.{ext}")), b"payload").unwrap();
            }
            store.summary_checkpoints.lock().unwrap().insert(
                "s".into(),
                SummaryCheckpointCacheEntry {
                    stamp: None,
                    latest: Some("retained".into()),
                },
            );
            let cleanup = bind(&store, "s").unwrap();
            let calls = std::cell::Cell::new(0);
            assert!(cleanup
                .remove_with(|file| {
                    calls.set(calls.get() + 1);
                    if calls.get() == fail_at {
                        Err(std::io::Error::other("sync failed"))
                    } else {
                        file.sync_all()
                    }
                })
                .is_err());
            assert!(!store.events_dir().join("s.terminal").exists());
            assert!(store.summary_checkpoints.lock().unwrap().contains_key("s"));
            calls.set(0);
            cleanup
                .remove_with(|file| {
                    calls.set(calls.get() + 1);
                    file.sync_all()
                })
                .unwrap();
            assert_eq!(calls.get(), 2);
            assert!(!store.summary_checkpoints.lock().unwrap().contains_key("s"));
        }
    }
}

#[cfg(all(test, unix))]
mod boundary_tests {
    use super::*;
    #[test]
    fn replay_cleanup_rejects_parent_symlinks_and_invalid_ids_and_handles_absence() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path());
        fs::write(other.path().join("s.terminal"), b"foreign").unwrap();
        std::os::unix::fs::symlink(other.path(), store.events_dir()).unwrap();
        assert!(bind(&store, "s").is_err());
        assert_eq!(
            fs::read(other.path().join("s.terminal")).unwrap(),
            b"foreign"
        );
        fs::remove_file(store.events_dir()).unwrap();
        bind(&store, "s").unwrap().remove().unwrap();
        for id in ["", "../s", "s/other", "s\0x"] {
            assert!(store.capture_replay_cleanup(id).is_err());
        }
        fs::create_dir(store.events_dir()).unwrap();
        std::os::unix::fs::symlink(
            other.path().join("s.terminal"),
            store.events_dir().join("s.terminal"),
        )
        .unwrap();
        bind(&store, "s").unwrap().remove().unwrap();
        assert_eq!(
            fs::read(other.path().join("s.terminal")).unwrap(),
            b"foreign"
        );
    }
}
#[cfg(all(test, not(unix)))]
mod unsupported_tests {
    use super::*;
    #[test]
    fn replay_cleanup_unqualified_platform_preserves_payloads() {
        let root = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path());
        fs::create_dir(store.events_dir()).unwrap();
        fs::write(store.events_dir().join("s.terminal"), b"keep").unwrap();
        assert_eq!(
            bind(&store, "s").err().unwrap().kind(),
            std::io::ErrorKind::Unsupported
        );
        assert_eq!(
            fs::read(store.events_dir().join("s.terminal")).unwrap(),
            b"keep"
        );
    }
}

#[cfg(all(test, unix))]
mod long_identity_tests {
    use super::*;
    #[test]
    fn replay_cleanup_accepts_maximum_identity_when_optional_basenames_exceed_name_max() {
        for length in [243, 256] {
            let root = tempfile::tempdir().unwrap();
            let store = MetadataStore::new(root.path());
            fs::create_dir(store.events_dir()).unwrap();
            let id = "x".repeat(length);
            for ext in ["ndjson", "terminal", "terminal-size"] {
                match fs::write(store.events_dir().join(format!("{id}.{ext}")), b"payload") {
                    Ok(()) => {}
                    Err(e) if e.raw_os_error() == Some(libc::ENAMETOOLONG) => {}
                    Err(e) => panic!("payload setup failed: {e}"),
                }
            }
            bind(&store, &id).unwrap().remove().unwrap();
            assert!(fs::read_dir(store.events_dir()).unwrap().next().is_none());
        }
    }
}

#[cfg(test)]
fn bind(store: &MetadataStore, id: &str) -> std::io::Result<AnchoredReplayCleanup> {
    let lease = crate::workspace_runtime::WorkspaceLease::acquire(&store.root)?;
    store.capture_replay_cleanup(id)?.bind(&lease)
}
