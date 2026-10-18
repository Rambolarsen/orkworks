//! Sidecar-owned session organization, independent of producer metadata snapshots.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const MAX_RECORD_BYTES: u64 = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionState {
    Active,
    Closed,
    Archived,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OrganizationProjection {
    pub(crate) session_state: Option<SessionState>,
    pub(crate) session_action_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) session_state_error: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    version: u8,
    session_id: String,
    created_at: String,
    state: SessionState,
    revision: u64,
    changed_at: String,
}

pub(crate) struct OrganizationStore {
    root: PathBuf,
    pub(crate) instance_id: String,
    pending: Mutex<HashSet<String>>,
}

pub(crate) struct Reservation {
    store: Arc<OrganizationStore>,
    id: String,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.store.pending.lock().unwrap().remove(&self.id);
    }
}

impl OrganizationStore {
    pub(crate) fn new(root: &Path) -> Self {
        Self {
            root: root.join("session-state"),
            instance_id: uuid::Uuid::new_v4().to_string(),
            pending: Mutex::new(HashSet::new()),
        }
    }
    fn path(&self, id: &str) -> io::Result<PathBuf> {
        if id.is_empty()
            || id.len() > 256
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid session identity",
            ));
        }
        Ok(self.root.join(format!("{id}.json")))
    }
    fn read(&self, id: &str, created_at: &str) -> io::Result<Option<Record>> {
        let file = match fs::File::open(self.path(id)?) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let mut bytes = Vec::new();
        file.take(MAX_RECORD_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_RECORD_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Session state exceeds limit",
            ));
        }
        let record: Record = serde_json::from_slice(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if record.version != 1
            || record.session_id != id
            || record.revision == 0
            || record.state == SessionState::Active
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid session state record",
            ));
        }
        // An orphan from a deleted session must never attach to a reused ID.
        Ok((record.created_at == created_at).then_some(record))
    }
    pub(crate) fn state(
        &self,
        id: &str,
        created_at: &str,
        closed: bool,
        runtime_active: bool,
    ) -> io::Result<(SessionState, u64)> {
        let record = self.read(id, created_at)?;
        let revision = record.as_ref().map_or(0, |r| r.revision);
        let stored_state = record.map(|record| record.state);
        let state = if stored_state == Some(SessionState::Archived) && !runtime_active {
            // Producer lifecycle snapshots cannot revoke sidecar organization.
            SessionState::Archived
        } else if runtime_active || !closed {
            SessionState::Active
        } else {
            stored_state.unwrap_or(SessionState::Closed)
        };
        Ok((state, revision))
    }
    pub(crate) fn write(&self, id: &str, created_at: &str, state: SessionState) -> io::Result<()> {
        if state == SessionState::Active {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Active is runtime-derived",
            ));
        }
        let revision = self
            .read(id, created_at)?
            .map_or(0, |r| r.revision)
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Session state revision exhausted"))?;
        let record = Record {
            version: 1,
            session_id: id.into(),
            created_at: created_at.into(),
            state,
            revision,
            changed_at: crate::workspace_runtime::iso_now(),
        };
        let bytes = serde_json::to_vec(&record)?;
        if bytes.len() as u64 > MAX_RECORD_BYTES {
            return Err(io::Error::other("Session identity exceeds limit"));
        }
        publish_record(&self.root, &self.path(id)?, &bytes, sync_directory)
    }

    pub(crate) fn remove(&self, id: &str) -> io::Result<()> {
        match fs::remove_file(self.path(id)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
    pub(crate) fn pending(&self, id: &str) -> bool {
        self.pending.lock().unwrap().contains(id)
    }
    pub(crate) fn reserve(self: &Arc<Self>, id: &str) -> Option<Reservation> {
        self.pending
            .lock()
            .unwrap()
            .insert(id.into())
            .then(|| Reservation {
                store: self.clone(),
                id: id.into(),
            })
    }
    pub(crate) fn project(
        &self,
        id: &str,
        created_at: &str,
        closed: bool,
        runtime_active: bool,
        runtime_key: &str,
    ) -> OrganizationProjection {
        match self.state(id, created_at, closed, runtime_active) {
            Ok((state, revision)) => {
                // Length-delimited JSON avoids ambiguous concatenations. This is
                // a stale-action precondition, never a credential.
                let token = serde_json::to_vec(&(
                    &self.instance_id,
                    id,
                    created_at,
                    revision,
                    state,
                    runtime_key,
                ))
                .unwrap();
                OrganizationProjection {
                    session_state: Some(state),
                    session_action_token: Some(hex::encode(Sha256::digest(token))),
                    session_state_error: None,
                }
            }
            Err(_) => OrganizationProjection {
                session_state_error: Some(
                    "Session state unavailable; refresh after resolving the storage error".into(),
                ),
                ..Default::default()
            },
        }
    }
}

fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(path)?.sync_all()
    }
    #[cfg(windows)]
    {
        // The write-through move in publish_staged_file provides the Windows
        // publication barrier; directory handles do not use Unix fsync.
        let _ = path;
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Durable session state is unsupported on this platform",
        ))
    }
}

