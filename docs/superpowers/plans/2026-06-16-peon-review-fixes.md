# Peon Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the latest review findings around Peon observer metadata, subprocess invocation, duplicate inference, priority preservation, and stale docs.

**Architecture:** Runtime lifecycle remains in `status`; Peon writes separate optional observer fields used by the UI for attention state. Peon tracks in-flight inference per session, respects metadata priority using file modification age, and sends prompts to harness stdin with structured argv.

**Tech Stack:** Rust sidecar with serde/axum/tokio, React/TypeScript frontend tests via Node runner, project docs in Markdown.

---

### Task 1: Peon Metadata Contract

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/peon.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `apps/desktop/src/api.ts`

- [ ] Write failing Rust tests proving Peon preserves lifecycle `status`, writes `observedStatus`, and includes observer fields in session listings.
- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml test_peon_inference_writes_metadata`.
- [ ] Add optional observer fields to Rust metadata/session structs and merge Peon inference into those fields.
- [ ] Update `/sessions` mapping to read observer fields from metadata.
- [ ] Run targeted Rust tests and confirm pass.

### Task 2: Priority And In-Flight Guard

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/peon.rs`

- [ ] Write failing tests for fresh agent metadata not being overwritten and one in-flight inference per session.
- [ ] Run targeted Rust tests and confirm expected failures.
- [ ] Add `in_flight` tracking to `PeonState`, mark before spawn, clear after completion.
- [ ] Add metadata file age lookup and use it in `should_overwrite`.
- [ ] Run targeted Rust tests and confirm pass.

### Task 3: Safe Harness Invocation

**Files:**
- Modify: `crates/orkworksd/src/peon.rs`
- Modify: `crates/orkworksd/tests/mock-peon-harness.sh` if needed

- [ ] Write failing tests proving `PEON_HARNESS_ARGS_JSON` is parsed and the prompt arrives on stdin, not argv.
- [ ] Run targeted Rust tests and confirm expected failures.
- [ ] Change `PeonConfig.harness_args` to `Vec<String>`, parse `PEON_HARNESS_ARGS_JSON`, pipe prompt to stdin, and remove argv prompt passing.
- [ ] Run targeted Rust tests and confirm pass.

### Task 4: Frontend Observer Attention

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/components/RightSidebarHelpers.ts`
- Modify: `apps/desktop/src/components/LeftSidebar.tsx`
- Modify: `apps/desktop/src/components/RightSidebar.tsx`
- Modify: `apps/desktop/tests/rightSidebar.test.ts`
- Modify: `apps/desktop/tests/api.test.ts`

- [ ] Write failing frontend tests for `observedStatus` attention/sorting and `SessionInfo` observer fields.
- [ ] Run Node frontend tests and confirm expected failures.
- [ ] Update helpers and sidebars to use `observedStatus ?? status` for attention/sorting while retaining lifecycle status display.
- [ ] Run frontend tests and typecheck.

### Task 5: Docs And Verification

**Files:**
- Modify: `docs/agents/architecture.md`
- Modify: `docs/superpowers/specs/2026-06-16-m6-peon-design.md` if implementation details changed

- [ ] Fix architecture doc dirty-check statement.
- [ ] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml`.
- [ ] Run `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` from `apps/desktop`.
- [ ] Run `pnpm --dir apps/desktop exec tsc --noEmit`.
- [ ] Run `bash .claude/hooks/doc-check.sh`.
