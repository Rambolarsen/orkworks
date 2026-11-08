//! Shared process-local admission; never wait for store I/O under app guards.
use super::StoreError;
use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
};

const BUSY: u8 = 1;
const SQL: u8 = 2;

pub(super) struct Owner {
    state: AtomicU8,
    released: tokio::sync::Notify,
}
pub(super) struct Operation {
    owner: Arc<Owner>,
}
impl Owner {
    pub(super) fn claim(self: &Arc<Self>) -> Result<Operation, StoreError> {
        self.state
            .compare_exchange(0, BUSY, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|state| {
                if state & BUSY != 0 {
                    StoreError::Busy
                } else {
                    StoreError::SqlAuthorityRequired
                }
            })?;
        Ok(Operation {
            owner: self.clone(),
        })
    }
    pub(super) fn claim_sql(self: &Arc<Self>) -> Result<Operation, StoreError> {
        self.state
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |state| {
                (state & BUSY == 0).then_some(state | BUSY)
            })
            .map_err(|_| StoreError::Busy)?;
        Ok(Operation {
            owner: self.clone(),
        })
    }
    pub(super) async fn wait_until_available(&self) {
        loop {
            let released = self.released.notified();
            tokio::pin!(released);
            // Register before checking busy, including releases before awaiting.
            released.as_mut().enable();
            if self.state.load(Ordering::Acquire) & BUSY == 0 {
                return;
            }
            released.await;
        }
    }
}
impl Operation {
    // Exclusive busy ownership makes the monotonic transition indivisible
    // with respect to all claims, without I/O, waiting or fallible allocation.
    pub(super) fn revoke_legacy(&self) {
        self.owner.state.fetch_or(SQL, Ordering::Release);
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        self.owner.state.fetch_and(!BUSY, Ordering::Release);
        self.owner.released.notify_waiters();
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
        state: AtomicU8::new(0),
        released: tokio::sync::Notify::new(),
    });
    owners.insert(root.into(), Arc::downgrade(&owner));
    Ok(owner)
}
