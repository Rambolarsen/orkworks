//! Private ordinary-session persistence. Production selection belongs to #1244.
#![allow(dead_code)] // Isolated adapter until recoverable format-2 activation.
mod conditional_attention;
mod internal_mutations;
#[allow(unused_imports)]
pub(crate) use conditional_attention::AttentionProof;
#[allow(unused_imports)] // Internal API becomes a production path only at qualified cutover.
pub(crate) use internal_mutations::InternalMutationResult;
mod queries;
mod worker;
#[allow(unused_imports)] // Query DTOs are consumed by the later format-2 adapter.
pub(crate) use queries::{Cursor, HistoryQuery, Page, Summary};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use std::{path::Path, time::Duration};
#[cfg(test)]
pub(crate) use worker::testing as test_worker;
#[allow(unused_imports)] // Format selection is intentionally not enabled here.
pub(crate) use worker::{
    ExecutionRequest, InternalCompletion, InternalMutationTicket, PreparedInternalMutation,
    RuntimeWriteResources, SessionStore,
};

#[derive(Debug)]
pub(crate) enum StoreError {
    Conflict,
    Missing,
    Unauthorized,
    Invalid(String),
    Unsupported,
    Corrupt,
    Busy,
    DiskFull,
    TooLarge,
    QueueFull,
    Expired,
    Cancelled,
    Unavailable,
    Io(std::io::Error),
    Sql(rusqlite::Error),
}
impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "session storage: {self:?}")
    }
}
impl std::error::Error for StoreError {}
impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        use rusqlite::ErrorCode::*;
        match e.sqlite_error_code() {
            Some(DatabaseBusy | DatabaseLocked) => Self::Busy,
            Some(DiskFull) => Self::DiskFull,
            Some(DatabaseCorrupt | NotADatabase) => Self::Corrupt,
            Some(OperationInterrupted) => Self::Expired,
            _ => Self::Sql(e),
        }
    }
}
impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Invalid(e.to_string())
    }
}
type Result<T> = std::result::Result<T, StoreError>;
const APP_ID: i64 = 0x4f575353;
const MAX_RECORD: usize = 1024 * 1024;

