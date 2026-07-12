# Simplified Session Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the frontend-facing session state matrix with lifecycle `creating → alive → stopping → dead` and alive-only attention `working | idle | needs_you | blocked | failed | capped` without weakening terminal finalization or crash recovery.

**Architecture:** Persist and expose canonical lifecycle and attention fields while temporarily deriving legacy fields for older desktop builds. The sidecar remains the only lifecycle authority: it records a pending outcome and snapshot when entering `stopping`, makes one bounded final scan, then persists a `dead` session. The renderer renders lifecycle and attention separately; it never treats legacy process status as an attention state.

**Tech Stack:** Rust, Axum, Serde, Tokio, React, TypeScript, Node built-in test runner.

## Global Constraints

- Preserve the `apps/desktop/electron/` and `apps/desktop/src/` import boundary; duplicate IPC contract types when both sides need them.
- Do not add dependencies.
- Keep Peon observer-only: it never writes terminal input.
- Keep terminal finalization idempotent, preserve final snapshots, and reconcile a daemon-restart orphan from persisted pending data.
- The normal UI must not render `running`, `done`, `stale`, `creating`, `stopping`, `dead`, `ended`, or `killed` as a persistent session label.
- The terminal outcome and snapshots remain available in debug/history data.
- Use pnpm for desktop commands.

---

## File map

| File | Responsibility |
| --- | --- |
| `docs/adr/0023-simplified-session-lifecycle.md` | Records the replacement of ADR 0021’s public vocabulary while retaining its finalization invariants. |
| `docs/adr/0021-session-lifecycle-phases.md` | Marked superseded by ADR 0023. |
| `docs/adr/README.md` | Adds ADR 0023 to the index. |
| `docs/agents/domain-entities.md` | Documents canonical lifecycle, attention, outcome, and snapshot meanings. |
| `crates/orkworksd/src/domain/session/value_objects.rs` | Defines canonical `SessionLifecycle` and reduced `AttentionState` vocabulary. |
| `crates/orkworksd/src/domain/session/entity.rs` | Owns `creating → alive → stopping → dead` transitions. |
| `crates/orkworksd/src/metadata.rs` | Persists canonical fields, normalizes legacy records, and recovers orphaned stopping sessions. |
| `crates/orkworksd/src/session_types.rs` and `session_view.rs` | Projects canonical API fields and derives temporary legacy compatibility fields. |
| `crates/orkworksd/src/runtime/terminal_runtime.rs` | Makes terminal activity, stopping, final scans, and dead finalization use canonical fields. |
| `crates/orkworksd/src/runtime/peon_runtime.rs` and `peon.rs` | Restricts Peon attention vocabulary and handles idle/activity/capacity precedence. |
| `crates/orkworksd/src/http/session_handlers.rs` | Creates/resumes sessions with canonical lifecycle and consumes canonical state at HTTP boundaries. |
| `apps/desktop/src/api.ts` | Replaces frontend presentation inputs with required `lifecycle` and optional `attention` fields. |
| `apps/desktop/src/sessionSort.ts`, `labels.ts`, `components/SessionListPanel.tsx`, `components/SessionDetailPanel.tsx` | Renders and ranks the simplified model. |
| Rust and desktop test files named below | Pin migration, transitions, API projection, and UI presentation behavior. |

### Task 1: Record the replacement architecture

**Files:**
- Create: `docs/adr/0023-simplified-session-lifecycle.md`
- Modify: `docs/adr/0021-session-lifecycle-phases.md:3-44`
- Modify: `docs/adr/README.md`
- Modify: `docs/agents/domain-entities.md:132-166, 218-230`
- Test: `bash .claude/hooks/doc-check.sh`

**Interfaces:**
- Consumes: the approved design in `docs/superpowers/specs/2026-07-12-simplified-session-lifecycle-design.md`.
- Produces: the authoritative architecture decision needed before Rust or frontend implementation.

- [ ] **Step 1: Write the ADR before implementation code**

Create ADR 0023 with this decision section:

```markdown
## Decision

- Canonical lifecycle is `creating → alive → stopping → dead`.
- Canonical attention is present only when lifecycle is `alive` and is one of
  `working`, `idle`, `needs_you`, `blocked`, `failed`, or `capped`.
- `running`, `done`, and `stale` are not canonical frontend states; legacy
  records normalize them to `alive`, `idle`, and `idle` respectively.
- `stopping` retains ADR 0021’s pending-outcome, bounded final-scan, fallback,
  and orphan-recovery guarantees.
```

- [ ] **Step 2: Supersede ADR 0021 and update the index/domain documentation**

