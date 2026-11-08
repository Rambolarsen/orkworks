use super::*;
#[test]
fn legacy_open_refuses_sql_artifacts_before_creating_or_repairing_files() {
    for name in ["format.json", "session-store"] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(name), b"retained").unwrap();
        assert!(WorkflowObservationStore::open(root.path().into()).is_err());
        assert!(!root.path().join("workflow-observations").exists());
        assert_eq!(fs::read(root.path().join(name)).unwrap(), b"retained");
    }
}
#[test]
fn legacy_sql_tag_preflight_never_truncates_a_qualified_partial_tail() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("workflow-observations");
    fs::create_dir(&dir).unwrap();
    let bytes = br#"{"record":"sql_observation_v1","observation":{"#;
    fs::write(dir.join("s.ndjson"), bytes).unwrap();
    fs::write(dir.join("sequence"), b"41").unwrap();
    assert!(WorkflowObservationStore::open(root.path().into()).is_err());
    assert_eq!(fs::read(dir.join("s.ndjson")).unwrap(), bytes);
    assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"41");
}

#[test]
fn legacy_floor_sees_reserved_top_level_tags_after_a_damaged_prefix() {
    for bytes in [
        br#"{"observation":NaN,"record":"sql_observation_v1","origin":{}"#.as_slice(),
        br#"{"broken":?,"record":"sql_tombstone_v1","origin":{}"#.as_slice(),
    ] {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("workflow-observations");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("s.ndjson"), bytes).unwrap();
        assert!(WorkflowObservationStore::open(root.path().into()).is_err());
        assert_eq!(fs::read(dir.join("s.ndjson")).unwrap(), bytes);
    }
}

#[test]
#[cfg(unix)]
fn canonical_aliases_and_independent_opens_share_nonblocking_admission_without_strong_registry() {
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("actual");
    let lease = crate::workspace_runtime::WorkspaceLease::acquire(&root).unwrap();
    let alias = outer.path().join("alias");
    std::os::unix::fs::symlink(&root, &alias).unwrap();
    let a = WorkflowObservationStore::open(root.clone()).unwrap();
    let b = WorkflowObservationStore::open(alias.clone()).unwrap();
    let weak = Arc::downgrade(&a.owner);
    let first = a.owner.legacy().unwrap();
    let second = b.owner.legacy().unwrap();
    assert!(matches!(
        SqlCandidate::bind(&alias, &lease),
        Err(crate::session_store::StoreError::Busy)
    ));
    drop(first);
    assert!(matches!(
        SqlCandidate::bind(&root, &lease),
        Err(crate::session_store::StoreError::Busy)
    ));
    drop(second);
    let pending = SqlCandidate::bind(&alias, &lease).unwrap();
    assert!(a.workspace_observations().is_err());
    assert!(b.workspace_observations().is_err());
    drop(pending);
    assert!(a.workspace_observations().is_ok());
    assert!(b.workspace_observations().is_ok());
    drop(a);
    drop(b);
    assert!(weak.upgrade().is_none());
    assert!(WorkflowObservationStore::open(root).is_ok());
}

#[test]
fn ignored_entries_cannot_complete_legacy_preflight_after_deadline_expiry() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("workflow-observations");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("ignored.tmp"), b"retained").unwrap();
    operation::preflight_after_entry(Box::new(|| {
        std::thread::sleep(std::time::Duration::from_millis(2100))
    }));
    let result = WorkflowObservationStore::open(root.path().into());
    assert!(
        matches!(result,Err(StoreError::Io(error)) if error.kind()==std::io::ErrorKind::TimedOut)
    );
    assert_eq!(fs::read(dir.join("ignored.tmp")).unwrap(), b"retained");
    assert!(!dir.join("sequence").exists());
}
