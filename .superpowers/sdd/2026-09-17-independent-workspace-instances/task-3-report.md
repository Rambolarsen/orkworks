# Task 3 report: deterministic one-instance workspace switching

Status: Task 3 implementation complete; review fix complete with bounded focused verification passed.

Worktree: `/Users/froomiebot/workspace/orkworks/.worktrees/545-production-seam-audit`

## Completed

- Added `WorkspaceSwitchCoordinator` with serialized switch, picker, retry,
  close, and quit operations and the states `picker`, `opening`, `ready`,
  `closing`, and `unresolved`.
- Validated destinations as accessible directories before closing the current
  runtime; validation failures preserve the current workspace.
- Made switching deterministic: close the current runtime, await bounded
  cleanup, enter picker/opening, start the destination unadopted, await
  sidecar readiness and workspace restoration, then publish ready only for the
  current generation.
- Added typed destination, readiness, restoration, cleanup, and quit failure
  outcomes. Destination failures remain in picker; cleanup failures remain
  unresolved; no failed destination silently reopens the previous workspace.
- Added awaited sidecar cleanup with a bounded timeout and intentional
  restoration cancellation so replacement and quit cannot bypass the cleanup
  gate. Cleanup timeout is a typed failure; the coordinator remains unresolved
  and does not admit the destination while the old process is still running.
- Disabled sidecar automatic recovery. A failed generation is reported to the
  coordinator, and the only recovery path is the explicit serialized
  `retry-backend` → `WorkspaceSwitchCoordinator.retry()` flow. Successful
  retry runs restoration and publishes `ready` through the coordinator.
- Moved successful history recording into the post-restoration coordinator
  path, before ready publication, while retaining Task 2's convenience-state
  failure behavior.
- Extended and independently duplicated the main/preload/renderer lifecycle
  contract for picker diagnostics and opening/closing/unresolved states.
- Updated `App.tsx` so the coordinator lifecycle is the renderer admission
  authority: create, resume, foreground submission, session selection, and
  Taskmaster fix handoff are closed outside `ready`, while the single active
  terminal/session-context rule remains intact. Polling is gated as well.
- Invalidated `backendGeneration` synchronously at close admission, before
  delayed sidecar cleanup, so in-flight old-workspace mutations fail their
  post-await generation check.
- Routed recommendation dismissal, Fix-with-AI handoff, and debug-attention
  injection through generation-bound main-process IPC; opening, closing, and
  unresolved lifecycle states reject these mutations.
- Preserved attempted-runtime cleanup failures independently from the nullable
  workspace path, so path-null close/quit requests remain unresolved and do
  not dispose the lifecycle until an explicit retry acknowledges cleanup.
- Kept cleanup failure and timeout tombstones authoritative after a later
  process exit, so `start()` and `retry()` remain blocked until Task 5 supplies
  an explicit native ownership receipt; added deterministic timeout → exit →
  start/retry regression coverage.
- Kept typed retry outcomes intact through the renderer: a destination retry
  returning `picker` remains picker, while unresolved cleanup retains its
  diagnostic and status.

## Verification

From `apps/desktop/`:

```text
node --experimental-strip-types --test \
  tests/workspaceSwitchCoordinator.test.ts \
  tests/backendRestoration.test.ts \
  tests/electronSidecarWiring.test.ts \
  tests/sidecarLifecycle.test.ts \
  tests/backendLifecycleWiring.test.ts
PASS — 59 tests, 0 failures

npx tsc --noEmit -p tsconfig.node.json
PASS

npx tsc --noEmit -p tsconfig.json
PASS
```

The lifecycle contract tests also passed with the new state and diagnostic
payload cases. `git diff --check` passed. No full desktop or Rust suite and no
network install were run.

## Scope notes

- No peer-instance registry or coordination was added.
- Task 4 still owns canonical identity and sidecar lease-adoption changes.
- Task 5 still owns native crash-surviving process ownership evidence. That
  native descendant proof remains unimplemented here; an unexpected sidecar
  exit therefore publishes `unresolved` and blocks replacement until cleanup
  and ownership are explicitly acknowledged. No crash-relaunch proof is
  claimed.

## Review-fix follow-up (2026-09-17)

The follow-up closes the path-null attempted-runtime cleanup seam and preserves
the picker result across the retry IPC/renderer boundary. The new coordinator
regression covers a failed destination cleanup, path-null quit rejection,
blocked replacement startup, explicit cleanup acknowledgement, and recovery.
The wiring regressions cover both typed picker and unresolved retry results.

Verification for this follow-up, from `apps/desktop/`:

```text
node --experimental-strip-types --test \
  tests/sidecarLifecycle.test.ts \
  tests/workspaceSwitchCoordinator.test.ts \
  tests/backendRestoration.test.ts \
  tests/electronSidecarWiring.test.ts \
  tests/backendLifecycleWiring.test.ts \
  tests/backendLifecycleEvent.test.ts
PASS — 72 tests, 0 failures

npx tsc --noEmit -p tsconfig.node.json
PASS

npx tsc --noEmit -p tsconfig.json
PASS

git diff --check
PASS
```

No full suite or network-dependent command was run.

## Final Task 3 verification (2026-09-17)

```text
node --experimental-strip-types --test \
  tests/sidecarLifecycle.test.ts \
  tests/workspaceSwitchCoordinator.test.ts \
  tests/backendRestoration.test.ts \
  tests/electronSidecarWiring.test.ts \
  tests/backendLifecycleWiring.test.ts \
  tests/backendLifecycleEvent.test.ts \
  tests/workspaceSessionController.test.ts
PASS — 97 tests, 0 failures

npx tsc --noEmit -p tsconfig.node.json
PASS

npx tsc --noEmit -p tsconfig.json
PASS

git diff --check
PASS
```

The final admission/ownership guard round also passed the same focused suite at
97/97 and both TypeScript configurations. Unexpected sidecar exits remain
fail-closed and replacement-blocked until Task 5 supplies native descendant
ownership proof; no crash-relaunch proof is claimed.
