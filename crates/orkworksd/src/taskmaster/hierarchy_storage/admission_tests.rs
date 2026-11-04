//! Production admission refusals must preserve every existing byte.
use super::mechanics::connection::Connection as MechanicalConnection;
use tempfile::tempdir;

#[test]
fn profile_fence_rejects_fresh_path_before_creating_directories() {
    let root = tempdir().unwrap();
    let parent = root.path().join("not-created");
    let result = super::connection::Connection::open(parent.join("store.sqlite"), "workspace-1");
    assert!(matches!(
        result,
        Err(super::HierarchyStorageError("unqualified profile"))
    ));
    assert!(
        !parent.exists(),
        "refusal must precede directory/database creation"
    );
}

fn directory_bytes(
    path: &std::path::Path,
) -> std::collections::BTreeMap<std::ffi::OsString, Vec<u8>> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), std::fs::read(entry.path()).unwrap())
        })
        .collect()
}

#[test]
fn profile_fence_rejects_partial_database_without_changing_any_file() {
    let root = tempdir().unwrap();
    let path = root.path().join("partial.sqlite");
    // This is the isolated, unqualified mechanical fixture, never an admitted store.
    drop(MechanicalConnection::open(&path, "workspace-1").unwrap());
    std::fs::write(root.path().join("owner-note"), b"retain original bytes").unwrap();
    let before = directory_bytes(root.path());
    let result = super::connection::Connection::open(&path, "workspace-1");
    assert!(matches!(
        result,
        Err(super::HierarchyStorageError("unqualified profile"))
    ));
    assert_eq!(directory_bytes(root.path()), before);
}

#[test]
fn profile_fence_preserves_empty_and_unknown_existing_files() {
    for bytes in [b"".as_slice(), b"unsupported store bytes\0\xff".as_slice()] {
        let root = tempdir().unwrap();
        let path = root.path().join("store.sqlite");
        std::fs::write(&path, bytes).unwrap();
        std::fs::write(
            root.path().join("store.sqlite-journal"),
            b"retain journal bytes",
        )
        .unwrap();
        let before = directory_bytes(root.path());
        let result = super::connection::Connection::open(&path, "workspace-1");
        assert!(matches!(
            result,
            Err(super::HierarchyStorageError("unqualified profile"))
        ));
        assert_eq!(directory_bytes(root.path()), before);
    }
}
