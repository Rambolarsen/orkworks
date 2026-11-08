//! Bounded source data checks, never external publication or a schema change.
use super::*;
use crate::taskmaster::assessment_sources::{OriginalInputs, SourceError, SqlAssessmentSources};
use sha2::{Digest, Sha256};

impl From<SourceError> for StoreError {
    fn from(error: SourceError) -> Self {
        match error {
            SourceError::TooLarge => Self::TooLarge,
            error => Self::Invalid(error.to_string()),
        }
    }
}
impl Database {
    pub(super) fn capture_assessment_sources(
        &self,
        original: OriginalInputs,
    ) -> Result<SqlAssessmentSources> {
        self.check_assessment_source_rows(original.sources())?;
        // Return the original scope/pairs unchanged, never construct from IDs.
        Ok(original.into_sources())
    }
    pub(super) fn validate_assessment_sources(
        &self,
        sources: &SqlAssessmentSources,
        assessment_id: &str,
        input_digest: &str,
    ) -> Result<()> {
        sources.validate()?;
        if sources.assessment_id() != assessment_id || sources.input_digest() != input_digest {
            return Err(StoreError::Conflict);
        }
        self.check_assessment_source_rows(sources)
    }
    fn check_assessment_source_rows(&self, sources: &SqlAssessmentSources) -> Result<()> {
        sources.validate()?;
        let _budget = queries::ReadBudget::new(&self.conn);
        let tx = self.conn.unchecked_transaction()?;
        let identity = self.opening_identity()?;
        if identity.epoch != sources.database_epoch()
            || hex::encode(Sha256::digest(self.workspace.as_bytes())) != sources.workspace_sha256()
        {
            return Err(StoreError::Conflict);
        }
        let mut statement = tx.prepare("SELECT s.created_at,s.deleted,EXISTS(SELECT 1 FROM deletion_jobs j WHERE j.id=s.id) FROM sessions s WHERE s.id=?1")?;
        for source in sources.sources() {
            let row: Option<(String, i64, bool)> = statement
                .query_row([source.session_id()], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                })
                .optional()?;
            match row {
                Some((_, 0, true)) => return Err(StoreError::Corrupt),
                Some((created, 0, false)) if created == source.created_at() => {}
                _ => return Err(StoreError::Conflict),
            }
        }
        drop(statement);
        tx.commit()?;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
