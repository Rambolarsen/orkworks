//! Owner-only Windows image bundles. The first profile is qualified separately.
mod native;

use self::native::{child, mark_remove, names, rename, Anchor, PrivateAcl};
use super::ValidatedImage;
use crate::workspace_runtime::WorkspaceIdentity;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    ffi::OsStr,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, SYNCHRONIZE,
};

const DIRECTORY: &str = "session-images";
const MANIFEST: &str = "owner.json";
const MANIFEST_OWNER: &str = "orkworks-image-context-v1";
const STAGE_PREFIX: &str = ".staging-";
const MANIFEST_LIMIT: usize = 1024;
const READ: u32 = FILE_GENERIC_READ;
const WRITE: u32 = FILE_GENERIC_READ | FILE_GENERIC_WRITE;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn valid_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|parsed| parsed.to_string() == id)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    owner: String,
    session: String,
}

struct Root {
    directory: File,
    metadata: Anchor,
    acl: PrivateAcl,
}

impl Root {
    fn open(path: &Path, create: bool) -> io::Result<Self> {
        let metadata = Anchor::open(path)?;
        let acl = PrivateAcl::current_user()?;
        let mode = if create { WRITE } else { READ };
        let directory = private_child(
            metadata.directory(),
            OsStr::new(DIRECTORY),
            true,
            mode,
            &acl,
            false,
        );
        let directory = if create && is_missing(&directory) {
            match private_child(
                metadata.directory(),
                OsStr::new(DIRECTORY),
                true,
                mode,
                &acl,
                true,
            ) {
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => private_child(
                    metadata.directory(),
                    OsStr::new(DIRECTORY),
                    true,
                    mode,
                    &acl,
                    false,
                )?,
                other => other?,
            }
        } else {
            directory?
        };
        Ok(Self {
            directory,
            metadata,
            acl,
        })
    }

    fn lock(&self) -> io::Result<File> {
        let name = OsStr::new(".lock");
        let lock = match private_child(&self.directory, name, false, WRITE, &self.acl, true) {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                private_child(&self.directory, name, false, WRITE, &self.acl, false)?
            }
            result => result?,
        };
        fs2::FileExt::try_lock_exclusive(&lock)?;
        Ok(lock)
    }

    fn bundle(&self, name: &OsStr, create: bool) -> io::Result<File> {
        private_child(
            &self.directory,
            name,
            true,
            WRITE | DELETE,
            &self.acl,
            create,
        )
    }

    fn metadata_absent(&self, session: &str) -> bool {
        let attrs = FILE_READ_ATTRIBUTES | SYNCHRONIZE;
        let sessions = match child(
            self.metadata.directory(),
            OsStr::new("sessions"),
            true,
            attrs,
            None,
        ) {
            Ok(sessions) => sessions,
            Err(error) => return error.kind() == io::ErrorKind::NotFound,
        };
        [".json", ".json.corrupt"].into_iter().all(|suffix| {
            let name = format!("{session}{suffix}");
            is_missing(&child(&sessions, OsStr::new(&name), false, attrs, None))
        })
    }
}

fn is_missing<T>(result: &io::Result<T>) -> bool {
    matches!(result, Err(error) if error.kind() == io::ErrorKind::NotFound)
}

fn private_child(
    parent: &File,
    name: &OsStr,
    directory: bool,
    access: u32,
    acl: &PrivateAcl,
    create: bool,
) -> io::Result<File> {
    let file = child(parent, name, directory, access, create.then_some(acl))?;
    acl.verify(&file)?;
    Ok(file)
}

fn read_manifest(directory: &File, acl: &PrivateAcl) -> io::Result<Manifest> {
    let file = private_child(directory, OsStr::new(MANIFEST), false, READ, acl, false)?;
    parse_manifest(&file)
}

fn parse_manifest(file: &File) -> io::Result<Manifest> {
    let mut contents = Vec::new();
    file.take((MANIFEST_LIMIT + 1) as u64)
        .read_to_end(&mut contents)?;
    if contents.len() > MANIFEST_LIMIT {
        return Err(invalid("Image ownership manifest is oversized"));
    }
    let manifest: Manifest = serde_json::from_slice(&contents)?;
    if manifest.owner != MANIFEST_OWNER || !valid_id(&manifest.session) {
        return Err(invalid("Invalid image ownership manifest"));
    }
    Ok(manifest)
}

