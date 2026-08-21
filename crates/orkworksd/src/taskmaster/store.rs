use super::{DismissalWatermark, Recommendation, RecommendationStatus, RecommendationType};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) enum StoreError {
    Io(io::Error),
    Json(serde_json::Error),
    InvalidTransition,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::InvalidTransition => write!(f, "recommendation is not a proposed workflow improvement"),
        }
    }
}

impl std::error::Error for StoreError {}

pub(crate) struct RecommendationStore {
    dir: PathBuf,
}

impl RecommendationStore {
    pub(crate) fn open(root: PathBuf) -> Result<Self, StoreError> {
        let dir = root.join("recommendations");
        fs::create_dir_all(&dir).map_err(StoreError::Io)?;
        Ok(Self { dir })
    }

    pub(crate) fn list(&self) -> Result<Vec<Recommendation>, StoreError> {
        let mut recommendations = Vec::new();
        for entry in fs::read_dir(&self.dir).map_err(StoreError::Io)? {
            let path = entry.map_err(StoreError::Io)?.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            recommendations.push(self.read_path(&path)?);
        }
        recommendations.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.id.cmp(&right.id))
        });
        Ok(recommendations)
    }

    pub(crate) fn get(&self, id: &str) -> Result<Option<Recommendation>, StoreError> {
        if !valid_id(id) {
            return Ok(None);
        }
        let path = self.path_for(id);
        match fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).map(Some).map_err(StoreError::Json),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(StoreError::Io(error)),
        }
    }

    pub(crate) fn put(&self, recommendation: &Recommendation) -> Result<(), StoreError> {
        let json = serde_json::to_vec_pretty(recommendation).map_err(StoreError::Json)?;
        let path = self.path_for(&recommendation.id);
        let temp = path.with_extension("json.tmp");
        let mut file = fs::File::create(&temp).map_err(StoreError::Io)?;
        use std::io::Write;
        file.write_all(&json).map_err(StoreError::Io)?;
        file.sync_all().map_err(StoreError::Io)?;
        let target_existed = path.exists();
        crate::harness::integration::atomic_replace(&temp, &path, target_existed)
            .map_err(StoreError::Io)?;
        if let Ok(directory) = fs::File::open(&self.dir) {
            let _ = directory.sync_all();
        }
        Ok(())
    }

    pub(crate) fn dismiss(
        &self,
        id: &str,
        dismissed_at: String,
    ) -> Result<Option<Recommendation>, StoreError> {
        let Some(mut recommendation) = self.get(id)? else {
            return Ok(None);
        };
        if recommendation.recommendation_type != RecommendationType::ImproveWorkflow
            || recommendation.status != RecommendationStatus::Proposed
        {
            return Err(StoreError::InvalidTransition);
        }
        let watermark = DismissalWatermark {
            dismissed_at: dismissed_at.clone(),
            dismissed_through_sequence: recommendation
                .evidence
                .iter()
                .map(|evidence| evidence.sequence)
                .max()
                .unwrap_or(0),
            observation_ids: recommendation
                .evidence
                .iter()
                .map(|evidence| evidence.observation_id.clone())
                .collect(),
            qualifying_count: recommendation.workflow_improvement.recurrence_count,
            highest_impact: recommendation.priority,
            affected_session_ids: recommendation
                .workflow_improvement
                .affected_session_ids
                .clone(),
        };
        recommendation.status = RecommendationStatus::Dismissed;
        recommendation.updated_at = dismissed_at;
        recommendation.workflow_improvement.dismissal_watermark = Some(watermark);
        self.put(&recommendation)?;
        Ok(Some(recommendation))
    }

    pub(crate) fn delete_referencing_session(&self, session_id: &str) -> Result<(), StoreError> {
        for recommendation in self.list()? {
            if references_session(&recommendation, session_id) {
                fs::remove_file(self.path_for(&recommendation.id)).map_err(StoreError::Io)?;
            }
        }
        Ok(())
    }

    pub(crate) fn scrub_orphans(
        &self,
        retained_session_ids: &HashSet<String>,
    ) -> Result<(), StoreError> {
        for recommendation in self.list()? {
            let mut referenced_session_ids = recommendation
                .source_session_ids
                .iter()
                .chain(recommendation.workflow_improvement.affected_session_ids.iter())
                .chain(
                    recommendation
                        .workflow_improvement
                        .dismissal_watermark
                        .iter()
                        .flat_map(|watermark| watermark.affected_session_ids.iter()),
                )
                .chain(recommendation.evidence.iter().map(|evidence| &evidence.session_id));
            let orphaned = referenced_session_ids.any(|id| !retained_session_ids.contains(id));
            if orphaned {
                fs::remove_file(self.path_for(&recommendation.id)).map_err(StoreError::Io)?;
            }
        }
        Ok(())
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    fn read_path(&self, path: &Path) -> Result<Recommendation, StoreError> {
        let json = fs::read_to_string(path).map_err(StoreError::Io)?;
        serde_json::from_str(&json).map_err(StoreError::Json)
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn references_session(recommendation: &Recommendation, session_id: &str) -> bool {
    recommendation.source_session_ids.iter().any(|id| id == session_id)
        || recommendation.evidence.iter().any(|evidence| evidence.session_id == session_id)
        || recommendation
            .workflow_improvement
            .affected_session_ids
            .iter()
            .any(|id| id == session_id)
        || recommendation
            .workflow_improvement
            .dismissal_watermark
            .as_ref()
            .is_some_and(|watermark| watermark.affected_session_ids.iter().any(|id| id == session_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::{
        RecommendationConfidence, RecommendationType, TargetSurface, WorkflowImprovement,
        WorkflowObservationEvidence,
    };
    use crate::workflow_observations::{Impact, ObservationKind, ObservationSource};

    fn recommendation(id: &str, session_id: &str) -> Recommendation {
        let evidence = WorkflowObservationEvidence {
            observation_id: format!("observation-{id}"),
            sequence: 4,
            session_id: session_id.into(),
            kind: ObservationKind::Obstacle,
            description: "A recurring obstacle".into(),
            evidence: "The same command failed twice".into(),
            reported_impact: Impact::High,
            source: ObservationSource::Peon,
            confidence: 0.9,
            observed_at: "2026-08-20T12:00:00Z".into(),
        };
        Recommendation {
            id: id.into(),
            workspace_id: "workspace-1".into(),
            chain_id: "chain-1".into(),
            chain_depth: 0,
            recommendation_type: RecommendationType::ImproveWorkflow,
            status: RecommendationStatus::Proposed,
            priority: Impact::High,
            title: "Improve workflow".into(),
            summary: "Make the obstacle easier to avoid".into(),
            reason: vec!["It recurred".into()],
            evidence: vec![evidence],
            source_session_ids: vec![session_id.into()],
            target_session_id: None,
            suggested_harness_id: None,
            suggested_model: None,
            suggested_working_directory: None,
            suggested_prompt: None,
            confidence: RecommendationConfidence::High,
            requires_approval: false,
            dedupe_key: "improve_workflow:v1:tooling:v1:obstacle:test".into(),
            created_at: "2026-08-20T12:00:00Z".into(),
            updated_at: "2026-08-20T12:00:00Z".into(),
            expires_at: None,
            workflow_improvement: WorkflowImprovement {
                proposed_improvement: "Remove the obstacle".into(),
                target_surface: TargetSurface::Tooling,
                observation_ids: vec!["observation-1".into()],
                recurrence_count: 1,
                affected_session_ids: vec![session_id.into()],
                impact: Impact::High,
                expected_benefit: "Less repeated failure".into(),
                supersedes_recommendation_id: None,
                dismissal_watermark: None,
            },
        }
    }

    #[test]
    fn persists_canonical_json_and_reloads_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let original = recommendation("recommendation-1", "session-1");
        store.put(&original).unwrap();

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("recommendations/recommendation-1.json"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(json["type"], "improve_workflow");
        assert_eq!(json["requiresApproval"], false);
        assert!(json["targetSessionId"].is_null());
        assert_eq!(RecommendationStore::open(dir.path().to_path_buf())
            .unwrap()
            .get("recommendation-1")
            .unwrap(), Some(original));
    }

    #[test]
    fn dismisses_in_place_with_immutable_evidence_and_watermark() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store.put(&recommendation("recommendation-1", "session-1")).unwrap();

        let dismissed = store
            .dismiss("recommendation-1", "2026-08-21T12:00:00Z".into())
            .unwrap()
            .unwrap();

        assert_eq!(dismissed.status, RecommendationStatus::Dismissed);
        assert_eq!(dismissed.evidence[0].description, "A recurring obstacle");
        assert_eq!(dismissed.workflow_improvement.dismissal_watermark
            .as_ref().unwrap().dismissed_through_sequence, 4);
    }

    #[test]
    fn deletes_references_and_scrubs_only_missing_evidence_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store.put(&recommendation("keep", "session-1")).unwrap();
        store.put(&recommendation("delete", "session-2")).unwrap();
        store.delete_referencing_session("session-2").unwrap();
        assert!(store.get("delete").unwrap().is_none());
        assert!(store.get("keep").unwrap().is_some());

        store.scrub_orphans(&HashSet::from(["session-1".to_string()])).unwrap();
        assert!(store.get("keep").unwrap().is_some());
    }
}
