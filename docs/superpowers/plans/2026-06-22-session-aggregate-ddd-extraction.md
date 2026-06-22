# Session Aggregate DDD Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the Session domain concept from the 1420-line God-object `main.rs` and flat `SessionInfo`/`sessionSort.ts` into proper DDD aggregate with domain/application/infrastructure layers in Rust and a matching domain layer in TypeScript.

**Architecture:** Rust gets `domain/session/` (value objects, entity, events, repository trait, lifecycle service), `application/session/` (commands, driven ports, use case handlers), and `infrastructure/` (adapters, composition root). TypeScript gets `apps/desktop/src/domain/session.ts` with matching value objects, enums, sort logic, and DTO mappers. Wire format (`api.ts`) unchanged. Existing modules become backing implementations.

**Tech Stack:** Rust with serde, chrono, tokio; TypeScript with Node built-in test runner, no new dependencies.

---

### Task 1: Domain value objects

**Files:**
- Create: `crates/orkworksd/src/domain/mod.rs`
- Create: `crates/orkworksd/src/domain/session/mod.rs`
- Create: `crates/orkworksd/src/domain/session/value_objects.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/orkworksd/src/domain/session/value_objects.rs` with basic types and a test module:

```rust
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
pub enum Phase {
    Ideation,
    Implementation,
    Review,
    Debugging,
    Unknown,
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml -- domain::session::value_objects`
Expected: compilation error — module `domain` not found (we need mod.rs)

- [ ] **Step 3: Create module files**

Create `crates/orkworksd/src/domain/mod.rs`:
```rust
pub mod session;
```

Create `crates/orkworksd/src/domain/session/mod.rs`:
```rust
pub mod value_objects;
```

Register in `crates/orkworksd/src/main.rs` — add after existing `mod providers;`:
```rust
mod domain;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml -- domain::session::value_objects`
Expected: all 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/domain/ crates/orkworksd/src/main.rs
git commit -m "feat(domain): add session value objects"
```

---

### Task 2: Domain entity (aggregate root)

**Files:**
- Create: `crates/orkworksd/src/domain/session/entity.rs`
- Modify: `crates/orkworksd/src/domain/session/mod.rs`

- [ ] **Step 1: Write the entity with tests**

Create `crates/orkworksd/src/domain/session/entity.rs`:

```rust
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
```

- [ ] **Step 2: Register module and run tests**

Add to `crates/orkworksd/src/domain/session/mod.rs`:
```rust
pub mod value_objects;
pub mod entity;
```

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml -- domain::session::entity`
Expected: 4 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/domain/session/
git commit -m "feat(domain): add session aggregate entity"
```

---

### Task 3: Domain events

**Files:**
- Create: `crates/orkworksd/src/domain/session/events.rs`
- Modify: `crates/orkworksd/src/domain/session/mod.rs`

- [ ] **Step 1: Write events with tests**

Create `crates/orkworksd/src/domain/session/events.rs`:

```rust
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
```

- [ ] **Step 2: Register module and run tests**

Add to `crates/orkworksd/src/domain/session/mod.rs`:
```rust
pub mod value_objects;
pub mod entity;
pub mod events;
```

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml -- domain::session::events`
Expected: 2 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/domain/session/
git commit -m "feat(domain): add session domain events"
```

---

### Task 4: Repository trait (driven port)

**Files:**
- Create: `crates/orkworksd/src/domain/session/repository.rs`
- Modify: `crates/orkworksd/src/domain/session/mod.rs`

- [ ] **Step 1: Write the trait**

Create `crates/orkworksd/src/domain/session/repository.rs`:

```rust
use super::{entity::Session, events::DomainEvent, value_objects::*};

pub trait SessionRepository: Send + Sync {
    fn save(&self, session: &Session, events: Vec<DomainEvent>) -> Result<(), String>;
    fn load(&self, id: &SessionId) -> Result<Option<Session>, String>;
    fn delete(&self, id: &SessionId) -> Result<(), String>;
    fn list_by_workspace(&self, path: &WorkspacePath) -> Result<Vec<Session>, String>;
    fn append_terminal_output(&self, id: &SessionId, lines: Vec<String>) -> Result<(), String>;
}
```

No tests — this is just a trait interface.

- [ ] **Step 2: Register module and verify compilation**

Add to `crates/orkworksd/src/domain/session/mod.rs`:
```rust
pub mod value_objects;
pub mod entity;
pub mod events;
pub mod repository;
```

Run: `cargo check --manifest-path crates/orkworksd/Cargo.toml`
Expected: compiles cleanly

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/domain/session/
git commit -m "feat(domain): add session repository trait"
```

---

### Task 5: Domain lifecycle service

**Files:**
- Create: `crates/orkworksd/src/domain/session/services.rs`
- Modify: `crates/orkworksd/src/domain/session/mod.rs`

- [ ] **Step 1: Write the lifecycle service with tests**

Create `crates/orkworksd/src/domain/session/services.rs`:

