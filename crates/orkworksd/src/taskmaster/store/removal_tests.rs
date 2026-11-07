#![cfg(any(target_os = "macos", target_os = "linux"))]
use super::*;
use crate::workspace_runtime::WorkspaceLease;
use std::{os::unix::fs::MetadataExt, sync::Arc};

fn fixture() -> (tempfile::TempDir, RecommendationStore, Arc<WorkspaceLease>) {
    let root = tempfile::tempdir().unwrap();
    let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
    let store = RecommendationStore::open(root.path().to_path_buf()).unwrap();
    (root, store, lease)
}
#[test]
fn recommendation_removal_preserves_original_bytes_identity_and_v2_manifest() {
    let (_root, store, lease) = fixture();
    let record = tests::recommendation("r", "s");
    let mut raw = serde_json::to_value(&record).unwrap();
    raw.as_object_mut()
        .unwrap()
        .insert("unknown".into(), serde_json::json!({"note":"retain me"}));
    let bytes = format!("  {}\n", serde_json::to_string(&raw).unwrap()).into_bytes();
    fs::write(store.path_for("r"), &bytes).unwrap();
    let identity = fs::metadata(store.path_for("r")).unwrap();
    let inventory = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap();
    let removal = inventory.prepare_session_removal("s").unwrap();
    let targets = removal.targets().collect::<Vec<_>>();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].id, "r");
    assert_eq!(targets[0].filename, "r.json");
    assert_eq!(targets[0].backup, "backups/r.json");
    assert_eq!(targets[0].bytes, bytes);
    assert_eq!(targets[0].identity, (identity.dev(), identity.ino()));
    for committed in [false, true] {
        let manifest: RollupTransactionManifest =
            serde_json::from_slice(removal.manifest_bytes(committed)).unwrap();
        assert_eq!(manifest.version, 2);
        assert_eq!(manifest.committed, committed);
        assert_eq!(manifest.entries.len(), 1);
        validate_manifest_entries(&manifest.entries).unwrap();
        let entry = &manifest.entries[0];
        assert_eq!(
            entry.old_sha256.as_deref(),
            Some(format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes)).as_str())
        );
        assert!(entry.old_exists);
        assert!(!entry.new_exists);
        assert!(entry.new_sha256.is_none());
        assert!(entry.staged.is_none());
    }
    assert_eq!(fs::read(store.path_for("r")).unwrap(), bytes);
    assert!(!store.dir.join(ROLLUP_TRANSACTION_DIR).exists());
}

#[test]
fn recommendation_removal_closes_forward_reverse_and_superseded_rollup_edges() {
    let (_root, store, lease) = fixture();
    let old_id = stable_rollup_id(&["a".into(), "b".into()]);
    let new_id = stable_rollup_id(&["a".into()]);
    let mut old = tests::rollup_parent(&old_id, &["a", "b"]);
    old.status = RecommendationStatus::Superseded;
    let new = tests::rollup_parent(&new_id, &["a"]);
    let mut a = tests::recommendation("a", "selected");
    a.status = RecommendationStatus::RolledUp;
    a.rolled_up_by = Some(new_id.clone());
    let b = tests::recommendation("b", "other");
    let unrelated = tests::recommendation("unrelated", "other");
    for record in [&old, &new, &a, &b, &unrelated] {
        fs::write(
            store.path_for(&record.id),
            serde_json::to_vec(record).unwrap(),
        )
        .unwrap();
    }
    let before = store.list_with_hashes().unwrap();
    let removal = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("selected")
        .unwrap();
    let ids = removal
        .targets()
        .map(|t| t.id.to_string())
        .collect::<Vec<_>>();
    let mut expected = vec!["a".to_string(), "b".to_string(), old_id, new_id];
    expected.sort();
    assert_eq!(ids, expected);
    for target in removal.targets() {
        assert!(!target.filename.contains(':'));
        assert!(!target.backup.contains(':'));
        if target.id.starts_with("rollup:") {
            assert!(target.filename.starts_with("rollup%3A"));
        }
    }
    drop(removal);
    assert_eq!(store.list_with_hashes().unwrap(), before);
}

