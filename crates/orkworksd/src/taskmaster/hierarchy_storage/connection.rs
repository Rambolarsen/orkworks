//! Store connection opener: accepted physical configuration with validated
//! page caps, DELETE journal, EXTRA synchronous mode, one retained worker
//! connection, no caller SQL, no ATTACH and no extension loading, and
//! unavailable reasons instead of repairs.

use super::schema::{apply_session_settings, verify_catalog, verify_physical};
use super::{
    canonical, domain_digest, ensure, schema, HierarchyStorageError, Result, CHANGE_DOMAIN,
    INITIALIZE_COMMAND, STORE_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;

pub(crate) struct Connection {
    database: Mutex<rusqlite::Connection>,
    workspace_id: String,
    epoch: String,
    /// Set when verification found unavailable state. Every write path checks
    /// this first and refuses with `unavailable`; verification never repairs.
    unavailable: Option<&'static str>,
}

impl Connection {
    /// Open (and, for a fresh empty path, initialize) the hierarchy store.
    ///
    /// # Errors
    /// Errors only for an invalid workspace id or a physical open/config
    /// failure. Verification failures surface as fenced-unavailable state
    /// (`unavailable_reason`); opening never repairs.
    pub(crate) fn open(path: impl AsRef<Path>, workspace_id: &str) -> Result<Connection> {
        ensure(valid_workspace(workspace_id), "workspace id")?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| HierarchyStorageError("io"))?;
        }
        let database = rusqlite::Connection::open(path)
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let mut store = Connection {
            workspace_id: workspace_id.to_string(),
            epoch: String::new(),
            unavailable: None,
            database: Mutex::new(database),
        };
        let size = std::fs::metadata(path)
            .map(|m| m.len())
            .map_err(|_| HierarchyStorageError("io"))?;
        if size == 0 {
            store.initialize_fresh()?;
        } else {
            if !verify_magic(path) {
                store.fence("physical database");
                return Ok(store);
            }
            store.verify_existing();
        }
        Ok(store)
    }

    /// `None` while the store verified available; otherwise the static reason.
    pub(crate) fn unavailable_reason(&self) -> Option<&'static str> {
        self.unavailable
    }

    /// One-transaction journal of changed rows (each `value` carries no
    /// `lastCommit`): builds the format-4 change manifest over the format-4
    /// domains, writes the proof row and replaces `header.latest` in the same
    /// transaction. Any failure rolls back before persistence. An exact
    /// retry of an identical Replace body is a no-op: nothing changes.
    pub(crate) fn journal_rows(
        &self,
        command_kind: &str,
        changes: Vec<RowChange>,
        conflict_reason: &'static str,
    ) -> Result<RowJournalCommit> {
        ensure(self.unavailable.is_none(), "unavailable")?;
        let mut database = self
            .database
            .lock()
            .map_err(|_| HierarchyStorageError("locked store"))?;
        let transaction = database
            .transaction()
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let latest_value: Value = {
            let bytes: Vec<u8> = transaction
                .query_row(
                    "SELECT latest FROM store_header WHERE singleton = 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => HierarchyStorageError("header"),
                    _ => HierarchyStorageError("physical database"),
                })?;
            serde_json::from_slice(&bytes).map_err(|_| HierarchyStorageError("header"))?
        };
        let latest = super::rows::commit_ref(&latest_value)?;
        // Monotonic commit sequence; exhaustion fences instead of wrapping,
        // well before the INTEGER lookup index overflows.
        let sequence = latest
            .sequence
            .checked_add(1)
            .ok_or(HierarchyStorageError("exhausted"))?;
        ensure(latest.storage_epoch == self.epoch, "epoch")?;
        // Change entries, sorted uniquely by UTF-8 table name then canonical
        // key bytes. The header and the new proof are control effects and are
        // excluded from the list, validated through the chain equations.
        let mut entries: Vec<(Vec<u8>, Vec<u8>, Value)> = Vec::new();
        for RowChange {
            table,
            mode,
            key,
            value,
        } in &changes
        {
            let key_bytes = canonical(key)?;
            let row_digest_after = super::rows::row_digest(table, key, value)?;
            use rusqlite::OptionalExtension;
            let existing: Option<Vec<u8>> = transaction
                .query_row(
                    &format!("SELECT value FROM {table} WHERE key = ?1"),
                    rusqlite::params![key_bytes],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| HierarchyStorageError("physical database"))?;
            let before_digest = match (mode, &existing) {
                (super::connection::RowMode::Insert, Some(_)) => {
                    return Err(HierarchyStorageError(conflict_reason));
                }
                (super::connection::RowMode::Replace, Some(bytes)) => {
                    let stored: Value = serde_json::from_slice(bytes)
                        .map_err(|_| HierarchyStorageError("physical database"))?;
                    let body = super::rows::without_field(&stored, "lastCommit");
                    // An exact retry (identical body) changes nothing: the
                    // committed chain and row links stay as they are.
                    if canonical(&body)? == canonical(value)? {
                        transaction.commit().ok();
                        return Ok(RowJournalCommit {
                            commit: latest.clone(),
                        });
                    }
                    serde_json::from_value(json!(super::rows::row_digest(table, key, &body)?))
                        .map_err(|_| HierarchyStorageError("row digest"))?
                }
                _ => Value::Null,
            };
            entries.push((
                table.as_bytes().to_vec(),
                key_bytes,
                serde_json::json!({
                    "afterDigest": row_digest_after,
                    "beforeDigest": before_digest,
                    "key": key,
                    "table": table,
                }),
            ));
        }
        entries.sort_by_key(|(table, key, _)| (table.clone(), key.clone()));
        let changes_list: Vec<Value> = entries.into_iter().map(|(_, _, entry)| entry).collect();
        let manifest = serde_json::json!({
            "changes": changes_list,
            "commandKind": command_kind,
            "commitSequence": sequence.to_string(),
            "predecessor": latest_value,
            "schemaVersion": super::STORE_SCHEMA_VERSION,
            "storageEpoch": self.epoch,
            "workspaceId": self.workspace_id,
        });
        let manifest_bytes = canonical(&manifest)?;
        let change_digest = super::domain_digest(super::CHANGE_DOMAIN, &manifest_bytes);
        let commit = super::rows::CommitRef {
            storage_epoch: self.epoch.clone(),
            sequence,
            change_digest: change_digest.clone(),
        };
        let commit_bytes = super::rows::commit_ref_bytes(&commit)?;
        for RowChange {
            table,
            mode,
            key,
            value,
        } in &changes
        {
            let key_bytes = canonical(key)?;
            let mut value_with_commit = value.clone();
            value_with_commit["lastCommit"] = serde_json::from_slice(&commit_bytes)
                .map_err(|_| HierarchyStorageError("header"))?;
            let value_bytes = canonical(&value_with_commit)?;
            let statement = match mode {
                super::connection::RowMode::Insert => format!(
                    "INSERT INTO {table} (key, value, last_commit, sequence) VALUES (?1, ?2, ?3, ?4)"
                ),
                super::connection::RowMode::Replace => format!(
                    "INSERT OR REPLACE INTO {table} (key, value, last_commit, sequence) VALUES (?1, ?2, ?3, ?4)"
                ),
            };
            transaction
                .execute(
                    &statement,
                    rusqlite::params![key_bytes, value_bytes, commit_bytes, sequence],
                )
                .map_err(|error| match error {
                    rusqlite::Error::SqliteFailure(failure, _)
                        if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
                    {
                        HierarchyStorageError(conflict_reason)
                    }
                    _ => HierarchyStorageError("physical database"),
                })?;
        }
        transaction
            .execute(
                "INSERT INTO commit_proof (sequence, body, change_digest) VALUES (?1, ?2, ?3)",
                rusqlite::params![sequence, manifest_bytes, change_digest],
            )
            .map_err(|_| HierarchyStorageError("physical database"))?;
        transaction
            .execute(
                "UPDATE store_header SET latest = ?1 WHERE singleton = 1",
                rusqlite::params![commit_bytes],
            )
            .map_err(|_| HierarchyStorageError("physical database"))?;
        transaction
            .commit()
            .map_err(|_| HierarchyStorageError("physical database"))?;
        Ok(RowJournalCommit { commit })
    }

    /// Canonical stored row for a key; refused when it is absent.
    pub(crate) fn read_row(&self, table: &str, key: &Value, reason: &'static str) -> Result<Value> {
        ensure(self.unavailable.is_none(), "unavailable")?;
        let key_bytes = canonical(key)?;
        let database = self
            .database
            .lock()
            .map_err(|_| HierarchyStorageError("locked store"))?;
        let bytes: Vec<u8> = database
            .query_row(
                &format!("SELECT value FROM {table} WHERE key = ?1"),
                rusqlite::params![key_bytes],
                |row| row.get(0),
            )
            .map_err(|_| HierarchyStorageError(reason))?;
        serde_json::from_slice(&bytes).map_err(|_| HierarchyStorageError(reason))
    }

    /// Read the current `header.latest` CommitRef of an available store.
    pub(crate) fn read_latest(&self) -> Result<Value> {
        ensure(self.unavailable.is_none(), "unavailable")?;
        let database = self
            .database
            .lock()
            .map_err(|_| HierarchyStorageError("locked store"))?;
        let bytes: Vec<u8> = database
            .query_row(
                "SELECT latest FROM store_header WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| HierarchyStorageError("header"))?;
        serde_json::from_slice(&bytes).map_err(|_| HierarchyStorageError("header"))
    }

    pub(crate) fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// Store epoch from the verified header; empty while unavailable.
    pub(crate) fn epoch(&self) -> &str {
        &self.epoch
    }

    /// Pin a verified-unavailable reason. Fenced stores stay fenced for their
    /// whole handle lifetime; the store bytes are never touched.
    fn fence(&mut self, reason: &'static str) {
        self.unavailable.get_or_insert(reason);
    }

    fn initialize_fresh(&mut self) -> Result<()> {
        let workspace_id = self.workspace_id.clone();
        let mut database = self
            .database
            .lock()
            .map_err(|_| HierarchyStorageError("locked store"))?;
        schema::configure_physical(&database)?;
        schema::create_catalog(&database)?;
        let epoch = fresh_epoch();
        // Initialization creates the empty-store initialize-store proof
        // (sequence 1, null predecessor, no changes) and header.latest
        // together, under explicit fresh-store preparation only.
        let manifest = init_manifest(&workspace_id, &epoch);
        let manifest_bytes = canonical(&manifest)?;
        let change_digest = domain_digest(CHANGE_DOMAIN, &manifest_bytes);
        let latest = canonical(&json!({
            "changeDigest": change_digest,
            "commitSequence": "1",
            "storageEpoch": epoch,
        }))?;
        {
            let transaction = database
                .transaction()
                .map_err(|_| HierarchyStorageError("physical database"))?;
            transaction
                .execute(
                    "INSERT INTO commit_proof (sequence, body, change_digest) VALUES (1, ?1, ?2)",
                    rusqlite::params![manifest_bytes, change_digest],
                )
                .map_err(|_| HierarchyStorageError("physical database"))?;
            transaction
                .execute(
                    "INSERT INTO store_header (singleton, schema_version, workspace_id, storage_epoch, latest)
                     VALUES (1, ?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        STORE_SCHEMA_VERSION,
                        workspace_id,
                        epoch,
                        latest
                    ],
                )
                .map_err(|_| HierarchyStorageError("physical database"))?;
            transaction
                .commit()
                .map_err(|_| HierarchyStorageError("physical database"))?;
        }
        drop(database);
        self.epoch = epoch;
        Ok(())
    }

    fn verify_existing(&mut self) {
        // The lock guard's borrow is confined to this block; fencing happens
        // after it is released.
        let verified = (|| {
            let database = self.database.lock().map_err(|_| "locked store")?;
            // Nothing is repaired: session settings first (they mutate no
            // bytes), then only read-only persisted-state verification.
            apply_session_settings(&database).map_err(|e| e.0)?;
            verify_physical(&database).map_err(|e| e.0)?;
            verify_catalog(&database).map_err(|e| e.0)?;
            self.read_header(&database).map_err(|e| e.0)
        })();
        // Nothing is repaired: the first unavailable step fences the store.
        match verified {
            Ok(epoch) => self.epoch = epoch,
            Err(reason) => self.fence(reason),
        }
    }

    fn read_header(&self, database: &rusqlite::Connection) -> Result<String> {
        let mut statement = database
            .prepare(
                "SELECT schema_version, workspace_id, storage_epoch
                 FROM store_header WHERE singleton = 1",
            )
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let (schema_version, workspace_id, epoch) = statement
            .query_row([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|_| HierarchyStorageError("header"))?;
        drop(statement);
        ensure(schema_version == STORE_SCHEMA_VERSION, "schema version")?;
        ensure(workspace_id == self.workspace_id, "workspace")?;
        ensure(valid_epoch(&epoch), "epoch")?;
        Ok(epoch)
    }
}

/// SQLite file-identity check on the first 16 header bytes. Purely read-only:
/// this is the same physical check SQLite performs, without repairing anything.
fn verify_magic(path: &Path) -> bool {
    let magic: &[u8; 16] = b"SQLite format 3\0";
    use std::io::Read;
    std::fs::File::open(path)
        .and_then(|mut file| {
            let mut probe = [0_u8; 16];
            file.read_exact(&mut probe)?;
            Ok(probe)
        })
        .is_ok_and(|probe| &probe == magic)
}

fn valid_epoch(epoch: &str) -> bool {
    epoch.len() == 64
        && epoch
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_workspace(workspace_id: &str) -> bool {
    (1..=128).contains(&workspace_id.len())
        && workspace_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

/// Random fresh-store epoch: 32 random bytes, lowercase hex.
fn fresh_epoch() -> String {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).expect("random epoch bytes");
    hex::encode(bytes)
}

fn init_manifest(workspace_id: &str, epoch: &str) -> Value {
    json!({
        "changes": [],
        "commandKind": INITIALIZE_COMMAND,
        "commitSequence": "1",
        "predecessor": null,
        "schemaVersion": STORE_SCHEMA_VERSION,
        "storageEpoch": epoch,
        "workspaceId": workspace_id,
    })
}

/// Row write mode: immutable rows insert-only; quota rows replace with
/// `beforeDigest` tracking (replacements must differ from the stored body).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowMode {
    Insert,
    Replace,
}

/// One changed logical row for the journal: the canonical key and the
/// value WITHOUT its `lastCommit` link (excluded from row commitments).
#[derive(Clone, Debug)]
pub(crate) struct RowChange {
    pub table: &'static str,
    pub mode: RowMode,
    pub key: Value,
    pub value: Value,
}

/// The CommitRef backing the journaled rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RowJournalCommit {
    pub commit: super::rows::CommitRef,
}
