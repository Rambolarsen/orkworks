# Peon Idle Scheduling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop per-session Peon inference after effective idle and resume it only from recorded, non-sensitive user input, while showing the runtime scheduler state under the existing debug-metadata toggle.

**Architecture:** Replace Peon's independently locked scheduling maps with one in-memory `PeonScheduler` guarded by one lock. Each normal inference gets a scheduler epoch lease. Finalization invalidates normal leases before changing lifecycle, so a normal inference cannot write provider context or metadata after ending begins. Add only an optional live-session DTO field; the renderer decides whether to display it.

**Tech Stack:** Rust/Tokio/Axum/Serde; Electron/React/TypeScript; Node built-in test runner.

## Global Constraints

- `idle_waiting_for_user_input` is a runtime scheduling hold, not persisted metadata or lifecycle state.
- A qualifying input is a completed, non-sensitive line from `collect_input_line`; short input qualifies, partial/control/sensitive input does not.
- Idle-held terminal output is persisted but neither resumes Peon nor clears effective idle.
- Preserve finalization: one provider scan only for a nonempty final snapshot; otherwise existing fallback.
- Sidecar always returns optional `peonSchedulerState`; renderer gates its display with `showDebugMetadata`.
- Preserve the `electron/`/`src/` import boundary and use pnpm for Node package work.

---

## File structure

| File | Responsibility |
| --- | --- |
| `crates/orkworksd/src/peon.rs` | Scheduler enum, lease, and atomically locked scheduler transitions. |
| `crates/orkworksd/src/main.rs` | Add and initialize the scheduler-state map. |
| `crates/orkworksd/src/runtime/peon_runtime.rs` | Candidate selection, timer idle, inference completion. |
| `crates/orkworksd/src/runtime/session_runtime.rs` | PTY-output and exit transitions. |
| `crates/orkworksd/src/runtime/terminal_runtime.rs` | Qualifying input and finalization coordination. |
| `crates/orkworksd/src/runtime/retention.rs` and `http/session_handlers.rs` | Scheduler cleanup for retention, forget, and delete. |
| `crates/orkworksd/src/session_types.rs`, `http/session_handlers.rs`, and `session_view.rs` | Live DTO field, mapping, and preservation through merged views. |
| `apps/desktop/src/api.ts`, `SessionDetailPanel.tsx` | DTO type and debug-only Details field. |
| `README.md` | Describe the idle hold. |

## Execution prerequisites

- Before editing code, use `starting-work` and `using-git-worktrees` to select an owned branch/worktree; this implementation changes code under `crates/` and `apps/desktop/`, so it requires a branch and PR.
- Check the GitHub issue board. If no scoped issue tracks this approved design, create one with the acceptance criteria from the design before implementation.
- Record an ADR only if the implementation changes the existing sidecar/renderer protocol beyond the optional diagnostic field specified here; otherwise cite this design in the PR and do not create an ADR for an internal scheduler change.
- Execute Tasks 1–5 with `test-driven-development`, request the required lightweight code review before PR handoff, and run `verification-before-completion` plus `.claude/hooks/doc-check.sh` before claiming completion.

### Task 1: Define scheduler state and lifecycle helpers

**Files:**
- Modify: `crates/orkworksd/src/peon.rs:1-105`
- Modify: `crates/orkworksd/src/main.rs:76-130` and every test `PeonState` literal
- Test: inline `#[cfg(test)]` module in `crates/orkworksd/src/peon.rs`

**Interfaces:**
- Produces `pub(crate) enum PeonSchedulerState { WaitingForOutput, Debouncing, Inferring, IdleWaitingForUserInput, FinalScan }`, serialized as `snake_case`.
- Produces `PeonState.scheduler: StdRwLock<PeonScheduler>`; it owns states, last-output deadlines, label hints/pending IDs, in-flight leases, and per-session epochs.
- Produces atomic operations `request_observation_from_output`, `resume_after_qualifying_input`, `claim_due_normal_inference`, `complete_normal_inference`, `hold_idle`, `invalidate_for_ending`, `begin_final_scan`, and `clear_tracking`.

- [ ] **Step 1: Write failing unit tests**

```rust
#[test]
fn idle_hold_removes_normal_scheduling_inputs_atomically() {
    let state = test_peon_state();
    let mut scheduler = state.scheduler.write().unwrap();
    scheduler.request_observation_from_output("s", Instant::now());
    scheduler.resume_after_qualifying_input("s", Some("continue".into()), Instant::now());
    scheduler.hold_idle("s");
    assert_eq!(scheduler.state_for("s"), PeonSchedulerState::IdleWaitingForUserInput);
    assert!(!scheduler.has_pending_normal_work("s"));
}

#[test]
fn scheduler_state_serializes_as_api_value() {
    assert_eq!(serde_json::to_string(&PeonSchedulerState::IdleWaitingForUserInput).unwrap(),
        "\"idle_waiting_for_user_input\"");
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon::tests::idle_hold_removes_normal_scheduling_inputs`

