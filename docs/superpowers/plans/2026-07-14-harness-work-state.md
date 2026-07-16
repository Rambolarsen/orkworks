# Harness-Verified Working State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mark a session Working only from a registered harness active-work signal, or for unsupported harnesses from guarded post-submission PTY output.

**Architecture:** Add an explicit active-work-hook capability to harness configuration and resolve it into each session at launch. A small per-session fallback state machine records a completed input line, consumes its echoed output across PTY chunks, and permits one working transition only during a ten-second window. Registered active-work hooks bypass that fallback and normalize `working`, `thinking`, and equivalent active status reports to `working`.

**Tech Stack:** Rust, Axum, Tokio, serde, portable-pty, existing Rust unit and async tests.

## Global Constraints

- Keep Electron and renderer code unchanged; this is sidecar-only state behavior.
- Keep existing non-working attention reports (`waiting_for_input`, `blocked`, `failed`, `done`, `idle`, `stale`) unchanged.
- A registered active-work hook fails closed: PTY output must not become a fallback path when that hook is unavailable or malformed.
- Fallback promotion is valid only for a non-empty CR/LF-terminated submission, within 10 seconds measured by `tokio::time::Instant`.
- Ignore ANSI/control-only chunks and the exact submitted-input echo, including echoes split across PTY chunks.

---

### Task 1: Represent and normalize harness active-work hooks

**Files:**
- Modify: `crates/orkworksd/src/harness_registry.rs:8-70`
- Modify: `crates/orkworksd/src/http/session_handlers.rs:49-53,431-490`
- Modify: `crates/orkworksd/src/main.rs:49-68`
- Test: `crates/orkworksd/src/harness_registry.rs` test module
- Test: `crates/orkworksd/src/http/session_handlers.rs` test module

**Interfaces:**
- Produces `HarnessAttentionCapabilities { reports_active_work: bool }`, serialized as `attention.activeWorkHook` and defaulting to `false` for persisted and built-in harness configurations.
- Produces `normalize_hook_attention_status(status: &str, supports_active_work: bool) -> Option<String>`: `working`, `thinking`, and `reasoning` normalize to `Some("working".into())` only when capability is true; existing valid non-working statuses normalize to owned copies of themselves.
- Produces a `SessionHandle` boolean resolved at session creation indicating whether that session has a registered active-work hook.

- [ ] **Step 1: Write failing configuration and hook-normalization tests**

```rust
#[test]
fn attention_capability_defaults_to_no_active_work_hook() {
    let parsed: HarnessConfig = serde_json::from_str(r#"{
        "id":"custom", "name":"Custom", "harness":"custom",
        "command":"custom"
    }"#).unwrap();
    assert!(!parsed.attention.active_work_hook);
}

#[test]
fn active_hook_aliases_normalize_only_for_capable_harnesses() {
    assert_eq!(normalize_hook_attention_status("thinking", true), Some("working".into()));
    assert_eq!(normalize_hook_attention_status("reasoning", true), Some("working".into()));
    assert_eq!(normalize_hook_attention_status("thinking", false), None);
    assert_eq!(normalize_hook_attention_status("waiting_for_input", false), Some("waiting_for_input".into()));
}
```

Add an async handler test that creates a capable session, posts `thinking`, and asserts both persisted metadata and in-memory `SessionInfo` contain `working`; add the inverse test for a non-capable session returning `BAD_REQUEST` without altering attention.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml attention_capability_defaults_to_no_active_work_hook && cargo test --manifest-path crates/orkworksd/Cargo.toml active_hook_aliases_normalize_only_for_capable_harnesses`

Expected: FAIL because the attention capability and normalization helper do not exist.

- [ ] **Step 3: Add the minimal hook capability and normalization implementation**

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct HarnessAttentionCapabilities {
    #[serde(rename = "activeWorkHook", default)]
    pub(crate) active_work_hook: bool,
}

pub(crate) fn normalize_hook_attention_status(
    status: &str,
    supports_active_work: bool,
) -> Option<String> {
    match status {
        "working" | "thinking" | "reasoning" if supports_active_work => Some("working".into()),
        "waiting_for_input" | "blocked" | "failed" | "done" | "stale" | "idle" => Some(status.into()),
        _ => None,
    }
}
```

Add `attention: HarnessAttentionCapabilities` to `HarnessConfig`, add the resolved boolean to `SessionHandle`, and initialize it in every production and test `SessionHandle` constructor. In `report_attention`, resolve the session’s boolean, normalize the reported status before validation/merge, and reject unsupported active aliases without changing state. Do not mark any current built-in harness active-work-capable until its hook actually emits active model events.

- [ ] **Step 4: Run focused tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml attention_capability_defaults_to_no_active_work_hook && cargo test --manifest-path crates/orkworksd/Cargo.toml active_hook_aliases_normalize_only_for_capable_harnesses && cargo test --manifest-path crates/orkworksd/Cargo.toml report_attention`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/harness_registry.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/main.rs
git commit -m "feat(sidecar): register active-work hook capability"
```

