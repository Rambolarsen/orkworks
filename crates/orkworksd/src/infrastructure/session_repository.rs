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
            let meta = session_to_metadata(session);
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

fn session_to_metadata(session: &Session) -> SessionMetadata {
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
        provider_id: session.provider_id.clone(),
        provider_label: None,
        provider_model: None,
        provider_state: None,
        created_at: session.created_at.clone(),
        last_activity: chrono::Utc::now().to_rfc3339(),
        metadata_source: "process".into(),
        metadata_confidence: 1.0,
        repo_root: session.repo_root.clone(),
        branch: session.branch.clone(),
        dirty: session.dirty,
        changed_files: session.changed_files,
        is_worktree: session.is_worktree,
        resume: session.resume.clone(),
        resumed_from: session.resumed_from.clone(),
    }
}
