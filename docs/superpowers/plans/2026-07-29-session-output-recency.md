# Session Output Recency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a live session's most recent terminal output in list recency without changing the semantic `lastActivityAt` timestamp.

**Architecture:** Add `lastOutputAt` to the persisted Rust metadata and independently owned API contracts. Update the live `SessionInfo` on every non-empty PTY frame, coalesce durable metadata writes in the session runtime, and flush once the PTY ends. Renderer helpers choose the newest valid `lastOutputAt` or `lastActivityAt` before using existing fallbacks.

**Tech Stack:** Rust/Axum/portable-pty/Tokio; React/TypeScript; Node built-in test runner.

## Global Constraints

- Keep `lastActivityAt` limited to meaningful situation changes and task-history refreshes.
- `lastOutputAt` advances for every non-empty PTY frame, including a frame without a newline.
- Persist output recency at a bounded cadence and flush the latest pending value before terminal finalization.
- Do not add dependencies or cross the `electron/` / `src/` import boundary.
- Preserve backward compatibility for session JSON without `lastOutputAt`.

---

### Task 1: Define and project output recency through the sidecar contract

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: `crates/orkworksd/src/session_view.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Test: unit tests in the same Rust modules

**Interfaces:**
- Produces: optional JSON field `lastOutputAt` on session payloads and optional JSON field `lastOutputAt` in persisted session metadata.
- Consumes: existing `SessionMetadata::last_activity`, `SessionInfo::last_activity_at`, and session construction helpers.

- [ ] **Step 1: Write failing metadata/API projection tests**

Add a serialization assertion that builds `SessionMetadata { last_output_at: Some("2026-07-29T10:00:00Z".into()), .. }` and checks JSON contains `"lastOutputAt":"2026-07-29T10:00:00Z"`. Add `SessionInfo` and `merge_live_session_info` tests that assert the new field survives live and metadata-backed projections while an omitted legacy field remains `None`.

- [ ] **Step 2: Run the focused Rust tests to verify failure**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml last_output_at`

Expected: compilation failure because `last_output_at` does not yet exist.

- [ ] **Step 3: Add the optional contract fields and initialise every constructor**

Add the serde-mapped fields:

```rust
// metadata.rs
#[serde(rename = "lastOutputAt", skip_serializing_if = "Option::is_none")]
pub last_output_at: Option<String>,

// session_types.rs
#[serde(rename = "lastOutputAt", skip_serializing_if = "Option::is_none")]
pub(crate) last_output_at: Option<String>,
```

Initialise them as `None` in all session/test constructors. In `merge_live_session_info`, carry the most recent live value when present and otherwise use persisted metadata; never substitute it for `last_activity_at`.

- [ ] **Step 4: Run focused Rust tests to verify success**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml last_output_at`

Expected: PASS.

- [ ] **Step 5: Commit the contract change**

```bash
git add crates/orkworksd/src/metadata.rs crates/orkworksd/src/session_types.rs crates/orkworksd/src/session_view.rs crates/orkworksd/src/main.rs crates/orkworksd/src/http/session_handlers.rs
git commit -m "feat: expose session output recency"
```

### Task 2: Update and coalesce output-recency persistence in the PTY runtime

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Test: `crates/orkworksd/src/runtime/session_runtime.rs`

**Interfaces:**
- Consumes: `SessionInfo::last_output_at`, `SessionMetadata::last_output_at`, and `DriverEvent::Output(Vec<u8>)`.
- Produces: immediate in-memory output recency, bounded durable writes, and a final durable flush before the terminal status transition.

- [ ] **Step 1: Write failing runtime tests**

Add deterministic tests around an extracted helper that accepts a non-empty `DriverEvent::Output` frame and timestamp. Pin all of these assertions:

```rust
assert_eq!(handle.info.last_output_at.as_deref(), Some("2026-07-29T10:00:00Z"));
assert_eq!(meta.last_output_at.as_deref(), Some("2026-07-29T10:00:00Z"));
assert_eq!(meta.last_activity, original_last_activity);
```

Cover a frame such as `b"spinner"` (no newline), two frames inside the persistence interval (only the first write occurs), and an exit flush that persists the second frame's timestamp.

- [ ] **Step 2: Run the focused test to verify failure**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_runtime::tests::output_recency`

Expected: FAIL because the runtime does not record `lastOutputAt` or coalesce its persistence.

- [ ] **Step 3: Implement one small recency helper and call it from the raw frame path**

