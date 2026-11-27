//! Handle-bound I/O for the Windows x64 legacy rollup journal.
use crate::windows_fs as native;
use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
    FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, SYNCHRONIZE,
};

pub(super) struct Directory {
    pub(super) path: PathBuf,
    file: File,
}

pub(super) struct Leaf(File);

impl Directory {
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        native::verify_kind(&file, true)?;
        Ok(Self {
            path: path.into(),
            file,
        })
    }

    fn directory(&self, name: &OsStr, create: bool) -> io::Result<Self> {
        let file = native::child_inherited(
            &self.file,
            name,
            true,
            FILE_GENERIC_READ | DELETE,
            create,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
        )?;
        Ok(Self {
            path: self.path.join(name),
            file,
        })
    }

    pub(super) fn open_dir(&self, name: &OsStr) -> io::Result<Self> {
        self.directory(name, false)
    }

    pub(super) fn create_dir(&self, name: &OsStr) -> io::Result<Self> {
        self.directory(name, true)
    }

    pub(super) fn open_candidate(&self, name: &OsStr) -> io::Result<Option<Self>> {
        let Some(candidate) = native::inspect_directory_entry(&self.file, name)? else {
            return Ok(None);
        };
        let directory = self.open_dir(name)?;
        if native::file_identity(&candidate)? != native::file_identity(&directory.file)? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Recommendation transaction directory changed during inspection",
            ));
        }
        Ok(Some(directory))
    }

    pub(super) fn names(&self) -> io::Result<Vec<OsString>> {
        native::names_up_to(&self.file, super::deletion_journal::MAX_DIRECTORY_ENTRIES)
    }

    fn leaf(&self, name: &OsStr, access: u32, create: bool) -> io::Result<File> {
        native::child_inherited(&self.file, name, false, access, create, FILE_SHARE_READ)
    }

    pub(super) fn read(&self, name: &OsStr) -> io::Result<Vec<u8>> {
        let mut file = self.leaf(name, FILE_GENERIC_READ, false)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub(super) fn file_exists(&self, name: &OsStr) -> io::Result<bool> {
        match self.leaf(name, FILE_READ_ATTRIBUTES | SYNCHRONIZE, false) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub(super) fn movable(&self, name: &OsStr) -> io::Result<Leaf> {
        self.leaf(name, FILE_GENERIC_READ | DELETE, false).map(Leaf)
    }

    pub(super) fn write(&self, name: &OsStr, bytes: &[u8]) -> io::Result<Leaf> {
        let mut file = self.leaf(name, FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE, true)?;
        if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            discard_scratch(&file);
            return Err(error);
        }
        Ok(Leaf(file))
    }

    pub(super) fn remove_file(&self, name: &OsStr) -> io::Result<()> {
        match self.leaf(name, DELETE | FILE_READ_ATTRIBUTES | SYNCHRONIZE, false) {
            Ok(file) => native::mark_remove(&file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub(super) fn remove_leaves(&self) -> io::Result<()> {
        for name in native::names(&self.file)? {
            self.remove_file(&name)?;
        }
        Ok(())
    }

    pub(super) fn remove(self) -> io::Result<()> {
        native::mark_remove(&self.file)
    }
}

impl Leaf {
    pub(super) fn read(&self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        (&self.0).read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub(super) fn publish(self, target: &Directory, name: &OsStr, replace: bool) -> io::Result<()> {
        self.rename(target, name, replace)
    }

    pub(super) fn publish_scratch(
        self,
        target: &Directory,
        name: &OsStr,
        replace: bool,
    ) -> io::Result<()> {
        let result = self.rename(target, name, replace);
        if result.is_err() {
            discard_scratch(&self.0);
        }
        result
    }

    fn rename(&self, target: &Directory, name: &OsStr, replace: bool) -> io::Result<()> {
        // Rename the verified source handle. The destination is a single
        // namespace entry under the pinned directory, never a followed link.
        native::rename_entry(&self.0, Some(&target.file), name, replace)
    }
}

fn discard_scratch(file: &File) {
    if let Err(error) = native::mark_remove(file) {
        tracing::warn!(%error, "Could not discard an unpublished rollup scratch file");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_never_follows_a_late_destination_link() {
        let root = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let victim = external.path().join("victim.json");
        std::fs::write(&victim, b"outside sentinel").unwrap();
        let directory = Directory::open(root.path()).unwrap();
        let target_path = root.path().join("target.json");
        std::fs::write(&target_path, b"replaceable file").unwrap();
        let leaf = directory
            .write(OsStr::new("source.json"), b"local replacement")
            .unwrap();
        assert!(directory.file_exists(OsStr::new("target.json")).unwrap());
        std::fs::remove_file(&target_path).unwrap();
        match std::os::windows::fs::symlink_file(&victim, &target_path) {
            Ok(()) => {}
            Err(error) if error.raw_os_error() == Some(1314) => {
                eprintln!("Symlink creation unavailable on this Windows host (1314)");
                return;
            }
            Err(error) => panic!("cannot create late destination link fixture: {error}"),
        }
        let result = leaf.publish(&directory, OsStr::new("target.json"), true);
        assert_eq!(std::fs::read(&victim).unwrap(), b"outside sentinel");
        if result.is_ok() {
            assert_eq!(std::fs::read(&target_path).unwrap(), b"local replacement");
        }
    }
}
