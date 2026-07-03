use crate::git;
use crate::harness;
use crate::metadata;
use crate::session_types::{MemoryState, SessionInfo};
use std::collections::HashMap;

pub(crate) fn detect_conflicts(sessions: &[SessionInfo]) -> Vec<(String, String)> {
    let mut cwd_groups: HashMap<&str, Vec<&SessionInfo>> = HashMap::new();
    for s in sessions {
        if s.status == "running" || s.status == "creating" {
            cwd_groups.entry(&s.cwd).or_default().push(s);
        }
    }
    let mut warnings = Vec::new();
    for group in cwd_groups.values() {
        if group.len() >= 2 {
            if let Some(s) = group.first() {
                if s.dirty.unwrap_or(false) {
                    let warning = format!("{} sessions in this dirty workspace", group.len());
                    for session in group {
                        warnings.push((session.id.clone(), warning.clone()));
                    }
                }
            }
        }
    }
    warnings
}

pub(crate) fn session_recommendation(
    ctx: &git::GitContext,
    session_count_in_cwd: usize,
) -> Option<String> {
    if ctx.is_worktree {
        return Some("Running in a separate worktree. Good isolation.".into());
    }
    if session_count_in_cwd >= 2 && ctx.dirty {
        return Some("Multiple sessions in the same dirty workspace. Consider separate worktrees.".into());
    }
    if !ctx.is_worktree && ctx.dirty && ctx.branch.as_deref() != Some("main") {
        return Some("Working outside main in a dirty workspace. A worktree may be safer.".into());
    }
    None
}

pub(crate) fn connectivity_for_status(status: &str) -> &'static str {
    match status {
        "creating" | "running" => "online",
        _ => "offline",
    }
}

pub(crate) fn terminal_outcome_for_status(status: &str) -> Option<String> {
    match status {
        "ended" | "killed" | "error" => Some(status.to_string()),
        _ => None,
    }
}

pub(crate) fn merge_live_session_info(
    info: SessionInfo,
    meta: Option<&metadata::SessionMetadata>,
    peon_last_inference: Option<&String>,
    capabilities: &harness::HarnessCapabilities,
) -> SessionInfo {
    let is_live = info.status != "killed" && info.status != "ended" && info.status != "error";
    let (memory_state, resume_strategy) = derive_memory_state(
        is_live,
        meta.and_then(|m| m.resume.as_ref()).or(info.resume.as_ref()),
        capabilities,
    );
    let resume = meta.and_then(|m| m.resume.clone()).or(info.resume);

    SessionInfo {
        id: info.id,
        label: meta.map(|m| m.label.clone()).unwrap_or(info.label),
        harness_id: meta
            .and_then(|m| (!m.harness.is_empty()).then(|| m.harness.clone()))
            .or(info.harness_id),
        model_provider_id: meta.and_then(|m| m.provider_id.clone()).or(info.model_provider_id),
        model_id: meta
            .and_then(|m| (!m.model.is_empty()).then(|| m.model.clone()))
            .or(info.model_id),
        harness: meta
            .and_then(|m| (!m.harness.is_empty()).then(|| m.harness.clone()))
            .or(info.harness),
        model: meta
            .and_then(|m| (!m.model.is_empty()).then(|| m.model.clone()))
            .or(info.model),
        status: info.status.clone(),
        connectivity: Some(connectivity_for_status(&info.status).to_string()),
        terminal_outcome: terminal_outcome_for_status(&info.status),
        cwd: info.cwd,
        created_at: info.created_at.clone(),
        last_activity_at: meta
            .map(|m| m.last_activity.clone())
            .or(info.last_activity_at)
            .or_else(|| Some(info.created_at)),
        observed_status: meta.and_then(|m| m.observed_status.clone()).or(info.observed_status),
        summary: meta.and_then(|m| m.summary.clone()).or(info.summary),
        next_action: meta.and_then(|m| m.next_action.clone()).or(info.next_action),
        needs_user_input: meta.and_then(|m| m.needs_user_input).or(info.needs_user_input),
        detected_question: meta
            .and_then(|m| m.detected_question.clone())
            .or(info.detected_question),
        suggested_options: meta
            .and_then(|m| m.suggested_options.clone())
            .or(info.suggested_options),
        blocker_description: meta
            .and_then(|m| m.blocker_description.clone())
            .or(info.blocker_description),
        failed_command: meta.and_then(|m| m.failed_command.clone()).or(info.failed_command),
        failed_test: meta.and_then(|m| m.failed_test.clone()).or(info.failed_test),
        capacity_hints: meta.and_then(|m| m.capacity_hints.clone()).or(info.capacity_hints),
        at_usage_limit: None,
        capacity_check_pending: info.capacity_check_pending,
        usage_limit_reset_hint: None,
        metadata_source: meta.map(|m| m.metadata_source.clone()).or(info.metadata_source),
        metadata_confidence: meta.map(|m| m.metadata_confidence).or(info.metadata_confidence),
        peon_last_inference: meta
            .and_then(|m| m.peon_last_inference.clone())
            .or(info.peon_last_inference)
            .or_else(|| peon_last_inference.cloned()),
        repo_root: meta.and_then(|m| m.repo_root.clone()).or(info.repo_root),
        branch: meta.and_then(|m| m.branch.clone()).or(info.branch),
        dirty: meta.and_then(|m| m.dirty).or(info.dirty),
        changed_files: meta.and_then(|m| m.changed_files).or(info.changed_files),
        is_worktree: meta.and_then(|m| m.is_worktree).or(info.is_worktree),
        conflict_warning: info.conflict_warning,
        recommendation: info.recommendation,
        memory_state,
        resume_strategy: resume_strategy.clone(),
        resume: resume.clone(),
        resume_options: metadata::derive_resume_options(
            &resume_strategy,
            resume.as_ref(),
            capabilities.resume_exact,
            capabilities.resume_latest_in_cwd,
            capabilities.resume_latest_in_repo,
        ),
        resumed_from: meta.and_then(|m| m.resumed_from.clone()).or(info.resumed_from),
        provider: meta.and_then(|m| m.provider_label.clone()).or(info.provider),
        provider_model: meta.and_then(|m| m.provider_model.clone()).or(info.provider_model),
        provider_state: meta.and_then(|m| m.provider_state.clone()).or(info.provider_state),
    }
}

