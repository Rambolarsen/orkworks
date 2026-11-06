//! Nonblocking ownership for a complete workspace-memory edit.
use super::{write_atomic, WorkspaceMemory};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub(super) struct Owner {
    root: PathBuf,
    editing: AtomicBool,
}
impl Owner {
    pub(super) fn new(root: &Path) -> Arc<Self> {
        Arc::new(Self {
            root: root.into(),
            editing: AtomicBool::new(false),
        })
    }
    pub(super) fn edit(self: &Arc<Self>) -> io::Result<Edit> {
        self.editing
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "workspace memory is being edited",
                )
            })?;
        Ok(Edit {
            owner: self.clone(),
        })
    }
}

/// Holds no application guards and never waits for another edit.
/// This is mutation serialization, not durable SQL cleanup authority.
pub(crate) struct Edit {
    owner: Arc<Owner>,
}
impl Edit {
    pub(crate) fn read(&self) -> io::Result<Option<WorkspaceMemory>> {
        let data = match fs::read(self.owner.root.join("workspace.json")) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        serde_json::from_slice(&data)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
    pub(crate) fn write(&self, memory: &WorkspaceMemory) -> io::Result<()> {
        fs::create_dir_all(&self.owner.root)?;
        let json = serde_json::to_string_pretty(memory)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        write_atomic(&self.owner.root.join("workspace.json"), &json)
    }
}
impl Drop for Edit {
    fn drop(&mut self) {
        self.owner.editing.store(false, Ordering::Release);
    }
}

mod sql_cleanup;
pub(crate) use sql_cleanup::{BoundSettingsCleanup, SettingsCleanup};
impl Owner {
    pub(super) fn capture_cleanup(self: &Arc<Self>) -> SettingsCleanup {
        SettingsCleanup {
            owner: self.clone(),
        }
    }
}
