# Task 3 Report: Integrate sidecar recovery into Electron main

## Status

Implemented and committed in `9f52489` (`feat: recover from sidecar lifecycle failures`).

## Changes

- `apps/desktop/electron/main.ts`
  - Replaced the two direct sidecar spawn/readiness implementations with one
    `SidecarLifecycle` controller and one environment/token construction path.
  - Routed initial startup, workspace switching, automatic recovery, explicit
    retry, and shutdown through the controller.
  - Added a main-process generation guard around asynchronous workspace and
    settings restoration so stale completions cannot publish readiness for a
    replacement sidecar.
  - Restores the remembered/current workspace, retention settings, and provider
    settings before resolving renderer-facing readiness or publishing `ready`.
  - Replaced the never-rejecting port promise with a current-generation
    readiness promise. Spawn errors, pre-ready exits, timeouts, post-ready
    failures, replacement, and restoration failures now reject or invalidate
    readiness normally.
  - Preserved the former `get-initial-workspace` fallback of returning `null`
    when initial restoration fails, avoiding an unhandled renderer rejection.
  - Added lifecycle IPC events and the explicit `retry-backend` handler without
    exposing process handles, paths, or the open-plan token.
- `apps/desktop/electron/preload.ts`
  - Added the duplicated Electron-boundary `BackendLifecycleEvent` union.
  - Added runtime payload validation before invoking renderer callbacks.
  - Exposed `onBackendLifecycle(callback)` with unsubscribe and
    `retryBackend()` as a one-shot IPC request.
- `apps/desktop/src/orkworksWindow.d.ts`
  - Added the renderer-owned copy of `BackendLifecycleEvent` and declarations
    for both new bridge methods.
- `apps/desktop/tests/electronSidecarWiring.test.ts`
  - Added source-level assertions for centralized lifecycle construction and
    startup, restoration ordering/generation guards, rejecting readiness,
    explicit retry, preload validation, and matching renderer declarations.

No Task 1 or Task 2 production/test files were changed by this task. The
concurrently modified `.superpowers/sdd/task-1-report.md` was left untouched
and excluded from the commit.

## TDD evidence

- Initial RED: `node --experimental-strip-types --test tests/electronSidecarWiring.test.ts`
  failed 5/5 because lifecycle construction, centralized startup, restoration,
  retry IPC, preload validation, and renderer declarations were absent.
- Initial GREEN: the same command passed 5/5 after the first implementation.
- Review-fix RED: the new initial-workspace rejection assertion failed 1/6
  against the uncaught handler.
- Review-fix GREEN: the same command passed 6/6 after restoring the `null`
  fallback.

## Review

`rtk codex review --uncommitted` performed a fresh-context review. It found:

- Relevant P2: initial workspace restoration could reject into an uncaught
  renderer call. Fixed with a test-first `null` fallback in
  `get-initial-workspace`.
- Out-of-scope P2: the concurrently modified Task 1 report did not describe
  this Electron diff. No action taken because it belongs to Task 1 and the
  task owner explicitly required preserving Task 1/2 work.

Self-review confirmed the commit contains only the four assigned Task 3 files,
one sidecar spawn call/token injection remains, old-generation restoration is
guarded by both generation and current port, and no privileged process/path/
token data crosses preload.

## Commands and results

- `node --experimental-strip-types --test tests/sidecarLifecycle.test.ts tests/electronSidecarWiring.test.ts`
  — PASS, 15/15 tests.
- `npx tsc --noEmit` — PASS, exit 0.
- Fresh-context reviewer also ran the complete desktop TypeScript/test command;
  the desktop suite passed. Its output included the repository's existing Node
  module-type warnings.
- `git diff --check` — PASS, exit 0.
- `git diff --cached --check` — PASS, exit 0.
- `bash .claude/hooks/doc-check.sh` — exit 0 with an advisory to update
  `docs/agents/architecture.md`.
