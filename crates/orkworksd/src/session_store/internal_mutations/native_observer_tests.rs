use super::*;
use crate::session_store::worker::SessionStore;
use serde_json::json;
const CREATED: &str = "2026-01-01T00:00:00Z";
const OBSERVED: &str = "2026-01-03T00:00:00Z";
fn active(id: &str) -> serde_json::Value {
    let mut value = crate::session_store::tests::document(id);
    value["harnessId"] = json!("codex");
    value["observedStatus"] = json!("working");
    value["attention"] = json!("working");
    value["attentionSource"] = json!("agent");
    value["attentionOrigin"] = json!("harness_hook");
    value["attentionConfidence"] = json!(1.0);
    value["workMetadataUpdatedAt"] = json!("2026-01-02T00:00:00Z");
    value
}
fn show() -> InternalMutation {
    InternalMutation::NativeShowWait {
        timestamp: OBSERVED.into(),
    }
}
#[tokio::test]
async fn native_show_wait_commits_the_tuple_and_publishes_an_attention_proof() {
    let dir = tempfile::tempdir().unwrap();
    let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
        .await
        .unwrap();
    store.create(active("native-show")).await.unwrap();
    let result = store
        .mutate_internal("native-show".into(), CREATED.into(), None, show())
        .await
        .unwrap();
    let doc = result.document.unwrap();
    assert_eq!(doc["observedStatus"], "waiting_for_input");
    assert_eq!(doc["attention"], "needs_you");
    assert_eq!(doc["attentionSource"], "agent");
    assert_eq!(doc["attentionOrigin"], "native_codex");
    assert_eq!(doc["attentionConfidence"], 1.0);
    assert_eq!(doc["lastActivity"], OBSERVED);
    assert_eq!(doc["metadataRevision"], "2");
    assert_eq!(doc["workMetadataUpdatedAt"], "2026-01-02T00:00:00Z");
    assert_eq!(doc["futureExtension"]["nested"][2], "keep");
    assert!(result.attention_proof.is_some());
    assert_eq!(store.get("native-show".into()).await.unwrap().unwrap(), doc);
    store.close().await.unwrap();
}

