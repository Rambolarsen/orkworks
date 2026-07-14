# Session Grouping by Recent Activity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a resumed session in the current day's session-list group while preserving its original creation timestamp.

**Architecture:** The sidecar already records the resume time in `lastActivityAt`. The renderer will use that valid timestamp as the session-grouping reference, falling back to `created_at` for older or malformed metadata. This is a renderer-only policy change.

**Tech Stack:** React/TypeScript; Node built-in test runner.

## Global Constraints

- Keep the Electron main-process and renderer import boundary intact.
- Do not change the HTTP response contract or persisted metadata schema.
- Preserve local-calendar-day grouping semantics.
- Use `pnpm` for Node package-management tasks.

---

### Task 1: Group sessions by valid recent activity

**Files:**
- Modify: `apps/desktop/tests/sessionGroups.test.ts`
- Modify: `apps/desktop/src/sessionGroups.ts`

**Interfaces:**
- Consumes: `SessionInfo.created_at: string` and optional `SessionInfo.lastActivityAt?: string`.
- Produces: `groupForSession(session, now): GroupKey`, using activity time when valid.

- [ ] **Step 1: Write the failing regression test**

Add this test after the existing same-day test in `apps/desktop/tests/sessionGroups.test.ts`:

```ts
test("groupForSession uses today's last activity for a session created yesterday", () => {
  const now = new Date(2026, 5, 28, 18, 0);
  const resumed = {
    ...session("a", new Date(2026, 5, 27, 20, 0).toISOString()),
    lastActivityAt: new Date(2026, 5, 28, 9, 0).toISOString(),
  };

  assert.equal(groupForSession(resumed, now), "today");
});
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/sessionGroups.test.ts`

Expected: FAIL because `groupForSession` still evaluates `created_at` and returns `week`.

- [ ] **Step 3: Implement the minimal timestamp selection**

Replace the first two lines of `groupForSession` in `apps/desktop/src/sessionGroups.ts` with:

```ts
const lastActivity = s.lastActivityAt ? new Date(s.lastActivityAt) : undefined;
const created = lastActivity && !Number.isNaN(lastActivity.getTime())
  ? lastActivity
  : new Date(s.created_at);
```

Leave the existing invalid-date guard and grouping calculations unchanged.

- [ ] **Step 4: Run the focused test to verify it passes**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/sessionGroups.test.ts`

Expected: PASS; all session-group tests pass.

- [ ] **Step 5: Run the desktop validation relevant to the change**

Run: `cd apps/desktop && npx tsc --noEmit && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`

Expected: type-check exits 0 and all frontend tests pass.

- [ ] **Step 6: Run documentation currency check and commit**

Run: `bash .claude/hooks/doc-check.sh`

Expected: no newly required documentation changes beyond the committed design and this plan.

Then commit the implementation and test:

```bash
git add apps/desktop/src/sessionGroups.ts apps/desktop/tests/sessionGroups.test.ts
git commit -m "fix(desktop): group resumed sessions by activity"
```
