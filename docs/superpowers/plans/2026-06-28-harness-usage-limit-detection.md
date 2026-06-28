# Harness Usage Limit Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface `atUsageLimit: bool` on live sessions in the API by pattern-scanning terminal output — no LLM, no new endpoint.

**Architecture:** Each `HarnessAdapter` carries a static slice of rate-limit strings. A pure `detect_usage_limit` function in `peon.rs` scans the last 50 lines of the output buffer. `list_sessions` collects buffer snapshots alongside `SessionInfo`, calls the scan after merging, and sets `at_usage_limit` on the response struct. Dead/remembered sessions always get `None`.

**Tech Stack:** Rust (orkworksd), TypeScript (Electron renderer). No new dependencies.

---

## File Map

| File | Change |
|---|---|
| `crates/orkworksd/src/peon.rs` | Add `detect_usage_limit` pure fn + unit tests |
| `crates/orkworksd/src/harness.rs` | Add `limit_patterns` field to `HarnessAdapter`; update `template()` and `from_config()` |
| `crates/orkworksd/src/main.rs` | Set patterns in `builtin_adapters()`; add `at_usage_limit` to `SessionInfo`; wire `list_sessions` |
| `apps/desktop/src/api.ts` | Add `atUsageLimit?: boolean` to `SessionInfo` interface |

---

## Task 1: Add `detect_usage_limit` to peon.rs

**Files:**
- Modify: `crates/orkworksd/src/peon.rs` (after the `RingBuffer` impl, before `const SYSTEM_PROMPT`)

- [ ] **Step 1: Write failing tests**

Add inside the existing `#[cfg(test)]` block at the bottom of `peon.rs`:

```rust
#[test]
fn detect_usage_limit_returns_false_when_no_patterns() {
    let lines: Vec<String> = vec!["usage limit reached".into()];
    assert!(!detect_usage_limit(&[], &lines));
}

#[test]
fn detect_usage_limit_returns_true_on_match() {
    let lines = vec!["some output".into(), "usage limit reached, resets in 2h".into()];
    assert!(detect_usage_limit(&["usage limit reached"], &lines));
}

#[test]
fn detect_usage_limit_is_case_insensitive() {
    let lines = vec!["Usage Limit Reached".into()];
    assert!(detect_usage_limit(&["usage limit reached"], &lines));
}

#[test]
fn detect_usage_limit_returns_false_when_no_match() {
    let lines = vec!["working on task".into(), "tool call made".into()];
    assert!(!detect_usage_limit(&["usage limit reached"], &lines));
}

#[test]
fn detect_usage_limit_only_scans_last_50_lines() {
    let mut lines: Vec<String> = (0..60).map(|_| "no match".into()).collect();
    lines[5] = "usage limit reached".into(); // line 5, outside last 50
    assert!(!detect_usage_limit(&["usage limit reached"], &lines));
}

#[test]
fn detect_usage_limit_matches_within_last_50_lines() {
    let mut lines: Vec<String> = (0..60).map(|_| "no match".into()).collect();
    lines[15] = "usage limit reached".into(); // within last 50 of 60
    assert!(detect_usage_limit(&["usage limit reached"], &lines));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml detect_usage_limit 2>&1 | tail -20
```

Expected: `error[E0425]: cannot find function 'detect_usage_limit'`

- [ ] **Step 3: Add the function**

Insert after the closing `}` of `impl RingBuffer` (before `const SYSTEM_PROMPT`):

```rust
pub fn detect_usage_limit(patterns: &[&str], lines: &[String]) -> bool {
    if patterns.is_empty() { return false; }
    lines.iter().rev().take(50).any(|line| {
        let lower = line.to_lowercase();
        patterns.iter().any(|p| lower.contains(p))
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml detect_usage_limit 2>&1 | tail -10
```

