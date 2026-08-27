# Runtime Recovery Latest-Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the final runtime-recovery review findings across Electron, the renderer, and the Rust provider runner.

**Architecture:** Publish restored workspace identity in the validated ready event, adopt it synchronously in the generation-safe renderer controller before polling, and keep local terminal attach recovery separate from global lifecycle recovery. Preserve bounded lifecycle attempts and validate sidecar protocol data at the lifecycle boundary.

**Tech Stack:** Electron/TypeScript, React, Node test runner, Rust 2021, Cargo tests.

## Global Constraints

- Leave `.superpowers/sdd/task-1-report.md` untouched.
- Use pnpm for Node package-management tasks.
- Keep Electron-main and renderer TypeScript boundaries separate; duplicate contract types where required.
- Three launches total per automatic recovery sequence, including a stable generation's eventual failure; preserve the first backoff delay.
- Renderer diagnostics must not leak prompts, workspace contents, credentials, or absolute paths.
- Keep one active terminal context and preserve generation/cancellation safety.

---

### Task 1: Publish and adopt restored workspace identity

**Files:**
- Modify: `apps/desktop/electron/backendLifecycleEvent.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Modify: `apps/desktop/src/workspaceSessionController.ts`
- Modify: `apps/desktop/src/App.tsx`
- Test: `apps/desktop/tests/backendLifecycleEvent.test.ts`
- Test: `apps/desktop/tests/workspaceSessionController.test.ts`
- Test: `apps/desktop/tests/electronSidecarWiring.test.ts`

**Behavior:** The ready event contains a validated restored workspace or null. App/controller adoption happens before connected polling. A failed switch leaves the old identity until a later ready event, and retry adoption publishes the new identity before refreshing sessions.

- [ ] Add failing event-shape and controller retry-after-failure tests.
- [ ] Run the focused tests and observe the expected missing workspace/adoption behavior.
- [ ] Add the duplicated validated workspace contract, publish it from restoration, and add `adoptRestoredWorkspace` with generation guards.
- [ ] Route ready events and successful switch results through adoption; avoid the second `/workspace` POST.
- [ ] Run focused event/controller/wiring tests and TypeScript.

### Task 2: Correct bounded retry and sidecar protocol buffering

**Files:**
- Modify: `apps/desktop/electron/sidecarLifecycle.ts`
- Modify: `apps/desktop/tests/sidecarLifecycle.test.ts`

**Behavior:** Stability resets the automatic attempt count to one retained launch. Invalid announced ports fail the generation before readiness. Pre-readiness stdout is capped.

- [ ] Add failing tests for stable-sequence launch count, invalid ports, and a large stdout prefix followed by a valid marker.
- [ ] Run lifecycle tests and confirm the failures.
- [ ] Reset attempts to one after stability, validate `1..=65535`, and cap the candidate stdout buffer.
- [ ] Run lifecycle tests and verify old-generation/cancellation cases remain green.

### Task 3: Broaden safe renderer diagnostic path redaction

**Files:**
- Modify: `apps/desktop/electron/rendererDiagnostic.ts`
- Test: `apps/desktop/tests/rendererDiagnostic.test.ts`

- [ ] Add failing cases for `/workspace`, `/opt`, `/Volumes`, and another arbitrary POSIX path while asserting useful surrounding metadata remains.
- [ ] Run the diagnostic test and confirm the old narrow allowlist leaks the new paths.
- [ ] Replace the narrow POSIX root list with boundary-aware arbitrary absolute POSIX redaction without changing URL or non-path text behavior.
- [ ] Run the diagnostic tests.

### Task 4: Make CenterPanel attach recovery local and cancellable

**Files:**
- Create: `apps/desktop/src/terminalBackendRecovery.ts`
- Modify: `apps/desktop/src/components/CenterPanel.tsx`
- Modify: `apps/desktop/src/components/TerminalPanel.tsx`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/App.tsx`
- Test: `apps/desktop/tests/terminalBackendRecovery.test.ts`
- Modify: `apps/desktop/tests/dockview.test.ts`

- [ ] Add failing pure helper tests for rejection, cancellation, successful retry, and retry failure.
- [ ] Run the focused helper test and confirm the missing seam failure.
- [ ] Implement local unavailable/retrying state, call `retryBackend`, re-run attachment on success, and invalidate async callbacks on cleanup.
- [ ] Remove the old attach-failure escalation prop and update source-wiring assertions.
- [ ] Run terminal recovery, Dockview, and TypeScript checks.

### Task 5: Strengthen macOS provider concurrency regression

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`

- [ ] Replace the single real invocation plus sibling thread with a barrier-synchronized set of real invocations.
- [ ] Run provider-focused Cargo tests on this Darwin host and the complete Rust suite.

### Task 6: Verify, document, and commit

**Files:**
- Create: `.superpowers/sdd/task-latest-fix-report.md`

- [ ] Run focused desktop/Rust tests, full desktop tests, TypeScript, desktop build, Rust tests/build, diff check, doc check, and worktree check.
- [ ] Confirm `.superpowers/sdd/task-1-report.md` is the only pre-existing dirty file and remains byte-for-byte untouched.
- [ ] Write the report with findings, test evidence, platform limitations, and final commit SHA.
- [ ] Commit the design, implementation, tests, and report without staging the pre-existing report.
