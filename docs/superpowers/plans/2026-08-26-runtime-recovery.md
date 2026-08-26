# Runtime Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent macOS provider-spawn aborts and make sidecar/renderer failures visible and recoverable instead of leaving OrkWorks white or permanently disconnected.

**Architecture:** Keep provider invocation behavior behind the existing `ProviderRunner` seam, but remove the Unix `pre_exec` fork callback. Extract Electron sidecar generation/readiness/retry behavior into a pure lifecycle module, then make `main.ts` the adapter for Electron process/IPC APIs. Add a renderer lifecycle context and a main-process local recovery document so failures before React mounts remain recoverable.

**Tech Stack:** Rust 2021, Tokio, `std::process::Command`, Electron 39, TypeScript, React 19, Node test runner.

## Global Constraints

- Use pnpm for Node package-management tasks.
- `electron/` and `src/` must not import from each other; duplicate boundary types when needed.
- Provider subprocess failures must remain ordinary fallback failures, not daemon panics.
- Sidecar retries must be bounded and generation-safe; no infinite automatic restart loop.
- Renderer diagnostics must not log arbitrary prompts, workspace contents, or tokens.
- Keep the single-active-context UI model and existing workspace/session semantics.
- Update `docs/agents/architecture.md` when the Electron sidecar lifecycle contract changes.

---

### Task 1: Remove the unsafe provider fork callback

**Files:**
- Modify: `crates/orkworksd/src/providers.rs:430-464`
- Test: `crates/orkworksd/src/providers.rs` test module, adding a direct real-runner test helper

**Interfaces:**
- Consumes: existing `ProviderRunner`, `ProcessRunner`, and `InvocationResult`.
- Produces: provider process startup through plain `Command::spawn()` with piped stdio; spawn errors remain `InvocationResult { success: false, ... }`.

- [ ] **Step 1: Add a real-runner regression test**

Add a test-only helper that invokes `ProcessRunner::run` directly. Cover a successful command using the platform's built-in no-op (`true` on Unix and `cmd /C exit 0` on Windows) and a deliberately missing executable. Assert success/failure and that the failure is returned rather than panicking. Keep the test independent of provider configuration.

- [ ] **Step 2: Run the focused Rust test and confirm the baseline**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests
```

Expected: existing tests pass; the new test compiles and fails only if the helper or expected result is incorrect.

- [ ] **Step 3: Remove the `pre_exec` block**

Delete the entire Unix `cmd.pre_exec` closure, including `setsid`, `sysconf`, and the descriptor loop. Keep:

```rust
let mut cmd = Command::new(command);
for arg in args {
    cmd.arg(arg);
}
cmd.stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
let mut child = match cmd.spawn() {
    Ok(child) => child,
    Err(error) => {
        tracing::warn!(provider = %id, error = %error, "peon: failed to spawn");
        return InvocationResult {
            success: false,
            stdout: String::new(),
            stderr: error.to_string(),
        };
    }
};
```

- [ ] **Step 4: Run the focused and complete Rust suites**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: both commands exit 0 and no provider test reports a panic or abort.

- [ ] **Step 5: Commit the Rust fix**

```bash
git add crates/orkworksd/src/providers.rs
git commit -m "fix: avoid unsafe provider fork callback"
```

### Task 2: Build a testable sidecar lifecycle controller

**Files:**
- Create: `apps/desktop/electron/sidecarLifecycle.ts`
- Test: `apps/desktop/tests/sidecarLifecycle.test.ts`

**Interfaces:**
- Produces `SidecarLifecycle` with `start(cwd: string): Promise<number>`, `stop(): void`, `retry(): Promise<number>`, `getPort(): number | null`, and `dispose(): void`.
- `SidecarLifecycle` accepts injected `spawn`, `fetch`, `setTimeout`, `clearTimeout`, `now`, and callbacks `{ onReady, onUnavailable, onState }` so tests never launch Electron or a real sidecar.
- Each process callback carries a generation number; stale stdout/exit/error events are ignored.

- [ ] **Step 1: Write failing lifecycle tests**

Cover these cases in `sidecarLifecycle.test.ts`:

```ts
test("rejects readiness when the process exits before publishing a port", async () => {
  // fake spawn emits exit(1) without ORKWORKSD_PORT
  // assert start() rejects and getPort() is null
});

