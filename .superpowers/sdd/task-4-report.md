# Task 4 report: renderer-visible backend and white-window recovery

## Status

Implemented on `fix-runtime-recovery`. Task 4 changes are ready to commit.

## Changes

- `apps/desktop/src/App.tsx`
  - Subscribes to Task 3's `window.orkworks.onBackendLifecycle` bridge.
  - Maps `ready`, `starting`/`retrying`, `failed`, and `exhausted` to the existing renderer backend status vocabulary.
  - Keeps session polling enabled only while the backend is connected, so failure states stop polling.
  - Adds a visible backend-unavailable recovery panel with a Retry action that resets status and invokes `window.orkworks.retryBackend()`.
  - Leaves workspace, session, and other existing React state intact across failure/retry.
- `apps/desktop/src/App.css`
  - Adds compact, token-based styling for the backend recovery panel and action.
- `apps/desktop/electron/main.ts`
  - Registers `did-fail-load`, `render-process-gone`, and `console-message` diagnostics.
  - Restricts renderer diagnostics to event type, relevant error/reason/origin or exit data, and bounded sanitized messages.
  - Loads an inline, resource-free recovery HTML document for main-frame load failure or renderer termination.
  - Guards recovery loading against destroyed windows and repeated fallback loads.
- `apps/desktop/tests/backendLifecycleWiring.test.ts`
  - Pins lifecycle subscription, failure status, polling gate, and retry wiring.
- `apps/desktop/tests/errorBoundaryWiring.test.ts`
  - Pins Electron renderer diagnostics and the one-button local recovery document in addition to the existing React error-boundary checks.

`apps/desktop/src/main.tsx` already wrapped `App` in the existing `ErrorBoundary`, so no change was necessary there.

## TDD evidence

- Red: the new focused tests failed 4 tests for the missing lifecycle subscription/retry wiring and main-process recovery handlers; the existing 5 checks passed.
- Green: the focused suite passed all 9 tests after implementation.

## Verification

- `cd apps/desktop && node --experimental-strip-types --test tests/backendLifecycleWiring.test.ts tests/errorBoundaryWiring.test.ts` — PASS, 9 passed, 0 failed.
- `cd apps/desktop && npx tsc --noEmit` — PASS, exit code 0.
- `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` — PASS, 433 passed, 0 failed.
- `cd apps/desktop && git diff --check` — PASS.
- `bash .claude/hooks/doc-check.sh` — exit code 0; emitted the expected reminder to consider `docs/agents/architecture.md`. That architecture document was intentionally not changed for Task 4.
- `bash .claude/hooks/worktree-check.sh` — PASS, exit code 0.

## Self-review

- No Task 1–3 production changes were reverted or modified.
- The pre-existing `.superpowers/sdd/task-1-report.md` modification remains outside the Task 4 change set.
- No architecture documentation was changed.
- Renderer diagnostics do not log renderer URLs beyond their origin, process details beyond reason/exit code, or unbounded console/error payloads.
- The fallback is main-frame-only and avoids repeated recovery navigation after the fallback document is loaded.

## Concerns

- The fallback action is intentionally `location.reload()` as required. Because the fallback is a data URL, that reloads the recovery document itself; retrying the original app URL from the fallback would be a follow-up design decision.
- Native Electron failure paths were covered by source-level wiring tests, TypeScript, and the full desktop test suite; no GUI/package smoke test was run in this session.
- Node test runs retain the repository's existing `MODULE_TYPELESS_PACKAGE_JSON` warnings.

## Commit

Planned commit message: `feat: show recovery state for renderer failures`