In `DriverEvent::Output(data)`, before `drain_persist_records`, record `iso_now()` whenever `!data.is_empty()`. Update `handle.info.last_output_at` under the existing sessions lock. Track the last durable write instant plus a dirty timestamp in `SessionRuntime`; write `meta.last_output_at` only when the configured interval has elapsed. On `Exited` and `WaitError`, flush a dirty value before calling `set_session_status`, so finalization cannot overwrite a newer unpersisted output timestamp.

The helper must update only `last_output_at`; it must not update `last_activity`, Peon state, attention, or summary fields.

- [ ] **Step 4: Run focused runtime tests to verify success**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_runtime::tests::output_recency`

Expected: PASS.

- [ ] **Step 5: Commit the runtime change**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs
git commit -m "fix: persist recent terminal output activity"
```

### Task 3: Use the newest valid recency timestamp in the renderer

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/labels.ts`
- Modify: `apps/desktop/src/sessionGroups.ts`
- Test: `apps/desktop/tests/api.test.ts`
- Test: `apps/desktop/tests/labels.test.ts`
- Test: `apps/desktop/tests/sessionGroups.test.ts`
- Test: `apps/desktop/tests/sessionSort.test.ts`

**Interfaces:**
- Consumes: `SessionInfo.lastOutputAt?: string`, `lastActivityAt?: string`, `peonLastInference?: string`, and `created_at`.
- Produces: `lastActivityTimestamp(session)` that returns the newest valid output/activity timestamp or existing fallback values.

- [ ] **Step 1: Write failing frontend tests**

Add `lastOutputAt?: string` to the API fixture test. In labels tests, pin that output at `12:59:55Z` beats activity at `12:00:00Z`, while activity at `13:00:00Z` beats output at `12:59:55Z`; invalid timestamp strings must be ignored. Add grouping and sorting fixtures that prove they share `lastActivityTimestamp` and choose the same newest timestamp.

- [ ] **Step 2: Run the focused desktop tests to verify failure**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/labels.test.ts tests/sessionGroups.test.ts tests/sessionSort.test.ts`

Expected: FAIL because `SessionInfo` lacks `lastOutputAt` and helpers do not compare the two timestamps.

- [ ] **Step 3: Add the frontend field and a single valid-timestamp selector**

Add `lastOutputAt?: string` to `SessionInfo`. In `labels.ts`, make `lastActivityTimestamp` parse `lastOutputAt` and `lastActivityAt`, return the original ISO string for the later valid value, and only then fall back to `peonLastInference` and `created_at`. Keep `lastActivity` and session grouping/sorting routed through that helper; do not modify `SessionDetailPanel`'s history refresh dependency.

- [ ] **Step 4: Run focused desktop tests to verify success**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/labels.test.ts tests/sessionGroups.test.ts tests/sessionSort.test.ts`

Expected: PASS.

- [ ] **Step 5: Commit the renderer recency change**

```bash
git add apps/desktop/src/api.ts apps/desktop/src/labels.ts apps/desktop/src/sessionGroups.ts apps/desktop/tests/api.test.ts apps/desktop/tests/labels.test.ts apps/desktop/tests/sessionGroups.test.ts apps/desktop/tests/sessionSort.test.ts
git commit -m "fix: show newest session output activity"
```

### Task 4: Document the metadata and verify the complete change

**Files:**
- Modify: `docs/agents/domain-entities.md`
- Modify: `docs/agents/architecture.md`
- Test: full existing Rust and desktop suites

**Interfaces:**
- Documents: `lastOutputAt` as raw terminal-output recency and `lastActivityAt` as meaningful situation recency.

- [ ] **Step 1: Update documentation**

Add `last_output_at` to the `SessionMetadata` field list in `docs/agents/domain-entities.md`. Update the API-flow paragraph in `docs/agents/architecture.md` to distinguish terminal-output recency from meaningful situation/history activity and state that session-list recency selects the newer valid timestamp.

- [ ] **Step 2: Run complete verification**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
cd apps/desktop && npx tsc --noEmit
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
git diff --check
```

Expected: every command exits 0; documentation and worktree checks have no unaddressed findings owned by this branch.

- [ ] **Step 3: Request lightweight code review**

Review the diff against this plan, focusing on raw-frame updates, bounded persistence, final flushing, backward compatibility, and renderer timestamp selection.

- [ ] **Step 4: Commit documentation and verification-ready state**

```bash
git add docs/agents/domain-entities.md docs/agents/architecture.md docs/superpowers/plans/2026-07-29-session-output-recency.md
git commit -m "docs: explain session output recency"
```
