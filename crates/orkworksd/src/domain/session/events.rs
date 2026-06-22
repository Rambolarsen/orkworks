use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "event_type")]
pub enum DomainEvent {
    SessionCreated {
        session_id: String,
        created_at: String,
        harness_name: Option<String>,
        workspace_path: String,
    },
    SessionKilled {
        session_id: String,
        killed_at: String,
    },
    SessionResumed {
        session_id: String,
        resumed_at: String,
        previous_session_id: Option<String>,
    },
    SessionAttentionChanged {
        session_id: String,
        old_state: Option<String>,
        new_state: String,
    },
    SessionForgotten {
        session_id: String,
        deleted_at: String,
    },
}

impl DomainEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::SessionCreated { .. } => "session.created",
            Self::SessionKilled { .. } => "session.killed",
            Self::SessionResumed { .. } => "session.resumed",
            Self::SessionAttentionChanged { .. } => "session.attention_changed",
            Self::SessionForgotten { .. } => "session.forgotten",
        }
    }

    pub fn session_id(&self) -> &str {
        match self {
            Self::SessionCreated { session_id, .. } => session_id,
            Self::SessionKilled { session_id, .. } => session_id,
            Self::SessionResumed { session_id, .. } => session_id,
            Self::SessionAttentionChanged { session_id, .. } => session_id,
            Self::SessionForgotten { session_id, .. } => session_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_names_match_existing_convention() {
        let e = DomainEvent::SessionCreated {
            session_id: "s1".into(),
            created_at: "now".into(),
            harness_name: None,
            workspace_path: "/ws".into(),
        };
        assert_eq!(e.event_type(), "session.created");
        assert_eq!(e.session_id(), "s1");
    }

    #[test]
    fn event_serde_roundtrip() {
        let e = DomainEvent::SessionKilled {
            session_id: "s2".into(),
            killed_at: "t1".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        let back: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event_type(), "session.killed");
    }
}
