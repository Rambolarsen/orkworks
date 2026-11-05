//! Recovery discovery only: a page or record never grants external cleanup.
use super::*;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DeletionCursor {
    epoch: String,
    workspace: String,
    id: String,
}
#[derive(Debug, Serialize)]
pub(crate) struct DeletionJob {
    pub(crate) id: String,
    pub(crate) created_at: String,
    pub(crate) progress: u8,
}
#[derive(Debug, Serialize)]
pub(crate) struct DeletionPage {
    pub(crate) jobs: Vec<DeletionJob>,
    pub(crate) cursor: Option<DeletionCursor>,
}
#[derive(Debug, Serialize)]
pub(crate) struct DeletionRecord {
    pub(crate) job: DeletionJob,
    pub(crate) document: serde_json::Value,
}
fn valid_identity(id: &str, created: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && !created.is_empty()
        && created.len() <= 256
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
}
impl Database {
    // Keyset discovery is intentionally not a snapshot. Completion does not
    // invalidate continuation; restart from None for a new pass to find jobs
    // inserted behind the cursor. Every effect still needs its original owner.
    pub(super) fn pending_deletions(
        &self,
        after: Option<DeletionCursor>,
        limit: usize,
    ) -> Result<DeletionPage> {
        if !(1..=100).contains(&limit) {
            return Err(StoreError::Invalid("invalid deletion page limit".into()));
        }
        let _budget = queries::ReadBudget::new(&self.conn);
        let tx = self.conn.unchecked_transaction()?;
        let (epoch, workspace): (String, String) = tx.query_row(
            "SELECT epoch,workspace FROM store_header WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if uuid::Uuid::parse_str(&epoch).is_err() {
            return Err(StoreError::Corrupt);
        }
        let workspace = hex::encode(Sha256::digest(workspace.as_bytes()));
        if let Some(c) = &after {
            if c.epoch != epoch || c.workspace != workspace || !valid_identity(&c.id, "cursor") {
                return Err(StoreError::Conflict);
            }
        }
        let predicate = if after.is_some() {
            "j.id>?1"
        } else {
            "j.id>=?1"
        };
        let sql = format!("SELECT j.id,j.created_at,j.progress,s.deleted FROM deletion_jobs j LEFT JOIN sessions s ON s.id=j.id AND s.created_at=j.created_at WHERE {predicate} ORDER BY j.id LIMIT ?2");
        let mut statement = tx.prepare(&sql)?;
        let mut rows = statement.query(rusqlite::params![
            after.as_ref().map(|c| c.id.as_str()).unwrap_or(""),
            (limit + 1) as i64
        ])?;
        let mut jobs = Vec::with_capacity(limit);
        let mut more = false;
        while let Some(row) = rows.next()? {
            let job = DeletionJob {
                id: row.get(0)?,
                created_at: row.get(1)?,
                progress: row.get(2)?,
            };
            if row.get::<_, Option<i64>>(3)? != Some(1) || !valid_identity(&job.id, &job.created_at)
            {
                return Err(StoreError::Corrupt);
            }
            if jobs.len() == limit {
                more = true;
                break;
            }
            jobs.push(job);
        }
        let cursor = if more {
            Some(DeletionCursor {
                epoch,
                workspace,
                id: jobs.last().ok_or(StoreError::Corrupt)?.id.clone(),
            })
        } else {
            None
        };
        drop(rows);
        drop(statement);
        tx.commit()?;
        let page = DeletionPage { jobs, cursor };
        if serde_json::to_vec(&page)?.len() > 256 * 1024 {
            return Err(StoreError::TooLarge);
        }
        Ok(page)
    }
    pub(super) fn deletion_record(
        &self,
        id: &str,
        created: &str,
    ) -> Result<Option<DeletionRecord>> {
        if !valid_identity(id, created) {
            return Err(StoreError::Invalid("invalid creation identity".into()));
        }
        let _budget = queries::ReadBudget::new(&self.conn);
        let tx = self.conn.unchecked_transaction()?;
        let job: Option<(u8, Option<i64>)> = tx.query_row(
            "SELECT j.progress,s.deleted FROM deletion_jobs j LEFT JOIN sessions s ON s.id=j.id AND s.created_at=j.created_at WHERE j.id=?1 AND j.created_at=?2",
            rusqlite::params![id, created], |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional()?;
        let Some((progress, deleted)) = job else {
            return Ok(None);
        };
        if deleted != Some(1) {
            return Err(StoreError::Corrupt);
        }
        let document = read_document(&tx, id, true)?.ok_or(StoreError::Corrupt)?;
        if document["createdAt"].as_str() != Some(created) {
            return Err(StoreError::Corrupt);
        }
        tx.commit()?;
        let record = DeletionRecord {
            job: DeletionJob {
                id: id.into(),
                created_at: created.into(),
                progress,
            },
            document,
        };
        if serde_json::to_vec(&record)?.len() > MAX_RECORD + 4096 {
            return Err(StoreError::TooLarge);
        }
        Ok(Some(record))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::closed_document;
    use super::super::*;
    const CREATED: &str = "2026-01-01T00:00:00Z";

    fn tombstone(db: &mut Database, id: &str) {
        db.insert(closed_document(id)).unwrap();
        db.begin_delete(id, CREATED, "0", "0").unwrap();
    }

    #[test]
    fn recovery_reopens_progress_without_resurrecting_hidden_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        let mut db = Database::create(&path, "/workspace").unwrap();
        tombstone(&mut db, "pending");
        db.deletion_progress("pending", CREATED, 5, false).unwrap();
        drop(db);
        let db = Database::open(&path, "/workspace").unwrap();
        let page = db.pending_deletions(None, 50).unwrap();
        assert!(page.cursor.is_none());
        assert_eq!(page.jobs.len(), 1);
        assert_eq!(page.jobs[0].id, "pending");
        assert_eq!(page.jobs[0].created_at, CREATED);
        assert_eq!(page.jobs[0].progress, 5);
        let selected = db.deletion_record("pending", CREATED).unwrap().unwrap();
        assert_eq!(selected.job.progress, 5);
        assert_eq!(selected.document["futureExtension"]["nested"][2], "keep");
        assert!(db.get("pending").unwrap().is_none());
        assert!(db.organization_state("pending").unwrap().is_none());
        assert!(db.exists(&["pending".into()]).unwrap().is_empty());
        assert!(db.history(HistoryQuery::default()).unwrap().rows.is_empty());
        assert!(db.retention(i64::MAX, 100).unwrap().is_empty());
    }

    #[test]
    fn recovery_pages_survive_completion_and_reopen_but_reject_another_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        let mut db = Database::create(&path, "/workspace").unwrap();
        for id in ["a", "b", "c"] {
            tombstone(&mut db, id);
        }
        let first = db.pending_deletions(None, 1).unwrap();
        assert_eq!(first.jobs[0].id, "a");
        let cursor = first.cursor.unwrap();
        // This insertion sorts behind the traversal and needs a new pass.
        tombstone(&mut db, "0");
        db.deletion_progress("a", CREATED, 0, true).unwrap();
        drop(db);
        let mut db = Database::open(&path, "/workspace").unwrap();
        let next = db.pending_deletions(Some(cursor.clone()), 1).unwrap();
        assert_eq!(next.jobs[0].id, "b");
        db.deletion_progress("b", CREATED, 0, true).unwrap();
        let last = db.pending_deletions(next.cursor, 1).unwrap();
        assert_eq!(last.jobs[0].id, "c");
        assert!(last.cursor.is_none());
        let restarted = db.pending_deletions(None, 1).unwrap();
        assert_eq!(restarted.jobs[0].id, "0");
        let other = Database::create(&dir.path().join("other.sqlite"), "/workspace").unwrap();
        assert!(matches!(
            other.pending_deletions(Some(cursor), 1),
            Err(StoreError::Conflict)
        ));
        for limit in [0, 101, usize::MAX] {
            assert!(matches!(
                db.pending_deletions(None, limit),
                Err(StoreError::Invalid(_))
            ));
        }
    }

