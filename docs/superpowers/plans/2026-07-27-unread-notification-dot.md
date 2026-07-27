# Unread Notification Dot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give every unread completed live-session result a visible dot whose color communicates the latest normalized result tone without changing unread-state behavior.

**Architecture:** Keep `sessionUnread.ts` as the sole derivation and latch for unread IDs. Change only `StatusIndicator`'s unread rendering and its CSS: completed unread results render as dots, while a new working turn shows the normal spinner; idle unread dots receive a blue override while existing needs-you, blocked, and failed colors continue to express their normalized tones.

**Tech Stack:** React, TypeScript, CSS custom properties, Node built-in test runner.

## Global Constraints

- Preserve unread derivation, selection clearing, in-memory lifetime, and live-session-only behavior in `apps/desktop/src/sessionUnread.ts`.
- Do not add a `done` attention treatment; completion is lifecycle state.
- Use blue for unread `idle`, `stale`, `needs_you`, and `waiting_for_input` tones; amber for `blocked`, `checking_capacity`, and `capped`; red for `failed`.
- Keep the Electron main-process and renderer import boundary intact.
- Do not add a dependency.

---

### Task 1: Render completed unread results as dots and apply the unread palette

**Files:**
- Modify: `apps/desktop/src/components/StatusIndicator.tsx:26-37`
- Modify: `apps/desktop/src/App.css:730-743`
- Modify: `apps/desktop/tests/dockview.test.ts:267-288`
- Modify: `apps/desktop/tests/statusIndicator.test.ts:1-25`

**Interfaces:**
- Consumes: `StatusIndicatorProps` (`tone`, `label`, optional `variant`) and the existing `AttentionTone` values from `apps/desktop/src/labels.ts`.
- Produces: For every non-working `variant="unread"`, a `status-indicator-unread` dot with `aria-label={\`Unread: ${label}\`}`. Its `data-attention` continues to carry the current normalized tone for CSS. A working tone keeps its normal spinner.
- Leaves unchanged: `trackUnread()` and `clearUnread()` in `apps/desktop/src/sessionUnread.ts`.

- [ ] **Step 1: Write the failing source-level regression assertions**

  In `apps/desktop/tests/dockview.test.ts`, replace the assertion that requires
  working precedence with assertions that require only completed unread
  variants to use the dot branch:

  ```ts
  assert.match(source, /variant\s*===\s*"unread"\s*&&\s*tone\s*!==\s*"working"/);
  assert.match(source, /className="status-indicator status-indicator-unread"/);
  assert.match(source, /aria-label=\{`Unread:\s*\$\{label\}`\}/);
  ```

  Add a CSS assertion that requires a blue idle unread override without
  weakening the existing tone-selector coverage:

  ```ts
  assert.match(
    css,
    /\.status-indicator-unread\[data-attention="idle"\][\s\S]*color:\s*var\(--attention-needs-you\)/,
  );
  ```

  In `apps/desktop/tests/statusIndicator.test.ts`, retain the existing
  read-state icon test and retain the assertion that unread rendering is gated
  on `working`.

- [ ] **Step 2: Run the focused tests to verify the regression assertions fail**

  Run:

  ```bash
  cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts tests/statusIndicator.test.ts
  ```

  Expected: failure because `StatusIndicator` still tests
  `tone !== "working"` and the idle unread CSS override does not exist.

- [ ] **Step 3: Make the minimal rendering and CSS change**

  In `apps/desktop/src/components/StatusIndicator.tsx`, make the unread branch
  gated on non-working tones so an unread session that starts a new working
  turn returns to the normal spinner:

  ```tsx
  if (variant === "unread" && tone !== "working") {
    return (
      <span
        className="status-indicator status-indicator-unread"
        data-attention={tone}
        role="img"
        aria-label={`Unread: ${label}`}
      />
    );
  }
  ```

  In `apps/desktop/src/App.css`, add narrowly scoped unread overrides before
  the general `data-attention` selectors (or later with equal-or-greater
  specificity):

  ```css
  .status-indicator-unread[data-attention="idle"] {
    color: var(--attention-needs-you);
  }
  ```

  Do not change the general idle selector; normal read idle status must remain
  gray. The existing needs-you, blocked, and failed selectors supply blue,
  amber, and red for their unread dots. `capped` and its aliases already
  normalize to the blocked tone in `attentionTone()`.

- [ ] **Step 4: Run focused tests to verify the change**

  Run:

  ```bash
  cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts tests/statusIndicator.test.ts tests/sessionUnread.test.ts
  ```

  Expected: all tests pass, including existing unread latch and ended-session
  coverage in `sessionUnread.test.ts`.

- [ ] **Step 5: Run type-check and the full frontend test suite**

  Run:

  ```bash
  cd apps/desktop && npx tsc --noEmit
  cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
  ```

  Expected: both commands exit 0.

- [ ] **Step 6: Commit the implementation**

  ```bash
  git add apps/desktop/src/components/StatusIndicator.tsx apps/desktop/src/App.css apps/desktop/tests/dockview.test.ts apps/desktop/tests/statusIndicator.test.ts
  git commit -m "fix: color unread session results by urgency"
  ```

## Final verification

- [ ] Run `git diff --check`.
- [ ] Run `bash .claude/hooks/doc-check.sh` and address every flagged file.
- [ ] Run `bash .claude/hooks/worktree-check.sh`; report, but do not alter, worktrees not owned by this task.
- [ ] Request the required lightweight code review before opening a PR, because this plan changes `apps/desktop/` code.
