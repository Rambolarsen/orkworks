use std::path::PathBuf;
use crate::domain::session::value_objects::SessionId;

pub struct PtyHandle {
    pub child_pty: Box<dyn std::any::Any + Send>,
    pub writer: Box<dyn std::io::Write + Send>,
}

pub trait PtySpawner: Send + Sync {
    fn spawn(&self, id: &SessionId, cwd: &PathBuf, command: &crate::harness::CommandSpec) -> Result<PtyHandle, String>;
}

pub trait PtyKiller: Send + Sync {
    fn kill(&self, handle: PtyHandle) -> Result<(), String>;
}

pub trait GitDetector: Send + Sync {
    fn detect(&self, path: &PathBuf) -> Option<crate::git::GitContext>;
}