Expected: FAIL because the enum and helpers do not exist.

- [ ] **Step 3: Implement the contract**

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PeonSchedulerState {
    #[default] WaitingForOutput,
    Debouncing, Inferring, IdleWaitingForUserInput, FinalScan,
}

pub(crate) struct PeonScheduler {
    states: HashMap<String, PeonSchedulerState>,
    last_output: HashMap<String, Instant>,
    label_hints: HashMap<String, String>,
    pending_labels: HashSet<String>,
    epochs: HashMap<String, u64>,
    in_flight: HashMap<String, PeonInferenceLease>,
}

pub(crate) fn invalidate_for_ending(&mut self, id: &str) {
    *self.epochs.entry(id.into()).or_default() += 1;
    self.last_output.remove(id);
    self.label_hints.remove(id);
    self.pending_labels.remove(id);
    self.in_flight.remove(id);
    self.states.insert(id.into(), PeonSchedulerState::WaitingForOutput);
}
```

`PeonInferenceLease { id, epoch }` is returned only by `claim_due_normal_inference`. `clear_tracking` removes every per-session scheduler entry. Keep `last_inference` as a display timestamp outside the scheduler.

- [ ] **Step 4: Verify unit tests pass**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon::tests`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/peon.rs crates/orkworksd/src/main.rs
git commit -m "feat(peon): add scheduler state tracking"
```

### Task 2: Hold both inference-idle and timer-idle sessions

**Files:**
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs:5-225`
- Test: inline `#[cfg(test)]` module in `crates/orkworksd/src/runtime/peon_runtime.rs`

**Interfaces:**
- Consumes Task 1's scheduler operations. This task operates only on sessions whose lifecycle remains `active`; Task 3 supplies the ending lease invalidation.
- Produces deterministic candidate selection that excludes `IdleWaitingForUserInput` and inference completion that returns non-idle/failure/timeout to `WaitingForOutput`.
- Test helpers added in this module: `fn state_with_fake_provider(stdout: &str) -> (Arc<AppState>, Arc<AtomicUsize>)`, `async fn run_one_peon_iteration(state: &Arc<AppState>)`, and `fn insert_active_session_with_output(state: &Arc<AppState>, id: &str)`.

- [ ] **Step 1: Write failing async tests**

```rust
#[tokio::test]
async fn idle_inference_is_not_retried_after_elapsed_intervals() {
    let (state, calls) = state_with_fake_provider(r#"{"observedStatus":"idle","confidence":0.9}"#);
    insert_active_session_with_output(&state, "idle-inference");
    run_one_peon_iteration(&state).await;
    run_one_peon_iteration(&state).await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.peon.scheduler.read().unwrap().state_for("idle-inference"), PeonSchedulerState::IdleWaitingForUserInput);
}

#[tokio::test]
async fn timer_idle_is_not_revived_by_terminal_output() {
    let (state, calls) = state_with_fake_provider(r#"{"observedStatus":"working","confidence":0.9}"#);
    insert_active_idle_eligible_session(&state, "timer-idle");
    trigger_idle_timer(&state, "timer-idle").await;
    record_test_terminal_output(&state, "timer-idle", "background text");
    run_one_peon_iteration(&state).await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn priority_rejected_idle_inference_keeps_normal_retry_available() {
    let state = active_session_with_fresh_agent_metadata_and_idle_provider();
    run_one_peon_iteration(&state).await;
    assert_eq!(state.scheduler.read().unwrap().state_for("s"), PeonSchedulerState::WaitingForOutput);
    assert!(state.scheduler.read().unwrap().is_retry_eligible("s"));
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_runtime`

Expected: FAIL because `last_output` is currently reinserted after nonterminal completion and timer idle does not remove it.

- [ ] **Step 3: Implement state-aware scheduling**

Extract one deterministic `run_peon_iteration(&state, now)` from the loop so tests do not depend on sleeping three seconds. `claim_due_normal_inference` excludes idle-held/final-scan IDs and returns a lease while atomically setting `Inferring`. Call `hold_idle` only after an idle inference persists as effective idle; a priority-rejected idle calls `complete_normal_inference` with `WaitingForOutput`. The timer path calls `hold_idle` after writing idle and its startup self-heal skips IDs whose state is `IdleWaitingForUserInput`. New running sessions default to `WaitingForOutput`; disabled Peon reports this state for live sessions but launches no work.

- [ ] **Step 4: Verify the runtime suite passes**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_runtime`

Expected: PASS, including active-lifecycle no-repeat coverage.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/runtime/peon_runtime.rs
git commit -m "fix(peon): hold idle sessions until user input"
```

