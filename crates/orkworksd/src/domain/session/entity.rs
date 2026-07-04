use super::value_objects::*;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub workspace_path: WorkspacePath,
    pub status: SessionStatus,
    pub memory_state: MemoryState,
    pub attention_state: AttentionState,
    pub work_phase: WorkPhase,
    pub lifecycle_phase: LifecyclePhase,
    pub pending_terminal_status: Option<TerminalOutcome>,
    pub ending_observed_status_snapshot: Option<ObservedStatusSnapshot>,
    pub final_observed_status_snapshot: Option<ObservedStatusSnapshot>,
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
        self.lifecycle_phase = LifecyclePhase::Ended;
        self.pending_terminal_status = None;
        self.ending_observed_status_snapshot = None;
        self.killed_at = Some(now.into());
        self.memory_state = MemoryState::Remembered;
    }

    pub fn mark_running(&mut self) {
        self.status = SessionStatus::Running;
        self.lifecycle_phase = LifecyclePhase::Active;
    }

    pub fn mark_active(&mut self) -> Result<(), SessionTransitionError> {
        if self.lifecycle_phase != LifecyclePhase::Creating {
            return Err(SessionTransitionError::InvalidPhase);
        }
        self.mark_running();
        Ok(())
    }

    pub fn begin_ending(
        &mut self,
        pending_terminal_status: TerminalOutcome,
        ending_observed_status_snapshot: ObservedStatusSnapshot,
    ) -> Result<(), SessionTransitionError> {
        if self.lifecycle_phase != LifecyclePhase::Active {
            return Err(SessionTransitionError::InvalidPhase);
        }
        self.lifecycle_phase = LifecyclePhase::Ending;
        self.status = SessionStatus::Running;
        self.pending_terminal_status = Some(pending_terminal_status);
        self.ending_observed_status_snapshot = Some(ending_observed_status_snapshot);
        Ok(())
    }

    pub fn complete_ending(
        &mut self,
        final_observed_status_snapshot: ObservedStatusSnapshot,
    ) -> Result<(), SessionTransitionError> {
        if self.lifecycle_phase == LifecyclePhase::Ended {
            return Ok(());
        }
        if self.lifecycle_phase != LifecyclePhase::Ending {
            return Err(SessionTransitionError::InvalidPhase);
        }
        let final_status = self
            .pending_terminal_status
            .clone()
            .ok_or(SessionTransitionError::MissingPendingOutcome)?;
        self.lifecycle_phase = LifecyclePhase::Ended;
        self.status = SessionStatus::from(final_status);
        self.final_observed_status_snapshot = Some(final_observed_status_snapshot);
        self.pending_terminal_status = None;
        self.ending_observed_status_snapshot = None;
        Ok(())
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
            work_phase: WorkPhase::Unknown,
            lifecycle_phase: LifecyclePhase::Creating,
            pending_terminal_status: None,
            ending_observed_status_snapshot: None,
            final_observed_status_snapshot: None,
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

    #[test]
    fn begin_ending_sets_lifecycle_and_captures_snapshot() {
        let mut s = make_test_session();
        s.mark_active().unwrap();
        s.begin_ending(
            TerminalOutcome::Ended,
            ObservedStatusSnapshot {
                value: Some(AttentionState::Blocked),
                source: "peon".into(),
                confidence: Some(0.82),
                observed_at: Some("2026-07-03T12:34:56Z".into()),
            },
        ).unwrap();
        assert_eq!(s.lifecycle_phase, LifecyclePhase::Ending);
        assert_eq!(s.status, SessionStatus::Running);
        assert_eq!(s.pending_terminal_status, Some(TerminalOutcome::Ended));
        assert_eq!(
            s.ending_observed_status_snapshot.as_ref().and_then(|x| x.value.as_ref()),
            Some(&AttentionState::Blocked)
        );
    }

    #[test]
    fn complete_ending_is_first_winner_and_sets_final_snapshot() {
        let mut s = make_test_session();
        s.mark_active().unwrap();
        s.begin_ending(
            TerminalOutcome::Killed,
            ObservedStatusSnapshot {
                value: None,
                source: "recovery".into(),
                confidence: None,
                observed_at: None,
            },
        ).unwrap();
        s.complete_ending(ObservedStatusSnapshot {
            value: Some(AttentionState::Done),
            source: "peon".into(),
            confidence: Some(0.91),
            observed_at: Some("2026-07-03T12:40:00Z".into()),
        }).unwrap();
        s.complete_ending(ObservedStatusSnapshot {
            value: Some(AttentionState::Failed),
            source: "peon".into(),
            confidence: Some(0.12),
            observed_at: Some("2026-07-03T12:41:00Z".into()),
        }).unwrap();
        assert_eq!(s.lifecycle_phase, LifecyclePhase::Ended);
        assert_eq!(s.status, SessionStatus::Killed);
        assert_eq!(
            s.final_observed_status_snapshot.as_ref().and_then(|x| x.value.as_ref()),
            Some(&AttentionState::Done)
        );
    }

    #[test]
    fn invalid_transition_shortcuts_are_rejected_and_completion_clears_pending_state() {
        let mut s = make_test_session();
        let snap = ObservedStatusSnapshot {
            value: None,
            source: "recovery".into(),
            confidence: None,
            observed_at: None,
        };
        assert!(s.begin_ending(TerminalOutcome::Ended, snap.clone()).is_err());
        assert!(s.complete_ending(snap.clone()).is_err());
        s.mark_active().unwrap();
        s.begin_ending(TerminalOutcome::Error, snap.clone()).unwrap();
        s.complete_ending(snap).unwrap();
        assert_eq!(s.pending_terminal_status, None);
        assert_eq!(s.ending_observed_status_snapshot, None);
        assert!(s.mark_active().is_err());
    }
}
