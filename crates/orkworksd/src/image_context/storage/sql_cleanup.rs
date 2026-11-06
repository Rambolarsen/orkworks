//! Private SQL cleanup keeps the image owner's lock through durable progress.
use super::unix::{component, Directory, Manifest};
use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    io::{self, Read},
    os::unix::{
        ffi::OsStringExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawFd, FromRawFd},
    },
    path::{Path, PathBuf},
};
pub(crate) struct ImageCleanupGuard {
    _lock: File,
}
pub(crate) struct SqlImageCleanup {
    binding: Binding,
}
enum Binding {
    NotApplicable,
    Bound {
        workspace: File,
        root: Option<Directory>,
        bundle: Option<Directory>,
        name: OsString,
        image_path: PathBuf,
    },
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn entry(parent: &File, name: &OsStr) -> io::Result<Option<(u64, u64)>> {
    let name = component(name)?;
    let mut found = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: live parent FD, validated basename, valid stat output pointer.
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            found.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } < 0
    {
        let error = io::Error::last_os_error();
        return if error.kind() == io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error)
        };
    }
    // SAFETY: successful fstatat initialized the complete stat structure.
    let found = unsafe { found.assume_init() };
    Ok(Some((found.st_dev as u64, found.st_ino as u64)))
}
fn absent(parent: &File, name: &OsStr) -> io::Result<()> {
    if entry(parent, name)?.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "image entry appeared after capture",
        ));
    }
    Ok(())
}
pub(crate) fn bind_sql_cleanup(
    root: &Path,
    id: &str,
    lease: &crate::workspace_runtime::WorkspaceLease,
) -> io::Result<SqlImageCleanup> {
    // Published image owners are generated UUIDs; never resolve another ID.
    if uuid::Uuid::parse_str(id).is_err() {
        return Ok(SqlImageCleanup {
            binding: Binding::NotApplicable,
        });
    }
    let workspace = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root)?;
    lease.validate_directory(&workspace)?;
    let name = component(OsStr::new("session-images"))?;
    // SAFETY: live workspace directory FD and fixed validated basename.
    let fd = unsafe {
        libc::openat(
            workspace.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    let images = if fd < 0 {
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::NotFound {
            return Err(error);
        }
        None
    } else {
        // SAFETY: successful openat returned a newly owned FD, closed once by File.
        Some(Directory::verify(
            unsafe { File::from_raw_fd(fd) },
            root.join("session-images"),
        )?)
    };
    let bundle = if let Some(images) = &images {
        match images.child(OsStr::new(id), false) {
            Ok(dir) => Some(dir),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        }
    } else {
        None
    };
    Ok(SqlImageCleanup {
        binding: Binding::Bound {
            workspace,
            root: images,
            bundle,
            name: id.into(),
            image_path: root.join("session-images"),
        },
    })
}
impl SqlImageCleanup {
    pub(crate) fn remove(&self) -> (io::Result<()>, Option<ImageCleanupGuard>) {
        self.remove_with(File::sync_all)
    }
    fn remove_with(
        &self,
        sync: impl Fn(&File) -> io::Result<()>,
    ) -> (io::Result<()>, Option<ImageCleanupGuard>) {
        let Binding::Bound {
            workspace,
            root,
            bundle,
            name,
            image_path,
        } = &self.binding
        else {
            return (Ok(()), None);
        };
        // An absent namespace still needs the existing publication fence.
        // Establish it only after admission; EEXIST refuses to adopt a root
        // created since capture. A publisher winning before lock acquisition is
        // either busy or detected by the target-absence check under the lock.
        let initialized;
        let root = match root {
            Some(root) => root,
            None => {
                initialized = match initialize_root(workspace, image_path) {
                    Ok(root) => root,
                    Err(e) => return (Err(e), None),
                };
                &initialized
            }
        };
        let lock = match root.lock() {
            Ok(lock) => ImageCleanupGuard { _lock: lock },
            Err(e) => return (Err(e), None),
        };
        let result = (|| {
            if let Some(bundle) = bundle {
                let meta = bundle.file.metadata()?;
                match entry(&root.file, name)? {
                    Some(found) if found == (meta.dev(), meta.ino()) => {}
                    None if meta.nlink() == 0 => {
                        sync(&root.file)?;
                        return sync(workspace);
                    }
                    _ => return Err(invalid("captured image bundle was replaced or moved")),
                }
                let entries = names_bounded(&bundle.file)?;
                let marker = OsStr::new("owner.json");
                match bundle.file(marker, libc::O_RDONLY) {
                    Ok(file) => {
                        if file.metadata()?.len() > 1024 {
                            return Err(invalid("image ownership marker is too large"));
                        }
                        let manifest: Manifest = serde_json::from_reader(file.take(1025))
                            .map_err(|_| invalid("invalid image ownership marker"))?;
                        if manifest.owner != "orkworks-image-context-v1"
                            || OsStr::new(&manifest.session) != name.as_os_str()
                        {
                            return Err(invalid("image ownership marker does not match session"));
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::NotFound && entries.is_empty() => {}
                    Err(e) => return Err(e),
                }
                for child in entries.iter().filter(|entry| entry.as_os_str() != marker) {
                    unlink(&bundle.file, child, 0)?;
                }
                // Retain the marker until payload removals are synchronized.
                sync(&bundle.file)?;
                if entries.iter().any(|entry| entry.as_os_str() == marker) {
                    unlink(&bundle.file, marker, 0)?;
                }
                sync(&bundle.file)?;
                let meta = bundle.file.metadata()?;
                if entry(&root.file, name)? != Some((meta.dev(), meta.ino())) {
                    return Err(invalid("image bundle changed during removal"));
                }
                unlink(&root.file, name, libc::AT_REMOVEDIR)?;
            } else {
                absent(&root.file, name)?;
            }
            // Always repeat both parent syncs, including already-absent retries.
            sync(&root.file)?;
            sync(workspace)
        })();
        (result, Some(lock))
    }
}
fn initialize_root(workspace: &File, path: &Path) -> io::Result<Directory> {
    let name = component(OsStr::new("session-images"))?;
    // SAFETY: live workspace directory FD, fixed basename, private mode.
    // Do not adopt an existing entry observed after captured absence.
    if unsafe { libc::mkdirat(workspace.as_raw_fd(), name.as_ptr(), 0o700) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: live directory FD and fixed basename; no-follow excludes aliases.
    let fd = unsafe {
        libc::openat(
            workspace.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful openat returned one newly owned FD.
    Directory::verify(unsafe { File::from_raw_fd(fd) }, path.to_path_buf())
}

fn unlink(parent: &File, name: &OsStr, flags: i32) -> io::Result<()> {
    let name = component(name)?;
    // SAFETY: live directory FD, validated basename; flags0 never follows leaves
    // or recurses, AT_REMOVEDIR is used only for the verified empty bundle.
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), flags) } < 0 {
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::NotFound {
            return Err(e);
        }
    }
    Ok(())
}
fn names_bounded(file: &File) -> io::Result<Vec<OsString>> {
    // SAFETY: fcntl creates a close-on-exec descriptor; fdopendir owns it on success.
    let fd = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        let e = io::Error::last_os_error();
        unsafe { libc::close(fd) };
        return Err(e);
    }
    struct Stream(*mut libc::DIR);
    impl Drop for Stream {
        fn drop(&mut self) {
            unsafe { libc::closedir(self.0) };
        }
    }
    let stream = Stream(stream);
    unsafe { libc::rewinddir(stream.0) };
    let mut names = Vec::new();
    loop {
        // SAFETY: platform errno slot is thread-local; resetting it distinguishes
        // normal EOF from a directory read error before any destructive effect.
        #[cfg(target_os = "macos")]
        unsafe {
            *libc::__error() = 0
        };
        #[cfg(target_os = "linux")]
        unsafe {
            *libc::__errno_location() = 0
        };
        let next = unsafe { libc::readdir(stream.0) };
        if next.is_null() {
            let e = io::Error::last_os_error();
            if e.raw_os_error() != Some(0) {
                return Err(e);
            }
            break;
        }
        // SAFETY: copy each live dirent's name before the next readdir call.
        let name = unsafe { std::ffi::CStr::from_ptr((*next).d_name.as_ptr()) }.to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        if names.len() == super::super::MAX_IMAGES + 1 {
            return Err(invalid("image bundle exceeds accepted entry bound"));
        }
        names.push(OsString::from_vec(name.to_vec()));
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        image_context::{ImageBundle, ValidatedImage},
        workspace_runtime::WorkspaceLease,
    };
    const ID: &str = "11111111-1111-4111-8111-111111111111";
    fn publish(root: &Path, id: &str) {
        let mut bundle = ImageBundle::publish(
            root,
            id,
            vec![ValidatedImage {
                bytes: b"payload".to_vec(),
                extension: "png",
            }],
        )
        .unwrap();
        bundle.accept();
    }
    fn fixture() -> (
        tempfile::TempDir,
        WorkspaceLease,
        std::path::PathBuf,
        SqlImageCleanup,
    ) {
        let root = tempfile::tempdir().unwrap();
        let lease = WorkspaceLease::acquire(root.path()).unwrap();
        publish(root.path(), ID);
        let path = root.path().join("session-images").join(ID);
        let cleanup = bind_sql_cleanup(root.path(), ID, &lease).unwrap();
        (root, lease, path, cleanup)
    }
    #[test]
    fn sql_image_owner_sync_failure_keeps_retryable_marker_and_repeats_parent_syncs() {
        for fail_at in 1..=4 {
            let (root, lease, path, cleanup) = fixture();
            let calls = std::cell::Cell::new(0);
            let (result, guard) = cleanup.remove_with(|file| {
                calls.set(calls.get() + 1);
                if calls.get() == fail_at {
                    Err(io::Error::other("injected sync failure"))
                } else {
                    file.sync_all()
                }
            });
            assert!(result.is_err());
            drop(guard);
            assert!(!path.join("0.png").exists());
            if fail_at == 1 {
                assert!(path.join("owner.json").exists());
            }
            let retry = bind_sql_cleanup(root.path(), ID, &lease).unwrap();
            calls.set(0);
            let (result, guard) = retry.remove_with(|file| {
                calls.set(calls.get() + 1);
                file.sync_all()
            });
            result.unwrap();
            assert_eq!(calls.get(), if fail_at <= 2 { 4 } else { 2 });
            assert!(!path.exists());
            drop(guard);
        }
    }
    #[test]
    fn sql_image_owner_rejects_ambiguous_markers_without_deleting_payloads() {
        for fault in [
            "missing",
            "malformed",
            "foreign-session",
            "foreign-owner",
            "oversized",
            "symlink",
        ] {
            let (_root, _lease, path, cleanup) = fixture();
            let marker = path.join("owner.json");
            let foreign = path.parent().unwrap().join("foreign-marker");
            match fault {
                "missing" => std::fs::remove_file(&marker).unwrap(),
                "malformed" => std::fs::write(&marker, b"not-json").unwrap(),
                "foreign-session" => std::fs::write(
                    &marker,
                    br#"{"owner":"orkworks-image-context-v1","session":"other"}"#,
                )
                .unwrap(),
                "foreign-owner" => std::fs::write(
                    &marker,
                    br#"{"owner":"foreign","session":"11111111-1111-4111-8111-111111111111"}"#,
                )
                .unwrap(),
                "oversized" => std::fs::write(&marker, vec![b'x'; 1025]).unwrap(),
                "symlink" => {
                    std::fs::write(&foreign, b"foreign").unwrap();
                    std::fs::remove_file(&marker).unwrap();
                    std::os::unix::fs::symlink(&foreign, &marker).unwrap();
                }
                _ => unreachable!(),
            }
            let (result, guard) = cleanup.remove();
            assert!(result.is_err(), "{fault}");
            drop(guard);
            assert_eq!(std::fs::read(path.join("0.png")).unwrap(), b"payload");
            if fault == "symlink" {
                assert_eq!(std::fs::read(&foreign).unwrap(), b"foreign");
            }
        }
    }
    #[test]
    fn sql_image_owner_bounds_entries_and_never_recurses() {
        let (root, lease, path, cleanup) = fixture();
        for i in 0..8 {
            std::fs::write(path.join(format!("extra-{i}")), b"extra").unwrap();
        }
        let (result, guard) = cleanup.remove();
        assert!(result.is_err());
        drop(guard);
        assert_eq!(std::fs::read(path.join("0.png")).unwrap(), b"payload");
        assert!(path.join("owner.json").exists());
        for i in 0..8 {
            std::fs::remove_file(path.join(format!("extra-{i}"))).unwrap();
        }
        std::fs::create_dir(path.join("nested")).unwrap();
        std::fs::write(path.join("nested/keep"), b"keep").unwrap();
        let (result, guard) = cleanup.remove();
        assert!(result.is_err());
        drop(guard);
        assert!(path.join("owner.json").exists());
        assert_eq!(std::fs::read(path.join("nested/keep")).unwrap(), b"keep");
        std::fs::remove_dir_all(path.join("nested")).unwrap();
        let (result, _guard) = bind_sql_cleanup(root.path(), ID, &lease).unwrap().remove();
        result.unwrap();
        assert!(!path.exists());
    }
    #[test]
    fn sql_image_owner_unlinks_leaf_symlinks_without_touching_the_target() {
        let (root, _lease, path, cleanup) = fixture();
        let foreign = root.path().join("foreign");
        std::fs::write(&foreign, b"keep").unwrap();
        std::fs::remove_file(path.join("0.png")).unwrap();
        std::os::unix::fs::symlink(&foreign, path.join("0.png")).unwrap();
        let (result, _guard) = cleanup.remove();
        result.unwrap();
        assert_eq!(std::fs::read(foreign).unwrap(), b"keep");
        assert!(!path.exists());
    }
    #[test]
    fn sql_image_owner_pins_parent_but_rejects_replaced_target() {
        for target in ["parent", "bundle"] {
            let (root, _lease, path, cleanup) = fixture();
            let original = root.path().join("original");
            if target == "parent" {
                std::fs::rename(path.parent().unwrap(), &original).unwrap();
            } else {
                std::fs::rename(&path, &original).unwrap();
            }
            publish(root.path(), ID);
            std::fs::write(path.join("0.png"), b"replacement").unwrap();
            let (result, guard) = cleanup.remove();
            drop(guard);
            assert_eq!(std::fs::read(path.join("0.png")).unwrap(), b"replacement");
            if target == "parent" {
                result.unwrap();
                assert!(!original.join(ID).exists());
            } else {
                assert!(result.is_err());
                assert_eq!(std::fs::read(original.join("0.png")).unwrap(), b"payload");
            }
        }
    }
    #[test]
    fn sql_image_owner_absence_refuses_later_bundles_until_recapture() {
        for root_exists in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let lease = WorkspaceLease::acquire(root.path()).unwrap();
            if root_exists {
                publish(root.path(), "22222222-2222-4222-8222-222222222222");
            }
            let cleanup = bind_sql_cleanup(root.path(), ID, &lease).unwrap();
            publish(root.path(), ID);
            let (result, guard) = cleanup.remove();
            assert!(result.is_err());
            drop(guard);
            assert!(root
                .path()
                .join("session-images")
                .join(ID)
                .join("0.png")
                .exists());
            let (result, _guard) = bind_sql_cleanup(root.path(), ID, &lease).unwrap().remove();
            result.unwrap();
        }
    }
    #[test]
    fn sql_image_owner_non_uuid_ids_never_resolve_paths() {
        let root = tempfile::tempdir().unwrap();
        let lease = WorkspaceLease::acquire(root.path()).unwrap();
        for id in ["ordinary-id", "../escape", &"x".repeat(256)] {
            let cleanup = bind_sql_cleanup(root.path(), id, &lease).unwrap();
            let (result, guard) = cleanup.remove();
            result.unwrap();
            assert!(guard.is_none());
        }
        assert!(!root.path().join("session-images").exists());
    }
    #[test]
    fn sql_image_owner_binding_rejects_foreign_lease_and_non_private_or_symlinked_directories() {
        use std::os::unix::fs::PermissionsExt;
        let (root, lease, path, _cleanup) = fixture();
        let foreign = tempfile::tempdir().unwrap();
        let _foreign_lease = WorkspaceLease::acquire(foreign.path()).unwrap();
        publish(foreign.path(), ID);
        assert!(bind_sql_cleanup(foreign.path(), ID, &lease).is_err());
        assert_eq!(
            std::fs::read(foreign.path().join("session-images").join(ID).join("0.png")).unwrap(),
            b"payload"
        );
        std::fs::set_permissions(
            path.parent().unwrap(),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(bind_sql_cleanup(root.path(), ID, &lease).is_err());
        std::fs::set_permissions(
            path.parent().unwrap(),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let original = root.path().join("original-images");
        std::fs::rename(path.parent().unwrap(), &original).unwrap();
        std::os::unix::fs::symlink(
            foreign.path().join("session-images"),
            root.path().join("session-images"),
        )
        .unwrap();
        assert!(bind_sql_cleanup(root.path(), ID, &lease).is_err());
        assert!(original.join(ID).join("0.png").exists());
    }
    #[test]
    fn sql_image_owner_absent_root_holds_publication_fence_until_progress_receipt() {
        let root = tempfile::tempdir().unwrap();
        let lease = WorkspaceLease::acquire(root.path()).unwrap();
        let workspace_ino = std::fs::metadata(root.path()).unwrap().ino();
        let cleanup = bind_sql_cleanup(root.path(), ID, &lease).unwrap();
        let publication_allowed = std::cell::Cell::new(false);
        let (result, guard) = cleanup.remove_with(|file| {
            if file.metadata()?.ino() == workspace_ino {
                let publisher = ImageBundle::publish(
                    root.path(),
                    ID,
                    vec![ValidatedImage {
                        bytes: b"late".to_vec(),
                        extension: "png",
                    }],
                );
                publication_allowed.set(publisher.is_ok());
                drop(publisher);
            }
            file.sync_all()
        });
        result.unwrap();
        assert!(
            !publication_allowed.get(),
            "publisher slipped between captured absence and SQL progress"
        );
        assert!(
            guard.is_some(),
            "absence success must retain the publication fence"
        );
        let publisher = ImageBundle::publish(
            root.path(),
            ID,
            vec![ValidatedImage {
                bytes: b"late".to_vec(),
                extension: "png",
            }],
        );
        assert!(publisher.is_err());
        drop(guard);
        assert!(ImageBundle::publish(
            root.path(),
            ID,
            vec![ValidatedImage {
                bytes: b"later".to_vec(),
                extension: "png"
            }]
        )
        .is_ok());
    }
}
