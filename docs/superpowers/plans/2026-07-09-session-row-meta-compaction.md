# Session Row Meta Compaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the session list row's right-side metadata/actions area smaller and move unread state to a fixed left-side slot so rows do not shift when unread appears.

**Architecture:** Add a left-side `session-row-leading` cluster in `SessionListPanel`, move the unread dot into a fixed `6px` slot before `session-row-primary`, and adjust `App.css` so the slot uses a `6px` internal gap and the row's left padding drops to `var(--space-2)`. Guard the change with source-based regression tests in `dockview.test.ts`.

**Tech Stack:** React, TypeScript, CSS, Node test runner

## Global Constraints

Keep the session row single-line and behaviorally unchanged.
Limit the change to `apps/desktop/src/components/SessionListPanel.tsx`, `apps/desktop/src/App.css`, and `apps/desktop/tests/dockview.test.ts`.
Do not change token definitions in `apps/desktop/src/styles/tokens.css`.

---

### Task 1: Add regression coverage for the fixed unread slot and compact spacing

**Files:**
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Modify: `apps/desktop/tests/dockview.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: Markup around `.session-row-leading`, `.session-row-primary`, `.session-row-actions`, `.session-row-unread-dot`
- Consumes: CSS selectors `.session-row`, `.session-row-secondary`, `.session-row-meta`, `.session-row-actions`
- Produces: Assertions that lock the unread slot placement and compact spacing values

- [ ] **Step 1: Write the failing test**

```ts
test("session list reserves a fixed slot for unread state before the primary content", () => {
  const panel = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );
  assert.match(panel, /session-row-leading[\s\S]*session-row-unread-slot[\s\S]*session-row-primary/);
  assert.match(panel, /session-row-unread-slot[\s\S]*session-row-unread-dot/);
  assert.doesNotMatch(panel, /className="session-row-actions"[\s\S]*session-row-unread-dot/);
  assert.match(panel, /session-row-kill[\s\S]*e\.stopPropagation\(\)/);
  assert.match(panel, /session-row-forget[\s\S]*e\.stopPropagation\(\)/);
});

test("session list keeps the row footprint compact", () => {
  const css = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");

  assert.match(css, /\.session-row\s*\{[\s\S]*padding:\s*var\(--space-3\) var\(--space-5\) var\(--space-3\) var\(--space-2\);/);
  assert.match(css, /\.session-row-leading\s*\{[\s\S]*gap:\s*6px;/);
  assert.match(css, /\.session-row-unread-slot\s*\{[\s\S]*width:\s*6px;/);
  assert.match(css, /\.session-row-secondary\s*\{[\s\S]*gap:\s*var\(--space-1\);/);
  assert.match(css, /\.session-row-meta\s*\{[\s\S]*grid-template-columns:\s*12px 6ch;/);
  assert.match(css, /\.session-row-meta\s*\{[\s\S]*column-gap:\s*var\(--space-2\);/);
  assert.match(css, /\.session-row-actions\s*\{[\s\S]*gap:\s*0;/);
  assert.match(css, /\.session-row-kill\s*\{[\s\S]*padding:\s*0 2px;/);
  assert.match(css, /\.session-row-forget\s*\{[\s\S]*padding:\s*0 2px;/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts`
Expected: FAIL because the current markup still renders the unread dot on the right and does not define the new left-side slot classes.

### Task 2: Move unread state into a fixed left slot and preserve spacing

**Files:**
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Modify: `apps/desktop/src/App.css`
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: Existing session row class names
- Produces: A `session-row-leading` cluster, a fixed unread slot, and stable left-edge alignment

- [ ] **Step 1: Write minimal implementation**

```tsx
<div className="session-row-leading" aria-hidden="true">
  <span className="session-row-unread-slot">
    {unread && <span className="session-row-unread-dot" />}
  </span>
  <div className="session-row-primary">
```

```css
.session-row {
  padding: var(--space-3) var(--space-5) var(--space-3) var(--space-2);
}

.session-row-leading {
  display: flex;
  align-items: center;
  gap: 6px;
}

.session-row-unread-slot {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 6px;
  flex-shrink: 0;
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts`
Expected: PASS

- [ ] **Step 3: Run the doc currency check**

Run: `bash .claude/hooks/doc-check.sh`
Expected: exit 0, or a list of files to review and either update or explicitly confirm as unchanged for this CSS-only tweak.
