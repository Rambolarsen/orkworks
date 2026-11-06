//! Shared process-local admission; never wait for store I/O under app guards.
use super::StoreError;
use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
};

pub(super) struct Owner {
    busy: AtomicBool,
}
pub(super) struct Operation {
    owner: Arc<Owner>,
}
impl Owner {
    pub(super) fn claim(self: &Arc<Self>) -> Result<Operation, StoreError> {
        self.busy
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| {
                StoreError::Io(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "recommendation store is busy",
                ))
            })?;
        Ok(Operation {
            owner: self.clone(),
        })
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        self.owner.busy.store(false, Ordering::Release);
    }
}

pub(super) fn for_root(root: &Path) -> Result<Arc<Owner>, StoreError> {
    static OWNERS: OnceLock<Mutex<HashMap<PathBuf, Weak<Owner>>>> = OnceLock::new();
    let mut owners = OWNERS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| {
            StoreError::Io(io::Error::other(
                "recommendation owner registry unavailable",
            ))
        })?;
    owners.retain(|_, owner| owner.strong_count() > 0);
    if let Some(owner) = owners.get(root).and_then(Weak::upgrade) {
        return Ok(owner);
    }
    let owner = Arc::new(Owner {
        busy: AtomicBool::new(false),
    });
    owners.insert(root.into(), Arc::downgrade(&owner));
    Ok(owner)
}
