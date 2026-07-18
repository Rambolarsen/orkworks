# Single-Key Work Signal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the `needs_you → working` attention transition for Claude Code sessions where the user answers the prompt with a single printable keystroke (e.g. `y`, `1`, or another choice key) rather than an Enter-terminated line.

**Architecture:** Add a single-key arming path inside `record_terminal_input` that arms `pending_work_signal` with the in-progress input-line buffer as the echo prefix whenever a printable keystroke arrives while the session is in `needs_you` set by an agent hook report. The existing Enter-terminated arming path and `should_infer_working` promotion logic are unchanged. The narrow-scope `metadata_source == "agent"` gate sidesteps shell-mode echo false positives.

**Tech Stack:** Rust (orkworksd sidecar). No new dependencies. Tests use the existing `test_state_with_runtime_session` scaffold and the live-PTY `#[tokio::test]` pattern from `output_within_startup_grace_is_replayed_without_marking_attention_working`.

## Global Constraints

- Implement behind the `fix/single-key-work-signal` branch in the `orkworks-fix-single-key-work-signal` worktree. Do not edit files outside this worktree.
- All changes are in `crates/orkworksd/` — must pass `cargo test --manifest-path crates/orkworksd/Cargo.toml` and `cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings` before each commit.
- Do not change `apps/desktop/` (no frontend change — `working`/`needs_you` labels already exist and the 2-second poll surfaces the transition).
- Do not change `crates/orkworksd/scripts/report-claude-session-from-hook.sh` (the bug is that it only sends `waiting_for_input`; the fix is in the sidecar's handling of the resulting state, not the hook script).
- Do not change the capable-hook path (`active_work_hook = true`) or the `should_infer_working` predicate.
- The parent spec `docs/superpowers/specs/2026-07-14-harness-work-state-design.md` was already updated in the design-doc commit (28945aa). Do not touch it again unless a test reveals a remaining inconsistency.
- Per AGENTS.md: this PR touches `crates/orkworksd/` and requires a `/code-review` run before merge.

---

## File Structure

- **Create:** `docs/superpowers/specs/2026-07-17-single-key-work-signal-design.md` — already done in commit 28945aa (design doc). No action in this plan.
- **Modify:** `crates/orkworksd/src/runtime/terminal_runtime.rs` — the new arming block inside `record_terminal_input`. This is the only production code change.
- **Modify (tests):** `crates/orkworksd/src/runtime/session_runtime.rs` — add 5 new tests in the existing `#[cfg(test)] mod tests` block, near the existing `terminal_input_arms_only_completed_hookless_submission` test (line 1039).

---

## Task 1: Refactor `record_terminal_input` to expose the in-progress buffer snapshot

This task is a pure refactor. It does not change behavior — the existing Enter-terminated arming path keeps working exactly as before, but now also produces an `in_progress_buf: String` snapshot that Task 2 will use. Splitting it into its own task lets the refactor land with a passing test (the existing `terminal_input_arms_only_completed_hookless_submission` test must still pass).

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:249–302` (the `record_terminal_input` function body)

**Interfaces:**
- Consumes: `state.peon.input_buf` (`RwLock<HashMap<String, String>>`), `collect_input_line(buf: &mut String, data: &str) -> Option<String>` (line 172).
- Produces: `record_terminal_input` returns `Option<()>` unchanged; internally computes `let (collected_line, in_progress_buf) = { ... };` instead of `let collected_line = { ... };`.

- [ ] **Step 1: Verify the existing test still passes (baseline)**

Run from the worktree root:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests::terminal_input_arms_only_completed_hookless_submission -- --nocapture
```

Expected: 1 test passes. (If this already fails, stop and investigate before refactoring.)

- [ ] **Step 2: Apply the refactor**

Open `crates/orkworksd/src/runtime/terminal_runtime.rs` and find the `record_terminal_input` function (line 249). Locate the existing `let collected_line = { ... };` block around lines 258–262:

```rust
let collected_line = {
    let mut bufs = state.peon.input_buf.write().unwrap();
    let buf = bufs.entry(id.to_string()).or_default();
    collect_input_line(buf, data)
};
```

Replace it with a version that also captures the post-`collect_input_line` in-progress buffer:

```rust
let (collected_line, in_progress_buf) = {
    let mut bufs = state.peon.input_buf.write().unwrap();
    let buf = bufs.entry(id.to_string()).or_default();
    let line = collect_input_line(buf, data);
    (line, buf.clone())
};
```

The downstream `let line = collected_line?;` (around line 261) continues to work — `collected_line` is still `Option<String>`. The new `in_progress_buf: String` is not yet consumed (Task 2 uses it). A Rust compiler may warn about an unused variable here; if so, prefix with underscore for this task only: `let (collected_line, _in_progress_buf) = { ... };` — Task 2 Step 2 will rename it back.

- [ ] **Step 3: Verify the existing test still passes after the refactor**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests::terminal_input_arms_only_completed_hookless_submission -- --nocapture
cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings
```

Expected: test passes; clippy is clean (no warnings).

- [ ] **Step 4: Commit the refactor**

```bash
git add crates/orkworksd/src/runtime/terminal_runtime.rs
git commit -m "refactor(terminal): expose in-progress input buf in record_terminal_input

Capture the post-collect_input_line in-progress buffer snapshot alongside
the existing collected_line so the next commit can arm pending_work_signal
on single-key strokes. No behavior change.

Refs #179."
```

---

## Task 2: Add the single-key arming block (production fix)

This is the actual fix. It adds the new arming block and removes the underscore prefix from `_in_progress_buf`. The five tests in Tasks 3–7 pin its behavior.

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs` — add the new arming block immediately after the `(collected_line, in_progress_buf)` tuple unpacking from Task 1.

**Interfaces:**
- Consumes: `arm_pending_work_signal(&str, tokio::time::Instant) -> PendingWorkSignal` (already imported at `crates/orkworksd/src/runtime/terminal_runtime.rs:1–4`), `handle.active_work_hook` (`bool`), `handle.info.attention` (`Option<String>`), `handle.info.metadata_source` (`Option<String>`), `handle.pending_work_signal` (`Option<PendingWorkSignal>`).
- Produces: armed `pending_work_signal` when the gate conditions hold.

- [ ] **Step 1: Write the failing regression test (single-key acceptance)**

This goes in `crates/orkworksd/src/runtime/session_runtime.rs`, inside the existing `#[cfg(test)] mod tests` block, immediately after the `terminal_input_arms_only_completed_hookless_submission` test (which ends around line 1058, just before the `long_submission_arms_fallback_with_untruncated_echo` test at ~line 1061).

Add this test:

```rust
#[test]
fn single_key_acceptance_at_hook_sourced_needs_you_arms_work_signal() {
    let session_id = "single-key-acceptance";
    let state = test_state_with_runtime_session(session_id);

    // Simulate a Claude Code hook report having set needs_you with agent source.
    {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(session_id).unwrap();
        handle.info.attention = Some("needs_you".into());
        handle.info.metadata_source = Some("agent".into());
    }

    // Single printable keystroke, no Enter.
    assert!(
        crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
            .is_none(),
        "single keystroke without Enter does not produce a completed line"
    );

    // The new arming block must have armed pending_work_signal despite the
    // early return (which is what today's bug looks like: the early return
    // skips the Enter-only arming site at the bottom of record_terminal_input).
    let sessions = state.sessions.lock().unwrap();
    let signal = sessions[session_id]
        .pending_work_signal
        .as_ref()
        .expect("single printable keystroke at hook-sourced needs_you must arm the work signal");
    assert_eq!(
        signal.remaining_echo, "y",
        "echo prefix must be the in-progress input-line buffer snapshot"
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests::single_key_acceptance_at_hook_sourced_needs_you_arms_work_signal -- --nocapture
```

Expected: FAIL with `panicked at 'single printable keystroke at hook-sourced needs_you must arm the work signal'` (the `expect` on `pending_work_signal` returns None today, since the early `let line = collected_line?;` returns before any arming happens).

- [ ] **Step 3: Implement the fix**

In `crates/orkworksd/src/runtime/terminal_runtime.rs`, rename `_in_progress_buf` (from Task 1) back to `in_progress_buf` and add the new arming block immediately after the `(collected_line, in_progress_buf) = { ... };` unwrap. The block goes *before* the `let line = collected_line?;` early-return so single-key strokes without Enter reach it.

Locate the lines (post-Task-1):

```rust
let (collected_line, _in_progress_buf) = {
    let mut bufs = state.peon.input_buf.write().unwrap();
    let buf = bufs.entry(id.to_string()).or_default();
    let line = collect_input_line(buf, data);
    (line, buf.clone())
};
let line = collected_line?;
```

Replace with:

```rust
let (collected_line, in_progress_buf) = {
    let mut bufs = state.peon.input_buf.write().unwrap();
    let buf = bufs.entry(id.to_string()).or_default();
    let line = collect_input_line(buf, data);
    (line, buf.clone())
};

// Single-key arming: a printable keystroke received while the session is in
// needs_you (set by an agent hook report) arms the work fallback using the
// in-progress input-line buffer as the echo prefix. This recovers the
// needs_you -> working transition for Claude Code's single-key prompts
// (y/n and choice lists), which never produce an Enter-terminated
// line. See docs/superpowers/specs/2026-07-17-single-key-work-signal-design.md.
let has_printable = data
    .chars()
    .any(|c| !c.is_whitespace() && !c.is_control());
if has_printable && !in_progress_buf.is_empty() {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(handle) = sessions.get_mut(id) {
        if !handle.active_work_hook
            && handle.info.attention.as_deref() == Some("needs_you")
            && handle.info.metadata_source.as_deref() == Some("agent")
        {
            handle.pending_work_signal = Some(arm_pending_work_signal(
                &in_progress_buf,
                tokio::time::Instant::now(),
            ));
        }
    }
}

let line = collected_line?;
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests::single_key_acceptance_at_hook_sourced_needs_you_arms_work_signal -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run the full session_runtime test module to confirm no regressions**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests -- --nocapture
```

Expected: all tests pass, including the existing `terminal_input_arms_only_completed_hookless_submission`, `long_submission_arms_fallback_with_untruncated_echo`, and `terminal_input_preserves_observed_attention_in_memory_and_metadata`.

- [ ] **Step 6: Run clippy on the crate**

```bash
cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings
```

Expected: clean.

- [ ] **Step 7: Do NOT commit yet — leave the working tree dirty for Task 3**

Tasks 3–7 add the remaining four pin tests in the same file. The full set will be committed together at the end of Task 7, mirroring how the production code change and its complete test suite ship as one logical unit. (If you prefer one commit per test, run `git commit` here with message `fix(sidecar): arm work signal on single-key acceptance at hook-sourced needs_you` and then amend after Task 7 — but the default is to commit once at the end.)

---

## Task 3: Pin that multi-char + Enter still promotes (regression guard)

This test pins acceptance criterion 2 from issue #179: multi-char submissions via Enter continue to work as today. It's a sync unit test mirroring the existing `terminal_input_arms_only_completed_hookless_submission` test's style. It also implicitly confirms the new single-key block doesn't double-arm on Enter — because `collect_input_line` clears the buffer on Enter, `in_progress_buf` will be empty when the new block runs, so `!in_progress_buf.is_empty()` fails and the new block is a no-op.

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs` — append to the `#[cfg(test)] mod tests` block, after the test added in Task 2.

- [ ] **Step 1: Write the test**

```rust
#[test]
fn multi_char_then_enter_arms_signal_via_existing_path_not_single_key() {
    let session_id = "multi-char-enter";
    let state = test_state_with_runtime_session(session_id);

    {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(session_id).unwrap();
        handle.info.attention = Some("needs_you".into());
        handle.info.metadata_source = Some("agent".into());
    }

    // First keystroke without Enter — single-key block arms with prefix "fix".
    assert!(crate::runtime::terminal_runtime::record_terminal_input(
        &state,
        session_id,
        "fix"
    )
    .is_none());
    {
        let sessions = state.sessions.lock().unwrap();
        let signal = sessions[session_id]
            .pending_work_signal
            .as_ref()
            .expect("first printable keystroke should arm via single-key block");
        assert_eq!(signal.remaining_echo, "fix");
    }

    // Enter submits the line — the existing Enter-terminated arming path fires
    // and re-arms with the full committed line. collect_input_line clears the
    // in-progress buffer first, so the single-key block's
    // !in_progress_buf.is_empty() check is false — no double-arm, no stale
    // shorter prefix. The Enter-path's arm wins with the correct full line.
    crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "\r")
        .expect("Enter submits the line and record_terminal_input returns Some(())");
    {
        let sessions = state.sessions.lock().unwrap();
        let signal = sessions[session_id]
            .pending_work_signal
            .as_ref()
            .expect("Enter-terminated submission should arm the work signal");
        assert_eq!(
            signal.remaining_echo, "fix",
            "Enter-path must re-arm with the committed line, not the stale single-key prefix"
        );
    }
}
```

- [ ] **Step 2: Run the test**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests::multi_char_then_enter_arms_signal_via_existing_path_not_single_key -- --nocapture
```

Expected: PASS. (If it fails, the single-key path is double-arming or the Enter-path is no longer firing — investigate before continuing.)

- [ ] **Step 3: Do NOT commit yet — continue to Task 4**

---

## Task 4: Pin no noise on working (acceptance criterion 3)

Pins that keystrokes during a `working` session don't re-arm the signal.

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs` — append after Task 3's test.

- [ ] **Step 1: Write the test**

```rust
#[test]
fn single_key_does_not_re_arm_when_attention_is_working() {
    let session_id = "single-key-no-noise-on-working";
    let state = test_state_with_runtime_session(session_id);

    // Session is already working (process-sourced — e.g. the model IS generating).
    {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(session_id).unwrap();
        handle.info.attention = Some("working".into());
        handle.info.metadata_source = Some("process".into());
    }

    // A printable keystroke arrives mid-working. It must NOT arm a work signal
    // — the session is already working and re-arming would introduce noise.
    assert!(
        crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
            .is_none()
    );

    let sessions = state.sessions.lock().unwrap();
    assert!(
        sessions[session_id].pending_work_signal.is_none(),
        "keystroke during working must not re-arm the work signal"
    );
}
```

- [ ] **Step 2: Run the test**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests::single_key_does_not_re_arm_when_attention_is_working -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Do NOT commit yet — continue to Task 5**

---

## Task 5: Pin narrow scope (Peon-sourced needs_you doesn't arm)

Pins that the `metadata_source == "agent"` gate excludes Peon-detected `needs_you` (which has `metadata_source == "peon"`).

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs` — append after Task 4's test.

- [ ] **Step 1: Write the test**

```rust
#[test]
fn single_key_does_not_arm_when_needs_you_is_peon_sourced() {
    let session_id = "single-key-not-for-peon-needs-you";
    let state = test_state_with_runtime_session(session_id);

    // Peon scraped the terminal and inferred needs_you; the metadata source is
    // "peon", not "agent" — the narrow-scope gate must exclude it so shell-mode
    // sessions where the terminal echoes each keystroke don't false-positive.
    {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(session_id).unwrap();
        handle.info.attention = Some("needs_you".into());
        handle.info.metadata_source = Some("peon".into());
    }

    assert!(
        crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
            .is_none()
    );

    let sessions = state.sessions.lock().unwrap();
    assert!(
        sessions[session_id].pending_work_signal.is_none(),
        "Peon-sourced needs_you must not arm via the single-key path"
    );
}
```

- [ ] **Step 2: Run the test**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests::single_key_does_not_arm_when_needs_you_is_peon_sourced -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Do NOT commit yet — continue to Task 6**

---

## Task 6: Pin capable-hook session doesn't arm via single key

Pins that the `!handle.active_work_hook` gate excludes capable-hook harnesses — only the hook drives `working` for them, per the parent spec.

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs` — append after Task 5's test.

- [ ] **Step 1: Write the test**

```rust
#[test]
fn single_key_does_not_arm_when_active_work_hook_is_true() {
    let session_id = "single-key-not-for-capable-hook";
    let state = test_state_with_runtime_session(session_id);

    {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(session_id).unwrap();
        handle.active_work_hook = true;
        handle.info.attention = Some("needs_you".into());
        handle.info.metadata_source = Some("agent".into());
    }

    assert!(
        crate::runtime::terminal_runtime::record_terminal_input(&state, session_id, "y")
            .is_none()
    );

    let sessions = state.sessions.lock().unwrap();
    assert!(
        sessions[session_id].pending_work_signal.is_none(),
        "capable-hook sessions must not arm via the single-key path (hook-driven only)"
    );
}
```

- [ ] **Step 2: Run the test**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests::single_key_does_not_arm_when_active_work_hook_is_true -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Do NOT commit yet — continue to Task 7**

---

## Task 7: End-to-end promotion test (live PTY) and final verification

Tasks 3–6 pinned the arming gate with synchronous unit tests. This task adds the end-to-end test: single-key at hook-sourced `needs_you`, then visible PTY output, then `attention == "working"` + `metadata_source == "process"`. It's a `#[tokio::test]` using the live-PTY `start_session_runtime` pattern, mirroring `output_within_startup_grace_is_replayed_without_marking_attention_working` (session_runtime.rs:1199).

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs` — append after Task 6's test.

- [ ] **Step 1: Write the end-to-end test**

```rust
#[tokio::test]
async fn single_key_at_hook_sourced_needs_you_promotes_to_working_on_visible_output() {
    let dir = tempfile::tempdir().unwrap();
    let session_id = "single-key-e2e-promote";
    let state = test_state_with_runtime_session(session_id);

    // Simulate a hook report: needs_you, metadata_source=agent.
    {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(session_id).unwrap();
        handle.info.attention = Some("needs_you".into());
        handle.info.metadata_source = Some("agent".into());
        handle.info.lifecycle = "alive".into();
    }

    // Single printable keystroke arms the work signal via the new block.
    assert!(crate::runtime::terminal_runtime::record_terminal_input(
        &state,
        session_id,
        "y"
    )
    .is_none());

    {
        let sessions = state.sessions.lock().unwrap();
        assert!(
            sessions[session_id].pending_work_signal.is_some(),
            "single-key arming must have fired before PTY output"
        );
    }

    // Spin up a real PTY that sleeps briefly (past the 2s startup grace) then
    // emits visible output. The output will flow through start_session_runtime's
    // DriverEvent::Output handler, which calls consume_pending_work_signal and
    // promotes attention to working + metadata_source to process.
    let (runtime, control_rx) =
        SessionRuntime::live(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS);
    let output_tx = runtime.output_tx.clone();
    let mut events = output_tx.subscribe();

    let command = harness::CommandSpec {
        program: "/bin/sh".into(),
        args: vec![
            "-lc".into(),
            "sleep 2.1; printf 'model-output-after-single-key\\n'; sleep 1".into(),
        ],
        cwd: dir.path().display().to_string(),
    };

    {
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(session_id).unwrap();
        handle.command = command.clone();
        handle.runtime = runtime;
    }

    let (kill_tx, kill_rx) = tokio::sync::watch::channel(false);
    start_session_runtime(
        state.clone(),
        session_id.to_string(),
        command,
        None,
        control_rx,
        output_tx,
        kill_rx,
        PtySize {
            rows: DEFAULT_TERMINAL_ROWS,
            cols: DEFAULT_TERMINAL_COLS,
            pixel_width: 0,
            pixel_height: 0,
        },
    )
    .await
    .unwrap();

    // Wait for the model-output marker to arrive. 3s window covers the 2.1s
    // sleep + printf + runtime latency.
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match events.recv().await {
                Ok(RuntimeEvent::Output { chunk, .. })
                    if String::from_utf8_lossy(&chunk).contains("model-output-after-single-key") =>
                {
                    break;
                }
                Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(error) => panic!("unexpected runtime event error: {error}"),
            }
        }
    })
    .await
    .expect("model output should arrive within the 3s window");

    // Yield once so the runtime's output handler finishes its metadata write.
    tokio::task::yield_now().await;

    let sessions = state.sessions.lock().unwrap();
    let handle = sessions.get(session_id).unwrap();
    assert_eq!(
        handle.info.attention.as_deref(),
        Some("working"),
        "single-key acceptance + visible output must promote to working"
    );
    assert_eq!(
        handle.info.metadata_source.as_deref(),
        Some("process"),
        "promotion sets metadata_source=process, ending the agent-source gate"
    );
    assert!(
        handle.pending_work_signal.is_none(),
        "consumed qualifying work signal must be cleared"
    );
    drop(sessions);

    kill_tx.send(true).unwrap();
}
```

- [ ] **Step 2: Run the new end-to-end test**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --lib runtime::session_runtime::tests::single_key_at_hook_sourced_needs_you_promotes_to_working_on_visible_output -- --nocapture
```

