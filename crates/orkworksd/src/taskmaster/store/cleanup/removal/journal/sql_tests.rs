use super::*;
use crate::session_store::{SessionStore, SqlDeletionIdentity};
use crate::taskmaster::store::tests::{recommendation, rollup_parent};

const CREATED: &str = "2026-01-01T00:00:00Z";
fn selected(root: &Path, id: &str) -> SqlDeletionIdentity {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let store = if root.join("session-store/v1.sqlite").exists() {
                SessionStore::open(root.into(), "/workspace".into())
                    .await
                    .unwrap()
            } else {
                SessionStore::initialize(root.into(), "/workspace".into())
                    .await
                    .unwrap()
            };
            if store
                .deletion_record(id.into(), CREATED.into())
                .await
                .unwrap()
                .is_none()
            {
                let mut meta = crate::test_support::test_session_metadata(
                    id,
                    "closed",
                    "/workspace",
                    "ended",
                    CREATED,
                    CREATED,
                );
                meta.lifecycle = "dead".into();
                meta.lifecycle_phase = "ended".into();
                store
                    .create(serde_json::to_value(meta).unwrap())
                    .await
                    .unwrap();
                store
                    .begin_delete(id.into(), CREATED.into(), "1".into(), "0".into())
                    .await
                    .unwrap();
            }
            let identity = store
                .deletion_record(id.into(), CREATED.into())
                .await
                .unwrap()
                .unwrap()
                .identity;
            store.close().await.unwrap();
            identity
        })
}
fn fixture() -> (
    tempfile::TempDir,
    RecommendationStore,
    Arc<WorkspaceLease>,
    SqlDeletionIdentity,
) {
    let root = tempfile::tempdir().unwrap();
    let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
    let store = RecommendationStore::open(root.path().into()).unwrap();
    for id in ["a", "b"] {
        store.put(&recommendation(id, "s")).unwrap();
    }
    store.put(&recommendation("keep", "other")).unwrap();
    let identity = selected(root.path(), "s");
    (root, store, lease, identity)
}
fn seed(
    store: &RecommendationStore,
    lease: Arc<WorkspaceLease>,
    identity: &SqlDeletionIdentity,
    committed: bool,
) -> PathBuf {
    seed_scope(store, lease, identity, committed, "s")
}
fn seed_scope(
    store: &RecommendationStore,
    lease: Arc<WorkspaceLease>,
    identity: &SqlDeletionIdentity,
    committed: bool,
    session: &str,
) -> PathBuf {
    let removal = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .prepare_removal(session)
        .unwrap();
    let tx = store
        .dir
        .join(ROLLUP_TRANSACTION_DIR)
        .join(identity.journal_name());
    fs::create_dir_all(tx.join("backups")).unwrap();
    fs::create_dir(tx.join("staged")).unwrap();
    for target in removal.targets() {
        fs::write(tx.join(target.backup), target.bytes).unwrap();
    }
    let bytes = sql_codec::encode(identity, &removal.entries, committed).unwrap();
    fs::write(tx.join("manifest.json"), bytes).unwrap();
    tx
}
fn originals(store: &RecommendationStore) -> [Vec<u8>; 3] {
    ["a", "b", "keep"].map(|id| fs::read(store.path_for(id)).unwrap())
}
#[test]
fn sql_bound_removal_retains_original_permit_and_lease_through_receipt() {
    let (root, store, lease, identity) = fixture();
    let keep = fs::read(store.path_for("keep")).unwrap();
    let bound = store.capture_cleanup().bind(lease.clone()).unwrap();
    let receipt = bound.remove_for_sql(&identity).unwrap();
    assert!(receipt.result().is_ok(), "{:?}", receipt.result());
    assert!(!store.path_for("a").exists());
    assert!(!store.path_for("b").exists());
    assert_eq!(fs::read(store.path_for("keep")).unwrap(), keep);
    assert!(!store.dir.join(ROLLUP_TRANSACTION_DIR).exists());
    assert!(matches!(store.list(), Err(StoreError::Busy)));
    drop(bound);
    drop(lease);
    assert!(WorkspaceLease::acquire(root.path()).is_err());
    drop(receipt);
    assert!(matches!(
        store.list(),
        Err(StoreError::SqlAuthorityRequired)
    ));
    assert!(WorkspaceLease::acquire(root.path()).is_ok());
}
#[test]
fn sql_bound_restarts_committed_partial_graph_and_then_runs_fresh_pass() {
    let (root, store, lease, identity) = fixture();
    let keep = fs::read(store.path_for("keep")).unwrap();
    let tx = seed(&store, lease.clone(), &identity, true);
    fs::remove_file(store.path_for("a")).unwrap();
    drop(store);
    drop(lease);
    let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
    let bound = RecommendationStore::bind_cleanup_recovery(root.path().into(), lease).unwrap();
    for _ in 0..2 {
        let receipt = bound.remove_for_sql(&identity).unwrap();
        assert!(receipt.result().is_ok(), "{:?}", receipt.result());
    }
    assert!(!tx.exists());
    assert!(!root.path().join("recommendations/b.json").exists());
    assert_eq!(
        fs::read(root.path().join("recommendations/keep.json")).unwrap(),
        keep
    );
}
#[test]
fn sql_bound_refuses_foreign_job_and_full_manifest_identity_before_effects() {
    for case in [
        "name",
        "databaseEpoch",
        "workspaceSha256",
        "sessionId",
        "createdAt",
    ] {
        let (root, store, lease, identity) = fixture();
        let other = selected(root.path(), "other-job");
        let tx = seed(&store, lease.clone(), &identity, true);
        if case != "name" {
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(tx.join("manifest.json")).unwrap()).unwrap();
            value["sqlDeletion"][case] = "foreign".into();
            fs::write(
                tx.join("manifest.json"),
                serde_json::to_vec(&value).unwrap(),
            )
            .unwrap();
        }
        let before = originals(&store);
        let manifest = fs::read(tx.join("manifest.json")).unwrap();
        let bound = store.capture_cleanup().bind(lease).unwrap();
        let receipt = bound
            .remove_for_sql(if case == "name" { &other } else { &identity })
            .unwrap();
        assert!(receipt.result().is_err(), "{case}");
        assert_eq!(originals(&store), before, "{case}");
        assert_eq!(fs::read(tx.join("manifest.json")).unwrap(), manifest);
        assert!(matches!(store.list(), Err(StoreError::Busy)));
    }
}
#[test]
fn sql_bound_v3_draft_and_first_temp_use_existing_discard_rules() {
    for first in [false, true] {
        let (_root, store, lease, identity) = fixture();
        let tx = seed(&store, lease.clone(), &identity, false);
        let mut manifest: RollupTransactionManifest =
            serde_json::from_slice(&fs::read(tx.join("manifest.json")).unwrap()).unwrap();
        manifest.committed = !first;
        let next = sql_codec::encode(&identity, &manifest.entries, manifest.committed).unwrap();
        fs::write(tx.join("manifest.json.tmp"), &next[..next.len() - 11]).unwrap();
        if first {
            fs::remove_file(tx.join("manifest.json")).unwrap();
        }
        let before = originals(&store);
        let bound = store.capture_cleanup().bind(lease).unwrap();
        let _operation = store.try_operation().unwrap();
        protocol::recover_sql(&bound.0, &identity).unwrap();
        assert_eq!(originals(&store), before);
        assert!(!tx.exists());
    }
}
#[test]
fn legacy_refuses_reserved_journals_before_recovery_or_filename_migration() {
    for missing in [false, true] {
        let (_root, store, lease, identity) = fixture();
        let tx = seed(&store, lease.clone(), &identity, true);
        if missing {
            fs::remove_file(tx.join("manifest.json")).unwrap();
        }
        let legacy = store
            .dir
            .join(ROLLUP_TRANSACTION_DIR)
            .join("tx-00000000-0000-4000-8000-000000000001");
        fs::create_dir(&legacy).unwrap();
        fs::write(legacy.join("preserve"), b"legacy").unwrap();
        let old = store.dir.join(format!("rollup:{}.json", "a".repeat(64)));
        fs::write(&old, b"legacy-name").unwrap();
        let before = originals(&store);
        assert!(store.list().is_err());
        assert!(store.get("a").is_err());
        assert!(store.put(&recommendation("new", "s")).is_err());
        assert!(store.delete_referencing_session("s").is_err());
        assert!(RecommendationStore::open(store.dir.parent().unwrap().into()).is_err());
        assert_eq!(originals(&store), before);
        assert!(old.exists());
        assert!(!store.path_for("new").exists());
        assert_eq!(fs::read(legacy.join("preserve")).unwrap(), b"legacy");
        assert!(tx.exists());
        let bound = store.capture_cleanup().bind(lease).unwrap();
        let receipt = bound.recover_removals().unwrap();
        assert!(receipt.result().is_err());
        assert_eq!(originals(&store), before);
    }
}

thread_local! {
    static AFTER_RECOVERY: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}
pub(super) fn after_recovery() {
    AFTER_RECOVERY.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook();
        }
    });
}
mod faults;
