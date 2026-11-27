//! Owner-only Windows image bundles. The first profile is qualified separately.
use crate::windows_fs as native;

use self::native::{child, file_identity, mark_remove, names, rename, Anchor, PrivateAcl};
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
const CLEANUP_PREFIX: &str = ".cleanup-";
const CLEANUP_DRAFT_PREFIX: &str = ".cleanup-draft-";
const CLEANUP_OWNER: &str = "orkworks-image-cleanup-v1";
const RECORD_LIMIT: usize = 1024;
const READ: u32 = FILE_GENERIC_READ;
const WRITE: u32 = FILE_GENERIC_READ | FILE_GENERIC_WRITE;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn valid_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|parsed| parsed.to_string() == id)
}

fn bundle_name(name: &str) -> bool {
    valid_id(name) || name.strip_prefix(STAGE_PREFIX).is_some_and(valid_id)
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

fn optional_child<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Err(error) if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32) => Ok(None),
        result => result.map(Some),
    }
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

fn read_record<T: serde::de::DeserializeOwned>(file: &File) -> io::Result<T> {
    let mut contents = Vec::new();
    file.take((RECORD_LIMIT + 1) as u64)
        .read_to_end(&mut contents)?;
    if contents.len() > RECORD_LIMIT {
        return Err(invalid("Image ownership record is oversized"));
    }
    Ok(serde_json::from_slice(&contents)?)
}

