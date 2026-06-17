# Harness Adapters and Resumable Sessions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a harness adapter interface plus remembered workspace/session state so OrkWorks can explicitly resume prior harness sessions by exact id when available, or latest-in-cwd/repo as fallback.

**Architecture:** The Rust sidecar owns harness adapter command construction, session metadata, resume strategy selection, and live-vs-remembered session normalization. Electron owns app-level recent workspace memory in the user data directory. The React frontend consumes normalized session memory fields and exposes an explicit Resume action.

**Tech Stack:** Rust/Axum/portable-pty/serde for backend session handling, Electron IPC and Node `fs` for app-level memory, React/TypeScript for UI, Node test runner and Cargo tests for verification.

---

## File Structure

- Create `crates/orkworksd/src/harness.rs`
  - Harness adapter types, command specs, capability flags, generic template rendering, resume strategy selection, and tests.
- Modify `crates/orkworksd/src/metadata.rs`
  - Session resume memory fields, repo-local workspace memory, all-session metadata reads, and tests.
- Modify `crates/orkworksd/src/main.rs`
  - Add `mod harness`, session command storage, remembered session normalization, active-session persistence endpoint, resume endpoint, and tests for pure helpers.
- Create `apps/desktop/electron/workspaceMemory.ts`
  - App-level last workspace memory read/write helpers with injectable base directory for tests.
- Modify `apps/desktop/electron/main.ts`
  - Start sidecar in the remembered workspace when valid, persist workspace choices, and expose initial workspace IPC.
- Modify `apps/desktop/electron/preload.ts`
  - Expose `getInitialWorkspace`.
- Modify `apps/desktop/src/api.ts`
  - Add session memory/resume types and API functions.
- Modify `apps/desktop/src/App.tsx`
  - Load initial workspace, persist active session, call resume endpoint, and pass resume handler to session/detail components.
- Modify `apps/desktop/src/components/SessionListPanel.tsx`
  - Mark remembered/resumable sessions as not live.
- Modify `apps/desktop/src/components/SessionDetailPanel.tsx`
  - Show resume state/strategy and a Resume button when enabled.
- Modify `apps/desktop/src/App.css`
  - Add small, restrained styles for remembered/resumable state.
- Modify tests under `apps/desktop/tests/`
  - API type coverage, Electron workspace memory tests, and UI source-level coverage for resume affordance.

---

### Task 1: Backend Harness Adapter Core

**Files:**
- Create: `crates/orkworksd/src/harness.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/harness.rs`

- [ ] **Step 1: Write failing tests for resume strategy and command templates**

Create `crates/orkworksd/src/harness.rs` with only these tests and the minimum type stubs needed to compile the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn caps() -> HarnessCapabilities {
        HarnessCapabilities {
            launch: true,
            resume_exact: true,
            resume_latest_in_cwd: true,
            resume_latest_in_repo: true,
            detect_session_id: true,
            detect_model: true,
            detect_context_usage: true,
            detect_capacity: true,
            native_voice: false,
        }
    }

    #[test]
    fn exact_resume_wins_when_session_id_exists() {
        let memory = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("sess-123".into()),
            latest_fallback: true,
            last_seen_at: Some("2026-06-17T12:00:00Z".into()),
        };

        assert_eq!(select_resume_strategy(&memory, &caps()), ResumeStrategy::Exact);
    }

    #[test]
    fn latest_cwd_is_fallback_without_exact_id() {
        let mut capabilities = caps();
        capabilities.resume_exact = true;
        let memory = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: None,
        };

        assert_eq!(
            select_resume_strategy(&memory, &capabilities),
            ResumeStrategy::LatestCwd,
        );
    }

    #[test]
    fn unsupported_resume_returns_none_strategy() {
        let mut capabilities = caps();
        capabilities.resume_exact = false;
        capabilities.resume_latest_in_cwd = false;
        capabilities.resume_latest_in_repo = false;
        let memory = ResumeMemory {
            state: ResumeState::Unavailable,
            preferred_strategy: ResumeStrategy::None,
            harness_session_id: Some("sess-123".into()),
            latest_fallback: false,
            last_seen_at: None,
        };

        assert_eq!(select_resume_strategy(&memory, &capabilities), ResumeStrategy::None);
    }

    #[test]
    fn template_adapter_builds_exact_resume_command() {
        let adapter = HarnessAdapter::template(
            "custom",
            "Custom Harness",
            HarnessCapabilities {
                launch: true,
                resume_exact: true,
                resume_latest_in_cwd: true,
                resume_latest_in_repo: false,
                detect_session_id: false,
                detect_model: false,
                detect_context_usage: false,
                detect_capacity: false,
                native_voice: false,
            },
            CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--resume".into(), "{harnessSessionId}".into()],
            },
            Some(CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--continue".into(), "--cwd".into(), "{cwd}".into()],
            }),
            None,
        );
        let request = ResumeRequest {
            strategy: ResumeStrategy::Exact,
            cwd: "/repo".into(),
            repo_root: Some("/repo".into()),
            harness_session_id: Some("sess-123".into()),
            model: Some("model-a".into()),
        };

        let command = adapter.build_resume_command(&request).unwrap();

        assert_eq!(command.program, "custom-ai");
        assert_eq!(command.args, vec!["--resume", "sess-123"]);
        assert_eq!(command.cwd, "/repo");
    }

    #[test]
    fn template_adapter_builds_launch_command() {
        let adapter = HarnessAdapter::template(
            "custom",
            "Custom Harness",
            HarnessCapabilities {
                launch: true,
                resume_exact: false,
                resume_latest_in_cwd: false,
                resume_latest_in_repo: false,
                detect_session_id: false,
                detect_model: false,
                detect_context_usage: false,
                detect_capacity: false,
                native_voice: false,
            },
            CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--model".into(), "{model}".into()],
            },
            None,
            None,
        );

        let command = adapter.build_launch_command(&LaunchRequest {
            cwd: "/repo".into(),
            model: Some("model-a".into()),
        });

        assert_eq!(command.program, "custom-ai");
        assert_eq!(command.args, vec!["--model", "model-a"]);
        assert_eq!(command.cwd, "/repo");
    }

    #[test]
    fn adapter_config_creates_template_adapter_without_code_changes() {
        let config = HarnessAdapterConfig {
            id: "custom".into(),
            display_name: "Custom Harness".into(),
            capabilities: caps(),
            launch: CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--run".into()],
            },
            resume_exact: Some(CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--resume".into(), "{harnessSessionId}".into()],
            }),
            resume_latest_cwd: None,
            resume_latest_repo: None,
        };

        let adapter = HarnessAdapter::from_config(config);

        assert_eq!(adapter.id, "custom");
        assert_eq!(adapter.display_name, "Custom Harness");
        assert!(adapter.capabilities.resume_exact);
    }
}
```

- [ ] **Step 2: Run the failing backend harness test**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml harness
```

