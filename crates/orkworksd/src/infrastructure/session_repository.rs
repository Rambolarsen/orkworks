use std::sync::{Arc, Mutex};
use crate::domain::session::{
    entity::Session,
    events::DomainEvent,
    repository::SessionRepository,
    value_objects::*,
};
use crate::metadata::{MetadataStore, SessionMetadata};

pub struct MetadataSessionRepository {
    store: Arc<Mutex<Option<MetadataStore>>>,
}

impl MetadataSessionRepository {
    pub fn new() -> Self {
        Self { store: Arc::new(Mutex::new(None)) }
    }

    pub fn set_store(&self, store: MetadataStore) {
        *self.store.lock().unwrap() = Some(store);
    }

    fn with_store<F, R>(&self, f: F) -> Result<R, String>
    where F: FnOnce(&MetadataStore) -> Result<R, String>
    {
        let guard = self.store.lock().unwrap();
        match guard.as_ref() {
            Some(store) => f(store),
            None => Err("no workspace set".into()),
        }
    }
}

impl SessionRepository for MetadataSessionRepository {
    fn save(&self, session: &Session, events: Vec<DomainEvent>) -> Result<(), String> {
        self.with_store(|store| {
            let existing = store.read_session(&session.id.0);
            let meta = session_to_metadata(session, existing.as_ref());
            store.write_session(&meta);

            let now = chrono::Utc::now().to_rfc3339();
            for event in &events {
                store.append_event(&session.id.0, &crate::metadata::Event {
                    event_type: event.event_type().to_string(),
                    timestamp: now.clone(),
                    status: session_status_str(&session.status).to_string(),
                    observed_status: None,
                    confidence: None,
                });
            }
            Ok(())
        })
    }

    fn load(&self, id: &SessionId) -> Result<Option<Session>, String> {
        self.with_store(|store| {
            Ok(store.read_session(&id.0).map(|m| meta_to_session(&m)))
        })
    }

    fn delete(&self, id: &SessionId) -> Result<(), String> {
        self.with_store(|store| {
            store.delete_session(&id.0)
                .map_err(|e| format!("delete failed: {e}"))
        })
    }

    fn list_by_workspace(&self, _path: &WorkspacePath) -> Result<Vec<Session>, String> {
        self.with_store(|store| {
            Ok(store.read_all_sessions().into_iter()
                .map(|m| meta_to_session(&m))
                .collect())
        })
    }

    fn append_terminal_output(&self, id: &SessionId, lines: Vec<String>) -> Result<(), String> {
        self.with_store(|store| {
            store.append_terminal_output_lines(&id.0, &lines);
            Ok(())
        })
    }
}

fn session_status_str(s: &SessionStatus) -> &'static str {
    match s {
        SessionStatus::Creating => "creating",
        SessionStatus::Running => "running",
        SessionStatus::Killed => "killed",
        SessionStatus::Ended => "ended",
        SessionStatus::Error => "error",
    }
}

fn phase_str(p: &Phase) -> &'static str {
    match p {
        Phase::Ideation => "ideation",
        Phase::Implementation => "implementation",
        Phase::Review => "review",
        Phase::Debugging => "debugging",
        Phase::Unknown => "",
    }
}

fn meta_to_session(meta: &SessionMetadata) -> Session {
    let status = match meta.status.as_str() {
        "creating" => SessionStatus::Creating,
        "running" => SessionStatus::Running,
        "killed" => SessionStatus::Killed,
        "ended" => SessionStatus::Ended,
        "error" => SessionStatus::Error,
        _ => SessionStatus::Ended,
    };
    let attention = AttentionState::from_str(
        meta.observed_status.as_deref().unwrap_or(&meta.status),
    );
    let phase = match meta.phase.as_str() {
        "ideation" => Phase::Ideation,
        "implementation" => Phase::Implementation,
        "review" => Phase::Review,
        "debugging" => Phase::Debugging,
        _ => Phase::Unknown,
    };

    Session {
        id: SessionId(meta.id.clone()),
        workspace_path: WorkspacePath(std::path::PathBuf::from(&meta.workspace)),
        status,
        memory_state: MemoryState::Remembered,
        attention_state: attention,
        phase,
        created_at: meta.created_at.clone(),
        killed_at: None,
        last_active_at: Some(meta.last_activity.clone()),
        harness_name: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
        provider_id: meta.provider_id.clone(),
        task_description: (!meta.task.is_empty()).then(|| meta.task.clone()),
        label: meta.label.clone(),
        cwd: meta.cwd.clone(),
        model: (!meta.model.is_empty()).then(|| meta.model.clone()),
        repo_root: meta.repo_root.clone(),
        branch: meta.branch.clone(),
        dirty: meta.dirty,
        changed_files: meta.changed_files,
        is_worktree: meta.is_worktree,
        resume: meta.resume.clone(),
        resumed_from: meta.resumed_from.clone(),
        resume_strategy: crate::harness::ResumeStrategy::None,
    }
}

