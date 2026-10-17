//! Read-only settings guard for consumers when inference transport is unavailable.
use super::{TaskmasterRuntime, TaskmasterSettings};
use std::path::Path;
impl TaskmasterRuntime {
    pub(crate) fn with_workspace_settings<T>(
        &self,
        workspace: &Path,
        apply: impl FnOnce(&TaskmasterSettings) -> T,
    ) -> Result<T, String> {
        let key =
            super::canonical_workspace_key(workspace).ok_or("Workspace identity is unavailable")?;
        let _guard = super::PersistenceGuard::acquire(&self.root)?;
        let mut data = self
            .data
            .lock()
            .map_err(|_| "Taskmaster policy is unavailable")?;
        super::reload_durable(&self.root, &mut data);
        if !data.ledger_readable {
            return Err("Taskmaster policy ledger is unavailable".into());
        }
        Ok(apply(&super::effective_settings(&data.settings, &key)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consumer_settings_guard_reloads_durable_policy_and_fails_closed_on_corrupt_ledger() {
        let directory = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(directory.path().into());
        let mut settings = TaskmasterSettings::default();
        settings.context_level = super::super::ContextLevel::SessionObservations;
        TaskmasterRuntime::open(directory.path().into())
            .replace_settings(settings.clone())
            .unwrap();
        assert_eq!(
            runtime
                .with_workspace_settings(workspace.path(), |current| current.context_level)
                .unwrap(),
            settings.context_level
        );
        std::fs::write(directory.path().join("evaluations.json"), "broken").unwrap();
        assert!(runtime
            .with_workspace_settings(workspace.path(), |_| panic!(
                "unreadable policy must not expose assessment data"
            ))
            .is_err());
    }
}