fn parse_manifest(file: &File) -> io::Result<Manifest> {
    let manifest: Manifest = read_record(file)?;
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CleanupReceipt {
    owner: String,
    bundle: String,
    session: String,
    volume: u32,
    file: u64,
}

fn read_receipt(
    root: &Root,
    name: &str,
    receipt_name: &OsStr,
) -> io::Result<Option<(File, CleanupReceipt)>> {
    let Some(file) = optional_child(private_child(
        &root.directory,
        receipt_name,
        false,
        WRITE | DELETE,
        &root.acl,
        false,
    ))?
    else {
        return Ok(None);
    };
    let receipt: CleanupReceipt = read_record(&file)?;
    if receipt.owner != CLEANUP_OWNER
        || receipt.bundle != name
        || !valid_id(&receipt.session)
        || (valid_id(name) && receipt.session != name)
    {
        return Err(invalid("Invalid image cleanup receipt"));
    }
    Ok(Some((file, receipt)))
}

fn create_receipt(root: &Root, receipt_name: &OsStr, receipt: &CleanupReceipt) -> io::Result<File> {
    // Only called under the store lock while the matching owner marker still
    // exists. A previous partial draft never grants deletion authority.
    let draft_name = format!("{CLEANUP_DRAFT_PREFIX}{}", receipt.bundle);
    let draft_name = OsStr::new(&draft_name);
    if let Some(interrupted) = optional_child(private_child(
        &root.directory,
        draft_name,
        false,
        WRITE | DELETE,
        &root.acl,
        false,
    ))? {
        mark_remove(&interrupted)?;
    }
    let mut file = private_child(
        &root.directory,
        draft_name,
        false,
        WRITE | DELETE,
        &root.acl,
        true,
    )?;
    serde_json::to_writer(&mut file, receipt)?;
    file.sync_all()?;
    rename(&file, receipt_name)?;
    Ok(file)
}

fn remove_bundle(
    root: &Root,
    name: &str,
    directory: &mut Option<File>,
    requested: Option<&str>,
) -> io::Result<()> {
    if !bundle_name(name) {
        return Err(invalid("Invalid image cleanup bundle name"));
    }
    let receipt_name = format!("{CLEANUP_PREFIX}{name}");
    let receipt_name = OsStr::new(&receipt_name);
    let mut receipt = read_receipt(root, name, receipt_name)?;
    let marker = match directory {
        Some(directory) => optional_child(private_child(
            directory,
            OsStr::new(MANIFEST),
            false,
            READ | DELETE,
            &root.acl,
            false,
        ))?,
        None => None,
    };
    let manifest = marker.as_ref().map(parse_manifest).transpose()?;
    let session = manifest
        .as_ref()
        .map(|manifest| manifest.session.as_str())
        .or_else(|| {
            receipt
                .as_ref()
                .map(|(_, receipt)| receipt.session.as_str())
        });
    if let Some(session) = session {
        if requested.is_some_and(|expected| expected != session)
            || (valid_id(name) && name != session)
            || receipt
                .as_ref()
                .is_some_and(|(_, receipt)| receipt.session != session)
        {
            return Err(invalid("Image bundle belongs to a different session"));
        }
        if requested.is_none() && !root.metadata_absent(session) {
            return Ok(());
        }
    }
    if let Some(handle) = directory.as_ref() {
        if session.is_none() {
            return Err(invalid(
                "Missing image ownership manifest and cleanup receipt",
            ));
        }
        let (volume, file) = file_identity(handle)?;
        if receipt
            .as_ref()
            .is_some_and(|(_, receipt)| (receipt.volume, receipt.file) != (volume, file))
        {
            return Err(invalid(
                "Image cleanup receipt refers to a different directory",
            ));
        }
        if let (Some(marker), Some(manifest)) = (marker, manifest) {
            // Verify every leaf before removing any of them. The manifest stays
            // present until the final leaf is gone and the receipt is flushed.
            let mut files = Vec::new();
            for entry in names(handle)? {
                if entry == MANIFEST {
                    continue;
                }
                if !image_name(&entry) {
                    return Err(invalid("Unexpected entry in private image bundle"));
                }
                files.push(private_child(
                    handle,
                    &entry,
                    false,
                    READ | DELETE,
                    &root.acl,
                    false,
                )?);
            }
            for file in files {
                mark_remove(&file)?;
            }
            let remaining = names(handle)?;
            if remaining.len() != 1 || remaining[0] != MANIFEST {
                return Err(invalid("Private image bundle changed during cleanup"));
            }
            match &receipt {
                Some((file, _)) => file.sync_all()?,
                None => {
                    let data = CleanupReceipt {
                        owner: CLEANUP_OWNER.into(),
                        bundle: name.into(),
                        session: manifest.session,
                        volume,
                        file,
                    };
                    receipt = Some((create_receipt(root, receipt_name, &data)?, data));
                }
            }
            mark_remove(&marker)?;
            drop(marker);
        }
        // A receipt alone only authorizes removal of an empty directory.
        // Windows checks emptiness again when setting the disposition.
        if !names(handle)?.is_empty() {
            return Err(invalid("Image cleanup requires an empty directory"));
        }
        mark_remove(handle)?;
        drop(directory.take());
    }
    let lookup = child(
        &root.directory,
        OsStr::new(name),
        true,
        FILE_READ_ATTRIBUTES | SYNCHRONIZE,
        None,
    );
    if optional_child(lookup)?.is_some() {
        return Err(invalid("Image bundle remains after cleanup"));
    }
    if let Some((file, _)) = receipt {
        mark_remove(&file)?;
    }
    Ok(())
}

pub(crate) fn supports_windows_images(root: &Path) -> bool {
    let store = Root::open(root, false);
    store.is_ok() || (is_missing(&store) && Anchor::open(root).is_ok())
}

pub(crate) struct ImageBundle {
    directory: Option<File>,
    name: String,
    session: String,
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
            directory: Some(directory),
            name: stage,
            session: id.into(),
            accepted: false,
            root: store,
            paths: Vec::with_capacity(images.len()),
            _lock: lock,
        };
        let directory = bundle.directory.as_ref().expect("new bundle directory");
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
        bundle.name = id.into();
        Ok(bundle)
    }

    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub(crate) fn verify_dispatch_paths(&self) -> io::Result<Vec<File>> {
        let directory = self.directory.as_ref().expect("published bundle directory");
        let mut pins = Vec::with_capacity(self.paths.len());
        for path in &self.paths {
            let name = path
                .file_name()
                .ok_or_else(|| invalid("Missing image file name"))?;
            let owned = private_child(directory, name, false, READ, &self.root.acl, false)?;
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
            if let Err(error) = remove_bundle(
                &self.root,
                &self.name,
                &mut self.directory,
                Some(&self.session),
            ) {
                tracing::warn!(%error, "Image bundle rollback deferred");
            }
        }
    }
}

pub(crate) struct ImageCleanup(CleanupTarget);