Expected: PASS. (This may take ~3s because of the `sleep 2.1` in the PTY child.)

- [ ] **Step 3: Run the full test suite for the crate**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: all tests pass. Pay particular attention to:
- `runtime::session_runtime::tests::*` — all 5 new tests + the existing ones
- `runtime::session_runtime::tests::output_within_startup_grace_is_replayed_without_marking_attention_working`
- `runtime::session_runtime::tests::partial_then_qualifying_hookless_terminal_input_and_output_promote_memory_and_metadata`
- `runtime::session_runtime::tests::capable_hook_sessions_never_infer_working_from_pty_output`
- `http::session_handlers::tests::*` — the existing `report_attention_*` tests must still pass.

- [ ] **Step 4: Run clippy with -D warnings**

```bash
cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings
```

Expected: clean.

- [ ] **Step 5: Commit the production fix + all five tests**

```bash
git add crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/runtime/session_runtime.rs
git commit -m "fix(sidecar): arm work signal on single-key acceptance at hook-sourced needs_you

Restore the needs_you -> working attention transition for Claude Code
sessions where the user answers with a single keystroke (y/n, choice lists,
choice keys). Today, record_terminal_input early-returns when collect_input_line
returns None (no Enter), so pending_work_signal is never armed and the
session sticks on needs_you indefinitely.

Arm pending_work_signal with the in-progress input-line buffer as the echo
prefix when a printable keystroke arrives while attention=needs_you and
metadata_source=agent. The narrow metadata_source=agent gate excludes
Peon-detected needs_you (sidesteps the shell-mode echo false-positive) and
capable-hook sessions (those stay hook-driven per the parent spec).

Tests:
- single_key_acceptance_at_hook_sourced_needs_you_arms_work_signal
- multi_char_then_enter_arms_signal_via_existing_path_not_single_key
- single_key_does_not_re_arm_when_attention_is_working
- single_key_does_not_arm_when_needs_you_is_peon_sourced
- single_key_does_not_arm_when_active_work_hook_is_true
- single_key_at_hook_sourced_needs_you_promotes_to_working_on_visible_output

Fixes #179."
```

