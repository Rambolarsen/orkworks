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

/// Catalog tables whose writer machinery is explicitly out of this kernel's
/// scope: created empty, never touched by a producer command here.
pub(super) const OUT_OF_SCOPE: &[&str] = &[
    "artifact_read_debits",
    "candidate_work",
    "evaluation_artifact_streams",
    "heads",
    "payloads",
    "pending_material",
    "receipt_slots",
    "summary_state",
];

/// Uniform logical-row physical layout: canonical key BLOB, canonical
/// projected value BLOB, canonical `lastCommit` CommitRef BLOB and a
/// denormalized integer lookup index derived from that CommitRef.
const ROW_TABLE_DDL: &str = "CREATE TABLE {} (\n  key BLOB PRIMARY KEY,\n  value BLOB NOT NULL,\n  last_commit BLOB NOT NULL,\n  sequence INTEGER NOT NULL\n) WITHOUT ROWID;";

/// Exact accepted column contract per catalog table: `(name, type, pk)`.
fn expected_columns(table: &str) -> &'static [(&'static str, &'static str, usize)] {
    const ROW_COLUMNS: &[(&str, &str, usize)] = &[
        ("key", "BLOB", 1),
        ("value", "BLOB", 0),
        ("last_commit", "BLOB", 0),
        ("sequence", "INTEGER", 0),
    ];
    match table {
        "store_header" => &[
            ("singleton", "INTEGER", 1),
            ("schema_version", "INTEGER", 0),
            ("workspace_id", "TEXT", 0),
            ("storage_epoch", "TEXT", 0),
            ("latest", "BLOB", 0),
        ],
        "commit_proof" => &[
            ("sequence", "INTEGER", 1),
            ("body", "BLOB", 0),
            ("change_digest", "TEXT", 0),
        ],
        _ => ROW_COLUMNS,
    }
}

/// Verify each catalog table's exact column set/names/types on an existing
/// store. Unknown or missing columns make the store unavailable.
fn verify_columns(database: &rusqlite::Connection) -> Result<()> {
    for table in CATALOG {
        let mut statement = database
            .prepare(&format!("PRAGMA table_info({})", table))
            .map_err(|_| super::HierarchyStorageError("physical database"))?;
        let mut rows = statement
            .query([])
            .map_err(|_| super::HierarchyStorageError("physical database"))?;
        let mut index = 0_usize;
        while let Some(row) = rows
            .next()
            .map_err(|_| super::HierarchyStorageError("physical database"))?
        {
            let name: String = row
                .get(1)
                .map_err(|_| super::HierarchyStorageError("physical database"))?;
            let declared: String = row
                .get(2)
                .map_err(|_| super::HierarchyStorageError("physical database"))?;
            let key = usize::try_from(
                row.get::<_, i64>(5)
                    .map_err(|_| super::HierarchyStorageError("physical database"))?,
            )
            .map_err(|_| super::HierarchyStorageError("catalog"))?;
            match expected_columns(table).get(index) {
                Some((expected_name, expected_type, expected_key))
                    if name == *expected_name
                        && declared.eq_ignore_ascii_case(expected_type)
                        && key == *expected_key =>
                {
                    index += 1;
                }
                _ => return Err(super::HierarchyStorageError("catalog")),
            }
        }
        ensure(index == expected_columns(table).len(), "catalog")?;
    }
    Ok(())
}

/// Apply the accepted physical configuration to a **fresh** store only. This
/// runs before any catalog exists; `page_size` and `journal_mode` here are
/// creating state, not repairing it.
pub(crate) fn configure_physical(database: &rusqlite::Connection) -> Result<()> {
    database
        .pragma_update(None, "page_size", PAGE_SIZE)
        .map_err(|_| super::HierarchyStorageError("page size"))?;
    database
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(|_| super::HierarchyStorageError("journal"))?;
    apply_session_settings(database)?;
    verify_physical(database)
}

/// Session-only settings re-applied for every connection. None of these
/// mutate database bytes or change persisted modes.
pub(crate) fn apply_session_settings(database: &rusqlite::Connection) -> Result<()> {
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
        .map_err(|_| super::HierarchyStorageError("physical database"))
}

/// Read-only verification of persisted physical state on an existing store.
pub(crate) fn verify_physical(database: &rusqlite::Connection) -> Result<()> {
    let read_page: i64 = database
        .pragma_query_value(None, "page_size", |row| row.get::<_, i64>(0))
        .map_err(|_| super::HierarchyStorageError("physical database"))?;
    ensure(read_page == PAGE_SIZE, "page size")?;
    verify_journal(database)
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
    ensure(tables.len() == CATALOG.len(), "catalog")?;
    // Column-level refusal: unknown/missing/mis-typed columns are unreplicable
    // state, so the store is fenced rather than migrated.
    verify_columns(database)
}

/// Column-level shape verification against a database file path (test entry
/// point); `Connection::open` runs the same check through `verify_catalog`.
pub(crate) fn verify_catalog_shape(path: &std::path::Path) -> Result<()> {
    let database = rusqlite::Connection::open(path)
        .map_err(|_| super::HierarchyStorageError("physical database"))?;
    verify_catalog(&database)?;
    verify_columns(&database)
}

/// The out-of-scope catalog tables that must exist and stay empty in every
/// database this kernel creates.
pub(crate) fn empty_out_of_scope_tables(path: &std::path::Path) -> Result<Vec<String>> {
    let database = rusqlite::Connection::open(path)
        .map_err(|_| super::HierarchyStorageError("physical database"))?;
    let mut empty = Vec::new();
    for table in OUT_OF_SCOPE {
        let count: i64 = database
            .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
                row.get(0)
            })
            .map_err(|_| super::HierarchyStorageError("catalog"))?;
        ensure(count == 0, "catalog")?;
        empty.push((*table).to_string());
    }
    Ok(empty)
}

/// Verify the persisted journal mode stays the accepted DELETE default.
pub(crate) fn verify_journal(database: &rusqlite::Connection) -> Result<()> {
    let mode: String = database
        .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
        .map_err(|_| super::HierarchyStorageError("journal"))?;
    ensure(mode == "delete", "journal")
}