```rust
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
        let (session, events) = SessionLifecycle::create(
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
        let (mut session, _) = SessionLifecycle::create(
            SessionId("s2".into()),
            WorkspacePath(PathBuf::from("/ws")),
            "Test".into(),
            "/ws".into(),
            None, None, None, "now".into(),
            None, None,
        );
        let events = SessionLifecycle::kill(&mut session, "later".into());
        assert_eq!(session.status, SessionStatus::Killed);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "session.killed");
    }

    #[test]
    fn kill_already_killed_produces_no_events() {
        let (mut session, _) = SessionLifecycle::create(
            SessionId("s3".into()),
            WorkspacePath(PathBuf::from("/ws")),
            "Test".into(),
            "/ws".into(),
            None, None, None, "now".into(),
            None, None,
        );
        session.status = SessionStatus::Killed;
        let events = SessionLifecycle::kill(&mut session, "later".into());
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn resume_produces_resumed_event() {
        let (session, _) = SessionLifecycle::create(
            SessionId("s4".into()),
            WorkspacePath(PathBuf::from("/ws")),
            "Test".into(),
            "/ws".into(),
            None, None, None, "now".into(),
            None, None,
        );
        let events = SessionLifecycle::resume(&session, "later".into());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "session.resumed");
    }
}
```

- [ ] **Step 2: Register module and run tests**

Add to `crates/orkworksd/src/domain/session/mod.rs`:
```rust
pub mod value_objects;
pub mod entity;
pub mod events;
pub mod repository;
pub mod services;
```

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml -- domain::session::services`
Expected: 4 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/domain/session/
git commit -m "feat(domain): add session lifecycle domain service"
```

---

### Task 6: Application commands and ports

**Files:**
- Create: `crates/orkworksd/src/application/mod.rs`
- Create: `crates/orkworksd/src/application/session/mod.rs`
- Create: `crates/orkworksd/src/application/session/commands.rs`
- Create: `crates/orkworksd/src/application/session/ports.rs`
- Modify: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Write commands**

Create `crates/orkworksd/src/application/mod.rs`:
```rust
pub mod session;
```

Create `crates/orkworksd/src/application/session/mod.rs`:
```rust
pub mod commands;
pub mod ports;
```

Create `crates/orkworksd/src/application/session/commands.rs`:

```rust
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
```

Create `crates/orkworksd/src/application/session/ports.rs`:

```rust
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
```

- [ ] **Step 2: Register modules and verify compilation**

In `crates/orkworksd/src/main.rs`, add after `mod domain;`:
```rust
mod application;
```

Run: `cargo check --manifest-path crates/orkworksd/Cargo.toml`
Expected: compiles cleanly

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/application/ crates/orkworksd/src/main.rs
git commit -m "feat(application): add session commands and ports"
```

---

### Task 7: Application use case handlers

**Files:**
- Create: `crates/orkworksd/src/application/session/handlers.rs`
- Modify: `crates/orkworksd/src/application/session/mod.rs`

- [ ] **Step 1: Write use case handlers**

Create `crates/orkworksd/src/application/session/handlers.rs`:

```rust
use crate::domain::session::{
    entity::Session,
    events::DomainEvent,
    repository::SessionRepository,
    services::SessionLifecycle,
    value_objects::*,
};
use crate::harness::CommandSpec;
use super::commands::*;
use super::ports::*;

pub struct CreateSessionHandler;

impl CreateSessionHandler {
    /// Returns (session, events, command_spec) — the command_spec is returned
    /// so the caller (main.rs) can pass it to the PTY spawner after resolving
    /// harness config. Harness resolution stays in main.rs for now.
    pub fn handle(
        lifecycle: &SessionLifecycle,
        cmd: &CreateSessionCommand,
        id: &SessionId,
        label: &str,
        workspace_path: &WorkspacePath,
        created_at: &str,
        command_spec: &CommandSpec,
        provider_id: Option<String>,
        provider_label: Option<String>,
    ) -> (Session, Vec<DomainEvent>) {
        let resume_memory = crate::harness::ResumeMemory {
            state: crate::harness::ResumeState::Available,
            preferred_strategy: crate::harness::ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some(created_at.to_string()),
        };

        lifecycle.create(
            id.clone(),
            workspace_path.clone(),
            label.to_string(),
            cmd.cwd.clone(),
            cmd.harness_name.clone(),
            provider_id,
            cmd.model.clone(),
            created_at.to_string(),
            None,  // git context is detected and mapped in the handler caller
            Some(resume_memory),
        )
    }
}

pub struct KillSessionHandler;

impl KillSessionHandler {
    pub fn handle(
        repo: &dyn SessionRepository,
        lifecycle: &SessionLifecycle,
        cmd: &KillSessionCommand,
        killed_at: &str,
    ) -> Result<(Session, Vec<DomainEvent>), String> {
        let mut session = repo.load(&cmd.session_id)?
            .ok_or_else(|| format!("session {} not found", cmd.session_id))?;

        let events = lifecycle.kill(&mut session, killed_at.to_string());
        if events.is_empty() {
            return Err("session already killed".into());
        }

        repo.save(&session, events.clone())?;
        Ok((session, events))
    }
}

pub struct ResumeSessionHandler;

impl ResumeSessionHandler {
    pub fn handle(
        repo: &dyn SessionRepository,
        lifecycle: &SessionLifecycle,
        cmd: &ResumeSessionCommand,
        resumed_at: &str,
    ) -> Result<(Session, Vec<DomainEvent>), String> {
        let session = repo.load(&cmd.session_id)?
            .ok_or_else(|| format!("session {} not found", cmd.session_id))?;

        let events = lifecycle.resume(&session, resumed_at.to_string());
        Ok((session, events))
    }
}

pub struct ForgetSessionHandler;

impl ForgetSessionHandler {
    pub fn handle(
        repo: &dyn SessionRepository,
        cmd: &ForgetSessionCommand,
    ) -> Result<(), String> {
        repo.delete(&cmd.session_id)
    }
}

pub struct ListWorkspaceSessionsHandler;