---

## Task 8: Push the branch, open PR, request code review

This is the PR-handoff task. No code changes — just publishing and the mandatory `/code-review` gate per AGENTS.md.

**Files:** none.

- [ ] **Step 1: Push the branch**

```bash
git push -u origin fix/single-key-work-signal
```

- [ ] **Step 2: Open the PR**

Use `gh pr create` with a description that:
- Names the fix and links #179 (use `Fixes #179` in the body so it auto-closes on merge).
- Lists the 6 new tests by name.
- Notes the spec doc commit (28945aa) — the design lives in `docs/superpowers/specs/2026-07-17-single-key-work-signal-design.md`.
- Notes this is a targeted regression fix for #177 (gate Working state on harness activity), scoped to hook-sourced `needs_you` so it doesn't reopen the false positives #177 closed.
- Confirms the two agentic spec reviews passed (PROCEED-TO-PLAN).

Example:

```bash
gh pr create \
  --title "fix(sidecar): arm work signal on single-key acceptance at hook-sourced needs_you (#179)" \
  --body "## Summary

Restore the \`needs_you → working\` attention transition for Claude Code sessions where the user answers the prompt with a single printable keystroke (e.g. \`y\`/\`n\` or a choice key). #177 (gate Working state on harness activity) tightened the fallback to require an Enter-terminated line; Claude Code's prompts don't take Enter, so the session stuck on \`needs_you\` indefinitely after such an answer.

The fix arms \`pending_work_signal\` on any printable keystroke received while \`attention = needs_you\` and \`metadata_source = agent\` (i.e. the Claude Code hook path). The narrow \`metadata_source = agent\` gate excludes Peon-detected \`needs_you\` (sidesteps the shell-mode echo false-positive) and capable-hook sessions (those stay hook-driven per the parent spec). The existing Enter-terminated arming path is unchanged.

Design: \`docs/superpowers/specs/2026-07-17-single-key-work-signal-design.md\` — two agentic review passes, both PROCEED-TO-PLAN.

Fixes #179.

## Tests added

- \`single_key_acceptance_at_hook_sourced_needs_you_arms_work_signal\` — sync unit test pinning the arming gate
- \`multi_char_then_enter_arms_signal_via_existing_path_not_single_key\` — regression guard for the Enter-terminated path
- \`single_key_does_not_re_arm_when_attention_is_working\` — no noise on working
- \`single_key_does_not_arm_when_needs_you_is_peon_sourced\` — narrow scope
- \`single_key_does_not_arm_when_active_work_hook_is_true\` — capable-hook gate
- \`single_key_at_hook_sourced_needs_you_promotes_to_working_on_visible_output\` — end-to-end live-PTY test

## Verification

- [x] \`cargo test --manifest-path crates/orkworksd/Cargo.toml\` green
- [x] \`cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings\` clean
- [x] Spec agentic reviews: PROCEED-TO-PLAN (both passes)
- [ ] \`/code-review\` run before merge — AGENTS.md requires it for PRs touching \`crates/orkworksd/\`

## Out of scope

- Wiring a true \`working\`/\`thinking\` signal from Claude Code's Notification hook (#71).
- Recovering Peon-detected \`needs_you\` on TUI harnesses (their \`metadata_source\` is \`peon\`, the narrow gate excludes them)."
```