Expected: FAIL with unresolved types/functions such as `HarnessCapabilities`, `ResumeMemory`, and `select_resume_strategy`.

- [ ] **Step 3: Implement `harness.rs`**

Replace `crates/orkworksd/src/harness.rs` with:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandTemplate {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarnessAdapterConfig {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub capabilities: HarnessCapabilities,
    pub launch: CommandTemplate,
    #[serde(rename = "resumeExact", skip_serializing_if = "Option::is_none")]
    pub resume_exact: Option<CommandTemplate>,
    #[serde(rename = "resumeLatestCwd", skip_serializing_if = "Option::is_none")]
    pub resume_latest_cwd: Option<CommandTemplate>,
    #[serde(rename = "resumeLatestRepo", skip_serializing_if = "Option::is_none")]
    pub resume_latest_repo: Option<CommandTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarnessCapabilities {
    pub launch: bool,
    pub resume_exact: bool,
    pub resume_latest_in_cwd: bool,
    pub resume_latest_in_repo: bool,
    pub detect_session_id: bool,
    pub detect_model: bool,
    pub detect_context_usage: bool,
    pub detect_capacity: bool,
    pub native_voice: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResumeState {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResumeStrategy {
    Exact,
    LatestCwd,
    LatestRepo,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeMemory {
    pub state: ResumeState,
    #[serde(rename = "preferredStrategy")]
    pub preferred_strategy: ResumeStrategy,
    #[serde(rename = "harnessSessionId", skip_serializing_if = "Option::is_none")]
    pub harness_session_id: Option<String>,
    #[serde(rename = "latestFallback")]
    pub latest_fallback: bool,
    #[serde(rename = "lastSeenAt", skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeRequest {
    pub strategy: ResumeStrategy,
    pub cwd: String,
    pub repo_root: Option<String>,
    pub harness_session_id: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchRequest {
    pub cwd: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HarnessAdapter {
    pub id: String,
    pub display_name: String,
    pub capabilities: HarnessCapabilities,
    launch_template: CommandTemplate,
    exact_resume_template: Option<CommandTemplate>,
    latest_cwd_resume_template: Option<CommandTemplate>,
    latest_repo_resume_template: Option<CommandTemplate>,
}

impl HarnessAdapter {
    pub fn from_config(config: HarnessAdapterConfig) -> Self {
        Self {
            id: config.id,
            display_name: config.display_name,
            capabilities: config.capabilities,
            launch_template: config.launch,
            exact_resume_template: config.resume_exact,
            latest_cwd_resume_template: config.resume_latest_cwd,
            latest_repo_resume_template: config.resume_latest_repo,
        }
    }

    pub fn template(
        id: impl Into<String>,
        display_name: impl Into<String>,
        capabilities: HarnessCapabilities,
        launch_template: CommandTemplate,
        exact_resume_template: Option<CommandTemplate>,
        latest_cwd_resume_template: Option<CommandTemplate>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            capabilities,
            launch_template,
            exact_resume_template,
            latest_cwd_resume_template,
            latest_repo_resume_template: None,
        }
    }

    pub fn build_resume_command(&self, request: &ResumeRequest) -> Option<CommandSpec> {
        let template = match request.strategy {
            ResumeStrategy::Exact => self.exact_resume_template.as_ref()?,
            ResumeStrategy::LatestCwd => self.latest_cwd_resume_template.as_ref()?,
            ResumeStrategy::LatestRepo => self.latest_repo_resume_template.as_ref()?,
            ResumeStrategy::None => return None,
        };
        Some(render_template(template, request))
    }

    pub fn build_launch_command(&self, request: &LaunchRequest) -> CommandSpec {
        render_launch_template(&self.launch_template, request)
    }
}

pub fn select_resume_strategy(memory: &ResumeMemory, capabilities: &HarnessCapabilities) -> ResumeStrategy {
    if memory.state != ResumeState::Available {
        return ResumeStrategy::None;
    }
    if capabilities.resume_exact && memory.harness_session_id.is_some() {
        return ResumeStrategy::Exact;
    }
    if memory.latest_fallback && capabilities.resume_latest_in_cwd {
        return ResumeStrategy::LatestCwd;
    }
    if memory.latest_fallback && capabilities.resume_latest_in_repo {
        return ResumeStrategy::LatestRepo;
    }
    ResumeStrategy::None
}

fn render_template(template: &CommandTemplate, request: &ResumeRequest) -> CommandSpec {
    let session_id = request.harness_session_id.as_deref().unwrap_or("");
    let repo_root = request.repo_root.as_deref().unwrap_or(&request.cwd);
    let model = request.model.as_deref().unwrap_or("");
    let args = template
        .args
        .iter()
        .map(|arg| {
            arg.replace("{harnessSessionId}", session_id)
                .replace("{cwd}", &request.cwd)
                .replace("{repoRoot}", repo_root)
                .replace("{model}", model)
        })
        .collect();

    CommandSpec {
        program: template.command.clone(),
        args,
        cwd: request.cwd.clone(),
    }
}

fn render_launch_template(template: &CommandTemplate, request: &LaunchRequest) -> CommandSpec {
    let model = request.model.as_deref().unwrap_or("");
    let args = template
        .args
        .iter()
        .map(|arg| arg.replace("{cwd}", &request.cwd).replace("{model}", model))
        .collect();

    CommandSpec {
        program: template.command.clone(),
        args,
        cwd: request.cwd.clone(),
    }
}
```

- [ ] **Step 4: Register the module**

Modify `crates/orkworksd/src/main.rs` near the existing module declarations:

```rust
mod git;
mod harness;
mod metadata;
mod peon;
mod watcher;
```

- [ ] **Step 5: Run the backend harness tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml harness
```

Expected: PASS with `test result: ok`.

- [ ] **Step 6: Commit Task 1**

```bash
git add crates/orkworksd/src/harness.rs crates/orkworksd/src/main.rs
git commit -m "feat: add harness adapter core"
```

---

### Task 2: Session and Workspace Memory Metadata

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`
- Test: `crates/orkworksd/src/metadata.rs`

- [ ] **Step 1: Write failing metadata tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `crates/orkworksd/src/metadata.rs`:

```rust
#[test]
fn write_and_read_workspace_memory() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());

    store.write_workspace_memory(&WorkspaceMemory {
        last_active_session_id: Some("session-1".into()),
        last_active_at: Some("2026-06-17T12:00:00Z".into()),
    });

    let memory = store.read_workspace_memory().unwrap();
    assert_eq!(memory.last_active_session_id.as_deref(), Some("session-1"));
    assert_eq!(memory.last_active_at.as_deref(), Some("2026-06-17T12:00:00Z"));
}

#[test]
fn read_all_sessions_includes_resume_memory() {
    let dir = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(dir.path());
    let mut meta = test_metadata("remembered");
    meta.resume = Some(crate::harness::ResumeMemory {
        state: crate::harness::ResumeState::Available,
        preferred_strategy: crate::harness::ResumeStrategy::Exact,
        harness_session_id: Some("sess-abc".into()),
        latest_fallback: true,
        last_seen_at: Some("2026-06-17T12:00:00Z".into()),
    });
    store.write_session(&meta);

    let all = store.read_all_sessions();

    assert_eq!(all.len(), 1);
    assert_eq!(
        all[0].resume.as_ref().and_then(|r| r.harness_session_id.as_deref()),
        Some("sess-abc"),
    );
}
```

Also add this helper inside the same test module:

```rust
fn test_metadata(id: &str) -> SessionMetadata {
    SessionMetadata {
        id: id.into(),
        label: "Test".into(),
        workspace: "/tmp".into(),
        task: "".into(),
        harness: "".into(),
        model: "".into(),
        cwd: "/tmp".into(),
        status: "running".into(),
        phase: "".into(),
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
        created_at: "now".into(),
        last_activity: "now".into(),
        metadata_source: "process".into(),
        metadata_confidence: 1.0,
        repo_root: None,
        branch: None,
        dirty: None,
        changed_files: None,
        is_worktree: None,
        resume: None,
        resumed_from: None,
    }
}
```

- [ ] **Step 2: Run failing metadata tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml metadata
```

Expected: FAIL with missing `WorkspaceMemory`, `resume`, `resumed_from`, and `read_all_sessions`.

- [ ] **Step 3: Add metadata fields and methods**

Modify `crates/orkworksd/src/metadata.rs`:

```rust
use crate::harness::ResumeMemory;
```

Extend `SessionMetadata`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub resume: Option<ResumeMemory>,
#[serde(rename = "resumedFrom", skip_serializing_if = "Option::is_none")]
pub resumed_from: Option<String>,
```

Add below `Event`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceMemory {
    #[serde(rename = "lastActiveSessionId", skip_serializing_if = "Option::is_none")]
    pub last_active_session_id: Option<String>,
    #[serde(rename = "lastActiveAt", skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<String>,
}
```

Add methods to `impl MetadataStore`:

```rust
pub fn workspace_memory_path(&self) -> PathBuf {
    self.root.join("workspace.json")
}

pub fn read_workspace_memory(&self) -> Option<WorkspaceMemory> {
    let data = fs::read_to_string(self.workspace_memory_path()).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn write_workspace_memory(&self, memory: &WorkspaceMemory) {
    if let Err(e) = fs::create_dir_all(&self.root) {
        warn!("failed to create metadata root {:?}: {e}", self.root);
        return;
    }
    let path = self.workspace_memory_path();
    match serde_json::to_string_pretty(memory) {
        Ok(json) => {
            if let Err(e) = fs::write(&path, json) {
                warn!("failed to write workspace memory {:?}: {e}", path);
            }
        }
        Err(e) => warn!("failed to serialize workspace memory: {e}"),
    }
}

pub fn read_all_sessions(&self) -> Vec<SessionMetadata> {
    let dir = self.sessions_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return vec![],
    };
    let mut sessions: Vec<SessionMetadata> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("json"))
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|data| serde_json::from_str::<SessionMetadata>(&data).ok())
        .collect();
    sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    sessions
}
```

Update every test `SessionMetadata` literal in this file to include:

```rust
resume: None,
resumed_from: None,
```

- [ ] **Step 4: Run metadata tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml metadata
```

Expected: PASS with `test result: ok`.

- [ ] **Step 5: Commit Task 2**

```bash
git add crates/orkworksd/src/metadata.rs
git commit -m "feat: persist session and workspace memory"
```

---

### Task 3: Backend Remembered Sessions and Resume Endpoint

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Write failing tests for normalized memory state**

Add these types and tests inside `#[cfg(test)] mod tests` in `crates/orkworksd/src/main.rs` after existing tests:

```rust
#[test]
fn memory_state_marks_absent_session_as_resumable_when_strategy_exists() {
    let caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: true,
        resume_latest_in_cwd: true,
        resume_latest_in_repo: false,
        detect_session_id: true,
        detect_model: true,
        detect_context_usage: false,
        detect_capacity: false,
        native_voice: false,
    };
    let resume = harness::ResumeMemory {
        state: harness::ResumeState::Available,
        preferred_strategy: harness::ResumeStrategy::Exact,
        harness_session_id: Some("sess-1".into()),
        latest_fallback: true,
        last_seen_at: None,
    };

    let (memory_state, strategy) = derive_memory_state(false, Some(&resume), &caps);

    assert_eq!(memory_state, MemoryState::Resumable);
    assert_eq!(strategy, harness::ResumeStrategy::Exact);
}

#[test]
fn memory_state_marks_active_session_as_live() {
    let caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: false,
        resume_latest_in_cwd: false,
        resume_latest_in_repo: false,
        detect_session_id: false,
        detect_model: false,
        detect_context_usage: false,
        detect_capacity: false,
        native_voice: false,
    };

    let (memory_state, strategy) = derive_memory_state(true, None, &caps);

    assert_eq!(memory_state, MemoryState::Live);
    assert_eq!(strategy, harness::ResumeStrategy::None);
}
```

- [ ] **Step 2: Run failing main tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml memory_state
```

Expected: FAIL with missing `MemoryState` and `derive_memory_state`.

- [ ] **Step 3: Add response fields and command storage**

Modify `crates/orkworksd/src/main.rs`:

```rust
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MemoryState {
    Live,
    Remembered,
    Resumable,
    Unsupported,
}
```

Extend `SessionInfo`:

```rust
#[serde(rename = "memoryState")]
memory_state: MemoryState,
#[serde(rename = "resumeStrategy")]
resume_strategy: harness::ResumeStrategy,
#[serde(skip_serializing_if = "Option::is_none")]
resume: Option<harness::ResumeMemory>,
#[serde(rename = "resumedFrom", skip_serializing_if = "Option::is_none")]
resumed_from: Option<String>,
```

Extend `SessionHandle`:

```rust
command: harness::CommandSpec,
```

Add helper:

```rust
fn default_shell_command(cwd: String) -> harness::CommandSpec {
    let (program, args) = shell_cmd();
    harness::CommandSpec { program, args, cwd }
}

fn default_capabilities() -> harness::HarnessCapabilities {
    harness::HarnessCapabilities {
        launch: true,
        resume_exact: false,
        resume_latest_in_cwd: false,
        resume_latest_in_repo: false,
        detect_session_id: false,
        detect_model: false,
        detect_context_usage: false,
        detect_capacity: false,
        native_voice: false,
    }
}

fn default_shell_adapter() -> harness::HarnessAdapter {
    let (program, args) = shell_cmd();
    let capabilities = default_capabilities();
    harness::HarnessAdapter::template(
        "generic-shell",
        "Generic Shell",
        capabilities,
        harness::CommandTemplate {
            command: program.clone(),
            args: args.clone(),
        },
        None,
        None,
    )
}

fn derive_memory_state(
    is_live: bool,
    resume: Option<&harness::ResumeMemory>,
    capabilities: &harness::HarnessCapabilities,
) -> (MemoryState, harness::ResumeStrategy) {
    if is_live {
        return (MemoryState::Live, harness::ResumeStrategy::None);
    }
    let Some(resume) = resume else {
        return (MemoryState::Remembered, harness::ResumeStrategy::None);
    };
    let strategy = harness::select_resume_strategy(resume, capabilities);
    if strategy == harness::ResumeStrategy::None {
        (MemoryState::Unsupported, strategy)
    } else {
        (MemoryState::Resumable, strategy)
    }
}
```

When constructing new `SessionInfo` in `create_session`, set:

```rust
memory_state: MemoryState::Live,
resume_strategy: harness::ResumeStrategy::None,
resume: Some(harness::ResumeMemory {
    state: harness::ResumeState::Available,
    preferred_strategy: harness::ResumeStrategy::LatestCwd,
    harness_session_id: None,
    latest_fallback: true,
    last_seen_at: Some(now.clone()),
}),
resumed_from: None,
```

When creating `SessionHandle`, set:

```rust
command: default_shell_command(info.cwd.clone()),
```

Update every `SessionInfo` literal in `list_sessions` to fill the new fields from metadata and `derive_memory_state`.

- [ ] **Step 4: Use stored command when spawning PTY**

In `handle_session_terminal`, replace the `shell_cmd()` command construction block with:

```rust
let command = {
    let sessions = state.sessions.lock().unwrap();
    sessions
        .get(&id)
        .map(|h| h.command.clone())
        .unwrap_or_else(|| default_shell_command(cwd.clone()))
};

let mut cmd = CommandBuilder::new(&command.program);
cmd.args(&command.args);
cmd.cwd(&command.cwd);
```

Keep the environment forwarding and terminal environment override code unchanged after this block.

- [ ] **Step 5: Add active session and resume routes**

Add request type:

```rust
#[derive(Deserialize)]
struct ActiveSessionRequest {
    #[serde(rename = "sessionId")]
    session_id: String,
}
```

Register routes:

```rust
.route("/workspace/active-session", post(set_active_session))
.route("/sessions/:id/resume", post(resume_session))
```

Add handlers:

```rust
async fn set_active_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActiveSessionRequest>,
) -> impl IntoResponse {
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        ws.metadata.write_workspace_memory(&metadata::WorkspaceMemory {
            last_active_session_id: Some(req.session_id),
            last_active_at: Some(now),
        });
        return axum::http::StatusCode::OK;
    }
    axum::http::StatusCode::CONFLICT
}

async fn resume_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let now = iso_now();
    let (meta, command, strategy) = {
        let ws_guard = state.workspace.lock().unwrap();
        let Some(ref ws) = *ws_guard else {
            return axum::http::StatusCode::CONFLICT.into_response();
        };
        let Some(meta) = ws.metadata.read_session(&id) else {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        };
        let Some(resume) = meta.resume.as_ref() else {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        };
        let capabilities = default_capabilities();
        let strategy = harness::select_resume_strategy(resume, &capabilities);
        if strategy == harness::ResumeStrategy::None {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        }
        let adapter = default_shell_adapter();
        let request = harness::ResumeRequest {
            strategy: strategy.clone(),
            cwd: meta.cwd.clone(),
            repo_root: meta.repo_root.clone(),
            harness_session_id: resume.harness_session_id.clone(),
            model: (!meta.model.is_empty()).then(|| meta.model.clone()),
        };
        let Some(command) = adapter.build_resume_command(&request) else {
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        };
        (meta, command, strategy)
    };

    let new_id = uuid::Uuid::new_v4().to_string();
    let (kill_tx, _kill_rx) = tokio::sync::watch::channel(false);
    let info = SessionInfo {
        id: new_id.clone(),
        label: format!("{} resumed", meta.label),
        harness: (!meta.harness.is_empty()).then(|| meta.harness.clone()),
        model: (!meta.model.is_empty()).then(|| meta.model.clone()),
        status: "creating".into(),
        cwd: command.cwd.clone(),
        created_at: now.clone(),
        observed_status: None,
        summary: meta.summary.clone(),
        next_action: meta.next_action.clone(),
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        metadata_source: Some("process".into()),
        metadata_confidence: Some(1.0),
        repo_root: meta.repo_root.clone(),
        branch: meta.branch.clone(),
        dirty: meta.dirty,
        changed_files: meta.changed_files,
        is_worktree: meta.is_worktree,
        conflict_warning: None,
        recommendation: None,
        peon_last_inference: None,
        memory_state: MemoryState::Live,
        resume_strategy: strategy,
        resume: meta.resume.clone(),
        resumed_from: Some(id.clone()),
    };

    state.sessions.lock().unwrap().insert(
        new_id.clone(),
        SessionHandle {
            info: info.clone(),
            kill_tx,
            output_buffer: peon::RingBuffer::new(state.peon.config.max_lines),
            command,
        },
    );

    Json(info).into_response()
}
```

The generic shell adapter is launch-only. It must not advertise exact or latest resume support because a shell restart is not a harness continuation mechanism. Task 1's `HarnessAdapterConfig` keeps custom command templates data-driven; wiring repo-local harness config can enable resume only after the harness-specific command flags are verified.

- [ ] **Step 6: Include remembered sessions in `list_sessions`**

After building live `infos`, read all metadata sessions from `ws.metadata.read_all_sessions()`. For each metadata session id not in the live session id set, append a `SessionInfo` with:

```rust
let capabilities = default_capabilities();
let (memory_state, resume_strategy) =
    derive_memory_state(false, meta.resume.as_ref(), &capabilities);
status: "ended".into(),
memory_state,
resume_strategy,
resume: meta.resume.clone(),
resumed_from: meta.resumed_from.clone(),
```

Copy metadata fields the same way the live-session mapping already copies label, harness, model, observed status, summary, repo root, branch, dirty, changed file count, and worktree flag.

- [ ] **Step 7: Run backend tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: PASS with `test result: ok`.

- [ ] **Step 8: Commit Task 3**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "feat: expose remembered sessions and resume endpoint"
```

---

### Task 4: Electron App-Level Workspace Memory

**Files:**
- Create: `apps/desktop/electron/workspaceMemory.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Test: `apps/desktop/tests/electronWorkspaceMemory.test.ts`

- [ ] **Step 1: Write failing workspace memory tests**

Create `apps/desktop/tests/electronWorkspaceMemory.test.ts`:

```ts
import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  readWorkspaceMemory,
  writeWorkspaceMemory,
  rememberWorkspacePath,
} from "../electron/workspaceMemory.ts";