impl ListWorkspaceSessionsHandler {
    pub fn handle(
        repo: &dyn SessionRepository,
        cmd: &ListWorkspaceSessionsCommand,
    ) -> Result<Vec<Session>, String> {
        repo.list_by_workspace(&WorkspacePath(cmd.workspace_path.clone()))
    }
}
```

- [ ] **Step 2: Register module and check compilation**

Add to `crates/orkworksd/src/application/session/mod.rs`:
```rust
pub mod commands;
pub mod ports;
pub mod handlers;
```

Run: `cargo check --manifest-path crates/orkworksd/Cargo.toml`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/application/session/
git commit -m "feat(application): add session use case handlers"
```

---

### Task 8: Infrastructure — MetadataSessionRepository adapter

**Files:**
- Create: `crates/orkworksd/src/infrastructure/mod.rs`
- Create: `crates/orkworksd/src/infrastructure/session_repository.rs`
- Modify: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Write the adapter**

Create `crates/orkworksd/src/infrastructure/mod.rs`:
```rust
pub mod session_repository;
pub mod session_pty;
pub mod session_git;
pub mod session_module;
```

Create `crates/orkworksd/src/infrastructure/session_repository.rs`:

```rust
use std::sync::{Arc, Mutex};
use crate::domain::session::{
    entity::Session,
    events::DomainEvent,
    repository::SessionRepository,
    value_objects::*,
};
use crate::metadata::{MetadataStore, SessionMetadata};

/// Wraps the workspace-scoped MetadataStore in Arc<Mutex<Option<_>>> so it
/// can be shared across handlers without cloning MetadataStore (which holds
/// PathBufs and is not Clone).
pub struct MetadataSessionRepository {
    store: Arc<Mutex<Option<MetadataStore>>>,
}

impl MetadataSessionRepository {
    pub fn new() -> Self {
        Self { store: Arc::new(Mutex::new(None)) }
    }

    pub fn set_store(&self, store: MetadataStore) {
        *self.store.lock().unwrap() = Some(store);
    }

    fn with_store<F, R>(&self, f: F) -> Result<R, String>
    where F: FnOnce(&MetadataStore) -> Result<R, String>
    {
        let guard = self.store.lock().unwrap();
        match guard.as_ref() {
            Some(store) => f(store),
            None => Err("no workspace set".into()),
        }
    }
}

impl SessionRepository for MetadataSessionRepository {
    fn save(&self, session: &Session, events: Vec<DomainEvent>) -> Result<(), String> {
        self.with_store(|store| {
            let meta = SessionMetadata {
                id: session.id.0.clone(),
                label: session.label.clone(),
                workspace: session.workspace_path.0.display().to_string(),
                task: session.task_description.clone().unwrap_or_default(),
                harness: session.harness_name.clone().unwrap_or_default(),
                model: session.model.clone().unwrap_or_default(),
                cwd: session.cwd.clone(),
                status: session_status_to_str(&session.status),
                phase: phase_to_str(&session.phase),
                observed_status: None,
                summary: None,
                next_action: None,
                needs_user_input: None,
                detected_question: None,
                suggested_options: None,
                blocker_description: None,
                failed_command: None,
                failed_test: None,
                capacity_hints: None,
                peon_last_inference: None,
                provider_id: session.provider_id.clone(),
                provider_label: None,
                provider_model: None,
                provider_state: None,
                created_at: session.created_at.clone(),
                last_activity: chrono::Utc::now().to_rfc3339(),
                metadata_source: "process".into(),
                metadata_confidence: 1.0,
                repo_root: session.repo_root.clone(),
                branch: session.branch.clone(),
                dirty: session.dirty,
                changed_files: session.changed_files,
                is_worktree: session.is_worktree,
                resume: session.resume.clone(),
                resumed_from: session.resumed_from.clone(),
            };
            store.write_session(&meta);

            for event in &events {
                store.append_event(&session.id.0, &crate::metadata::Event {
                    event_type: event.event_type().to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    status: session_status_to_str(&session.status).to_string(),
                    observed_status: None,
                    confidence: None,
                });
            }

            Ok(())
        })
    }

    fn load(&self, id: &SessionId) -> Result<Option<Session>, String> {
        self.with_store(|store| {
            Ok(store.read_session(&id.0).map(|meta| meta_to_session(&meta)))
        })
    }

    fn delete(&self, id: &SessionId) -> Result<(), String> {
        self.with_store(|store| {
            store.delete_session(&id.0)
                .map_err(|e| format!("delete failed: {e}"))
        })
    }

    fn list_by_workspace(&self, path: &WorkspacePath) -> Result<Vec<Session>, String> {
        self.with_store(|store| {
            Ok(store.read_all_sessions().into_iter()
                .map(|meta| meta_to_session(&meta))
                .collect())
        })
    }

    fn append_terminal_output(&self, id: &SessionId, lines: Vec<String>) -> Result<(), String> {
        self.with_store(|store| {
            store.append_terminal_output(&id.0, &lines);
            Ok(())
        })
    }
}

fn session_status_to_str(s: &SessionStatus) -> &'static str {
    match s {
        SessionStatus::Creating => "creating",
        SessionStatus::Running => "running",
        SessionStatus::Killed => "killed",
        SessionStatus::Ended => "ended",
        SessionStatus::Error => "error",
    }
}

fn phase_to_str(p: &Phase) -> &'static str {
    match p {
        Phase::Ideation => "ideation",
        Phase::Implementation => "implementation",
        Phase::Review => "review",
        Phase::Debugging => "debugging",
        Phase::Unknown => "",
    }
}

fn meta_to_session(meta: &SessionMetadata) -> Session {
    use crate::domain::session::value_objects::*;
    use std::path::PathBuf;

    Session {
        id: SessionId(meta.id.clone()),
        workspace_path: WorkspacePath(PathBuf::from(&meta.workspace)),
        status: match meta.status.as_str() {
            "creating" => SessionStatus::Creating,
            "running" => SessionStatus::Running,
            "killed" => SessionStatus::Killed,
            "ended" => SessionStatus::Ended,
            "error" => SessionStatus::Error,
            _ => SessionStatus::Ended,
        },
        memory_state: MemoryState::Remembered,
        attention_state: AttentionState::from_str(
            meta.observed_status.as_deref().unwrap_or(&meta.status),
        ),
        phase: match meta.phase.as_str() {
            "ideation" => Phase::Ideation,
            "implementation" => Phase::Implementation,
            "review" => Phase::Review,
            "debugging" => Phase::Debugging,
            _ => Phase::Unknown,
        },
        created_at: meta.created_at.clone(),
        killed_at: None,
        last_active_at: Some(meta.last_activity.clone()),
        harness_name: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
        provider_id: meta.provider_id.clone(),
        task_description: (!meta.task.is_empty()).then(|| meta.task.clone()),
        label: meta.label.clone(),
        cwd: meta.cwd.clone(),
        model: (!meta.model.is_empty()).then(|| meta.model.clone()),
        repo_root: meta.repo_root.clone(),
        branch: meta.branch.clone(),
        dirty: meta.dirty,
        changed_files: meta.changed_files,
        is_worktree: meta.is_worktree,
        resume: meta.resume.clone(),
        resumed_from: meta.resumed_from.clone(),
        resume_strategy: crate::harness::ResumeStrategy::None,
    }
}
```

