use super::{entity::Session, events::DomainEvent, value_objects::*};

pub struct SessionLifecycle;

impl SessionLifecycle {
    pub fn create(
        id: SessionId,
        workspace_path: WorkspacePath,
        label: String,
        cwd: String,
        harness_name: Option<String>,
        provider_id: Option<String>,
        model: Option<String>,
        created_at: String,
        git_context: Option<(Option<String>, Option<String>, bool, usize, bool)>,
        resume: Option<crate::harness::ResumeMemory>,
    ) -> (Session, Vec<DomainEvent>) {
        let (repo_root, branch, dirty, changed_files, is_worktree) = git_context
            .map(|(r, b, d, c, w)| (r, b, Some(d), Some(c), Some(w)))
            .unwrap_or((None, None, None, None, None));

        let session = Session {
            id: id.clone(),
            workspace_path,
            status: SessionStatus::Creating,
            memory_state: MemoryState::Live,
            attention_state: AttentionState::Idle,
            phase: Phase::Unknown,
            created_at: created_at.clone(),
            killed_at: None,
            last_active_at: None,
            harness_name: harness_name.clone(),
            provider_id: provider_id.clone(),
            task_description: None,
            label,
            cwd,
            model,
            repo_root,
            branch,
            dirty,
            changed_files,
            is_worktree,
            resume,
            resumed_from: None,
            resume_strategy: crate::harness::ResumeStrategy::None,
        };

        let event = DomainEvent::SessionCreated {
            session_id: id.0.clone(),
            created_at,
            harness_name,
            workspace_path: session.workspace_path.0.display().to_string(),
        };

        (session, vec![event])
    }

    pub fn kill(session: &mut Session, killed_at: String) -> Vec<DomainEvent> {
        if !session.can_be_killed() {
            return vec![];
        }
        session.kill(&killed_at);
        vec![DomainEvent::SessionKilled {
            session_id: session.id.0.clone(),
            killed_at,
        }]
    }

    pub fn resume(session: &Session, resumed_at: String) -> Vec<DomainEvent> {
        vec![DomainEvent::SessionResumed {
            session_id: session.id.0.clone(),
            resumed_at,
            previous_session_id: session.resumed_from.clone(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_resume() -> crate::harness::ResumeMemory {
        crate::harness::ResumeMemory {
            state: crate::harness::ResumeState::Available,
            preferred_strategy: crate::harness::ResumeStrategy::LatestCwd,
            harness_session_id: Some("hs1".into()),
            latest_fallback: true,
            last_seen_at: Some("now".into()),
        }
    }

    #[test]
    fn create_produces_live_session_with_created_event() {
        let (session, events) = lifecycle.create(
            SessionId("s1".into()),
            WorkspacePath(PathBuf::from("/ws")),
            "Test".into(),
            "/ws".into(),
            Some("claude-code".into()),
            None,
            None,
            "2026-01-01T00:00:00Z".into(),
            Some((Some("/ws".into()), Some("main".into()), true, 3, false)),
            Some(make_test_resume()),
        );
        assert_eq!(session.status, SessionStatus::Creating);
        assert!(session.is_live());
        assert_eq!(session.dirty, Some(true));
        assert_eq!(session.changed_files, Some(3));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "session.created");
    }

    #[test]
    fn kill_produces_killed_event() {
        let (mut session, _) = lifecycle.create(
            SessionId("s2".into()),
            WorkspacePath(PathBuf::from("/ws")),
            "Test".into(),
            "/ws".into(),
            None, None, None, "now".into(),
            None, None,
        );
        let events = lifecycle.kill(&mut session, "later".into());
        assert_eq!(session.status, SessionStatus::Killed);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "session.killed");
    }

    #[test]
    fn kill_already_killed_produces_no_events() {
        let (mut session, _) = lifecycle.create(
            SessionId("s3".into()),
            WorkspacePath(PathBuf::from("/ws")),
            "Test".into(),
            "/ws".into(),
            None, None, None, "now".into(),
            None, None,
        );
        session.status = SessionStatus::Killed;
        let events = lifecycle.kill(&mut session, "later".into());
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn resume_produces_resumed_event() {
        let (session, _) = lifecycle.create(
            SessionId("s4".into()),
            WorkspacePath(PathBuf::from("/ws")),
            "Test".into(),
            "/ws".into(),
            None, None, None, "now".into(),
            None, None,
        );
        let events = lifecycle.resume(&session, "later".into());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "session.resumed");
    }
}
