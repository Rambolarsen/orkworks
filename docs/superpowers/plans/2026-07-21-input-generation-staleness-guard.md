# Input-generation staleness guard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent accepted terminal input from being overwritten by stale Claude attention hooks or in-flight Peon inference, while keeping durable and live session state consistent.

**Architecture:** `SessionHandle` owns a process-lifetime input epoch, accepted-input timestamp, and minimum Peon-output revision. Terminal input advances those boundaries only after PTY write/flush acknowledgment and commits disk plus memory together. The managed Claude reporter captures microsecond UTC `observedAt` before all hook I/O; the handler rejects stale reports before side effects. Peon retains revision-tagged lines and sends state-bearing inference only the post-input segment; scheduler candidates explicitly distinguish output analysis from input-label analysis.

**Tech Stack:** Rust, Axum, Tokio, Chrono, serde, Node-free Rust tests, Bash + Python 3 managed hook reporter.

## Global Constraints

- Preserve `workspace` → `sessions` order whenever both guarded-commit locks are held.
- Do not add a durable metadata field or alter metadata priority rules for non-stale observations.
- `observedAt` is optional for custom reporters; when present it is RFC 3339 UTC with exactly six fractional digits and malformed values return HTTP 400.
- The managed reporter sends its hook-start `observedAt` with the attention POST; comparison is `observed_at <= accepted_input_at`, so a same-microsecond tie favors user input.
- Capture `observedAt` at hook invocation before the harness-session POST, then reuse it for the later attention POST; a hook process scheduled only after input has no upstream event timestamp and therefore remains an unavoidable arrival-order limitation.
- Accepted input means PTY write **and** flush succeeded. Failed delivery or failed metadata persistence must not advance the epoch or publish an in-memory transition.
- Clear `needs_user_input`, `detected_question`, and `suggested_options` in durable metadata and the live projection with the input `working` transition.
- A state-bearing Peon snapshot must contain only revision-tagged lines newer than the input’s recorded boundary; count-only eligibility is insufficient because retained old lines must never reach the inference prompt.
- Candidate scheduling carries `InferenceMode::Output` or `InferenceMode::InputLabel`. `InputLabel` sends only the descriptive input and changes only `SessionInfo::label` in memory; it never calls `merge_peon_inference`, persists provider context, changes metadata source, or updates inference scheduling/bookkeeping.
- Keep active terminal context single; this change must not introduce terminal UI behavior.

---

## File structure

- `crates/orkworksd/src/main.rs` — add per-session staleness-boundary fields to `SessionHandle` and initialize every production/test handle.
- `crates/orkworksd/src/runtime/session_runtime.rs` — retain the taken-over PTY input acknowledgment command shape and test failed/successful delivery.
- `crates/orkworksd/src/runtime/terminal_runtime.rs` — advance and commit the input boundary atomically; clear stale prompt state.
- `crates/orkworksd/src/http/session_handlers.rs` — parse `observedAt`, reject stale hooks before buffer mutation, and commit accepted hooks atomically.
- `crates/orkworksd/scripts/report-claude-session-from-hook.sh` — add the managed reporter timestamp without changing its failure-tolerant behavior.
- `crates/orkworksd/src/peon.rs` — give the Peon ring buffer revision-tagged append and `snapshot_after` operations.
- `crates/orkworksd/src/runtime/session_runtime.rs` — assign Peon output revisions when stripped nonempty PTY lines enter the ring buffer.
- `crates/orkworksd/src/runtime/peon_runtime.rs` — schedule explicit inference modes and pass only the post-boundary output segment to state-bearing inference.
- `crates/orkworksd/src/workspace_runtime.rs` — provide the one UTC microsecond timestamp formatter/parser helper used by the input and hook paths.

### Task 1: Reconcile the taken-over branch with current `main`

**Files:**
- Modify: current uncommitted changes in `crates/orkworksd/src/runtime/session_runtime.rs`
- Modify: current uncommitted changes in `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `docs/superpowers/plans/2026-07-21-input-generation-staleness-guard.md`

**Interfaces:**
- Consumes: `main` commit `15d555e` (`mark committed terminal input working`) and the taken-over acknowledgement changes.
- Produces: a clean branch rebased on current `main`, with the acknowledgement behavior retained exactly once.

- [ ] **Step 1: Save and inspect the takeover diff before changing branch topology**

Run: `rtk git diff -- crates/orkworksd/src/runtime/session_runtime.rs crates/orkworksd/src/runtime/terminal_runtime.rs`

Expected: the diff changes `RuntimeCommand::Input` to carry an optional oneshot sender, adds `send_runtime_input`, and makes write-plus-flush acknowledgment explicit.

- [ ] **Step 2: Commit the current acknowledgement work as its own checkpoint**

```bash
rtk git add crates/orkworksd/src/runtime/session_runtime.rs crates/orkworksd/src/runtime/terminal_runtime.rs
rtk git commit -m "fix(sidecar): acknowledge terminal input delivery"
```

Expected: only the two runtime files are committed; the design and plan commits stay separate.

- [ ] **Step 3: Rebase the takeover branch onto `main` and resolve only equivalent prior-fix overlap**

```bash
rtk git fetch origin
rtk git rebase origin/main
```

When the already-merged input-attention change conflicts, retain the current `main` behavior plus the acknowledgment API. Do not drop the acknowledgment tests.

- [ ] **Step 4: Verify the reconciled baseline**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::session_runtime::tests runtime::terminal_runtime::tests`