pub(crate) fn derive_memory_state(
    is_live: bool,
    resume: Option<&harness::ResumeMemory>,
    capabilities: &harness::HarnessCapabilities,
) -> (MemoryState, harness::ResumeStrategy) {
    if is_live {
        return (MemoryState::Live, harness::ResumeStrategy::None);
    }
    let Some(resume) = resume else {
        return (MemoryState::Remembered, harness::ResumeStrategy::None);
    };
    let strategy = harness::select_resume_strategy(resume, capabilities);
    if strategy == harness::ResumeStrategy::None {
        (MemoryState::Unsupported, strategy)
    } else {
        (MemoryState::Resumable, strategy)
    }
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
        let status = status.into();
        let created_at = created_at.into();
        SessionInfo {
            id: id.into(),
            label: label.into(),
            harness_id: None,
            model_provider_id: None,
            model_id: None,
            harness: None,
            model: None,
            status: status.clone(),
            connectivity: Some(connectivity_for_status(&status).to_string()),
            terminal_outcome: terminal_outcome_for_status(&status),
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
            capacity_check_pending: None,
            usage_limit_reset_hint: None,
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
    fn merge_live_session_info_uses_live_contract_fields_without_metadata() {
        let info = SessionInfo {
            connectivity: Some("offline".into()),
            terminal_outcome: Some("ended".into()),
            last_activity_at: Some("2026-06-28T09:05:00Z".into()),
            resume_options: vec![metadata::ResumeOption {
                strategy: harness::ResumeStrategy::Exact,
                label: "Resume exact session".into(),
                available: true,
                preferred: true,
                reason: None,
            }],
            ..test_session_info(
                "merge-live",
                "Merge Live",
                "/tmp/project",
                "ended",
                "2026-06-28T09:00:00Z",
            )
        };
        let caps = harness::HarnessCapabilities {
            launch: true,
            resume_exact: true,
            resume_latest_in_cwd: true,
            resume_latest_in_repo: true,
            detect_session_id: true,
            detect_model: true,
            detect_context_usage: true,
            detect_capacity: true,
            native_voice: false,
        };

        let merged = merge_live_session_info(info, None, None, &caps);

        assert_eq!(merged.connectivity.as_deref(), Some("offline"));
        assert_eq!(merged.terminal_outcome.as_deref(), Some("ended"));
        assert_eq!(merged.last_activity_at.as_deref(), Some("2026-06-28T09:05:00Z"));
        assert_eq!(merged.resume_options.len(), 3);
        assert!(!merged.resume_options[0].available);
        assert_eq!(
            merged.resume_options[0].reason.as_deref(),
            Some("No compatible remembered session exists"),
        );
        assert!(!merged.resume_options[1].available);
        assert!(!merged.resume_options[2].available);
    }

    #[test]
    fn connectivity_for_status_marks_running_sessions_online() {
        assert_eq!(connectivity_for_status("creating"), "online");
        assert_eq!(connectivity_for_status("running"), "online");
        assert_eq!(connectivity_for_status("ended"), "offline");
    }

    #[test]
    fn terminal_outcome_for_status_marks_ended_sessions_offline_with_terminal_outcome() {
        assert_eq!(terminal_outcome_for_status("running"), None);
        assert_eq!(terminal_outcome_for_status("ended").as_deref(), Some("ended"));
        assert_eq!(terminal_outcome_for_status("killed").as_deref(), Some("killed"));
    }

    #[test]
    fn merge_live_session_info_derives_resume_options_from_resume_memory_and_capabilities() {
        let info = test_session_info(
            "merge-derived",
            "Merge Derived",
            "/tmp/project",
            "ended",
            "2026-06-28T09:00:00Z",
        );
        let meta = metadata::SessionMetadata {
            id: "merge-derived".into(),
            label: "Merge Derived".into(),
            workspace: "/tmp/project".into(),
            task: "".into(),
            harness: "opencode".into(),
            model: "".into(),
            cwd: "/tmp/project".into(),
            status: "ended".into(),
            phase: "".into(),
            connectivity: "offline".into(),
            terminal_outcome: Some("ended".into()),
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
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "2026-06-28T09:00:00Z".into(),
            last_activity: "2026-06-28T09:05:00Z".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: Some("/tmp/project".into()),
            branch: Some("main".into()),
            dirty: Some(false),
            changed_files: Some(0),
            is_worktree: Some(false),
            resume: Some(harness::ResumeMemory {
                state: harness::ResumeState::Available,
                preferred_strategy: harness::ResumeStrategy::Exact,
                harness_session_id: None,
                latest_fallback: true,
                last_seen_at: Some("2026-06-28T09:05:00Z".into()),
            }),
            resume_options: vec![metadata::ResumeOption {
                strategy: harness::ResumeStrategy::Exact,
                label: "Resume exact session".into(),
                available: true,
                preferred: true,
                reason: None,
            }],
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
            resumed_from: None,
            last_user_input: None,
        };
        let caps = harness::HarnessCapabilities {
            launch: true,
            resume_exact: true,
            resume_latest_in_cwd: true,
            resume_latest_in_repo: false,
            detect_session_id: true,
            detect_model: true,
            detect_context_usage: true,
            detect_capacity: true,
            native_voice: false,
        };

        let merged = merge_live_session_info(info, Some(&meta), None, &caps);

        assert_eq!(merged.resume_options.len(), 3);
        assert_eq!(merged.resume_options[0].strategy, harness::ResumeStrategy::Exact);
        assert!(!merged.resume_options[0].available);
        assert_eq!(
            merged.resume_options[0].reason.as_deref(),
            Some("No harness session id was captured"),
        );
        assert_eq!(merged.resume_options[1].strategy, harness::ResumeStrategy::LatestCwd);
        assert!(merged.resume_options[1].available);
        assert!(merged.resume_options[1].preferred);
        assert_eq!(merged.resume_options[2].strategy, harness::ResumeStrategy::LatestRepo);
        assert!(!merged.resume_options[2].available);
    }

    #[test]
    fn detect_conflicts_warns_on_multiple_dirty_sessions() {
        let sessions = vec![
            SessionInfo {
                dirty: Some(true),
                ..test_session_info("a", "A", "/repo", "running", "now")
            },
            SessionInfo {
                dirty: Some(true),
                ..test_session_info("b", "B", "/repo", "running", "now")
            },
        ];
        let warnings = detect_conflicts(&sessions);
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|(id, _)| id == "a"));
        assert!(warnings.iter().any(|(id, _)| id == "b"));
        for (_, w) in &warnings {
            assert!(w.contains("2 sessions"));
        }
    }

    #[test]
    fn detect_conflicts_no_warning_on_clean_sessions() {
        let sessions = vec![
            SessionInfo {
                dirty: Some(false),
                ..test_session_info("a", "A", "/repo", "running", "now")
            },
            SessionInfo {
                dirty: Some(false),
                ..test_session_info("b", "B", "/repo", "running", "now")
            },
        ];
        let warnings = detect_conflicts(&sessions);
        assert!(warnings.is_empty());
    }

    #[test]
    fn detect_conflicts_no_warning_when_dirty_is_none() {
        let sessions = vec![
            SessionInfo {
                dirty: None,
                ..test_session_info("a", "A", "/repo", "running", "now")
            },
            SessionInfo {
                dirty: None,
                ..test_session_info("b", "B", "/repo", "running", "now")
            },
        ];
        let warnings = detect_conflicts(&sessions);
        assert!(warnings.is_empty());
    }

    #[test]
    fn memory_state_marks_absent_session_as_resumable_when_strategy_exists() {
        let caps = harness::HarnessCapabilities {
            launch: true,
            resume_exact: true,
            resume_latest_in_cwd: true,
            resume_latest_in_repo: false,
            detect_session_id: true,
            detect_model: true,
            detect_context_usage: false,
            detect_capacity: false,
            native_voice: false,
        };
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::Exact,
            harness_session_id: Some("sess-1".into()),
            latest_fallback: true,
            last_seen_at: None,
        };

        let (memory_state, strategy) = derive_memory_state(false, Some(&resume), &caps);

        assert_eq!(memory_state, MemoryState::Resumable);
        assert_eq!(strategy, harness::ResumeStrategy::Exact);
    }

    #[test]
    fn memory_state_marks_active_session_as_live() {
        let caps = harness::HarnessCapabilities {
            launch: true,
            resume_exact: false,
            resume_latest_in_cwd: false,
            resume_latest_in_repo: false,
            detect_session_id: false,
            detect_model: false,
            detect_context_usage: false,
            detect_capacity: false,
            native_voice: false,
        };

        let (memory_state, strategy) = derive_memory_state(true, None, &caps);

        assert_eq!(memory_state, MemoryState::Live);
        assert_eq!(strategy, harness::ResumeStrategy::None);
    }
}