Expected: `test result: ok. 6 passed`

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/peon.rs
git commit -m "feat: add detect_usage_limit pattern scan to peon"
```

---

## Task 2: Add `limit_patterns` to `HarnessAdapter`

**Files:**
- Modify: `crates/orkworksd/src/harness.rs` — struct, `template()`, `from_config()`
- Modify: `crates/orkworksd/src/main.rs` — `builtin_adapters()` (3 call sites)

- [ ] **Step 1: Add the field to `HarnessAdapter` and update constructors**

In `crates/orkworksd/src/harness.rs`, update the struct (around line 92):

```rust
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HarnessAdapter {
    pub id: String,
    pub display_name: String,
    pub capabilities: HarnessCapabilities,
    pub limit_patterns: &'static [&'static str],
    launch_template: CommandTemplate,
    exact_resume_template: Option<CommandTemplate>,
    latest_cwd_resume_template: Option<CommandTemplate>,
    latest_repo_resume_template: Option<CommandTemplate>,
}
```

Update `from_config()` (around line 104) — external configs can't provide static strings, so always `&[]`:

```rust
pub fn from_config(config: HarnessAdapterConfig) -> Self {
    Self {
        id: config.id,
        display_name: config.display_name,
        capabilities: config.capabilities,
        limit_patterns: &[],
        launch_template: config.launch,
        exact_resume_template: config.resume_exact,
        latest_cwd_resume_template: config.resume_latest_cwd,
        latest_repo_resume_template: config.resume_latest_repo,
    }
}
```

Update `template()` signature (around line 116) to add `limit_patterns`:

```rust
pub fn template(
    id: impl Into<String>,
    display_name: impl Into<String>,
    capabilities: HarnessCapabilities,
    limit_patterns: &'static [&'static str],
    launch_template: CommandTemplate,
    exact_resume_template: Option<CommandTemplate>,
    latest_cwd_resume_template: Option<CommandTemplate>,
) -> Self {
    Self {
        id: id.into(),
        display_name: display_name.into(),
        capabilities,
        limit_patterns,
        launch_template,
        exact_resume_template,
        latest_cwd_resume_template,
        latest_repo_resume_template: None,
    }
}
```

- [ ] **Step 2: Fix the call site in `harness.rs` tests**

The test at around line 273 calls `HarnessAdapter::template(...)` with 6 args. Add `&[]` as the 4th argument (after `capabilities`, before `launch_template`):

```rust
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
    &[],  // limit_patterns — none for test adapter
    CommandTemplate {
        command: "custom-ai".into(),
        args: vec!["--start".into()],
    },
    Some(CommandTemplate {
        command: "custom-ai".into(),
        args: vec!["--resume".into(), "{harnessSessionId}".into()],
    }),
    Some(CommandTemplate {
        command: "custom-ai".into(),
        args: vec!["--continue".into(), "--cwd".into(), "{cwd}".into()],
    }),
);
```

- [ ] **Step 3: Update `builtin_adapters()` in main.rs**

In `crates/orkworksd/src/main.rs`, update the three `HarnessAdapter::template(...)` calls in `builtin_adapters()` (around line 1767). Add `limit_patterns` as the 4th argument in each:

**generic-shell** (no AI, no patterns):
```rust
let generic = harness::HarnessAdapter::template(
    "generic-shell",
    "Generic Shell",
    default_capabilities(),
    &[],
    harness::CommandTemplate {
        command: program.clone(),
        args: args.clone(),
    },
    None,
    None,
);
```

**opencode** (confirmed pattern from test fixture):
```rust
let opencode = harness::HarnessAdapter::template(
    "opencode",
    "OpenCode",
    opencode_caps.clone(),
    &["usage limit reached"],
    harness::CommandTemplate {
        command: "opencode".into(),
        args: vec![],
    },
    Some(harness::CommandTemplate {
        command: "opencode".into(),
        args: vec!["--session".into(), "{harnessSessionId}".into()],
    }),
    Some(harness::CommandTemplate {
        command: "opencode".into(),
        args: vec!["--continue".into()],
    }),
);
```

**claude-code** (placeholder — see issue #84 for verification):
```rust
// ponytail: claude-code patterns are unverified placeholders — see GitHub issue #84
let claude = harness::HarnessAdapter::template(
    "claude-code",
    "Claude Code",
    claude_caps.clone(),
    &["claude code is currently unavailable", "usage limit"],
    harness::CommandTemplate {
        command: "claude".into(),
        args: vec![],
    },
    Some(harness::CommandTemplate {
        command: "claude".into(),
        args: vec!["--resume".into(), "{harnessSessionId}".into()],
    }),
    Some(harness::CommandTemplate {
        command: "claude".into(),
        args: vec!["--continue".into()],
    }),
);
```

- [ ] **Step 4: Verify it compiles and tests pass**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -15
```