#[test]
fn recommendation_removal_matches_every_reference_channel_exactly() {
    let (_root, store, lease) = fixture();
    for id in ["source", "evidence", "affected", "watermark", "unrelated"] {
        let mut record = tests::recommendation(id, "selected-extra");
        match id {
            "source" => record.source_session_ids = vec!["selected".into()],
            "evidence" => record.evidence[0].session_id = "selected".into(),
            "affected" => {
                record.workflow_improvement.affected_session_ids = vec!["selected".into()]
            }
            "watermark" => {
                record = dismissed_record(record, "now", None);
                record
                    .workflow_improvement
                    .dismissal_watermark
                    .as_mut()
                    .unwrap()
                    .affected_session_ids = vec!["selected".into()];
            }
            _ => {}
        }
        store.put(&record).unwrap();
    }
    let removal = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("selected")
        .unwrap();
    assert_eq!(
        removal.targets().map(|t| t.id).collect::<Vec<_>>(),
        vec!["affected", "evidence", "source", "watermark"]
    );
}

#[test]
fn recommendation_removal_retains_shared_permit_and_original_lease() {
    let (root, store, lease) = fixture();
    store.put(&tests::recommendation("r", "s")).unwrap();
    let independent = RecommendationStore::open(root.path().to_path_buf()).unwrap();
    let removal = store
        .capture_cleanup()
        .bind(lease.clone())
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("s")
        .unwrap();
    drop(store);
    drop(lease);
    assert!(matches!(independent.get("r"), Err(StoreError::Busy)));
    assert!(WorkspaceLease::acquire(root.path()).is_err());
    assert_eq!(removal.targets().next().unwrap().id, "r");
    drop(removal);
    assert!(independent.get("r").unwrap().is_some());
    WorkspaceLease::acquire(root.path()).unwrap();
}

#[test]
fn recommendation_removal_preparation_does_no_io_and_keeps_captured_proofs() {
    let (root, store, lease) = fixture();
    store.put(&tests::recommendation("r", "s")).unwrap();
    let original = fs::read(store.path_for("r")).unwrap();
    let metadata = fs::metadata(store.path_for("r")).unwrap();
    let inventory = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap();
    fs::remove_dir_all(root.path()).unwrap();
    let removal = inventory.prepare_session_removal("s").unwrap();
    let target = removal.targets().next().unwrap();
    assert_eq!(target.bytes, original);
    assert_eq!(target.identity, (metadata.dev(), metadata.ino()));
    assert!(!root.path().exists());
}

#[test]
fn recommendation_removal_empty_and_unreferenced_graphs_prepare_noop_manifests() {
    for populated in [false, true] {
        let (_root, store, lease) = fixture();
        if populated {
            store.put(&tests::recommendation("r", "other")).unwrap();
        }
        let removal = store
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .inspect()
            .unwrap()
            .prepare_session_removal("s")
            .unwrap();
        assert_eq!(removal.targets().count(), 0);
        for committed in [false, true] {
            let manifest: RollupTransactionManifest =
                serde_json::from_slice(removal.manifest_bytes(committed)).unwrap();
            assert_eq!(manifest.committed, committed);
            assert_eq!(manifest.version, 2);
            assert!(manifest.entries.is_empty());
        }
    }
}

#[test]
fn recommendation_removal_largest_bounded_graph_stays_supported_without_publication() {
    let (_root, store, lease) = fixture();
    for i in 0..1024 {
        let id = format!("{i:04}{}", "x".repeat(246));
        let record = tests::recommendation(&id, "s");
        fs::write(store.path_for(&id), serde_json::to_vec(&record).unwrap()).unwrap();
    }
    let inventory = store
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap();
    let removal = inventory.prepare_session_removal("s").unwrap();
    assert_eq!(removal.targets().count(), 1024);
    assert!(removal.manifest_bytes(false).len() <= 1024 * 1024);
    assert!(removal.manifest_bytes(true).len() <= 1024 * 1024);
    assert!(!store.dir.join(ROLLUP_TRANSACTION_DIR).exists());
    drop(removal);
    assert_eq!(store.list().unwrap().len(), 1024);
}

#[test]
fn recommendation_removal_manifests_are_deterministic_across_directory_order_and_roots() {
    let mut bytes = Vec::new();
    for order in [["c", "a", "b"], ["b", "a", "c"]] {
        let (_root, store, lease) = fixture();
        for id in order {
            fs::write(
                store.path_for(id),
                serde_json::to_vec(&tests::recommendation(id, "s")).unwrap(),
            )
            .unwrap();
        }
        let removal = store
            .capture_cleanup()
            .bind(lease)
            .unwrap()
            .inspect()
            .unwrap()
            .prepare_session_removal("s")
            .unwrap();
        assert_eq!(
            removal
                .targets()
                .map(|target| target.id)
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
        bytes.push((
            removal.manifest_bytes(false).to_vec(),
            removal.manifest_bytes(true).to_vec(),
        ));
    }
    assert_eq!(bytes[0], bytes[1]);
}
