use super::*;

#[test]
fn sql_bound_permit_has_no_gap_between_recovery_and_fresh_pass() {
    let (_root, store, lease, identity) = fixture();
    seed(&store, lease.clone(), &identity, true);
    let competing = store.clone();
    let ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = ran.clone();
    AFTER_RECOVERY.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            assert!(matches!(
                competing.put(&recommendation("foreign", "s")),
                Err(StoreError::Busy)
            ));
            observed.store(true, std::sync::atomic::Ordering::Release);
        }))
    });
    let receipt = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .remove_for_sql(&identity)
        .unwrap();
    assert!(receipt.result().is_ok(), "{:?}", receipt.result());
    assert!(ran.load(std::sync::atomic::Ordering::Acquire));
    assert!(!store.path_for("foreign").exists());
}
#[test]
fn sql_bound_fresh_pass_removes_late_references_after_completed_targets() {
    let (_root, store, lease, identity) = fixture();
    seed(&store, lease.clone(), &identity, true);
    fs::remove_file(store.path_for("a")).unwrap();
    fs::remove_file(store.path_for("b")).unwrap();
    fs::write(
        store.path_for("late"),
        serde_json::to_vec(&recommendation("late", "s")).unwrap(),
    )
    .unwrap();
    let keep = fs::read(store.path_for("keep")).unwrap();
    let receipt = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .remove_for_sql(&identity)
        .unwrap();
    assert!(receipt.result().is_ok(), "{:?}", receipt.result());
    assert!(!store.path_for("late").exists());
    assert_eq!(fs::read(store.path_for("keep")).unwrap(), keep);
}
#[test]
fn sql_bound_recovery_preserves_complete_connected_rollup_policy() {
    let (_root, store, lease, identity) = fixture();
    let parent = stable_rollup_id(&["a".into(), "b".into()]);
    let mut a = recommendation("a", "s");
    a.status = RecommendationStatus::RolledUp;
    a.rolled_up_by = Some(parent.clone());
    let mut b = recommendation("b", "other");
    b.status = RecommendationStatus::RolledUp;
    b.rolled_up_by = Some(parent.clone());
    for record in [&a, &b, &rollup_parent(&parent, &["a", "b"])] {
        fs::write(
            store.path_for(&record.id),
            serde_json::to_vec(record).unwrap(),
        )
        .unwrap();
    }
    seed(&store, lease.clone(), &identity, true);
    fs::remove_file(store.path_for("a")).unwrap();
    let receipt = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .remove_for_sql(&identity)
        .unwrap();
    assert!(receipt.result().is_ok(), "{:?}", receipt.result());
    assert!(!store.path_for(&parent).exists());
    assert!(!store.path_for("b").exists());
    drop(receipt);
    assert!(matches!(
        store.list(),
        Err(StoreError::SqlAuthorityRequired)
    ));
}
#[test]
fn sql_bound_refuses_valid_graph_targets_from_another_session() {
    for committed in [false, true] {
        let (_root, store, lease, identity) = fixture();
        let tx = seed_scope(&store, lease.clone(), &identity, committed, "other");
        let before = originals(&store);
        let manifest = fs::read(tx.join("manifest.json")).unwrap();
        let receipt = store
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .remove_for_sql(&identity)
            .unwrap();
        assert!(receipt.result().is_err(), "committed={committed}");
        assert_eq!(originals(&store), before);
        assert_eq!(fs::read(tx.join("manifest.json")).unwrap(), manifest);
    }
}
#[test]
fn sql_bound_refuses_corruption_aliases_and_mixed_journals_without_effects() {
    use std::os::unix::fs::symlink;
    for case in [
        "hash", "missing", "temp", "manifest", "extra", "alias", "hardlink", "two", "legacy",
        "oversize",
    ] {
        let (_root, store, lease, identity) = fixture();
        let tx = seed(&store, lease.clone(), &identity, true);
        match case {
            "hash" => fs::write(tx.join("backups/a.json"), b"changed").unwrap(),
            "missing" => fs::remove_file(tx.join("backups/a.json")).unwrap(),
            "temp" => fs::write(tx.join("manifest.json.tmp"), b"foreign").unwrap(),
            "manifest" => {
                let mut v: serde_json::Value =
                    serde_json::from_slice(&fs::read(tx.join("manifest.json")).unwrap()).unwrap();
                v["unknown"] = true.into();
                fs::write(tx.join("manifest.json"), serde_json::to_vec(&v).unwrap()).unwrap();
            }
            "extra" => fs::write(tx.join("foreign"), b"preserve").unwrap(),
            "alias" | "hardlink" => {
                fs::remove_file(tx.join("backups/a.json")).unwrap();
                if case == "alias" {
                    symlink(store.path_for("a"), tx.join("backups/a.json")).unwrap();
                } else {
                    fs::hard_link(store.path_for("a"), tx.join("backups/a.json")).unwrap();
                }
            }
            "two" => fs::create_dir(
                store
                    .dir
                    .join(ROLLUP_TRANSACTION_DIR)
                    .join("sql-delete-v1-foreign"),
            )
            .unwrap(),
            "legacy" => fs::create_dir(
                store
                    .dir
                    .join(ROLLUP_TRANSACTION_DIR)
                    .join("tx-00000000-0000-4000-8000-000000000001"),
            )
            .unwrap(),
            "oversize" => fs::OpenOptions::new()
                .write(true)
                .open(tx.join("manifest.json"))
                .unwrap()
                .set_len(1024 * 1024 + 1)
                .unwrap(),
            _ => unreachable!(),
        }
        let before = originals(&store);
        let manifest = fs::read(tx.join("manifest.json")).unwrap();
        let receipt = store
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .remove_for_sql(&identity)
            .unwrap();
        assert!(receipt.result().is_err(), "{case}");
        assert_eq!(originals(&store), before, "{case}");
        assert_eq!(
            fs::read(tx.join("manifest.json")).unwrap(),
            manifest,
            "{case}"
        );
    }
}
#[test]
fn sql_bound_manifestless_draft_discards_only_known_prefixes() {
    for malformed in [false, true] {
        let (_root, store, lease, identity) = fixture();
        let tx = seed(&store, lease.clone(), &identity, false);
        fs::remove_file(tx.join("manifest.json")).unwrap();
        fs::remove_file(tx.join("backups/b.json")).unwrap();
        let bytes = fs::read(store.path_for("a")).unwrap();
        fs::write(
            tx.join("backups/a.json"),
            if malformed { b"foreign" } else { &bytes[..17] },
        )
        .unwrap();
        let before = originals(&store);
        let bound = store.capture_cleanup().bind(lease).unwrap();
        let _operation = store.try_operation().unwrap();
        let result = protocol::recover_sql(&bound.0, &identity);
        assert_eq!(result.is_err(), malformed);
        assert_eq!(originals(&store), before);
        assert_eq!(tx.exists(), malformed);
    }
}
#[test]
fn sql_bound_refuses_root_replacement_and_noop_creates_no_journal() {
    let (root, store, lease, identity) = fixture();
    let bound = store.capture_cleanup().bind(lease).unwrap();
    let moved = root.path().with_extension("moved");
    fs::rename(root.path(), &moved).unwrap();
    fs::create_dir(root.path()).unwrap();
    let receipt = bound.remove_for_sql(&identity).unwrap();
    assert!(receipt.result().is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    assert!(moved.join("recommendations/a.json").exists());
    assert!(!moved.join("recommendations/.rollup-transactions").exists());
    fs::remove_dir(root.path()).unwrap();
    fs::rename(moved, root.path()).unwrap();
    drop(receipt);
    drop(bound);
    for absent in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
        let store = RecommendationStore::open(root.path().into()).unwrap();
        let identity = selected(root.path(), "s");
        if absent {
            fs::remove_dir(&store.dir).unwrap();
        }
        let receipt = store
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .remove_for_sql(&identity)
            .unwrap();
        assert!(receipt.result().is_ok(), "{:?}", receipt.result());
        assert_eq!(store.dir.exists(), !absent);
        assert!(!store.dir.join(ROLLUP_TRANSACTION_DIR).exists());
    }
}
#[test]
fn sql_bound_recovers_every_publication_boundary_after_fresh_lease() {
    let mut success = false;
    for boundary in 0..180 {
        let (root, store, lease, identity) = fixture();
        let keep = fs::read(store.path_for("keep")).unwrap();
        let bound = store.capture_cleanup().bind(lease.clone()).unwrap();
        cleanup::JOURNAL_FAILURE.with(|f| f.set(Some(boundary)));
        let receipt = bound.remove_for_sql(&identity).unwrap();
        cleanup::JOURNAL_FAILURE.with(|f| f.set(None));
        let completed = receipt.result().is_ok();
        assert!(matches!(store.list(), Err(StoreError::Busy)));
        drop(receipt);
        drop(bound);
        drop(store);
        drop(lease);
        let bound = RecommendationStore::bind_cleanup_recovery(
            root.path().into(),
            Arc::new(WorkspaceLease::acquire(root.path()).unwrap()),
        )
        .unwrap();
        for _ in 0..2 {
            let receipt = bound.remove_for_sql(&identity).unwrap();
            assert!(
                receipt.result().is_ok(),
                "boundary {boundary}: {:?}",
                receipt.result()
            );
        }
        assert_eq!(
            fs::read(root.path().join("recommendations/keep.json")).unwrap(),
            keep
        );
        assert!(!root.path().join("recommendations/a.json").exists());
        assert!(!root.path().join("recommendations/b.json").exists());
        assert!(!root
            .path()
            .join("recommendations/.rollup-transactions")
            .exists());
        if completed {
            assert!(boundary > 50);
            success = true;
            break;
        }
    }
    assert!(success);
}
#[test]
fn sql_bound_recovers_interruptions_during_draft_and_committed_recovery() {
    for phase in ["draft", "first-temp", "committed"] {
        let mut success = false;
        for boundary in 0..130 {
            let (root, store, lease, identity) = fixture();
            let tx = seed(&store, lease.clone(), &identity, phase == "committed");
            if phase == "first-temp" {
                let bytes = fs::read(tx.join("manifest.json")).unwrap();
                fs::write(tx.join("manifest.json.tmp"), &bytes[..bytes.len() - 11]).unwrap();
                fs::remove_file(tx.join("manifest.json")).unwrap();
            }
            let before = originals(&store);
            let bound = store.capture_cleanup().bind(lease.clone()).unwrap();
            let operation = store.try_operation().unwrap();
            cleanup::JOURNAL_FAILURE.with(|f| f.set(Some(boundary)));
            let result = protocol::recover_sql(&bound.0, &identity);
            cleanup::JOURNAL_FAILURE.with(|f| f.set(None));
            let completed = result.is_ok();
            drop(operation);
            drop(bound);
            drop(store);
            drop(lease);
            let bound = RecommendationStore::bind_cleanup_recovery(
                root.path().into(),
                Arc::new(WorkspaceLease::acquire(root.path()).unwrap()),
            )
            .unwrap();
            for _ in 0..2 {
                let _operation = bound.0.store.try_operation().unwrap();
                protocol::recover_sql(&bound.0, &identity)
                    .unwrap_or_else(|e| panic!("{phase} boundary {boundary}: {e}"));
            }
            if phase == "committed" {
                assert!(!root.path().join("recommendations/a.json").exists());
                assert!(!root.path().join("recommendations/b.json").exists());
            } else {
                assert_eq!(
                    ["a", "b", "keep"].map(|id| fs::read(
                        root.path()
                            .join("recommendations")
                            .join(format!("{id}.json"))
                    )
                    .unwrap()),
                    before
                );
            }
            assert_eq!(
                fs::read(root.path().join("recommendations/keep.json")).unwrap(),
                before[2]
            );
            assert!(!tx.exists());
            if completed {
                success = true;
                break;
            }
        }
        assert!(success, "{phase}");
    }
}

