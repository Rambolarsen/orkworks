//! Acceptance tests for the dormant hierarchy storage kernel. Golden inputs
//! are the merged child-source component vectors; every rejection case mirrors
//! the owning contracts. Wired only here — the module has no production caller.

use super::connection::Connection;
use tempfile::tempdir;


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