enum CleanupTarget {
    Bundle {
        root: Root,
        name: String,
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
                name,
                session,
                directory,
                ..
            } => remove_bundle(root, name, &mut directory.borrow_mut(), Some(session)),
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
    // Old text sessions can use another UUID spelling. Check the canonical
    // image name for collisions, then keep the original for owner comparison.
    let name = uuid::Uuid::parse_str(id)
        .map_err(|_| invalid("Invalid image bundle owner"))?
        .to_string();
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
    let lock = root.lock()?;
    let directory = optional_child(root.bundle(OsStr::new(&name), false))?;
    if name != id {
        let receipt_name = format!("{CLEANUP_PREFIX}{name}");
        if directory.is_some() || read_receipt(&root, &name, OsStr::new(&receipt_name))?.is_some() {
            return Err(invalid("Image bundle belongs to a different session"));
        }
    }
    Ok(Some(ImageCleanup(CleanupTarget::Bundle {
        root,
        name,
        session: id.into(),
        directory: RefCell::new(directory),
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
    let entries = names(&root.directory)?;
    let candidates: std::collections::BTreeSet<_> = entries
        .iter()
        .filter_map(|name| name.to_str())
        .map(|name| name.strip_prefix(CLEANUP_PREFIX).unwrap_or(name))
        .filter(|name| bundle_name(name))
        .collect();
    for name in candidates {
        let result = optional_child(root.bundle(OsStr::new(name), false))
            .and_then(|mut directory| remove_bundle(&root, name, &mut directory, None));
        if let Err(error) = result {
            tracing::warn!(%error, "Image recovery left an ambiguous bundle intact");
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
    const COMMITTED: &str = "55555555-5555-4555-8555-555555555555";
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
        assert!(supports_windows_images(root.path()));
        assert!(!image_root.exists());
        fs::create_dir(&image_root).unwrap();
        assert!(!supports_windows_images(root.path()));
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

    fn interrupt_cleanup(root: &Path, session: &str) -> (PathBuf, PathBuf) {
        let mut bundle = ImageBundle::publish(root, session, fixture()).unwrap();
        let image = bundle.paths()[0].clone();
        let directory = image.parent().unwrap().to_path_buf();
        bundle.accept();
        drop(bundle);
        let store = Root::open(root, false).unwrap();
        let _lock = store.lock().unwrap();
        let handle = store.bundle(OsStr::new(session), false).unwrap();
        let (volume, file) = file_identity(&handle).unwrap();
        let record_name = format!("{CLEANUP_PREFIX}{session}");
        fs::remove_file(image).unwrap();
        let _record = create_receipt(
            &store,
            OsStr::new(&record_name),
            &CleanupReceipt {
                owner: CLEANUP_OWNER.into(),
                bundle: session.into(),
                session: session.into(),
                volume,
                file,
            },
        )
        .unwrap();
        fs::remove_file(directory.join(MANIFEST)).unwrap();
        (directory, root.join(DIRECTORY).join(record_name))
    }

    #[test]
    fn cleanup_retries_after_the_manifest_was_unlinked() {
        let root = tempfile::tempdir().unwrap();
        let (directory, record) = interrupt_cleanup(root.path(), SESSION);
        let cleanup = prepare_cleanup(root.path(), SESSION).unwrap().unwrap();
        cleanup.remove().unwrap();
        assert!(!directory.exists());
        assert!(!record.exists());
    }

    #[test]
    fn cleanup_checks_absence_under_the_store_lock() {
        let root = tempfile::tempdir().unwrap();
        let mut writer = ImageBundle::publish(root.path(), SESSION, fixture()).unwrap();
        assert!(prepare_cleanup(root.path(), RETAINED).is_err());
        writer.accept();
        drop(writer);
        let cleanup = prepare_cleanup(root.path(), RETAINED).unwrap().unwrap();
        assert!(ImageBundle::publish(root.path(), RETAINED, fixture()).is_err());
        cleanup.remove().unwrap();
    }

    #[test]
    fn legacy_uuid_can_be_cleaned_without_claiming_a_canonical_bundle() {
        let root = tempfile::tempdir().unwrap();
        let legacy = SESSION.replace('-', "");
        let mut bundle = ImageBundle::publish(root.path(), SESSION, fixture()).unwrap();
        let image = bundle.paths()[0].clone();
        bundle.accept();
        drop(bundle);

        assert!(prepare_cleanup(root.path(), &legacy).is_err());
        assert!(image.is_file());

        let cleanup = prepare_cleanup(root.path(), SESSION).unwrap().unwrap();
        cleanup.remove().unwrap();
        drop(cleanup);

        let cleanup = prepare_cleanup(root.path(), &legacy).unwrap().unwrap();
        cleanup.remove().unwrap();
    }

    #[test]
    fn cleanup_retries_an_interrupted_receipt_draft() {
        let root = tempfile::tempdir().unwrap();
        let mut bundle = ImageBundle::publish(root.path(), SESSION, fixture()).unwrap();
        let image = bundle.paths()[0].clone();
        bundle.accept();
        drop(bundle);

        let store = Root::open(root.path(), false).unwrap();
        let draft_name = format!("{CLEANUP_DRAFT_PREFIX}{SESSION}");
        let mut draft = private_child(
            &store.directory,
            OsStr::new(&draft_name),
            false,
            WRITE | DELETE,
            &store.acl,
            true,
        )
        .unwrap();
        draft.write_all(b"{").unwrap();
        draft.sync_all().unwrap();
        drop(draft);
        drop(store);

        let draft_path = root.path().join(DIRECTORY).join(draft_name);
        let metadata = crate::metadata::MetadataStore::new(root.path());
        recover(root.path(), &metadata).unwrap();
        assert!(!draft_path.exists());
        assert!(!image.parent().unwrap().exists());
    }

    #[test]
    fn cleanup_flushes_the_receipt_before_attempting_manifest_removal() {
        let root = tempfile::tempdir().unwrap();
        let mut bundle = ImageBundle::publish(root.path(), SESSION, fixture()).unwrap();
        let image = bundle.paths()[0].clone();
        let marker = image.with_file_name(MANIFEST);
        let receipt = root
            .path()
            .join(DIRECTORY)
            .join(format!("{CLEANUP_PREFIX}{SESSION}"));
        bundle.accept();
        drop(bundle);

        let mut permissions = fs::metadata(&marker).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&marker, permissions.clone()).unwrap();
        let cleanup = prepare_cleanup(root.path(), SESSION).unwrap().unwrap();
        let result = cleanup.remove();
        permissions.set_readonly(false);
        fs::set_permissions(&marker, permissions).unwrap();
        assert!(result.is_err());
        assert!(marker.is_file() && receipt.is_file());
        assert!(!image.exists());

        cleanup.remove().unwrap();
        assert!(!marker.exists() && !receipt.exists());
    }

    #[test]
    fn cleanup_keeps_the_receipt_and_late_entry_until_the_directory_is_empty() {
        let root = tempfile::tempdir().unwrap();
        let (directory, record) = interrupt_cleanup(root.path(), SESSION);
        let cleanup = prepare_cleanup(root.path(), SESSION).unwrap().unwrap();
        let late = directory.join("late-entry");
        fs::write(&late, b"keep").unwrap();
        assert!(cleanup.remove().is_err());
        assert_eq!(fs::read(&late).unwrap(), b"keep");
        assert!(record.is_file());
        fs::remove_file(late).unwrap();
        cleanup.remove().unwrap();
        assert!(!directory.exists());
        assert!(!record.exists());
    }

    #[test]
    fn cleanup_refuses_a_receipt_for_a_replaced_directory() {
        let root = tempfile::tempdir().unwrap();
        let (directory, record) = interrupt_cleanup(root.path(), SESSION);
        let moved = directory.with_file_name("moved");
        fs::rename(&directory, &moved).unwrap();
        let store = Root::open(root.path(), false).unwrap();
        drop(store.bundle(OsStr::new(SESSION), true).unwrap());
        drop(store);
        let cleanup = prepare_cleanup(root.path(), SESSION).unwrap().unwrap();
        assert!(cleanup.remove().is_err());
        assert!(directory.is_dir() && moved.is_dir() && record.is_file());
    }

    #[test]
    fn cleanup_refuses_an_oversized_receipt() {
        let root = tempfile::tempdir().unwrap();
        let (directory, record) = interrupt_cleanup(root.path(), SESSION);
        fs::write(&record, vec![b' '; RECORD_LIMIT + 1]).unwrap();
        let cleanup = prepare_cleanup(root.path(), SESSION).unwrap().unwrap();
        assert!(cleanup.remove().is_err());
        assert!(directory.is_dir() && record.is_file());
    }

    #[test]
    fn recovery_clears_a_receipt_after_directory_removal() {
        let root = tempfile::tempdir().unwrap();
        let (directory, record) = interrupt_cleanup(root.path(), SESSION);
        fs::remove_dir(directory).unwrap();
        let metadata = crate::metadata::MetadataStore::new(root.path());
        recover(root.path(), &metadata).unwrap();
        assert!(!record.exists());
    }

    #[test]
    fn recovery_retries_receipt_only_orphans_and_keeps_retained_sessions() {
        let root = tempfile::tempdir().unwrap();
        let (orphan, orphan_receipt) = interrupt_cleanup(root.path(), SESSION);
        let (unreadable, unreadable_receipt) = interrupt_cleanup(root.path(), RETAINED);
        let (committed, committed_receipt) = interrupt_cleanup(root.path(), COMMITTED);
        let metadata = crate::metadata::MetadataStore::new(root.path());
        fs::create_dir_all(metadata.sessions_dir()).unwrap();
        fs::write(
            metadata
                .sessions_dir()
                .join(format!("{RETAINED}.json.corrupt")),
            b"retained",
        )
        .unwrap();
        let record = crate::test_support::test_session_metadata(
            COMMITTED,
            "Image publication fixture",
            root.path().to_str().unwrap(),
            "ended",
            "2026-11-27T00:00:00Z",
            "2026-11-27T00:00:00Z",
        );
        metadata.try_write_session(&record).unwrap();

        recover(root.path(), &metadata).unwrap();
        assert!(!orphan.exists() && !orphan_receipt.exists());
        assert!(unreadable.is_dir() && unreadable_receipt.is_file());
        assert!(committed.is_dir() && committed_receipt.is_file());
    }

    #[test]
    fn rollback_retires_a_valid_staged_bundle_after_a_rename_collision() {
        let root = tempfile::tempdir().unwrap();
        let mut bundle = ImageBundle::publish(root.path(), SESSION, fixture()).unwrap();
        bundle.accept();
        drop(bundle);
        assert!(ImageBundle::publish(root.path(), SESSION, fixture()).is_err());
        let store = Root::open(root.path(), false).unwrap();
        let children = names(&store.directory).unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|name| name == SESSION));
        assert!(children.iter().any(|name| name == ".lock"));
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
        let id = match std::env::var(CHILD_RETAINED).unwrap_or_default().as_str() {
            "" => SESSION,
            "corrupt" => RETAINED,
            "committed" => COMMITTED,
            mode => panic!("unexpected recovery fixture mode {mode}"),
        };
        let mut bundle = ImageBundle::publish(&root, id, fixture()).unwrap();
        if id == RETAINED {
            let metadata = crate::metadata::MetadataStore::new(&root);
            fs::create_dir_all(metadata.sessions_dir()).unwrap();
            fs::write(
                metadata.sessions_dir().join(format!("{id}.json.corrupt")),
                b"retained",
            )
            .unwrap();
        } else if id == COMMITTED {
            let metadata = crate::metadata::MetadataStore::new(&root);
            let record = crate::test_support::test_session_metadata(
                id,
                "Image publication fixture",
                root.to_str().unwrap(),
                "ended",
                "2026-11-27T00:00:00Z",
                "2026-11-27T00:00:00Z",
            );
            metadata.try_write_session(&record).unwrap();
            bundle.accept();
        }
        assert!(bundle.paths()[0].is_file());
        // Exit without running the bundle destructors, as after a sidecar crash.
        std::process::exit(0);
    }

    #[test]
    fn recovery_removes_abandoned_bundles_and_keeps_persisted_owners() {
        let root = tempfile::tempdir().unwrap();
        for mode in ["", "corrupt", "committed"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "image_context::storage::windows::tests::windows_image_recovery_child",
                ])
                .env(CHILD_ROOT, root.path())
                .env(CHILD_RETAINED, mode)
                .output()
                .unwrap();
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
        let committed = storage.join(COMMITTED).join("0.png");
        assert!(orphan.is_file() && retained.is_file() && committed.is_file());
        let metadata = crate::metadata::MetadataStore::new(root.path());
        recover(root.path(), &metadata).unwrap();
        assert!(!orphan.exists());
        assert!(retained.is_file());
        assert_eq!(fs::read(committed).unwrap(), fixture()[0].bytes);
        assert_eq!(metadata.read_session(COMMITTED).unwrap().id, COMMITTED);
    }
}