- `bash .claude/hooks/worktree-check.sh` — PASS, exit 0 with no output.
- `git commit -m "feat: recover from sidecar lifecycle failures"` — PASS;
  commit `9f52489`.

## Concerns

- `docs/agents/architecture.md` still needs the complete runtime-recovery
  lifecycle documentation. This is explicitly assigned to Task 5 of the
  approved plan, so Task 3 did not edit or commit that file.
- Node test runs emit the existing `MODULE_TYPELESS_PACKAGE_JSON` warnings;
  no test failed.
- Tests exercise the lifecycle controller behavior and source-level Electron
  wiring, but do not launch a real Electron window or packaged sidecar.
- No blocker remains for Task 3.

## Interruption / process audit

The final handoff turn was interrupted after the implementation commit and
after this report had been written. No test, type-check, or reviewer process
was still running, so there was no process to terminate and no command stall.

Process audit command: `ps aux`, filtered for `codex review`,
`node --experimental-strip-types --test`, and `tsc --noEmit`.

Exact output:

```text
NO_MATCHING_TASK3_PROCESSES
```

Current implementation commit remains `9f52489`; completed code was not
discarded. The only uncommitted files are this requested Task 3 report and the
concurrently owned Task 1 report. Blocker status: none.

## Review-finding remediation interruption — 2026-08-26

### Status

`NEEDS_CONTEXT` — implementation is preserved but the remediation is not
complete, fully verified, reviewed, reported, or committed.

Current `HEAD` is still `9f524895adc60319c5b4174db6942e56dd231060`.
No remediation commit exists yet.

### Completed code

- Added the Electron-free `backendRestoration.ts` coordinator. It owns a
  readiness deferred, timeout, and `AbortController` per backend generation;
  aborts and rejects on replacement, explicit failure, and shutdown; ignores
  stale deferred completion; and publishes restoration failures through an
  injected callback.
- Added `switchWorkspaceBackend`, which persists the new workspace before it
  invokes replacement startup. If persistence throws, the current backend is
  not stopped or replaced.
- Added the pure `backendLifecycleEvent.ts` canonicalizer. It requires exact
  lifecycle payload keys, creates a new trusted object, and accepts ready ports
  only when they are integers in `1..65535`.
- Integrated the coordinator into `main.ts`. Workspace POST, retention POST,
  and provider synchronization receive the generation-owned abort signal.
  Restoration readiness now comes from the coordinator.
- Updated `open-workspace` to use persist-before-start ordering and to report a
  synchronous replacement-start failure coherently.
- Updated preload to call the pure canonicalizer and never forward the original
  IPC object.
- Added latest-lifecycle replay through a narrow renderer subscription
  handshake so a late subscription receives the current lifecycle state.
- Added behavioral tests for stale completion, timeout/abort, explicit
  failure/shutdown abort, persistence failure, canonical object creation,
  malicious extra fields, and invalid ports. Updated source-level tests to keep
  only Electron wiring/boundary assertions.

The concurrently modified `.superpowers/sdd/task-1-report.md` remains
untouched by this remediation and must not be staged.

### TDD and verification state

1. Baseline focused command:

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/sidecarLifecycle.test.ts tests/electronSidecarWiring.test.ts
   ```

   Result before remediation: passed 15/15.

2. Pure-seam RED command:

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/backendRestoration.test.ts tests/backendLifecycleEvent.test.ts
   ```

   Result: failed as expected with `ERR_MODULE_NOT_FOUND` for both new
   production modules.

3. Pure-seam GREEN command after the minimal implementations:

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/backendRestoration.test.ts tests/backendLifecycleEvent.test.ts
   ```

   Result: passed 6/6.

4. Electron wiring RED command before integration:

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/electronSidecarWiring.test.ts
   ```

   Result: failed 6/8 as expected for missing coordinator integration, safe
   workspace switching, lifecycle canonicalization, and late-state replay.