### Task 2: Add a deterministic fallback work-signal state machine

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:20-37,451-559`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:64-116,548-626`
- Modify: `crates/orkworksd/src/main.rs:49-68`
- Test: `crates/orkworksd/src/runtime/session_runtime.rs` test module
- Test: `crates/orkworksd/src/runtime/terminal_runtime.rs` test module

**Interfaces:**
- Produces `PendingWorkSignal { remaining_echo: String, expires_at: tokio::time::Instant }` stored as `Option<PendingWorkSignal>` in `SessionHandle`.
- Produces `arm_pending_work_signal(submitted_line: &str, now: Instant) -> PendingWorkSignal` with `expires_at == now + Duration::from_secs(10)`.
- Produces `consume_pending_work_signal(signal: &mut PendingWorkSignal, output: &str, now: Instant) -> bool`, returning true only for qualifying non-echo visible output inside the window.

- [ ] **Step 1: Write failing pure-state-machine tests**

```rust
#[test]
fn split_echo_does_not_qualify_until_new_visible_output_arrives() {
    let now = tokio::time::Instant::now();
    let mut signal = arm_pending_work_signal("fix status", now);
    assert!(!consume_pending_work_signal(&mut signal, "fix ", now));
    assert!(!consume_pending_work_signal(&mut signal, "status\\r\\n", now));
    assert!(consume_pending_work_signal(&mut signal, "Thinking…", now));
}

#[test]
fn ansi_only_output_and_expired_submission_do_not_qualify() {
    let now = tokio::time::Instant::now();
    let mut signal = arm_pending_work_signal("fix", now);
    assert!(!consume_pending_work_signal(&mut signal, "\\x1b[2K\\r", now));
    assert!(!consume_pending_work_signal(
        &mut signal, "model output", now + std::time::Duration::from_secs(10),
    ));
}
```

Add integration-level tests covering: partial typing never arms a signal; a CR/LF-completed non-empty line arms one only when `active_work_hook_registered == false`; a capable session’s qualifying PTY output cannot change `idle` to `working`; and the first qualifying fallback output updates both the in-memory session and persisted metadata once.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml split_echo_does_not_qualify_until_new_visible_output_arrives && cargo test --manifest-path crates/orkworksd/Cargo.toml ansi_only_output_and_expired_submission_do_not_qualify`

Expected: FAIL because `PendingWorkSignal` and its helpers do not exist.

- [ ] **Step 3: Implement the minimal guarded fallback**

```rust
const WORK_SIGNAL_WINDOW: std::time::Duration = std::time::Duration::from_secs(10);

pub(crate) struct PendingWorkSignal {
    remaining_echo: String,
    expires_at: tokio::time::Instant,
}
```

In the terminal WebSocket input path, retain `collect_input_line` for labels and `last_user_input`, but remove the input-driven `last_output`/Peon scheduling that causes typing to trigger inference. After a non-empty collected line, arm `pending_work_signal` only for a session without a registered active-work hook.

In the PTY output path, call `consume_pending_work_signal` on ANSI-stripped output before `should_infer_working`. Promote `observed_status` and `attention` to `working` only when the helper returns true, the lifecycle is alive, and startup grace has elapsed. On promotion, consume the signal and persist the same state transition to metadata. Do not change output replay, output buffering, or Peon persistence behavior.

- [ ] **Step 4: Run focused tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml split_echo_does_not_qualify_until_new_visible_output_arrives && cargo test --manifest-path crates/orkworksd/Cargo.toml ansi_only_output_and_expired_submission_do_not_qualify && cargo test --manifest-path crates/orkworksd/Cargo.toml observer_only_output_cannot_resume_finished_states`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/main.rs
git commit -m "fix(sidecar): gate working state on harness activity"
```

### Task 3: Verify compatibility and documentation currency

**Files:**
- Modify: `docs/agents/domain-entities.md` only if implementation changes a documented session domain field or lifecycle contract.
- Test: `crates/orkworksd` unit and integration test suite.

**Interfaces:**
- Consumes the hook capability and pending-work state machine from Tasks 1–2.
- Produces evidence that existing attention reports, metadata persistence, and terminal output behavior remain compatible.

- [ ] **Step 1: Run the complete Rust suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS with the new hook and fallback tests plus all existing sidecar tests.

- [ ] **Step 2: Run static and diff checks**

Run: `cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings && git diff --check`

Expected: both commands exit 0.

- [ ] **Step 3: Run the required documentation currency check**

Run: `bash .claude/hooks/doc-check.sh`

Expected: no unaddressed documentation triggers. If it reports a domain-model trigger, update `docs/agents/domain-entities.md` in the same task and rerun this check.

- [ ] **Step 4: Commit documentation only if changed**

```bash
git add docs/agents/domain-entities.md
git commit -m "docs: document harness work-state behavior"
```
