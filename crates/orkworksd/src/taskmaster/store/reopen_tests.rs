use super::*;

#[test]
fn legacy_open_refuses_each_cutover_entry_before_recovery_or_creation() {
    for name in ["format.json", "session-store"] {
        for directory in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let evidence = dir.path().join(name);
            if directory {
                fs::create_dir(&evidence).unwrap();
            } else {
                fs::write(&evidence, b"unreadable format is still authority evidence").unwrap();
            }
            assert!(
                matches!(
                    RecommendationStore::open(dir.path().into()),
                    Err(StoreError::SqlAuthorityRequired)
                ),
                "{name} directory={directory}"
            );
            assert!(!dir.path().join("recommendations").exists());
            assert!(evidence.symlink_metadata().is_ok());
        }
    }
}

#[test]
fn cutover_refusal_preserves_graph_recovery_evidence_and_busy_precedence() {
    let dir = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(dir.path().into()).unwrap();
    let file = store.dir.join("must-not-read.json");
    fs::write(&file, b"invalid graph remains unchanged").unwrap();
    let transaction = store.dir.join(ROLLUP_TRANSACTION_DIR).join("unfinished");
    fs::create_dir_all(&transaction).unwrap();
    fs::write(transaction.join("preserve"), b"no recovery").unwrap();
    fs::create_dir(dir.path().join("session-store")).unwrap();
    let busy = store.owner.claim().unwrap();
    assert!(matches!(
        RecommendationStore::open(dir.path().into()),
        Err(StoreError::Busy)
    ));
    drop(busy);
    assert!(matches!(
        RecommendationStore::open(dir.path().join(".")),
        Err(StoreError::SqlAuthorityRequired)
    ));
    assert_eq!(fs::read(file).unwrap(), b"invalid graph remains unchanged");
    assert_eq!(
        fs::read(transaction.join("preserve")).unwrap(),
        b"no recovery"
    );
}

#[cfg(unix)]
#[test]
fn cutover_refusal_does_not_follow_dangling_entries_and_propagates_inspection_failure() {
    for name in ["format.json", "session-store"] {
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(dir.path().join("absent"), dir.path().join(name)).unwrap();
        assert!(matches!(
            RecommendationStore::open(dir.path().into()),
            Err(StoreError::SqlAuthorityRequired)
        ));
        assert!(!dir.path().join("recommendations").exists());
    }
    let dir = tempfile::tempdir().unwrap();
    let blocked = dir.path().join("not-a-directory");
    fs::write(&blocked, b"preserve").unwrap();
    assert!(matches!(
        RecommendationStore::open(blocked.clone()),
        Err(StoreError::Io(_))
    ));
    assert_eq!(fs::read(blocked).unwrap(), b"preserve");
}

#[cfg(unix)]
#[test]
fn cutover_refusal_is_shared_by_root_aliases_and_unreadable_is_not_absent() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("session-store")).unwrap();
    let alias = dir.path().join("alias");
    std::os::unix::fs::symlink(&root, &alias).unwrap();
    assert!(matches!(
        RecommendationStore::open(alias),
        Err(StoreError::SqlAuthorityRequired)
    ));
    let permissions = fs::metadata(&root).unwrap().permissions();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o000)).unwrap();
    let result = RecommendationStore::open(root.clone());
    fs::set_permissions(&root, permissions).unwrap();
    assert!(
        matches!(result, Err(StoreError::Io(error)) if error.kind() == io::ErrorKind::PermissionDenied)
    );
    assert!(!root.join("recommendations").exists());
}
