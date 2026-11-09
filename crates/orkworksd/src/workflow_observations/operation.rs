//! Observation-specific weak shared admission. No filesystem work under registry/state locks.
use super::*;
use std::{io, sync::OnceLock};
type SqlResult<T> = std::result::Result<T, crate::session_store::StoreError>;
#[derive(Default)]
struct State {
    legacy: usize,
    sql: bool,
    busy: bool,
}
pub(super) struct Owner {
    state: Mutex<State>,
}
pub(super) struct Legacy {
    owner: Arc<Owner>,
}
pub(crate) struct SqlOwner {
    _owner: Arc<Owner>,
}
/// Original admitted file ownership; read-only opening never constructs this type.
pub(crate) struct SqlWriter {
    candidate: SqlCandidate,
    admission: crate::session_application::sql_writes::ObservationWriteAdmission,
    deadline: std::time::Instant,
}
impl SqlWriter {
    /// Blocking-only, outside application guards. Actual effects retain the owner.
    pub(crate) fn persist(
        self,
        request: sql_origin::ScanRequest,
        origin: ObservationOrigin,
        key: &str,
        candidate: ObservationCandidate,
    ) -> std::result::Result<RecordOutcome, sql_origin::WriteError> {
        sql_origin::persist(
            &self.candidate.directory,
            self.admission,
            request.limit_deadline(self.deadline),
            origin,
            key,
            candidate,
        )
    }
}
pub(crate) struct SqlCandidate {
    owner: Arc<Owner>,
    directory: fs::File,
    admitted: bool,
}
impl Owner {
    pub(super) fn legacy(self: &Arc<Self>) -> io::Result<Legacy> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("observation admission unavailable"))?;
        if state.sql {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "SQL observation authority required",
            ));
        }
        if state.busy {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "observation opening is active",
            ));
        }
        state.legacy = state
            .legacy
            .checked_add(1)
            .ok_or_else(|| io::Error::other("observation admission exhausted"))?;
        Ok(Legacy {
            owner: self.clone(),
        })
    }
    fn sql(self: &Arc<Self>, directory: fs::File) -> SqlResult<SqlCandidate> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| crate::session_store::StoreError::Unavailable)?;
        if state.legacy != 0 || state.busy {
            return Err(crate::session_store::StoreError::Busy);
        }
        state.busy = true;
        Ok(SqlCandidate {
            owner: self.clone(),
            directory,
            admitted: false,
        })
    }
}
impl Drop for Legacy {
    fn drop(&mut self) {
        self.owner.state.lock().unwrap().legacy -= 1;
    }
}
impl Drop for SqlCandidate {
    fn drop(&mut self) {
        self.owner.state.lock().unwrap().busy = false;
    }
}
impl SqlCandidate {
    pub(crate) fn revalidate(
        &self,
        root: &Path,
        lease: &crate::workspace_runtime::WorkspaceLease,
    ) -> SqlResult<()> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            use std::os::unix::fs::MetadataExt;
            let current = deletion::Anchor::open(root)?.checked_directory(lease)?;
            let before = self.directory.metadata()?;
            let after = current.metadata()?;
            if before.dev() != after.dev() || before.ino() != after.ino() {
                return Err(crate::session_store::StoreError::Conflict);
            }
            Ok(())
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = (root, lease);
            Err(crate::session_store::StoreError::Unsupported)
        }
    }

    pub(crate) fn settle(self) -> SqlResult<SqlOwner> {
        if !self.admitted {
            return Err(crate::session_store::StoreError::Conflict);
        }
        Ok(SqlOwner {
            _owner: self.owner.clone(),
        })
    }
    /// Only a checked application opener can turn this read-only binding into admission.
    pub(crate) fn bind(
        root: &Path,
        lease: &crate::workspace_runtime::WorkspaceLease,
    ) -> SqlResult<Self> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let canonical = root_key(root)?;
            let owner = for_root(&canonical)?;
            let anchor = deletion::Anchor::open(&canonical)?;
            owner.sql(anchor.checked_directory(lease)?)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = (root, lease);
            Err(crate::session_store::StoreError::Unsupported)
        }
    }

    /// Only a short checked application callback can supply this sealed token.
    pub(crate) fn admit(
        &mut self,
        _admission: crate::session_application::sql_writes::ObservationOpenAdmission,
        claim: &FileClaim,
    ) -> SqlResult<()> {
        self.admit_file(claim)
    }
    fn admit_file(&mut self, claim: &FileClaim) -> SqlResult<()> {
        if self.admitted {
            return Err(crate::session_store::StoreError::Conflict);
        }
        claim.admit()?;
        self.owner
            .state
            .lock()
            .map_err(|_| crate::session_store::StoreError::Unavailable)?
            .sql = true;
        self.admitted = true;
        Ok(())
    }
    /// Memory-only admission in the short checked application callback. The
    /// returned grant leaves that callback before any filesystem work starts.
    pub(crate) fn admit_write(
        mut self,
        admission: crate::session_application::sql_writes::ObservationWriteAdmission,
        claim: &FileClaim,
    ) -> SqlResult<SqlWriter> {
        self.admit_file(claim)?;
        Ok(SqlWriter {
            candidate: self,
            admission,
            deadline: claim.deadline,
        })
    }
    pub(crate) fn read_scope(
        &self,
        request: sql_origin::ScanRequest,
        epoch: &str,
        workspace_hash: &str,
    ) -> SqlResult<sql_origin::Snapshot> {
        if !self.admitted {
            return Err(crate::session_store::StoreError::Conflict);
        }
        request.read_at(&self.directory, epoch, workspace_hash)
    }
    pub(crate) fn read(
        &self,
        request: sql_origin::ScanRequest,
        source: &crate::session_store::ObservationSource,
    ) -> SqlResult<sql_origin::Snapshot> {
        if !self.admitted {
            return Err(crate::session_store::StoreError::Conflict);
        }
        request.read_at(
            &self.directory,
            source.database_epoch(),
            source.workspace_sha256(),
        )
    }
}
pub(super) fn for_root(root: &Path) -> io::Result<Arc<Owner>> {
    static OWNERS: OnceLock<Mutex<HashMap<PathBuf, Weak<Owner>>>> = OnceLock::new();
    let mut owners = OWNERS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| io::Error::other("observation registry unavailable"))?;
    owners.retain(|_, owner| owner.strong_count() > 0);
    if let Some(owner) = owners.get(root).and_then(Weak::upgrade) {
        return Ok(owner);
    }
    let owner = Arc::new(Owner {
        state: Mutex::new(State::default()),
    });
    owners.insert(root.into(), Arc::downgrade(&owner));
    Ok(owner)
}
pub(super) fn root_key(root: &Path) -> io::Result<PathBuf> {
    let mut current = std::path::absolute(root)?;
    let mut suffix = Vec::new();
    let mut resolved = loop {
        match fs::canonicalize(&current) {
            Ok(path) => break path,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                suffix.push(
                    current
                        .components()
                        .next_back()
                        .ok_or(error)?
                        .as_os_str()
                        .to_owned(),
                );
                if !current.pop() {
                    return Err(io::ErrorKind::NotFound.into());
                }
            }
            Err(error) => return Err(error),
        }
    };
    for component in suffix.into_iter().rev() {
        if component == ".." {
            resolved.pop();
        } else if component != "." {
            resolved.push(component);
        }
    }
    Ok(resolved)
}
impl WorkflowObservationStore {
    pub(crate) fn sql_candidate(
        &self,
        _lease: &crate::workspace_runtime::WorkspaceLease,
    ) -> SqlResult<SqlCandidate> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let anchor = self.cleanup_anchor.as_ref().as_ref().map_err(|error| {
                crate::session_store::StoreError::Io(io::Error::new(
                    error.kind(),
                    error.to_string(),
                ))
            })?;
            let directory = anchor.checked_directory(_lease)?;
            self.owner.sql(directory)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Err(crate::session_store::StoreError::Unsupported)
        }
    }
}
mod preflight;
pub(super) use preflight::legacy_preflight;

#[cfg(test)]
pub(super) fn preflight_after_entry(callback: Box<dyn FnOnce()>) {
    preflight::AFTER_ENTRY.with(|slot| *slot.borrow_mut() = Some(callback));
}
