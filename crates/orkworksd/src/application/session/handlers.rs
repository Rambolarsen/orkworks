use crate::domain::session::{
    entity::Session,
    events::DomainEvent,
    repository::SessionRepository,
    services::SessionLifecycle,
    value_objects::*,
};
use super::commands::*;

pub struct CreateSessionHandler;

impl CreateSessionHandler {
    pub fn handle(
        cmd: &CreateSessionCommand,
        id: &SessionId,
        label: &str,
        workspace_path: &WorkspacePath,
        created_at: &str,
        provider_id: Option<String>,
    ) -> (Session, Vec<DomainEvent>) {
        let resume_memory = crate::harness::ResumeMemory {
            state: crate::harness::ResumeState::Available,
            preferred_strategy: crate::harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some(created_at.to_string()),
        };

        SessionLifecycle::create(
            id.clone(),
            workspace_path.clone(),
            label.to_string(),
            cmd.cwd.clone(),
            cmd.harness_name.clone(),
            provider_id,
            cmd.model.clone(),
            created_at.to_string(),
            None,
            Some(resume_memory),
        )
    }
}

pub struct KillSessionHandler;

impl KillSessionHandler {
    pub fn handle(
        repo: &dyn SessionRepository,
        cmd: &KillSessionCommand,
        killed_at: &str,
    ) -> Result<(Session, Vec<DomainEvent>), String> {
        let mut session = repo.load(&cmd.session_id)?
            .ok_or_else(|| format!("session {} not found", cmd.session_id))?;

        let events = SessionLifecycle::kill(&mut session, killed_at.to_string());
        if events.is_empty() {
            return Err("session already killed".into());
        }

        repo.save(&session, events.clone())?;
        Ok((session, events))
    }
}

pub struct ResumeSessionHandler;

impl ResumeSessionHandler {
    pub fn handle(
        repo: &dyn SessionRepository,
        cmd: &ResumeSessionCommand,
        resumed_at: &str,
    ) -> Result<(Session, Vec<DomainEvent>), String> {
        let session = repo.load(&cmd.session_id)?
            .ok_or_else(|| format!("session {} not found", cmd.session_id))?;

        let events = SessionLifecycle::resume(&session, resumed_at.to_string());
        Ok((session, events))
    }
}

pub struct ForgetSessionHandler;

impl ForgetSessionHandler {
    pub fn handle(
        repo: &dyn SessionRepository,
        cmd: &ForgetSessionCommand,
    ) -> Result<(), String> {
        repo.delete(&cmd.session_id)
    }
}

pub struct ListWorkspaceSessionsHandler;

impl ListWorkspaceSessionsHandler {
    pub fn handle(
        repo: &dyn SessionRepository,
        cmd: &ListWorkspaceSessionsCommand,
    ) -> Result<Vec<Session>, String> {
        repo.list_by_workspace(&WorkspacePath(cmd.workspace_path.clone()))
    }
}
