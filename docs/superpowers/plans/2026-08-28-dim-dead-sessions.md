# Dim Dead Sessions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make terminally dead sessions visibly dimmer in the Sessions list while preserving existing interaction behavior.

**Architecture:** Reuse the existing `s.lifecycle === "dead"` → `session-row--remembered` semantic and change only that class's opacity in the renderer stylesheet. Add a source-level regression test alongside the existing Dockview/session-list source contract tests.

**Tech Stack:** React, TypeScript, CSS, Node's built-in test runner.

## Global Constraints

- Only sessions with `lifecycle === "dead"` are dimmed.
- Creating, alive, and stopping sessions keep their current emphasis.
- No session data, sorting, labels, status logic, or terminal behavior changes.
- Use pnpm for Node.js package-management tasks.

---

### Task 1: Add the dead-row styling regression test

**Files:**
- Modify: `apps/desktop/tests/dockview.test.ts` near the existing session-list source assertions
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: the source text of `apps/desktop/src/components/SessionListPanel.tsx` and `apps/desktop/src/App.css` using the test file's existing `readFileSync` pattern.
- Produces: a regression test that fails until the existing remembered-row opacity is strengthened.

- [ ] **Step 1: Write the failing test**

Add a test asserting both the existing dead-session class condition and the exact CSS value:

```ts
test("dead session rows use the dimmed remembered treatment", () => {
  const panel = readFileSync(new URL("../src/components/SessionListPanel.tsx", import.meta.url), "utf8");
  const styles = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");

  assert.match(panel, /remembered \? "session-row--remembered"/);
  assert.match(styles, /\.session-row--remembered \{ opacity: 0\.62; \}/);
});
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/dockview.test.ts
```

Expected: the test fails because `App.css` currently defines `opacity: 0.78`.

- [ ] **Step 3: Commit the failing test**

```bash
git add apps/desktop/tests/dockview.test.ts
git commit -m "test: pin dimmed dead session rows"
```

### Task 2: Strengthen the existing dead-row dimming

**Files:**
- Modify: `apps/desktop/src/App.css:1040`

**Interfaces:**
- Consumes: the existing `session-row--remembered` class emitted by `SessionListPanel` for dead sessions.
- Produces: a dead-session row with `opacity: 0.62`; no changes to row hit targets, hover selectors, or focus behavior.

- [ ] **Step 1: Change the exact opacity value**

Change:

```css
.session-row--remembered { opacity: 0.78; }
```

to:

```css
.session-row--remembered { opacity: 0.62; }
```

- [ ] **Step 2: Run the focused test to verify it passes**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/dockview.test.ts
```

Expected: the focused test passes with zero failures.

- [ ] **Step 3: Commit the implementation**

```bash
git add apps/desktop/src/App.css
git commit -m "style: dim dead session rows"
```

### Task 3: Run complete verification

**Files:**
- Verify: `apps/desktop/`
- Verify: repository documentation and worktree state

- [ ] **Step 1: Run desktop type-check**

```bash
npx tsc --noEmit
```

Expected: exit code 0 with no TypeScript errors.

- [ ] **Step 2: Run the desktop test suite**

```bash
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: exit code 0 with zero failed tests.

- [ ] **Step 3: Run repository checks**

From the repository root:

```bash
bash scripts/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: both checks complete; address any doc-drift flags before handoff and
report any worktree flags that belong to another owner without modifying them.
