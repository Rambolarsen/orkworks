//! Captured admission/progress around real strict-v3 restart fixtures.
use super::*;
use std::path::PathBuf;

pub(super) async fn journal(
    state: &Arc<AppState>,
    store: &Arc<SessionStore>,
    committed: bool,
) -> PathBuf {
    let identity = store
        .deletion_record(ID.into(), CREATED.into())
        .await
        .unwrap()
        .unwrap()
        .identity;
    let files = recommendations(state);
    let lease = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .lease
        .clone()
        .unwrap();
    let removal = files
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .prepare_removal(ID)
        .unwrap();
    let tx = root(state)
        .join("recommendations/.rollup-transactions")
        .join(identity.journal_name());
    std::fs::create_dir_all(tx.join("backups")).unwrap();
    std::fs::create_dir(tx.join("staged")).unwrap();
    for target in removal.targets() {
        std::fs::write(tx.join(target.backup), target.bytes).unwrap();
    }
    // Preserve the literal publisher byte order/pretty layout: a first-temp
    // prefix must match the reconstructed write, unlike a complete JSON object.
    let identity = serde_json::to_string_pretty(&identity)
        .unwrap()
        .replace('\n', "\n  ");
    let manifest = String::from_utf8(removal.manifest_bytes(committed).to_vec())
        .unwrap()
        .replacen("\"version\": 2", "\"version\": 3", 1)
        .replacen(
            "\"entries\":",
            &format!("\"sqlDeletion\": {identity},\n  \"entries\":"),
            1,
        );
    std::fs::write(tx.join("manifest.json"), manifest).unwrap();
    tx
}

#[tokio::test]
async fn captured_sql_recovers_its_committed_journal_and_preserves_prior_progress() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    seed(&state);
    store
        .deletion_progress(ID.into(), CREATED.into(), 5, false)
        .await
        .unwrap();
    let tx = journal(&state, &store, true).await;
    // A newly observed independent reference must be removed by the fresh pass.
    std::fs::write(
        root(&state).join("recommendations/late.json"),
        serde_json::to_vec(&recommendation("late", ID)).unwrap(),
    )
    .unwrap();
    // A crash after the canonical unlink but before journal cleanup/progress.
    std::fs::remove_file(root(&state).join("recommendations/a.json")).unwrap();
    let keep = std::fs::read(root(&state).join("recommendations/keep.json")).unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(receipt.files.is_ok(), "{:?}", receipt.files);
    assert!(matches!(receipt.progress, Some(Ok(21))));
    assert!(!receipt.detached);
    assert!(!tx.exists());
    assert!(!root(&state).join("recommendations/late.json").exists());
    assert_eq!(
        std::fs::read(root(&state).join("recommendations/keep.json")).unwrap(),
        keep
    );
    assert!(root(&state).join("recommendations/b.json").exists());
    assert_eq!(
        store.pending_deletions(None, 100).await.unwrap().jobs.len(),
        1
    );
    let (path, key) = store.location();
    let (path, key) = (path.to_owned(), key.to_owned());
    store.close().await.unwrap();
    let reopened = SessionStore::open(path, key).await.unwrap();
    assert_eq!(
        reopened
            .deletion_record(ID.into(), CREATED.into())
            .await
            .unwrap()
            .unwrap()
            .job
            .progress,
        21
    );
    reopened.close().await.unwrap();
}

#[tokio::test]
async fn captured_sql_rejects_epoch_change_even_without_a_pending_journal() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    seed(&state);
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let original = std::fs::read(root(&state).join("recommendations/a.json")).unwrap();
    let db = rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
    db.execute(
        "UPDATE store_header SET epoch=?1",
        [uuid::Uuid::new_v4().to_string()],
    )
    .unwrap();
    assert!(matches!(
        view.remove_recommendations().await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        std::fs::read(root(&state).join("recommendations/a.json")).unwrap(),
        original
    );
    assert_eq!(progress(&store).await, 0);
    store.close().await.unwrap();
}