test("workspace memory round-trips last workspace and recent paths", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-memory-"));
  try {
    writeWorkspaceMemory(dir, {
      lastWorkspacePath: "/repo/a",
      recentWorkspacePaths: ["/repo/a"],
    });

    const memory = readWorkspaceMemory(dir);

    assert.equal(memory.lastWorkspacePath, "/repo/a");
    assert.deepEqual(memory.recentWorkspacePaths, ["/repo/a"]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("rememberWorkspacePath deduplicates and keeps newest first", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-memory-"));
  try {
    rememberWorkspacePath(dir, "/repo/a");
    rememberWorkspacePath(dir, "/repo/b");
    rememberWorkspacePath(dir, "/repo/a");

    const memory = readWorkspaceMemory(dir);

    assert.equal(memory.lastWorkspacePath, "/repo/a");
    assert.deepEqual(memory.recentWorkspacePaths, ["/repo/a", "/repo/b"]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
```

- [ ] **Step 2: Run failing Electron memory tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/electronWorkspaceMemory.test.ts
```

Expected: FAIL because `electron/workspaceMemory.ts` does not exist.

- [ ] **Step 3: Implement `workspaceMemory.ts`**

Create `apps/desktop/electron/workspaceMemory.ts`:

```ts
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export interface AppWorkspaceMemory {
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
}

const fileName = "workspace-memory.json";

export function workspaceMemoryPath(userDataPath: string): string {
  return join(userDataPath, fileName);
}

export function readWorkspaceMemory(userDataPath: string): AppWorkspaceMemory {
  const path = workspaceMemoryPath(userDataPath);
  if (!existsSync(path)) {
    return { lastWorkspacePath: null, recentWorkspacePaths: [] };
  }
  try {
    const parsed = JSON.parse(readFileSync(path, "utf8")) as Partial<AppWorkspaceMemory>;
    return {
      lastWorkspacePath: typeof parsed.lastWorkspacePath === "string" ? parsed.lastWorkspacePath : null,
      recentWorkspacePaths: Array.isArray(parsed.recentWorkspacePaths)
        ? parsed.recentWorkspacePaths.filter((item): item is string => typeof item === "string")
        : [],
    };
  } catch {
    return { lastWorkspacePath: null, recentWorkspacePaths: [] };
  }
}

export function writeWorkspaceMemory(userDataPath: string, memory: AppWorkspaceMemory): void {
  mkdirSync(userDataPath, { recursive: true });
  writeFileSync(workspaceMemoryPath(userDataPath), JSON.stringify(memory, null, 2));
}

export function rememberWorkspacePath(userDataPath: string, workspacePath: string): AppWorkspaceMemory {
  const current = readWorkspaceMemory(userDataPath);
  const recentWorkspacePaths = [
    workspacePath,
    ...current.recentWorkspacePaths.filter((path) => path !== workspacePath),
  ].slice(0, 10);
  const next = { lastWorkspacePath: workspacePath, recentWorkspacePaths };
  writeWorkspaceMemory(userDataPath, next);
  return next;
}
```

- [ ] **Step 4: Wire Electron startup memory**

Modify `apps/desktop/electron/main.ts` imports:

```ts
import { existsSync } from "fs";
import { readWorkspaceMemory, rememberWorkspacePath } from "./workspaceMemory";
```

Change `startSidecar` to accept an optional cwd:

```ts
function startSidecar(cwdOverride?: string): void {
  const binaryPath = getSidecarPath();
  const sidecarCwd = cwdOverride ?? (app.isPackaged ? app.getPath("home") : getDevRepoRoot(__dirname));
```

Inside `app.whenReady().then(() => {`, compute:

```ts
const appMemory = readWorkspaceMemory(app.getPath("userData"));
const initialWorkspacePath =
  appMemory.lastWorkspacePath && existsSync(appMemory.lastWorkspacePath)
    ? appMemory.lastWorkspacePath
    : null;
```

Add IPC:

```ts
ipcMain.handle("get-initial-workspace", async () => {
  if (!initialWorkspacePath) return null;
  const port = await portPromise;
  const resp = await fetch(`http://127.0.0.1:${port}/workspace`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path: initialWorkspacePath }),
  });
  if (!resp.ok) return null;
  return resp.json();
});
```

After successful `open-workspace` POST, persist:

```ts
rememberWorkspacePath(app.getPath("userData"), dirPath);
```

Start the sidecar with:

```ts
startSidecar(initialWorkspacePath ?? undefined);
```

- [ ] **Step 5: Update preload**

Modify `apps/desktop/electron/preload.ts`:

```ts
contextBridge.exposeInMainWorld("orkworks", {
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  getInitialWorkspace: (): Promise<unknown> => ipcRenderer.invoke("get-initial-workspace"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
});
```

- [ ] **Step 6: Run Electron memory tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/electronWorkspaceMemory.test.ts
```

Expected: PASS.

- [ ] **Step 7: Commit Task 4**

```bash
git add apps/desktop/electron/workspaceMemory.ts apps/desktop/electron/main.ts apps/desktop/electron/preload.ts apps/desktop/tests/electronWorkspaceMemory.test.ts
git commit -m "feat: remember last workspace in electron"
```

---

### Task 5: Frontend API and App State

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/App.tsx`
- Test: `apps/desktop/tests/api.test.ts`

- [ ] **Step 1: Extend API type test first**

Modify the `SessionInfo type accepts metadata fields` object in `apps/desktop/tests/api.test.ts`:

```ts
memoryState: "resumable",
resumeStrategy: "exact",
resume: {
  state: "available",
  preferredStrategy: "exact",
  harnessSessionId: "sess-123",
  latestFallback: true,
  lastSeenAt: "2026-06-17T12:00:00Z",
},
resumedFrom: "older-session",
```

Add assertions:

```ts
assert.equal(session.memoryState, "resumable");
assert.equal(session.resumeStrategy, "exact");
assert.equal(session.resume?.harnessSessionId, "sess-123");
assert.equal(session.resumedFrom, "older-session");
```

- [ ] **Step 2: Run failing API type test**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts
```

Expected: FAIL because `SessionInfo` lacks memory fields.

- [ ] **Step 3: Add frontend API types and functions**

Modify `apps/desktop/src/api.ts`:

```ts
export type MemoryState = "live" | "remembered" | "resumable" | "unsupported";
export type ResumeStrategy = "exact" | "latest_cwd" | "latest_repo" | "none";

export interface ResumeMemory {
  state: "available" | "unavailable";
  preferredStrategy: ResumeStrategy;
  harnessSessionId?: string;
  latestFallback: boolean;
  lastSeenAt?: string;
}
```

Extend `SessionInfo`:

```ts
memoryState: MemoryState;
resumeStrategy: ResumeStrategy;
resume?: ResumeMemory;
resumedFrom?: string;
```

Add API functions:

```ts
export async function setActiveWorkspaceSession(
  baseUrl: string,
  sessionId: string,
): Promise<void> {
  const resp = await fetch(`${baseUrl}/workspace/active-session`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ sessionId }),
  });
  if (!resp.ok) throw new Error(`set active session failed: ${resp.status}`);
}

export async function resumeSession(
  baseUrl: string,
  id: string,
): Promise<SessionInfo> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/resume`, {
    method: "POST",
  });
  if (!resp.ok) throw new Error(`resume session failed: ${resp.status}`);
  return resp.json();
}
```

- [ ] **Step 4: Wire app state**

Modify imports in `apps/desktop/src/App.tsx`:

```ts
import { useCallback, useEffect, useRef, useState } from "react";
```

Import new API functions:

```ts
resumeSession,
setActiveWorkspaceSession,
```

Extend the global window type:

```ts
getInitialWorkspace: () => Promise<WorkspaceInfo | null>;
```

After backend connects, load initial workspace:

```ts
useEffect(() => {
  if (backendStatus !== "connected" || workspace) return;
  let cancelled = false;
  async function loadInitialWorkspace() {
    const info = await window.orkworks.getInitialWorkspace();
    if (!cancelled && info) {
      setWorkspaceState(info);
      await refreshSessions();
    }
  }
  loadInitialWorkspace();
  return () => {
    cancelled = true;
  };
}, [backendStatus, refreshSessions, workspace]);
```

Persist active session:

```ts
useEffect(() => {
  if (backendStatus !== "connected" || !activeSessionId) return;
  async function persistActiveSession() {
    const baseUrl = await window.orkworks.getBackendUrl();
    await setActiveWorkspaceSession(baseUrl, activeSessionId);
  }
  persistActiveSession().catch(() => {
    /* backend not ready */
  });
}, [activeSessionId, backendStatus]);
```

Add handler:

```ts
const handleResumeSession = useCallback(async (id: string) => {
  const baseUrl = await window.orkworks.getBackendUrl();
  const session = await resumeSession(baseUrl, id);
  setSessions((prev) => [...prev, session]);
  setActiveSessionId(session.id);
}, []);
```

Pass `onResumeSession={handleResumeSession}` to `DockviewApp`, add it to `DockviewAppData`, include it in `ctxValue`, and pass `ctx.onResumeSession` from `DetailPanel` into `SessionDetailPanel`.

- [ ] **Step 5: Run frontend API tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit Task 5**

```bash
git add apps/desktop/src/api.ts apps/desktop/src/App.tsx apps/desktop/tests/api.test.ts
git commit -m "feat: wire resumable session API"
```

---

### Task 6: Frontend Resumable Session UI

**Files:**
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/App.css`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Add source-level UI tests**

Add to `apps/desktop/tests/dockview.test.ts`:

```ts
test("session detail exposes resumable session action", () => {
  const source = readFileSync(
    new URL("../src/components/SessionDetailPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /onResumeSession/);
  assert.match(source, /Resume/);
  assert.match(source, /resumeStrategy/);
});

test("session list marks remembered sessions separately from live sessions", () => {
  const source = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /memoryState/);
  assert.match(source, /session-item--remembered/);
});
```

- [ ] **Step 2: Run failing UI tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts tests/rightSidebar.test.ts
```

Expected: FAIL because the resume UI strings and props are absent.

- [ ] **Step 3: Update session detail component**

Modify `apps/desktop/src/components/SessionDetailPanel.tsx` props:

```ts
interface SessionDetailPanelProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onResumeSession: (id: string) => void;
}
```

Add derived state:

```ts
const active = sessions.find((s) => s.id === activeSessionId);
const canResume = active?.memoryState === "resumable" && active.resumeStrategy !== "none";
const resumeLabel =
  active?.resumeStrategy === "exact"
    ? "Resume exact session"
    : active?.resumeStrategy === "latest_cwd"
      ? "Resume latest in folder"
      : active?.resumeStrategy === "latest_repo"
        ? "Resume latest in repo"
        : "Resume unavailable";
