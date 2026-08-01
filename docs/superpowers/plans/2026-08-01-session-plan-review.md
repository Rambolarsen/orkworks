# Session Plan Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Read a selected session's plan/spec in one Review tab and explicitly submit the fixed review prompt to that live session.

**Architecture:** Keep artifact authority in the sidecar. Reuse `resolve_openable_plan` for canonical containment, expose document content only through a session-ID endpoint, and reuse the existing terminal runtime's input action after Electron-main authenticated approval. The Details card opens the reusable tab; it does not become a queue.

**Tech Stack:** Rust/Axum/portable-pty; Electron IPC; React/Dockview.

## Global Constraints

- Renderer sends only a session ID, never a path or prompt.
- Permit only the fixed review prompt from `specs/session-plan-review.md`; reject control characters before PTY input.
- Keep one active center context: Review is a tab beside Terminal.
- No watcher, repo queue, digest Peon, or generic terminal-write API.

---

### Task 1: Secure session artifact and review-handoff endpoints

**Files:**
- Modify: `crates/orkworksd/src/plan_handoff.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Test: `crates/orkworksd/src/plan_handoff.rs`, `crates/orkworksd/src/http/session_handlers.rs`

- [ ] Write failing tests for terminal-output fallback association (without replacing a harness path), control-character rejection, content returned only for a valid stored artifact, missing/dead session rejection, and bad secret rejection on **both** endpoints.
- [ ] Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml plan` and observe the new tests fail.
- [ ] Detect and validate a printed `docs/superpowers/plans/*.md` or `specs/*.md` path while terminal output is persisted; store it only when no harness path is present. Add authenticated `GET /sessions/:id/plan-content` returning `{content}` only after `resolve_openable_plan` validates the stored relative path. Add authenticated `POST /sessions/:id/request-plan-review`, whose handler validates the live session and artifact and constructs the fixed prompt internally.
- [ ] Extract the existing accepted-input bookkeeping into one shared terminal-runtime submit function. Both the WebSocket and review handler use it; it records Peon side effects and appends the audit event only after PTY acceptance.
- [ ] Register both routes and run the focused tests, then `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml`.

### Task 2: Deliver the action through Electron main

**Files:**
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Modify: `apps/desktop/tests/planOpener.test.ts`

- [ ] Write failing tests that require the review IPC to accept only a non-empty session ID and forward the sidecar secret without returning a path or prompt.
- [ ] Run `rtk node --experimental-strip-types --test tests/planOpener.test.ts` from `apps/desktop` and observe failure.
- [ ] Replace `electron/planOpener.ts` with path-free content/review helpers and replace OS-only `openPlan` with `getPlanContent(sessionId)` and `requestPlanReview(sessionId)` bridges. Electron main calls both secret-authenticated endpoints; neither shell-opening nor arbitrary text is retained.
- [ ] Run the focused test and `rtk npx tsc --noEmit` from `apps/desktop`.

### Task 3: Add the Details card and reusable Review tab

**Files:**
- Create: `apps/desktop/src/components/ReviewPanel.tsx`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/styles.css`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] Write behavioral tests in `labels.test.ts` that require a review card for every `hasOpenablePlan` session and hide only the send action for non-live sessions. Add Dockview tests requiring `review` to share Terminal's group and to be get-or-added when absent from a restored layout.
- [ ] Run the focused test and observe failure.
- [ ] Add a Review panel that clears stale content on session change, asks main for the selected session's content, and renders plain text (`white-space: pre-wrap`). `onReviewPlan` get-or-adds the one `review` panel with `{ referencePanel: "terminal", direction: "within" }`, then activates it. Pass the callback to Details; its card uses status-sensitive copy and offers Review plan plus Ask this agent to review for a live session.
- [ ] Wire the review handoff button to the path-free preload method, show a toast on failure, and run focused tests plus `rtk npx tsc --noEmit`.

### Task 4: Regression and documentation verification

**Files:**
- Modify: `docs/agents/architecture.md`
- Modify: `README.md` only if implementation differs from the approved documentation

- [ ] Document the two session-ID endpoints, Review tab, and the restricted review prompt boundary.
- [ ] Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml`.
- [ ] Run `rtk npx tsc --noEmit` and `rtk node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` from `apps/desktop`.
- [ ] Run `rtk git diff --check`, `bash .claude/hooks/doc-check.sh`, and `bash .claude/hooks/worktree-check.sh`.