test("ignores exit from an obsolete generation", async () => {
  // start generation 1, replace it with generation 2, then emit generation 1 exit
  // assert generation 2 remains current and ready
});

test("stops after three automatic attempts and permits explicit retry", async () => {
  // make three starts fail, assert exhausted state and no fourth automatic spawn
  // call retry(), make the next start publish a port, assert ready
});
```

Also test post-ready exit notification, spawn error, readiness timeout, and only-one-recovery-sequence behavior.

- [ ] **Step 2: Run the focused test to verify it fails**

Run:

```bash
node --experimental-strip-types --test tests/sidecarLifecycle.test.ts
```

Expected: FAIL because `sidecarLifecycle.ts` does not yet exist.

- [ ] **Step 3: Implement the lifecycle controller**

Implement an explicit state model:

```ts
type SidecarState = "starting" | "ready" | "failed" | "retrying" | "exhausted";
```

Use a monotonically increasing generation, a readiness promise per generation, a readiness timeout, and retry delays such as 250ms and 1s. Reject readiness on spawn error, pre-ready exit, and timeout. Clear the port on current-generation failure. Ignore callbacks whose generation is not current. Reset the attempt counter only after a successful ready period, not immediately on port publication.

- [ ] **Step 4: Run lifecycle tests until green**

Run the focused command again. Expected: all lifecycle tests pass, including fake-timer retry tests.

- [ ] **Step 5: Commit the lifecycle module**

```bash
git add apps/desktop/electron/sidecarLifecycle.ts apps/desktop/tests/sidecarLifecycle.test.ts
git commit -m "feat: add generation-safe sidecar lifecycle"
```

### Task 3: Integrate sidecar recovery into Electron main

**Files:**
- Modify: `apps/desktop/electron/main.ts:20-145,466-525`
- Modify: `apps/desktop/electron/preload.ts` and `apps/desktop/src/orkworksWindow.d.ts` for the lifecycle event/retry bridge
- Test: `apps/desktop/tests/electronSidecarWiring.test.ts`

**Interfaces:**
- Consumes: `SidecarLifecycle` from Task 2.
- Produces: `window.orkworks.onBackendLifecycle(callback)` and `window.orkworks.retryBackend()`; existing `get-backend-url` and workspace APIs use the current generation readiness promise.

- [ ] **Step 1: Add source-level wiring tests**

Assert that `main.ts` constructs one lifecycle controller, routes both initial startup and `open-workspace` through it, handles lifecycle notifications, and no longer contains a second direct `spawn(...ORKWORKS_OPEN_PLAN_TOKEN...)` implementation. Assert preload and renderer declarations expose the same callback payload shape:

```ts
type BackendLifecycleEvent =
  | { state: "starting" | "retrying" }
  | { state: "ready"; port: number }
  | { state: "failed" | "exhausted"; message: string };