Expected: exit code 0 before adding the generation guard.

- [ ] **Step 5: Commit the rebase resolution only if Git required a manual resolution commit**

```bash
rtk git status --short
rtk git add crates/orkworksd/src/runtime/session_runtime.rs crates/orkworksd/src/runtime/terminal_runtime.rs
rtk git commit -m "fix(sidecar): reconcile input delivery with main"
```

Expected: no commit when rebase is clean; otherwise the commit contains only conflict-resolution edits.

### Task 2: Make accepted input an atomic staleness boundary

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Test: `crates/orkworksd/src/runtime/session_runtime.rs`

**Interfaces:**
- Produces on `SessionHandle`: `input_generation: u64`, `accepted_input_at: Option<chrono::DateTime<chrono::Utc>>`, and `min_peon_output_revision: u64`.
- Produces in terminal runtime: `record_dispatched_terminal_input(state, id, data)` advances this boundary only after successful PTY delivery.

- [ ] **Step 1: Add failing tests for the input transition’s full cleanup and persistence-failure behavior**

Add tests that seed both metadata and `SessionInfo` with a waiting prompt, then call accepted-input bookkeeping. Assert all of the following after success:

```rust
assert_eq!(meta.observed_status.as_deref(), Some("working"));
assert_eq!(meta.attention.as_deref(), Some("working"));
assert_eq!(meta.needs_user_input, None);
assert_eq!(meta.detected_question, None);
assert_eq!(meta.suggested_options, None);
assert_eq!(handle.input_generation, 1);
assert!(handle.accepted_input_at.is_some());
assert_eq!(handle.info.needs_user_input, None);
assert_eq!(handle.info.detected_question, None);
assert_eq!(handle.info.suggested_options, None);
assert_eq!(handle.min_peon_output_revision, handle.peon_output_revision);
```

For an induced `try_write_session` failure, assert `input_generation == 0`, `accepted_input_at.is_none()`, and that both persisted and live prompt fields retain their original values.

- [ ] **Step 2: Run the focused tests to verify they fail against the pre-guard transition**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::session_runtime::tests::accepted_input`

Expected: failure because the handle has no boundary fields and prompt fields are not cleared.

- [ ] **Step 3: Add the boundary fields and initialize every `SessionHandle` literal**

```rust
use chrono::{DateTime, Utc};

struct SessionHandle {
    // existing fields
    input_generation: u64,
    accepted_input_at: Option<DateTime<Utc>>,
    peon_output_revision: u64,
    min_peon_output_revision: u64,
}
```

Initialize every constructor/test helper with `input_generation: 0` and `accepted_input_at: None`.

- [ ] **Step 4: Commit metadata and live projection together under the guarded lock order**

In `record_dispatched_terminal_input`, retain the workspace guard while obtaining the sessions guard. Write a cloned metadata record first; only after a successful write mutate the matching alive handle:

```rust
let mut sessions = state.sessions.lock().unwrap();
let Some(handle) = sessions.get_mut(id) else { return; };
let Some(next_generation) = handle.input_generation.checked_add(1) else {
    tracing::warn!(session_id = %id, "input generation overflow");
    return;
};
let accepted_at = Utc::now();
meta.observed_status = Some("working".into());
meta.attention = Some("working".into());
meta.needs_user_input = None;
meta.detected_question = None;
meta.suggested_options = None;
if ws.metadata.try_write_session(&meta).is_err() { return; }
handle.input_generation = next_generation;
handle.accepted_input_at = Some(accepted_at);
handle.min_peon_output_revision = handle.peon_output_revision;
// mirror the metadata fields into handle.info
```

Use `checked_add`; on overflow log and leave the transition unchanged. Capture `Utc::now()` before the durable write and assign it only after the write succeeds, so the stored handle boundary describes the committed transition.

- [ ] **Step 5: Run the focused transition tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::session_runtime::tests::accepted_input`

Expected: exit code 0, including full prompt-field cleanup and failure atomicity.

- [ ] **Step 6: Commit the atomic input boundary**

