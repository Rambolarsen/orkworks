# Hermes Harness Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Hermes Agent as a built-in OrkWorks harness with documented launch support and truthful resume behavior for exact resume and global-latest continue.

**GitHub Issue:** [#57](https://github.com/Rambolarsen/orkworks/issues/57) `Add Hermes Agent built-in harness and documented resume support`

**Architecture:** The Rust sidecar remains the owner of harness definitions, resume strategy selection, and adapter command construction. Hermes launch stays in the existing built-in harness config path, while Hermes resume behavior is added through the existing adapter layer by extending the neutral resume model with a `latest_global` strategy instead of overloading cwd/repo-scoped resume semantics.

**Tech Stack:** Rust/Axum/serde for sidecar launch and resume behavior, React/TypeScript for session detail copy and typed API surfaces, Node built-in test runner and Cargo tests for verification.

---

## File Structure

- Modify `crates/orkworksd/src/harness.rs`
  - Extend neutral resume capabilities and strategy types with `latest_global`, add Hermes-aware command-template support, and cover the new semantics with unit tests.
- Modify `crates/orkworksd/src/main.rs`
  - Add Hermes to the built-in harness registry, wire a Hermes adapter into the sidecar registry, and extend remembered-session tests to cover Hermes launch and resume behavior.
- Modify `apps/desktop/src/api.ts`
  - Add `latest_global` to the typed resume strategy union used by the renderer.
- Modify `apps/desktop/src/labels.ts`
  - Add generic and Hermes-specific resume copy helpers for the new strategy.
- Modify `apps/desktop/src/components/SessionDetailPanel.tsx`
  - Use the new helper so the button copy distinguishes exact resume from “continue latest Hermes session.”
- Modify `apps/desktop/tests/api.test.ts`
  - Cover the new `latest_global` typed shape.
- Modify `apps/desktop/tests/labels.test.ts`
  - Cover the new resume label and Hermes-specific button/title copy.
- Modify `apps/desktop/tests/dockview.test.ts`
  - Keep the source-level panel assertion aligned with the new helper usage.

---

### Task 1: Extend The Neutral Resume Model For Global-Latest Continue

**Files:**
- Modify: `crates/orkworksd/src/harness.rs`
- Test: `crates/orkworksd/src/harness.rs`

- [ ] **Step 1: Write the failing backend tests for `latest_global`**

Add these tests to `crates/orkworksd/src/harness.rs` inside the existing `#[cfg(test)]` module:

```rust
#[test]
fn latest_global_is_selected_before_scoped_latest_when_exact_id_is_missing() {
    let capabilities = HarnessCapabilities {
        launch: true,
        resume_exact: true,
        resume_latest_global: true,
        resume_latest_in_cwd: true,
        resume_latest_in_repo: true,
        detect_session_id: true,
        detect_model: true,
        detect_context_usage: true,
        detect_capacity: true,
        native_voice: false,
    };
    let memory = ResumeMemory {
        state: ResumeState::Available,
        preferred_strategy: ResumeStrategy::LatestGlobal,
        harness_session_id: None,
        latest_fallback: true,
        last_seen_at: None,
    };

    assert_eq!(
        select_resume_strategy(&memory, &capabilities),
        ResumeStrategy::LatestGlobal,
    );
}

#[test]
fn template_adapter_builds_latest_global_resume_command() {
    let adapter = HarnessAdapter::template(
        "hermes",
        "Hermes Agent",
        HarnessCapabilities {
            launch: true,
            resume_exact: true,
            resume_latest_global: true,
            resume_latest_in_cwd: false,
            resume_latest_in_repo: false,
            detect_session_id: false,
            detect_model: false,
            detect_context_usage: false,
            detect_capacity: false,
            native_voice: false,
        },
        CommandTemplate {
            command: "hermes".into(),
            args: vec!["chat".into()],
        },
        Some(CommandTemplate {
            command: "hermes".into(),
            args: vec!["--resume".into(), "{harnessSessionId}".into()],
        }),
        Some(CommandTemplate {
            command: "hermes".into(),
            args: vec!["--continue".into()],
        }),
        None,
        None,
    );
    let request = ResumeRequest {
        strategy: ResumeStrategy::LatestGlobal,
        cwd: "/repo".into(),
        repo_root: Some("/repo".into()),
        harness_session_id: None,
        model: None,
    };

    let command = adapter.build_resume_command(&request).unwrap();

    assert_eq!(command.program, "hermes");
    assert_eq!(command.args, vec!["--continue"]);
    assert_eq!(command.cwd, "/repo");
}

#[test]
fn adapter_config_reads_resume_latest_global_template() {
    let config = HarnessAdapterConfig {
        id: "hermes".into(),
        display_name: "Hermes Agent".into(),
        capabilities: HarnessCapabilities {
            launch: true,
            resume_exact: true,
            resume_latest_global: true,
            resume_latest_in_cwd: false,
            resume_latest_in_repo: false,
            detect_session_id: false,
            detect_model: false,
            detect_context_usage: false,
            detect_capacity: false,
            native_voice: false,
        },
        launch: CommandTemplate {
            command: "hermes".into(),
            args: vec!["chat".into()],
        },
        resume_exact: Some(CommandTemplate {
            command: "hermes".into(),
            args: vec!["--resume".into(), "{harnessSessionId}".into()],
        }),
        resume_latest_global: Some(CommandTemplate {
            command: "hermes".into(),
            args: vec!["--continue".into()],
        }),
        resume_latest_cwd: None,
        resume_latest_repo: None,
    };

    let adapter = HarnessAdapter::from_config(config);

    assert!(adapter.capabilities.resume_latest_global);
    let command = adapter
        .build_resume_command(&ResumeRequest {
            strategy: ResumeStrategy::LatestGlobal,
            cwd: "/repo".into(),
            repo_root: Some("/repo".into()),
            harness_session_id: None,
            model: None,
        })
        .unwrap();
    assert_eq!(command.args, vec!["--continue"]);
}
```

- [ ] **Step 2: Run the harness unit tests to verify the new cases fail**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml latest_global
```

Expected: FAIL with unresolved fields or enum variants such as `resume_latest_global`, `ResumeStrategy::LatestGlobal`, or the expanded `HarnessAdapter::template(...)` signature.

- [ ] **Step 3: Implement `latest_global` support in `crates/orkworksd/src/harness.rs`**

Update the neutral types and adapter implementation to carry the new strategy explicitly:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarnessAdapterConfig {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub capabilities: HarnessCapabilities,
    pub launch: CommandTemplate,
    #[serde(rename = "resumeExact", skip_serializing_if = "Option::is_none")]
    pub resume_exact: Option<CommandTemplate>,
    #[serde(rename = "resumeLatestGlobal", skip_serializing_if = "Option::is_none")]
    pub resume_latest_global: Option<CommandTemplate>,
    #[serde(rename = "resumeLatestCwd", skip_serializing_if = "Option::is_none")]
    pub resume_latest_cwd: Option<CommandTemplate>,
    #[serde(rename = "resumeLatestRepo", skip_serializing_if = "Option::is_none")]
    pub resume_latest_repo: Option<CommandTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarnessCapabilities {
    pub launch: bool,
    pub resume_exact: bool,
    pub resume_latest_global: bool,
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
pub enum ResumeStrategy {
    Exact,
    LatestGlobal,
    LatestCwd,
    LatestRepo,
    None,
}

pub struct HarnessAdapter {
    pub id: String,
    pub display_name: String,
    pub capabilities: HarnessCapabilities,
    launch_template: CommandTemplate,
    exact_resume_template: Option<CommandTemplate>,
    latest_global_resume_template: Option<CommandTemplate>,
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
            latest_global_resume_template: config.resume_latest_global,
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
        latest_global_resume_template: Option<CommandTemplate>,
        latest_cwd_resume_template: Option<CommandTemplate>,
        latest_repo_resume_template: Option<CommandTemplate>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            capabilities,
            launch_template,
            exact_resume_template,
            latest_global_resume_template,
            latest_cwd_resume_template,
            latest_repo_resume_template,
        }
    }

    pub fn build_resume_command(&self, request: &ResumeRequest) -> Option<CommandSpec> {
        let template = match request.strategy {
            ResumeStrategy::Exact => self.exact_resume_template.as_ref()?,
            ResumeStrategy::LatestGlobal => self.latest_global_resume_template.as_ref()?,
            ResumeStrategy::LatestCwd => self.latest_cwd_resume_template.as_ref()?,
            ResumeStrategy::LatestRepo => self.latest_repo_resume_template.as_ref()?,
            ResumeStrategy::None => return None,
        };
        Some(render_template(template, request))
    }
}

pub fn select_resume_strategy(
    memory: &ResumeMemory,
    capabilities: &HarnessCapabilities,
) -> ResumeStrategy {
    if memory.state != ResumeState::Available {
        return ResumeStrategy::None;
    }
    if capabilities.resume_exact && memory.harness_session_id.is_some() {
        return ResumeStrategy::Exact;
    }
    if memory.latest_fallback && capabilities.resume_latest_global {
        return ResumeStrategy::LatestGlobal;
    }
    if memory.latest_fallback && capabilities.resume_latest_in_cwd {
        return ResumeStrategy::LatestCwd;
    }
    if memory.latest_fallback && capabilities.resume_latest_in_repo {
        return ResumeStrategy::LatestRepo;
    }
    ResumeStrategy::None
}
```

- [ ] **Step 4: Run the harness unit tests again**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml latest_global
cargo test --manifest-path crates/orkworksd/Cargo.toml harness
```

Expected: PASS for the new `latest_global` tests and the pre-existing harness adapter tests.

- [ ] **Step 5: Commit the neutral resume model change**

Run:

```bash
git add crates/orkworksd/src/harness.rs
git commit -m "feat: add global latest resume strategy"
```

---

### Task 2: Add Hermes To The Sidecar Harness And Adapter Registries

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Write failing sidecar tests for Hermes launch and resume wiring**

Add these tests to the existing `#[cfg(test)]` block in `crates/orkworksd/src/main.rs`:

```rust
#[test]
fn builtin_harness_configs_include_hermes_agent() {
    let harnesses = builtin_harness_configs();
    let hermes = harnesses.iter().find(|h| h.id == "hermes").unwrap();

    assert_eq!(hermes.name, "Hermes Agent");
    assert_eq!(hermes.harness, "hermes");
    assert_eq!(hermes.command, "hermes");
    assert_eq!(hermes.args, vec!["chat", "--model={model}"]);
}

#[test]
fn resolve_session_launch_uses_hermes_chat_command() {
    let harnesses = builtin_harness_configs();
    let launch = resolve_session_launch(
        &harnesses,
        &CreateSessionRequest {
            harness_id: Some("hermes".into()),
            model: Some("nous-hermes-3".into()),
            initial_prompt: None,
        },
        "/repo".into(),
    );

    assert_eq!(launch.session_harness_id.as_deref(), Some("hermes"));
    assert_eq!(launch.adapter_harness_id.as_deref(), Some("hermes"));
    assert_eq!(launch.command.program, "hermes");
    assert_eq!(launch.command.args, vec!["chat", "--model=nous-hermes-3"]);
}

#[test]
fn builtin_hermes_adapter_builds_exact_and_latest_global_resume_commands() {
    let adapter = builtin_adapters().get("hermes").unwrap();

    let exact = adapter
        .build_resume_command(&harness::ResumeRequest {
            strategy: harness::ResumeStrategy::Exact,
            cwd: "/repo".into(),
            repo_root: Some("/repo".into()),
            harness_session_id: Some("sess-42".into()),
            model: None,
        })
        .unwrap();
    assert_eq!(exact.program, "hermes");
    assert_eq!(exact.args, vec!["--resume", "sess-42"]);

    let latest = adapter
        .build_resume_command(&harness::ResumeRequest {
            strategy: harness::ResumeStrategy::LatestGlobal,
            cwd: "/repo".into(),
            repo_root: Some("/repo".into()),
            harness_session_id: None,
            model: None,
        })
        .unwrap();
    assert_eq!(latest.program, "hermes");
    assert_eq!(latest.args, vec!["--continue"]);
}

#[test]
fn memory_state_marks_latest_global_resume_as_resumable() {
    let caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: true,
        resume_latest_global: true,
        resume_latest_in_cwd: false,
        resume_latest_in_repo: false,
        detect_session_id: true,
        detect_model: true,
        detect_context_usage: false,
        detect_capacity: false,
        native_voice: false,
    };
    let resume = harness::ResumeMemory {
        state: harness::ResumeState::Available,
        preferred_strategy: harness::ResumeStrategy::LatestGlobal,
        harness_session_id: None,
        latest_fallback: true,
        last_seen_at: None,
    };

    let (memory_state, strategy) = derive_memory_state(false, Some(&resume), &caps);

    assert_eq!(memory_state, MemoryState::Resumable);
    assert_eq!(strategy, harness::ResumeStrategy::LatestGlobal);
}
```

- [ ] **Step 2: Run the sidecar tests to verify Hermes coverage is missing**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml hermes
```

Expected: FAIL because the built-in harness list and adapter registry do not yet contain a `hermes` entry, and `ResumeStrategy::LatestGlobal` is not yet wired through `main.rs`.

- [ ] **Step 3: Implement the Hermes built-in harness and adapter**

Update `crates/orkworksd/src/main.rs` in three places:

1. Add Hermes to the built-in harness list:

```rust
HarnessConfig {
    id: "hermes".into(),
    name: "Hermes Agent".into(),
    harness: "hermes".into(),
    command: "hermes".into(),
    args: vec!["chat".into(), "--model={model}".into()],
    default_model: String::new(),
    capabilities: HarnessVoiceCapabilities::default(),
    is_builtin: true,
},
```

2. Carry the new capability field through `default_capabilities()`:

```rust
fn default_capabilities() -> harness::HarnessCapabilities {
    harness::HarnessCapabilities {
        launch: true,
        resume_exact: false,
        resume_latest_global: false,
        resume_latest_in_cwd: false,
        resume_latest_in_repo: false,
        detect_session_id: false,
        detect_model: false,
        detect_context_usage: false,
        detect_capacity: false,
        native_voice: false,
    }
}
```

3. Register a Hermes adapter that advertises exact and global-latest resume only:

```rust
let hermes_caps = harness::HarnessCapabilities {
    launch: true,
    resume_exact: true,
    resume_latest_global: true,
    resume_latest_in_cwd: false,
    resume_latest_in_repo: false,
    detect_session_id: false,
    detect_model: false,
    detect_context_usage: false,
    detect_capacity: false,
    native_voice: false,
};
let hermes = harness::HarnessAdapter::template(
    "hermes",
    "Hermes Agent",
    hermes_caps,
    harness::CommandTemplate {
        command: "hermes".into(),
        args: vec!["chat".into()],
    },
    Some(harness::CommandTemplate {
        command: "hermes".into(),
        args: vec!["--resume".into(), "{harnessSessionId}".into()],
    }),
    Some(harness::CommandTemplate {
        command: "hermes".into(),
        args: vec!["--continue".into()],
    }),
    None,
    None,
);
map.insert("hermes".into(), hermes);
```

Also update the existing `generic-shell`, `opencode`, and `claude-code` `HarnessAdapter::template(...)` calls to satisfy the expanded function signature by passing `None` for the new `latest_global` slot unless you intentionally choose to reclassify those adapters in a separate scoped change.

- [ ] **Step 4: Run the sidecar tests again**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml hermes
cargo test --manifest-path crates/orkworksd/Cargo.toml memory_state_marks_latest_global_as_resumable
```

Expected: PASS for the Hermes-specific tests and the remembered-session state assertion.

- [ ] **Step 5: Commit the Hermes sidecar wiring**

Run:

```bash
git add crates/orkworksd/src/main.rs
git commit -m "feat: add Hermes harness adapter"
```

---

### Task 3: Update Renderer Types And Resume Copy For `latest_global`

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/labels.ts`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/tests/api.test.ts`
- Modify: `apps/desktop/tests/labels.test.ts`
- Modify: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Write failing renderer tests for the new strategy and Hermes-specific wording**

Update `apps/desktop/tests/api.test.ts` with a `latest_global`-shaped session:

```ts
test("SessionInfo type accepts global-latest resume metadata", () => {
  const session: SessionInfo = {
    id: "hermes-1",
    label: "Hermes",
    harness: "hermes",
    status: "ended",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "resumable",
    resumeStrategy: "latest_global",
    resume: {
      state: "available",
      preferredStrategy: "latest_global",
      latestFallback: true,
    },
  };

  assert.equal(session.resumeStrategy, "latest_global");
  assert.equal(session.resume?.preferredStrategy, "latest_global");
});
```

Update `apps/desktop/tests/labels.test.ts` with the new copy expectations:

```ts
import {
  attentionLabel,
  attentionTone,
  memoryStateLabel,
  relativeTime,
  resumeActionLabel,
  resumeActionTitle,
  sourceLabel,
  sourceWithConfidence,
  VOCAB,
} from "../src/labels.ts";

test("resume helpers distinguish global latest from exact resume", () => {
  assert.equal(resumeActionLabel("latest_global"), "Continue latest session");
  assert.equal(resumeActionTitle("latest_global", "hermes"), "Continue latest Hermes session");
  assert.equal(resumeActionTitle("latest_global", "opencode"), "Continue latest session");
  assert.equal(resumeActionTitle("exact", "hermes"), "Resume session");
});
```

Update `apps/desktop/tests/dockview.test.ts` so the source-level assertion expects the new helper in `SessionDetailPanel.tsx`:

```ts
assert.match(source, /resumeActionTitle/);
```

- [ ] **Step 2: Run the renderer tests to verify the current types and copy are incomplete**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/labels.test.ts tests/dockview.test.ts
```

Expected: FAIL because `latest_global` is not yet part of the `ResumeStrategy` union and `resumeActionTitle` does not exist.

- [ ] **Step 3: Implement the typed union and harness-aware resume copy**

Update `apps/desktop/src/api.ts`:

```ts
export type ResumeStrategy =
  | "exact"
  | "latest_global"
  | "latest_cwd"
  | "latest_repo"
  | "none";
```

Update `apps/desktop/src/labels.ts`:

```ts
export function resumeActionLabel(strategy: ResumeStrategy): string {
  switch (strategy) {
    case "exact":         return "Resume session";
    case "latest_global": return "Continue latest session";
    case "latest_cwd":    return "Resume latest in folder";
    case "latest_repo":   return "Resume latest in repo";
    case "none":          return "Resume unavailable";
  }
}

export function resumeActionTitle(
  strategy: ResumeStrategy,
  harness?: string,
): string {
  if (strategy === "latest_global" && harness === "hermes") {
    return "Continue latest Hermes session";
  }
  return resumeActionLabel(strategy);
}
```

Update `apps/desktop/src/components/SessionDetailPanel.tsx` to use the new helper where the user triggers resume:

```tsx
import {
  attentionLabel,
  attentionTone,
  memoryStateLabel,
  relativeTime,
  resumeActionLabel,
  resumeActionTitle,
  sourceLabel,
  sourceWithConfidence,
} from "../labels";

const canResume = active.memoryState === "resumable" && active.resumeStrategy !== "none";
const resumeText = resumeActionTitle(active.resumeStrategy, active.harness);
```

Keep the “Memory” summary row generic via `resumeActionLabel(active.resumeStrategy)` so the detail panel distinguishes the button promise without turning the summary line into harness-specific prose.

- [ ] **Step 4: Run the renderer tests again**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/labels.test.ts tests/dockview.test.ts
```

Expected: PASS for the new typed union, resume copy, and source-level detail-panel assertion.

- [ ] **Step 5: Commit the renderer update**

Run:

```bash
git add apps/desktop/src/api.ts apps/desktop/src/labels.ts apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/tests/api.test.ts apps/desktop/tests/labels.test.ts apps/desktop/tests/dockview.test.ts
git commit -m "feat: surface Hermes latest-session resume"
```

---

### Task 4: Final Verification And Doc Currency Check

**Files:**
- Modify: none
- Test: `crates/orkworksd/src/harness.rs`
- Test: `crates/orkworksd/src/main.rs`
- Test: `apps/desktop/tests/api.test.ts`
- Test: `apps/desktop/tests/labels.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Run the targeted backend verification suite**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml latest_global
cargo test --manifest-path crates/orkworksd/Cargo.toml hermes
```

Expected: PASS for the neutral resume-model tests and the Hermes sidecar tests.

- [ ] **Step 2: Run the targeted frontend verification suite**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/labels.test.ts tests/dockview.test.ts
```

Expected: PASS for the typed API and resume-copy assertions.

- [ ] **Step 3: Run the repo doc currency hook**

Run:

```bash
bash .claude/hooks/doc-check.sh
```

Expected: no newly flagged doc updates beyond the Hermes design and plan artifacts already added, because this slice does not add dependencies, change repo workflow rules, or create a new ADR.

- [ ] **Step 4: Review the diff for scope discipline**

Run:

```bash
git diff --stat HEAD~3..HEAD
git diff -- crates/orkworksd/src/harness.rs crates/orkworksd/src/main.rs apps/desktop/src/api.ts apps/desktop/src/labels.ts apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/tests/api.test.ts apps/desktop/tests/labels.test.ts apps/desktop/tests/dockview.test.ts
```

Expected: the diff stays limited to the neutral resume-model extension, Hermes sidecar wiring, and the small renderer copy/type update. No provider-registry, Taskmaster, or Hermes-internals work should appear.

- [ ] **Step 5: Create the final implementation commit if the execution path keeps prior task commits local**

Run:

```bash
git status --short
```

Expected: clean working tree if each task commit was made as written. If the execution flow intentionally batches multiple tasks before commit, create the missing commit(s) using the task-level commit messages above before handoff.