fn publish_record(
    root: &Path,
    target: &Path,
    bytes: &[u8],
    sync: impl Fn(&Path) -> io::Result<()>,
) -> io::Result<()> {
    fs::create_dir_all(root)?;
    // Persist creation of session-state itself before publishing a record.
    if let Some(parent) = root.parent() {
        sync(parent)?;
    }
    let mut staging = tempfile::NamedTempFile::new_in(root)?;
    staging.write_all(bytes)?;
    staging.as_file().sync_all()?;
    // Closing the staged handle also supports native Windows rename rules.
    let staged_path = staging.into_temp_path();
    publish_staged_file(&staged_path, target)?;
    // File sync alone does not persist the renamed directory entry on Unix.
    // Never acknowledge an archive decision before this durability barrier.
    sync(root)
}

fn publish_staged_file(source: &Path, target: &Path) -> io::Result<()> {
    #[cfg(not(windows))]
    {
        fs::rename(source, target)
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let source: Vec<u16> = source
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let target: Vec<u16> = target
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        if source[..source.len() - 1].contains(&0) || target[..target.len() - 1].contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid session state path",
            ));
        }
        // SAFETY: both paths are validated, nul-terminated UTF-16 buffers,
        // retained for the complete API call. Same-directory staging avoids a
        // cross-volume copy. WRITE_THROUGH waits for publication on disk.
        let result = unsafe {
            MoveFileExW(
                source.as_ptr(),
                target.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_cannot_acknowledge_a_failed_directory_sync() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("session-state");
        let target = root.join("one.json");
        let result = publish_record(&root, &target, b"record", |path| {
            if path == root {
                Err(io::Error::other("directory sync failed"))
            } else {
                sync_directory(path)
            }
        });
        assert!(
            result.is_err(),
            "archive success must include directory-entry durability"
        );
        assert_eq!(
            fs::read(target).unwrap(),
            b"record",
            "failure can occur after rename and must still be reported"
        );
    }

    #[test]
    fn archive_survives_restart_and_restore_preserves_identity() {
        let dir = tempfile::tempdir().unwrap();
        let store = OrganizationStore::new(dir.path());
        assert_eq!(
            store.state("old", "created", true, false).unwrap(),
            (SessionState::Closed, 0)
        );
        store
            .write("old", "created", SessionState::Archived)
            .unwrap();
        let reopened = OrganizationStore::new(dir.path());
        assert_eq!(
            reopened.state("old", "created", true, false).unwrap(),
            (SessionState::Archived, 1)
        );
        assert_eq!(
            reopened.state("old", "replacement", true, false).unwrap(),
            (SessionState::Closed, 0)
        );
        reopened
            .write("old", "created", SessionState::Closed)
            .unwrap();
        assert_eq!(
            reopened.state("old", "created", true, false).unwrap(),
            (SessionState::Closed, 2)
        );
    }
    #[test]
    fn malformed_and_oversized_records_disable_mutation_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let store = OrganizationStore::new(dir.path());
        fs::create_dir_all(&store.root).unwrap();
        for bytes in [b"bad".to_vec(), vec![b' '; 4097]] {
            fs::write(store.path("old").unwrap(), &bytes).unwrap();
            let projected = store.project("old", "created", true, false, "none");
            assert_eq!(projected.session_state, None);
            assert_eq!(projected.session_action_token, None);
            assert!(store
                .write("old", "created", SessionState::Archived)
                .is_err());
            assert_eq!(fs::read(store.path("old").unwrap()).unwrap(), bytes);
        }
    }
    #[test]
    fn failed_publication_preserves_record_and_revision() {
        let dir = tempfile::tempdir().unwrap();
        let store = OrganizationStore::new(dir.path());
        store
            .write("old", "created", SessionState::Archived)
            .unwrap();
        let before = fs::read(store.path("old").unwrap()).unwrap();
        // Reject an invalid next identity before publication; keep the archive intact.
        assert!(store
            .write("old", &"x".repeat(5000), SessionState::Closed)
            .is_err());
        assert_eq!(fs::read(store.path("old").unwrap()).unwrap(), before);
        assert!(store.state("../escape", "created", true, false).is_err());
    }
    #[test]
    fn archive_survives_lifecycle_regression_but_real_runtime_wins() {
        let dir = tempfile::tempdir().unwrap();
        let store = OrganizationStore::new(dir.path());
        store
            .write("old", "created", SessionState::Archived)
            .unwrap();
        assert_eq!(
            store.state("old", "created", false, false).unwrap().0,
            SessionState::Archived
        );
        assert_eq!(
            store.state("old", "created", false, true).unwrap().0,
            SessionState::Active
        );
        store.write("old", "created", SessionState::Closed).unwrap();
        assert_eq!(store.state("old", "created", false, false).unwrap().0, SessionState::Active,
            "a stored Closed choice can predate a resumed run and cannot prove current finalization");
    }

    #[test]
    fn reservations_release_on_cancellation_and_tokens_bind_store_runtime_revision() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(OrganizationStore::new(dir.path()));
        let first = store
            .project("old", "created", true, false, "generation1")
            .session_action_token;
        assert_ne!(
            first,
            store
                .project("old", "created", true, false, "generation2")
                .session_action_token
        );
        let reservation = store.reserve("old").unwrap();
        assert!(store.reserve("old").is_none());
        drop(reservation);
        assert!(!store.pending("old"));
        store
            .write("old", "created", SessionState::Archived)
            .unwrap();
        assert_ne!(
            first,
            store
                .project("old", "created", true, false, "generation1")
                .session_action_token
        );
        assert_ne!(
            first,
            OrganizationStore::new(dir.path())
                .project("old", "created", true, false, "generation1")
                .session_action_token
        );
    }
}
