use super::super::*;
use crate::taskmaster::assessment_sources::OriginalInputs;
use serde_json::json;
use sha2::{Digest, Sha256};
fn original(db: &Database, id: &str, created: &str) -> OriginalInputs {
    let epoch: String = db
        .conn
        .query_row("SELECT epoch FROM store_header", [], |r| r.get(0))
        .unwrap();
    OriginalInputs::fixture("assessment-1", json!({"schemaVersion":1,"databaseEpoch":epoch,"workspaceSha256":hex::encode(Sha256::digest(db.workspace.as_bytes())),"observations":[{"value":{"id":"observation"},"sources":[{"sessionId":id,"createdAt":created}]}],"recommendations":[],"readinessFocus":null})).unwrap()
}
#[test]
fn assessment_sources_refuse_reused_creation_and_changed_epoch_without_rebinding() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
    db.insert(super::super::tests::closed_document("s"))
        .unwrap();
    let claim = original(&db, "s", "original-creation");
    assert!(matches!(
        db.capture_assessment_sources(claim),
        Err(StoreError::Conflict)
    ));
    let claim = original(&db, "s", "2026-01-01T00:00:00Z");
    db.conn
        .execute(
            "UPDATE store_header SET epoch=?1",
            [uuid::Uuid::new_v4().to_string()],
        )
        .unwrap();
    assert!(matches!(
        db.capture_assessment_sources(claim),
        Err(StoreError::Conflict)
    ));
}

#[test]
fn source_rows_require_exact_live_creation_and_valid_scope_for_empty_sets() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
    db.insert(super::super::tests::closed_document("s"))
        .unwrap();
    let epoch = db.opening_identity().unwrap().epoch;
    let make = || {
        crate::taskmaster::assessment_sources::tests::original(
            &epoch,
            "/workspace",
            &[("s", "2026-01-01T00:00:00Z")],
        )
    };
    let packet = db.capture_assessment_sources(make()).unwrap();
    let bytes = serde_json::to_vec(&packet).unwrap();
    for state in ["running", "closed", "archived"] {
        db.conn
            .execute("UPDATE sessions SET state=?1", [state])
            .unwrap();
        assert_eq!(db.capture_assessment_sources(make()).unwrap(), packet);
    }
    for (id, digest) in [
        ("other-assessment", packet.input_digest()),
        (packet.assessment_id(), "wrong-digest"),
    ] {
        assert!(matches!(
            db.validate_assessment_sources(&packet, id, digest),
            Err(StoreError::Conflict)
        ));
    }
    db.conn
        .execute(
            "INSERT INTO deletion_jobs VALUES('s','2026-01-01T00:00:00Z',0)",
            [],
        )
        .unwrap();
    assert!(matches!(
        db.capture_assessment_sources(make()),
        Err(StoreError::Corrupt)
    ));
    db.conn
        .execute("UPDATE sessions SET deleted=1", [])
        .unwrap();
    assert!(matches!(
        db.capture_assessment_sources(make()),
        Err(StoreError::Conflict)
    ));
    db.conn.execute("DELETE FROM deletion_jobs", []).unwrap();
    db.conn.execute("DELETE FROM sessions", []).unwrap();
    assert!(matches!(
        db.capture_assessment_sources(make()),
        Err(StoreError::Conflict)
    ));
    let empty = crate::taskmaster::assessment_sources::tests::original(&epoch, "/workspace", &[])
        .into_sources();
    db.validate_assessment_sources(&empty, empty.assessment_id(), empty.input_digest())
        .unwrap();
    let wrong = crate::taskmaster::assessment_sources::tests::original(&epoch, "/foreign", &[]);
    assert!(matches!(
        db.capture_assessment_sources(wrong),
        Err(StoreError::Conflict)
    ));
    db.conn
        .execute(
            "UPDATE store_header SET epoch=?1",
            [uuid::Uuid::new_v4().to_string()],
        )
        .unwrap();
    assert!(matches!(
        db.validate_assessment_sources(&empty, empty.assessment_id(), empty.input_digest()),
        Err(StoreError::Conflict)
    ));
    assert_eq!(serde_json::to_vec(&packet).unwrap(), bytes);
}

#[tokio::test]
async fn copied_pending_fixture_survives_worker_reopen_and_refuses_changed_sources_without_effects()
{
    use crate::taskmaster::assessment_store::{sql_source_tests::pending, AssessmentStore};
    for change in ["unchanged", "creation", "tombstone", "epoch", "unavailable"] {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store
            .create(super::super::tests::closed_document("s"))
            .await
            .unwrap();
        let epoch = store.opening_identity().await.unwrap().epoch;
        let original = crate::taskmaster::assessment_sources::tests::original(
            &epoch,
            "/workspace",
            &[("s", "2026-01-01T00:00:00Z")],
        );
        let expected = serde_json::to_vec(original.sources()).unwrap();
        let sources = store.capture_assessment_sources(original).await.unwrap();
        assert_eq!(serde_json::to_vec(&sources).unwrap(), expected);
        let assessments = AssessmentStore::new(dir.path());
        assessments.save(&pending(sources)).unwrap();
        let before = std::fs::read(dir.path().join("assessment.json")).unwrap();
        store.close().await.unwrap();
        let path = dir.path().join("session-store/v1.sqlite");
        if change == "unavailable" {
            std::fs::remove_file(&path).unwrap();
            assert!(SessionStore::open(dir.path().into(), "/workspace".into())
                .await
                .is_err());
        } else {
            let db = Database::open(&path, "/workspace").unwrap();
            match change {
                "creation" => {
                    db.conn
                        .execute("UPDATE sessions SET created_at='reused'", [])
                        .unwrap();
                }
                "tombstone" => {
                    db.conn
                        .execute("UPDATE sessions SET deleted=1", [])
                        .unwrap();
                }
                "epoch" => {
                    db.conn
                        .execute(
                            "UPDATE store_header SET epoch=?1",
                            [uuid::Uuid::new_v4().to_string()],
                        )
                        .unwrap();
                }
                _ => {}
            }
            drop(db);
            let reopened = SessionStore::open(dir.path().into(), "/workspace".into())
                .await
                .unwrap();
            let saved = assessments.load_sql().unwrap().unwrap();
            let sources = saved.sources_for_sql_validation().unwrap();
            assert_eq!(serde_json::to_vec(sources).unwrap(), expected);
            let result = reopened
                .validate_assessment_sources(
                    sources.clone(),
                    saved.id.clone(),
                    sources.input_digest().into(),
                )
                .await;
            if change == "unchanged" {
                result.unwrap();
            } else {
                assert!(matches!(result, Err(StoreError::Conflict)));
            }
            assert_eq!(
                saved.state,
                crate::taskmaster::assessment_report::AssessmentState::PendingRecommendation
            );
            reopened.close().await.unwrap();
        }
        assert_eq!(
            std::fs::read(dir.path().join("assessment.json")).unwrap(),
            before
        );
        assert!(!dir.path().join("recommendations").exists());
        assert!(!dir.path().join("sessions").exists());
    }
}