fn clear(started: std::time::Instant) -> InternalMutation {
    InternalMutation::NativeClearWait {
        timestamp: OBSERVED.into(),
        observation_started_at: started,
    }
}
#[tokio::test]
async fn native_clear_uses_its_exact_proof_and_preserves_newer_work() {
    let dir = tempfile::tempdir().unwrap();
    let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
        .await
        .unwrap();
    store.create(active("native-clear")).await.unwrap();
    let shown = store
        .mutate_internal("native-clear".into(), CREATED.into(), None, show())
        .await
        .unwrap();
    let proof = shown.attention_proof.unwrap();
    let shown = shown.document.unwrap();
    assert!(matches!(
        store
            .mutate_internal(
                "native-clear".into(),
                CREATED.into(),
                None,
                clear(std::time::Instant::now())
            )
            .await,
        Err(StoreError::Conflict)
    ));
    store.patch("native-clear".into(), json!({"metadataRevision":shown["metadataRevision"],"patch":{"agentMetadata":{"workFields":{"task":"Newest work","nextAction":null,"workPhase":"review","blockerDescription":null,"failedCommand":null,"failedTest":null}}}})).await.unwrap();
    let cleared = store
        .mutate_owned_attention(
            "native-clear".into(),
            CREATED.into(),
            proof.clone(),
            clear(std::time::Instant::now()),
        )
        .await
        .unwrap()
        .document
        .unwrap();
    assert_eq!(cleared["observedStatus"], "working");
    assert_eq!(cleared["attention"], "working");
    assert_eq!(cleared["attentionOrigin"], "native_codex");
    assert_eq!(cleared["attentionConfidence"], 1.0);
    assert_eq!(cleared["metadataRevision"], "4");
    assert_eq!(cleared["task"], "Newest work");
    assert_eq!(cleared["futureExtension"]["nested"][2], "keep");
    assert!(matches!(
        store
            .mutate_owned_attention(
                "native-clear".into(),
                CREATED.into(),
                proof,
                clear(std::time::Instant::now())
            )
            .await,
        Err(StoreError::Conflict)
    ));
    store.close().await.unwrap();
}
#[tokio::test]
async fn native_clear_rejects_expired_or_future_observations_without_consuming_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
        .await
        .unwrap();
    store.create(active("native-time")).await.unwrap();
    let shown = store
        .mutate_internal("native-time".into(), CREATED.into(), None, show())
        .await
        .unwrap();
    let before = shown.document.unwrap();
    let proof = shown.attention_proof.unwrap();
    let now = std::time::Instant::now();
    for started in [
        now - std::time::Duration::from_millis(301),
        now + std::time::Duration::from_secs(1),
    ] {
        assert!(matches!(
            store
                .mutate_owned_attention(
                    "native-time".into(),
                    CREATED.into(),
                    proof.clone(),
                    clear(started)
                )
                .await,
            Err(StoreError::Expired)
        ));
        assert_eq!(
            store.get("native-time".into()).await.unwrap().unwrap(),
            before
        );
    }
    assert_eq!(
        store
            .mutate_owned_attention(
                "native-time".into(),
                CREATED.into(),
                proof,
                clear(std::time::Instant::now())
            )
            .await
            .unwrap()
            .document
            .unwrap()["attention"],
        "working"
    );
    store.close().await.unwrap();
}
#[test]
fn native_clear_expiring_after_sql_staging_rolls_back_and_preserves_the_proof() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::time::{Duration, Instant};
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
    db.insert(active("native-rollback")).unwrap();
    let shown = db
        .mutate_internal("native-rollback", CREATED, None, &show())
        .unwrap();
    let before = shown.document.unwrap();
    let proof = shown.attention_proof.unwrap();
    let staged = Arc::new(AtomicBool::new(false));
    let observed = staged.clone();
    db.conn.update_hook(Some(
        move |action: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
            if action == rusqlite::hooks::Action::SQLITE_UPDATE && table == "sessions" {
                observed.store(true, Ordering::SeqCst);
            }
        },
    ));
    let started = Instant::now();
    let result = db.mutate_internal_owned_with_clock(
        "native-rollback",
        CREATED,
        None,
        Some(&proof),
        &clear(started),
        || {
            started
                + Duration::from_millis(if staged.load(Ordering::SeqCst) {
                    301
                } else {
                    300
                })
        },
    );
    assert!(
        staged.load(Ordering::SeqCst),
        "fixture never staged the real SQL update"
    );
    assert!(matches!(result, Err(StoreError::Expired)));
    assert_eq!(db.get("native-rollback").unwrap().unwrap(), before);
    assert!(db.owns_attention("native-rollback", CREATED, &proof));
    let retried = db
        .mutate_internal_owned(
            "native-rollback",
            CREATED,
            None,
            Some(&proof),
            &clear(Instant::now()),
        )
        .unwrap();
    assert_eq!(retried.document.unwrap()["attention"], "working");
    assert!(!db.owns_attention("native-rollback", CREATED, &proof));
}

