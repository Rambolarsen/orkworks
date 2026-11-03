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