Set ADR 0021’s status to `superseded` and link to ADR 0023. Add a row for 0023 to `docs/adr/README.md`. Replace the four old lifecycle values and old attention vocabulary in `docs/agents/domain-entities.md` with the canonical model and note that terminal outcome/snapshots are history, not live attention.

- [ ] **Step 3: Verify the documentation change**

Run: `git diff --check && bash .claude/hooks/doc-check.sh`

Expected: exit 0; no malformed diff and no unaddressed documentation trigger.

- [ ] **Step 4: Commit the decision**

```bash
git add docs/adr docs/agents/domain-entities.md
git commit -m "docs: simplify session lifecycle architecture"
```

### Task 2: Make canonical lifecycle and attention persistable

**Files:**
- Modify: `crates/orkworksd/src/domain/session/value_objects.rs:20-110`
- Modify: `crates/orkworksd/src/domain/session/entity.rs:1-110`
- Modify: `crates/orkworksd/src/metadata.rs:1-310`
- Modify: `crates/orkworksd/src/infrastructure/session_repository.rs:80-190`
- Test: `crates/orkworksd/src/domain/session/value_objects.rs`
- Test: `crates/orkworksd/src/domain/session/entity.rs`
- Test: `crates/orkworksd/src/metadata.rs`
- Test: `crates/orkworksd/src/infrastructure/session_repository.rs`

**Interfaces:**
- Consumes: ADR 0023 and legacy persisted `status`, `lifecyclePhase`, and `observedStatus` values.
- Produces: `SessionLifecycle::{Creating, Alive, Stopping, Dead}` and `AttentionState::{Working, Idle, NeedsYou, Blocked, Failed, Capped}`; serialized metadata fields `lifecycle` and optional `attention`.

- [ ] **Step 1: Write failing Rust tests for canonical normalization and recovery**

Add table-driven tests that deserialize legacy metadata and assert:

```rust
assert_eq!(meta.lifecycle, SessionLifecycle::Alive);
assert_eq!(meta.attention, Some(AttentionState::Idle)); // legacy stale
assert_eq!(dead.attention, None);
assert_eq!(dead.terminal_outcome.as_deref(), Some("error"));
assert_eq!(dead.final_observed_status_snapshot, Some(canonical_null_snapshot(...)));
```

Cover `creating`, `running/active`, `ending`, terminal statuses, `working`, `idle`, `stale`, `done`, `waiting_for_input`, `blocked`, `failed`, `atUsageLimit`, and `capacityCheckPending`. Assert that `checking_capacity` is retained only as capacity diagnostic data and never serializes as attention.

- [ ] **Step 2: Run the focused tests to prove the new contract is absent**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml metadata::tests:: -- --nocapture`

Expected: FAIL because `SessionMetadata` does not yet expose canonical `lifecycle`/`attention` fields and legacy normalization still emits old values.

- [ ] **Step 3: Implement canonical value objects and metadata projection**

Introduce serde-backed types and fields equivalent to:

```rust
pub enum SessionLifecycle { Creating, Alive, Stopping, Dead }
pub enum AttentionState { Working, Idle, NeedsYou, Blocked, Failed, Capped }

pub struct SessionMetadata {
    #[serde(default, rename = "lifecycle")]
    pub lifecycle: SessionLifecycle,
    #[serde(default, rename = "attention", skip_serializing_if = "Option::is_none")]
    pub attention: Option<AttentionState>,
    // Legacy fields remain read-compatible during the migration.
}
```

Centralize legacy conversion in `normalize_session_metadata`. Enforce `attention = None` unless lifecycle is `Alive`; map `stale` and live `done` to `Idle`; map `waiting_for_input` to `NeedsYou`. Derive legacy `status`, `lifecyclePhase`, `observedStatus`, and `connectivity` only for compatibility instead of using them as canonical storage.

- [ ] **Step 4: Implement aggregate transitions and recovery invariants**

Replace `LifecyclePhase` transitions with `SessionLifecycle` methods:

```rust
fn mark_alive(&mut self) -> Result<(), SessionTransitionError>;
fn begin_stopping(&mut self, outcome: TerminalOutcome, snapshot: ObservedStatusSnapshot)
    -> Result<(), SessionTransitionError>;
fn complete_stopping(&mut self, snapshot: ObservedStatusSnapshot)
    -> Result<(), SessionTransitionError>;
