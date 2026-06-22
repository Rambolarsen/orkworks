use std::path::PathBuf;
use crate::application::session::ports::{PtyHandle, PtySpawner, PtyKiller};
use crate::domain::session::value_objects::SessionId;
use crate::harness::CommandSpec;

pub struct RealPtySpawner;

impl PtySpawner for RealPtySpawner {
    fn spawn(&self, _id: &SessionId, _cwd: &PathBuf, _command: &CommandSpec) -> Result<PtyHandle, String> {
        Err("PTY spawner not yet wired - called from thin handler".into())
    }
}

pub struct RealPtyKiller;

impl PtyKiller for RealPtyKiller {
    fn kill(&self, _handle: PtyHandle) -> Result<(), String> {
        Ok(())
    }
}
