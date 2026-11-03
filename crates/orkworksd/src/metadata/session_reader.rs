//! Owned exact-session reads share the legacy decoder. Normal reads take no
//! mutation lock; corrupt-file quarantine uses the existing transaction lane.
//! Async consumers admit bounded work before spawning it.
use super::{MetadataStore, SessionMetadata};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(crate) struct SessionReader {
    root: PathBuf,
    admission: Arc<Semaphore>,
    write_locks: SessionWriteLocks,
}
impl SessionReader {
    pub(super) fn new(root: &Path, write_locks: SessionWriteLocks) -> Arc<Self> {
        Arc::new(Self {
            root: root.to_owned(),
            admission: Arc::new(Semaphore::new(8)),
            write_locks,
        })
    }
    pub(crate) fn try_admit(&self) -> Option<OwnedSemaphorePermit> {
        self.admission.clone().try_acquire_owned().ok()
    }
    pub(crate) fn read(&self, id: &str) -> Option<SessionMetadata> {
        let path = self.root.join("sessions").join(format!("{id}.json"));
        #[cfg(test)]
        testing::before_read(&path);
        self.read_path(&path)
    }
    pub(super) fn read_path(&self, path: &Path) -> Option<SessionMetadata> {
        MetadataStore::load_session_file(path, |data, error| {
            let id = path.file_stem().unwrap_or_default().to_string_lossy();
            // Quarantine is a mutation: share the existing transaction lane and
            // verify the bytes again so a repaired/replaced file is never moved.
            with_session_write_lock(&self.write_locks, &id, || {
                if std::fs::read_to_string(path).ok().as_deref() != Some(data) {
                    return;
                }
                #[cfg(test)]
                testing::before_rename(path);
                let quarantine = super::corrupt_session_path(path);
                match std::fs::rename(path, &quarantine) {
                    Ok(()) => tracing::warn!("session file {:?} is corrupt ({error}); quarantined to {:?}", path, quarantine),
                    Err(rename_error) => tracing::warn!("session file {:?} is corrupt ({error}) and could not be quarantined: {rename_error}", path),
                }
            });
        })
    }
    #[cfg(test)]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

// One shared registry, including the original MetadataStore transaction paths.
pub(super) type SessionWriteLocks = Arc<Mutex<HashMap<String, Weak<Mutex<()>>>>>;
pub(super) fn session_write_lock(locks: &SessionWriteLocks, id: &str) -> Arc<Mutex<()>> {
    let mut locks = locks.lock().unwrap();
    if let Some(lock) = locks.get(id).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(Mutex::new(()));
    locks.insert(id.to_owned(), Arc::downgrade(&lock));
    lock
}
pub(super) fn with_session_write_lock<T>(
    locks: &SessionWriteLocks,
    id: &str,
    operation: impl FnOnce() -> T,
) -> T {
    let lock = session_write_lock(locks, id);
    let guard = lock.lock().unwrap();
    let result = operation();
    drop(guard);
    let mut locks = locks.lock().unwrap();
    let remove = locks
        .get(id)
        .and_then(Weak::upgrade)
        .is_some_and(|registered| {
            Arc::ptr_eq(&registered, &lock) && Arc::strong_count(&registered) == 2
        });
    if remove {
        locks.remove(id);
    }
    result
}

#[cfg(test)]
pub(crate) mod testing {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::{LazyLock, Mutex};
    use tokio::sync::oneshot;
    type Gate = (usize, oneshot::Sender<()>, std::sync::mpsc::Receiver<()>);
    static GATES: LazyLock<Mutex<HashMap<PathBuf, Gate>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    pub(crate) struct Pause {
        entered: Option<oneshot::Receiver<()>>,
        release: Option<std::sync::mpsc::Sender<()>>,
        key: PathBuf,
    }
    impl Pause {
        pub(crate) async fn entered(&mut self) {
            self.entered.take().unwrap().await.unwrap();
        }
    }
    impl Drop for Pause {
        fn drop(&mut self) {
            GATES.lock().unwrap().remove(&self.key);
            if let Some(tx) = self.release.take() {
                let _ = tx.send(());
            }
        }
    }
    pub(crate) fn hold_next_quarantine(path: PathBuf) -> Pause {
        hold(path, 0)
    }
    fn hold(path: PathBuf, skip: usize) -> Pause {
        let (tx, entered) = oneshot::channel();
        let (release, rx) = std::sync::mpsc::channel();
        assert!(GATES
            .lock()
            .unwrap()
            .insert(path.clone(), (skip, tx, rx))
            .is_none());
        Pause {
            entered: Some(entered),
            release: Some(release),
            key: path,
        }
    }
    pub(crate) fn hold_nth_read(path: &Path, nth: usize) -> Pause {
        assert!(nth > 0);
        hold(path.with_extension("json.before-read"), nth - 1)
    }
    pub(super) fn before_read(path: &Path) {
        before_quarantine(&path.with_extension("json.before-read"));
    }
    pub(crate) fn hold_next_rename(path: &Path) -> Pause {
        hold_next_quarantine(path.with_extension("json.before-quarantine-rename"))
    }
    pub(super) fn before_rename(path: &Path) {
        before_quarantine(&path.with_extension("json.before-quarantine-rename"));
    }
    pub(crate) fn before_quarantine(path: &Path) {
        let held = {
            let mut gates = GATES.lock().unwrap();
            if let Some((skip, _, _)) = gates.get_mut(path) {
                if *skip > 0 {
                    *skip -= 1;
                    return;
                }
            }
            gates.remove(path)
        };
        if let Some((_, entered, release)) = held {
            let _ = entered.send(());
            let _ = release.recv();
        }
    }
}
