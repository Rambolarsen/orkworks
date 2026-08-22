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

The post-commit chained verification rerun was intentionally interrupted by
the user before it produced output. Therefore there is no fresh post-commit
verification result for that final chain; the preceding standalone `npx tsc
--noEmit` and required focused test run both passed before the report-only
commit.

## Commit

- `b66462a` — `refactor: deepen workspace session controller`

## Concerns

- The three legacy `dockview.test.ts` source-shape assertions should be
  updated in a follow-up to inspect the controller seam instead of `App.tsx`.
- Final post-commit verification was interrupted and could not be confirmed
  complete; `ps` inspection was also denied by the sandbox, so no claim is
  made about leftover process state.

## Review fix

### Files

- `apps/desktop/src/workspaceSessionController.ts`
  - Split foreground-operation invalidation from refresh/poll work.
  - Added `setPollingEnabled()` so construction is timer-free and App owns
    the connected-workspace lifecycle seam.
  - Preserved superseded foreground and disposed-operation rejection.
- `apps/desktop/src/App.tsx`
  - Enables polling only after backend connection and workspace publication;
    disables it during lifecycle cleanup.
- `apps/desktop/tests/workspaceSessionController.test.ts`
  - Added poll/foreground race, explicit polling ownership, live/dead
    restoration, and post-disposal coverage.
- `apps/desktop/tests/dockview.test.ts`
  - Completed the three extraction-aware source-shape assertions while
    preserving the existing uncommitted changes.
- `apps/desktop/tests/sessionSort.test.ts`
  - Updated one remaining extraction-era source-shape assertion to inspect
    the controller.

### Tests and commands

- TDD red: focused controller tests failed because `setPollingEnabled()` was
  absent.
- TDD green: `node --experimental-strip-types --test tests/workspaceSessionController.test.ts tests/dockview.test.ts` — 73/73 passed.
- `./node_modules/.bin/tsc --noEmit` — passed, exit 0.
- `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` — 362/362 passed.
- `git diff --check` — passed.
- `pnpm exec tsc --noEmit` — sandbox EPERM while opening pnpm's temporary
  wrapper; direct local `tsc` produced the same successful type-check.

### Concerns

- Node test runs continue to emit existing module-type warnings; no test
  failures or new runtime warnings were introduced.

## Re-review fix

The poll/foreground regression test now keeps the create pending before
polling begins, resolves the poll first, and asserts active-session
publication. This fails with the former shared-generation implementation and
passes with the separated foreground generation.

- `node --experimental-strip-types --test tests/workspaceSessionController.test.ts` — 10/10 passed.

## Poll lifecycle fix

Polling now uses an epoch invalidated on disable, re-enable, and dispose, so
an old in-flight response cannot publish beside a newly enabled loop. Added a
regression test for disable → in-flight response → re-enable ordering.

- `node --experimental-strip-types --test tests/workspaceSessionController.test.ts` — 11/11 passed.
