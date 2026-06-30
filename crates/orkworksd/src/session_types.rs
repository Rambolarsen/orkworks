use crate::harness;
use crate::metadata;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SessionInfo {
    pub(crate) id: String,
    pub(crate) label: String,
    #[serde(rename = "harnessId", skip_serializing_if = "Option::is_none")]
    pub(crate) harness_id: Option<String>,
    #[serde(rename = "modelProviderId", skip_serializing_if = "Option::is_none")]
    pub(crate) model_provider_id: Option<String>,
    #[serde(rename = "modelId", skip_serializing_if = "Option::is_none")]
    pub(crate) model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) harness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    pub(crate) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) connectivity: Option<String>,
    #[serde(rename = "terminalOutcome", skip_serializing_if = "Option::is_none")]
    pub(crate) terminal_outcome: Option<String>,
    pub(crate) cwd: String,
    pub(crate) created_at: String,
    #[serde(rename = "lastActivityAt", skip_serializing_if = "Option::is_none")]
    pub(crate) last_activity_at: Option<String>,
    #[serde(rename = "observedStatus")]
    pub(crate) observed_status: Option<String>,
    pub(crate) summary: Option<String>,
    #[serde(rename = "nextAction")]
    pub(crate) next_action: Option<String>,
    #[serde(rename = "needsUserInput")]
    pub(crate) needs_user_input: Option<bool>,
    #[serde(rename = "detectedQuestion")]
    pub(crate) detected_question: Option<String>,
    #[serde(rename = "suggestedOptions")]
    pub(crate) suggested_options: Option<Vec<String>>,
    #[serde(rename = "blockerDescription")]
    pub(crate) blocker_description: Option<String>,
    #[serde(rename = "failedCommand")]
    pub(crate) failed_command: Option<String>,
    #[serde(rename = "failedTest")]
    pub(crate) failed_test: Option<String>,
    #[serde(rename = "capacityHints")]
    pub(crate) capacity_hints: Option<Vec<String>>,
    #[serde(rename = "atUsageLimit", skip_serializing_if = "Option::is_none")]
    pub(crate) at_usage_limit: Option<bool>,
    #[serde(rename = "metadataSource")]
    pub(crate) metadata_source: Option<String>,
    #[serde(rename = "metadataConfidence")]
    pub(crate) metadata_confidence: Option<f64>,
    #[serde(rename = "repoRoot")]
    pub(crate) repo_root: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) dirty: Option<bool>,
    #[serde(rename = "changedFiles")]
    pub(crate) changed_files: Option<usize>,
    #[serde(rename = "isWorktree")]
    pub(crate) is_worktree: Option<bool>,
    #[serde(rename = "conflictWarning")]
    pub(crate) conflict_warning: Option<String>,
    pub(crate) recommendation: Option<String>,
    #[serde(rename = "peonLastInference")]
    pub(crate) peon_last_inference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) provider: Option<String>,
    #[serde(rename = "providerModel", skip_serializing_if = "Option::is_none")]
    pub(crate) provider_model: Option<String>,
    #[serde(rename = "providerState", skip_serializing_if = "Option::is_none")]
    pub(crate) provider_state: Option<String>,
    #[serde(rename = "memoryState")]
    pub(crate) memory_state: MemoryState,
    #[serde(rename = "resumeStrategy")]
    pub(crate) resume_strategy: harness::ResumeStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) resume: Option<harness::ResumeMemory>,
    #[serde(rename = "resumeOptions", skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) resume_options: Vec<metadata::ResumeOption>,
    #[serde(rename = "resumedFrom", skip_serializing_if = "Option::is_none")]
    pub(crate) resumed_from: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MemoryState {
    Live,
    Remembered,
    Resumable,
    Unsupported,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session_info(
        id: impl Into<String>,
        label: impl Into<String>,
        cwd: impl Into<String>,
        status: impl Into<String>,
        created_at: impl Into<String>,
    ) -> SessionInfo {
        let created_at = created_at.into();
        SessionInfo {
            id: id.into(),
            label: label.into(),
            harness_id: None,
            model_provider_id: None,
            model_id: None,
            harness: None,
            model: None,
            status: status.into(),
            connectivity: None,
            terminal_outcome: None,
            cwd: cwd.into(),
            created_at: created_at.clone(),
            last_activity_at: Some(created_at),
            observed_status: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            at_usage_limit: None,
            metadata_source: None,
            metadata_confidence: None,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            provider: None,
            provider_model: None,
            provider_state: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resume_options: vec![],
            resumed_from: None,
        }
    }

    #[test]
    fn session_info_serializes_provider_fields() {
        let info = SessionInfo {
            harness_id: Some("codex".into()),
            model_provider_id: Some("openrouter".into()),
            model_id: Some("gpt-5".into()),
            observed_status: Some("waiting_for_input".into()),
            summary: Some("Needs approval".into()),
            next_action: Some("Choose an option".into()),
            needs_user_input: Some(true),
            detected_question: Some("Proceed?".into()),
            suggested_options: Some(vec!["yes".into(), "no".into()]),
            metadata_source: Some("process".into()),
            metadata_confidence: Some(1.0),
            provider: Some("Claude Code".into()),
            provider_model: Some("sonnet".into()),
            provider_state: Some("healthy".into()),
            ..test_session_info("test", "Test", "/tmp", "running", "now")
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"harnessId\":\"codex\""));
        assert!(json.contains("\"modelProviderId\":\"openrouter\""));
        assert!(json.contains("\"modelId\":\"gpt-5\""));
        assert!(json.contains("\"provider\":\"Claude Code\""));
        assert!(json.contains("\"providerModel\":\"sonnet\""));
        assert!(json.contains("\"providerState\":\"healthy\""));
    }

    #[test]
    fn session_info_includes_metadata_fields() {
        let info = SessionInfo {
            observed_status: Some("waiting_for_input".into()),
            summary: Some("Needs approval".into()),
            next_action: Some("Choose an option".into()),
            needs_user_input: Some(true),
            detected_question: Some("Proceed?".into()),
            suggested_options: Some(vec!["yes".into(), "no".into()]),
            metadata_source: Some("process".into()),
            metadata_confidence: Some(1.0),
            ..test_session_info("test", "Test", "/tmp", "running", "now")
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"metadataSource\":\"process\""));
        assert!(json.contains("\"metadataConfidence\":1.0"));
        assert!(json.contains("\"observedStatus\":\"waiting_for_input\""));
        assert!(json.contains("\"needsUserInput\":true"));
    }

    #[test]
    fn session_info_without_metadata_is_valid() {
        let info = test_session_info("test", "Test", "/tmp", "creating", "now");
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"metadataSource\":null"));
        assert!(json.contains("\"metadataConfidence\":null"));
    }
}
