//! Bind the Windows metadata paths used by one Forget or retention operation.
use super::{native_directory_identity, WorkspaceLease};
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf, Prefix},
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

const MAX_ALIASES: usize = 8;

pub(crate) struct CleanupNamespace {
    _handles: Vec<File>,
    pub(crate) events: bool,
    pub(crate) session_state: bool,
    pub(crate) recommendations: bool,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn open_entry(path: &Path, sharing: u32) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .share_mode(sharing)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

fn pin_directory(path: &Path, handles: &mut Vec<File>, aliases: &mut usize) -> io::Result<()> {
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return Err(invalid(
            "Cleanup metadata must have an absolute local drive path",
        ));
    };
    if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        || components.next() != Some(Component::RootDir)
    {
        return Err(invalid(
            "Cleanup metadata must have an absolute local drive path",
        ));
    }

    let mut current = PathBuf::from(prefix.as_os_str());
    current.push(Path::new("\\"));
    pin_entry(&current, handles, aliases)?;
    for component in components {
        let Component::Normal(part) = component else {
            return Err(invalid("Unexpected cleanup directory component"));
        };
        current.push(part);
        pin_entry(&current, handles, aliases)?;
    }
    Ok(())
}

fn pin_entry(path: &Path, handles: &mut Vec<File>, aliases: &mut usize) -> io::Result<()> {
    let directory = open_entry(path, FILE_SHARE_READ | FILE_SHARE_WRITE)?;
    let attributes = directory.metadata()?.file_attributes();
    if attributes & FILE_ATTRIBUTE_DIRECTORY == 0 {
        return Err(invalid("Cleanup path entry is not a directory"));
    }
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        handles.push(directory);
        return Ok(());
    }
    *aliases = aliases
        .checked_sub(1)
        .ok_or_else(|| invalid("Too many or unsupported cleanup directory aliases"))?;
    let alias = open_entry(path, FILE_SHARE_READ)?;
    let metadata = alias.metadata()?;
    if native_directory_identity(&alias, &metadata)?
        != native_directory_identity(&directory, &directory.metadata()?)?
    {
        return Err(invalid("Cleanup directory alias changed"));
    }
    let target = fs::read_link(path)?;
    let absolute = if target.is_absolute() {
        target
    } else {
        path.parent()
            .ok_or_else(|| invalid("Cleanup directory alias has no parent"))?
            .join(target)
    };
    handles.push(alias);
    pin_directory(&absolute, handles, aliases)
}

pub(crate) fn pin_windows_directory_path(path: &Path) -> io::Result<Vec<File>> {
    let mut handles = Vec::new();
    let mut aliases = MAX_ALIASES;
    pin_directory(path, &mut handles, &mut aliases)?;
    Ok(handles)
}

fn open_plain_entry(path: &Path) -> io::Result<(File, bool)> {
    let file = open_entry(path, FILE_SHARE_READ | FILE_SHARE_WRITE)?;
    let attributes = file.metadata()?.file_attributes();
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(invalid("Cleanup store path is a reparse point"));
    }
    Ok((file, attributes & FILE_ATTRIBUTE_DIRECTORY != 0))
}

fn open_plain_directory(path: &Path) -> io::Result<File> {
    let (directory, is_directory) = open_plain_entry(path)?;
    if !is_directory {
        return Err(invalid("Cleanup store path is not a directory"));
    }
    Ok(directory)
}

fn pin_child(root: &Path, name: &str, handles: &mut Vec<File>) -> io::Result<bool> {
    match open_plain_directory(&root.join(name)) {
        Ok(file) => {
            handles.push(file);
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub(crate) fn pin_metadata_root(
    root: &Path,
    lease: Option<&WorkspaceLease>,
) -> io::Result<Vec<File>> {
    let mut handles = pin_windows_directory_path(root)?;
    if let Some(lease) = lease {
        handles.push(lease.pin_cleanup_lock(root)?);
    } else if !cfg!(test) {
        return Err(invalid("Cleanup metadata has no active workspace lease"));
    }
    Ok(handles)
}

// The caller retains the metadata root binding through creation and record write.
pub(crate) fn pin_sessions_for_create(root: &Path) -> io::Result<File> {
    let sessions = root.join("sessions");
    match open_plain_directory(&sessions) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match fs::create_dir(&sessions) {
                Ok(()) => (),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
                Err(error) => return Err(error),
            }
            open_plain_directory(&sessions)
        }
        result => result,
    }
}

impl CleanupNamespace {
    pub(crate) fn acquire(root: &Path, lease: Option<&WorkspaceLease>) -> io::Result<Self> {
        let mut handles = pin_metadata_root(root, lease)?;

        for name in ["sessions", "workflow-observations"] {
            if !pin_child(root, name, &mut handles)? {
                return Err(invalid("Cleanup store directory is unavailable"));
            }
        }
        match crate::durable_file::open_metadata_file(&root.join("workspace.json")) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let events = pin_child(root, "events", &mut handles)?;
        let session_state = pin_child(root, "session-state", &mut handles)?;
        // Preserve post-record-removal failure for a missing or ordinary file
        // at this store path. A reparse point is an invalid binding.
        let recommendations = match open_plain_entry(&root.join("recommendations")) {
            Ok((file, is_directory)) => {
                handles.push(file);
                is_directory
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error),
        };
        Ok(Self {
            _handles: handles,
            events,
            session_state,
            recommendations,
        })
    }
}
