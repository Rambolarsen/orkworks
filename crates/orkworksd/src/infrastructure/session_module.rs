use std::sync::Arc;
use crate::application::session::handlers::*;
use crate::infrastructure::session_repository::MetadataSessionRepository;
use crate::infrastructure::session_pty::{RealPtySpawner, RealPtyKiller};
use crate::infrastructure::session_git::RealGitDetector;

pub struct SessionModule {
    pub create_session_handler: CreateSessionHandler,
    pub kill_session_handler: KillSessionHandler,
    pub resume_session_handler: ResumeSessionHandler,
    pub forget_session_handler: ForgetSessionHandler,
    pub list_handler: ListWorkspaceSessionsHandler,
    pub repository: Arc<MetadataSessionRepository>,
    pub pty_spawner: Arc<RealPtySpawner>,
    pub pty_killer: Arc<RealPtyKiller>,
    pub git_detector: Arc<RealGitDetector>,
}

impl SessionModule {
    pub fn new() -> Self {
        Self {
            create_session_handler: CreateSessionHandler,
            kill_session_handler: KillSessionHandler,
            resume_session_handler: ResumeSessionHandler,
            forget_session_handler: ForgetSessionHandler,
            list_handler: ListWorkspaceSessionsHandler,
            repository: Arc::new(MetadataSessionRepository::new()),
            pty_spawner: Arc::new(RealPtySpawner),
            pty_killer: Arc::new(RealPtyKiller),
            git_detector: Arc::new(RealGitDetector),
        }
    }
}
