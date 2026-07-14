# Session Activity and Unread Result Redesign Implementation Plan

> Execute with the repository's `executing-plans`, `test-driven-development`,
> `verification-before-completion`, and code-review workflows.

**Goal:** Revise issue #62 so an inactive session becomes unread only when it
transitions from `working` to a live non-working result, and render that unread
result in the existing status slot.

**Architecture:** Keep unread as renderer-local state in `sessionUnread.ts`.
Derive transitions from normalized attention and render the latch through an
internal `StatusIndicator` variant. No backend or cross-process contracts
change.

---

### Task 1: Synchronize issue scope and design artifacts

**Files:**

- Create: `docs/superpowers/specs/2026-07-14-session-activity-unread-redesign.md`
- Create: `docs/superpowers/plans/2026-07-14-session-activity-unread-redesign.md`
- Update: GitHub issue #62

Update the issue acceptance criteria to cover all five canonical working
results, the single status slot, accessible result-colored dots, selection
clearing, and the absence of backend changes. Remove the obsolete non-Codex
execution restriction and link these artifacts.

### Task 2: Implement result-transition unread semantics with TDD

**Files:**

- Modify: `apps/desktop/tests/sessionUnread.test.ts`
- Modify: `apps/desktop/src/sessionUnread.ts`

Write failing tests for `working -> idle|needs_you|blocked|failed|capped` and
for active-session suppression, first/new/dead sessions, non-working changes,
persistence, clearing, raw activity, unexpected return to working, and
unchanged-poll identity. Run the focused test to verify RED. Change
`trackUnread()` minimally so only an inactive live working-result transition
creates the latch, then rerun the focused test to verify GREEN.

### Task 3: Collapse unread into the status slot with TDD

**Files:**

- Modify: `apps/desktop/tests/dockview.test.ts`
- Modify: `apps/desktop/src/components/StatusIndicator.tsx`
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Modify: `apps/desktop/src/App.css`
- Modify: `apps/desktop/src/styles/tokens.css` if the old unread token becomes
  unused

First update source-contract tests to require one signal slot, the internal
`variant` interface, working-spinner precedence, a 7px accessible unread dot,
tone-driven color, unread tint without unread label bolding, and the unchanged
detail-panel call. Run the focused test to verify RED. Implement the minimal
component, markup, and CSS changes, then rerun the focused tests to verify
GREEN.

### Task 4: Track the observer-only resumption invariant

Create a separate stabilization issue requiring observer-only updates never to
move a finished/non-working session back to `working` without intervening user
input. Preserve genuine user-driven session resumption and include scoped
checkbox acceptance criteria.

### Task 5: Verify and review

From `apps/desktop`, run:

```bash
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
pnpm exec tsc --noEmit
pnpm build
```

Then run `bash .claude/hooks/doc-check.sh`, address every flag, and perform the
required lightweight code review. Re-run affected verification after any
review fix before handoff.