```

Add action markup in the detail panel action area:

```tsx
{active && (
  <button
    className="session-resume-button"
    type="button"
    disabled={!canResume}
    onClick={() => onResumeSession(active.id)}
    title={resumeLabel}
  >
    {resumeLabel}
  </button>
)}
```

Add memory details near status/source:

```tsx
{active && (
  <div className="detail-row">
    <span className="detail-label">Memory</span>
    <span className="detail-value">
      {active.memoryState} · {active.resumeStrategy}
    </span>
  </div>
)}
```

- [ ] **Step 4: Wire resume through Dockview**

Modify `apps/desktop/src/components/DockviewApp.tsx`:

```ts
interface DockviewAppData {
  backendStatus: string;
  workspace: WorkspaceInfo | null;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onOpenWorkspace: () => void;
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
  onResumeSession: (id: string) => void;
}
```

Update `DetailPanel`:

```tsx
function DetailPanel() {
  const ctx = useContext(DockviewContext);
  return (
    <SessionDetailPanel
      sessions={ctx.sessions}
      activeSessionId={ctx.activeSessionId}
      onResumeSession={ctx.onResumeSession}
    />
  );
}
```

Update the `DockviewApp` destructure and `ctxValue` so `onResumeSession` is preserved:

```ts
const {
  backendStatus,
  workspace,
  sessions,
  activeSessionId,
  onOpenWorkspace,
  onSelectSession,
  onCreateSession,
  onKillSession,
  onResumeSession,
} = props;