#[tokio::test]
async fn captured_sql_recovers_uncommitted_states_before_a_fresh_removal_pass() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for phase in ["draft", "first-temp", "scaffold"] {
        let (_dir, state, store, writes) = deleted().await;
        seed(&state);
        let tx = journal(&state, &store, false).await;
        if phase == "first-temp" {
            let draft = std::fs::read(tx.join("manifest.json")).unwrap();
            std::fs::write(tx.join("manifest.json.tmp"), &draft[..draft.len() - 11]).unwrap();
            std::fs::remove_file(tx.join("manifest.json")).unwrap();
        } else if phase == "scaffold" {
            std::fs::remove_file(tx.join("manifest.json")).unwrap();
        }
        let receipt = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap()
            .remove_recommendations()
            .await
            .unwrap();
        assert!(receipt.files.is_ok(), "{phase}: {:?}", receipt.files);
        assert!(matches!(receipt.progress, Some(Ok(16))));
        assert!(!tx.exists());
        assert!(!root(&state).join("recommendations/a.json").exists());
        assert!(root(&state).join("recommendations/b.json").exists());
        assert!(root(&state).join("recommendations/keep.json").exists());
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn captured_sql_foreign_or_corrupt_journals_keep_all_evidence_and_progress() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for field in [
        "databaseEpoch",
        "workspaceSha256",
        "sessionId",
        "createdAt",
        "corrupt",
    ] {
        let (_dir, state, store, writes) = deleted().await;
        let files = seed(&state);
        let tx = journal(&state, &store, true).await;
        let manifest = tx.join("manifest.json");
        if field == "corrupt" {
            std::fs::write(&manifest, b"{broken").unwrap();
        } else {
            let mut value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
            value["sqlDeletion"][field] = "foreign".into();
            std::fs::write(&manifest, serde_json::to_vec(&value).unwrap()).unwrap();
        }
        let before = snapshot(&root(&state).join("recommendations"));
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let mut gate = hold(&writes.replay_removed);
        let action = tokio::spawn(view.remove_recommendations());
        gate.entered().await;
        // Errors after operation claim must retain the same permit through the runner.
        assert!(
            matches!(files.list(), Err(RecommendationError::Busy)),
            "{field}"
        );
        drop(gate);
        let receipt = action.await.unwrap().unwrap();
        assert!(receipt.files.is_err(), "{field}");
        assert!(receipt.progress.is_none());
        assert_eq!(progress(&store).await, 0);
        assert_eq!(
            snapshot(&root(&state).join("recommendations")),
            before,
            "{field}"
        );
        store.close().await.unwrap();
    }
}

fn snapshot(path: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(
        base: &std::path::Path,
        path: &std::path::Path,
        out: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(base, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(base).unwrap().into(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    visit(path, path, &mut out);
    out
}

#[tokio::test]
async fn captured_sql_unavailable_or_changed_header_refuses_before_recovery_effects() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in ["unavailable", "workspace", "epoch", "missing-job"] {
        let (_dir, state, store, writes) = deleted().await;
        seed(&state);
        let tx = journal(&state, &store, true).await;
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let before = snapshot(&root(&state).join("recommendations"));
        if change == "unavailable" {
            store.close().await.unwrap();
        } else {
            let db =
                rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
            db.execute_batch(match change {
                "workspace" => "UPDATE store_header SET workspace='/foreign'",
                "epoch" => "UPDATE store_header SET epoch='invalid'",
                _ => "DELETE FROM deletion_jobs",
            })
            .unwrap();
        }
        let result = view.remove_recommendations().await;
        assert!(
            matches!(
                (&result, change),
                (Err(StoreError::Unavailable), "unavailable")
                    | (Err(StoreError::Conflict), "workspace")
                    | (Err(StoreError::Corrupt), "epoch")
                    | (Err(StoreError::Missing), "missing-job")
            ),
            "{change}: {result:?}"
        );
        assert_eq!(snapshot(&root(&state).join("recommendations")), before);
        assert!(tx.exists());
        if change != "unavailable" {
            store.close().await.unwrap();
        }
    }
}

pub(super) fn pause_after_recovery(
    state: &Arc<AppState>,
) -> super::super::delete_replay_tests::Held {
    let gate = Arc::new(std::sync::Mutex::new(None));
    let held = hold(&gate);
    crate::taskmaster::store::sql_cleanup_test_gate::on_recovered(
        root(state).join("recommendations"),
        move || enter_replay_gate(&gate),
    );
    held
}