```bash
rtk git add crates/orkworksd/src/main.rs crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/runtime/session_runtime.rs
rtk git commit -m "fix(sidecar): guard committed input state"
```

### Task 3: Add the managed-hook `observedAt` contract and stale rejection

**Files:**
- Modify: `crates/orkworksd/src/workspace_runtime.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/scripts/report-claude-session-from-hook.sh`
- Test: `crates/orkworksd/src/http/session_handlers.rs`

**Interfaces:**
- `AttentionReportRequest { status: String, message: Option<String>, observed_at: Option<String> }`, with `#[serde(rename = "observedAt")]`.
- `parse_hook_observed_at(&str) -> Result<DateTime<Utc>, ()>` accepts only `YYYY-MM-DDTHH:MM:SS.ffffffZ`.

- [ ] **Step 1: Write failing request-validation and stale-side-effect tests**

Add tests covering: malformed/non-UTC `observedAt` returns `BAD_REQUEST`; omitted `observedAt` returns existing success; an old or same-microsecond timestamp returns `OK` as a rejected stale report without changing metadata or clearing `state.peon.input_buf`; and a newer timestamp persists `needs_you` normally.

```rust
Json(AttentionReportRequest {
    status: "waiting_for_input".into(),
    message: None,
    observed_at: Some("2026-07-21T08:00:00.123456Z".into()),
})
```

- [ ] **Step 2: Run the handler tests and verify the new cases fail**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::session_handlers::tests::report_attention`

Expected: compile/test failure until the request field, parser, and stale gate exist.

- [ ] **Step 3: Implement strict UTC microsecond parsing and hook emission**

```rust
pub(crate) fn parse_hook_observed_at(raw: &str) -> Result<DateTime<Utc>, ()> {
    let parsed = DateTime::parse_from_rfc3339(raw).map_err(|_| ())?;
    let Some((_, fraction_and_z)) = raw.rsplit_once('.') else { return Err(()); };
    let Some(fraction) = fraction_and_z.strip_suffix('Z') else { return Err(()); };
    if fraction.len() != 6 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(());
    }
    Ok(parsed.with_timezone(&Utc))
}
```

At the top of the shell reporter, before reading hook payload or making either
curl, capture the timestamp once. Reuse it when constructing the later
attention payload:

```bash
observed_at="$(python3 -c 'from datetime import datetime, timezone; print(datetime.now(timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z"))')"
attention_payload="$(python3 -c 'import json,sys; print(json.dumps({"status":"waiting_for_input","observedAt":sys.argv[1]}))' "$observed_at")"
```

Keep the existing `curl ... || true` semantics.

Add a shell-level regression test (or an extracted script helper test) that
asserts timestamp capture appears before the harness-session curl and that the
same captured value is supplied to the attention JSON. The handler integration
test must model: timestamp captured, input commits, attention request arrives;
assert it is rejected and leaves both durable and live prompt fields unchanged.

- [ ] **Step 4: Move hook input-buffer mutation after accepted stale validation and durable write**

While holding `workspace` then `sessions`, compare a parsed timestamp to `handle.accepted_input_at`. If stale, return the existing ignored success without metadata or buffer side effects. For an accepted hook, call `merge_agent_attention_signal`, mirror its metadata into the live handle, release the commit locks, then clear only a non-descriptive input buffer.

- [ ] **Step 5: Run the focused handler tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::session_handlers::tests::report_attention`

Expected: exit code 0; stale reports preserve metadata and the input buffer.

- [ ] **Step 6: Commit the hook contract**

```bash
rtk git add crates/orkworksd/src/workspace_runtime.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/scripts/report-claude-session-from-hook.sh
rtk git commit -m "fix(sidecar): reject stale attention hooks"
```

### Task 4: Send Peon only post-input output and separate label-only work

**Files:**
- Modify: `crates/orkworksd/src/peon.rs`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Test: `crates/orkworksd/src/peon.rs`
- Test: `crates/orkworksd/src/runtime/peon_runtime.rs`

**Interfaces:**
- Consumes: revision-tagged ring-buffer lines, `SessionHandle::input_generation`, and `min_peon_output_revision`.
- Produces: `InferenceMode::{Output, InputLabel}` and `PeonCommitOutcome::{Persisted, StaleInput, NoPostInputOutput, LabelUpdated}`. Non-persisted output outcomes remove the obsolete output candidate and release `in_flight`; `LabelUpdated` changes only `handle.info.label`.

- [ ] **Step 1: Write a deterministic stale-inference regression test**

First add a failing `RingBuffer::snapshot_after(revision)` unit test. Seed
revisions 3–6, set the accepted-input boundary to 4, and assert the snapshot
contains only lines 5 and 6. Then extract the Peon commit portion into a
package-visible helper accepting `snapshot_generation`, `snapshot_revision`,
`InferenceMode`, `PeonInference`, and optional `ProviderObservation`. Cover an
in-flight N snapshot after input advances to N+1, and a new N+1 candidate whose
post-boundary segment is empty.