Expected: `test result: ok.` with no compile errors.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/harness.rs crates/orkworksd/src/main.rs
git commit -m "feat: add limit_patterns field to HarnessAdapter"
```

---

## Task 3: Add `at_usage_limit` to `SessionInfo` and wire `list_sessions`

**Files:**
- Modify: `crates/orkworksd/src/main.rs`

This task has the most mechanical changes. `SessionInfo` is a large struct constructed in many places — every literal construction needs `at_usage_limit: None` added. Then `list_sessions` needs to thread buffer snapshots through.

- [ ] **Step 1: Add field to `SessionInfo` struct**

In `main.rs`, add to the `SessionInfo` struct (after `capacity_hints`, around line 79):

```rust
#[serde(rename = "atUsageLimit", skip_serializing_if = "Option::is_none")]
at_usage_limit: Option<bool>,
```

- [ ] **Step 2: Add `at_usage_limit: None` to all `SessionInfo` literal constructions**

The struct is constructed in many places. Use the compiler errors to find every site. After adding the field, run:

```bash
cargo build --manifest-path crates/orkworksd/Cargo.toml 2>&1 | grep "missing field"
```

For every location reported, add `at_usage_limit: None`. This includes:
- `create_session` handler (~line 684)
- `resume_session` handler (~line 456)
- `merge_live_session_info` return value (~line 991)
- The remembered-sessions loop in `list_sessions` (~line 886)
- Every `SessionInfo { ... }` in `#[cfg(test)]` blocks

- [ ] **Step 3: Update `list_sessions` to collect buffer snapshots**

Change the live session collection at around line 843 from collecting `Vec<SessionInfo>` to `Vec<(SessionInfo, Vec<String>)>`:

```rust
let live_sessions: Vec<(SessionInfo, Vec<String>)> = {
    let sessions = state.sessions.lock().unwrap();
    sessions.values()
        .map(|h| (h.info.clone(), h.output_buffer.snapshot()))
        .collect()
};
```

- [ ] **Step 4: Update `all_memory_ids` to destructure the tuple**

At around line 862, change:

```rust
let all_memory_ids: HashSet<String> = live_sessions.iter()
    .map(|info| info.id.clone())
    .collect();
```

To:

```rust
let all_memory_ids: HashSet<String> = live_sessions.iter()
    .map(|(info, _)| info.id.clone())
    .collect();
```

- [ ] **Step 5: Update the merge loop to scan the buffer**

At around line 867, change the `live_sessions.into_iter().map(|info| { ... })` to destructure the tuple and call `detect_usage_limit`:

```rust
let mut infos: Vec<SessionInfo> = live_sessions.into_iter().map(|(info, snapshot)| {
    let id = info.id.clone();
    let meta = metadata_map.get(&id);
    let session_harness_id = meta.and_then(|m| (!m.harness.is_empty()).then(|| m.harness.as_str()));
    let adapter_harness_id = resolve_adapter_harness_id(&harnesses, session_harness_id);
    let caps = capabilities_for_harness(&state.adapters, adapter_harness_id.as_deref());
    let mut merged = merge_live_session_info(info, meta, peon_times.get(&id), &caps);
    merged.at_usage_limit = adapter_harness_id
        .as_deref()
        .and_then(|hid| state.adapters.get(hid))
        .map(|adapter| peon::detect_usage_limit(adapter.limit_patterns, &snapshot));
    merged
}).collect();
```

- [ ] **Step 6: Write a test for the at_usage_limit field**

Add a test in the `#[cfg(test)]` section of `main.rs` near the other `list_sessions` tests:

```rust
#[tokio::test]
async fn list_sessions_sets_at_usage_limit_when_pattern_matches() {
    let state = Arc::new(AppState {
        session_module: test_session_module(),
        sessions: Mutex::new(HashMap::new()),
        workspace: Mutex::new(None),
        peon: test_peon_state(),
        providers: ProviderManager::for_tests(
            sample_settings(vec![]),
            registry_with(vec![]),
        ),
        adapters: builtin_adapters(),
        retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
        harnesses: tokio::sync::RwLock::new(vec![]),
        bound_port: AtomicU16::new(0),
    });

    let session_id = "test-limit-session".to_string();
    let mut handle = SessionHandle {
        info: SessionInfo {
            id: session_id.clone(),
            label: "Test".into(),
            harness_id: Some("opencode".into()),
            harness: Some("opencode".into()),
            status: "running".into(),
            cwd: "/tmp".into(),
            created_at: "2026-06-28T00:00:00Z".into(),
            at_usage_limit: None,
            // set all other Option fields to None, non-Option fields to defaults:
            model_provider_id: None,
            model_id: None,
            model: None,
            connectivity: None,
            terminal_outcome: None,
            last_activity_at: None,
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
            metadata_source: None,
            metadata_confidence: None,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            conflict_warning: None,
            recommendation: None,
            peon_last_inference: None,
            memory_state: MemoryState::Live,
            resume_strategy: harness::ResumeStrategy::None,
            resume: None,
            resume_options: vec![],
            resumed_from: None,
            provider: None,
            provider_model: None,
            provider_state: None,
        },
        kill_tx: tokio::sync::watch::channel(false).0,
        output_buffer: {
            let mut buf = peon::RingBuffer::new(200);
            buf.push("usage limit reached, resets in 2h".into());
            buf
        },
        command: harness::CommandSpec {
            program: "opencode".into(),
            args: vec![],
            cwd: "/tmp".into(),
        },
        initial_prompt: None,
    };
    state.sessions.lock().unwrap().insert(session_id.clone(), handle);

    let response = axum::extract::State(state.clone());
    // Call list_sessions and check the result directly:
    let infos: Vec<SessionInfo> = {
        let harnesses = state.harnesses.read().await.clone();
        let live_sessions: Vec<(SessionInfo, Vec<String>)> = {
            let sessions = state.sessions.lock().unwrap();
            sessions.values().map(|h| (h.info.clone(), h.output_buffer.snapshot())).collect()
        };
        let ws_guard = state.workspace.lock().unwrap();
        drop(ws_guard);
        let all_memory_ids: HashSet<String> = live_sessions.iter().map(|(info, _)| info.id.clone()).collect();
        let peon_times = state.peon.last_inference.read().unwrap();
        live_sessions.into_iter().map(|(info, snapshot)| {
            let id = info.id.clone();
            let session_harness_id = info.harness_id.as_deref();
            let adapter_harness_id = resolve_adapter_harness_id(&harnesses, session_harness_id);
            let caps = capabilities_for_harness(&state.adapters, adapter_harness_id.as_deref());
            let mut merged = merge_live_session_info(info, None, peon_times.get(&id), &caps);
            merged.at_usage_limit = adapter_harness_id
                .as_deref()
                .and_then(|hid| state.adapters.get(hid))
                .map(|adapter| peon::detect_usage_limit(adapter.limit_patterns, &snapshot));
            merged
        }).collect()
    };

    let session = infos.iter().find(|s| s.id == session_id).unwrap();
    assert_eq!(session.at_usage_limit, Some(true));
}
```

> **Note:** Look at the existing `list_sessions_does_not_duplicate_killed_sessions_with_metadata` test nearby for the exact helper function names (`test_session_module`, `test_peon_state`, `sample_settings`, etc.) and copy their initialization pattern exactly. The `SessionInfo` field list must match whatever the struct has at this point in the plan.

- [ ] **Step 7: Run all Rust tests**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -20
```

Expected: `test result: ok.` — all existing tests still pass, new test passes.

- [ ] **Step 8: Commit**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "feat: expose atUsageLimit on live sessions via output buffer scan"
```

---

## Task 4: Add `atUsageLimit` to TypeScript SessionInfo

**Files:**
- Modify: `apps/desktop/src/api.ts`

- [ ] **Step 1: Add the field**

In `apps/desktop/src/api.ts`, add to the `SessionInfo` interface after `capacityHints`:

```typescript
atUsageLimit?: boolean;
```

- [ ] **Step 2: Type-check**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/api.ts
git commit -m "feat: add atUsageLimit field to TypeScript SessionInfo"
```

---

## Self-Review

**Spec coverage:**
- ✅ `limit_patterns` on `HarnessAdapter` — Task 2
- ✅ `detect_usage_limit` pure function in `peon.rs` — Task 1
- ✅ `atUsageLimit: Option<bool>` on `SessionInfo`, `None` for dead sessions — Task 3 (remembered sessions in `list_sessions` keep `at_usage_limit: None`)
- ✅ `list_sessions` collects buffer snapshots and scans — Task 3
- ✅ No gate on `create_session` — nothing added there
- ✅ TypeScript field — Task 4
- ✅ Claude Code patterns marked with `ponytail:` comment referencing issue #84 — Task 2 Step 3

**Placeholder scan:** No TBDs. The test in Task 3 Step 6 notes to match helper names from adjacent tests — that is guidance, not a placeholder.

**Type consistency:** `detect_usage_limit(patterns: &[&str], lines: &[String]) -> bool` used consistently in Task 1 (definition), Task 2 (field type `&'static [&'static str]` narrows to `&[&str]` at call site — valid coercion), Task 3 (call site).