### Task 3: Integrate output, input, exit, and deletion events

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:450-550`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:430-580`
- Modify: `crates/orkworksd/src/runtime/retention.rs:70-115`
- Modify: `crates/orkworksd/src/http/session_handlers.rs:1007-1075`
- Test: inline tests in each modified Rust runtime module and session-handler tests

**Interfaces:**
- Consumes Task 1 helpers.
- Produces `resume_after_qualifying_input` scheduling and a normal-inference lease guard that prevents all post-ending provider-context and metadata writes.
- Test helpers added in the owning runtime test modules: `record_test_terminal_output`, `send_terminal_input`, `state_with_idle_hold`, and a fake provider with a `tokio::sync::Notify` barrier plus separate normal/final write counters.

- [ ] **Step 1: Write failing tests for each event boundary**

```rust
#[test]
fn idle_held_output_is_persisted_without_clearing_idle_or_scheduling() {
    let state = state_with_idle_hold("held");
    record_test_terminal_output(&state, "held", "still written");
    assert_eq!(state.peon.scheduler.read().unwrap().state_for("held"), PeonSchedulerState::IdleWaitingForUserInput);
    assert_eq!(session_info(&state, "held").observed_status.as_deref(), Some("idle"));
    assert!(!state.peon.scheduler.read().unwrap().has_output_deadline("held"));
}

#[tokio::test]
async fn short_non_sensitive_completed_input_resumes_idle_hold_once() {
    let state = state_with_idle_hold("held");
    send_terminal_input(&state, "held", "go\\r").await;
    assert_eq!(state.peon.scheduler.read().unwrap().state_for("held"), PeonSchedulerState::Debouncing);
    assert!(state.peon.scheduler.read().unwrap().has_output_deadline("held"));
}

#[tokio::test]
async fn partial_control_and_sensitive_input_do_not_resume_idle_hold() {
    let state = state_with_idle_hold("held");
    send_terminal_input(&state, "held", "partial").await;
    send_terminal_input(&state, "held", "\\x1b[A").await;
    mark_recent_output_as_password_prompt(&state, "held");
    send_terminal_input(&state, "held", "secret\\r").await;
    assert_eq!(state.peon.scheduler.read().unwrap().state_for("held"), PeonSchedulerState::IdleWaitingForUserInput);
}

#[tokio::test]
async fn repeated_qualifying_input_coalesces_during_debounce_and_inference() {
    let state = state_with_idle_hold("held");
    send_terminal_input(&state, "held", "first descriptive input\\r").await;
    send_terminal_input(&state, "held", "second descriptive input\\r").await;
    assert_eq!(pending_normal_launches(&state, "held"), 1);
    assert_eq!(pending_prompt_hint(&state, "held"), Some("second descriptive input".into()));
}

#[tokio::test]
async fn ending_session_discards_normal_inference_result_and_finalizes_once() {
    let (state, barrier) = state_with_blocked_provider();
    start_normal_inference(&state, "ending");
    begin_ending(&state, "ending");
    barrier.release();
    await_finalization(&state, "ending").await;
    assert_eq!(normal_provider_context_writes_after_ending(&state, "ending"), 0);
    assert_eq!(normal_metadata_writes_after_ending(&state, "ending"), 0);
    assert_eq!(finalization_paths_started(&state, "ending"), 1);
}
```

- [ ] **Step 2: Verify tests fail**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_runtime`

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_runtime`

Expected: FAIL: output currently clears idle and inserts `last_output`; short input is not scheduled; ending does not reject a normal inference result.

- [ ] **Step 3: Implement the event transitions**

In `session_runtime.rs`, always persist/replay output, but call `request_observation_from_output` and clear terminal observed status only when the atomic scheduler state is not idle-held. In `terminal_runtime.rs`, call `resume_after_qualifying_input` for every completed non-sensitive line, regardless of `HINT_MIN_LEN`; retain a descriptive line as `[User input]` context and schedule short qualifying input without a prompt hint. Coalesce repeated input in `Debouncing` and `Inferring`.

Use this lock order everywhere: **scheduler → sessions → workspace metadata**. On ending, acquire the scheduler lock, invalidate ordinary leases, acquire sessions and mark lifecycle `ending`, then release both before finalization I/O. Before normal completion writes provider context or metadata, hold scheduler, verify its lease epoch is current, then read lifecycle under the sessions lock; perform both writes before releasing the scheduler lock. Thus ending either waits for a pre-existing normal write to finish or invalidates it before it can write. `begin_final_scan` is mutually exclusive with ordinary leases and only sets `FinalScan` for a nonempty snapshot. Add separate tests for nonempty/empty idle-held exits, final-scan state duration, cleanup after finalization/forget/delete/retention, and zero post-ending normal writes.