fn image_name(name: &OsStr) -> bool {
    let Some((slot, extension)) = name.to_str().and_then(|name| name.split_once('.')) else {
        return false;
    };
    slot.len() == 1
        && slot.as_bytes()[0].is_ascii_digit()
        && usize::from(slot.as_bytes()[0] - b'0') < super::super::MAX_IMAGES
        && matches!(extension, "png" | "jpg" | "webp")
}

fn remove_bundle(directory: &File, acl: &PrivateAcl, expected: Option<&str>) -> io::Result<()> {
    // Verify every entry and the retained marker handle before removing any name.
    let mut files = Vec::new();
    let mut manifest = None;
    for name in names(directory)? {
        let marker = name == MANIFEST;
        if !marker && !image_name(&name) {
            return Err(invalid("Unexpected entry in private image bundle"));
        }
        let file = private_child(directory, &name, false, READ | DELETE, acl, false)?;
        if marker {
            manifest = Some(file);
        } else {
            files.push(file);
        }
    }
    if let Some(expected) = expected {
        let marker = manifest
            .as_ref()
            .ok_or_else(|| invalid("Missing image ownership manifest"))?;
        if parse_manifest(marker)?.session != expected {
            return Err(invalid("Image bundle belongs to a different session"));
        }
    }
    for file in files {
        mark_remove(&file)?;
    }
    if let Some(marker) = manifest {
        mark_remove(&marker)?;
    }
    mark_remove(directory)
}

pub(crate) struct ImageBundle {
    directory: File,
    accepted: bool,
    root: Root,
    paths: Vec<PathBuf>,
    _lock: File,
}

impl ImageBundle {
    pub(crate) fn publish(root: &Path, id: &str, images: Vec<ValidatedImage>) -> io::Result<Self> {
        if !valid_id(id) {
            return Err(invalid("Invalid image bundle owner"));
        }
        let store = Root::open(root, true)?;
        let lock = store.lock()?;
        let stage = format!("{STAGE_PREFIX}{}", uuid::Uuid::new_v4());
        let directory = store.bundle(OsStr::new(&stage), true)?;
        let mut bundle = Self {
            directory,
            accepted: false,
            root: store,
            paths: Vec::with_capacity(images.len()),
            _lock: lock,
        };
        let directory = &bundle.directory;
        {
            let mut manifest = private_child(
                directory,
                OsStr::new(MANIFEST),
                false,
                WRITE,
                &bundle.root.acl,
                true,
            )?;
            serde_json::to_writer(
                &mut manifest,
                &Manifest {
                    owner: MANIFEST_OWNER.into(),
                    session: id.into(),
                },
            )?;
            manifest.sync_all()?;
        }
        for (index, image) in images.iter().enumerate() {
            let name = format!("{index}.{}", image.extension);
            let mut file = private_child(
                directory,
                OsStr::new(&name),
                false,
                WRITE,
                &bundle.root.acl,
                true,
            )?;
            file.write_all(&image.bytes)?;
            file.sync_all()?;
            bundle.paths.push(root.join(DIRECTORY).join(id).join(name));
        }
        // The staging handle was opened write-through. The rename is within
        // the same verified NTFS directory and cannot replace another bundle.
        rename(directory, OsStr::new(id))?;
        Ok(bundle)
    }

    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub(crate) fn verify_dispatch_paths(&self) -> io::Result<Vec<File>> {
        let mut pins = Vec::with_capacity(self.paths.len());
        for path in &self.paths {
            let name = path
                .file_name()
                .ok_or_else(|| invalid("Missing image file name"))?;
            let owned = private_child(&self.directory, name, false, READ, &self.root.acl, false)?;
            pins.push(native::verify_dispatch_path(path, &owned)?);
        }
        Ok(pins)
    }

    pub(crate) fn accept(&mut self) {
        self.accepted = true;
    }
}

