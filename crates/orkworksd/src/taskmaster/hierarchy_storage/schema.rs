//! DDL-free store opener pieces: accepted physical configuration, the
//! complete accepted format-4 catalog, and structural catalog verification.
//! Unknown objects make the store unavailable; nothing here repairs.

use super::{ensure, Result, MAX_PAGE_COUNT, PAGE_SIZE};

/// The complete accepted format-4 catalog: every contract table (including
/// out-of-scope tables created empty, per the whole-catalog rule) plus the
/// accepted child additions and the artifact-producer additions.
pub(super) const CATALOG: &[&str] = &[
    "artifact_read_debits",
    "candidate_work",
    "child_allocations",
    "child_attachments",
    "child_plan_approvals",
    "child_reservations",
    "commit_proof",
    "dependencies",
    "evaluation_artifact_streams",
    "heads",
    "objects",
    "payloads",
    "pending_material",
    "quota_usage",
    "receipt_slots",
    "store_header",
    "summary_state",
];

/// Uniform logical-row physical layout: canonical key BLOB, canonical
/// projected value BLOB, canonical `lastCommit` CommitRef BLOB and a
/// denormalized integer lookup index derived from that CommitRef.
const ROW_TABLE_DDL: &str = "CREATE TABLE {} (\n  key BLOB PRIMARY KEY,\n  value BLOB NOT NULL,\n  last_commit BLOB NOT NULL,\n  sequence INTEGER NOT NULL\n) WITHOUT ROWID;";

/// Apply the accepted physical configuration to a fresh or existing store.
/// `page_size` only takes effect while the database is empty; it is verified
/// on every apply.
pub(crate) fn configure_physical(database: &rusqlite::Connection) -> Result<()> {
    database
        .pragma_update(None, "page_size", PAGE_SIZE)
        .map_err(|_| super::HierarchyStorageError("page size"))?;
    database
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(|_| super::HierarchyStorageError("journal"))?;
    database
        .pragma_update(None, "synchronous", 3_i64)
        .map_err(|_| super::HierarchyStorageError("physical database"))?;
    database
        .pragma_update(None, "foreign_keys", true)
        .map_err(|_| super::HierarchyStorageError("physical database"))?;
    database
        .pragma_update(None, "temp_store", 2_i64)
        .map_err(|_| super::HierarchyStorageError("physical database"))?;
    database
        .pragma_update(None, "max_page_count", MAX_PAGE_COUNT)
        .map_err(|_| super::HierarchyStorageError("physical database"))?;
    // journal_mode readback is a persistent property; page_size readback must
    // match what was configured (or what an existing store was created with).
    ensure(page_size(database)? == PAGE_SIZE, "page size")
}

fn page_size(database: &rusqlite::Connection) -> Result<i64> {
    database
        .pragma_query_value(None, "page_size", |row| row.get::<_, i64>(0))
        .map_err(|_| super::HierarchyStorageError("physical database"))
}

/// Create the complete catalog on a fresh, physically configured connection.
pub(crate) fn create_catalog(database: &rusqlite::Connection) -> Result<()> {
    let mut batch = String::from(
        "CREATE TABLE store_header (\n  singleton INTEGER PRIMARY KEY CHECK (singleton = 1),\n  schema_version INTEGER NOT NULL,\n  workspace_id TEXT NOT NULL,\n  storage_epoch TEXT NOT NULL,\n  latest BLOB NOT NULL\n);\n",
    );
    batch.push_str(
        "CREATE TABLE commit_proof (\n  sequence INTEGER PRIMARY KEY,\n  body BLOB NOT NULL,\n  change_digest TEXT NOT NULL\n);\n",
    );
    for table in CATALOG {
        if *table == "store_header" || *table == "commit_proof" {
            continue;
        }
        batch.push_str(&ROW_TABLE_DDL.replace("{}", table));
        batch.push('\n');
    }
    database
        .execute_batch(&batch)
        .map_err(|_| super::HierarchyStorageError("catalog"))
}

/// Structural catalog verification on an existing store. Unknown tables,
/// views, triggers or indexes make the store unavailable; nothing is repaired.
pub(crate) fn verify_catalog(database: &rusqlite::Connection) -> Result<()> {
    let mut statement = database
        .prepare("SELECT type, name FROM sqlite_master")
        .map_err(|_| super::HierarchyStorageError("physical database"))?;
    let mut rows = statement
        .query([])
        .map_err(|_| super::HierarchyStorageError("physical database"))?;
    let mut tables = std::collections::BTreeSet::new();
    while let Some(row) = rows
        .next()
        .map_err(|_| super::HierarchyStorageError("physical database"))?
    {
        let kind: String = row
            .get(0)
            .map_err(|_| super::HierarchyStorageError("physical database"))?;
        let name: String = row
            .get(1)
            .map_err(|_| super::HierarchyStorageError("physical database"))?;
        match kind.as_str() {
            "table" => {
                ensure(
                    tables.insert(name.clone()) && CATALOG.contains(&name.as_str()),
                    "catalog",
                )?;
            }
            _ => return Err(super::HierarchyStorageError("catalog")),
        }
    }
    ensure(tables.len() == CATALOG.len(), "catalog")
}

/// Verify the persisted journal mode stays the accepted DELETE default.
pub(crate) fn verify_journal(database: &rusqlite::Connection) -> Result<()> {
    let mode: String = database
        .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
        .map_err(|_| super::HierarchyStorageError("journal"))?;
    ensure(mode == "delete", "journal")
}
