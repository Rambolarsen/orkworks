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
    ) -> io::Result<(SessionState, u64)> {
        let record = self.read(id, created_at)?;
        let revision = record.as_ref().map_or(0, |r| r.revision);
        let state = if !closed {
            SessionState::Active
        } else {
            record.map_or(SessionState::Closed, |r| r.state)
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
        fs::create_dir_all(&self.root)?;
        let mut staging = tempfile::NamedTempFile::new_in(&self.root)?;
        staging.write_all(&bytes)?;
        staging.as_file().sync_all()?;
        staging.persist(self.path(id)?).map_err(|e| e.error)?;
        Ok(())
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
        runtime_key: &str,
    ) -> OrganizationProjection {
        match self.state(id, created_at, closed) {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archive_survives_restart_and_restore_preserves_identity() {
        let dir = tempfile::tempdir().unwrap();
        let store = OrganizationStore::new(dir.path());
        assert_eq!(
            store.state("old", "created", true).unwrap(),
            (SessionState::Closed, 0)
        );
        store
            .write("old", "created", SessionState::Archived)
            .unwrap();
        let reopened = OrganizationStore::new(dir.path());
        assert_eq!(
            reopened.state("old", "created", true).unwrap(),
            (SessionState::Archived, 1)
        );
        assert_eq!(
            reopened.state("old", "replacement", true).unwrap(),
            (SessionState::Closed, 0)
        );
        reopened
            .write("old", "created", SessionState::Closed)
            .unwrap();
        assert_eq!(
            reopened.state("old", "created", true).unwrap(),
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
            let projected = store.project("old", "created", true, "none");
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
        assert!(store.state("../escape", "created", true).is_err());
    }
    #[test]
    fn reservations_release_on_cancellation_and_tokens_bind_store_runtime_revision() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(OrganizationStore::new(dir.path()));
        let first = store
            .project("old", "created", true, "generation1")
            .session_action_token;
        assert_ne!(
            first,
            store
                .project("old", "created", true, "generation2")
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
                .project("old", "created", true, "generation1")
                .session_action_token
        );
        assert_ne!(
            first,
            OrganizationStore::new(dir.path())
                .project("old", "created", true, "generation1")
                .session_action_token
        );
    }
}
