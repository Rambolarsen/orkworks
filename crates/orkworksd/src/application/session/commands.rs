use std::path::PathBuf;
use crate::domain::session::value_objects::SessionId;

pub struct CreateSessionCommand {
    pub harness_name: Option<String>,
    pub model: Option<String>,
    pub initial_prompt: Option<String>,
    pub cwd: String,
}

pub struct KillSessionCommand {
    pub session_id: SessionId,
}

pub struct ResumeSessionCommand {
    pub session_id: SessionId,
}

pub struct ForgetSessionCommand {
    pub session_id: SessionId,
}

pub struct ListWorkspaceSessionsCommand {
    pub workspace_path: PathBuf,
}
