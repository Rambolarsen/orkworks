# Session List Duplicate Prevention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure the desktop session list renders each session ID at most once when polling and create responses overlap.

**Architecture:** Add a small renderer-state helper that merges session snapshots by ID and applies the existing ordering. Route both polling and successful session creation through it. The sidecar API and metadata files remain unchanged.

**Tech Stack:** React, TypeScript, Node built-in test runner.

## Global Constraints

- Keep the Electron renderer and main-process import boundary intact.
- Preserve the existing two-second polling behavior and session ordering.
- Do not change sidecar session lifecycle or metadata behavior.

---

### Task 1: Normalize renderer session snapshots

**Files:**
- Modify: `apps/desktop/src/sessionSort.ts`
- Modify: `apps/desktop/src/App.tsx`
- Test: `apps/desktop/tests/sessionSort.test.ts`

**Interfaces:**
- Produces: `mergeSessionsById(existing: readonly SessionInfo[], incoming: readonly SessionInfo[]): SessionInfo[]`
- Consumes: `sortSessions(list: SessionInfo[]): SessionInfo[]`

- [ ] **Step 1: Write the failing test**

Add a test named `mergeSessionsById keeps one row when a creation response repeats a polled session`. Create an existing session and a new session; call `mergeSessionsById([existing, polledNew], [createdNew])`; assert the IDs are `existing` and `new`, and the retained `new` object is `createdNew`.

- [ ] **Step 2: Run test to verify it fails**

Run `cd apps/desktop && node --experimental-strip-types --test tests/sessionSort.test.ts`. Expect failure because `mergeSessionsById` is not exported.

- [ ] **Step 3: Write minimal implementation**

Export `mergeSessionsById` from `sessionSort.ts`. Seed a `Map` from existing sessions, call `set` for each incoming session, and return `sortSessions` over the map values. In `App.tsx`, pass poll results through `mergeSessionsById([], list)` and merge the creation response with `mergeSessionsById(prev, [session])`.

- [ ] **Step 4: Run test to verify it passes**

Run `cd apps/desktop && node --experimental-strip-types --test tests/sessionSort.test.ts`. Expect no failures.

- [ ] **Step 5: Run frontend verification**

Run `cd apps/desktop && npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`. Expect exit code 0.

- [ ] **Step 6: Commit**

Stage the two source files, the test, this plan, and the design document. Commit with `fix(desktop): deduplicate session snapshots`.