#[test]
fn protected_native_tuples_are_true_noops_even_at_revision_exhaustion() {
    let show_cases = [
        json!({"attentionSource":"user"}),
        json!({"needsUserInput":true}),
        json!({"detectedQuestion":"Question?"}),
        json!({"suggestedOptions":[]}),
        json!({"attention":"needs_you"}),
        json!({"lifecycle":"dead"}),
    ];
    let clear_cases = [
        json!({"attentionSource":"user"}),
        json!({"attentionOrigin":"harness_hook"}),
        json!({"attentionConfidence":0.9}),
        json!({"observedStatus":"blocked"}),
        json!({"attention":"working"}),
        json!({"needsUserInput":false}),
        json!({"needsUserInput":true}),
        json!({"detectedQuestion":"Question?"}),
        json!({"suggestedOptions":[]}),
    ];
    for (clearing, cases) in [
        (false, show_cases.as_slice()),
        (true, clear_cases.as_slice()),
    ] {
        for changes in cases {
            let dir = tempfile::tempdir().unwrap();
            let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
            let mut value = active("protected");
            if clearing {
                value["observedStatus"] = json!("waiting_for_input");
                value["attention"] = json!("needs_you");
                value["attentionOrigin"] = json!("native_codex");
            }
            for (field, value_change) in changes.as_object().unwrap() {
                value[field] = value_change.clone();
            }
            value["metadataRevision"] = json!(u64::MAX.to_string());
            db.insert(value).unwrap();
            let before = db.get("protected").unwrap().unwrap();
            let proof = db
                .read_owned_attention("protected", CREATED)
                .unwrap()
                .map(|(_, proof)| proof);
            let result = if clearing {
                db.mutate_internal_owned(
                    "protected",
                    CREATED,
                    None,
                    proof.as_ref(),
                    &clear(std::time::Instant::now()),
                )
            } else {
                db.mutate_internal("protected", CREATED, None, &show())
            }
            .unwrap();
            assert!(result.document.is_none(), "{clearing}: {changes}");
            assert!(result.attention_proof.is_none());
            assert_eq!(db.get("protected").unwrap().unwrap(), before);
            if let Some(proof) = proof {
                assert!(db.owns_attention("protected", CREATED, &proof));
            }
        }
    }
}
#[tokio::test]
async fn native_clear_rejects_foreign_creation_worker_and_reopened_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("first");
    let first = SessionStore::initialize(root.clone(), "/workspace".into())
        .await
        .unwrap();
    let other = SessionStore::initialize(dir.path().join("other"), "/workspace".into())
        .await
        .unwrap();
    for store in [&first, &other] {
        store.create(active("identity")).await.unwrap();
        store
            .mutate_internal("identity".into(), CREATED.into(), None, show())
            .await
            .unwrap();
    }
    let (_, proof) = first
        .read_owned_attention("identity".into(), CREATED.into())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        first
            .mutate_owned_attention(
                "identity".into(),
                "different-creation".into(),
                proof.clone(),
                clear(std::time::Instant::now())
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        other
            .mutate_owned_attention(
                "identity".into(),
                CREATED.into(),
                proof.clone(),
                clear(std::time::Instant::now())
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        first
            .mutate_owned_attention("identity".into(), CREATED.into(), proof.clone(), show())
            .await,
        Err(StoreError::Conflict)
    ));
    first.close().await.unwrap();
    let reopened = SessionStore::open(root, "/workspace".into()).await.unwrap();
    assert!(reopened
        .read_owned_attention("identity".into(), CREATED.into())
        .await
        .unwrap()
        .is_none());
    assert!(matches!(
        reopened
            .mutate_owned_attention(
                "identity".into(),
                CREATED.into(),
                proof,
                clear(std::time::Instant::now())
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        reopened.get("identity".into()).await.unwrap().unwrap()["attention"],
        "needs_you"
    );
    other.close().await.unwrap();
    reopened.close().await.unwrap();
}
#[tokio::test]
async fn a_queued_native_clear_cannot_outlive_its_observation_window() {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(
        SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap(),
    );
    store.create(active("queued-native")).await.unwrap();
    let shown = store
        .mutate_internal("queued-native".into(), CREATED.into(), None, show())
        .await
        .unwrap();
    let before = shown.document.unwrap();
    let proof = shown.attention_proof.unwrap();
    let paused = crate::session_store::test_worker::pause(&store);
    let owner = store.clone();
    let retry = proof.clone();
    let intent = clear(Instant::now());
    let pending = tokio::spawn(async move {
        owner
            .mutate_owned_attention("queued-native".into(), CREATED.into(), proof, intent)
            .await
    });
    crate::session_store::test_worker::wait_for_internal(&store, "queued-native").await;
    tokio::time::sleep(Duration::from_millis(310)).await;
    drop(paused);
    assert!(matches!(pending.await.unwrap(), Err(StoreError::Expired)));
    assert_eq!(
        store.get("queued-native".into()).await.unwrap().unwrap(),
        before
    );
    assert_eq!(
        store
            .mutate_owned_attention(
                "queued-native".into(),
                CREATED.into(),
                retry,
                clear(Instant::now())
            )
            .await
            .unwrap()
            .document
            .unwrap()["attention"],
        "working"
    );
    store.close().await.unwrap();
}