#[test]
fn sql_bound_refuses_legacy_recovery_even_for_a_valid_deletion_manifest() {
    let (_root, store, lease, identity) = fixture();
    let tx = seed(&store, lease.clone(), &identity, true);
    let mut manifest: RollupTransactionManifest =
        serde_json::from_slice(&fs::read(tx.join("manifest.json")).unwrap()).unwrap();
    manifest.version = 2;
    fs::write(
        tx.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let legacy = tx.with_file_name("tx-00000000-0000-4000-8000-000000000001");
    fs::rename(tx, &legacy).unwrap();
    let before = originals(&store);
    let receipt = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .remove_for_sql(&identity)
        .unwrap();
    assert!(receipt.result().is_err());
    assert_eq!(originals(&store), before);
    assert!(legacy.exists());
    drop(receipt);
    assert!(matches!(
        store.list(),
        Err(StoreError::SqlAuthorityRequired)
    ));
}
#[test]
fn sql_bound_maximum_inventory_uses_bounded_descriptors() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "taskmaster::store::cleanup::removal::journal::sql_tests::faults::sql_bound_fd_limit_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("ORKWORKS_SQL_BOUND_FD_TEST", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("verified SQL-bound FD case"),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn sql_bound_fd_limit_child() {
    if std::env::var_os("ORKWORKS_SQL_BOUND_FD_TEST").is_none() {
        return;
    }
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: initialized output for this isolated child process's descriptor limit.
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
        0
    );
    limit.rlim_cur = 64.min(limit.rlim_max);
    // SAFETY: lower this child process's soft limit, preserving its hard limit.
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) }, 0);
    for restart in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
        let store = RecommendationStore::open(root.path().into()).unwrap();
        let identity = selected(root.path(), "s");
        for n in 0..1024 {
            let id = format!("record-{n:04}");
            fs::write(
                store.path_for(&id),
                serde_json::to_vec(&recommendation(&id, "s")).unwrap(),
            )
            .unwrap();
        }
        let receipt = if restart {
            seed(&store, lease.clone(), &identity, true);
            for n in 0..512 {
                fs::remove_file(store.path_for(&format!("record-{n:04}"))).unwrap();
            }
            drop(store);
            drop(lease);
            let reselected = selected(root.path(), "s");
            assert_eq!(reselected, identity);
            RecommendationStore::bind_cleanup_recovery(
                root.path().into(),
                Arc::new(WorkspaceLease::acquire(root.path()).unwrap()),
            )
            .unwrap()
            .remove_for_sql(&reselected)
            .unwrap()
        } else {
            store
                .capture_cleanup()
                .bind(lease)
                .unwrap()
                .remove_for_sql(&identity)
                .unwrap()
        };
        assert!(
            receipt.result().is_ok(),
            "restart={restart}: {:?}",
            receipt.result()
        );
        assert_eq!(
            fs::read_dir(root.path().join("recommendations"))
                .unwrap()
                .count(),
            0
        );
    }
    println!("verified SQL-bound FD case");
}

