use super::*;
use crate::workspace_runtime::WorkspaceLease;
use std::sync::Arc;

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn fixture() -> (tempfile::TempDir, RecommendationStore, Arc<WorkspaceLease>) {
    let dir = tempfile::tempdir().unwrap();
    let lease = Arc::new(WorkspaceLease::acquire(dir.path()).unwrap());
    let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
    store.put(&tests::recommendation("r", "s")).unwrap();
    (dir, store, lease)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_inventory_retains_shared_owner_and_original_lease() {
    let (dir, store, lease) = fixture();
    let independent = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
    let binding = store.capture_cleanup().bind(lease.clone()).unwrap();
    let inventory = binding.inspect().unwrap();
    assert_eq!(inventory.records().len(), 1);
    assert_eq!(inventory.records()[0].id, "r");
    assert!(matches!(store.get("r"), Err(StoreError::Busy)));
    assert!(matches!(independent.get("r"), Err(StoreError::Busy)));
    drop(binding);
    drop(store);
    drop(lease);
    assert!(WorkspaceLease::acquire(dir.path()).is_err());
    drop(inventory);
    assert!(independent.get("r").unwrap().is_some());
    WorkspaceLease::acquire(dir.path()).unwrap();
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_preflight_is_read_only_and_repeatable() {
    let (_dir, store, lease) = fixture();
    let path = store.path_for("r");
    let before = fs::read(&path).unwrap();
    let binding = store.capture_cleanup().bind(lease).unwrap();
    for _ in 0..2 {
        let inventory = binding.inspect().unwrap();
        assert_eq!(inventory.records().len(), 1);
        assert_eq!(fs::read(&path).unwrap(), before);
        drop(inventory);
    }
    let edit = store.try_operation().unwrap();
    assert!(matches!(binding.inspect(), Err(StoreError::Busy)));
    drop(edit);
    assert_eq!(binding.inspect().unwrap().records()[0].id, "r");
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_reads_original_root_and_refuses_replaced_recommendation_directory() {
    let (dir, store, lease) = fixture();
    let bound = store.capture_cleanup().bind(lease.clone()).unwrap();
    let moved = dir.path().with_extension("original");
    fs::rename(dir.path(), &moved).unwrap();
    fs::create_dir(dir.path()).unwrap();
    fs::write(dir.path().join("keep"), b"replacement").unwrap();
    assert_eq!(bound.inspect().unwrap().records()[0].id, "r");
    assert!(store.capture_cleanup().bind(lease).is_err());
    let records = moved.join("recommendations");
    fs::rename(&records, moved.join("old-records")).unwrap();
    fs::create_dir(&records).unwrap();
    fs::write(records.join("keep"), b"replacement records").unwrap();
    assert!(bound.inspect().is_err());
    assert_eq!(
        fs::read(records.join("keep")).unwrap(),
        b"replacement records"
    );
    assert_eq!(fs::read(dir.path().join("keep")).unwrap(), b"replacement");
    fs::remove_dir_all(moved).unwrap();
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_absence_never_adopts_a_later_directory() {
    let (_dir, store, lease) = fixture();
    fs::remove_dir_all(&store.dir).unwrap();
    let bound = store.capture_cleanup().bind(lease).unwrap();
    assert!(bound.inspect().unwrap().records().is_empty());
    assert!(!store.dir.exists());
    fs::create_dir(&store.dir).unwrap();
    assert!(bound.inspect().is_err());
    assert!(fs::read_dir(&store.dir).unwrap().next().is_none());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_rejects_foreign_lease_and_directory_symlink() {
    let (_dir, store, lease) = fixture();
    let other = tempfile::tempdir().unwrap();
    let foreign = Arc::new(WorkspaceLease::acquire(other.path()).unwrap());
    assert!(store.capture_cleanup().bind(foreign).is_err());
    fs::rename(&store.dir, other.path().join("records")).unwrap();
    std::os::unix::fs::symlink(other.path().join("records"), &store.dir).unwrap();
    assert!(store.capture_cleanup().bind(lease).is_err());
    assert!(other.path().join("records/r.json").exists());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_rejects_ambiguous_records_and_releases_owner_on_error() {
    let (_dir, store, lease) = fixture();
    let bound = store.capture_cleanup().bind(lease).unwrap();
    let path = store.path_for("r");
    let good = fs::read(&path).unwrap();
    let duplicate = String::from_utf8(good.clone())
        .unwrap()
        .replacen('{', "{\"id\":\"r\",", 1);
    for bad in [
        b"not json".as_slice(),
        b"{} {}",
        duplicate.as_bytes(),
        br#"{"id":"r","extension":{"a":1,"a":2}}"#,
    ] {
        fs::write(&path, bad).unwrap();
        assert!(bound.inspect().is_err());
        assert_eq!(fs::read(&path).unwrap(), bad);
        assert!(store.try_operation().is_ok());
    }
    fs::write(&path, good).unwrap();
    assert_eq!(bound.inspect().unwrap().records().len(), 1);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_rejects_unsafe_leaves_without_following_or_waiting() {
    use std::os::unix::{ffi::OsStrExt, fs::symlink};
    let (dir, store, lease) = fixture();
    let bound = store.capture_cleanup().bind(lease).unwrap();
    let path = store.path_for("r");
    let foreign = dir.path().join("foreign");
    fs::rename(&path, &foreign).unwrap();
    let before = fs::read(&foreign).unwrap();
    symlink(&foreign, &path).unwrap();
    assert!(bound.inspect().is_err());
    fs::remove_file(&path).unwrap();
    fs::hard_link(&foreign, &path).unwrap();
    assert!(bound.inspect().is_err());
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(bound.inspect().is_err());
    fs::remove_dir(&path).unwrap();
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: private fixture pathname and mode; no live user files.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert!(bound.inspect().is_err());
    assert_eq!(fs::read(foreign).unwrap(), before);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_rejects_pending_recovery_unknown_names_and_invalid_graphs() {
    let (_dir, store, lease) = fixture();
    let bound = store.capture_cleanup().bind(lease).unwrap();
    let tx = store.dir.join(ROLLUP_TRANSACTION_DIR);
    fs::create_dir(&tx).unwrap();
    fs::write(tx.join("manifest"), b"unrecovered").unwrap();
    assert!(bound.inspect().is_err());
    assert_eq!(fs::read(tx.join("manifest")).unwrap(), b"unrecovered");
    fs::remove_dir_all(tx).unwrap();
    fs::write(store.dir.join("unexpected.tmp"), b"keep").unwrap();
    assert!(bound.inspect().is_err());
    fs::remove_file(store.dir.join("unexpected.tmp")).unwrap();
    fs::rename(store.path_for("r"), store.dir.join("wrong.json")).unwrap();
    assert!(bound.inspect().is_err());
    fs::rename(store.dir.join("wrong.json"), store.path_for("r")).unwrap();
    let mut bad = tests::recommendation("r", "s");
    bad.status = RecommendationStatus::RolledUp;
    bad.rolled_up_by = Some("missing".into());
    fs::write(store.path_for("r"), serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(bound.inspect().is_err());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_enforces_finite_entry_and_record_byte_bounds() {
    let (_dir, store, lease) = fixture();
    let bound = store.capture_cleanup().bind(lease).unwrap();
    fs::write(store.path_for("r"), vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(bound
        .inspect()
        .err()
        .unwrap()
        .to_string()
        .contains("record byte bound"));
    fs::remove_file(store.path_for("r")).unwrap();
    for i in 0..1024 {
        let r = tests::recommendation(&format!("r{i}"), "s");
        fs::write(store.path_for(&r.id), serde_json::to_vec(&r).unwrap()).unwrap();
    }
    assert_eq!(bound.inspect().unwrap().records().len(), 1024);
    fs::write(store.dir.join("overflow"), b"keep").unwrap();
    assert!(bound
        .inspect()
        .err()
        .unwrap()
        .to_string()
        .contains("entry bound"));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_enforces_aggregate_bytes_before_returning_partial_inventory() {
    let (_dir, store, lease) = fixture();
    let bound = store.capture_cleanup().bind(lease).unwrap();
    fs::remove_file(store.path_for("r")).unwrap();
    for i in 0..32 {
        let r = tests::recommendation(&format!("r{i}"), "s");
        let mut value = serde_json::to_value(&r).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("extension".into(), serde_json::json!(""));
        let overhead = serde_json::to_vec(&value).unwrap().len();
        value.as_object_mut().unwrap().insert(
            "extension".into(),
            serde_json::json!("x".repeat(1024 * 1024 - overhead)),
        );
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(bytes.len(), 1024 * 1024);
        fs::write(store.path_for(&r.id), bytes).unwrap();
    }
    // Exactly one MiB per record and 32 MiB aggregate must remain accepted.
    assert_eq!(bound.inspect().unwrap().records().len(), 32);
    fs::write(
        store.path_for("r32"),
        serde_json::to_vec(&tests::recommendation("r32", "s")).unwrap(),
    )
    .unwrap();
    assert!(bound
        .inspect()
        .err()
        .unwrap()
        .to_string()
        .contains("aggregate byte bound"));
    assert_eq!(fs::read_dir(&store.dir).unwrap().count(), 33);
    assert!(store.try_operation().is_ok());
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[test]
fn recommendation_cleanup_unqualified_platform_fails_before_file_effects() {
    let dir = tempfile::tempdir().unwrap();
    let lease = Arc::new(WorkspaceLease::acquire(dir.path()).unwrap());
    let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
    assert!(
        matches!(store.capture_cleanup().bind(lease), Err(StoreError::Io(e)) if e.kind() == io::ErrorKind::Unsupported)
    );
    assert!(fs::read_dir(&store.dir).unwrap().next().is_none());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn recommendation_cleanup_capture_is_memory_only_and_inspection_rechecks_lease() {
    let (dir, store, lease) = fixture();
    let permit = store.try_operation().unwrap();
    let intent = store.capture_cleanup();
    drop(permit);
    let bound = intent.bind(lease).unwrap();
    let lock = dir.path().join(".sidecar.lock");
    fs::rename(&lock, dir.path().join("old-lock")).unwrap();
    fs::write(&lock, b"replacement").unwrap();
    assert!(bound.inspect().is_err());
    assert!(store.try_operation().is_ok());
    let intent = store.capture_cleanup();
    fs::remove_dir_all(dir.path()).unwrap();
    // Capture remains possible with no directory; binding performs validation.
    let _unused = store.capture_cleanup();
    let other = tempfile::tempdir().unwrap();
    let other_lease = Arc::new(WorkspaceLease::acquire(other.path()).unwrap());
    assert!(intent.bind(other_lease).is_err());
}