- [ ] **Step 4: Verify affected Rust suites pass**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_runtime`

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_runtime`

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml retention`

Expected: all commands exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/runtime/retention.rs
git commit -m "fix(peon): resume idle observation only on user input"
```

### Task 4: Expose scheduler state and render it in Debug metadata

**Files:**
- Modify: `crates/orkworksd/src/session_types.rs:6-90`
- Modify: `crates/orkworksd/src/http/session_handlers.rs:709-850`
- Modify: `crates/orkworksd/src/session_view.rs:60-140`
- Modify: every production `SessionInfo` constructor and affected Rust test fixture
- Modify: `apps/desktop/src/api.ts:36-85`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx:164-185`
- Test: `apps/desktop/tests/api.test.ts`, `apps/desktop/tests/dockview.test.ts`, and inline Rust DTO/handler tests

**Interfaces:**
- Produces `SessionInfo.peon_scheduler_state: Option<PeonSchedulerState>` serialized as `peonSchedulerState`.
- Produces TypeScript `PeonSchedulerState` and optional `SessionInfo.peonSchedulerState`.

- [ ] **Step 1: Write failing DTO and UI tests**

```ts
test("SessionInfo accepts optional Peon scheduler state", () => {
  const session: SessionInfo = {
    id: "s", label: "S", status: "running", cwd: "/tmp", created_at: "now",
    memoryState: "live", resumeStrategy: "none",
    peonSchedulerState: "idle_waiting_for_user_input",
  };
  assert.equal(session.peonSchedulerState, "idle_waiting_for_user_input");
});

test("SessionDetailPanel gates Peon scheduler behind debug metadata", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /showDebugMetadata[\\s\\S]*label="Peon scheduler"[\\s\\S]*active\.peonSchedulerState/);
});
```

- [ ] **Step 2: Verify tests fail**

Run: `cd apps/desktop && rtk node --experimental-strip-types --test tests/api.test.ts tests/dockview.test.ts`

Expected: FAIL because the API type and debug DetailField do not exist.

- [ ] **Step 3: Implement DTO mapping and rendering**

```rust
#[serde(rename = "peonSchedulerState", skip_serializing_if = "Option::is_none")]
pub(crate) peon_scheduler_state: Option<peon::PeonSchedulerState>,
```

Snapshot the state map in `list_sessions`; use `WaitingForOutput` when a live session has no explicit entry, and set `None` for remembered-only metadata sessions. Do not add it to `SessionMetadata`. Thread the field through `merge_live_session_info` in `session_view.rs`, then update all production constructors and test fixtures. Add a Rust serialization test that asserts `Some(IdleWaitingForUserInput)` emits `peonSchedulerState` and that `None` is omitted. In `api.ts` define:

```ts
export type PeonSchedulerState =
  | "waiting_for_output" | "debouncing" | "inferring"
  | "idle_waiting_for_user_input" | "final_scan";
```

Inside the existing `showDebugMetadata` fragment add:

```tsx
<DetailField className="detail-fact" label="Peon scheduler">
  {active.peonSchedulerState ?? "Not scheduled"}
</DetailField>
```

- [ ] **Step 4: Verify contracts pass**

Run: `cd apps/desktop && rtk node --experimental-strip-types --test tests/api.test.ts tests/dockview.test.ts`

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_types session_handlers session_view`

Expected: PASS; debug-off omits the field, debug-on renders it, and live/remembered serialization follows the optional-field contract.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/session_types.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/session_view.rs apps/desktop/src/api.ts apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/tests/api.test.ts apps/desktop/tests/dockview.test.ts
git commit -m "feat(peon): show scheduler state in debug details"
```

### Task 5: Document and verify the completed behavior

**Files:**
- Modify: `README.md:114-129`
- Test: complete Rust and frontend suites

**Interfaces:**
- Consumes Tasks 1-4.
- Produces current documentation for idle scheduling.

- [ ] **Step 1: Update README Peon behavior**

Replace the activity sentence with:

```markdown
After Peon or the idle timer marks a session idle, ongoing observation stops.
Terminal output remains persisted but does not resume Peon; a completed,
non-sensitive user input line resumes observation. The bounded final scan on
session exit remains unchanged.
```

- [ ] **Step 2: Run complete verification**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml`

Run: `cd apps/desktop && rtk node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`

Run: `cd apps/desktop && rtk npx tsc --noEmit`

Expected: all commands exit 0.

- [ ] **Step 3: Run diff and documentation checks**

Run: `rtk git diff --check`

Run: `rtk bash .claude/hooks/doc-check.sh`

Expected: no whitespace errors and no unresolved documentation prompts.

- [ ] **Step 4: Commit documentation**

```bash
git add README.md
git commit -m "docs(peon): explain idle scheduling hold"
```
