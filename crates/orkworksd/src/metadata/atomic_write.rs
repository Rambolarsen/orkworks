//! Same-directory staging with checked durability and an explicit publication cutoff.
use std::{io, io::Write, path::Path};
use tempfile::TempPath;

fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

/// Complete staging I/O before the caller's final admission/deadline check.
/// A unique, exclusively created name cannot collide with another writer.
pub(crate) fn stage(target: &Path, contents: &[u8]) -> io::Result<TempPath> {
    stage_with_privacy(target, contents, false)
}

/// Private bytes must never pass through a broadly readable staging inode.
pub(crate) fn stage_private(target: &Path, contents: &[u8]) -> io::Result<TempPath> {
    stage_with_privacy(target, contents, true)
}

fn stage_with_privacy(target: &Path, contents: &[u8], private: bool) -> io::Result<TempPath> {
    // Filesystem-scoped fault injection lets application tests exercise failed
    // persistence across async workers without process-global fault state.
    #[cfg(test)]
    {
        let mut marker = target.as_os_str().to_os_string();
        marker.push(".write-blocked");
        if Path::new(&marker).is_dir() {
            return Err(io::Error::other("injected metadata staging failure"));
        }
    }
    let mut builder = tempfile::Builder::new();
    builder.prefix(".orkworks-write-").suffix(".tmp");
    // Create an ordinary file through the unique-name/cleanup helper. The
    // default tempfile creator marks Windows files TEMPORARY; these bytes are
    // about to become persistent metadata. OpenOptions also retains the prior
    // fs::write creation permissions (0666 filtered by umask on Unix).
    let mut staged = builder.make_in(parent(target), |path| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            if private {
                options.mode(0o600);
            }
        }
        #[cfg(not(unix))]
        let _ = private; // Inherit the owning directory's ordinary Windows ACL.
        options.open(path)
    })?;
    staged.write_all(contents)?;
    #[cfg(test)]
    testing::before_sync(testing::SyncPoint::File, target)?;
    staged.as_file().sync_all()?;
    // Close the writing handle before native Windows replacement.
    Ok(staged.into_temp_path())
}

/// Rename is the visible-write cutoff, even if the following directory sync
/// fails. Retire old ownership at that cutoff; an error must not revive it.
/// Errors after replacement do not imply that the old bytes remain on disk.
pub(crate) fn publish(
    staged: TempPath,
    target: &Path,
    on_publish: impl FnOnce(),
) -> io::Result<()> {
    crate::durable_file::replace(&staged, target)?;
    on_publish();
    sync_parent(target)
}

pub(crate) fn sync_parent(target: &Path) -> io::Result<()> {
    #[cfg(all(test, unix))]
    testing::before_sync(testing::SyncPoint::Directory, target)?;
    crate::durable_file::sync_directory(parent(target))
}

