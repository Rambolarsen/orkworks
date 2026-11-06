//! Durable conditional selection clear; no session or organization authority.
use super::{Edit, Owner};
use std::{io, sync::Arc};

pub(crate) struct SettingsCleanup {
    pub(super) owner: Arc<Owner>,
}
pub(crate) struct BoundSettingsCleanup {
    owner: Arc<Owner>,
    #[cfg(unix)]
    directory: std::fs::File,
}
impl SettingsCleanup {
    #[cfg(unix)]
    pub(crate) fn bind(
        self,
        lease: &crate::workspace_runtime::WorkspaceLease,
    ) -> io::Result<BoundSettingsCleanup> {
        use std::os::unix::fs::OpenOptionsExt;
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.owner.root)?;
        lease.validate_directory(&directory)?;
        Ok(BoundSettingsCleanup {
            owner: self.owner,
            directory,
        })
    }
    #[cfg(not(unix))]
    pub(crate) fn bind(
        self,
        _lease: &crate::workspace_runtime::WorkspaceLease,
    ) -> io::Result<BoundSettingsCleanup> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "durable settings cleanup is not qualified on this platform",
        ))
    }
}
impl BoundSettingsCleanup {
    pub(crate) fn clear(&self, id: &str) -> (io::Result<()>, Option<Edit>) {
        let edit = match self.owner.edit() {
            Ok(edit) => edit,
            Err(error) => return (Err(error), None),
        };
        #[cfg(unix)]
        let result = self.clear_with(id, std::fs::File::sync_all);
        #[cfg(not(unix))]
        let result = Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "durable settings cleanup is not qualified on this platform",
        ));
        (result, Some(edit))
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::{
        collections::BTreeMap,
        ffi::{CStr, CString},
        fs::File,
        io::{Read, Write},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::MetadataExt,
        },
    };
    const LIMIT: usize = 1024 * 1024;
    fn invalid(message: &str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }
    fn identity(file: &File) -> io::Result<(u64, u64)> {
        let m = file.metadata()?;
        Ok((m.dev(), m.ino()))
    }
    fn entry(directory: &File, name: &CStr) -> io::Result<Option<(u64, u64)>> {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: live directory FD, valid C basename and writable stat output.
        if unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } < 0
        {
            let e = io::Error::last_os_error();
            return if e.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(e)
            };
        }
        // SAFETY: successful fstatat initialized stat.
        let stat = unsafe { stat.assume_init() };
        Ok(Some((stat.st_dev as u64, stat.st_ino as u64)))
    }
    struct Staged<'a> {
        directory: &'a File,
        name: CString,
        file: File,
        renamed: bool,
    }
    impl<'a> Staged<'a> {
        fn create(directory: &'a File) -> io::Result<Self> {
            let mut random = [0; 16];
            getrandom::fill(&mut random)
                .map_err(|_| io::Error::other("settings temporary name unavailable"))?;
            let name = CString::new(format!(
                ".workspace-sql-cleanup-{}.tmp",
                hex::encode(random)
            ))
            .map_err(|_| invalid("invalid settings temporary name"))?;
            // SAFETY: live directory, generated single C basename, private mode.
            // O_EXCL refuses existing entries; O_NOFOLLOW rejects aliases.
            let fd = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDWR
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: openat returned a new owned FD, closed exactly once by File.
            let file = unsafe { File::from_raw_fd(fd) };
            Ok(Self {
                directory,
                name,
                file,
                renamed: false,
            })
        }
    }
    impl Drop for Staged<'_> {
        fn drop(&mut self) {
            if !self.renamed
                && identity(&self.file)
                    .ok()
                    .is_some_and(|id| entry(self.directory, &self.name).ok() == Some(Some(id)))
            {
                // SAFETY: live directory FD and this attempt's verified temp entry;
                // flags0 unlinks a leaf only, never follows or recurses.
                unsafe { libc::unlinkat(self.directory.as_raw_fd(), self.name.as_ptr(), 0) };
            }
        }
    }
    impl BoundSettingsCleanup {
        pub(super) fn clear_with(
            &self,
            id: &str,
            sync: impl Fn(&File) -> io::Result<()>,
        ) -> io::Result<()> {
            let name = c"workspace.json";
            // SAFETY: live original directory FD and fixed basename. Nonblocking
            // avoids waiting on a FIFO; no-follow rejects a leaf symlink.
            let fd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                let error = io::Error::last_os_error();
                return if error.kind() == io::ErrorKind::NotFound {
                    sync(&self.directory)
                } else {
                    Err(error)
                };
            }
            // SAFETY: successful openat returned one new owned FD.
            let file = unsafe { File::from_raw_fd(fd) };
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.nlink() != 1 {
                return Err(invalid("settings entry is not an unaliased regular file"));
            }
            if metadata.len() > LIMIT as u64 {
                return Err(invalid("settings exceed cleanup byte bound"));
            }
            let original = identity(&file)?;
            let mut bytes = Vec::new();
            (&file).take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
            if bytes.len() > LIMIT {
                return Err(invalid("settings exceed cleanup byte bound"));
            }
            let mut values: BTreeMap<String, Box<serde_json::value::RawValue>> =
                crate::harness::definition::parse_strict_json(&bytes, LIMIT)
                    .map_err(|_| invalid("invalid or ambiguous workspace settings"))?;
            let memory: crate::metadata::WorkspaceMemory = serde_json::from_slice(&bytes)
                .map_err(|_| invalid("invalid workspace settings fields"))?;
            if memory.last_active_session_id.as_deref() != Some(id) {
                sync(&file)?;
                if entry(&self.directory, name)? != Some(original) {
                    return Err(invalid("workspace settings changed during cleanup"));
                }
                return sync(&self.directory);
            }
            values.remove("lastActiveSessionId");
            values.remove("lastActiveAt");
            drop(bytes);
            drop(memory);
            let bytes = serde_json::to_vec(&values).map_err(io::Error::other)?;
            if bytes.len() > LIMIT {
                return Err(invalid("replacement settings exceed cleanup byte bound"));
            }
            let mut staged = Staged::create(&self.directory)?;
            staged.file.write_all(&bytes)?;
            sync(&staged.file)?;
            if entry(&self.directory, name)? != Some(original) {
                return Err(invalid("workspace settings replaced during cleanup"));
            }
            if entry(&self.directory, &staged.name)? != Some(identity(&staged.file)?) {
                return Err(invalid(
                    "settings staging entry was replaced during cleanup",
                ));
            }
            // SAFETY: both basenames are fixed/generated, both live FDs pin the
            // same original directory; rename replaces only the verified leaf.
            if unsafe {
                libc::renameat(
                    self.directory.as_raw_fd(),
                    staged.name.as_ptr(),
                    self.directory.as_raw_fd(),
                    name.as_ptr(),
                )
            } < 0
            {
                return Err(io::Error::last_os_error());
            }
            staged.renamed = true;
            sync(&self.directory)
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::{metadata::MetadataStore, workspace_runtime::WorkspaceLease};
    use std::{fs, os::unix::fs::MetadataExt};
    fn fixture() -> (
        tempfile::TempDir,
        MetadataStore,
        WorkspaceLease,
        BoundSettingsCleanup,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("metadata");
        let lease = WorkspaceLease::acquire(&root).unwrap();
        let store = MetadataStore::new(&root);
        fs::write(store.workspace_memory_path(), br#"{"lastActiveSessionId":"s","lastActiveAt":"before","activeHarnessIds":["codex"],"activeHarnessRevision":7,"extension":{"n":123456789012345678901234567890}}"#).unwrap();
        let bound = store.capture_settings_cleanup().bind(&lease).unwrap();
        (dir, store, lease, bound)
    }
    fn bytes(store: &MetadataStore) -> Vec<u8> {
        fs::read(store.workspace_memory_path()).unwrap()
    }
    #[test]
    fn sql_settings_owner_sync_failures_preserve_retry_and_repeat_absent_parent_sync() {
        for fail_at in [1, 2] {
            let (_dir, store, _lease, bound) = fixture();
            let before = bytes(&store);
            let _edit = bound.owner.edit().unwrap();
            let calls = std::cell::Cell::new(0);
            assert!(bound
                .clear_with("s", |file| {
                    calls.set(calls.get() + 1);
                    if calls.get() == fail_at {
                        Err(io::Error::other("sync fault"))
                    } else {
                        file.sync_all()
                    }
                })
                .is_err());
            if fail_at == 1 {
                assert_eq!(bytes(&store), before);
            } else {
                assert_eq!(
                    store
                        .read_workspace_memory()
                        .unwrap()
                        .last_active_session_id,
                    None
                );
            }
            calls.set(0);
            bound
                .clear_with("s", |file| {
                    calls.set(calls.get() + 1);
                    file.sync_all()
                })
                .unwrap();
            assert_eq!(calls.get(), 2);
            assert_eq!(
                store
                    .read_workspace_memory()
                    .unwrap()
                    .last_active_session_id,
                None
            );
            assert!(String::from_utf8(bytes(&store))
                .unwrap()
                .contains("123456789012345678901234567890"));
            assert_eq!(
                fs::read_dir(store.root_path())
                    .unwrap()
                    .filter(|p| p
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".workspace-sql-cleanup-"))
                    .count(),
                0
            );
        }
        let (_dir, store, _lease, bound) = fixture();
        fs::remove_file(store.workspace_memory_path()).unwrap();
        let _edit = bound.owner.edit().unwrap();
        assert!(bound
            .clear_with("s", |_| Err(io::Error::other("parent sync failed")))
            .is_err());
        let calls = std::cell::Cell::new(0);
        bound
            .clear_with("s", |file| {
                calls.set(calls.get() + 1);
                file.sync_all()
            })
            .unwrap();
        assert_eq!(calls.get(), 1);
        assert!(!store.workspace_memory_path().exists());
    }
    #[test]
    fn sql_settings_owner_reads_latest_and_holds_shared_permit_even_for_noop() {
        let (_dir, store, _lease, bound) = fixture();
        let replacement = br#"{"lastActiveSessionId":"other","lastActiveAt":"new","extension":123456789012345678901234567890}"#;
        fs::write(store.workspace_memory_path(), replacement).unwrap();
        let (result, guard) = bound.clear("s");
        result.unwrap();
        assert_eq!(bytes(&store), replacement);
        assert_eq!(
            store.edit_workspace_memory().err().unwrap().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(
            store
                .reopen_with_shared_workspace_memory()
                .edit_workspace_memory()
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::WouldBlock
        );
        drop(guard);
        assert!(store.edit_workspace_memory().is_ok());
    }
    #[test]
    fn sql_settings_owner_preserves_invalid_ambiguous_and_oversized_memory() {
        let (_dir, store, _lease, bound) = fixture();
        let oversized = format!("{{\"pad\":\"{}\"}}", "x".repeat(1024 * 1024));
        for bad in [
            b"not json".as_slice(),
            b"{} {}",
            br#"{"lastActiveSessionId":"s","lastActiveSessionId":"other"}"#,
            br#"{"lastActiveSessionId":"s","unknown":{"x":1,"x":2}}"#,
            br#"{"lastActiveSessionId":"s","activeHarnessIds":false}"#,
            b"[]",
            oversized.as_bytes(),
        ] {
            fs::write(store.workspace_memory_path(), bad).unwrap();
            let (result, guard) = bound.clear("s");
            assert!(result.is_err());
            drop(guard);
            assert_eq!(bytes(&store), bad);
        }
    }
    #[test]
    fn sql_settings_owner_refuses_leaf_aliases_directories_and_fifos_without_following() {
        use std::os::unix::fs::symlink;
        let (dir, store, _lease, bound) = fixture();
        let path = store.workspace_memory_path();
        let foreign = dir.path().join("foreign");
        fs::write(&foreign, bytes(&store)).unwrap();
        let original = fs::read(&foreign).unwrap();
        fs::remove_file(&path).unwrap();
        symlink(&foreign, &path).unwrap();
        let (r, g) = bound.clear("s");
        assert!(r.is_err());
        drop(g);
        assert_eq!(fs::read(&foreign).unwrap(), original);
        fs::remove_file(&path).unwrap();
        fs::hard_link(&foreign, &path).unwrap();
        let (r, g) = bound.clear("s");
        assert!(r.is_err());
        drop(g);
        assert_eq!(fs::read(&foreign).unwrap(), original);
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep"), b"keep").unwrap();
        let (r, g) = bound.clear("s");
        assert!(r.is_err());
        drop(g);
        assert_eq!(fs::read(path.join("keep")).unwrap(), b"keep");
        fs::remove_dir_all(&path).unwrap();
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: valid C fixture path and private FIFO mode; no production data.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let (r, g) = bound.clear("s");
        assert!(r.is_err());
        drop(g);
    }
    #[test]
    fn sql_settings_owner_pins_original_parent_and_validates_lease() {
        let (dir, store, lease, bound) = fixture();
        let foreign = tempfile::tempdir().unwrap();
        let foreign_lease = WorkspaceLease::acquire(foreign.path()).unwrap();
        assert!(store
            .capture_settings_cleanup()
            .bind(&foreign_lease)
            .is_err());
        drop(foreign_lease);
        let moved = dir.path().join("original");
        fs::rename(store.root_path(), &moved).unwrap();
        fs::create_dir(store.root_path()).unwrap();
        fs::write(store.workspace_memory_path(), b"replacement").unwrap();
        let (r, g) = bound.clear("s");
        r.unwrap();
        drop(g);
        assert_eq!(bytes(&store), b"replacement");
        let saved: crate::metadata::WorkspaceMemory =
            serde_json::from_slice(&fs::read(moved.join("workspace.json")).unwrap()).unwrap();
        assert_eq!(saved.last_active_session_id, None);
        assert!(store.capture_settings_cleanup().bind(&lease).is_err());
    }
    #[test]
    fn sql_settings_owner_refuses_target_replacement_before_rename() {
        let (_dir, store, _lease, bound) = fixture();
        let before = bytes(&store);
        let old = store.root_path().join("old-workspace");
        let _edit = bound.owner.edit().unwrap();
        let result = bound.clear_with("s", |file| {
            if file.metadata()?.is_file() {
                fs::rename(store.workspace_memory_path(), &old)?;
                fs::write(store.workspace_memory_path(), b"replacement")?;
            }
            file.sync_all()
        });
        assert!(result.is_err());
        assert_eq!(bytes(&store), b"replacement");
        assert_eq!(fs::read(old).unwrap(), before);
    }
    #[test]
    fn sql_settings_owner_refuses_staged_entry_replacement_without_overwriting_settings() {
        let (_dir, store, _lease, bound) = fixture();
        let before = bytes(&store);
        let _edit = bound.owner.edit().unwrap();
        let foreign_name = std::cell::RefCell::new(None);
        let result = bound.clear_with("s", |file| {
            if file.metadata()?.is_file() {
                let inode = file.metadata()?.ino();
                let stage = fs::read_dir(store.root_path())?
                    .map(|p| p.unwrap().path())
                    .find(|p| fs::symlink_metadata(p).is_ok_and(|m| m.ino() == inode))
                    .unwrap();
                fs::rename(&stage, store.root_path().join("original-stage"))?;
                fs::write(&stage, b"foreign stage")?;
                *foreign_name.borrow_mut() = Some(stage);
            }
            file.sync_all()
        });
        assert!(
            result.is_err(),
            "cleanup renamed an unowned replacement stage"
        );
        assert_eq!(bytes(&store), before);
        assert_eq!(
            fs::read(foreign_name.borrow().as_ref().unwrap()).unwrap(),
            b"foreign stage"
        );
    }
}

#[cfg(all(test, not(unix)))]
mod unsupported_tests {
    #[test]
    fn sql_settings_owner_unsupported_bind_preserves_settings() {
        let root = tempfile::tempdir().unwrap();
        let lease = crate::workspace_runtime::WorkspaceLease::acquire(root.path()).unwrap();
        let store = crate::metadata::MetadataStore::new(root.path());
        std::fs::write(store.workspace_memory_path(), b"keep").unwrap();
        assert!(
            matches!(store.capture_settings_cleanup().bind(&lease),Err(e) if e.kind()==std::io::ErrorKind::Unsupported)
        );
        assert_eq!(
            std::fs::read(store.workspace_memory_path()).unwrap(),
            b"keep"
        );
    }
}