- [ ] **Step 2: Register modules and check compilation**

In `crates/orkworksd/src/main.rs`, add after `mod application;`:
```rust
mod infrastructure;
```

Run: `cargo check --manifest-path crates/orkworksd/Cargo.toml`
Expected: compiles (may have warnings about unused imports)

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/infrastructure/ crates/orkworksd/src/main.rs
git commit -m "feat(infrastructure): add metadata session repository adapter"
```

---

### Task 9: Infrastructure — PTY and Git adapters

**Files:**
- Create: `crates/orkworksd/src/infrastructure/session_pty.rs`
- Create: `crates/orkworksd/src/infrastructure/session_git.rs`

- [ ] **Step 1: Write PTY adapter**

Create `crates/orkworksd/src/infrastructure/session_pty.rs`:

```rust
use std::path::PathBuf;
use crate::application::session::ports::{PtyHandle, PtySpawner, PtyKiller};
use crate::domain::session::value_objects::SessionId;
use crate::harness::CommandSpec;

pub struct RealPtySpawner;

impl PtySpawner for RealPtySpawner {
    fn spawn(&self, id: &SessionId, cwd: &PathBuf, command: &CommandSpec)
        -> Result<PtyHandle, String>
    {
        let pty_system = make_pty_system();
        let pair = pty_system.openpty(crate::main::default_pty_size())
            .map_err(|e| format!("pty open failed: {e}"))?;

        let mut cmd = portable_pty::CommandBuilder::new(&command.program);
        cmd.args(&command.args);
        cmd.cwd(cwd);
        for (key, value) in &crate::main::terminal_env_overrides() {
            cmd.env(key, value);
        }
        for (key, value) in std::env::vars() {
            if !crate::main::should_forward_terminal_env(&key) {
                continue;
            }
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd)
            .map_err(|e| format!("spawn failed: {e}"))?;

        drop(pair.slave);

        Ok(PtyHandle {
            child_pty: Box::new(pair.master),
            writer: Box::new(std::io::sink()),
        })
    }
}

fn make_pty_system() -> Box<dyn portable_pty::PtySystem> {
    #[cfg(unix)]
    { Box::new(portable_pty::unix::UnixPtySystem {}) }
    #[cfg(windows)]
    { Box::new(portable_pty::win::conpty::ConPtySystem {}) }
}

pub struct RealPtyKiller;

impl PtyKiller for RealPtyKiller {
    fn kill(&self, _handle: PtyHandle) -> Result<(), String> {
        Ok(())
    }
}
```

Create `crates/orkworksd/src/infrastructure/session_git.rs`:

```rust
use std::path::PathBuf;
use crate::application::session::ports::GitDetector;

pub struct RealGitDetector;

impl GitDetector for RealGitDetector {
    fn detect(&self, path: &PathBuf) -> Option<crate::git::GitContext> {
        Some(crate::git::detect(path))
    }
}
```

- [ ] **Step 2: Check compilation**

Run: `cargo check --manifest-path crates/orkworksd/Cargo.toml`
Expected: compiles (session_pty.rs needs reference to main functions — we'll fix in next task)

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/infrastructure/
git commit -m "feat(infrastructure): add PTY and Git adapters"
```

---

### Task 10: Infrastructure — composition root (SessionModule)

**Files:**
- Create: `crates/orkworksd/src/infrastructure/session_module.rs`
- Modify: `crates/orkworksd/src/infrastructure/mod.rs`

- [ ] **Step 1: Write composition root**

Create `crates/orkworksd/src/infrastructure/session_module.rs`:

```rust
use std::sync::Arc;
use crate::application::session::handlers::*;
use crate::infrastructure::session_repository::MetadataSessionRepository;
use crate::infrastructure::session_pty::{RealPtySpawner, RealPtyKiller};
use crate::infrastructure::session_git::RealGitDetector;
use crate::domain::session::services::SessionLifecycle;

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
    pub lifecycle: SessionLifecycle,
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
            lifecycle: SessionLifecycle,
        }
    }
}
```