const ctxValue: DockviewAppData = {
  backendStatus,
  workspace,
  sessions,
  activeSessionId,
  onOpenWorkspace,
  onSelectSession,
  onCreateSession,
  onKillSession,
  onResumeSession,
};
```

- [ ] **Step 5: Update session list component**

In `apps/desktop/src/components/SessionListPanel.tsx`, include memory state in class names:

```tsx
className={[
  "session-item",
  session.id === activeSessionId ? "session-item--active" : "",
  session.memoryState !== "live" ? "session-item--remembered" : "",
  session.memoryState === "resumable" ? "session-item--resumable" : "",
].filter(Boolean).join(" ")}
```

Add a compact memory label inside each item:

```tsx
{session.memoryState !== "live" && (
  <span className="session-memory-badge">
    {session.memoryState === "resumable" ? "resumable" : "remembered"}
  </span>
)}
```

- [ ] **Step 6: Add CSS**

Modify `apps/desktop/src/App.css`:

```css
.session-item--remembered {
  opacity: 0.78;
}

.session-item--resumable {
  border-left-color: #6aa6ff;
}

.session-memory-badge {
  color: #9fb7d7;
  font-size: 11px;
  line-height: 1;
}

.session-resume-button {
  border: 1px solid #3d5f8f;
  background: #1d2f49;
  color: #e8f0ff;
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 12px;
}