fn session_to_metadata(session: &Session, existing: Option<&SessionMetadata>) -> SessionMetadata {
    SessionMetadata {
        id: session.id.0.clone(),
        label: session.label.clone(),
        workspace: session.workspace_path.0.display().to_string(),
        task: session.task_description.clone().unwrap_or_default(),
        harness: session.harness_name.clone().unwrap_or_default(),
        model: session.model.clone().unwrap_or_default(),
        cwd: session.cwd.clone(),
        status: session_status_str(&session.status).to_string(),
        phase: phase_str(&session.phase).to_string(),
        connectivity: if matches!(
            session.status,
            SessionStatus::Killed | SessionStatus::Ended | SessionStatus::Error
        ) {
            "offline".into()
        } else {
            "online".into()
        },
        terminal_outcome: match session.status {
            SessionStatus::Killed => Some("killed".into()),
            SessionStatus::Ended => Some("ended".into()),
            SessionStatus::Error => Some("error".into()),
            _ => None,
        },
        observed_status: existing.and_then(|meta| meta.observed_status.clone()),
        summary: existing.and_then(|meta| meta.summary.clone()),
        next_action: existing.and_then(|meta| meta.next_action.clone()),
        needs_user_input: existing.and_then(|meta| meta.needs_user_input),
        detected_question: existing.and_then(|meta| meta.detected_question.clone()),
        suggested_options: existing.and_then(|meta| meta.suggested_options.clone()),
        blocker_description: existing.and_then(|meta| meta.blocker_description.clone()),
        failed_command: existing.and_then(|meta| meta.failed_command.clone()),
        failed_test: existing.and_then(|meta| meta.failed_test.clone()),
        capacity_hints: existing.and_then(|meta| meta.capacity_hints.clone()),
        peon_last_inference: existing.and_then(|meta| meta.peon_last_inference.clone()),
        provider_id: session.provider_id.clone(),
        provider_label: existing.and_then(|meta| meta.provider_label.clone()),
        provider_model: existing.and_then(|meta| meta.provider_model.clone()),
        provider_state: existing.and_then(|meta| meta.provider_state.clone()),
        created_at: session.created_at.clone(),
        last_activity: session
            .last_active_at
            .clone()
            .or_else(|| existing.map(|meta| meta.last_activity.clone()))
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        metadata_source: existing
            .map(|meta| meta.metadata_source.clone())
            .unwrap_or_else(|| "process".into()),
        metadata_confidence: existing
            .map(|meta| meta.metadata_confidence)
            .unwrap_or(1.0),
        repo_root: session.repo_root.clone(),
        branch: session.branch.clone(),
        dirty: session.dirty,
        changed_files: session.changed_files,
        is_worktree: session.is_worktree,
        resume: session.resume.clone(),
        resume_options: existing
            .map(|meta| meta.resume_options.clone())
            .unwrap_or_default(),
        harness_session_id_source: existing.and_then(|meta| meta.harness_session_id_source.clone()),
        harness_session_id_confidence: existing.and_then(|meta| meta.harness_session_id_confidence),
        harness_session_id_captured_at: existing.and_then(|meta| meta.harness_session_id_captured_at.clone()),
        resumed_from: session.resumed_from.clone(),
        last_user_input: existing.and_then(|meta| meta.last_user_input.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::ResumeOption;
    use std::path::PathBuf;

    fn make_session() -> Session {
        Session {
            id: SessionId("roundtrip".into()),
            workspace_path: WorkspacePath(PathBuf::from("/tmp")),
            status: SessionStatus::Ended,
            memory_state: MemoryState::Remembered,
            attention_state: AttentionState::Done,
            phase: Phase::Review,
            created_at: "2026-06-28T09:00:00Z".into(),
            killed_at: None,
            last_active_at: Some("2026-06-28T09:05:00Z".into()),
            harness_name: Some("opencode".into()),
            provider_id: Some("openrouter".into()),
            task_description: Some("Round-trip".into()),
            label: "Round Trip".into(),
            cwd: "/tmp".into(),
            model: Some("gpt-5".into()),
            repo_root: Some("/tmp".into()),
            branch: Some("main".into()),
            dirty: Some(false),
            changed_files: Some(0),
            is_worktree: Some(false),
            resume: None,
            resumed_from: Some("older".into()),
            resume_strategy: crate::harness::ResumeStrategy::Exact,
        }
    }

    #[test]
    fn save_preserves_resume_options_and_last_activity_from_existing_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let repo = MetadataSessionRepository::new();
        repo.set_store(MetadataStore::new(dir.path()));

        store.write_session(&SessionMetadata {
            id: "roundtrip".into(),
            label: "Round Trip".into(),
            workspace: "/tmp".into(),
            task: "Round-trip".into(),
            harness: "opencode".into(),
            model: "gpt-5".into(),
            cwd: "/tmp".into(),
            status: "ended".into(),
            phase: "review".into(),
            connectivity: "offline".into(),
            terminal_outcome: Some("ended".into()),
            observed_status: Some("done".into()),
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
            provider_id: Some("openrouter".into()),
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "2026-06-28T09:00:00Z".into(),
            last_activity: "2026-06-28T09:05:00Z".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: Some("/tmp".into()),
            branch: Some("main".into()),
            dirty: Some(false),
            changed_files: Some(0),
            is_worktree: Some(false),
            resume: None,
            resume_options: vec![ResumeOption {
                strategy: crate::harness::ResumeStrategy::Exact,
                label: "Resume exact session".into(),
                available: true,
                preferred: true,
                reason: None,
            }],
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
            resumed_from: Some("older".into()),
            last_user_input: None,
        });

        let loaded = repo.load(&SessionId("roundtrip".into())).unwrap().unwrap();
        repo.save(&loaded, vec![]).unwrap();

        let persisted = store.read_session("roundtrip").unwrap();
        assert_eq!(persisted.last_activity, "2026-06-28T09:05:00Z");
        assert_eq!(persisted.resume_options.len(), 1);
        assert_eq!(persisted.resume_options[0].label, "Resume exact session");
        assert_eq!(persisted.connectivity, "offline");
        assert_eq!(persisted.terminal_outcome.as_deref(), Some("ended"));
    }
}