- [ ] **Step 2: Check compilation**

Run: `cargo check --manifest-path crates/orkworksd/Cargo.toml`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/infrastructure/
git commit -m "feat(infrastructure): add session module composition root"
```

---

### Task 11: Rewire main.rs — thin HTTP handlers

**Files:**
- Modify: `crates/orkworksd/src/main.rs`

Replace `sessions: Mutex<HashMap<String, SessionHandle>>` with `session_module: SessionModule` in AppState. Rewrite `create_session`, `delete_session`, and `forget_session` as thin adapters. `list_sessions`, `resume_session`, `session_terminal_handler`, and the Peon loop need manual follow-up — the plan provides patterns for each below.

- [ ] **Step 1: Add SessionModule to AppState**

In `main.rs`, add imports:
```rust
use crate::infrastructure::session_module::SessionModule;
use crate::domain::session::value_objects::{SessionId, WorkspacePath, MemoryState as DomainMemoryState, SessionStatus as DomainSessionStatus};
```

Modify `AppState` — remove `sessions` field, add `session_module`:
```rust
struct AppState {
    session_module: SessionModule,
    workspace: Mutex<Option<WorkspaceState>>,
    peon: PeonState,
    providers: providers::ProviderManager,
    adapters: HashMap<String, harness::HarnessAdapter>,
    retention_config: tokio::sync::RwLock<RetentionConfig>,
    harnesses: tokio::sync::RwLock<Vec<HarnessConfig>>,
}
```

Update `main()` init — replace the `sessions: Mutex::new(HashMap::new())` line with:
```rust
session_module: SessionModule::new(),
```

- [ ] **Step 2: Rewrite create_session handler**

```rust
async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".into());
    let now = iso_now();

    let resolved_launch = {
        let harnesses = state.harnesses.read().await;
        resolve_session_launch(&harnesses, &req, cwd.clone())
    };

    let session_id = SessionId(id.clone());
    let ws_path = WorkspacePath(std::path::PathBuf::from(&cwd));

    let cmd = crate::application::session::commands::CreateSessionCommand {
        harness_name: resolved_launch.session_harness_id.clone(),
        model: resolved_launch.model.clone(),
        initial_prompt: req.initial_prompt.clone(),
        cwd: cwd.clone(),
    };

    // Domain logic: create session (no I/O)
    let (session, events) = state.session_module.create_session_handler.handle(
        &state.session_module.lifecycle,
        &cmd,
        &session_id,
        &format!("Session {}", &id[..8]),
        &ws_path,
        &now,
        &resolved_launch.command,
        resolved_launch.provider_id.clone(),
        resolved_launch.provider_label.clone(),
    );

    // Persist metadata through repository
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        // Sync repository with current workspace store (avoid clone by
        // sharing the store reference via set_store or direct calls)
        ws.metadata.write_session(&session_to_metadata(&session, &resolved_launch, &now));
        for event in &events {
            ws.metadata.append_event(&id, &domain_event_to_metadata_event(event, &session, &now));
        }
        // Write git context to stored metadata
        let git_ctx = crate::git::detect(&ws.path);
        if let Some(mut meta) = ws.metadata.read_session(&id) {
            meta.repo_root = git_ctx.repo_root;
            meta.branch = git_ctx.branch;
            meta.dirty = Some(git_ctx.dirty);
            meta.changed_files = Some(git_ctx.changed_files);
            meta.is_worktree = Some(git_ctx.is_worktree);
            ws.metadata.write_session(&meta);
        }
    }
    drop(ws_guard);

    // Spawn PTY (existing spawn logic extracted to a helper)
    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);
    let handle = SessionHandle {
        info: session_to_info(&session, &resolved_launch),
        kill_tx,
        output_buffer: crate::peon::RingBuffer::new(state.peon.config.max_lines),
        command: resolved_launch.command,
        initial_prompt: req.initial_prompt.clone(),
    };
    state.session_module.session_handles().lock().unwrap().insert(id.clone(), handle);

    // Build SessionInfo response using existing mapping
    let info = session_to_info(&session, &resolved_launch);
    Json(info)
}
```

**Note:** This handler currently stores the `SessionHandle` (PTY state) on a new `session_handles` HashMap on `SessionModule` rather than the old `AppState.sessions`. Add this method to `SessionModule`:
```rust
use std::sync::Mutex;
use std::collections::HashMap;

