# Task 2 — Sidecar lifecycle controller report

## Scope delivered

Implemented only the standalone, testable Electron-main sidecar lifecycle
controller and its focused tests. `main.ts` was not changed.

- Added `apps/desktop/electron/sidecarLifecycle.ts`.
- Added `apps/desktop/tests/sidecarLifecycle.test.ts`.
- Committed the two Task 2 files as `6f7b809 feat: add generation-safe sidecar lifecycle`.

The controller provides `start`, `stop`, `retry`, `getPort`, and `dispose`.
It accepts injected process spawning, fetch, clocks/timers, and state/readiness
callbacks. It uses a monotonically increasing generation to ignore stale
stdout, exit, and error callbacks; it rejects generation-specific readiness on
pre-ready exit, error, and timeout; clears the port on a current failure; and
performs a bounded three-attempt automatic recovery sequence before explicit
retry is required. The attempt counter resets only after the configured ready
stability timer, not when a port is first published.

## TDD record and commands

1. Wrote the lifecycle tests before creating the production module.

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/sidecarLifecycle.test.ts
   ```

   Result: failed as expected with `ERR_MODULE_NOT_FOUND` for
   `electron/sidecarLifecycle.ts`.

2. Implemented the smallest injected state machine to satisfy the tests.
   The first green run revealed unhandled rejected promises from automatic
   retries, because those retries have no external promise owner. The
   controller now consumes those internal rejections after failure callbacks
   schedule the next bounded retry.

3. Final focused verification:

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/sidecarLifecycle.test.ts
   npx tsc --noEmit
   git diff --check
   ```

   Result: lifecycle suite passed 7/7; TypeScript exited 0; diff check exited
   0. Node emitted the existing package-type warning for TypeScript ESM tests.

4. Self-review checked the committed controller and tests for generation
   guards, readiness settlement, retry bounds, port clearing, stale callback
   handling, and accidental Electron/main integration.

5. Repository closeout checks:

   ```bash
   bash .claude/hooks/doc-check.sh
   bash .claude/hooks/worktree-check.sh
   ```

   Result: both exited 0 with no findings.

## Test coverage

- pre-ready process exit rejects readiness and clears the port
- obsolete-generation exit cannot invalidate the current ready process
- three automatic attempts exhaust recovery; explicit retry can recover
- ready-process exit sends unavailable notification and invalidates the port
- process error rejects readiness
- readiness timeout rejects readiness
- repeated error/exit callbacks schedule only one automatic recovery sequence

## Concerns and handoff

- Task 3 must integrate this controller into `main.ts`, including concrete
  binary/env spawning, workspace restoration, settings replay, and renderer
  notification wiring. Those changes are intentionally out of scope here.
- `.superpowers/sdd/task-1-report.md` was already modified before this task and
  remains untouched/uncommitted by Task 2.
- This report is deliberately not part of the Task 2 code commit; the exact
  task brief specifies committing only the lifecycle module and its tests.

## Review-finding fix — 2026-08-26

### Scope

- `fail()` now terminates its own failed process before scheduling recovery.
  `exited` prevents a second kill after an exit event, and `killRequested`
  prevents duplicate error/timeout callbacks from killing the same process.
  Termination receives the captured generation, so it cannot kill a replacement.
- The injected clock now records each generation's ready time and explicitly
  gates retry-counter reset until the complete stability period has elapsed.
  The unused stored lifecycle state was removed; state remains observable via
  the existing callback contract.
- The deterministic fake timers now model due times and clock advancement.
  New coverage proves retries are retained after immediate post-ready failure,
  reset after `readyStabilityMs`, and that timeout/error failures kill their
  original process.

### Commands and results

1. Baseline:

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/sidecarLifecycle.test.ts
   ```

   Result: passed 7/7 before the review-fix tests were added.

2. TDD red run after adding the failure-termination assertion:

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/sidecarLifecycle.test.ts
   ```

   Result: failed as expected: readiness timeout left `processes[0].killed`
   false (8 passed, 1 failed).

3. Covering verification after the lifecycle fix:

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/sidecarLifecycle.test.ts
   npx tsc --noEmit
   ```

   Result: lifecycle suite passed 9/9; TypeScript completed with no errors.

4. Repository closeout:

   ```bash
   git diff --check
   bash .claude/hooks/doc-check.sh
   bash .claude/hooks/worktree-check.sh
   ```

   Result: all exited 0 with no findings.

### Concerns

- The focused Node test continues to emit the existing package-type warning for
  TypeScript ESM files. No package metadata was changed because that is outside
  this review-fix scope.
- Pre-existing edits to `.superpowers/sdd/task-1-report.md` were preserved and
  are not included in this fix commit.
