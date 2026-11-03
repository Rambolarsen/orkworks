//! Acceptance tests for the dormant hierarchy storage kernel. Golden inputs
//! are the merged child-source component vectors; every rejection case mirrors
//! the owning contracts. Wired only here — the module has no production caller.

use super::connection::Connection;
use tempfile::tempdir;

#[test]
fn complete_catalog_column_shape_verifies() {
    let root = tempdir().unwrap();
    let path = root.path().join("catalog.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(store.unavailable_reason(), None);
    drop(store);
    super::schema::verify_catalog_shape(&path).expect("fresh catalog shape is complete");
}

#[test]
fn foreign_column_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("snooping.sqlite");
    drop(Connection::open(&path, "workspace-1").unwrap());
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("ALTER TABLE objects ADD COLUMN snoop TEXT;")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("catalog"),
        "unknown column fences rather than repairs"
    );
}

// The following three cases pin scope-1 refusal behavior that shipped in
// Task 1; they are acceptance pins against regression, not new behavior.

#[test]
fn foreign_page_size_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("foreign_page.sqlite");
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.pragma_update(None, "page_size", 512).unwrap();
        raw.execute_batch("CREATE TABLE foreign_store (x INTEGER);")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("page size"),
        "non-4096-byte page configuration is unrepairable state"
    );
}

#[test]
fn foreign_wal_mode_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("foreign_wal.sqlite");
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.pragma_update(None, "journal_mode", "WAL").unwrap();
        raw.execute_batch("CREATE TABLE foreign_store (x INTEGER);")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("journal"),
        "the accepted store never runs in WAL mode, and never migrates one"
    );
}

#[test]
fn unknown_schema_version_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("unknown_version.sqlite");
    drop(Connection::open(&path, "workspace-1").unwrap());
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("UPDATE store_header SET schema_version = 5;")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("schema version"),
        "unknown format versions fence the store; nothing is migrated"
    );
}

#[test]
fn out_of_scope_tables_stay_empty() {
    let root = tempdir().unwrap();
    let path = root.path().join("empty_out_of_scope.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(store.unavailable_reason(), None);
    drop(store);
    assert_eq!(
        super::schema::empty_out_of_scope_tables(&path).unwrap(),
        super::schema::OUT_OF_SCOPE,
        "out-of-scope catalog tables exist and are untouched by this kernel"
    );
}

#[test]
fn fresh_open_creates_store() {
    let root = tempdir().unwrap();
    let path = root.path().join("hierarchy").join("v1.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(store.unavailable_reason(), None, "fresh store is available");
    assert_eq!(store.workspace_id(), "workspace-1");
    assert!(!store.epoch().is_empty(), "fresh store owns a random epoch");
    drop(store);
    assert!(path.exists(), "catalog must be created on disk");
}

#[test]
fn reopen_verifies_epoch_and_workspace() {
    let root = tempdir().unwrap();
    let path = root.path().join("store.sqlite");
    let epoch = {
        let store = Connection::open(&path, "workspace-1").unwrap();
        store.epoch().to_string()
    };
    let reopened = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(reopened.unavailable_reason(), None);
    assert_eq!(
        reopened.epoch(),
        &epoch,
        "reopen must not re-roll the epoch"
    );
    assert_eq!(reopened.workspace_id(), "workspace-1");
}

#[test]
fn workspace_mismatch_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("store.sqlite");
    drop(Connection::open(&path, "workspace-1").unwrap());
    let store = Connection::open(&path, "other-workspace").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("workspace"),
        "workspace mismatch fences the store"
    );
}

#[test]
fn non_sqlite_file_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("store.sqlite");
    std::fs::write(&path, b"definitely not a database").unwrap();
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("physical database"),
        "a foreign magic file must not be repaired"
    );
}

#[test]
fn foreign_table_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("store.sqlite");
    drop(Connection::open(&path, "workspace-1").unwrap());
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("CREATE TABLE snooping (x INTEGER);")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("catalog"),
        "unknown table fences rather than repairs"
    );
}