```

- [ ] **Step 2: Implement centralized startup and workspace restoration**

Move sidecar environment construction and process spawning behind the lifecycle controller. On ready, POST the remembered/current workspace path, then apply retention and saved provider settings before emitting ready to the renderer. On workspace switch, stop the old generation, update `workspacePath`, remember it, and start the replacement generation. Use the generation guard so an old exit cannot clear the replacement.

- [ ] **Step 3: Forward lifecycle state through preload**

Add an IPC listener in `preload.ts` that validates the event shape before invoking the renderer callback. Add `retryBackend()` as a one-shot IPC request that calls the lifecycle controller’s explicit retry. Do not expose the sidecar process, filesystem paths, or tokens to the renderer.

- [ ] **Step 4: Update backend URL consumers to fail normally**

Have `get-backend-url` await the current readiness promise and reject after the lifecycle timeout/failure. Ensure `get-initial-workspace`, plan handlers, and workspace integration handlers catch or propagate this rejection rather than awaiting a promise that can never settle.

- [ ] **Step 5: Run desktop wiring tests and type-check**

Run:

```bash
node --experimental-strip-types --test tests/sidecarLifecycle.test.ts tests/electronSidecarWiring.test.ts
npx tsc --noEmit
```

Expected: all focused tests pass and TypeScript reports no errors.

- [ ] **Step 6: Commit Electron sidecar integration**

```bash
git add apps/desktop/electron apps/desktop/src/orkworksWindow.d.ts apps/desktop/tests/electronSidecarWiring.test.ts
git commit -m "feat: recover from sidecar lifecycle failures"
```

### Task 4: Add renderer-visible backend and white-window recovery

**Files:**
- Modify: `apps/desktop/src/main.tsx`, `apps/desktop/src/App.tsx`, `apps/desktop/src/App.css`
- Modify: `apps/desktop/electron/main.ts`
- Test: `apps/desktop/tests/backendLifecycleWiring.test.ts`, `apps/desktop/tests/errorBoundaryWiring.test.ts`

**Interfaces:**
- Consumes: `BackendLifecycleEvent`, `window.orkworks.onBackendLifecycle`, and `window.orkworks.retryBackend()` from Task 3.
- Produces: visible retryable backend-unavailable state while React is alive, plus a main-process recovery document for pre-mount/renderer-gone failures.

- [ ] **Step 1: Add failing renderer wiring tests**

Assert that `App.tsx` subscribes to backend lifecycle events, stops polling when state is not ready, displays a retry action for `failed`/`exhausted`, and invokes `retryBackend()`. Assert `main.ts` registers `did-fail-load` and `render-process-gone` and loads a recovery document that contains a single reload/retry action.

- [ ] **Step 2: Implement the renderer lifecycle state**

Initialize the existing `backendStatus` from lifecycle events, preserve current health polling for normal startup, and on failure set status to `unreachable`/`exhausted` so terminal/session polling stops. Add a compact recovery panel with a clear message and button; retry resets the status to `connecting…` and invokes the preload bridge. Keep all existing workspace/session state intact.

- [ ] **Step 3: Implement main-process renderer diagnostics and fallback**

Register `did-fail-load`, `render-process-gone`, and `console-message` handlers. Log only event type, error code/reason, URL origin, process reason/exit code, and allowlisted console metadata (level, origin, line), with renderer message payloads excluded. For load failure or renderer termination, load a local recovery HTML string with no external resources and a button that calls `location.replace(originalUrl)`.

- [ ] **Step 4: Run renderer tests and type-check**

Run:

```bash
node --experimental-strip-types --test tests/backendLifecycleWiring.test.ts tests/errorBoundaryWiring.test.ts
npx tsc --noEmit
```

Expected: focused tests pass and TypeScript reports no errors.

- [ ] **Step 5: Commit renderer recovery**

```bash
git add apps/desktop/src apps/desktop/electron/main.ts apps/desktop/tests/backendLifecycleWiring.test.ts apps/desktop/tests/errorBoundaryWiring.test.ts
git commit -m "feat: show recovery state for renderer failures"
```

### Task 5: Document and verify the complete fix

**Files:**
- Modify: `docs/agents/architecture.md` Electron/sidecar lifecycle section
- Verify: `docs/superpowers/specs/2026-08-26-runtime-recovery-design.md`, `.claude/hooks/doc-check.sh`, `.claude/hooks/worktree-check.sh`

**Interfaces:**
- Consumes: completed Rust and Electron runtime behavior from Tasks 1–4.
- Produces: architecture documentation matching the generation-safe lifecycle, readiness failure, retry, and renderer fallback contract.

- [ ] **Step 1: Update architecture documentation**

Document that sidecar readiness is generation-specific, pre-ready failures reject, post-ready failures notify the renderer, workspace/provider settings are restored before ready, retries are capped at three automatic attempts, and renderer failures have a main-process fallback document.

- [ ] **Step 2: Run complete verification**

Run from the repository root:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
cd apps/desktop
npx tsc --noEmit
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
pnpm build
cd ../..
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
git diff --check
```

Expected: Rust tests, desktop tests, type-check, build, doc check, worktree check, and whitespace check all pass. Any doc-check trigger is addressed before handoff.

- [ ] **Step 3: Review the final diff**

Run:

```bash
git status --short
git diff main...HEAD --stat
git diff main...HEAD -- crates/orkworksd/src/providers.rs apps/desktop/electron apps/desktop/src docs/agents/architecture.md
```

Confirm no unrelated files changed, no sensitive diagnostics are logged, and every new lifecycle callback is generation-guarded.

- [ ] **Step 4: Commit documentation and verification changes**

```bash
git add docs/agents/architecture.md
git commit -m "docs: document runtime recovery lifecycle"
```
