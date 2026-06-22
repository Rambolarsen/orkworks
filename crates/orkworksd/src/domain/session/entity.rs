use super::value_objects::*;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub workspace_path: WorkspacePath,
    pub status: SessionStatus,
    pub memory_state: MemoryState,
    pub attention_state: AttentionState,
    pub phase: Phase,
    pub created_at: String,
    pub killed_at: Option<String>,
    pub last_active_at: Option<String>,
    pub harness_name: Option<String>,
    pub provider_id: Option<String>,
    pub task_description: Option<String>,
    pub label: String,
    pub cwd: String,
    pub model: Option<String>,
    pub repo_root: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
    pub changed_files: Option<usize>,
    pub is_worktree: Option<bool>,
    pub resume: Option<crate::harness::ResumeMemory>,
    pub resumed_from: Option<String>,
    pub resume_strategy: crate::harness::ResumeStrategy,
}

impl Session {
    pub fn is_live(&self) -> bool {
        self.memory_state == MemoryState::Live
    }

    pub fn is_killed(&self) -> bool {
        self.status == SessionStatus::Killed
    }

    pub fn can_be_killed(&self) -> bool {
        matches!(self.status, SessionStatus::Creating | SessionStatus::Running)
    }

    pub fn kill(&mut self, now: &str) {
        self.status = SessionStatus::Killed;
        self.killed_at = Some(now.into());
        self.memory_state = MemoryState::Remembered;
    }

    pub fn mark_running(&mut self) {
        self.status = SessionStatus::Running;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_session() -> Session {
        Session {
            id: SessionId("s1".into()),
            workspace_path: WorkspacePath(PathBuf::from("/tmp")),
            status: SessionStatus::Creating,
            memory_state: MemoryState::Live,
            attention_state: AttentionState::Idle,
            phase: Phase::Unknown,
            created_at: "2026-01-01T00:00:00Z".into(),
            killed_at: None,
            last_active_at: None,
            harness_name: None,
            provider_id: None,
            task_description: None,
            label: "Test".into(),
            cwd: "/tmp".into(),
            model: None,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            resume: None,
            resumed_from: None,
            resume_strategy: crate::harness::ResumeStrategy::None,
        }
    }

    #[test]
    fn fresh_session_is_live() {
        let s = make_test_session();
        assert!(s.is_live());
        assert!(s.can_be_killed());
        assert!(!s.is_killed());
    }

    #[test]
    fn kill_transitions_status_and_memory() {
        let mut s = make_test_session();
        s.kill("2026-01-01T01:00:00Z");
        assert_eq!(s.status, SessionStatus::Killed);
        assert_eq!(s.memory_state, MemoryState::Remembered);
        assert!(!s.is_live());
        assert!(!s.can_be_killed());
    }

    #[test]
    fn already_killed_cannot_be_killed() {
        let mut s = make_test_session();
        s.status = SessionStatus::Killed;
        assert!(!s.can_be_killed());
    }

    #[test]
    fn mark_running_sets_status() {
        let mut s = make_test_session();
        s.mark_running();
        assert_eq!(s.status, SessionStatus::Running);
    }
}