5. Post-integration focused command:

   ```bash
   cd apps/desktop
   node --experimental-strip-types --test tests/backendRestoration.test.ts tests/backendLifecycleEvent.test.ts tests/sidecarLifecycle.test.ts tests/electronSidecarWiring.test.ts
   ```

   Result: 18/23 passed. The five failures were stale source-test assumptions
   about local variable names/formatting after the behavioral code was already
   green. Those assertions were narrowed afterward, but the command was not
   rerun because the user interrupted and requested an immediate stop.

6. TypeScript command run after production integration:

   ```bash
   cd apps/desktop
   npx tsc --noEmit
   ```

   Result: passed with no errors.

The full desktop test command was not started. No long-running test command is
active.

### Process stop audit

The process table was checked for:

```text
node --experimental-strip-types --test
tsc --noEmit
codex review
```

The escalated filtered audit returned no matches, so there was no Task 3
process to terminate. The initial sandboxed process query was denied by the OS;
the approved escalated query completed successfully.

### Exact blocker and remaining work

There is no technical blocker. The user interrupted the turn and explicitly
requested that all work stop. A fresh continuation context is required to:

1. inspect the preserved diff and coordinator edge cases;
2. rerun the adjusted focused tests;
3. run the complete desktop TypeScript/test checks;
4. run the required fresh-context code review and address findings;
5. run `git diff --check`, doc currency, and worktree currency checks;
6. append final command results and concerns here; and
7. stage Task 3 files only and create the requested commit.

One implementation concern remains for review: the late-subscription replay
handshake sends the cached event to the renderer channel, so registering a
second callback in the same renderer could also replay to an existing callback.
The current app appears to have one lifecycle subscriber, but this should be
confirmed or deduplicated before completion.

## Review-finding remediation completion — 2026-08-26

### Status

Complete. The interrupted remediation was preserved, inspected, corrected,
fully verified, independently reviewed, and prepared as one Task 3 review-wave
commit. The unrelated uncommitted `.superpowers/sdd/task-1-report.md` remains
untouched and excluded from staging.

### Final changes

- Kept restoration work generation-owned through one deferred readiness,
  timeout, and `AbortController`. Replacement, sidecar failure, timeout, and
  shutdown abort the owned work; late completion after timeout/replacement
  cannot publish readiness or restored workspace state.
- Kept workspace persistence ahead of replacement startup. A persistence
  exception prevents `SidecarLifecycle.start`, so the current backend is not
  stopped by a failed memory write.
- Hardened lifecycle payload canonicalization to require exact keys, bounded
  integer ports, a fresh trusted output object, and one read per untrusted
  field. A stateful getter can no longer pass validation with one value and
  forward a different value.
- Replaced renderer-wide replay broadcasting with a per-subscriber IPC
  snapshot. The preload subscribes to live events first, validates both live
  and snapshot payloads through the same canonicalizer, suppresses a stale
  snapshot when newer live state arrives, and unregisters cleanly.
- Added behavior-level regression coverage for timeout followed by ignored-abort
  completion, explicit persistence ordering, single-read canonicalization, and
  late-subscriber snapshot/live ordering. Electron source tests retain only the
  wiring assertions that require the Electron boundary.

### TDD evidence

The adjusted focused baseline passed 23/23 before the final inspection. New
regressions were then run before their production changes:

- single-read canonicalization failed because the original failure payload
  validator read `message` twice and forwarded `42` after validating
  `"offline"`;
- the pure late-subscription helper assertion failed because the helper did not
  exist; and
- the Electron wiring assertion failed because main still broadcast replay on
  the shared lifecycle channel.

After the minimal canonicalization and per-subscriber snapshot changes, the
adjusted focused suite passed 27/27.

### Commands and results

- `cd apps/desktop && node --experimental-strip-types --test tests/backendRestoration.test.ts tests/backendLifecycleEvent.test.ts tests/electronSidecarWiring.test.ts tests/sidecarLifecycle.test.ts`
  — initial preserved state PASS, 23/23; final implementation PASS, 27/27.