// In SessionModule:
pub session_handles: Mutex<HashMap<String, crate::main::SessionHandle>>,
```

And import `SessionHandle` via a public re-export from main.rs or migrate the `SessionHandle` struct to `infrastructure/` separately.

- [ ] **Step 3: Rewrite kill_session handler**

```rust
async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let session_id = SessionId(id.clone());
    let now = iso_now();

    let cmd = crate::application::session::commands::KillSessionCommand {
        session_id: session_id.clone(),
    };

    // Send kill signal to PTY (existing logic)
    let handle = state.session_module.session_handles.lock().unwrap().get(&id).map(|h| h.kill_tx.clone());
    if let Some(kill_tx) = handle {
        let _ = kill_tx.send(true);
    } else {
        return axum::http::StatusCode::NOT_FOUND;
    }

    // Update session handle status
    {
        let mut handles = state.session_module.session_handles.lock().unwrap();
        if let Some(h) = handles.get_mut(&id) {
            h.info.status = "killed".to_string();
        }
    }

    // Persist through metadata
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(&id) {
            meta.status = "killed".to_string();
            meta.last_activity = now.clone();
            ws.metadata.write_session(&meta);
        }
        ws.metadata.append_event(&id, &crate::metadata::Event {
            event_type: "session.killed".into(),
            timestamp: now,
            status: "killed".into(),
            observed_status: None,
            confidence: None,
        });
    }
    drop(ws_guard);

    state.peon.last_output.write().unwrap().remove(&id);
    state.peon.last_inference.write().unwrap().remove(&id);
    axum::http::StatusCode::OK
}
```

- [ ] **Step 4: Rewrite forget_session handler**

```rust
async fn forget_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Check not a live session (existing logic)
    {
        let handles = state.session_module.session_handles.lock().unwrap();
        if let Some(h) = handles.get(&id) {
            if h.info.status == "live" || h.info.status == "creating" || h.info.status == "running" {
                return (axum::http::StatusCode::CONFLICT, "Cannot forget a live session. Kill it first.").into_response();
            }
        }
    }

    let ws_guard = state.workspace.lock().unwrap();
    let ws = match &*ws_guard {
        Some(ws) => ws,
        None => return axum::http::StatusCode::CONFLICT.into_response(),
    };

    if ws.metadata.read_session(&id).is_none() {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }

    if let Err(e) = ws.metadata.delete_session(&id) {
        tracing::error!("failed to delete session {id}: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if let Err(e) = ws.metadata.delete_events(&id) {
        tracing::error!("failed to delete events for {id}: {e}");
    }
    let _ = ws.metadata.clear_last_active_session_if_matches(&id);
    drop(ws_guard);

    state.session_module.session_handles.lock().unwrap().remove(&id);
    state.peon.last_output.write().unwrap().remove(&id);
    state.peon.last_inference.write().unwrap().remove(&id);

    axum::http::StatusCode::OK.into_response()
}
```

- [ ] **Step 5: Add helper conversion functions**

Add to `main.rs`:

```rust
fn session_to_metadata(
    session: &crate::domain::session::entity::Session,
    launch: &ResolvedSessionLaunch,
    now: &str,
) -> crate::metadata::SessionMetadata {
    crate::metadata::SessionMetadata {
        id: session.id.0.clone(),
        label: session.label.clone(),
        workspace: session.workspace_path.0.display().to_string(),
        task: session.task_description.clone().unwrap_or_default(),
        harness: session.harness_name.clone().unwrap_or_default(),
        model: session.model.clone().unwrap_or_default(),
        cwd: session.cwd.clone(),
        status: domain_status_str(&session.status),
        phase: domain_phase_str(&session.phase),
        observed_status: None,
        summary: None,
        next_action: None,
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        peon_last_inference: None,
        provider_id: launch.provider_id.clone(),
        provider_label: launch.provider_label.clone(),
        provider_model: None,
        provider_state: None,
        created_at: now.to_string(),
        last_activity: now.to_string(),
        metadata_source: "process".into(),
        metadata_confidence: 1.0,
        repo_root: session.repo_root.clone(),
        branch: session.branch.clone(),
        dirty: session.dirty,
        changed_files: session.changed_files,
        is_worktree: session.is_worktree,
        resume: session.resume.clone(),
        resumed_from: session.resumed_from.clone(),
    }
}

fn session_to_info(
    session: &crate::domain::session::entity::Session,
    launch: &ResolvedSessionLaunch,
) -> SessionInfo {
    SessionInfo {
        id: session.id.0.clone(),
        label: session.label.clone(),
        harness: session.harness_name.clone(),
        model: session.model.clone(),
        status: domain_status_str(&session.status),
        cwd: session.cwd.clone(),
        created_at: session.created_at.clone(),
        observed_status: None,
        summary: None,
        next_action: None,
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        metadata_source: Some("process".into()),
        metadata_confidence: Some(1.0),
        repo_root: session.repo_root.clone(),
        branch: session.branch.clone(),
        dirty: session.dirty,
        changed_files: session.changed_files,
        is_worktree: session.is_worktree,
        conflict_warning: None,
        recommendation: None,
        peon_last_inference: None,
        memory_state: to_memory_state(&session.memory_state),
        resume_strategy: crate::harness::ResumeStrategy::None,
        resume: session.resume.clone(),
        resumed_from: session.resumed_from.clone(),
        provider: launch.provider_label.clone(),
        provider_model: None,
        provider_state: None,
    }
}

fn to_memory_state(ms: &DomainMemoryState) -> MemoryState {
    match ms {
        DomainMemoryState::Live => MemoryState::Live,
        DomainMemoryState::Remembered => MemoryState::Remembered,
        DomainMemoryState::Resumable => MemoryState::Resumable,
        DomainMemoryState::Unsupported => MemoryState::Unsupported,
    }
}

fn domain_status_str(s: &DomainSessionStatus) -> String {
    match s {
        DomainSessionStatus::Creating => "creating".into(),
        DomainSessionStatus::Running => "running".into(),
        DomainSessionStatus::Killed => "killed".into(),
        DomainSessionStatus::Ended => "ended".into(),
        DomainSessionStatus::Error => "error".into(),
    }
}

fn domain_phase_str(p: &crate::domain::session::value_objects::Phase) -> String {
    match p {
        crate::domain::session::value_objects::Phase::Ideation => "ideation".into(),
        crate::domain::session::value_objects::Phase::Implementation => "implementation".into(),
        crate::domain::session::value_objects::Phase::Review => "review".into(),
        crate::domain::session::value_objects::Phase::Debugging => "debugging".into(),
        crate::domain::session::value_objects::Phase::Unknown => String::new(),
    }
}

