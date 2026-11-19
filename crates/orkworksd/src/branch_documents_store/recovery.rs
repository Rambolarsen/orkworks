//! Explicit recovery only; reads and reports never replace invalid records.
use super::{Record, Store};
use crate::{branch_documents::Identity, metadata::atomic_write};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::atomic::Ordering};

impl Store {
    fn recovery_bytes(&self, id: &Identity) -> Result<Vec<u8>, String> {
        let directory = std::fs::symlink_metadata(&self.root).map_err(|_| {
            "Saved choices cannot be read safely. Check storage access, then try again."
        })?;
        if !directory.is_dir() || directory.file_type().is_symlink() {
            return Err("Saved choices storage is not a regular directory. Try again after restoring storage.".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if directory.file_attributes() & 0x400 != 0 {
                return Err("Saved choices storage is linked; reset unavailable.".into());
            }
        }
        let bytes = crate::git_changes_fs::metadata_file(&self.path(id))
            .map_err(|_| "Saved choices cannot be preserved safely. Check storage access and size, then try again.")?
            .ok_or("Saved choices changed; refresh before resetting.")?;
        if Self::decode(id, &bytes).is_ok() {
            return Err("Saved choices are readable; refresh before resetting.".into());
        }
        Ok(bytes)
    }
    fn recovery_token(&self, root: &Path, id: &Identity, bytes: &[u8]) -> String {
        let mut hash = Sha256::new();
        for field in [
            self.recovery_nonce.as_bytes(),
            root.as_os_str().as_encoded_bytes(),
            Self::key(id).as_bytes(),
            &self.generation.load(Ordering::Acquire).to_be_bytes(),
            bytes,
        ] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field);
        }
        hex::encode(hash.finalize())
    }
    pub(crate) fn recovery_id(&self, root: &Path) -> Option<String> {
        let id = crate::branch_documents::identity(root).ok()?;
        let lock = self.lock(&id);
        let _guard = lock.try_lock().ok()?;
        let bytes = self.recovery_bytes(&id).ok()?;
        Some(self.recovery_token(root, &id, &bytes))
    }
    /// All slow I/O precedes admission. Only replacement and invalidation run
    /// while the caller's short final-admission guards are retained.
    pub(crate) fn reset_invalid<G>(
        &self,
        root: &Path,
        expected: &str,
        admit: impl FnOnce() -> Result<G, String>,
    ) -> Result<(), String> {
        let id = crate::branch_documents::identity(root)?;
        let lock = self.lock(&id);
        let _guard = lock
            .lock()
            .map_err(|_| "Saved choices are busy; try again.")?;
        let bytes = self.recovery_bytes(&id)?;
        if self.recovery_token(root, &id, &bytes) != expected {
            return Err("Saved choices changed; refresh before resetting.".into());
        }
        let target = self.path(&id);
        // Reserve a unique sibling exclusively. Its temporary empty inode is
        // replaced with ordinary staged bytes, including Windows write-through.
        let reserved = tempfile::Builder::new()
            .prefix(&format!(".{}-recovery-", Self::key(&id)))
            .suffix(".preserved")
            .tempfile_in(&self.root)
            .map_err(|_| "Could not preserve saved choices. Nothing was reset.")?
            .into_temp_path();
        atomic_write::stage_private(&reserved, &bytes)
            .and_then(|staged| atomic_write::publish(staged, &reserved, || {}))
            .map_err(|_| "Could not preserve saved choices. Nothing was reset.")?;
        reserved
            .keep()
            .map_err(|_| "Could not retain preserved choices. Nothing was reset.")?;
        let empty = serde_json::to_vec(&Record {
            version: 1,
            identity: id.clone(),
            documents: Vec::new(),
        })
        .map_err(|_| "Could not prepare reset. Nothing was reset.")?;
        let staged = atomic_write::stage_private(&target, &empty)
            .map_err(|_| "Could not prepare reset. Nothing was reset.")?;
        // Recheck external replacement too. This remains a single-owner store,
        // not a promise to serialize uncoordinated filesystem edits.
        if self.recovery_bytes(&id)? != bytes {
            return Err("Saved choices changed; refresh before resetting.".into());
        }
        let admission = admit()?;
        self.generation
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
            .map_err(|_| "Document generation exhausted; restart the app.")?;
        crate::durable_file::replace(&staged, &target)
            .map_err(|_| "Could not reset saved choices. Refresh before trying again.")?;
        drop(admission);
        atomic_write::sync_parent(&target).map_err(|_| {
            "Reset may already have been applied. Refresh before trying again.".to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io};
    fn fixture() -> (tempfile::TempDir, Store, Identity) {
        let root = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(root.path()).unwrap();
        repo.set_head("refs/heads/recovery").unwrap();
        let store = Store::new(root.path());
        fs::create_dir(&store.root).unwrap();
        let id = crate::branch_documents::identity(root.path()).unwrap();
        fs::write(store.path(&id), b"{broken").unwrap();
        (root, store, id)
    }
    fn preserved(store: &Store) -> Vec<Vec<u8>> {
        fs::read_dir(&store.root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".preserved"))
            .map(|e| fs::read(e.path()).unwrap())
            .collect()
    }
    #[test]
    fn reset_invalid_preserves_malformed_unsupported_and_identity_invalid_bytes() {
        for case in 0..3 {
            let (root, store, id) = fixture();
            let bytes = match case {
                0 => b"{broken".to_vec(),
                _ => serde_json::to_vec(&Record {
                    version: if case == 1 { 2 } else { 1 },
                    identity: Identity {
                        branch: if case == 2 {
                            "refs/heads/foreign".into()
                        } else {
                            id.branch.clone()
                        },
                        ..id.clone()
                    },
                    documents: Vec::new(),
                })
                .unwrap(),
            };
            fs::write(store.path(&id), &bytes).unwrap();
            let token = store.recovery_id(root.path()).unwrap();
            store.reset_invalid(root.path(), &token, || Ok(())).unwrap();
            assert_eq!(preserved(&store), vec![bytes]);
            assert!(store.load(&id).unwrap().is_empty());
            assert!(store.recovery_id(root.path()).is_none());
            assert!(store.reset_invalid(root.path(), &token, || Ok(())).is_err());
            let reopened = Store::new(root.path());
            assert!(reopened.load(&id).unwrap().is_empty());
        }
    }
    #[test]
    fn reset_invalid_refuses_changed_missing_valid_and_reopened_snapshots() {
        let (root, store, id) = fixture();
        let token = store.recovery_id(root.path()).unwrap();
        fs::write(store.path(&id), "{changed").unwrap();
        assert!(store.reset_invalid(root.path(), &token, || Ok(())).is_err());
        let fresh = store.recovery_id(root.path()).unwrap();
        assert!(Store::new(root.path())
            .reset_invalid(root.path(), &fresh, || Ok(()))
            .is_err());
        fs::remove_file(store.path(&id)).unwrap();
        assert!(store.recovery_id(root.path()).is_none());
        assert!(store.reset_invalid(root.path(), &fresh, || Ok(())).is_err());
        store.save(&id, Vec::new()).unwrap();
        assert!(store.recovery_id(root.path()).is_none());
        assert!(store.reset_invalid(root.path(), &fresh, || Ok(())).is_err());
        assert!(preserved(&store).is_empty());
    }
    #[test]
    fn reset_invalid_admission_failure_preserves_active_record_and_never_replaces_a_backup() {
        let (root, store, id) = fixture();
        let token = store.recovery_id(root.path()).unwrap();
        for _ in 0..2 {
            assert!(store
                .reset_invalid(root.path(), &token, || Err::<(), _>(
                    "Context changed".into()
                ))
                .is_err());
            assert_eq!(fs::read(store.path(&id)).unwrap(), b"{broken");
        }
        assert_eq!(preserved(&store), vec![b"{broken".to_vec(); 2]);
    }
    #[test]
    fn reset_invalid_preservation_failure_never_replaces_active_record() {
        use atomic_write::testing::{self, SyncPoint};
        let (root, store, id) = fixture();
        let token = store.recovery_id(root.path()).unwrap();
        let _hook = testing::install(|point, path| {
            if point == SyncPoint::File && path.to_string_lossy().ends_with(".preserved") {
                Err(io::Error::other("Preservation failed"))
            } else {
                Ok(())
            }
        });
        assert!(store.reset_invalid(root.path(), &token, || Ok(())).is_err());
        assert_eq!(fs::read(store.path(&id)).unwrap(), b"{broken");
        assert!(preserved(&store).is_empty());
        assert_eq!(fs::read_dir(&store.root).unwrap().count(), 1);
    }
    #[test]
    fn reset_invalid_staging_failure_after_preservation_leaves_original_retryable() {
        use atomic_write::testing::{self, SyncPoint};
        let (root, store, id) = fixture();
        let token = store.recovery_id(root.path()).unwrap();
        let target = store.path(&id);
        let hook = testing::install(move |point, path| {
            if point == SyncPoint::File && path == target {
                Err(io::Error::other("Reset staging failed"))
            } else {
                Ok(())
            }
        });
        assert!(store.reset_invalid(root.path(), &token, || Ok(())).is_err());
        assert_eq!(fs::read(store.path(&id)).unwrap(), b"{broken");
        assert_eq!(preserved(&store), vec![b"{broken".to_vec()]);
        drop(hook);
        store.reset_invalid(root.path(), &token, || Ok(())).unwrap();
        assert!(store.load(&id).unwrap().is_empty());
        assert_eq!(preserved(&store), vec![b"{broken".to_vec(); 2]);
    }
    #[cfg(windows)]
    #[test]
    fn reset_invalid_windows_preserved_copy_has_ordinary_attributes() {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileAttributesW, FILE_ATTRIBUTE_TEMPORARY, INVALID_FILE_ATTRIBUTES,
        };
        let (root, store, _id) = fixture();
        let token = store.recovery_id(root.path()).unwrap();
        store.reset_invalid(root.path(), &token, || Ok(())).unwrap();
        let copy = fs::read_dir(&store.root)
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy().ends_with(".preserved"))
            .unwrap();
        let wide: Vec<u16> = copy
            .path()
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // SAFETY: the live path buffer is NUL terminated and valid for this call.
        let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
        assert_ne!(attributes, INVALID_FILE_ATTRIBUTES);
        assert_eq!(attributes & FILE_ATTRIBUTE_TEMPORARY, 0);
    }
    #[cfg(unix)]
    #[test]
    fn reset_invalid_preserved_bytes_are_private_to_the_owner() {
        use atomic_write::testing::{self, SyncPoint};
        use std::os::unix::fs::PermissionsExt;
        let (root, store, _id) = fixture();
        let token = store.recovery_id(root.path()).unwrap();
        let _hook = testing::install(|point, target| {
            if point == SyncPoint::File {
                for entry in fs::read_dir(target.parent().unwrap())
                    .unwrap()
                    .filter_map(Result::ok)
                {
                    if entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".orkworks-write-")
                    {
                        assert_eq!(
                            entry.metadata().unwrap().permissions().mode() & 0o777,
                            0o600,
                            "private bytes exposed during staging"
                        );
                    }
                }
            }
            Ok(())
        });
        store.reset_invalid(root.path(), &token, || Ok(())).unwrap();
        let copy = fs::read_dir(&store.root)
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy().ends_with(".preserved"))
            .unwrap();
        assert_eq!(copy.metadata().unwrap().permissions().mode() & 0o777, 0o600);
    }
    #[cfg(unix)]
    #[test]
    fn reset_invalid_post_publication_sync_failure_rejects_stale_scans_without_rollback() {
        use atomic_write::testing::{self, SyncPoint};
        let (root, store, id) = fixture();
        store.save(&id, Vec::new()).unwrap();
        let prepared = store.prepare_reconciliation(&id, None).unwrap();
        fs::write(store.path(&id), "{broken").unwrap();
        let token = store.recovery_id(root.path()).unwrap();
        let target = store.path(&id);
        let _hook = testing::install(move |point, path| {
            if point == SyncPoint::Directory && path == target {
                Err(io::Error::other("Directory sync failed"))
            } else {
                Ok(())
            }
        });
        assert!(store
            .reset_invalid(root.path(), &token, || Ok(()))
            .unwrap_err()
            .contains("may already"));
        assert!(store.load(&id).unwrap().is_empty());
        assert_eq!(preserved(&store), vec![b"{broken".to_vec()]);
        assert!(store.try_publish(&prepared).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn reset_invalid_refuses_links_and_nonregular_records_without_following_them() {
        let (root, store, id) = fixture();
        let external = root.path().join("external");
        fs::write(&external, "{foreign").unwrap();
        fs::remove_file(store.path(&id)).unwrap();
        std::os::unix::fs::symlink(&external, store.path(&id)).unwrap();
        assert!(store.recovery_id(root.path()).is_none());
        assert!(store
            .reset_invalid(root.path(), &"a".repeat(64), || Ok(()))
            .is_err());
        assert_eq!(fs::read(&external).unwrap(), b"{foreign");
        fs::remove_file(store.path(&id)).unwrap();
        fs::create_dir(store.path(&id)).unwrap();
        assert!(store.recovery_id(root.path()).is_none());
        assert!(preserved(&store).is_empty());
    }
}
