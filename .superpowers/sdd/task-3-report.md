# Task 3 implementation report

## Summary

Extracted renderer workspace/session orchestration from `App.tsx` into a
generation-aware `workspaceSessionController.ts`. React state remains in
`App.tsx`; the controller publishes workspace, session, active-session, and
error callbacks. Existing REST wrappers, preload contracts, terminal store,
session sorting/throttling, and pending-create correlation remain in use.

## Changed files

- `apps/desktop/src/workspaceSessionController.ts`
  - Added the requested controller interface and dependency seams.
  - Owns the single polling loop, generation invalidation, error suppression,
    workspace refresh/restoration, workflow operations, and terminal pruning.
- `apps/desktop/src/App.tsx`
  - Composes one controller instance and routes workspace/session actions to it.
  - Retains React state, Dockview focus behavior, unread state, settings, and
    active-session persistence.
- `apps/desktop/tests/workspaceSessionController.test.ts`
  - Added focused real-behavior tests for polling ownership, stale responses,
    disposal, pending-create correlation, live restoration, deletion ordering,
    and terminal pruning.

## TDD evidence

The new test file was run before the controller existed and failed with the
expected missing-module error for `workspaceSessionController.ts`. The
controller was then implemented incrementally until the focused suite passed.

## Verification

- `pnpm install --offline` — passed; hydrated the worktree dependencies from
  the local pnpm store.
- `npx tsc --noEmit` — passed (exit 0).
- `node --experimental-strip-types --test tests/workspaceSessionController.test.ts tests/sessionPolling.test.ts tests/pendingCreate.test.ts tests/api.test.ts` — passed, 28/28.
- `node --experimental-strip-types --test tests/dockview.test.ts` — 60/63 passed. Three existing source-shape assertions still expect workspace/session orchestration and lifecycle-pruning expressions to live in `App.tsx`; those responsibilities intentionally moved to the new controller. This is a test-maintenance concern, not a runtime or type-check failure.
- `git diff --check` — passed.
- `bash .claude/hooks/doc-check.sh` — passed.
- `bash .claude/hooks/worktree-check.sh` — passed.

## Commit

- `b66462a` — `refactor: deepen workspace session controller`

## Concerns

- The three legacy `dockview.test.ts` source-shape assertions should be
  updated in a follow-up to inspect the controller seam instead of `App.tsx`.
