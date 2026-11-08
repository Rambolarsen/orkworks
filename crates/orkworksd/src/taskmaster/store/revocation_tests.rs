use super::*;
use crate::workspace_runtime::WorkspaceLease;
use std::sync::Arc;
fn fixture() -> (tempfile::TempDir, RecommendationStore, BoundCleanup) {
    let root = tempfile::tempdir().unwrap();
    let store = RecommendationStore::open(root.path().join("workspace")).unwrap();
    store.put(&tests::recommendation("r", "s")).unwrap();
    let lease = Arc::new(WorkspaceLease::acquire(store.dir.parent().unwrap()).unwrap());
    let bound = store.capture_cleanup().bind(lease).unwrap();
    (root, store, bound)
}
fn revoked<T>(result: Result<T, StoreError>) {
    assert!(matches!(result, Err(StoreError::SqlAuthorityRequired)));
}
fn revoke(bound: &BoundCleanup) {
    // Trusted private fixture admission, no application/runtime authority implied.
    drop(
        bound
            .claim_sql_cleanup()
            .unwrap()
            .admit(crate::session_application::sql_writes::trusted_cleanup_admission_fixture()),
    );
}
#[test]
fn sql_revocation_covers_every_legacy_operation_before_recovery_and_validation() {
    let (_root, store, bound) = fixture();
    let before = fs::read(store.path_for("r")).unwrap();
    let tx = store
        .dir
        .join(ROLLUP_TRANSACTION_DIR)
        .join("uncommitted-legacy");
    fs::create_dir_all(&tx).unwrap();
    fs::write(tx.join("preserve"), b"must not recover").unwrap();
    let old_name = store.dir.join(format!("rollup:{}.json", "a".repeat(64)));
    fs::write(&old_name, b"must not migrate").unwrap();
    revoke(&bound);
    let request = super::super::completion::CompletionMutationRequest {
        packet_revision: 1,
        evidence_fingerprint: "a".repeat(64),
        idempotency_key: "stale".into(),
    };
    revoked(store.list());
    revoked(store.list_with_hashes());
    revoked(store.get("r"));
    revoked(store.put(&tests::recommendation("new", "s")));
    revoked(store.apply_recommendation_graph_transaction(&BTreeMap::new(), &[]));
    revoked(store.dismiss("r", "later".into(), None));
    revoked(store.begin_execution("r", "target".into(), "later".into()));
    revoked(store.begin_execution_checked(
        "r",
        "target".into(),
        "later".into(),
        &request,
        "user".into(),
    ));
    revoked(store.complete_execution("r", "later".into()));
    revoked(store.complete_accepted("r", "later".into()));
    revoked(store.complete_accepted_checked("r", "later".into(), &request));
    revoked(store.cancel_execution("r", "later".into()));
    revoked(store.replace_completion_packet(
        "r",
        1,
        crate::taskmaster::completion_tests::test_packet(),
        "later".into(),
    ));
    revoked(store.recover_orphaned_executions(&HashSet::new(), &HashSet::new(), "later".into()));
    revoked(store.delete_referencing_session("s"));
    revoked(store.scrub_orphans(&HashSet::new()));
    revoked(bound.inspect());
    revoked(bound.prepare_removal("s"));
    revoked(bound.recover_removals());
    revoked(RecommendationStore::open(
        store.dir.parent().unwrap().into(),
    ));
    assert_eq!(fs::read(store.path_for("r")).unwrap(), before);
    assert_eq!(fs::read(tx.join("preserve")).unwrap(), b"must not recover");
    assert_eq!(fs::read(old_name).unwrap(), b"must not migrate");
    assert!(!store.path_for("new").exists());
}
#[test]
fn sql_revocation_claim_cancellation_busy_precedence_and_retry_are_exact() {
    let (_root, store, bound) = fixture();
    let candidate = bound.claim_sql_cleanup().unwrap();
    assert!(matches!(store.list(), Err(StoreError::Busy)));
    drop(candidate); // Cancellation cannot mint the private application proof.
    assert_eq!(store.list().unwrap().len(), 1);
    let admitted = bound
        .claim_sql_cleanup()
        .unwrap()
        .admit(crate::session_application::sql_writes::trusted_cleanup_admission_fixture());
    assert!(matches!(store.list(), Err(StoreError::Busy)));
    assert!(matches!(bound.claim_sql_cleanup(), Err(StoreError::Busy)));
    drop(admitted);
    revoked(store.list());
    let retry = bound
        .claim_sql_cleanup()
        .unwrap()
        .admit(crate::session_application::sql_writes::trusted_cleanup_admission_fixture());
    assert!(matches!(store.list(), Err(StoreError::Busy)));
    drop(retry);
    revoked(store.list());
}
#[test]
fn sql_revocation_refuses_missing_root_and_directory_without_creating_them() {
    for remove_root in [false, true] {
        let (_outer, store, bound) = fixture();
        let root = store.dir.parent().unwrap().to_path_buf();
        revoke(&bound);
        if remove_root {
            fs::remove_dir_all(&root).unwrap();
        } else {
            fs::remove_dir_all(&store.dir).unwrap();
        }
        revoked(RecommendationStore::open(root.clone()));
        assert!(!store.dir.exists());
        if remove_root {
            assert!(!root.exists());
        }
    }
}
#[test]
fn sql_revocation_is_shared_by_aliases_but_not_other_roots_or_future_owners() {
    let (outer, store, bound) = fixture();
    let root = store.dir.parent().unwrap().to_path_buf();
    let alias = RecommendationStore::open(root.join(".")).unwrap();
    let other = RecommendationStore::open(outer.path().join("other/nested")).unwrap();
    revoke(&bound);
    revoked(alias.list());
    revoked(RecommendationStore::open(root.join(".")));
    let link = outer.path().join("alias");
    std::os::unix::fs::symlink(&root, &link).unwrap();
    revoked(RecommendationStore::open(link));
    assert!(other.list().unwrap().is_empty());
    drop(bound);
    drop(store);
    revoked(alias.list()); // Last original owner keeps revocation alive.
    drop(alias);
    // Explicit non-durable limit: a fresh owner is not SQL qualification.
    assert_eq!(
        RecommendationStore::open(root)
            .unwrap()
            .list()
            .unwrap()
            .len(),
        1
    );
}