fn domain_event_to_metadata_event(
    event: &crate::domain::session::events::DomainEvent,
    session: &crate::domain::session::entity::Session,
    now: &str,
) -> crate::metadata::Event {
    crate::metadata::Event {
        event_type: event.event_type().to_string(),
        timestamp: now.to_string(),
        status: domain_status_str(&session.status),
        observed_status: None,
        confidence: None,
    }
}
```

- [ ] **Step 6: Fix remaining references to old sessions HashMap**

Search `main.rs` for `state.sessions.lock()`. For each occurrence:
- `list_sessions` (line ~719): change to read from `state.session_module.session_handles.lock()` for live sessions, and `state.workspace` for remembered
- `session_terminal_handler` (line ~1142): same pattern
- `peon_loop` (line ~1212): change `state.sessions.lock()` to `state.session_module.session_handles.lock()`
- `retention_cleanup_task` (line ~1036): same pattern

Use `cargo check` iteratively to find and fix each reference. The pattern is mechanical: replace `state.sessions.lock().unwrap()` with `state.session_module.session_handles.lock().unwrap()`.

- [ ] **Step 7: Verify compilation and run tests**

Run: `cargo check --manifest-path crates/orkworksd/Cargo.toml`
Fix errors iteratively until clean compile.

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests pass (existing tests may need minor updates for the HashMap→session_handles rename)

- [ ] **Step 8: Commit**

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/src/infrastructure/
git commit -m "refactor(main): rewire session handlers through domain layer"
```

---

### Task 12: TypeScript domain layer

**Files:**
- Create: `apps/desktop/src/domain/session.ts`
- Create: `apps/desktop/src/domain/mod.ts`

- [ ] **Step 1: Write domain layer**

Create `apps/desktop/src/domain/mod.ts`:
```typescript
export * from "./session.ts";
```

Create `apps/desktop/src/domain/session.ts`:

```typescript
import type { SessionInfo, MemoryState as ApiMemoryState } from "../api.ts";

declare const __sessionIdBrand: unique symbol;
export type SessionId = string & { readonly [__sessionIdBrand]: true };

export enum SessionStatus {
  Creating = "creating",
  Running = "running",
  Killed = "killed",
  Ended = "ended",
  Error = "error",
}

export enum MemoryState {
  Live = "live",
  Remembered = "remembered",
  Resumable = "resumable",
  Unsupported = "unsupported",
}

export enum AttentionState {
  WaitingForInput = "waiting_for_input",
  Blocked = "blocked",
  Failed = "failed",
  Done = "done",
  Stale = "stale",
  Working = "working",
  Idle = "idle",
}

export enum Phase {
  Ideation = "ideation",
  Implementation = "implementation",
  Review = "review",
  Debugging = "debugging",
  Unknown = "unknown",
}

export interface Session {
  id: SessionId;
  label: string;
  workspacePath: string;
  status: SessionStatus;
  memoryState: MemoryState;
  attentionState: AttentionState;
  phase: Phase;
  created: Date;
  killed?: Date;
  lastActive?: Date;
  cwd: string;
  harnessName?: string;
  providerId?: string;
  taskDescription?: string;
  model?: string;
  repoRoot?: string;
  branch?: string;
  dirty?: boolean;
  changedFiles?: number;
  isWorktree?: boolean;
  observedStatus?: string;
  metadataSource?: string;
  metadataConfidence?: number;
  summary?: string;
  nextAction?: string;
  needsUserInput?: boolean;
  detectedQuestion?: string;
  suggestedOptions?: string[];
  blockerDescription?: string;
  failedCommand?: string;
  failedTest?: string;
  capacityHints?: string[];
  peonLastInference?: string;
  conflictWarning?: string;
  recommendation?: string;
  provider?: string;
  providerModel?: string;
  providerState?: string;
  resumeStrategy?: string;
  resume?: Record<string, unknown>;
  resumedFrom?: string;
}

export const ATTENTION_PRIORITY: Record<string, number> = {
  [AttentionState.WaitingForInput]: 0,
  [AttentionState.Blocked]: 1,
  [AttentionState.Failed]: 2,
  [AttentionState.Done]: 3,
  [AttentionState.Stale]: 4,
  [AttentionState.Working]: 5,
  [AttentionState.Idle]: 6,
  creating: 7,
  running: 8,
  ended: 9,
  killed: 10,
  error: 11,
};

export function needsAttention(session: Session): boolean {
  const state = sessionAttentionStatus(session);
  return state === AttentionState.Blocked
    || state === AttentionState.Failed
    || state === AttentionState.WaitingForInput;
}

export function sessionAttentionStatus(session: Session): string {
  return session.observedStatus ?? session.status;
}

export function sortSessions(sessions: Session[]): Session[] {
  return [...sessions].sort((a, b) => {
    const la = a.memoryState === MemoryState.Live ? 0 : 1;
    const lb = b.memoryState === MemoryState.Live ? 0 : 1;
    if (la !== lb) return la - lb;
    const pa = ATTENTION_PRIORITY[sessionAttentionStatus(a)] ?? 99;
    const pb = ATTENTION_PRIORITY[sessionAttentionStatus(b)] ?? 99;
    if (pa !== pb) return pa - pb;
    return a.label.localeCompare(b.label);
  });
}

export function fromApiDto(dto: SessionInfo): Session {
  return {
    id: dto.id as SessionId,
    label: dto.label,
    workspacePath: dto.cwd,
    status: dto.status as SessionStatus,
    memoryState: dto.memoryState as MemoryState,
    attentionState: (dto.observedStatus ?? dto.status) as AttentionState,
    phase: Phase.Unknown,
    created: new Date(dto.created_at),
    lastActive: dto.peonLastInference ? new Date(dto.peonLastInference) : undefined,
    cwd: dto.cwd,
    harnessName: dto.harness,
    providerId: dto.provider,
    taskDescription: undefined,
    model: dto.model,
    repoRoot: dto.repoRoot,
    branch: dto.branch,
    dirty: dto.dirty,
    changedFiles: dto.changedFiles,
    isWorktree: dto.isWorktree,
    observedStatus: dto.observedStatus,
    metadataSource: dto.metadataSource,
    metadataConfidence: dto.metadataConfidence,
    summary: dto.summary,
    nextAction: dto.nextAction,
    needsUserInput: dto.needsUserInput,
    detectedQuestion: dto.detectedQuestion,
    suggestedOptions: dto.suggestedOptions,
    blockerDescription: dto.blockerDescription,
    failedCommand: dto.failedCommand,
    failedTest: dto.failedTest,
    capacityHints: dto.capacityHints,
    peonLastInference: dto.peonLastInference,
    conflictWarning: dto.conflictWarning,
    recommendation: dto.recommendation,
    provider: dto.provider,
    providerModel: dto.providerModel,
    providerState: dto.providerState,
    resumeStrategy: dto.resumeStrategy,
    resume: dto.resume as Record<string, unknown> | undefined,
    resumedFrom: dto.resumedFrom,
  };
}
```

