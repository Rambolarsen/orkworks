# Session List Stability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep session rows stable during polling, promoting fresh activity only after the top active session has been quiet for one minute.

**Architecture:** `App` passes its current session order into the existing merge function on every poll. The merge updates known session data in place, appends newly discovered sessions, and promotes the newest actually-updated active session only if the current top active session's newest activity/output timestamp is at least 60 seconds old. Lifecycle ordering still keeps dead sessions below live ones.

**Tech Stack:** React, TypeScript, Node's built-in test runner.

## Global Constraints

- Use the existing `lastActivityTimestamp` helper; add no dependencies or settings.
- Keep renderer and Electron-main boundaries unchanged.
- Run the focused test with `node --experimental-strip-types --test` from `apps/desktop/`.

---

### Task 1: Stable session polling order

**Files:**
- Modify: `apps/desktop/tests/sessionSort.test.ts`
- Modify: `apps/desktop/src/sessionSort.ts`
- Modify: `apps/desktop/src/App.tsx`

**Interfaces:**
- Consumes: `mergeSessionsById(existing, incoming)` and `lastActivityTimestamp(session)`.
- Produces: `mergeSessionsById` retains existing live-row order, except for a qualifying one-minute promotion.

- [ ] **Step 1: Write the failing test**

```ts
test("mergeSessionsById promotes fresh activity only after the top session is quiet for one minute", () => {
  const top = { ...session("top", "alive"), lastOutputAt: "2026-07-31T12:00:30.000Z" };
  const updated = { ...session("updated", "alive"), lastOutputAt: "2026-07-31T12:01:00.000Z" };

  assert.deepEqual(mergeSessionsById([top, updated], [{ ...updated, lastOutputAt: "2026-07-31T12:01:01.000Z" }], new Date("2026-07-31T12:01:01.000Z")).map((item) => item.id), ["top", "updated"]);
  assert.deepEqual(mergeSessionsById([{ ...top, lastOutputAt: "2026-07-31T12:00:01.000Z" }, updated], [{ ...updated, lastOutputAt: "2026-07-31T12:01:01.000Z" }], new Date("2026-07-31T12:01:01.000Z")).map((item) => item.id), ["updated", "top"]);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --experimental-strip-types --test tests/sessionSort.test.ts`

Expected: FAIL because `mergeSessionsById` does not accept the current time and still sorts by timestamp.

- [ ] **Step 3: Write minimal implementation**

```ts
export function mergeSessionsById(existing: readonly SessionInfo[], incoming: readonly SessionInfo[], now = new Date()): SessionInfo[] {
  // Keep the prior sequence; replace known rows and append new rows. Partition
  // alive rows before non-alive rows. Promote only a known alive row with an
  // advanced valid activity timestamp when the current top alive row is 60s old.
}
```

Change `refreshSessions` to call `setSessions((previous) => mergeSessionsById(previous, list))`.

- [ ] **Step 4: Run test to verify it passes**

Run: `node --experimental-strip-types --test tests/sessionSort.test.ts`

Expected: PASS with all session-sort tests green.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/sessionSort.ts apps/desktop/tests/sessionSort.test.ts
git commit -m "fix: stabilize active session ordering"
```
