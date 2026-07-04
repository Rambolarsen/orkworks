use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: String) -> Self { Self(id) }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Creating,
    Running,
    Killed,
    Ended,
    Error,
}

impl From<TerminalOutcome> for SessionStatus {
    fn from(value: TerminalOutcome) -> Self {
        match value {
            TerminalOutcome::Ended => Self::Ended,
            TerminalOutcome::Killed => Self::Killed,
            TerminalOutcome::Error => Self::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    Live,
    Remembered,
    Resumable,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionState {
    WaitingForInput,
    Blocked,
    Failed,
    Done,
    Stale,
    Working,
    Idle,
}

impl AttentionState {
    pub fn from_str(s: &str) -> Self {
        match s {
            "waiting_for_input" => Self::WaitingForInput,
            "blocked" => Self::Blocked,
            "failed" => Self::Failed,
            "done" => Self::Done,
            "stale" => Self::Stale,
            "working" => Self::Working,
            "idle" => Self::Idle,
            _ => Self::Idle,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WaitingForInput => "waiting_for_input",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Done => "done",
            Self::Stale => "stale",
            Self::Working => "working",
            Self::Idle => "idle",
        }
    }

    pub fn needs_attention(&self) -> bool {
        matches!(self, Self::WaitingForInput | Self::Blocked | Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkPhase {
    Ideation,
    Implementation,
    Review,
    Debugging,
    Unknown,
}

pub type Phase = WorkPhase;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePhase {
    Creating,
    Active,
    Ending,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    Ended,
    Killed,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedStatusSnapshot {
    pub value: Option<AttentionState>,
    pub source: String,
    pub confidence: Option<f64>,
    #[serde(rename = "observedAt")]
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionTransitionError {
    InvalidPhase,
    MissingPendingOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePath(pub PathBuf);

impl WorkspacePath {
    pub fn new(path: PathBuf) -> Self { Self(path) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_equality() {
        let a = SessionId::new("abc".into());
        let b = SessionId::new("abc".into());
        let c = SessionId::new("def".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn memory_state_serde_roundtrip() {
        let states = vec![
            MemoryState::Live,
            MemoryState::Remembered,
            MemoryState::Resumable,
            MemoryState::Unsupported,
        ];
        for s in states {
            let json = serde_json::to_string(&s).unwrap();
            let back: MemoryState = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn attention_state_from_str_all_variants() {
        assert_eq!(AttentionState::from_str("waiting_for_input"), AttentionState::WaitingForInput);
        assert_eq!(AttentionState::from_str("blocked"), AttentionState::Blocked);
        assert_eq!(AttentionState::from_str("failed"), AttentionState::Failed);
        assert_eq!(AttentionState::from_str("done"), AttentionState::Done);
        assert_eq!(AttentionState::from_str("stale"), AttentionState::Stale);
        assert_eq!(AttentionState::from_str("working"), AttentionState::Working);
        assert_eq!(AttentionState::from_str("idle"), AttentionState::Idle);
        assert_eq!(AttentionState::from_str("bogus"), AttentionState::Idle);
    }

    #[test]
    fn needs_attention_only_for_blocked_failed_waiting() {
        assert!(AttentionState::WaitingForInput.needs_attention());
        assert!(AttentionState::Blocked.needs_attention());
        assert!(AttentionState::Failed.needs_attention());
        assert!(!AttentionState::Done.needs_attention());
        assert!(!AttentionState::Stale.needs_attention());
        assert!(!AttentionState::Working.needs_attention());
        assert!(!AttentionState::Idle.needs_attention());
    }

    #[test]
    fn work_phase_and_lifecycle_phase_serde_roundtrip() {
        let work_json = serde_json::to_string(&WorkPhase::Implementation).unwrap();
        let work_back: WorkPhase = serde_json::from_str(&work_json).unwrap();
        assert_eq!(work_back, WorkPhase::Implementation);

        let lifecycle_json = serde_json::to_string(&LifecyclePhase::Ending).unwrap();
        let lifecycle_back: LifecyclePhase = serde_json::from_str(&lifecycle_json).unwrap();
        assert_eq!(lifecycle_back, LifecyclePhase::Ending);
    }

    #[test]
    fn terminal_outcome_converts_to_terminal_status() {
        assert_eq!(SessionStatus::from(TerminalOutcome::Ended), SessionStatus::Ended);
        assert_eq!(SessionStatus::from(TerminalOutcome::Killed), SessionStatus::Killed);
        assert_eq!(SessionStatus::from(TerminalOutcome::Error), SessionStatus::Error);
    }

    #[test]
    fn observed_status_snapshot_preserves_structured_metadata() {
        let snapshot = ObservedStatusSnapshot {
            value: Some(AttentionState::Blocked),
            source: "peon".into(),
            confidence: Some(0.82),
            observed_at: Some("2026-07-03T12:34:56Z".into()),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"observedAt\""));
        let back: ObservedStatusSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value, Some(AttentionState::Blocked));
        assert_eq!(back.source, "peon");
        assert_eq!(back.confidence, Some(0.82));
        assert_eq!(back.observed_at.as_deref(), Some("2026-07-03T12:34:56Z"));
    }
}