impl Drop for ImageBundle {
    fn drop(&mut self) {
        if !self.accepted {
            if let Err(error) = remove_bundle(&self.directory, &self.root.acl, None) {
                tracing::warn!(%error, "Image bundle rollback deferred");
            }
        }
    }
}

pub(crate) struct ImageCleanup(CleanupTarget);

enum CleanupTarget {
    Bundle {
        root: Root,
        session: String,
        directory: RefCell<Option<File>>,
        _lock: File,
    },
    AbsentStore(WorkspaceIdentity),
}

fn verify_store_absent(metadata: &WorkspaceIdentity) -> io::Result<()> {
    metadata.revalidate()?;
    // This lookup observes only the image-store entry relative to the retained
    // metadata directory. It never follows the entry or opens a bundle.
    let store = child(
        metadata.directory(),
        OsStr::new(DIRECTORY),
        true,
        FILE_READ_ATTRIBUTES | SYNCHRONIZE,
        None,
    );
    match store {
        Err(error) if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32) => {
            metadata.revalidate()
        }
        Err(error) => Err(error),
        Ok(_) => Err(invalid("Image store is present on an ineligible root")),
    }
}

impl ImageCleanup {
    pub(crate) fn remove(&self) -> io::Result<()> {
        match &self.0 {
            CleanupTarget::Bundle {
                root,
                session,
                directory,
                ..
            } => {
                let mut directory = directory.borrow_mut();
                if let Some(handle) = directory.as_ref() {
                    remove_bundle(handle, &root.acl, Some(session))?;
                    directory.take();
                }
                Ok(())
            }
            CleanupTarget::AbsentStore(metadata) => verify_store_absent(metadata),
        }
    }

    pub(crate) fn remove_and_delete_record(
        &self,
        metadata: &crate::metadata::MetadataStore,
        id: &str,
    ) -> io::Result<()> {
        let CleanupTarget::AbsentStore(root) = &self.0 else {
            self.remove()?;
            return metadata.delete_session(id);
        };
        metadata.delete_session_using(id, |paths| {
            let sessions = child(
                root.directory(),
                OsStr::new("sessions"),
                true,
                FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                None,
            )?;
            let mut records = Vec::with_capacity(paths.len());
            for path in &paths {
                let name = path
                    .file_name()
                    .ok_or_else(|| invalid("Missing session record name"))?;
                match child(
                    &sessions,
                    name,
                    false,
                    DELETE | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                    None,
                ) {
                    Ok(record) => records.push(record),
                    Err(error) if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32) => {}
                    Err(error) => return Err(error),
                }
            }
            verify_store_absent(root)?;
            // Record handles are relative to the exact directory checked above.
            for record in records {
                native::mark_record_remove(&record)?;
            }
            Ok(())
        })
    }
}

pub(crate) fn prepare_cleanup(root: &Path, id: &str) -> io::Result<Option<ImageCleanup>> {
    if !valid_id(id) {
        return Err(invalid("Invalid image bundle owner"));
    }
    let root = match Root::open(root, false) {
        Ok(root) => root,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            let absence = WorkspaceIdentity::resolve(root).and_then(|metadata| {
                verify_store_absent(&metadata)?;
                Ok(metadata)
            });
            return match absence {
                Ok(metadata) => Ok(Some(ImageCleanup(CleanupTarget::AbsentStore(metadata)))),
                Err(_) => Err(error),
            };
        }
    };
    let directory = root.bundle(OsStr::new(id), false);
    if is_missing(&directory) {
        return Ok(None);
    }
    let directory = directory?;
    let lock = root.lock()?;
    Ok(Some(ImageCleanup(CleanupTarget::Bundle {
        root,
        session: id.into(),
        directory: RefCell::new(Some(directory)),
        _lock: lock,
    })))
}

