#![cfg(any(target_os = "macos", target_os = "linux"))]
use super::*;
use crate::workspace_runtime::WorkspaceLease;
use std::sync::Arc;

fn fixture() -> (tempfile::TempDir, RecommendationStore, Arc<WorkspaceLease>) {
    let root = tempfile::tempdir().unwrap();
    let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
    let store = RecommendationStore::open(root.path().to_path_buf()).unwrap();
    for id in ["a", "b"] {
        store.put(&tests::recommendation(id, "s")).unwrap();
    }
    store.put(&tests::recommendation("keep", "other")).unwrap();
    (root, store, lease)
}
#[test]
fn anchored_journal_publishes_and_retains_ownership_until_receipt_drop() {
    let (root, store, lease) = fixture();
    let keep = fs::read(store.path_for("keep")).unwrap();
    let receipt = store
        .capture_cleanup()
        .bind(lease.clone())
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("s")
        .unwrap()
        .publish();
    assert!(receipt.result().is_ok(), "{:?}", receipt.result());
    assert!(!store.path_for("a").exists());
    assert!(!store.path_for("b").exists());
    assert_eq!(fs::read(store.path_for("keep")).unwrap(), keep);
    assert!(!store.dir.join(ROLLUP_TRANSACTION_DIR).exists());
    assert!(matches!(store.list(), Err(StoreError::Busy)));
    drop(lease);
    assert!(WorkspaceLease::acquire(root.path()).is_err());
    drop(receipt);
    assert_eq!(store.list().unwrap().len(), 1);
}
#[test]
fn anchored_journal_refuses_changed_target_before_writes_and_retains_failure_owner() {
    let (_root, store, lease) = fixture();
    let removal = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("s")
        .unwrap();
    let bytes = serde_json::to_vec(&tests::recommendation("a", "changed")).unwrap();
    fs::write(store.path_for("a"), &bytes).unwrap();
    let receipt = removal.publish();
    assert!(receipt.result().is_err());
    assert!(!store.dir.join(ROLLUP_TRANSACTION_DIR).exists());
    assert_eq!(fs::read(store.path_for("a")).unwrap(), bytes);
    assert!(matches!(store.list(), Err(StoreError::Busy)));
}
#[test]
fn anchored_journal_restart_finishes_committed_partial_publication() {
    let (root, store, lease) = fixture();
    let removal = store
        .capture_cleanup()
        .bind(lease.clone())
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("s")
        .unwrap();
    let tx = store
        .dir
        .join(ROLLUP_TRANSACTION_DIR)
        .join(format!("tx-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(tx.join("backups")).unwrap();
    fs::create_dir(tx.join("staged")).unwrap();
    for target in removal.targets() {
        fs::write(tx.join(target.backup), target.bytes).unwrap();
    }
    fs::write(tx.join("manifest.json"), removal.manifest_bytes(true)).unwrap();
    fs::remove_file(store.path_for("a")).unwrap();
    drop(removal);
    drop(store);
    drop(lease);
    let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
    let bound =
        RecommendationStore::bind_cleanup_recovery(root.path().to_path_buf(), lease).unwrap();
    let receipt = bound.recover_removals().unwrap();
    assert!(receipt.result().is_ok(), "{:?}", receipt.result());
    drop(receipt);
    drop(bound);
    let store = RecommendationStore::open(root.path().to_path_buf()).unwrap();
    assert_eq!(
        store
            .list()
            .unwrap()
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        vec!["keep"]
    );
}

#[test]
fn anchored_journal_recovers_every_publication_and_sync_boundary_after_restart() {
    let mut reached_success = false;
    for boundary in 0..150 {
        let (root, store, lease) = fixture();
        let keep = fs::read(store.path_for("keep")).unwrap();
        let removal = store
            .capture_cleanup()
            .bind(lease.clone())
            .unwrap()
            .inspect()
            .unwrap()
            .prepare_session_removal("s")
            .unwrap();
        cleanup::JOURNAL_FAILURE.with(|f| f.set(Some(boundary)));
        let receipt = removal.publish();
        cleanup::JOURNAL_FAILURE.with(|f| f.set(None));
        let success = receipt.result().is_ok();
        let committed = !store.path_for("a").exists()
            || !store.path_for("b").exists()
            || fs::read_dir(store.dir.join(ROLLUP_TRANSACTION_DIR))
                .ok()
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .any(|e| {
                    fs::read(e.path().join("manifest.json"))
                        .ok()
                        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                        .is_some_and(|v| v["committed"] == true)
                });
        drop(receipt);
        drop(store);
        drop(lease);
        let bound = RecommendationStore::bind_cleanup_recovery(
            root.path().to_path_buf(),
            Arc::new(WorkspaceLease::acquire(root.path()).unwrap()),
        )
        .unwrap();
        for _ in 0..2 {
            let receipt = bound.recover_removals().unwrap();
            assert!(
                receipt.result().is_ok(),
                "boundary {boundary}: {:?}",
                receipt.result()
            );
        }
        drop(bound);
        let store = RecommendationStore::open(root.path().to_path_buf()).unwrap();
        assert_eq!(
            store.list().unwrap().len(),
            if committed { 1 } else { 3 },
            "boundary {boundary}"
        );
        assert_eq!(fs::read(store.path_for("keep")).unwrap(), keep);
        assert!(!store.dir.join(ROLLUP_TRANSACTION_DIR).exists());
        if success {
            assert!(boundary > 50);
            reached_success = true;
            break;
        }
    }
    assert!(reached_success);
}

#[test]
fn anchored_journal_refuses_root_replacement_without_writing_either_root() {
    let (root, store, lease) = fixture();
    let removal = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("s")
        .unwrap();
    let moved = root.path().with_extension("moved");
    fs::rename(root.path(), &moved).unwrap();
    fs::create_dir(root.path()).unwrap();
    let receipt = removal.publish();
    assert!(receipt.result().is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    assert!(moved.join("recommendations/a.json").exists());
    assert!(!moved.join("recommendations/.rollup-transactions").exists());
    drop(receipt);
    fs::remove_dir(root.path()).unwrap();
    fs::rename(moved, root.path()).unwrap();
}

fn journal(store: &RecommendationStore, lease: Arc<WorkspaceLease>, committed: bool) -> PathBuf {
    let removal = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("s")
        .unwrap();
    let tx = store
        .dir
        .join(ROLLUP_TRANSACTION_DIR)
        .join("tx-00000000-0000-4000-8000-000000000001");
    fs::create_dir_all(tx.join("backups")).unwrap();
    fs::create_dir(tx.join("staged")).unwrap();
    for target in removal.targets() {
        fs::write(tx.join(target.backup), target.bytes).unwrap();
    }
    fs::write(tx.join("manifest.json"), removal.manifest_bytes(committed)).unwrap();
    tx
}
#[test]
fn anchored_journal_discards_partial_drafts_without_rewriting_originals() {
    use std::os::unix::fs::MetadataExt;
    for manifest in [false, true] {
        let (_root, store, lease) = fixture();
        let tx = journal(&store, lease.clone(), false);
        let before = fs::read(store.path_for("a")).unwrap();
        let inode = fs::metadata(store.path_for("a")).unwrap().ino();
        fs::write(tx.join("backups/a.json"), &before[..17]).unwrap();
        fs::remove_file(tx.join("backups/b.json")).unwrap();
        if !manifest {
            fs::remove_file(tx.join("manifest.json")).unwrap();
        }
        let bound = store.capture_cleanup().bind(lease).unwrap();
        let receipt = bound.recover_removals().unwrap();
        assert!(receipt.result().is_ok(), "{:?}", receipt.result());
        assert_eq!(fs::read(store.path_for("a")).unwrap(), before);
        assert_eq!(fs::metadata(store.path_for("a")).unwrap().ino(), inode);
        assert!(!store.dir.join(ROLLUP_TRANSACTION_DIR).exists());
    }
}
#[test]
fn anchored_journal_refuses_corruption_and_foreign_entries_before_any_deletion() {
    use std::os::unix::fs::symlink;
    for case in [
        "hash",
        "missing",
        "version",
        "replacement",
        "duplicate",
        "trailing",
        "unknown",
        "extra-backup",
        "staged",
        "symlink",
        "hardlink",
        "fifo",
        "oversize",
        "temp-alias",
        "two",
    ] {
        let (_root, store, lease) = fixture();
        let tx = journal(&store, lease.clone(), true);
        match case {
            "hash" => fs::write(tx.join("backups/a.json"), b"changed").unwrap(),
            "missing" => fs::remove_file(tx.join("backups/a.json")).unwrap(),
            "version" | "replacement" => {
                let mut m: serde_json::Value =
                    serde_json::from_slice(&fs::read(tx.join("manifest.json")).unwrap()).unwrap();
                if case == "version" {
                    m["version"] = 1.into();
                } else {
                    m["entries"][0]["newExists"] = true.into();
                }
                fs::write(tx.join("manifest.json"), serde_json::to_vec(&m).unwrap()).unwrap();
            }
            "duplicate" => fs::write(
                tx.join("manifest.json"),
                br#"{"version":2,"committed":true,"committed":false,"entries":[]}"#,
            )
            .unwrap(),
            "trailing" => {
                let mut b = fs::read(tx.join("manifest.json")).unwrap();
                b.extend(b"junk");
                fs::write(tx.join("manifest.json"), b).unwrap();
            }
            "unknown" => fs::write(tx.join("foreign"), b"preserve").unwrap(),
            "extra-backup" => fs::write(tx.join("backups/foreign.json"), b"preserve").unwrap(),
            "staged" => fs::write(tx.join("staged/a.json"), b"preserve").unwrap(),
            "symlink" => {
                fs::remove_file(tx.join("backups/a.json")).unwrap();
                symlink(store.path_for("a"), tx.join("backups/a.json")).unwrap();
            }
            "hardlink" => {
                fs::remove_file(tx.join("backups/a.json")).unwrap();
                fs::hard_link(store.path_for("a"), tx.join("backups/a.json")).unwrap();
            }
            "fifo" => {
                use std::os::unix::ffi::OsStrExt;
                fs::remove_file(tx.join("backups/a.json")).unwrap();
                let path = std::ffi::CString::new(tx.join("backups/a.json").as_os_str().as_bytes())
                    .unwrap();
                // SAFETY: valid NUL-terminated temporary-fixture pathname.
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            "oversize" => fs::File::options()
                .write(true)
                .open(tx.join("backups/a.json"))
                .unwrap()
                .set_len(1024 * 1024 + 1)
                .unwrap(),
            "temp-alias" => symlink(store.path_for("keep"), tx.join("manifest.json.tmp")).unwrap(),
            "two" => fs::create_dir(
                store
                    .dir
                    .join(ROLLUP_TRANSACTION_DIR)
                    .join("tx-00000000-0000-4000-8000-000000000002"),
            )
            .unwrap(),
            _ => unreachable!(),
        }
        let before = ["a", "b", "keep"].map(|id| fs::read(store.path_for(id)).unwrap());
        let manifest = fs::read(tx.join("manifest.json")).unwrap();
        let bound = store.capture_cleanup().bind(lease).unwrap();
        let receipt = bound.recover_removals().unwrap();
        assert!(receipt.result().is_err(), "{case}");
        assert!(matches!(store.list(), Err(StoreError::Busy)));
        assert_eq!(
            ["a", "b", "keep"].map(|id| fs::read(store.path_for(id)).unwrap()),
            before,
            "{case}"
        );
        assert_eq!(
            fs::read(tx.join("manifest.json")).unwrap(),
            manifest,
            "{case}"
        );
    }
}
#[test]
fn anchored_journal_noop_does_not_create_a_directory_or_journal() {
    for absent in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
        let store = RecommendationStore::open(root.path().to_path_buf()).unwrap();
        if absent {
            fs::remove_dir(&store.dir).unwrap();
        }
        let receipt = store
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .inspect()
            .unwrap()
            .prepare_session_removal("missing")
            .unwrap()
            .publish();
        assert!(receipt.result().is_ok());
        assert_eq!(store.dir.exists(), !absent);
        assert!(!store.dir.join(ROLLUP_TRANSACTION_DIR).exists());
    }
}
#[test]
fn anchored_journal_recovery_refuses_changed_uncommitted_and_committed_targets() {
    for committed in [false, true] {
        let (_root, store, lease) = fixture();
        let tx = journal(&store, lease.clone(), committed);
        let changed = serde_json::to_vec(&tests::recommendation("a", "foreign-session")).unwrap();
        fs::write(store.path_for("a"), &changed).unwrap();
        let receipt = store
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .recover_removals()
            .unwrap();
        assert!(receipt.result().is_err());
        assert_eq!(fs::read(store.path_for("a")).unwrap(), changed);
        assert!(store.path_for("b").exists());
        assert!(tx.join("manifest.json").exists());
    }
}
#[test]
fn anchored_journal_recovery_removes_complete_connected_rollup_graph() {
    let (_root, store, lease) = fixture();
    let parent = stable_rollup_id(&["a".into(), "b".into()]);
    let mut a = tests::recommendation("a", "s");
    a.status = RecommendationStatus::RolledUp;
    a.rolled_up_by = Some(parent.clone());
    let mut b = tests::recommendation("b", "other");
    b.status = RecommendationStatus::RolledUp;
    b.rolled_up_by = Some(parent.clone());
    for record in [&a, &b, &tests::rollup_parent(&parent, &["a", "b"])] {
        fs::write(
            store.path_for(&record.id),
            serde_json::to_vec(record).unwrap(),
        )
        .unwrap();
    }
    let tx = journal(&store, lease.clone(), true);
    fs::remove_file(store.path_for("a")).unwrap();
    let receipt = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .recover_removals()
        .unwrap();
    assert!(receipt.result().is_ok(), "{:?}", receipt.result());
    assert!(!tx.exists());
    drop(receipt);
    assert_eq!(
        store
            .list()
            .unwrap()
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        vec!["keep"]
    );
}

#[test]
fn anchored_journal_refuses_foreign_manifest_temporary_data_before_effects() {
    for committed in [false, true] {
        let (_root, store, lease) = fixture();
        let tx = journal(&store, lease.clone(), committed);
        let foreign = b"foreign data, not a manifest write";
        fs::write(tx.join("manifest.json.tmp"), foreign).unwrap();
        let originals = ["a", "b", "keep"].map(|id| fs::read(store.path_for(id)).unwrap());
        let receipt = store
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .recover_removals()
            .unwrap();
        assert!(receipt.result().is_err(), "committed={committed}");
        assert_eq!(fs::read(tx.join("manifest.json.tmp")).unwrap(), foreign);
        assert_eq!(
            ["a", "b", "keep"].map(|id| fs::read(store.path_for(id)).unwrap()),
            originals
        );
    }
}

#[test]
fn anchored_journal_discards_only_valid_temporary_manifest_prefixes() {
    for first in [false, true] {
        let (_root, store, lease) = fixture();
        let tx = journal(&store, lease.clone(), false);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(tx.join("manifest.json")).unwrap()).unwrap();
        // Use the actual v2 field order while choosing the expected phase independently.
        value["committed"] = (!first).into();
        let manifest: RollupTransactionManifest = serde_json::from_value(value).unwrap();
        let bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        fs::write(tx.join("manifest.json.tmp"), &bytes[..bytes.len() - 11]).unwrap();
        if first {
            fs::remove_file(tx.join("manifest.json")).unwrap();
        }
        let receipt = store
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .recover_removals()
            .unwrap();
        assert!(receipt.result().is_ok(), "{:?}", receipt.result());
        assert!(store.path_for("a").exists());
        assert!(store.path_for("b").exists());
        assert!(!tx.exists());
    }
}

#[test]
fn anchored_journal_refuses_temporary_manifest_without_any_completed_backup() {
    let (_root, store, lease) = fixture();
    let tx = journal(&store, lease.clone(), false);
    fs::remove_file(tx.join("manifest.json")).unwrap();
    fs::remove_dir_all(tx.join("backups")).unwrap();
    fs::write(tx.join("manifest.json.tmp"), b"").unwrap();
    let receipt = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .recover_removals()
        .unwrap();
    assert!(receipt.result().is_err());
    assert!(tx.join("manifest.json.tmp").exists());
    assert!(store.path_for("a").exists());
    assert!(store.path_for("b").exists());
}

#[test]
fn anchored_journal_refuses_parent_rename_hidden_by_symlink() {
    use std::os::unix::fs::symlink;
    let container = tempfile::tempdir().unwrap();
    let parent = container.path().join("parent");
    let root = parent.join("metadata");
    fs::create_dir_all(&root).unwrap();
    let lease = Arc::new(WorkspaceLease::acquire(&root).unwrap());
    let store = RecommendationStore::open(root.clone()).unwrap();
    store.put(&tests::recommendation("a", "s")).unwrap();
    let removal = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("s")
        .unwrap();
    let moved = container.path().join("moved");
    fs::rename(&parent, &moved).unwrap();
    symlink(&moved, &parent).unwrap();
    let receipt = removal.publish();
    assert!(receipt.result().is_err());
    assert!(moved.join("metadata/recommendations/a.json").exists());
    assert!(!moved
        .join("metadata/recommendations/.rollup-transactions")
        .exists());
}