.session-resume-button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}
```

- [ ] **Step 7: Run UI tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts tests/rightSidebar.test.ts
```

Expected: PASS.

- [ ] **Step 8: Commit Task 6**

```bash
git add apps/desktop/src/components/DockviewApp.tsx apps/desktop/src/components/SessionListPanel.tsx apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/src/App.css apps/desktop/tests/dockview.test.ts
git commit -m "feat: show resumable session state"
```

---

### Task 7: Full Verification and Documentation Check

**Files:**
- Modify: `README.md` or `AGENTS.md` only if implementation changes developer workflow, commands, runtime dependencies, directory architecture, or conventions.

- [ ] **Step 1: Run Rust tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: PASS with `test result: ok`.

- [ ] **Step 2: Run frontend tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: PASS for all test files.

- [ ] **Step 3: Run TypeScript type-check**

Run:

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: exit 0 with no TypeScript errors.

- [ ] **Step 4: Run repo doc currency check**

Run:

```bash
bash .claude/hooks/doc-check.sh
```

Expected: exit 0. If it lists docs that need updates, update those files and rerun this command.

- [ ] **Step 5: Review final diff**

Run:

```bash
git status --short
git diff --stat HEAD
```

Expected: only files touched by this plan are changed. Pre-existing unrelated worktree changes must remain separate.

- [ ] **Step 6: Commit verification/docs adjustments**

If Step 4 required doc changes:

```bash
git add README.md AGENTS.md docs/agents/architecture.md
git commit -m "docs: update resumable session memory notes"
```

If Step 4 required no doc changes, skip this commit.

---

## Self-Review

- Spec coverage: Task 1 covers the adapter interface, capabilities, command templates, and exact/latest strategy. Task 2 covers session and repo-local memory. Task 3 covers remembered session normalization and explicit resume. Task 4 covers app-level last workspace memory. Task 5 covers frontend API state. Task 6 covers UI distinction between live and remembered sessions. Task 7 covers verification and doc currency.
- Type consistency: Rust `ResumeStrategy` serializes as `exact`, `latest_cwd`, `latest_repo`, and `none`; frontend `ResumeStrategy` uses the same strings. Rust `ResumeMemory` field renames match frontend `ResumeMemory`.
- Scope control: The first implementation stores the interface and generic/template behavior. Verified built-in CLI-specific resume flags for Codex, Claude Code, OpenCode, Gemini CLI, and Aider are not hardcoded in this plan because the spec requires verification before hardcoding harness-specific commands.