pub(crate) fn recover(root: &Path, metadata: &crate::metadata::MetadataStore) -> io::Result<()> {
    if root != metadata.root_path() {
        return Err(invalid("Image recovery metadata root mismatch"));
    }
    let root = Root::open(root, false);
    if is_missing(&root) {
        return Ok(());
    }
    let root = root?;
    let _lock = root.lock()?;
    for name in names(&root.directory)? {
        let Some(text) = name.to_str() else {
            continue;
        };
        let staging = text.strip_prefix(STAGE_PREFIX).is_some_and(valid_id);
        if !staging && !valid_id(text) {
            continue;
        }
        let Ok(directory) = root.bundle(&name, false) else {
            continue;
        };
        let Ok(manifest) = read_manifest(&directory, &root.acl) else {
            continue;
        };
        if (staging || manifest.session == text) && root.metadata_absent(&manifest.session) {
            if let Err(error) = remove_bundle(&directory, &root.acl, Some(&manifest.session)) {
                tracing::warn!(%error, "Image recovery left an ambiguous bundle intact");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SESSION: &str = "33333333-3333-4333-8333-333333333333";
    const RETAINED: &str = "44444444-4444-4444-8444-444444444444";
    const CHILD_RETAINED: &str = "ORKWORKS_TEST_WINDOWS_IMAGE_CRASH_RETAINED";
    const CHILD_ROOT: &str = "ORKWORKS_TEST_WINDOWS_IMAGE_CRASH_ROOT";

    fn fixture() -> Vec<ValidatedImage> {
        vec![ValidatedImage {
            bytes: b"validated storage fixture".to_vec(),
            extension: "png",
        }]
    }

    #[test]
    fn refuses_an_inherited_acl_and_a_reparse_root() {
        let root = tempfile::tempdir().unwrap();
        let image_root = root.path().join(DIRECTORY);
        fs::create_dir(&image_root).unwrap();
        assert!(ImageBundle::publish(root.path(), SESSION, fixture()).is_err());
        assert_eq!(fs::read_dir(&image_root).unwrap().count(), 0);
        fs::remove_dir(&image_root).unwrap();

        let outside = tempfile::tempdir().unwrap();
        let link = std::os::windows::fs::symlink_dir(outside.path(), &image_root);
        if link
            .as_ref()
            .is_err_and(|error| error.raw_os_error() == Some(1314))
        {
            eprintln!("Symlink creation unavailable on this Windows host (1314)");
            return;
        }
        link.unwrap();
        assert!(ImageBundle::publish(root.path(), SESSION, fixture()).is_err());
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
        fs::remove_dir(&image_root).unwrap();
    }

    #[test]
    fn absence_guard_preserves_records_if_the_store_appears_or_the_alias_changes() {
        let parent = tempfile::tempdir().unwrap();
        let original = parent.path().join("original");
        let other = parent.path().join("other");
        let alias = parent.path().join("alias");
        let record = PathBuf::from("sessions").join(format!("{SESSION}.json"));
        for root in [&original, &other] {
            fs::create_dir_all(root.join("sessions")).unwrap();
            fs::write(root.join(&record), b"synthetic record").unwrap();
        }

        let link = std::os::windows::fs::symlink_dir(&original, &alias);
        if link
            .as_ref()
            .is_err_and(|error| error.raw_os_error() == Some(1314))
        {
            eprintln!("Symlink creation unavailable on this Windows host (1314)");
            return;
        }
        link.unwrap();

        let metadata = crate::metadata::MetadataStore::new(&alias);
        let cleanup = prepare_cleanup(&alias, SESSION).unwrap().unwrap();
        let image_store = original.join(DIRECTORY);
        fs::create_dir(&image_store).unwrap();
        assert!(cleanup
            .remove_and_delete_record(&metadata, SESSION)
            .is_err());
        assert!(original.join(&record).is_file());
        fs::remove_dir(&image_store).unwrap();

        fs::remove_dir(&alias).unwrap();
        std::os::windows::fs::symlink_dir(&other, &alias).unwrap();
        assert!(cleanup
            .remove_and_delete_record(&metadata, SESSION)
            .is_err());
        assert!(original.join(&record).is_file());
        assert!(other.join(&record).is_file());
    }

    #[test]
    fn refuses_to_remove_a_hard_link_and_keeps_cleanup_retryable() {
        let root = tempfile::tempdir().unwrap();
        let mut bundle = ImageBundle::publish(root.path(), SESSION, fixture()).unwrap();
        let original = bundle.paths()[0].clone();
        bundle.accept();
        drop(bundle);
        let other = root.path().join("hard-link.png");
        fs::hard_link(&original, &other).unwrap();

        let cleanup = prepare_cleanup(root.path(), SESSION).unwrap().unwrap();
        assert!(cleanup.remove().is_err());
        assert_eq!(fs::read(&other).unwrap(), b"validated storage fixture");
        assert!(original.with_file_name(MANIFEST).is_file());
        fs::remove_file(&other).unwrap();
        cleanup.remove().unwrap();
        assert!(!original.exists());
    }

    #[test]
    fn unlinks_verified_files_before_the_manifest_with_a_share_delete_reader() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ};
        let root = tempfile::tempdir().unwrap();
        let mut bundle = ImageBundle::publish(root.path(), SESSION, fixture()).unwrap();
        let path = bundle.paths()[0].clone();
        bundle.accept();
        drop(bundle);
        let mut reader = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
            .open(&path)
            .unwrap();

        let cleanup = prepare_cleanup(root.path(), SESSION).unwrap().unwrap();
        cleanup.remove().unwrap();
        assert!(!path.parent().unwrap().exists());
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"validated storage fixture");
    }

    #[test]
    fn refuses_a_replaced_dispatch_path_and_an_unmarked_cleanup_candidate() {
        let root = tempfile::tempdir().unwrap();
        let mut bundle = ImageBundle::publish(root.path(), SESSION, fixture()).unwrap();
        assert!(bundle.verify_dispatch_paths().is_ok());
        let outside = tempfile::tempdir().unwrap();
        let replacement = outside.path().join("0.png");
        fs::write(&replacement, b"unrelated").unwrap();
        bundle.paths[0] = replacement.clone();
        assert!(bundle.verify_dispatch_paths().is_err());
        drop(bundle);
        assert_eq!(fs::read(replacement).unwrap(), b"unrelated");

        let store = Root::open(root.path(), false).unwrap();
        drop(store.bundle(OsStr::new(SESSION), true).unwrap());
        let cleanup = prepare_cleanup(root.path(), SESSION).unwrap().unwrap();
        assert!(cleanup.remove().is_err());
        assert!(root.path().join(DIRECTORY).join(SESSION).is_dir());
    }

    #[test]
    #[ignore = "subprocess helper for the synthetic Windows recovery test"]
    fn windows_image_recovery_child() {
        let root = PathBuf::from(std::env::var_os(CHILD_ROOT).expect("recovery fixture root"));
        let id = if std::env::var_os(CHILD_RETAINED).is_some() {
            RETAINED
        } else {
            SESSION
        };
        let bundle = ImageBundle::publish(&root, id, fixture()).unwrap();
        if id == RETAINED {
            let metadata = crate::metadata::MetadataStore::new(&root);
            fs::create_dir_all(metadata.sessions_dir()).unwrap();
            fs::write(
                metadata.sessions_dir().join(format!("{id}.json.corrupt")),
                b"retained",
            )
            .unwrap();
        }
        assert!(bundle.paths()[0].is_file());
        // Exit without running the bundle destructors, as after a sidecar crash.
        std::process::exit(0);
    }

    #[test]
    fn recovery_removes_abandoned_bundle_and_keeps_corrupt_metadata_ownership() {
        let root = tempfile::tempdir().unwrap();
        for retain in [false, true] {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap());
            child
                .args([
                    "--ignored",
                    "--exact",
                    "image_context::storage::windows::tests::windows_image_recovery_child",
                ])
                .env(CHILD_ROOT, root.path())
                .env_remove(CHILD_RETAINED);
            if retain {
                child.env(CHILD_RETAINED, "1");
            }
            let output = child.output().unwrap();
            assert!(
                output.status.success(),
                "recovery setup failed: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let storage = root.path().join(DIRECTORY);
        let orphan = storage.join(SESSION).join("0.png");
        let retained = storage.join(RETAINED).join("0.png");
        assert!(orphan.is_file() && retained.is_file());
        recover(
            root.path(),
            &crate::metadata::MetadataStore::new(root.path()),
        )
        .unwrap();
        assert!(!orphan.exists());
        assert!(retained.is_file());
    }
}
