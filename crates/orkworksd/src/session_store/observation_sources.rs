//! Checked source data for a newly captured final scan; never external I/O authority.
use super::*;
use sha2::{Digest, Sha256};
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct ObservationSource {
    database_epoch: String,
    workspace_sha256: String,
    session_id: String,
    created_at: String,
}
impl ObservationSource {
    pub(super) fn captured(
        opening: &OpeningIdentity,
        workspace: &str,
        id: String,
        created: String,
    ) -> Result<Self> {
        let source = Self {
            database_epoch: opening.epoch.clone(),
            workspace_sha256: hex::encode(Sha256::digest(workspace.as_bytes())),
            session_id: id,
            created_at: created,
        };
        source.validate()?;
        Ok(source)
    }
    pub(super) fn validate(&self) -> Result<()> {
        if self.session_id.is_empty()
            || self.session_id.len() > 256
            || !self
                .session_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            || self.created_at.is_empty()
            || self.created_at.len() > 256
        {
            return Err(StoreError::Invalid(
                "invalid observation source identity".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn database_epoch(&self) -> &str {
        &self.database_epoch
    }
    pub(crate) fn workspace_sha256(&self) -> &str {
        &self.workspace_sha256
    }
    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }
    pub(crate) fn created_at(&self) -> &str {
        &self.created_at
    }
}
impl Database {
    pub(super) fn validate_observation_opening(&self, original: &OpeningIdentity) -> Result<()> {
        let _budget = queries::ReadBudget::new(&self.conn);
        let tx = self.conn.unchecked_transaction()?;
        if self.opening_identity()? != *original {
            return Err(StoreError::Conflict);
        }
        tx.commit()?;
        Ok(())
    }

    pub(super) fn validate_observation_source(&self, source: &ObservationSource) -> Result<()> {
        source.validate()?;
        let _budget = queries::ReadBudget::new(&self.conn);
        let tx = self.conn.unchecked_transaction()?;
        if self.opening_identity()?.epoch != source.database_epoch
            || hex::encode(Sha256::digest(self.workspace.as_bytes())) != source.workspace_sha256
        {
            return Err(StoreError::Conflict);
        }
        let row:Option<(String,i64,bool)>=tx.query_row("SELECT s.created_at,s.deleted,EXISTS(SELECT 1 FROM deletion_jobs j WHERE j.id=s.id) FROM sessions s WHERE s.id=?1",[&source.session_id],|r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        match row {
            Some((_, 0, true)) => return Err(StoreError::Corrupt),
            Some((created, 0, false)) if created == source.created_at => {}
            _ => return Err(StoreError::Conflict),
        }
        tx.commit()?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::super::*;
    #[tokio::test]
    async fn original_worker_epoch_cannot_be_adopted_from_a_replaced_header() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(tests::closed_document("s")).await.unwrap();
        let original = store
            .capture_observation_source("s".into(), "2026-01-01T00:00:00Z".into())
            .await
            .unwrap();
        let conn = Connection::open(dir.path().join("session-store/v1.sqlite")).unwrap();
        let epoch = uuid::Uuid::new_v4().to_string();
        conn.execute("UPDATE store_header SET epoch=?1", [epoch])
            .unwrap();
        assert!(matches!(
            store
                .capture_observation_source("s".into(), "2026-01-01T00:00:00Z".into())
                .await,
            Err(StoreError::Conflict)
        ));
        assert!(matches!(
            store.validate_observation_source(original).await,
            Err(StoreError::Conflict)
        ));
        store.close().await.unwrap();
    }
    #[test]
    fn observation_sources_check_scope_exact_live_creation_and_job_in_one_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        db.insert(tests::closed_document("s")).unwrap();
        let original = super::ObservationSource::captured(
            &db.opening_identity().unwrap(),
            "/workspace",
            "s".into(),
            "2026-01-01T00:00:00Z".into(),
        )
        .unwrap();
        for state in ["running", "closed", "archived"] {
            db.conn
                .execute("UPDATE sessions SET state=?1", [state])
                .unwrap();
            db.validate_observation_source(&original).unwrap();
        }
        let foreign = super::ObservationSource::captured(
            &db.opening_identity().unwrap(),
            "/other",
            "s".into(),
            "2026-01-01T00:00:00Z".into(),
        )
        .unwrap();
        assert!(matches!(
            db.validate_observation_source(&foreign),
            Err(StoreError::Conflict)
        ));
        db.conn
            .execute(
                "INSERT INTO deletion_jobs VALUES('s','2026-01-01T00:00:00Z',0)",
                [],
            )
            .unwrap();
        assert!(matches!(
            db.validate_observation_source(&original),
            Err(StoreError::Corrupt)
        ));
        db.conn
            .execute("UPDATE sessions SET deleted=1", [])
            .unwrap();
        assert!(matches!(
            db.validate_observation_source(&original),
            Err(StoreError::Conflict)
        ));
        db.conn.execute("DELETE FROM deletion_jobs", []).unwrap();
        db.conn
            .execute("UPDATE sessions SET deleted=0,created_at='reused'", [])
            .unwrap();
        assert!(matches!(
            db.validate_observation_source(&original),
            Err(StoreError::Conflict)
        ));
        db.conn.execute("DELETE FROM sessions", []).unwrap();
        assert!(matches!(
            db.validate_observation_source(&original),
            Err(StoreError::Conflict)
        ));
    }
    #[tokio::test]
    async fn observation_source_busy_corrupt_and_unavailable_are_visible_without_rebinding() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(tests::closed_document("s")).await.unwrap();
        let original = store
            .capture_observation_source("s".into(), "2026-01-01T00:00:00Z".into())
            .await
            .unwrap();
        let other = Connection::open(dir.path().join("session-store/v1.sqlite")).unwrap();
        other.execute_batch("BEGIN EXCLUSIVE").unwrap();
        assert!(matches!(
            store.validate_observation_source(original.clone()).await,
            Err(StoreError::Busy)
        ));
        assert!(matches!(
            store.validate_observation_opening().await,
            Err(StoreError::Busy)
        ));
        other.execute_batch("ROLLBACK").unwrap();
        store
            .validate_observation_source(original.clone())
            .await
            .unwrap();
        other
            .execute("UPDATE store_header SET epoch='not-a-uuid'", [])
            .unwrap();
        assert!(matches!(
            store.validate_observation_source(original.clone()).await,
            Err(StoreError::Corrupt)
        ));
        store.close().await.unwrap();
        assert!(matches!(
            store.validate_observation_source(original).await,
            Err(StoreError::Unavailable)
        ));
    }
}
