//! SQL organization is not proof of current persisted finalization.
use super::*;
use crate::metadata::{
    internal_mutations::InternalMutation, lifecycle_mutations::LifecycleMutation,
};

const CREATED: &str = "2026-01-01T00:00:00Z";

fn regressed_archive(db: &mut Database, status: &str) {
    db.insert(tests::closed_document("one")).unwrap();
    db.organize("one", CREATED, "0", "archived").unwrap();
    db.mutate_internal(
        "one",
        CREATED,
        None,
        &InternalMutation::Lifecycle(LifecycleMutation::Transition {
            status: status.into(),
            had_live_handle: true,
            already_ending: false,
            live_phase: Some("active".into()),
            resume: None,
            resumed_from: None,
            timestamp: "2026-01-03T00:00:00Z".into(),
        }),
    )
    .unwrap();
    let document = db.get("one").unwrap().unwrap();
    assert_ne!(document["lifecyclePhase"], "ended");
    assert_eq!(
        db.conn
            .query_row("SELECT state FROM sessions WHERE id='one'", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "archived"
    );
}

// Observe the authoritative document, organization revision/choice, history
// generation, search membership and deletion job. Rejected actions change none.
fn snapshot(db: &Database) -> (Option<serde_json::Value>, String, String, i64, i64, i64) {
    let (choice, revision) = db
        .conn
        .query_row(
            "SELECT choice,revision FROM organization WHERE id='one'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    (
        db.get("one").unwrap(),
        choice,
        revision,
        db.conn
            .query_row("SELECT generation FROM store_header", [], |r| r.get(0))
            .unwrap(),
        db.conn
            .query_row(
                "SELECT count(*) FROM metadata_search WHERE id='one'",
                [],
                |r| r.get(0),
            )
            .unwrap(),
        db.conn
            .query_row("SELECT count(*) FROM deletion_jobs", [], |r| r.get(0))
            .unwrap(),
    )
}

#[test]
fn archived_organization_requires_current_finalized_metadata() {
    for status in ["running", "creating", "killed"] {
        for choice in ["closed", "archived"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("v1.sqlite");
            let mut db = Database::create(&path, "/workspace").unwrap();
            regressed_archive(&mut db, status);
            // The protected archived choice survives a normal store reopen.
            drop(db);
            let mut db = Database::open(&path, "/workspace").unwrap();
            let before = snapshot(&db);
            assert!(
                matches!(
                    db.organize("one", CREATED, "1", choice),
                    Err(StoreError::Conflict)
                ),
                "{status} must not permit {choice}"
            );
            assert_eq!(snapshot(&db), before);
            finalize(&mut db);
            assert_eq!(db.organize("one", CREATED, "1", choice).unwrap(), "2");
        }
    }
}

#[test]
fn archived_tombstone_requires_current_finalized_metadata() {
    for status in ["running", "creating", "killed"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        let mut db = Database::create(&path, "/workspace").unwrap();
        regressed_archive(&mut db, status);
        let revision = db.get("one").unwrap().unwrap()["metadataRevision"]
            .as_str()
            .unwrap()
            .to_owned();
        let before = snapshot(&db);
        assert!(
            matches!(
                db.begin_delete("one", CREATED, &revision, "1"),
                Err(StoreError::Conflict)
            ),
            "{status} must not permit deletion"
        );
        assert_eq!(snapshot(&db), before);
        finalize(&mut db);
        let revision = db.get("one").unwrap().unwrap()["metadataRevision"]
            .as_str()
            .unwrap()
            .to_owned();
        db.begin_delete("one", CREATED, &revision, "1").unwrap();
        assert!(db.get("one").unwrap().is_none());
        assert_eq!(
            db.conn
                .query_row("SELECT count(*) FROM deletion_jobs", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.conn
                .query_row(
                    "SELECT count(*) FROM metadata_search WHERE id='one'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}

fn finalize(db: &mut Database) {
    db.mutate_internal(
        "one",
        CREATED,
        None,
        &InternalMutation::Lifecycle(LifecycleMutation::Finalize {
            snapshot: crate::metadata::ObservedStatusSnapshotMetadata {
                value: None,
                source: "process".into(),
                confidence: None,
                observed_at: None,
                attention_origin: None,
            },
            fallback_status: "ended".into(),
            timestamp: "2026-01-04T00:00:00Z".into(),
        }),
    )
    .unwrap();
}

#[test]
fn unreadable_or_inconsistent_canonical_metadata_blocks_organization_and_tombstones() {
    for invalid in ["shape", "identity", "creation", "revision"] {
        for delete in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
            db.insert(tests::closed_document("one")).unwrap();
            db.organize("one", CREATED, "0", "archived").unwrap();
            let mut document = db.get("one").unwrap().unwrap();
            match invalid {
                "identity" => document["id"] = serde_json::json!("replacement"),
                "creation" => document["createdAt"] = serde_json::json!("other"),
                "revision" => document["metadataRevision"] = serde_json::json!("99"),
                _ => {}
            }
            let bytes = match invalid {
                "shape" => "[]".to_owned(),
                _ => serde_json::to_string(&document).unwrap(),
            };
            db.conn
                .execute("UPDATE sessions SET metadata=?1 WHERE id='one'", [&bytes])
                .unwrap();
            let generation: i64 = db
                .conn
                .query_row("SELECT generation FROM store_header", [], |r| r.get(0))
                .unwrap();
            let result = if delete {
                db.begin_delete("one", CREATED, "0", "1")
            } else {
                db.organize("one", CREATED, "1", "closed").map(|_| ())
            };
            assert!(
                matches!(
                    result,
                    Err(StoreError::Invalid(_) | StoreError::Corrupt | StoreError::TooLarge)
                ),
                "{invalid}, delete={delete}: {result:?}"
            );
            assert_eq!(
                db.conn
                    .query_row(
                        "SELECT metadata FROM sessions WHERE id='one' AND deleted=0",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                bytes
            );
            assert_eq!(
                db.conn
                    .query_row("SELECT generation FROM store_header", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                generation
            );
            assert_eq!(
                db.conn
                    .query_row(
                        "SELECT revision FROM organization WHERE id='one'",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                "1"
            );
            assert_eq!(
                db.conn
                    .query_row("SELECT count(*) FROM deletion_jobs", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert_eq!(
                db.conn
                    .query_row(
                        "SELECT count(*) FROM metadata_search WHERE id='one'",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                1
            );
        }
    }
}