    #[test]
    fn recovery_selected_read_is_exact_and_does_not_adopt_reused_identity() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        tombstone(&mut db, "reused");
        assert!(db.deletion_record("reused", "other").unwrap().is_none());
        assert!(db.deletion_record("missing", CREATED).unwrap().is_none());
        db.deletion_progress("reused", CREATED, 0, true).unwrap();
        let mut replacement = closed_document("reused");
        replacement["createdAt"] = serde_json::json!("2026-02-01T00:00:00Z");
        db.insert(replacement).unwrap();
        assert!(db.deletion_record("reused", CREATED).unwrap().is_none());
        assert!(db
            .deletion_record("reused", "2026-02-01T00:00:00Z")
            .unwrap()
            .is_none());
        db.begin_delete("reused", "2026-02-01T00:00:00Z", "0", "0")
            .unwrap();
        assert!(db.deletion_record("reused", CREATED).unwrap().is_none());
        assert!(db
            .deletion_record("reused", "2026-02-01T00:00:00Z")
            .unwrap()
            .is_some());
    }

    #[test]
    fn recovery_reports_inconsistent_jobs_and_canonical_identity() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        tombstone(&mut db, "broken");
        db.conn
            .execute("UPDATE sessions SET deleted=0 WHERE id='broken'", [])
            .unwrap();
        assert!(matches!(
            db.pending_deletions(None, 1),
            Err(StoreError::Corrupt)
        ));
        assert!(matches!(
            db.deletion_record("broken", CREATED),
            Err(StoreError::Corrupt)
        ));
        db.conn.execute("UPDATE sessions SET deleted=1,metadata=json_set(metadata,'$.createdAt','other') WHERE id='broken'", []).unwrap();
        assert!(matches!(
            db.deletion_record("broken", CREATED),
            Err(StoreError::Corrupt)
        ));
        db.conn.pragma_update(None, "foreign_keys", false).unwrap();
        db.conn
            .execute("DELETE FROM sessions WHERE id='broken'", [])
            .unwrap();
        assert!(matches!(
            db.pending_deletions(None, 1),
            Err(StoreError::Corrupt)
        ));
        assert!(matches!(
            db.deletion_record("broken", CREATED),
            Err(StoreError::Corrupt)
        ));
    }

    #[tokio::test]
    async fn recovery_worker_exposes_tombstones_after_close_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(closed_document("pending")).await.unwrap();
        store
            .begin_delete("pending".into(), CREATED.into(), "1".into(), "0".into())
            .await
            .unwrap();
        store
            .deletion_progress("pending".into(), CREATED.into(), 3, false)
            .await
            .unwrap();
        store.close().await.unwrap();
        let store = SessionStore::open(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let page = store.pending_deletions(None, 1).await.unwrap();
        assert_eq!(page.jobs[0].progress, 3);
        assert_eq!(
            store
                .deletion_record("pending".into(), CREATED.into())
                .await
                .unwrap()
                .unwrap()
                .job
                .progress,
            3
        );
        store.close().await.unwrap();
        assert!(matches!(
            store.pending_deletions(None, 1).await,
            Err(StoreError::Unavailable)
        ));
    }
    #[test]
    fn recovery_bounds_maximum_escaped_pages_and_selected_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let created = "\u{1}".repeat(256);
        for n in 0..101 {
            let id = format!("{n:03}{}", "x".repeat(253));
            let mut value = closed_document(&id);
            value["createdAt"] = serde_json::json!(created);
            if n == 0 {
                value["futureExtension"] = serde_json::json!("x".repeat(MAX_RECORD - 10000));
            }
            db.insert(value).unwrap();
            db.begin_delete(&id, &created, "0", "0").unwrap();
        }
        let page = db.pending_deletions(None, 100).unwrap();
        assert_eq!(page.jobs.len(), 100);
        assert!(page.cursor.is_some());
        assert!(serde_json::to_vec(&page).unwrap().len() < 256 * 1024);
        let record = db
            .deletion_record(&page.jobs[0].id, &created)
            .unwrap()
            .unwrap();
        assert_eq!(
            record.document["futureExtension"].as_str().unwrap().len(),
            MAX_RECORD - 10000
        );
        assert!(serde_json::to_vec(&record).unwrap().len() < MAX_RECORD + 4096);
        assert_eq!(
            db.pending_deletions(page.cursor, 100).unwrap().jobs.len(),
            1
        );
    }

    #[test]
    fn recovery_pages_seek_the_job_index_without_reading_session_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let mut statement = db.conn.prepare("EXPLAIN QUERY PLAN SELECT j.id,j.created_at,j.progress,s.deleted FROM deletion_jobs j LEFT JOIN sessions s ON s.id=j.id AND s.created_at=j.created_at WHERE j.id>?1 ORDER BY j.id LIMIT ?2").unwrap();
        let plan = statement
            .query_map(rusqlite::params!["a", 101], |r| r.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
            .join("\n");
        assert!(plan.contains("SEARCH j USING INDEX"), "{plan}");
        assert!(plan.contains("SEARCH s USING INDEX"), "{plan}");
        assert!(!plan.contains("TEMP B-TREE"), "{plan}");
        // Deny any canonical payload read. Discovery must still work; selected
        // reads are the only recovery operation allowed to load that payload.
        db.conn
            .authorizer(Some(|ctx: rusqlite::hooks::AuthContext<'_>| {
                if matches!(
                    ctx.action,
                    rusqlite::hooks::AuthAction::Read {
                        table_name: "sessions",
                        column_name: "metadata"
                    }
                ) {
                    rusqlite::hooks::Authorization::Deny
                } else {
                    rusqlite::hooks::Authorization::Allow
                }
            }));
        assert!(db.pending_deletions(None, 100).unwrap().jobs.is_empty());
    }
    #[test]
    fn recovery_busy_is_an_error_and_preserves_the_pending_job() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        let mut db = Database::create(&path, "/workspace").unwrap();
        tombstone(&mut db, "pending");
        let blocker = Connection::open(&path).unwrap();
        blocker.execute_batch("BEGIN EXCLUSIVE").unwrap();
        assert!(matches!(
            db.pending_deletions(None, 1),
            Err(StoreError::Busy)
        ));
        assert!(matches!(
            db.deletion_record("pending", CREATED),
            Err(StoreError::Busy)
        ));
        blocker.execute_batch("ROLLBACK").unwrap();
        assert_eq!(db.pending_deletions(None, 1).unwrap().jobs.len(), 1);
        assert_eq!(
            db.deletion_record("pending", CREATED)
                .unwrap()
                .unwrap()
                .job
                .progress,
            0
        );
    }
}