```

`begin_stopping` must durably retain the outcome and captured snapshot. `complete_stopping` must clear attention and pending state, set `Dead`, and preserve a final snapshot. The launch-failure path must create the canonical null final snapshot. `reconcile_orphaned_session` must consume persisted pending data for `Stopping` and finalize it as `Dead`.

- [ ] **Step 5: Run focused Rust tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit the canonical persistence layer**

```bash
git add crates/orkworksd/src/domain/session crates/orkworksd/src/metadata.rs crates/orkworksd/src/infrastructure/session_repository.rs
git commit -m "refactor: persist simplified session lifecycle"
```

### Task 3: Route runtime, Peon, and HTTP state through the canonical model

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:160-430`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs:20-320`
- Modify: `crates/orkworksd/src/peon.rs:320-490`
- Modify: `crates/orkworksd/src/http/session_handlers.rs:200-720`
- Modify: `crates/orkworksd/src/session_types.rs:1-95`
- Modify: `crates/orkworksd/src/session_view.rs:46-215`
- Test: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Test: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Test: `crates/orkworksd/src/http/session_handlers.rs`
- Test: `crates/orkworksd/src/session_view.rs`

**Interfaces:**
- Consumes: canonical persisted state from Task 2.
- Produces: runtime transitions that use `creating/alive/stopping/dead`, API DTO `lifecycle: String` and `attention: Option<String>`, plus compatibility fields derived from them.

- [ ] **Step 1: Write failing runtime tests for transition and attention precedence**

Add tests for:

```rust
assert_eq!(info.lifecycle, "stopping");
assert_eq!(info.attention, None);
assert_eq!(finalized.lifecycle, "dead");
assert_eq!(finalized.attention, None);
assert_eq!(after_output.attention.as_deref(), Some("working"));
```

Include an idempotent double exit, final-scan timeout fallback, orphan recovery, and a confirmed capacity limit followed by terminal output. The last case must prove that terminal output clears displayed `capped` attention while retaining capacity history for diagnostics; a later fresh capacity report may restore `capped`.

- [ ] **Step 2: Run the focused runtime tests and verify failure**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml -- --nocapture`

Expected: FAIL because runtime code still gates on `status == "running"` and `lifecycle_phase == "active"`.

- [ ] **Step 3: Replace runtime status writes with lifecycle transitions**

Rename `set_session_status` to an intent-oriented helper such as `transition_session_lifecycle`. It must:

```rust
// process became usable
Lifecycle::Alive

// process exit/kill/error
Lifecycle::Stopping { pending_terminal_outcome, ending_snapshot }

// bounded scan completion or fallback
Lifecycle::Dead { terminal_outcome, final_snapshot }
```

Continue deriving the old status fields for compatibility at the API/metadata boundary. Update terminal attachment guards and Peon loops to accept only `Alive`; `Stopping` and `Dead` must reject attachment and inference.

- [ ] **Step 4: Reduce Peon output and activity behavior**

Restrict the parser/schema prompt to `needs_you`, `blocked`, `failed`, `capped`, and `idle` for inferred attention; remove `done`, `stale`, and `working` as Peon conclusions. Terminal runtime owns `working`: any accepted output for an alive session sets it, and the idle scheduler sets `idle` after quiescence. Normalize old producer output before merge. Treat `capacityCheckPending` as diagnostic-only; only a confirmed cap writes `Capped`.

- [ ] **Step 5: Project canonical API fields with legacy compatibility**

Extend `SessionInfo` and `merge_live_session_info` so every current sidecar response contains:

```rust
#[serde(rename = "lifecycle")]
pub lifecycle: String,
#[serde(rename = "attention", skip_serializing_if = "Option::is_none")]
pub attention: Option<String>,
```

Keep old `status`, `lifecyclePhase`, `observedStatus`, and `connectivity` fields during the compatibility window, but derive them from canonical state. Ensure `attention` is absent for every non-alive lifecycle.

- [ ] **Step 6: Run focused runtime/API tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml runtime:: http::session_handlers:: session_view:: -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit the runtime/API conversion**

```bash
git add crates/orkworksd/src/runtime crates/orkworksd/src/peon.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/session_types.rs crates/orkworksd/src/session_view.rs
git commit -m "refactor: expose canonical session state"
```

### Task 4: Render canonical lifecycle and attention in the desktop app