#[test]
fn sql_bound_first_temp_requires_the_complete_expected_backup_set() {
    let (_root, store, lease, identity) = fixture();
    let tx = seed(&store, lease.clone(), &identity, false);
    let draft = fs::read(tx.join("manifest.json")).unwrap();
    // A short prefix alone cannot establish that all intended backups exist.
    fs::write(tx.join("manifest.json.tmp"), &draft[..20]).unwrap();
    fs::remove_file(tx.join("manifest.json")).unwrap();
    fs::remove_file(tx.join("backups/b.json")).unwrap();
    let before = originals(&store);
    let bound = store.capture_cleanup().bind(lease).unwrap();
    let _operation = store.try_operation().unwrap();
    assert!(protocol::recover_sql(&bound.0, &identity).is_err());
    assert_eq!(originals(&store), before);
    assert_eq!(
        fs::read(tx.join("manifest.json.tmp")).unwrap(),
        &draft[..20]
    );
    assert!(tx.join("backups/a.json").exists());
}

#[test]
fn legacy_preflight_reuses_the_existing_directory_entry_bound() {
    for count in [1025, 1026] {
        let root = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(root.path().into()).unwrap();
        let parent = store.dir.join(ROLLUP_TRANSACTION_DIR);
        fs::create_dir(&parent).unwrap();
        for n in 0..count {
            fs::create_dir(parent.join(format!("tx-00000000-0000-4000-8000-{n:012x}"))).unwrap();
        }
        let result = store.list();
        if count == 1025 {
            assert!(result.is_ok(), "{:?}", result);
            assert!(!parent.exists());
        } else {
            assert!(result.is_err());
            assert_eq!(fs::read_dir(parent).unwrap().count(), 1026);
        }
    }
}