- [ ] **Step 2: Run existing tests to ensure nothing breaks**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/sessionSort.test.ts`
Expected: still passes (we haven't changed sessionSort.ts yet)

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/domain/
git commit -m "feat(ui): add TypeScript session domain layer"
```

---

### Task 13: Refactor sessionSort.ts to delegate to domain layer

**Files:**
- Modify: `apps/desktop/src/sessionSort.ts`

- [ ] **Step 1: Rewrite sessionSort.ts as delegation layer**

Replace `apps/desktop/src/sessionSort.ts` content:

```typescript
import type { SessionInfo } from "./api.ts";
import { fromApiDto, sortSessions as domainSortSessions, needsAttention as domainNeedsAttention, sessionAttentionStatus as domainAttentionStatus, ATTENTION_PRIORITY } from "./domain/session.ts";

export { ATTENTION_PRIORITY };

export function needsAttention(status: string): boolean {
  const s = { observedStatus: status, status } as Session;
  return domainNeedsAttention({ observedStatus: status, status } as any);
}

export function sessionAttentionStatus(session: SessionInfo): string {
  return domainAttentionStatus(fromApiDto(session));
}

export function sortSessions(list: SessionInfo[]): SessionInfo[] {
  const domainSessions = list.map(fromApiDto);
  const sorted = domainSortSessions(domainSessions);
  // Map back to SessionInfo by ID lookup
  return sorted.map(ds => list.find(s => s.id === ds.id)!);
}
```

Wait — that's convoluted. Better approach: `sortSessions` sorts `SessionInfo[]` directly using domain rules but returns `SessionInfo[]`:

```typescript
import type { SessionInfo } from "./api.ts";
import { fromApiDto, sortSessions as domainSortSessions, needsAttention as domainNeedsAttention, sessionAttentionStatus as domainAttentionStatus, type ATTENTION_PRIORITY as DomainPriority } from "./domain/session.ts";

export const ATTENTION_PRIORITY: Record<string, number> = {
  waiting_for_input: 0,
  blocked: 1,
  failed: 2,
  done: 3,
  stale: 4,
  working: 5,
  idle: 6,
  creating: 7,
  running: 8,
  ended: 9,
  killed: 10,
  error: 11,
};

export function needsAttention(status: string): boolean {
  return status === "blocked" || status === "failed" || status === "waiting_for_input";
}

export function sessionAttentionStatus(session: SessionInfo): string {
  return fromApiDto(session).observedStatus ?? session.status;
}

export function sortSessions(list: SessionInfo[]): SessionInfo[] {
  const domainSessions = list.map(fromApiDto);
  const sorted = domainSortSessions(domainSessions);
  const lookup = new Map(list.map(s => [s.id, s]));
  return sorted.map(ds => lookup.get(ds.id as string)!).filter(Boolean);
}
```

- [ ] **Step 2: Run tests**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/sessionSort.test.ts`
Expected: all 4 tests PASS

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/sessionSort.ts
git commit -m "refactor(ui): delegate sessionSort to domain layer"
```

---

### Task 14: Verify App.tsx integration

**Files:**
- Modify: `apps/desktop/src/App.tsx`

- [ ] **Step 1: Verify App.tsx compiles and test nothing breaks**

Check that `App.tsx` still imports from `sessionSort.ts` (which now delegates to domain). No changes needed to App.tsx if the sessionSort exports are preserved.

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: compiles with no new errors

- [ ] **Step 2: Run the full test suite**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: all tests pass

Then run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/App.tsx  # if changed
git commit -m "refactor(ui): verify App.tsx with domain layer"
```

---

### Task 15: Run verification-before-completion

- [ ] **Step 1: Run Rust tests**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: all tests PASS

- [ ] **Step 2: Run TypeScript tests**

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: all tests PASS

- [ ] **Step 3: Run TS type check**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: clean

- [ ] **Step 4: Run doc currency check**

```bash
bash .claude/hooks/doc-check.sh
```

- [ ] **Step 5: Commit if docs changed**

```bash
git add docs/
git commit -m "docs: update docs for session aggregate extraction"
```

- [ ] **Step 6: Final commit**

```bash
git status
git log --oneline -15
```