struct Database {
    conn: Connection,
    attention_owners:
        std::collections::HashMap<String, crate::runtime::observed_status::AttentionOwner>,
}
impl Database {
    // Only explicit staging creation may create a file. Open never recreates it.
    fn create(path: &Path, workspace: &str) -> Result<Self> {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        let mut conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        Self::configure(&conn)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(SCHEMA)?;
        tx.execute(
            "INSERT INTO store_header VALUES (1, 1, ?1, ?2, 0, NULL)",
            rusqlite::params![workspace, uuid::Uuid::new_v4().to_string()],
        )?;
        tx.pragma_update(None, "application_id", APP_ID)?;
        tx.pragma_update(None, "user_version", 1)?;
        tx.commit()?;
        Ok(Self {
            conn,
            attention_owners: Default::default(),
        })
    }
    fn open(path: &Path, workspace: &str) -> Result<Self> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        conn.busy_timeout(Duration::from_millis(100))?;
        let app: i64 = conn.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if app != APP_ID || version != 1 {
            return Err(StoreError::Unsupported);
        }
        let (version, stored, epoch, generation): (i64, String, String, i64) = conn.query_row(
            "SELECT schema_version,workspace,epoch,generation FROM store_header WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        if version != 1 {
            return Err(StoreError::Unsupported);
        }
        if stored != workspace {
            return Err(StoreError::Conflict);
        }
        if uuid::Uuid::parse_str(&epoch).is_err() || generation < 0 {
            return Err(StoreError::Corrupt);
        }
        let integrity: String = conn.pragma_query_value(None, "quick_check", |r| r.get(0))?;
        if integrity != "ok" {
            return Err(StoreError::Corrupt);
        }
        Self::configure(&conn)?;
        // Preparing an actual FTS5 query verifies the bundled capability/schema.
        conn.prepare("SELECT id FROM metadata_search WHERE metadata_search MATCH ?1 LIMIT 1")?;
        Ok(Self {
            conn,
            attention_owners: Default::default(),
        })
    }
    fn create_session(&mut self, mut document: serde_json::Value) -> Result<()> {
        document
            .as_object_mut()
            .ok_or_else(|| StoreError::Invalid("metadata must be an object".into()))?
            .insert("metadataRevision".into(), serde_json::json!("1"));
        self.insert(document)
    }
    fn insert(&mut self, document: serde_json::Value) -> Result<()> {
        let (document, record) = canonical(document)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx
            .query_row(
                "SELECT 1 FROM sessions WHERE id=?1",
                [&record.metadata.id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .is_some()
        {
            return Err(StoreError::Conflict);
        }
        persist(&tx, &document, &record)?;
        bump_generation(&tx)?;
        tx.commit()?;
        self.accepted_attention(&record.metadata);
        Ok(())
    }
    fn get(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let _budget = queries::ReadBudget::new(&self.conn);
        read_document(&self.conn, id, false)
    }
    fn patch(&mut self, id: &str, envelope: serde_json::Value) -> Result<serde_json::Value> {
        let patch: crate::metadata::agent_patch::AgentPatchEnvelope =
            serde_json::from_value(envelope)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let original = read_document(&tx, id, false)?.ok_or(StoreError::Missing)?;
        let mut record: crate::metadata::session_transactions::RevisionedSession =
            serde_json::from_value(original.clone())?;
        if record.revision != patch.metadata_revision {
            return Err(StoreError::Conflict);
        }
        let next = record.revision.next()?;
        let old = serde_json::to_value(&record)?;
        patch
            .patch
            .apply_checked(&mut record.metadata)
            .map_err(|e| match e {
                crate::metadata::agent_patch::AgentPatchError::Conflict => StoreError::Conflict,
                crate::metadata::agent_patch::AgentPatchError::Unauthorized => {
                    StoreError::Unauthorized
                }
                crate::metadata::agent_patch::AgentPatchError::BadRequest => {
                    StoreError::Invalid("invalid agent metadata".into())
                }
                crate::metadata::agent_patch::AgentPatchError::Io(e) => StoreError::Io(e),
            })?;
        let scope = patch.patch.scope();
        let now = crate::workspace_runtime::iso_now();
        if scope.attention {
            record.metadata.provenance.attention_updated_at = Some(now.clone());
        }
        if scope.work {
            record.metadata.provenance.work_metadata_updated_at = Some(now);
        }
        record.revision = next;
        let document = merge_known(original, &old, serde_json::to_value(&record)?)?;
        check_size(&document)?;
        persist(&tx, &document, &record)?;
        // No history invalidation for unrelated active output/descriptive changes.
        if record.metadata.lifecycle != "alive" {
            bump_generation(&tx)?;
        }
        tx.commit()?;
        if scope.attention {
            self.accepted_attention(&record.metadata);
        }
        Ok(document)
    }
    fn organize(
        &mut self,
        id: &str,
        created: &str,
        revision: &str,
        choice: &str,
    ) -> Result<String> {
        if !matches!(choice, "closed" | "archived") {
            return Err(StoreError::Invalid("active is runtime-derived".into()));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state: Option<String> = tx
            .query_row(
                "SELECT state FROM sessions WHERE id=?1 AND created_at=?2 AND deleted=0",
                rusqlite::params![id, created],
                |r| r.get(0),
            )
            .optional()?;
        if !matches!(state.as_deref(), Some("closed" | "archived")) {
            return Err(StoreError::Conflict);
        }
        let current: String = tx
            .query_row("SELECT revision FROM organization WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?
            .unwrap_or_else(|| "0".into());
        if current != revision {
            return Err(StoreError::Conflict);
        }
        let next = String::from(
            crate::metadata::session_transactions::MetadataRevision::try_from(current)
                .map_err(|_| StoreError::Corrupt)?
                .next()?,
        );
        tx.execute("INSERT INTO organization VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET choice=excluded.choice,revision=excluded.revision,changed_at=excluded.changed_at",
            rusqlite::params![id,created,choice,next,crate::workspace_runtime::iso_now()])?;
        tx.execute(
            "UPDATE sessions SET state=?2 WHERE id=?1",
            rusqlite::params![id, choice],
        )?;
        bump_generation(&tx)?;
        tx.commit()?;
        Ok(next)
    }
    fn begin_delete(
        &mut self,
        id: &str,
        created: &str,
        revision: &str,
        organization_revision: &str,
    ) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row: Option<(String, String)> = tx
            .query_row(
                "SELECT state,revision FROM sessions WHERE id=?1 AND created_at=?2 AND deleted=0",
                rusqlite::params![id, created],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let current_organization: String = tx
            .query_row(
                "SELECT revision FROM organization WHERE id=?1 AND created_at=?2",
                rusqlite::params![id, created],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or_else(|| "0".into());
        if current_organization != organization_revision
            || !row.is_some_and(|(state, r)| {
                matches!(state.as_str(), "closed" | "archived") && r == revision
            })
        {
            return Err(StoreError::Conflict);
        }
        tx.execute(
            "INSERT INTO deletion_jobs VALUES(?1,?2,0)",
            rusqlite::params![id, created],
        )?;
        tx.execute("UPDATE sessions SET deleted=1 WHERE id=?1", [id])?;
        tx.execute("DELETE FROM metadata_search WHERE rowid=(SELECT search_rowid FROM sessions WHERE id=?1)", [id])?;
        bump_generation(&tx)?;
        tx.commit()?;
        Ok(())
    }
    // Progress is an owner-defined monotonic bit set; completion is a separate
    // explicit cleanup-owner assertion, not a claim of external-file atomicity.
    fn deletion_progress(
        &mut self,
        id: &str,
        created: &str,
        progress: u8,
        complete: bool,
    ) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<u8> = tx
            .query_row(
                "SELECT progress FROM deletion_jobs WHERE id=?1 AND created_at=?2",
                rusqlite::params![id, created],
                |r| r.get(0),
            )
            .optional()?;
        let current = current.ok_or(StoreError::Conflict)?;
        if current & progress != current {
            return Err(StoreError::Conflict);
        }
        if complete {
            tx.execute(
                "DELETE FROM sessions WHERE id=?1 AND created_at=?2 AND deleted=1",
                rusqlite::params![id, created],
            )?;
        } else {
            tx.execute(
                "UPDATE deletion_jobs SET progress=?2 WHERE id=?1",
                rusqlite::params![id, progress],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    fn configure(conn: &Connection) -> Result<()> {
        conn.busy_timeout(Duration::from_millis(100))?;
        conn.pragma_update(None, "journal_mode", "DELETE")?;
        conn.pragma_update(None, "synchronous", "EXTRA")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let mode: String = conn.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
        let sync: i64 = conn.pragma_query_value(None, "synchronous", |r| r.get(0))?;
        let fk: i64 = conn.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
        if mode != "delete" || sync != 3 || fk != 1 {
            return Err(StoreError::Unsupported);
        }
        // Use SQLite's own unicode61 tokenizer for query normalization. These
        // per-connection temporary rows never enter the workspace database.
        conn.execute_batch(
            "CREATE VIRTUAL TABLE temp.query_tokens USING fts5(text,tokenize='unicode61');
            CREATE VIRTUAL TABLE temp.query_vocabulary USING fts5vocab(query_tokens,instance);",
        )?;
        Ok(())
    }
}
fn check_size(document: &serde_json::Value) -> Result<()> {
    if serde_json::to_vec(document)?.len() > MAX_RECORD {
        return Err(StoreError::TooLarge);
    }
    Ok(())
}
fn canonical(
    document: serde_json::Value,
) -> Result<(
    serde_json::Value,
    crate::metadata::session_transactions::RevisionedSession,
)> {
    check_size(&document)?;
    let mut record: crate::metadata::session_transactions::RevisionedSession =
        serde_json::from_value(document.clone())?;
    if record.metadata.id.is_empty()
        || record.metadata.id.len() > 256
        || record.metadata.created_at.is_empty()
        || record.metadata.created_at.len() > 256
        || !record
            .metadata
            .id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(StoreError::Invalid("invalid creation identity".into()));
    }
    let old = serde_json::to_value(&record)?;
    record.metadata = crate::metadata::normalize_session_metadata(record.metadata);
    let document = merge_known(document, &old, serde_json::to_value(&record)?)?;
    check_size(&document)?;
    Ok((document, record))
}
fn merge_known(
    original: serde_json::Value,
    old: &serde_json::Value,
    new: serde_json::Value,
) -> Result<serde_json::Value> {
    if !original.is_object() || !old.is_object() || !new.is_object() {
        return Err(StoreError::Invalid("metadata must be an object".into()));
    }
    let mut merged = merge_member(original, old, new);
    let map = merged.as_object_mut().ok_or(StoreError::Corrupt)?;
    for alias in ["harness", "model", "phase", "providerId", "providerLabel"] {
        map.remove(alias);
    }
    Ok(merged)
}
fn merge_member(
    original: serde_json::Value,
    old: &serde_json::Value,
    new: serde_json::Value,
) -> serde_json::Value {
    use serde_json::Value;
    match (original, old, new) {
        (Value::Object(mut original), Value::Object(old), Value::Object(mut new)) => {
            // Update/remove only known members, retaining unknown siblings at
            // every object depth rather than replacing a complete typed object.
            for (key, old_value) in old {
                if let Some(new_value) = new.remove(key) {
                    let original_value = original.remove(key).unwrap_or(Value::Null);
                    original.insert(
                        key.clone(),
                        merge_member(original_value, old_value, new_value),
                    );
                } else {
                    original.remove(key);
                }
            }
            original.extend(new);
            Value::Object(original)
        }
        // Unrelated patches retain extension members in typed list entries.
        // Explicit list replacement owns that complete scope.
        (original @ Value::Array(_), Value::Array(old), Value::Array(new)) if old == &new => {
            original
        }
        (_, _, new) => new,
    }
}
fn read_document(conn: &Connection, id: &str, deleted: bool) -> Result<Option<serde_json::Value>> {
    let row: Option<(String, String, String)> = conn
        .query_row(
            "SELECT metadata,created_at,revision FROM sessions WHERE id=?1 AND deleted=?2",
            rusqlite::params![id, deleted],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    row.map(|(bytes, created, revision)| {
        if bytes.len() > MAX_RECORD {
            return Err(StoreError::TooLarge);
        }
        let document: serde_json::Value = serde_json::from_str(&bytes)?;
        let record: crate::metadata::session_transactions::RevisionedSession =
            serde_json::from_value(document.clone())?;
        if record.metadata.id != id
            || record.metadata.created_at != created
            || String::from(record.revision) != revision
        {
            return Err(StoreError::Corrupt);
        }
        Ok(document)
    })
    .transpose()
}
fn recorded_state(meta: &crate::metadata::SessionMetadata) -> &'static str {
    if matches!(meta.status.as_str(), "ended" | "killed" | "error")
        && meta.lifecycle == "dead"
        && meta.lifecycle_phase == "ended"
    {
        "closed"
    } else if meta.lifecycle == "alive"
        || matches!(
            (meta.lifecycle.as_str(), meta.lifecycle_phase.as_str()),
            ("creating", "creating") | ("stopping", "ending")
        )
    {
        "active"
    } else {
        "unknown"
    }
}
fn recency(meta: &crate::metadata::SessionMetadata) -> i64 {
    [
        meta.last_output_at.as_deref(),
        Some(meta.last_activity.as_str()),
    ]
    .into_iter()
    .flatten()
    .filter_map(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|t| t.timestamp_micros())
    })
    .max()
    .or_else(|| {
        chrono::DateTime::parse_from_rfc3339(&meta.created_at)
            .ok()
            .map(|t| t.timestamp_micros())
    })
    .unwrap_or(i64::MIN)
}
fn persist(
    conn: &Connection,
    document: &serde_json::Value,
    record: &crate::metadata::session_transactions::RevisionedSession,
) -> Result<()> {
    let m = &record.metadata;
    let state: String = conn
        .query_row(
            "SELECT choice FROM organization WHERE id=?1 AND created_at=?2",
            rusqlite::params![m.id, m.created_at],
            |r| r.get(0),
        )
        .optional()?
        .filter(|choice: &String| choice == "archived")
        .unwrap_or_else(|| recorded_state(m).into());
    let native = document
        .pointer("/resume/harnessSessionId")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    // An explicit indexed key survives VACUUM and avoids scanning FTS5's
    // UNINDEXED text identity during every live update.
    let search_rowid: Option<i64> = conn
        .query_row(
            "SELECT search_rowid FROM sessions WHERE id=?1",
            [&m.id],
            |r| r.get(0),
        )
        .optional()?;
    let search_rowid = match search_rowid {
        Some(key) => key,
        None => {
            let last: i64 = conn.query_row(
                "SELECT coalesce(max(search_rowid),0) FROM sessions",
                [],
                |r| r.get(0),
            )?;
            last.checked_add(1)
                .ok_or_else(|| StoreError::Invalid("search identity exhausted".into()))?
        }
    };
    // Retention keeps the existing lastActivity-only eligibility policy.
    // History recency has separate output/creation fallbacks and cannot own it.
    let retention_at = chrono::DateTime::parse_from_rfc3339(&m.last_activity)
        .ok()
        .map(|t| t.timestamp_micros());
    conn.execute("INSERT INTO sessions(id,created_at,revision,metadata,state,harness,model,repository,branch,native_id,outcome,recency,title,summary,search_rowid,retention_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
        ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,metadata=excluded.metadata,state=excluded.state,harness=excluded.harness,model=excluded.model,repository=excluded.repository,branch=excluded.branch,native_id=excluded.native_id,outcome=excluded.outcome,recency=excluded.recency,title=excluded.title,summary=excluded.summary,retention_at=excluded.retention_at",
        rusqlite::params![m.id,m.created_at,String::from(record.revision.clone()),serde_json::to_string(document)?,state,m.harness,m.model,m.repo_root,m.branch,native,m.terminal_outcome,recency(m),m.label,m.summary,search_rowid,retention_at])?;
    conn.execute("DELETE FROM metadata_search WHERE rowid=?1", [search_rowid])?;
    conn.execute("INSERT INTO metadata_search(rowid,id,title,summary,repository,branch,native_id) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        rusqlite::params![search_rowid,m.id,m.label,m.summary,m.repo_root,m.branch,native])?;
    Ok(())
}
fn bump_generation(conn: &Connection) -> Result<()> {
    if conn.execute(
        "UPDATE store_header SET generation=generation+1 WHERE generation<9223372036854775807",
        [],
    )? != 1
    {
        return Err(StoreError::Invalid("history generation exhausted".into()));
    }
    Ok(())
}
const SCHEMA: &str = r#"
CREATE TABLE store_header (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1), schema_version INTEGER NOT NULL,
 workspace TEXT NOT NULL, epoch TEXT NOT NULL, generation INTEGER NOT NULL CHECK(generation>=0),
 import_digest TEXT
) STRICT;
CREATE TABLE sessions (
 id TEXT PRIMARY KEY, created_at TEXT NOT NULL, revision TEXT NOT NULL,
 search_rowid INTEGER NOT NULL UNIQUE CHECK(search_rowid>0), retention_at INTEGER,
 metadata TEXT NOT NULL CHECK(length(CAST(metadata AS BLOB))<=1048576 AND json_valid(metadata)),
 state TEXT NOT NULL, harness TEXT NOT NULL, model TEXT NOT NULL,
 repository TEXT, branch TEXT, native_id TEXT, outcome TEXT,
 recency INTEGER NOT NULL, title TEXT NOT NULL, summary TEXT,
 deleted INTEGER NOT NULL DEFAULT 0 CHECK(deleted IN (0,1)), UNIQUE(id,created_at)
) STRICT;
CREATE INDEX history_state ON sessions(deleted,state,recency DESC,id DESC);
CREATE INDEX history_all ON sessions(deleted,recency DESC,id DESC);
CREATE INDEX history_harness ON sessions(deleted,state,harness,recency DESC,id DESC);
CREATE INDEX session_native ON sessions(harness,native_id);
CREATE INDEX retention_candidates ON sessions(deleted,state,retention_at,id);
CREATE TABLE organization (
 id TEXT PRIMARY KEY, created_at TEXT NOT NULL, choice TEXT NOT NULL CHECK(choice IN ('closed','archived')),
 revision TEXT NOT NULL, changed_at TEXT NOT NULL,
 FOREIGN KEY(id,created_at) REFERENCES sessions(id,created_at) ON DELETE CASCADE
) STRICT;
CREATE VIRTUAL TABLE metadata_search USING fts5(id UNINDEXED,title,summary,repository,branch,native_id,tokenize='unicode61');
CREATE TABLE deletion_jobs (
 id TEXT PRIMARY KEY, created_at TEXT NOT NULL, progress INTEGER NOT NULL CHECK(progress BETWEEN 0 AND 255),
 FOREIGN KEY(id,created_at) REFERENCES sessions(id,created_at) ON DELETE CASCADE
) STRICT;
"#;
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn document(id: &str) -> serde_json::Value {
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Before",
            "/workspace",
            "running",
            "2026-01-01T00:00:00Z",
            "2026-01-02T00:00:00Z",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        let mut value = serde_json::to_value(meta).unwrap();
        value["futureExtension"] = serde_json::json!({"nested":[1,true,"keep"]});
        value
    }
    #[test]
    fn reopen_rejects_missing_corrupt_foreign_and_future_stores_without_recreation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        assert!(Database::open(&path, "/workspace").is_err());
        assert!(!path.exists());
        drop(Database::create(&path, "/workspace").unwrap());
        assert!(matches!(
            Database::open(&path, "/foreign"),
            Err(StoreError::Conflict)
        ));
        let c = Connection::open(&path).unwrap();
        c.execute("UPDATE store_header SET schema_version=2", [])
            .unwrap();
        drop(c);
        assert!(matches!(
            Database::open(&path, "/workspace"),
            Err(StoreError::Unsupported)
        ));
        std::fs::write(&path, b"corrupt retained data").unwrap();
        assert!(matches!(
            Database::open(&path, "/workspace"),
            Err(StoreError::Corrupt)
        ));
        assert_eq!(std::fs::read(path).unwrap(), b"corrupt retained data");
    }
    #[test]
    fn checked_patch_preserves_extensions_identity_and_latest_record_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        let mut db = Database::create(&path, "/workspace").unwrap();
        db.insert(document("one")).unwrap();
        assert!(matches!(
            db.insert(document("one")),
            Err(StoreError::Conflict)
        ));
        let patch = serde_json::json!({"metadataRevision":"0","patch":{"agentMetadata":{"summary":"new summary"}}});
        let result = db.patch("one", patch.clone()).unwrap();
        assert_eq!(result["metadataRevision"], "1");
        assert_eq!(result["summary"], "new summary");
        assert_eq!(result["createdAt"], "2026-01-01T00:00:00Z");
        assert_eq!(
            result["futureExtension"],
            serde_json::json!({"nested":[1,true,"keep"]})
        );
        assert!(matches!(db.patch("one", patch), Err(StoreError::Conflict)));
        drop(db);
        let db = Database::open(&path, "/workspace").unwrap();
        assert_eq!(db.get("one").unwrap().unwrap(), result);
        assert!(db.get("missing").unwrap().is_none());
    }
    #[test]
    fn source_conflict_and_revision_exhaustion_leave_canonical_record_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let mut original = document("user");
        original["summarySource"] = serde_json::json!("user");
        db.insert(original).unwrap();
        let before = db.get("user").unwrap();
        let patch = serde_json::json!({"metadataRevision":"0","patch":{"agentMetadata":{"summary":"replace"}}});
        assert!(matches!(db.patch("user", patch), Err(StoreError::Conflict)));
        assert_eq!(db.get("user").unwrap(), before);
        let mut max = document("max");
        max["metadataRevision"] = serde_json::json!("18446744073709551615");
        db.insert(max).unwrap();
        let before = db.get("max").unwrap();
        assert!(db.patch("max",serde_json::json!({"metadataRevision":"18446744073709551615","patch":{"agentMetadata":{"summary":"replace"}}})).is_err());
        assert_eq!(db.get("max").unwrap(), before);
    }
    #[test]
    fn record_limit_rejects_oversize_without_truncating_valid_content() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let mut value = document("large");
        value["futureExtension"] = serde_json::json!("é".repeat(400_000));
        db.insert(value).unwrap();
        let before = db.get("large").unwrap().unwrap();
        assert_eq!(before["futureExtension"].as_str().unwrap().len(), 800_000);
        let mut too_large = document("oversize");
        too_large["futureExtension"] = serde_json::json!("x".repeat(MAX_RECORD));
        assert!(matches!(db.insert(too_large), Err(StoreError::TooLarge)));
        assert!(db.get("oversize").unwrap().is_none());
    }
    pub(super) fn closed_document(id: &str) -> serde_json::Value {
        let mut value = document(id);
        value["status"] = serde_json::json!("ended");
        value["lifecycle"] = serde_json::json!("dead");
        value["lifecyclePhase"] = serde_json::json!("ended");
        value
    }
    #[test]
    fn organization_uses_checked_identity_revision_and_close_first_preconditions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        let mut db = Database::create(&path, "/workspace").unwrap();
        db.insert(document("live")).unwrap();
        db.insert(closed_document("old")).unwrap();
        assert!(db
            .organize("live", "2026-01-01T00:00:00Z", "0", "archived")
            .is_err());
        assert!(db
            .organize("old", "other-creation", "0", "archived")
            .is_err());
        assert_eq!(
            db.organize("old", "2026-01-01T00:00:00Z", "0", "archived")
                .unwrap(),
            "1"
        );
        assert!(matches!(
            db.organize("old", "2026-01-01T00:00:00Z", "0", "closed"),
            Err(StoreError::Conflict)
        ));
        drop(db);
        let mut db = Database::open(&path, "/workspace").unwrap();
        assert_eq!(
            db.conn
                .query_row("SELECT state FROM sessions WHERE id='old'", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "archived"
        );
        assert_eq!(
            db.organize("old", "2026-01-01T00:00:00Z", "1", "closed")
                .unwrap(),
            "2"
        );
    }
    #[test]
    fn tombstone_hides_history_and_deletion_progress_cannot_touch_reused_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        let mut db = Database::create(&path, "/workspace").unwrap();
        db.insert(closed_document("old")).unwrap();
        assert!(db.begin_delete("old", "other", "0", "0").is_err());
        db.begin_delete("old", "2026-01-01T00:00:00Z", "0", "0")
            .unwrap();
        assert!(db.get("old").unwrap().is_none());
        assert!(matches!(
            db.insert(document("old")),
            Err(StoreError::Conflict)
        ));
        assert!(db.deletion_progress("old", "other", 1, false).is_err());
        db.deletion_progress("old", "2026-01-01T00:00:00Z", 3, false)
            .unwrap();
        drop(db);
        let mut db = Database::open(&path, "/workspace").unwrap();
        assert_eq!(
            db.conn
                .query_row(
                    "SELECT progress FROM deletion_jobs WHERE id='old'",
                    [],
                    |r| r.get::<_, u8>(0)
                )
                .unwrap(),
            3
        );
        assert!(db
            .deletion_progress("old", "2026-01-01T00:00:00Z", 1, false)
            .is_err());
        db.deletion_progress("old", "2026-01-01T00:00:00Z", 3, true)
            .unwrap();
        db.insert(document("old")).unwrap();
        assert!(db
            .deletion_progress("old", "2026-01-01T00:00:00Z", 3, true)
            .is_err());
        assert!(db.get("old").unwrap().is_some());
    }
    #[test]
    fn failed_transaction_rolls_back_canonical_search_and_generation() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        db.conn
            .execute("UPDATE store_header SET generation=9223372036854775807", [])
            .unwrap();
        assert!(db.insert(closed_document("rollback")).is_err());
        assert!(db.get("rollback").unwrap().is_none());
        assert_eq!(
            db.conn
                .query_row("SELECT count(*) FROM metadata_search", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn busy_and_disk_full_are_explicit_and_cannot_acknowledge_failed_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        let mut db = Database::create(&path, "/workspace").unwrap();
        db.insert(document("retained")).unwrap();
        let other = Connection::open(&path).unwrap();
        other.execute_batch("BEGIN IMMEDIATE").unwrap();
        let start = std::time::Instant::now();
        assert!(matches!(db.insert(document("busy")), Err(StoreError::Busy)));
        assert!(start.elapsed() < Duration::from_secs(2));
        other.execute_batch("ROLLBACK").unwrap();
        assert!(db.get("busy").unwrap().is_none());
        let pages: i64 = db
            .conn
            .pragma_query_value(None, "page_count", |r| r.get(0))
            .unwrap();
        db.conn
            .pragma_update(None, "max_page_count", pages)
            .unwrap();
        let mut big = document("full");
        big["futureExtension"] = serde_json::json!("x".repeat(500_000));
        assert!(matches!(db.insert(big), Err(StoreError::DiskFull)));
        assert!(db.get("full").unwrap().is_none());
        assert!(db.get("retained").unwrap().is_some());
    }
    #[test]
    fn invalid_or_unbounded_creation_preconditions_cannot_enter_the_store() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let mut value = document("bad");
        value["createdAt"] = serde_json::json!("x".repeat(9000));
        assert!(db.insert(value).is_err());
        assert!(db.insert(document("../escape")).is_err());
    }
    #[test]
    fn explicit_archived_delete_checks_organization_revision() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        db.insert(closed_document("archived")).unwrap();
        db.organize("archived", "2026-01-01T00:00:00Z", "0", "archived")
            .unwrap();
        assert!(db
            .begin_delete("archived", "2026-01-01T00:00:00Z", "0", "0")
            .is_err());
        db.begin_delete("archived", "2026-01-01T00:00:00Z", "0", "1")
            .unwrap();
    }
    #[test]
    fn live_patch_search_maintenance_does_not_scan_historical_rows() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let mut measurements = Vec::new();
        for (end, revision) in [(100, "0"), (1000, "1")] {
            let start = if end == 100 { 0 } else { 100 };
            for i in start..end {
                db.insert(document(&format!("row-{i}"))).unwrap();
            }
            let steps = Arc::new(AtomicUsize::new(0));
            let counter = steps.clone();
            db.conn.progress_handler(
                1,
                Some(move || {
                    counter.fetch_add(1, Ordering::Relaxed);
                    false
                }),
            );
            db.patch("row-0",serde_json::json!({"metadataRevision":revision,"patch":{"agentMetadata":{"summary":"same summary"}}})).unwrap();
            db.conn.progress_handler(0, None::<fn() -> bool>);
            measurements.push(steps.load(Ordering::Relaxed));
        }
        eprintln!("search maintenance VM steps at 100/1000 records: {measurements:?}");
        assert!(
            measurements[1] < measurements[0] * 3,
            "search maintenance grew with history: {measurements:?}"
        );
    }
    #[test]
    fn nested_unknown_metadata_survives_normalization_and_unrelated_patch() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let mut value = document("nested");
        value["resume"] = serde_json::json!({"state":"available","preferredStrategy":"exact","harnessSessionId":"native-one","latestFallback":false,"futureNative":{"version":2,"keep":[1,2,3]}});
        value["resumeOptions"] = serde_json::json!([{"strategy":"exact","label":"Exact","available":true,"preferred":true,"futureOption":{"keep":true}}]);
        db.insert(value).unwrap();
        assert_eq!(
            db.get("nested").unwrap().unwrap()["resumeOptions"][0]["futureOption"],
            serde_json::json!({"keep":true})
        );
        assert_eq!(
            db.get("nested").unwrap().unwrap()["resume"]["futureNative"],
            serde_json::json!({"version":2,"keep":[1,2,3]})
        );
        let changed=db.patch("nested",serde_json::json!({"metadataRevision":"0","patch":{"agentMetadata":{"summary":"new summary"}}})).unwrap();
        assert_eq!(
            changed["resumeOptions"][0]["futureOption"],
            serde_json::json!({"keep":true})
        );
        assert_eq!(
            changed["resume"]["futureNative"],
            serde_json::json!({"version":2,"keep":[1,2,3]})
        );
    }
    #[test]
    fn initializes_durable_workspace_store_without_touching_legacy_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        let _db = Database::create(&path, "/workspace").unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        let header: (i64, String, String, i64) = conn
            .query_row(
                "SELECT schema_version, workspace, epoch, generation FROM store_header",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(header.0, 1);
        assert_eq!(header.1, "/workspace");
        assert!(uuid::Uuid::parse_str(&header.2).is_ok());
        assert_eq!(header.3, 0);
        assert!(!dir.path().join("format.json").exists());
        assert!(!dir.path().join("sessions").exists());
    }
}