- [ ] **Step 3: Run `/code-review` on the PR**

Per AGENTS.md, this PR touches \`crates/orkworksd/\` and must have a \`/code-review\` run before merge. Run it from this OpenCode session with the PR URL or branch name as the target. Default to lightweight review (it's a ~30-line production change plus 6 focused tests, no concurrency/lifecycle/protocol/security work — medium review is not warranted by the rubric in AGENTS.md).

- [ ] **Step 4: Address any review findings or note why each is intentional in the PR description**

If review surfaces findings, apply fixes as a follow-up commit on the same branch. If a finding is intentional (e.g. a design constraint from the spec), note why in the PR description rather than "fixing" it.

---

## Self-Review

**Spec coverage skim:** All six behaviors the spec calls out are pinned by tests:
- §Mechanism (the new arming block) → Task 2 test #1 + Task 7 end-to-end test
- §Gates `has_printable && !in_progress_buf.is_empty()` → implicitly covered by Task 4 (single "y" is printable; control sequences would not be — the test covers the positive)
- §Gates `!handle.active_work_hook` → Task 6 test
- §Gates `attention == "needs_you"` → Task 4 test (working doesn't arm)
- §Gates `metadata_source == "agent"` → Task 5 test (peon doesn't arm)
- §Edge case "User answers with Enter for once" → Task 3 test
- §Edge case "Capable-hook harness POSTs waiting_for_input" → Task 6 test
- §Echo-gating (TUI redraws don't promote, visible model output does) → Task 7 end-to-end test (the live PTY emits real output; the test asserts promotion to working)
- §Parent design doc updated in 28945aa — no further spec change needed in this plan

**Placeholder scan:** No TBD/TODO. Every step contains the exact code, the exact test, or the exact command.

**Type consistency:** `pending_work_signal` is `Option<PendingWorkSignal>`. `PendingWorkSignal` has `remaining_echo: String` and `expires_at: tokio::time::Instant` (private fields; tests access them because the test module is in the same crate and `pub(crate)` exposure is sufficient — actually the fields are private to the struct's module. The tests assert on `.remaining_echo` which is a private field. Since the test module is inside the same crate and same module-tree as `session_runtime`, and `PendingWorkSignal` is defined in `session_runtime` (line 25), the test module at the bottom of the same file has access to private fields via normal Rust visibility rules. Confirm by checking: the existing `long_submission_arms_fallback_with_untruncated_echo` test (line ~1061) does exactly this (`signal.remaining_echo.len()`), so the access pattern is proven.

**Scope check:** Single focused fix, one PR, one worktree. Six tests pin the behavior. No supporting infra, no follow-on tasks beyond PR handoff + review.

No issues found. Plan is ready.