```rust
let outcome = commit_peon_inference(&state, "session", snapshot_generation, &inference, None);
assert_eq!(outcome, PeonCommitOutcome::StaleInput);
assert_eq!(read_meta().observed_status.as_deref(), Some("working"));
assert!(read_meta().detected_question.is_none());
assert!(state.peon.last_inference.read().unwrap().get("session").is_none());
```

Also assert the stale/empty outcomes remove the old `last_output` candidate so
they are not immediately retried. Add a complementary post-boundary segment
proving normal inference persists. For `InputLabel`, seed old attention
metadata and provider context, run the mode, then assert only
`handle.info.label` changes: metadata JSON, `metadata_source`, provider
context, `last_inference`, and `last_output` are byte-for-byte/unmodified.

- [ ] **Step 2: Run the focused Peon tests and verify the stale case fails**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::peon_runtime::tests::stale_input_generation`

Expected: failure because the current ring buffer has no revision-segment API and the runtime combines label and output candidates in one `Vec<String>`.

- [ ] **Step 3: Capture generation with the terminal snapshot and perform the guarded commit**

Change `RingBuffer` to retain `(revision, line)` pairs and add
`snapshot_after(min_revision) -> Vec<String>`. In the PTY output path, assign
the next `peon_output_revision` to each nonempty stripped line before pushing.
At input, store that latest revision as `min_peon_output_revision`.

Replace the scheduler's `Vec<String>` with candidates carrying the explicit
mode. `Output` captures `(generation, latest_revision, output_buffer
snapshot_after(min_peon_output_revision))`; it skips provider inference when
the segment is empty and validates generation again before `merge_peon_inference`.
`InputLabel` sends only `"[User input]: {label}"` to inference and, on a result,
assigns only `handle.info.label = normalized_label`; it does not call metadata
methods or mutate `last_inference`, `last_output`, or provider context.

```rust
if handle.input_generation != snapshot_generation {
    return PeonCommitOutcome::StaleInput;
}
if snapshot_revision <= handle.min_peon_output_revision {
    return PeonCommitOutcome::NoPostInputOutput;
}
```

Keep normal priority checks (`peon_should_overwrite`) unchanged for equal-generation inferences.

- [ ] **Step 4: Run Peon tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::peon_runtime::tests`

Expected: exit code 0, including stale rejection and normal persistence coverage.

- [ ] **Step 5: Commit the Peon guard**

```bash
rtk git add crates/orkworksd/src/peon.rs crates/orkworksd/src/runtime/session_runtime.rs crates/orkworksd/src/runtime/peon_runtime.rs
rtk git commit -m "fix(sidecar): gate Peon state on fresh output"
```

### Task 5: Verify cross-path behavior and update repository records

**Files:**
- Modify if required by output: `docs/agents/domain-entities.md`
- Modify if required by output: `AGENTS.md`, `README.md`
- Verify: all changed Rust files and managed reporter script

**Interfaces:**
- Consumes: completed Tasks 1–4.
- Produces: evidence that accepted input, delayed hooks, and Peon inference cannot diverge disk from memory.

- [ ] **Step 1: Run formatting and the complete Rust suite**

```bash
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: both commands exit 0.

- [ ] **Step 2: Run the exact regression-focused suite once more**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml accepted_input
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml report_attention
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml stale_input_generation
```

Expected: exit code 0 with all input-boundary, hook, and Peon cases passing.

- [ ] **Step 3: Inspect documentation currency and worktree state**

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
rtk git diff --check
rtk git status --short
```

Expected: no whitespace errors; update `docs/agents/domain-entities.md` only if a public session field/vocabulary changed (this design adds only runtime-private fields).

- [ ] **Step 4: Commit final verification/documentation-only adjustments**

```bash
rtk git add AGENTS.md README.md docs/agents/domain-entities.md
rtk git commit -m "docs: record input staleness guard"
```

Skip the commit when no documentation file changed; do not stage unrelated files.

## Self-review

- Spec coverage: Task 2 implements the post-ack input, prompt, and output boundaries; Task 3 implements the delayed-hook boundary and no-side-effect rejection; Task 4 prevents both in-flight and newly-started pre-input Peon inference from seeing or reviving prompt state and isolates label-only work; Task 5 covers failure paths and full verification.
- Placeholder scan: no deferred implementation markers or unspecified error handling remain.
- Type consistency: `input_generation`, `accepted_input_at`, `observed_at`, `parse_hook_observed_at`, and `PeonCommitOutcome::StaleInput` are introduced before their consuming tasks.