- `cd apps/desktop && npx tsc --noEmit` — PASS, no TypeScript errors.
- `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
  — PASS, 422/422.
- `rtk codex review --uncommitted` — PASS. The fresh-context reviewer reran
  TypeScript, the 27 focused tests, and the 422-test desktop suite and reported
  no actionable correctness or regression findings.
- `git diff --check` — PASS.
- `bash .claude/hooks/doc-check.sh` — exit 0 with the existing advisory to
  update `docs/agents/architecture.md`; complete runtime-recovery documentation
  remains assigned to Task 5 of the approved plan.
- `bash .claude/hooks/worktree-check.sh` — PASS, exit 0 with no output.

### Concerns

- Node test runs still emit the repository's existing
  `MODULE_TYPELESS_PACKAGE_JSON` warnings; no test fails.
- The tests cover the lifecycle/restoration seams and Electron source wiring,
  but do not launch a packaged Electron app with a real sidecar process.
- `docs/agents/architecture.md` still needs Task 5's complete recovery-flow
  documentation; this review wave does not preempt that assigned task.
- No Task 3 blocker remains.

## Review-finding remediation — 2026-08-26

### Status

Complete in the pending changeset. The unrelated uncommitted
`.superpowers/sdd/task-1-report.md` remains untouched and will not be staged.

### Findings addressed

- Added `backendLifecycleFailure.ts` as the main-process failure-message
  boundary. Main logs raw sidecar/restoration details, while `failed` and
  `exhausted` lifecycle events receive only the stable path-free message
  `The OrkWorks sidecar is unavailable.`.
- Changed `sidecarLifecycle.launch()` to install the current generation before
  invoking `spawn()`. Synchronous spawn, token, path, timer, or listener setup
  failures now use the normal failed-state path, reject lifecycle readiness,
  publish failure through the main callback, and remain explicitly retryable.
- Added behavioral coverage for synchronous initial launch failure, a wired
  synchronous explicit-retry failure, path-free sanitization, hung retention,
  and hung provider synchronization. Both restoration-step tests verify abort,
  readiness rejection, failure publication, no `ready` publication, and no
  retained workspace.

### TDD evidence

- RED: the new lifecycle test reproduced `spawn failed for /missing-sidecar-cwd`
  escaping `start()` from `sidecarLifecycle.ts`; the sanitizer wiring test
  failed because main still forwarded `error.message`; the sanitizer module
  test initially failed with `ERR_MODULE_NOT_FOUND`.
- GREEN: after the minimal changes, the focused lifecycle/restoration/wiring
  suite passed 29/29.
- The retention and provider synchronization tests passed against the existing
  coordinator implementation, confirming that they add coverage for the
  already-required abort contract rather than depending on an unnecessary
  coordinator rewrite.

### Verification

- `cd apps/desktop && node --experimental-strip-types --test tests/sidecarLifecycle.test.ts tests/backendRestoration.test.ts tests/backendLifecycleFailure.test.ts tests/electronSidecarWiring.test.ts` — PASS, 29/29.
- `cd apps/desktop && npx tsc --noEmit` — PASS, no TypeScript errors.
- `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` — PASS, 428/428.
- `rtk codex review --uncommitted` — PASS; fresh-context review found no actionable correctness issues and reran TypeScript, focused tests, and the full desktop suite.
- `git diff --check` — PASS.

### Self-review and concerns

- Production changes are limited to `main.ts`, `sidecarLifecycle.ts`, and the
  new path-free failure helper; tests cover each new behavior. No Task 1 or
  Task 2 production files were changed.
- Node test runs retain the repository's existing
  `MODULE_TYPELESS_PACKAGE_JSON` warnings, and the existing terminal-link
  failure-path test logs its expected diagnostic. Neither causes a failure.
- The tests do not launch a packaged Electron app or a real sidecar process.
- `docs/agents/architecture.md` still carries the existing Task 5 advisory for
  complete runtime-recovery documentation.
