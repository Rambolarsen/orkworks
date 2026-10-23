//! Unix storage operations are relative to retained directory handles.
use super::ValidatedImage;
use std::path::{Path, PathBuf};

#[cfg(unix)]
mod unix {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::{
        ffi::{CString, OsStr, OsString},
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        os::unix::{
            ffi::{OsStrExt, OsStringExt},
            fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
            io::{AsRawFd, FromRawFd},
        },
    };
    fn component(name: &OsStr) -> std::io::Result<CString> {
        let bytes = name.as_bytes();
        if bytes == b"." || bytes == b".." || bytes.contains(&b'/') {
            return Err(std::io::Error::other("Invalid image component"));
        }
        CString::new(bytes).map_err(|_| std::io::Error::other("Invalid image component"))
    }
    struct Directory {
        file: File,
        path: PathBuf,
    }
    impl Directory {
        fn verify(file: File, path: PathBuf) -> std::io::Result<Self> {
            let meta = file.metadata()?;
            // SAFETY: geteuid has no arguments or memory preconditions.
            if !meta.is_dir()
                || meta.mode() & 0o077 != 0
                || meta.uid() != unsafe { libc::geteuid() }
            {
                return Err(std::io::Error::other("Image directory is not private"));
            }
            Ok(Self { file, path })
        }
        fn root(root: &Path) -> std::io::Result<Self> {
            let path = root.join("session-images");
            match fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&path)?;
            Self::verify(file, path)
        }
        fn file(&self, name: &OsStr, flags: i32) -> std::io::Result<File> {
            let name = component(name)?;
            // SAFETY: retained directory fd and NUL-terminated single component;
            // on success ownership of the newly returned descriptor transfers to File.
            let fd = unsafe {
                libc::openat(
                    self.file.as_raw_fd(),
                    name.as_ptr(),
                    flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let file = unsafe { File::from_raw_fd(fd) };
            if flags & libc::O_DIRECTORY == 0 && !file.metadata()?.is_file() {
                return Err(std::io::Error::other("Image entry is not a regular file"));
            }
            Ok(file)
        }
        fn child(&self, name: &OsStr, create: bool) -> std::io::Result<Self> {
            if create {
                let component = component(name)?;
                // SAFETY: same component/retained-directory invariant as openat above.
                if unsafe { libc::mkdirat(self.file.as_raw_fd(), component.as_ptr(), 0o700) } < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Self::verify(
                self.file(name, libc::O_RDONLY | libc::O_DIRECTORY)?,
                self.path.join(name),
            )
        }
        fn names(&self) -> std::io::Result<Vec<OsString>> {
            // SAFETY: dup creates an owned fd; fdopendir consumes it on success.
            let fd = unsafe { libc::dup(self.file.as_raw_fd()) };
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let stream = unsafe { libc::fdopendir(fd) };
            if stream.is_null() {
                let error = std::io::Error::last_os_error();
                unsafe { libc::close(fd) };
                return Err(error);
            }
            struct Stream(*mut libc::DIR);
            impl Drop for Stream {
                fn drop(&mut self) {
                    unsafe { libc::closedir(self.0) };
                }
            }
            let stream = Stream(stream);
            unsafe {
                libc::rewinddir(stream.0);
            }
            let mut names = Vec::new();
            loop {
                // SAFETY: stream remains alive; copy each entry before the next call.
                let entry = unsafe { libc::readdir(stream.0) };
                if entry.is_null() {
                    break;
                }
                let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
                if name != b"." && name != b".." {
                    names.push(OsString::from_vec(name.to_vec()));
                }
            }
            Ok(names)
        }
        fn lock(&self) -> std::io::Result<File> {
            let file = self.file(OsStr::new(".lock"), libc::O_RDWR | libc::O_CREAT)?;
            fs2::FileExt::try_lock_exclusive(&file)?;
            Ok(file)
        }
        fn remove(&self, name: &OsStr) -> std::io::Result<()> {
            let child = match self.child(name, false) {
                Ok(child) => child,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(e),
            };
            let mut entries = child.names()?;
            // Keep the ownership marker until all files were removed successfully.
            entries.sort_by_key(|entry| entry == "owner.json");
            for entry in entries {
                let entry = component(&entry)?;
                // Unlink only the entry itself. Never follow or recurse into a child.
                if unsafe { libc::unlinkat(child.file.as_raw_fd(), entry.as_ptr(), 0) } < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            let name = component(name)?;
            if unsafe { libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) }
                < 0
            {
                return Err(std::io::Error::last_os_error());
            }
            self.file.sync_all()
        }
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        owner: String,
        session: String,
    }
    pub(crate) struct ImageBundle {
        root: Directory,
        name: OsString,
        paths: Vec<PathBuf>,
        accepted: bool,
        _lock: File,
    }
    impl ImageBundle {
        pub(crate) fn publish(
            root: &Path,
            id: &str,
            images: Vec<ValidatedImage>,
        ) -> std::io::Result<Self> {
            valid_id(id)?;
            let root = Directory::root(root)?;
            let lock = root.lock()?;
            let stage = OsString::from(format!(".staging-{}", uuid::Uuid::new_v4()));
            let directory = root.child(&stage, true)?;
            let mut bundle = Self {
                root,
                name: stage,
                paths: Vec::new(),
                accepted: false,
                _lock: lock,
            };
            let mut manifest = directory.file(
                OsStr::new("owner.json"),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            )?;
            serde_json::to_writer(
                &mut manifest,
                &Manifest {
                    owner: "orkworks-image-context-v1".into(),
                    session: id.into(),
                },
            )?;
            manifest.sync_all()?;
            for (index, image) in images.iter().enumerate() {
                let name = format!("{index}.{}", image.extension);
                let mut file = directory.file(
                    OsStr::new(&name),
                    libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
                )?;
                file.write_all(&image.bytes)?;
                file.sync_all()?;
                bundle.paths.push(bundle.root.path.join(id).join(name));
            }
            directory.file.sync_all()?;
            let from = component(&bundle.name)?;
            let to = component(OsStr::new(id))?;
            // The exclusive bundle lock serializes publication and recovery.
            if bundle.root.child(OsStr::new(id), false).is_ok() {
                return Err(std::io::Error::other("Image bundle already exists"));
            }
            if unsafe {
                libc::renameat(
                    bundle.root.file.as_raw_fd(),
                    from.as_ptr(),
                    bundle.root.file.as_raw_fd(),
                    to.as_ptr(),
                )
            } < 0
            {
                return Err(std::io::Error::last_os_error());
            }
            bundle.name = OsString::from(id);
            bundle.root.file.sync_all()?;
            Ok(bundle)
        }
        pub(crate) fn paths(&self) -> &[PathBuf] {
            &self.paths
        }
        pub(crate) fn accept(&mut self) {
            self.accepted = true;
        }
    }
    impl Drop for ImageBundle {
        fn drop(&mut self) {
            if !self.accepted {
                if let Err(error) = self.root.remove(&self.name) {
                    tracing::warn!(%error,"Image bundle rollback deferred");
                }
            }
        }
    }
    fn valid_id(id: &str) -> std::io::Result<()> {
        uuid::Uuid::parse_str(id)
            .map(|_| ())
            .map_err(|_| std::io::Error::other("Invalid image bundle owner"))
    }
    pub(crate) struct ImageCleanup {
        root: Directory,
        name: OsString,
        _lock: File,
    }
    impl ImageCleanup {
        pub(crate) fn remove(&self) -> std::io::Result<()> {
            self.root.remove(&self.name)
        }
    }
    /// Reserve cleanup without blocking a caller that holds the workspace lock.
    /// The caller must retain this lease while deleting the owning metadata.
    pub(crate) fn prepare_cleanup(root: &Path, id: &str) -> std::io::Result<Option<ImageCleanup>> {
        valid_id(id)?;
        if !root.join("session-images").try_exists()? {
            return Ok(None);
        }
        let root = Directory::root(root)?;
        match root.child(OsStr::new(id), false) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
            Ok(_) => {}
        }
        let lock = root.lock()?;
        Ok(Some(ImageCleanup {
            root,
            name: OsString::from(id),
            _lock: lock,
        }))
    }
    pub(crate) fn recover(
        root: &Path,
        metadata: &crate::metadata::MetadataStore,
    ) -> std::io::Result<()> {
        if !root.join("session-images").try_exists()? {
            return Ok(());
        }
        let root = Directory::root(root)?;
        let _lock = root.lock()?;
        for name in root.names()? {
            let text = name.to_string_lossy();
            let staging = text.starts_with(".staging-");
            if !staging && valid_id(&text).is_err() {
                continue;
            }
            let directory = match root.child(&name, false) {
                Ok(directory) => directory,
                Err(_) => continue,
            };
            let file = match directory.file(OsStr::new("owner.json"), libc::O_RDONLY) {
                Ok(file) => file,
                Err(_) => continue,
            };
            if file.metadata()?.len() > 1024 {
                continue;
            }
            let manifest: Manifest = match serde_json::from_reader(file.take(1025)) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if manifest.owner != "orkworks-image-context-v1"
                || valid_id(&manifest.session).is_err()
                || (!staging && manifest.session != text)
            {
                continue;
            }
            let sessions = metadata.sessions_dir();
            let absent=[sessions.join(format!("{}.json",manifest.session)),sessions.join(format!("{}.json.corrupt",manifest.session))].iter().all(|path|matches!(fs::symlink_metadata(path),Err(error) if error.kind()==std::io::ErrorKind::NotFound));
            if absent {
                root.remove(&name)?;
            }
        }
        Ok(())
    }
}
#[cfg(unix)]
pub(crate) use unix::{prepare_cleanup, recover, ImageBundle};
#[cfg(not(unix))]
pub(crate) struct ImageBundle;
#[cfg(not(unix))]
impl ImageBundle {
    pub(crate) fn publish(_: &Path, _: &str, _: Vec<ValidatedImage>) -> std::io::Result<Self> {
        Err(std::io::Error::other(
            "Private image storage is not qualified on this platform",
        ))
    }
    pub(crate) fn paths(&self) -> &[PathBuf] {
        &[]
    }
    pub(crate) fn accept(&mut self) {}
}
#[cfg(not(unix))]
pub(crate) struct ImageCleanup;
#[cfg(not(unix))]
impl ImageCleanup {
    pub(crate) fn remove(&self) -> std::io::Result<()> {
        Ok(())
    }
}
#[cfg(not(unix))]
pub(crate) fn prepare_cleanup(_: &Path, id: &str) -> std::io::Result<Option<ImageCleanup>> {
    uuid::Uuid::parse_str(id)
        .map(|_| None)
        .map_err(|_| std::io::Error::other("Invalid image bundle owner"))
}
#[cfg(not(unix))]
pub(crate) fn recover(_: &Path, _: &crate::metadata::MetadataStore) -> std::io::Result<()> {
    Ok(())
}