#[test]
fn sql_bound_all_absent_targets_still_validate_complete_backup_graph() {
    for empty in [false, true] {
        let (_root, store, lease, identity) = fixture();
        let tx = if empty {
            let tx = store
                .dir
                .join(ROLLUP_TRANSACTION_DIR)
                .join(identity.journal_name());
            fs::create_dir_all(tx.join("backups")).unwrap();
            fs::create_dir(tx.join("staged")).unwrap();
            fs::write(
                tx.join("manifest.json"),
                sql_codec::encode(&identity, &[], true).unwrap(),
            )
            .unwrap();
            tx
        } else {
            let tx = seed_scope(&store, lease.clone(), &identity, true, "other");
            fs::remove_file(store.path_for("keep")).unwrap();
            tx
        };
        let before = fs::read(store.path_for("a")).unwrap();
        let manifest = fs::read(tx.join("manifest.json")).unwrap();
        let bound = store.capture_cleanup().bind(lease).unwrap();
        let _operation = store.try_operation().unwrap();
        assert!(
            protocol::recover_sql(&bound.0, &identity).is_err(),
            "empty={empty}"
        );
        assert_eq!(fs::read(store.path_for("a")).unwrap(), before);
        assert_eq!(fs::read(tx.join("manifest.json")).unwrap(), manifest);
    }
}
