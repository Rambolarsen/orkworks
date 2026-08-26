# Task 4 report: renderer-visible backend and white-window recovery

## Status

Implemented on `fix-runtime-recovery` and committed as `4f6f047` (`feat: show recovery state for renderer failures`). This report was finalized in the follow-up report-only commit after the implementation commit.

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

`4f6f047` — `feat: show recovery state for renderer failures`

## Review-fix follow-up — 2026-08-26

### Status

Implemented in the current `fix-runtime-recovery` branch. The pre-existing
uncommitted edit to `task-1-report.md` remains untouched and is not part of
this changeset.

### Changes

- `apps/desktop/electron/main.ts` now captures the original renderer URL:
  `VITE_DEV_SERVER_URL` in development or the packaged `dist/index.html`
  file URL in production. The local recovery document embeds that URL safely
  and its single Retry action calls `location.replace(originalUrl)`.
- `apps/desktop/electron/rendererDiagnostic.ts` owns renderer-origin
  extraction and diagnostic sanitization. It redacts bearer credentials,
  structured sensitive fields, URLs, and common absolute filesystem paths
  before bounding messages to 200 characters.
- `apps/desktop/src/backendPollingGate.ts` owns the session-polling predicate,
  which requires a connected backend, a workspace, and no active workspace
  switch. `App.tsx` now includes `isSwitchingWorkspace` in both the predicate
  and effect dependencies.
- Added direct redaction and polling-gate tests. The polling tests cover the
  `starting → ready → openWorkspace complete` sequence so a ready replacement
  backend cannot restart polling against the old workspace during a switch.

### TDD evidence

1. Added the recovery, redaction, and polling tests before the new production
   modules and predicate wiring.
2. The first focused run was expected RED: 4 failures (missing diagnostic and
   polling modules, stale polling assertion, and missing original-URL
   recovery contract).
3. After the minimal implementation, one redaction assertion exposed a
   duplicate marker for `Authorization: Bearer ...`; the helper was corrected
   to consume the bearer value once. The focused suite then passed.

### Verification

| Command | Result |
| --- | --- |
| `rtk node --experimental-strip-types --test tests/backendLifecycleWiring.test.ts tests/errorBoundaryWiring.test.ts tests/rendererDiagnostic.test.ts tests/backendPollingGate.test.ts` | PASS — 14 passed |
| `rtk npx tsc --noEmit` | PASS — no errors |
| `rtk node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` | PASS — 438 passed, 0 failed |
| `rtk pnpm build` | PASS — Electron TypeScript and Vite build completed; Vite emitted the existing large-chunk warning |
| `rtk git diff --check` | PASS |

### Self-review and concerns

- The scoped diff contains only the recovery/polling helpers, their tests,
  the two existing wiring-test updates, `App.tsx`, `main.ts`, and this report;
  Task 1–3 implementation files are unchanged.
- The focused and full Node tests continue to emit the repository's existing
  `MODULE_TYPELESS_PACKAGE_JSON` warnings.
- No packaged GUI smoke test was run; the Electron-main path was type-checked
  and included in the successful desktop build.

### Commit

The review fixes and this follow-up report are included in the final fix
commit; its hash is reported at handoff because changing this line changes the
hash itself.