#[cfg(test)]
pub(crate) mod testing {
    use std::{cell::RefCell, io, path::Path};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum SyncPoint {
        File,
        #[cfg(unix)]
        Directory,
    }
    type Hook = Box<dyn FnMut(SyncPoint, &Path) -> io::Result<()>>;
    thread_local! {
        static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub(crate) struct Guard;
    pub(crate) fn install(hook: impl FnMut(SyncPoint, &Path) -> io::Result<()> + 'static) -> Guard {
        HOOK.with(|slot| assert!(slot.borrow_mut().replace(Box::new(hook)).is_none()));
        Guard
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            HOOK.with(|slot| {
                slot.borrow_mut().take();
            });
        }
    }
    pub(super) fn before_sync(point: SyncPoint, target: &Path) -> io::Result<()> {
        HOOK.with(|slot| match slot.borrow_mut().as_mut() {
            Some(hook) => hook(point, target),
            None => Ok(()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{self, SyncPoint};
    use crate::metadata::{write_atomic, MetadataStore};
    use std::{
        cell::{Cell, RefCell},
        fs, io,
        rc::Rc,
    };

    #[cfg(unix)]
    #[test]
    fn durable_publication_syncs_file_before_replace_and_directory_after() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("record.json");
        fs::write(&target, "old").unwrap();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let observed = calls.clone();
        let _hook = testing::install(move |point, path| {
            assert_eq!(
                fs::read_to_string(path).unwrap(),
                match point {
                    SyncPoint::File => "old",
                    SyncPoint::Directory => "new",
                }
            );
            observed.borrow_mut().push(point);
            Ok(())
        });
        write_atomic(&target, "new").unwrap();
        assert_eq!(*calls.borrow(), vec![SyncPoint::File, SyncPoint::Directory]);
    }

    #[test]
    fn durable_publication_file_sync_failure_preserves_target_and_cleans_stage() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("record.json");
        fs::write(&target, "old").unwrap();
        let _hook = testing::install(|point, _| {
            assert_eq!(point, SyncPoint::File);
            Err(io::Error::other("file sync failed"))
        });
        assert!(write_atomic(&target, "new").is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "old");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn durable_publication_directory_sync_failure_is_not_acknowledged() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("record.json");
        fs::write(&target, "old").unwrap();
        let _hook = testing::install(|point, _| match point {
            SyncPoint::File => Ok(()),
            SyncPoint::Directory => Err(io::Error::other("directory sync failed")),
        });
        assert!(write_atomic(&target, "new").is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn durable_publication_does_not_share_another_writers_staging_path() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("record.json");
        let legacy_stage = dir.path().join("record.json.tmp");
        fs::create_dir(&legacy_stage).unwrap();
        write_atomic(&target, "new").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        assert!(legacy_stage.is_dir());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn durable_publication_keeps_final_admission_after_file_sync() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = crate::test_support::test_session_metadata(
            "admission",
            "Session",
            "/workspace",
            "running",
            "now",
            "now",
        );
        let owner = store.try_write_session_owned(&meta).unwrap().unwrap();
        let before = fs::read(store.sessions_dir().join("admission.json")).unwrap();
        let synced = Rc::new(Cell::new(false));
        let observed = synced.clone();
        let _hook = testing::install(move |point, _| {
            assert_eq!(
                point,
                SyncPoint::File,
                "rejected write must not publish or sync directory"
            );
            observed.set(true);
            Ok(())
        });
        assert!(!store
            .try_write_session_if_owned_when(&meta, &owner, || {
                assert!(
                    synced.get(),
                    "final admission must follow staging durability work"
                );
                false
            })
            .unwrap());
        assert_eq!(
            fs::read(store.sessions_dir().join("admission.json")).unwrap(),
            before
        );
        assert_eq!(fs::read_dir(store.sessions_dir()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn durable_publication_invalidates_attention_owner_even_if_directory_sync_fails() {
        for writer in 0..4 {
            let dir = tempfile::tempdir().unwrap();
            let store = MetadataStore::new(dir.path());
            let mut meta = crate::test_support::test_session_metadata(
                "ownership",
                "Session",
                "/workspace",
                "running",
                "now",
                "now",
            );
            meta.lifecycle = "alive".into();
            meta.lifecycle_phase = "active".into();
            meta.connectivity = "online".into();
            meta.terminal_outcome = None;
            meta.final_observed_status_snapshot = None;
            meta.attention = Some("working".into());
            let owner = store.try_write_session_owned(&meta).unwrap().unwrap();
            let revision = store
                .try_read_revisioned_session(&meta.id)
                .unwrap()
                .unwrap()
                .revision;
            let mut replacement = meta.clone();
            replacement.attention = Some("needs_you".into());
            let hook = testing::install(|point, _| match point {
                SyncPoint::File => Ok(()),
                SyncPoint::Directory => Err(io::Error::other("directory sync failed")),
            });
            let failed = match writer {
                0 => store
                    .try_update_session(&meta.id, &revision, |current| {
                        current.attention = Some("needs_you".into());
                        Ok(())
                    })
                    .is_err(),
                1 => store.try_write_session_owned(&replacement).is_err(),
                2 => store
                    .try_mutate_session_owned(&meta.id, |current| {
                        current.attention = Some("needs_you".into());
                        Ok(true)
                    })
                    .is_err(),
                3 => store
                    .try_write_session_if_owned_when(&replacement, &owner, || true)
                    .is_err(),
                _ => unreachable!(),
            };
            assert!(failed, "writer {writer} must report the durability failure");
            drop(hook);
            assert_eq!(
                store.read_session(&meta.id).unwrap().attention.as_deref(),
                Some("needs_you")
            );
            assert!(
                !store.try_write_session_if_owned(&meta, &owner).unwrap(),
                "writer {writer} must retire the old owner after publication"
            );
            assert_eq!(
                store.read_session(&meta.id).unwrap().attention.as_deref(),
                Some("needs_you")
            );
        }
    }

    #[test]
    fn durable_publication_concurrent_writers_keep_separate_stages() {
        use std::{sync::mpsc, thread, time::Duration};
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("record.json");
        fs::write(&target, "old").unwrap();
        let (arrived_tx, arrived_rx) = mpsc::channel();
        let mut releases = Vec::new();
        let mut writers = Vec::new();
        let payloads: Vec<_> = (0..8)
            .map(|n| format!("payload-{n}").repeat(4096))
            .collect();
        for payload in &payloads {
            let target = target.clone();
            let payload = payload.clone();
            let arrived = arrived_tx.clone();
            let (release_tx, release_rx) = mpsc::channel();
            releases.push(release_tx);
            writers.push(thread::spawn(move || {
                let _hook = testing::install(move |point, _| {
                    if point == SyncPoint::File {
                        arrived.send(()).map_err(io::Error::other)?;
                        release_rx
                            .recv_timeout(Duration::from_secs(5))
                            .map_err(io::Error::other)?;
                    }
                    Ok(())
                });
                write_atomic(&target, &payload)
            }));
        }
        for _ in 0..8 {
            arrived_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            9,
            "each writer must own its own stage"
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "old");
        // Callers serialize publication for a metadata key. Windows can reject
        // simultaneous MoveFileExW replacements with ACCESS_DENIED; private
        // staging does not grant uncoordinated writers a publication lock.
        // Keep all stages alive together, then exercise that caller ordering.
        for ((release, writer), payload) in releases.into_iter().zip(writers).zip(&payloads) {
            release.send(()).unwrap();
            writer.join().unwrap().unwrap();
            assert_eq!(&fs::read_to_string(&target).unwrap(), payload);
        }
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn durable_publication_unchanged_session_polling_performs_no_syncs() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        fs::create_dir(dir.path().join("specs")).unwrap();
        fs::write(dir.path().join("specs/design.md"), "# Design").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            for id in 0..8 {
                let mut meta = crate::test_support::test_session_metadata(
                    format!("poll-{id}"),
                    "Session",
                    dir.path().display().to_string(),
                    "ended",
                    "2026-11-17T10:00:00Z",
                    "2026-11-17T10:00:00Z",
                );
                meta.cwd = dir.path().display().to_string();
                meta.worktree_binding = Some(
                    crate::session_location::WorktreeBinding::resolve(&meta.cwd, &meta.cwd)
                        .unwrap(),
                );
                store.try_write_session(&meta).unwrap();
            }
        }
        let projection = crate::session_projection::SessionProjection::new(state);
        let before = projection.list().unwrap();
        assert_eq!(before.len(), 8);
        assert!(before.iter().all(|row| row
            .plan_documents
            .as_ref()
            .is_some_and(|documents| documents.steps.iter().any(|step| step.available))));
        let _hook =
            testing::install(|_, _| panic!("unchanged polling must not add durability I/O"));
        for _ in 0..3 {
            let rows = projection.list().unwrap();
            assert_eq!(rows.len(), 8);
            assert!(rows.iter().all(|row| row
                .plan_documents
                .as_ref()
                .is_some_and(|documents| documents.steps.iter().any(|step| step.available))));
        }
    }

    #[cfg(windows)]
    #[test]
    fn durable_publication_windows_creates_and_replaces_a_record() {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileAttributesW, FILE_ATTRIBUTE_TEMPORARY, INVALID_FILE_ATTRIBUTES,
        };
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("record.json");
        let native_path: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        for payload in ["first", "replacement"] {
            write_atomic(&target, payload).unwrap();
            assert_eq!(fs::read_to_string(&target).unwrap(), payload);
            // SAFETY: native_path is a valid NUL-terminated UTF-16 path and
            // remains alive throughout the attribute lookup.
            let attributes = unsafe { GetFileAttributesW(native_path.as_ptr()) };
            assert_ne!(attributes, INVALID_FILE_ATTRIBUTES);
            assert_eq!(
                attributes & FILE_ATTRIBUTE_TEMPORARY,
                0,
                "published metadata must not retain temporary-file caching semantics"
            );
        }
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
