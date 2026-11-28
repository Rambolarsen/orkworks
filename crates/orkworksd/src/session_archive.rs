//! Sidecar-owned session organization, independent of producer metadata snapshots.
use crate::durable_file::{replace as publish_staged_file, sync_directory};
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
const RECORD_VERSION_V1: u8 = 1;
const RECORD_VERSION_V2: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionState {
    Active,
    Closed,
    Archived,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ManagedWorktreeCleanupStatus {
    Active,
    Removed,
    RetainedDirty,
    RemovalFailed,
}

impl ManagedWorktreeCleanupStatus {
    pub(crate) fn protects_from_deletion(self) -> bool {
        !matches!(self, Self::Removed)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManagedHookFileRecord {
    pub(crate) relative_path: String,
    pub(crate) sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManagedWorktreeRecord {
    pub(crate) root: String,
    pub(crate) common_dir: String,
    pub(crate) branch: String,
    pub(crate) base_ref: String,
    pub(crate) cleanup_status: ManagedWorktreeCleanupStatus,
    #[serde(default)]
    pub(crate) cleanup_note: Option<String>,
    #[serde(default)]
    pub(crate) hook_note: Option<String>,
    #[serde(default)]
    pub(crate) managed_hook_files: Vec<ManagedHookFileRecord>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OrganizationProjection {
    pub(crate) session_state: Option<SessionState>,
    pub(crate) session_action_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) session_state_error: Option<String>,
    pub(crate) favorited: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) archived_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecordV1 {
    version: u8,
    session_id: String,
    created_at: String,
    state: SessionState,
    revision: u64,
    changed_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecordV2 {
    version: u8,
    session_id: String,
    created_at: String,
    state: SessionState,
    favorited: bool,
    archived_at: Option<String>,
    restored_at: Option<String>,
    managed_worktree: Option<ManagedWorktreeRecord>,
    revision: u64,
    changed_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OrganizationRecord {
    pub(crate) version: u8,
    pub(crate) session_id: String,
    pub(crate) created_at: String,
    pub(crate) state: SessionState,
    pub(crate) favorited: bool,
    pub(crate) archived_at: Option<String>,
    pub(crate) restored_at: Option<String>,
    pub(crate) managed_worktree: Option<ManagedWorktreeRecord>,
    pub(crate) revision: u64,
    pub(crate) changed_at: String,
}

impl OrganizationRecord {
    pub(crate) fn protects_from_auto_delete(&self) -> bool {
        self.favorited
            || self
                .managed_worktree
                .as_ref()
                .is_some_and(|worktree| worktree.cleanup_status.protects_from_deletion())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OrganizationStateDetails {
    pub(crate) state: SessionState,
    pub(crate) revision: u64,
    pub(crate) favorited: bool,
    pub(crate) archived_at: Option<String>,
    pub(crate) restored_at: Option<String>,
    pub(crate) managed_worktree: Option<ManagedWorktreeRecord>,
}

impl OrganizationStateDetails {
    pub(crate) fn protects_from_auto_delete(&self) -> bool {
        self.favorited
            || self
                .managed_worktree
                .as_ref()
                .is_some_and(|worktree| worktree.cleanup_status.protects_from_deletion())
    }
}

pub(crate) struct OrganizationStore {
    root: PathBuf,
    pub(crate) instance_id: String,
    pending: Mutex<HashSet<String>>,
}

/// Whether any durable organization record under `root` (the
/// `session-state/` directory) requires the same retention protection the
/// staged cleanup loop applies: the owning read path
/// (`OrganizationStore::read`) protects anything it would report as an
/// error — bounded-size violations, decode failures, non-current record
/// fields (unknown `version`, a stored identity that does not match its file
/// name, zero `revision`, a persisted `active` state the store never
/// writes) and every invalid-name record — plus any `favorited: true` record
/// (including pre-upgrade `version: 1` `archived` records, which auto-promote
/// to `favorited: true` on read) and any non-removed `managedWorktree`
/// (`active`, `retained_dirty`, or `removal_failed`). Ordinary non-favorited
/// `closed` or clean `archived` records do not protect a deleted source's
/// history. Startup workspace GC uses this as its conservative protection
/// check; identity-bound staged retention (`runtime::retention`) remains the
/// authoritative runtime protection.
pub(crate) fn dir_has_retention_protected_records(root: &Path) -> bool {
    let mut entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        // A missing directory holds nothing to protect; an existing but
        // unlistable directory protects exactly like an unreadable record
        // file. (No record can exist under a non-directory path.)
        Err(_) => {
            return fs::symlink_metadata(root)
                .map(|metadata| metadata.is_dir())
                .unwrap_or(false)
        }
    };
    loop {
        let Some(entry) = entries.next() else { break };
        let Ok(entry) = entry else { return true };
        if !entry.path().is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let Some(id) = file_name.strip_suffix(".json") else {
            return true;
        };
        if !is_valid_session_id(id) {
            return true;
        }
        let Ok(record) = read_record_file(&entry.path(), id) else {
            return true;
        };
        if record.protects_from_auto_delete() {
            return true;
        }
    }
    false
}

fn is_valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

/// Bounded read and strict v1/v2 decode of one durable organization record file,
/// mirroring `OrganizationStore::read`'s size and validation rules: any read
/// error, oversized file, invalid version, mismatched ID, zero revision, or
/// persisted `active` state is rejected as invalid (and therefore protected).
fn read_record_file(path: &Path, expected_id: &str) -> io::Result<OrganizationRecord> {
    let file = fs::File::open(path)?;
    decode_record_from_reader(file, expected_id)
}

fn decode_record_from_reader(file: fs::File, expected_id: &str) -> io::Result<OrganizationRecord> {
    let mut bytes = Vec::new();
    file.take(MAX_RECORD_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Session state exceeds limit",
        ));
    }
    parse_record_bytes(&bytes, expected_id)
}

fn parse_record_bytes(bytes: &[u8], expected_id: &str) -> io::Result<OrganizationRecord> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Invalid session state record")
        })?;

    let record = match version {
        1 => {
            let v1: RecordV1 = serde_json::from_value(value)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            if v1.version != RECORD_VERSION_V1
                || v1.session_id != expected_id
                || v1.revision == 0
                || v1.state == SessionState::Active
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid session state record",
                ));
            }
            // Upgrade contract (ADR 0124): pre-existing v1 archived records were
            // archived under the "exempt from retention forever" contract. Auto-promote
            // them on read to favorited: true with archivedAt = changedAt.
            let promoted_archive = v1.state == SessionState::Archived;
            OrganizationRecord {
                version: RECORD_VERSION_V1,
                session_id: v1.session_id,
                created_at: v1.created_at,
                state: v1.state,
                favorited: promoted_archive,
                archived_at: promoted_archive.then(|| v1.changed_at.clone()),
                restored_at: None,
                managed_worktree: None,
                revision: v1.revision,
                changed_at: v1.changed_at,
            }
        }
        2 => {
            let v2: RecordV2 = serde_json::from_value(value)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            if v2.version != RECORD_VERSION_V2
                || v2.session_id != expected_id
                || v2.revision == 0
                || v2.state == SessionState::Active
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid session state record",
                ));
            }
            OrganizationRecord {
                version: RECORD_VERSION_V2,
                session_id: v2.session_id,
                created_at: v2.created_at,
                state: v2.state,
                favorited: v2.favorited,
                archived_at: v2.archived_at,
                restored_at: v2.restored_at,
                managed_worktree: v2.managed_worktree,
                revision: v2.revision,
                changed_at: v2.changed_at,
            }
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported session state version",
            ));
        }
    };

    Ok(record)
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
        if !is_valid_session_id(id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid session identity",
            ));
        }
        Ok(self.root.join(format!("{id}.json")))
    }
    pub(crate) fn read(
        &self,
        id: &str,
        created_at: &str,
    ) -> io::Result<Option<OrganizationRecord>> {
        let file = match crate::durable_file::open_metadata_file(&self.path(id)?) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let record = decode_record_from_reader(file, id)?;
        // An orphan from a deleted session must never attach to a reused ID.
        Ok((record.created_at == created_at).then_some(record))
    }
    pub(crate) fn read_details(
        &self,
        id: &str,
        created_at: &str,
        closed: bool,
        runtime_active: bool,
    ) -> io::Result<OrganizationStateDetails> {
        let record = self.read(id, created_at)?;
        let revision = record.as_ref().map_or(0, |r| r.revision);
        let stored_state = record.as_ref().map(|r| r.state);
        let state = if stored_state == Some(SessionState::Archived) && !runtime_active {
            // Producer lifecycle snapshots cannot revoke sidecar organization.
            SessionState::Archived
        } else if runtime_active || !closed {
            SessionState::Active
        } else {
            stored_state.unwrap_or(SessionState::Closed)
        };
        let favorited = record.as_ref().is_some_and(|r| r.favorited);
        let archived_at = if state == SessionState::Archived {
            record.as_ref().and_then(|r| r.archived_at.clone())
        } else {
            None
        };
        let restored_at = record.as_ref().and_then(|r| r.restored_at.clone());
        let managed_worktree = record.and_then(|r| r.managed_worktree);
        Ok(OrganizationStateDetails {
            state,
            revision,
            favorited,
            archived_at,
            restored_at,
            managed_worktree,
        })
    }
    pub(crate) fn state(
        &self,
        id: &str,
        created_at: &str,
        closed: bool,
        runtime_active: bool,
    ) -> io::Result<(SessionState, u64)> {
        let details = self.read_details(id, created_at, closed, runtime_active)?;
        Ok((details.state, details.revision))
    }
    pub(crate) fn write(&self, id: &str, created_at: &str, state: SessionState) -> io::Result<()> {
        let now = crate::workspace_runtime::iso_now();
        self.write_at(id, created_at, state, &now)
    }
    pub(crate) fn write_at(
        &self,
        id: &str,
        created_at: &str,
        state: SessionState,
        now: &str,
    ) -> io::Result<()> {
        if state == SessionState::Active {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Active is runtime-derived",
            ));
        }
        let existing = self.read(id, created_at)?;
        let revision = existing
            .as_ref()
            .map_or(0, |r| r.revision)
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Session state revision exhausted"))?;
        let favorited = existing.as_ref().is_some_and(|r| r.favorited);
        let managed_worktree = existing.and_then(|r| r.managed_worktree);
        let (archived_at, restored_at) = match state {
            SessionState::Archived => (Some(now.to_owned()), None),
            SessionState::Closed => (None, Some(now.to_owned())),
            SessionState::Active => unreachable!("checked above"),
        };
        let record = RecordV2 {
            version: RECORD_VERSION_V2,
            session_id: id.into(),
            created_at: created_at.into(),
            state,
            favorited,
            archived_at,
            restored_at,
            managed_worktree,
            revision,
            changed_at: now.into(),
        };
        self.publish_v2_record(id, &record)
    }
    pub(crate) fn set_favorited(
        &self,
        id: &str,
        created_at: &str,
        favorited: bool,
    ) -> io::Result<()> {
        let now = crate::workspace_runtime::iso_now();
        self.set_favorited_at(id, created_at, favorited, &now)
    }
    pub(crate) fn set_favorited_at(
        &self,
        id: &str,
        created_at: &str,
        favorited: bool,
        now: &str,
    ) -> io::Result<()> {
        let existing = self.read(id, created_at)?;
        let revision = existing
            .as_ref()
            .map_or(0, |r| r.revision)
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Session state revision exhausted"))?;
        let stored_state = existing.as_ref().map_or(SessionState::Closed, |r| r.state);
        let archived_at = match stored_state {
            SessionState::Archived => {
                if !favorited {
                    // Unfavoriting an already-Archived session grants a fresh full
                    // archived-deletion grace window (ADR 0124).
                    Some(now.to_owned())
                } else {
                    existing
                        .as_ref()
                        .and_then(|r| r.archived_at.clone())
                        .or_else(|| Some(now.to_owned()))
                }
            }
            SessionState::Closed | SessionState::Active => None,
        };
        let restored_at = existing.as_ref().and_then(|r| r.restored_at.clone());
        let managed_worktree = existing.and_then(|r| r.managed_worktree);
        let record = RecordV2 {
            version: RECORD_VERSION_V2,
            session_id: id.into(),
            created_at: created_at.into(),
            state: stored_state,
            favorited,
            archived_at,
            restored_at,
            managed_worktree,
            revision,
            changed_at: now.into(),
        };
        self.publish_v2_record(id, &record)
    }
    fn publish_v2_record(&self, id: &str, record: &RecordV2) -> io::Result<()> {
        let bytes = serde_json::to_vec(record)?;
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
        match self.read_details(id, created_at, closed, runtime_active) {
            Ok(details) => {
                // Length-delimited JSON avoids ambiguous concatenations. This is
                // a stale-action precondition, never a credential.
                let token = serde_json::to_vec(&(
                    &self.instance_id,
                    id,
                    created_at,
                    details.revision,
                    details.state,
                    runtime_key,
                ))
                .unwrap();
                OrganizationProjection {
                    session_state: Some(details.state),
                    session_action_token: Some(hex::encode(Sha256::digest(token))),
                    session_state_error: None,
                    favorited: details.favorited,
                    archived_at: details.archived_at,
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

    #[test]
    fn v1_archived_records_auto_promote_to_favorited_with_archived_at_and_v1_closed_stays_unfavorited(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let store = OrganizationStore::new(dir.path());
        fs::create_dir_all(&store.root).unwrap();
        fs::write(
            store.path("v1-archived").unwrap(),
            r#"{"version":1,"sessionId":"v1-archived","createdAt":"2025-01-01T00:00:00Z","state":"archived","revision":2,"changedAt":"2025-02-03T04:05:06Z"}"#,
        )
        .unwrap();
        fs::write(
            store.path("v1-closed").unwrap(),
            r#"{"version":1,"sessionId":"v1-closed","createdAt":"2025-01-01T00:00:00Z","state":"closed","revision":1,"changedAt":"2025-02-03T04:05:06Z"}"#,
        )
        .unwrap();

        let archived = store
            .read_details("v1-archived", "2025-01-01T00:00:00Z", true, false)
            .unwrap();
        assert_eq!(archived.state, SessionState::Archived);
        assert_eq!(archived.revision, 2);
        assert!(archived.favorited);
        assert_eq!(
            archived.archived_at.as_deref(),
            Some("2025-02-03T04:05:06Z")
        );
        assert_eq!(archived.restored_at, None);
        assert_eq!(archived.managed_worktree, None);

        let projected = store.project(
            "v1-archived",
            "2025-01-01T00:00:00Z",
            true,
            false,
            "remembered",
        );
        assert!(projected.favorited);
        assert_eq!(
            projected.archived_at.as_deref(),
            Some("2025-02-03T04:05:06Z")
        );

        let closed = store
            .read_details("v1-closed", "2025-01-01T00:00:00Z", true, false)
            .unwrap();
        assert_eq!(closed.state, SessionState::Closed);
        assert!(!closed.favorited);
        assert_eq!(closed.archived_at, None);
    }

    #[test]
    fn v2_writes_persist_favorited_across_archive_restore_and_reset_clocks() {
        let dir = tempfile::tempdir().unwrap();
        let store = OrganizationStore::new(dir.path());

        // Favorite while active/closed, then archive, then restore, then unfavorite while archived.
        store
            .set_favorited_at("s1", "2026-01-01T00:00:00Z", true, "2026-01-02T00:00:00Z")
            .unwrap();
        let raw: serde_json::Value =
            serde_json::from_slice(&fs::read(store.path("s1").unwrap()).unwrap()).unwrap();
        assert_eq!(raw["version"], 2);
        assert_eq!(raw["favorited"], true);
        assert!(raw["managedWorktree"].is_null());

        store
            .write_at(
                "s1",
                "2026-01-01T00:00:00Z",
                SessionState::Archived,
                "2026-01-05T00:00:00Z",
            )
            .unwrap();
        let archived = store
            .read_details("s1", "2026-01-01T00:00:00Z", true, false)
            .unwrap();
        assert_eq!(archived.state, SessionState::Archived);
        assert!(archived.favorited);
        assert_eq!(
            archived.archived_at.as_deref(),
            Some("2026-01-05T00:00:00Z")
        );
        assert_eq!(archived.restored_at, None);

        // Restore clears archivedAt, stamps restoredAt, and preserves favorited.
        store
            .write_at(
                "s1",
                "2026-01-01T00:00:00Z",
                SessionState::Closed,
                "2026-01-06T00:00:00Z",
            )
            .unwrap();
        let restored = store
            .read_details("s1", "2026-01-01T00:00:00Z", true, false)
            .unwrap();
        assert_eq!(restored.state, SessionState::Closed);
        assert!(restored.favorited);
        assert_eq!(restored.archived_at, None);
        assert_eq!(
            restored.restored_at.as_deref(),
            Some("2026-01-06T00:00:00Z")
        );

        // Re-archive, then unfavorite -> resets archivedAt = now for fresh deletion window.
        store
            .write_at(
                "s1",
                "2026-01-01T00:00:00Z",
                SessionState::Archived,
                "2026-01-07T00:00:00Z",
            )
            .unwrap();
        store
            .set_favorited_at("s1", "2026-01-01T00:00:00Z", false, "2026-02-20T12:00:00Z")
            .unwrap();
        let unfavorited = store
            .read_details("s1", "2026-01-01T00:00:00Z", true, false)
            .unwrap();
        assert_eq!(unfavorited.state, SessionState::Archived);
        assert!(!unfavorited.favorited);
        assert_eq!(
            unfavorited.archived_at.as_deref(),
            Some("2026-02-20T12:00:00Z")
        );
    }

    #[test]
    fn v2_round_trips_managed_worktree_and_fails_closed_on_unknown_version() {
        let dir = tempfile::tempdir().unwrap();
        let store = OrganizationStore::new(dir.path());
        fs::create_dir_all(&store.root).unwrap();
        let wt_json = serde_json::json!({
            "version": 2,
            "sessionId": "wt1",
            "createdAt": "2026-01-01T00:00:00Z",
            "state": "archived",
            "favorited": false,
            "archivedAt": "2026-01-05T00:00:00Z",
            "restoredAt": null,
            "managedWorktree": {
                "root": "/tmp/.orkworks/worktrees/abc/wt1",
                "commonDir": "/tmp/repo/.git",
                "branch": "orkworks/test-123456",
                "baseRef": "origin/main",
                "cleanupStatus": "retained_dirty",
                "cleanupNote": "2 modified files",
                "hookNote": null,
                "managedHookFiles": [
                    { "relativePath": ".claude/settings.local.json", "sha256": "deadbeef" }
                ]
            },
            "revision": 1,
            "changedAt": "2026-01-05T00:00:00Z"
        });
        fs::write(
            store.path("wt1").unwrap(),
            serde_json::to_vec(&wt_json).unwrap(),
        )
        .unwrap();
        let details = store
            .read_details("wt1", "2026-01-01T00:00:00Z", true, false)
            .unwrap();
        assert!(details.protects_from_auto_delete());
        assert_eq!(
            details.managed_worktree.as_ref().unwrap().cleanup_status,
            ManagedWorktreeCleanupStatus::RetainedDirty
        );
        // Preserves managedWorktree across favorite toggles.
        store
            .set_favorited_at("wt1", "2026-01-01T00:00:00Z", true, "2026-01-06T00:00:00Z")
            .unwrap();
        let after = store
            .read_details("wt1", "2026-01-01T00:00:00Z", true, false)
            .unwrap();
        assert_eq!(
            after.managed_worktree.as_ref().unwrap().branch,
            "orkworks/test-123456"
        );

        // Unknown version fails closed without overwriting.
        let future = br#"{"version":99,"sessionId":"future","createdAt":"2026-01-01T00:00:00Z","state":"closed","revision":1,"changedAt":"2026-01-01T00:00:00Z"}"#;
        fs::write(store.path("future").unwrap(), future).unwrap();
        let projected = store.project("future", "2026-01-01T00:00:00Z", true, false, "remembered");
        assert_eq!(projected.session_state, None);
        assert_eq!(projected.session_action_token, None);
        assert!(projected.session_state_error.is_some());
        assert!(store
            .set_favorited("future", "2026-01-01T00:00:00Z", true)
            .is_err());
        assert_eq!(fs::read(store.path("future").unwrap()).unwrap(), future);
    }
}