**Files:**
- Modify: `apps/desktop/src/api.ts:1-85`
- Modify: `apps/desktop/src/sessionSort.ts:1-55`
- Modify: `apps/desktop/src/labels.ts:1-250`
- Modify: `apps/desktop/src/components/SessionListPanel.tsx:115-205`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx:35-190`
- Modify: `apps/desktop/src/domain/session.ts:1-190`
- Test: `apps/desktop/tests/api.test.ts`
- Test: `apps/desktop/tests/sessionSort.test.ts`
- Test: `apps/desktop/tests/labels.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`
- Test: `apps/desktop/tests/sessionUnread.test.ts`

**Interfaces:**
- Consumes: `SessionInfo.lifecycle: "creating" | "alive" | "stopping" | "dead"` and `SessionInfo.attention?: "working" | "idle" | "needs_you" | "blocked" | "failed" | "capped"` from Task 3.
- Produces: list/detail presentation that has no normal `running`, `done`, `stale`, or terminal-outcome state labels.

- [ ] **Step 1: Write failing desktop tests for the presentation contract**

Add DTO and component-source/behavior tests that assert:

```ts
assert.equal(sessionAttentionStatus(aliveWorking), "working");
assert.equal(sessionAttentionStatus(aliveIdle), "idle");
assert.equal(sessionAttentionStatus(dead), "neutral");
assert.equal(needsAttention({ lifecycle: "alive", attention: "needs_you" }), true);
assert.equal(needsAttention({ lifecycle: "alive", attention: "working" }), false);
```

Assert sorting order: actionable alive sessions, then alive working, then alive idle, then dead. Assert creating/stopping use the transitional indicator and disable conflicting controls. Assert a dead row exposes Resume/Forget without the words `Dead`, `Ended`, or `Killed` in the normal row/detail surface.

- [ ] **Step 2: Run desktop tests to verify failure**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/sessionSort.test.ts tests/labels.test.ts tests/dockview.test.ts tests/sessionUnread.test.ts`

Expected: FAIL because the frontend currently calls `effectiveLifecyclePhase`, reads `status`/`observedStatus`, and maps `done`/`stale`/`running` into UI labels.

- [ ] **Step 3: Replace API and domain presentation inputs**

Define canonical DTO types in `api.ts`:

```ts
export type SessionLifecycle = "creating" | "alive" | "stopping" | "dead";
export type SessionAttention = "working" | "idle" | "needs_you" | "blocked" | "failed" | "capped";

export interface SessionInfo {
  lifecycle: SessionLifecycle;
  attention?: SessionAttention;
  // Keep legacy fields optional only for old-sidecar diagnostics; do not read them for UI.
}
```

Delete `effectiveLifecyclePhase` and replace the renderer’s `Session` lifecycle/attention properties with the canonical names.

- [ ] **Step 4: Implement the simplified rendering and sorting rules**

Make `sessionAttentionStatus` return `neutral` unless `session.lifecycle === "alive"`; otherwise return `session.attention ?? "idle"`. Rank `needs_you`, `blocked`, `failed`, and `capped` above `working` and `idle`; rank every dead session after alive sessions. Map only canonical attention to labels/tones. Render creating/stopping as transient spinner states, hide lifecycle/outcome from normal detail facts, and retain them only under Debug metadata/history.

- [ ] **Step 5: Run the focused desktop test suite**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/sessionSort.test.ts tests/labels.test.ts tests/dockview.test.ts tests/sessionUnread.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit the desktop presentation conversion**

```bash
git add apps/desktop/src apps/desktop/tests
git commit -m "refactor: simplify session state presentation"
```

### Task 5: Verify the cross-layer migration and update user-facing documentation

**Files:**
- Modify: `README.md:25-55`
- Modify: `AGENTS.md:204-245, 266-275`
- Modify: `docs/agents/architecture.md`
- Modify: `docs/agents/domain-entities.md`
- Test: `crates/orkworksd` Rust suite
- Test: `apps/desktop` type-check, test suite, and build

**Interfaces:**
- Consumes: completed canonical backend and renderer changes from Tasks 2–4.
- Produces: accurate project documentation and end-to-end verification evidence.

- [ ] **Step 1: Write a migration regression test across serialization and rendering**

Add a backend API test that serializes an old-format session and asserts the response contains canonical `lifecycle` and correct optional `attention`, alongside compatibility fields. Add a frontend test consuming that response and asserting it uses canonical fields even if legacy `status` conflicts.

- [ ] **Step 2: Run the regression tests and verify failure before the compatibility projection is complete**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_metadata_serializes` and `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts`

Expected: PASS. If either test fails, fix the canonical projection rather than adding a frontend legacy fallback.

- [ ] **Step 3: Update documentation from the implemented contract**

Replace README/AGENTS lifecycle vocabulary with the canonical model, document that normal UI is quiet for dead sessions and that terminal outcome is historical detail, and update architecture/domain docs to match actual API field names and ownership.

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
cd apps/desktop && npx tsc --noEmit
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
cd apps/desktop && pnpm build
git diff --check
bash .claude/hooks/doc-check.sh
```

Expected: every command exits 0. Investigate and correct any failure before claiming completion.

- [ ] **Step 5: Commit documentation and verification fixes**

```bash
git add README.md AGENTS.md docs apps/desktop crates/orkworksd
git commit -m "docs: document simplified session lifecycle"
```
