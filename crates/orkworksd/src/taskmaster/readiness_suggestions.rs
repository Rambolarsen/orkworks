//! Source-free combined-operation linkage. Evidence remains owned by the two
//! existing assessment stores; restart recovery never dispatches inference.
use super::{
    assessment_report::{AssessmentRecord, AssessmentState},
    assessment_store::AssessmentStore,
    readiness_store::{now, ReadinessStore, RunState},
};
use serde::{Deserialize, Serialize};
use std::path::Path;

const FILE: &str = "readiness-suggestion-run.json";
const CAP: usize = 8 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    Scoring,
    Suggesting,
    Finished,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SuggestionRun {
    pub format_version: u8,
    pub id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub state: RunState,
    pub phase: Phase,
    pub readiness_run_id: Option<String>,
    pub assessment_id: Option<String>,
    pub recommendation_id: Option<String>,
    pub reason_code: Option<String>,
}
impl SuggestionRun {
    pub(crate) fn queued(readiness_id: &str) -> Self {
        Self {
            format_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            started_at: now(),
            completed_at: None,
            state: RunState::Queued,
            phase: Phase::Scoring,
            readiness_run_id: Some(readiness_id.into()),
            assessment_id: None,
            recommendation_id: None,
            reason_code: None,
        }
    }
    pub(crate) fn finish(&mut self, state: RunState, code: &str) {
        self.state = state;
        self.phase = Phase::Finished;
        self.completed_at = Some(now());
        self.reason_code = Some(code.into());
    }
    fn validate(&self) -> Result<(), String> {
        let active = matches!(self.state, RunState::Queued | RunState::Running);
        let valid_id = |id: &str| {
            !id.is_empty()
                && id.len() <= 128
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        };
        let known_reason = self.reason_code.as_deref().is_none_or(|code| {
            matches!(
                code,
                "active_recommendation"
                    | "no_proposal"
                    | "recommendation_created"
                    | "inputs_changed"
                    | "guidance_unavailable"
                    | "metadata_unavailable"
                    | "scoring_failed"
                    | "suggestion_failed"
                    | "interrupted"
            )
        });
        if self.format_version != 1
            || uuid::Uuid::parse_str(&self.id).is_err()
            || chrono::DateTime::parse_from_rfc3339(&self.started_at).is_err()
            || self
                .completed_at
                .as_ref()
                .is_some_and(|t| chrono::DateTime::parse_from_rfc3339(t).is_err())
            || active != self.completed_at.is_none()
            || active == (self.phase == Phase::Finished)
            || active != self.reason_code.is_none()
            || !known_reason
            || self
                .readiness_run_id
                .as_ref()
                .is_none_or(|id| uuid::Uuid::parse_str(id).is_err())
            || self.assessment_id.as_ref().is_some_and(|id| !valid_id(id))
            || self
                .recommendation_id
                .as_ref()
                .is_some_and(|id| !valid_id(id))
        {
            return Err("Combined assessment metadata is invalid".into());
        }
        let valid_stage = match self.phase {
            Phase::Scoring => self.assessment_id.is_none() && self.recommendation_id.is_none(),
            Phase::Suggesting => {
                self.state == RunState::Running
                    && self.assessment_id.is_some()
                    && self.recommendation_id.is_none()
            }
            Phase::Finished => match self.state {
                RunState::Interrupted => {
                    self.recommendation_id.is_none()
                        && matches!(
                            self.reason_code.as_deref(),
                            Some("interrupted" | "inputs_changed")
                        )
                }
                RunState::Failed => {
                    self.recommendation_id.is_none()
                        && match self.reason_code.as_deref() {
                            Some("scoring_failed") => self.assessment_id.is_none(),
                            Some("suggestion_failed" | "guidance_unavailable") => {
                                self.assessment_id.is_some()
                            }
                            Some("metadata_unavailable") => true,
                            _ => false,
                        }
                }
                RunState::Succeeded => true,
                _ => false,
            },
        };
        if !valid_stage {
            return Err("Combined assessment stage is inconsistent".into());
        }
        if self.state == RunState::Succeeded
            && !match self.reason_code.as_deref() {
                Some("no_proposal") => {
                    self.assessment_id.is_some() && self.recommendation_id.is_none()
                }
                Some("recommendation_created") => {
                    self.assessment_id.is_some() && self.recommendation_id.is_some()
                }
                Some("active_recommendation") => {
                    self.assessment_id.is_none() && self.recommendation_id.is_some()
                }
                _ => false,
            }
        {
            return Err("Combined assessment outcome is invalid".into());
        }
        Ok(())
    }
}
pub(crate) fn load(root: &Path) -> Result<Option<SuggestionRun>, String> {
    let value = ReadinessStore::new(root).read::<SuggestionRun>(FILE, CAP)?;
    if let Some(run) = &value {
        run.validate()?;
    }
    Ok(value)
}
pub(crate) fn save(root: &Path, run: &SuggestionRun) -> Result<(), String> {
    run.validate()?;
    ReadinessStore::new(root).write(FILE, run, CAP)
}
/// Caller holds the installation-wide analysis lease and has recovered children.
pub(crate) fn recover(root: &Path) -> Result<(), String> {
    let Some(mut parent) = load(root)? else {
        return Ok(());
    };
    if !matches!(parent.state, RunState::Queued | RunState::Running) {
        return Ok(());
    }
    let score_exists = ReadinessStore::new(root)
        .history()?
        .snapshots
        .iter()
        .any(|s| Some(&s.id) == parent.readiness_run_id.as_ref());
    if !score_exists && parent.phase == Phase::Scoring {
        if ReadinessStore::new(root).run()?.is_some_and(|child| {
            Some(&child.id) == parent.readiness_run_id.as_ref() && child.state == RunState::Failed
        }) {
            parent.finish(RunState::Failed, "scoring_failed");
            return save(root, &parent);
        }
    }
    if score_exists && parent.assessment_id.is_some() {
        if let Some(record) = AssessmentStore::new(root).load::<AssessmentRecord>()? {
            if Some(&record.id) == parent.assessment_id.as_ref()
                && record.state == AssessmentState::Failed
            {
                parent.finish(RunState::Failed, "suggestion_failed");
                return save(root, &parent);
            }
            if Some(&record.id) == parent.assessment_id.as_ref()
                && record.state == AssessmentState::Succeeded
            {
                if let Some(report) = record.report.as_ref().filter(|r| r.id == record.id) {
                    parent.recommendation_id = report.recommendation_id.clone();
                    let code = if parent.recommendation_id.is_some() {
                        "recommendation_created"
                    } else {
                        "no_proposal"
                    };
                    parent.finish(RunState::Succeeded, code);
                    return save(root, &parent);
                }
            }
        }
    }
    parent.finish(RunState::Interrupted, "interrupted");
    save(root, &parent)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SuggestionStatus {
    #[serde(flatten)]
    pub run: SuggestionRun,
    pub no_proposal_reason: Option<String>,
}
pub(crate) fn project(
    state: &std::sync::Arc<crate::AppState>,
    runtime: &super::runtime::TaskmasterRuntime,
    mut run: SuggestionRun,
) -> SuggestionStatus {
    let mut no_proposal_reason = None;
    if run.state == RunState::Succeeded && run.assessment_id.is_some() {
        let status = super::assessment_access::status_for(state, runtime);
        let report = &status["report"];
        if report["id"].as_str() == run.assessment_id.as_deref()
            && status["state"] == "succeeded"
            && report["recommendationId"].as_str() == run.recommendation_id.as_deref()
        {
            no_proposal_reason = report["noProposalReason"].as_str().map(str::to_owned);
        } else {
            // Keep the fixed audit identities; do not expose a stale or unrelated result.
            run.reason_code = Some("inputs_changed".into());
            run.recommendation_id = None;
        }
    }
    if run.state == RunState::Succeeded
        && run.reason_code.as_deref() == Some("active_recommendation")
    {
        let visible = run.recommendation_id.as_deref().is_some_and(|id| {
            crate::session_application::SessionApplication::new(state.clone())
                .get_recommendation(id)
                .ok()
                .flatten()
                .is_some_and(|r| {
                    matches!(
                        r.status,
                        super::RecommendationStatus::Proposed
                            | super::RecommendationStatus::Accepted
                            | super::RecommendationStatus::Executing
                    ) && !r.assessment.as_ref().is_some_and(|p| p.access_redacted)
                })
        });
        if !visible {
            run.reason_code = Some("inputs_changed".into());
            run.recommendation_id = None;
        }
    }
    SuggestionStatus {
        run,
        no_proposal_reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_metadata_is_strict_bounded_and_contains_only_fixed_audit_fields() {
        let root = tempfile::tempdir().unwrap();
        let score_id = uuid::Uuid::new_v4().to_string();
        let run = SuggestionRun::queued(&score_id);
        save(root.path(), &run).unwrap();
        assert_eq!(load(root.path()).unwrap(), Some(run.clone()));
        let original = std::fs::read(root.path().join(FILE)).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        value["formatVersion"] = serde_json::json!(2);
        std::fs::write(root.path().join(FILE), value.to_string()).unwrap();
        assert!(load(root.path()).is_err());
        assert_eq!(
            std::fs::read(root.path().join(FILE)).unwrap(),
            value.to_string().as_bytes()
        );
        value["formatVersion"] = serde_json::json!(1);
        value["sourceText"] = serde_json::json!("not permitted");
        std::fs::write(root.path().join(FILE), value.to_string()).unwrap();
        assert!(load(root.path()).is_err());
        std::fs::write(root.path().join(FILE), vec![b' '; CAP + 1]).unwrap();
        assert!(load(root.path()).is_err());
        let mut invalid = run.clone();
        invalid.reason_code = Some("private provider error".into());
        assert!(save(root.path(), &invalid).is_err());
        invalid = run;
        invalid.finish(RunState::Succeeded, "recommendation_created");
        assert!(
            save(root.path(), &invalid).is_err(),
            "success without exact child identities was accepted"
        );
    }
    #[test]
    fn recovery_preserves_exact_committed_stage_failures() {
        use super::super::readiness_store::{Run, Snapshot};
        for suggestion in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let mut score = Run::queued();
            let mut parent = SuggestionRun::queued(&score.id);
            if suggestion {
                ReadinessStore::new(root.path())
                    .append(Snapshot {
                        id: score.id.clone(),
                        started_at: score.started_at.clone(),
                        completed_at: now(),
                        redacted: true,
                        data: None,
                    })
                    .unwrap();
                let id = format!("assessment-{}", uuid::Uuid::new_v4());
                parent.state = RunState::Running;
                parent.phase = Phase::Suggesting;
                parent.assessment_id = Some(id.clone());
                AssessmentStore::new(root.path())
                    .save(&AssessmentRecord {
                        format_version: 2,
                        input_manifest: None,
                        phase: None,
                        id,
                        state: AssessmentState::Failed,
                        started_at: now(),
                        completed_at: Some(now()),
                        report: None,
                        pending_recommendation: None,
                        error_summary: Some("Taskmaster returned an invalid assessment".into()),
                        reused: false,
                    })
                    .unwrap();
            } else {
                score.finish(RunState::Failed, Some("scoring_invalid_response"));
                ReadinessStore::new(root.path()).save_run(&score).unwrap();
            }
            save(root.path(), &parent).unwrap();
            recover(root.path()).unwrap();
            let recovered = load(root.path()).unwrap().unwrap();
            assert_eq!(recovered.state, RunState::Failed, "suggestion={suggestion}");
            assert_eq!(
                recovered.reason_code.as_deref(),
                Some(if suggestion {
                    "suggestion_failed"
                } else {
                    "scoring_failed"
                })
            );
        }
    }
    #[test]
    fn persisted_phase_and_child_id_combinations_cannot_contradict_each_other() {
        let root = tempfile::tempdir().unwrap();
        let mut run = SuggestionRun::queued(&uuid::Uuid::new_v4().to_string());
        run.phase = Phase::Suggesting;
        assert!(
            save(root.path(), &run).is_err(),
            "suggestion phase without a child identity was accepted"
        );
    }

    #[test]
    fn interrupted_scoring_is_recovered_without_inference_or_destroying_the_record() {
        let root = tempfile::tempdir().unwrap();
        let run = SuggestionRun::queued(&uuid::Uuid::new_v4().to_string());
        save(root.path(), &run).unwrap();
        recover(root.path()).unwrap();
        let saved = load(root.path()).unwrap().unwrap();
        assert_eq!(saved.id, run.id);
        assert_eq!(saved.state, RunState::Interrupted);
        assert_eq!(saved.reason_code.as_deref(), Some("interrupted"));
        assert!(!root.path().join("assessment.json").exists());
    }
}
