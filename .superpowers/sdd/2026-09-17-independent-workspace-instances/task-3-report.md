# Task 3 report: deterministic one-instance workspace switching

Status: Task 3 implementation complete; bounded focused verification passed.

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
  gate.
- Moved successful history recording into the post-restoration coordinator
  path, before ready publication, while retaining Task 2's convenience-state
  failure behavior.
- Extended and independently duplicated the main/preload/renderer lifecycle
  contract for picker diagnostics and opening/closing/unresolved states.
- Updated `App.tsx` to gate polling during transitions, retain picker and
  diagnostics after failure, and expose explicit retry behavior.

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
- Task 5 still owns native crash-surviving process ownership evidence.
