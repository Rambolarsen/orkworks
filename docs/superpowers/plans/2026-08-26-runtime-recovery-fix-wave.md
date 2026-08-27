# Runtime Recovery Fix-Wave Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the whole-branch runtime-recovery review findings with safe renderer diagnostics, cancellable terminal recovery, correct retry backoff, macOS provider-spawn coverage, accurate docs, and behavior-level tests.

**Architecture:** Keep Electron main as the adapter for process and IPC APIs, while moving diagnostic projection and renderer attach decisions into pure functions. Preserve the existing generation-safe sidecar controller and make its attempt counter include a stable generation's eventual failure. Keep provider invocation on plain `Command::spawn()` with piped stdio.

**Tech Stack:** TypeScript, React, Node test runner, Electron, Rust 2021, `std::process::Command`, Cargo tests, Markdown.

## Global Constraints

- Do not modify or revert `.superpowers/sdd/task-1-report.md`.
- Use pnpm for Node package-management tasks.
- Keep `electron/` and `src/` as separate TypeScript boundaries.
- Renderer diagnostics must not log arbitrary renderer payloads, prompts, workspace contents, paths, or tokens.
- Sidecar retries remain generation-safe and bounded; explicit retry remains user-triggered.
- Provider subprocess failures remain ordinary `InvocationResult` failures.
- Keep the single-active-context UI and existing recovery copy semantics.

---

### Task 1: Make renderer diagnostic logging allowlist-only

**Files:**
- Create: `apps/desktop/electron/rendererDiagnosticLog.ts`
- Modify: `apps/desktop/electron/main.ts`
- Test: `apps/desktop/tests/rendererDiagnostic.test.ts`

**Interfaces:**
- `projectRendererDiagnostic(input: unknown): RendererDiagnosticRecord` returns a record containing only the documented event-specific fields.
- `main.ts` logs the projected record for `did-fail-load`, `render-process-gone`, and `console-message`; it never passes an arbitrary event object or console payload to `console.*`.

- [ ] **Step 1: Add failing pure projection tests**

  Test exact projections for load failure and process-gone events, and assert a console-message input produces `{ type: "console-message" }` with no message field. Include secret-looking payloads containing `prompt`, `workspacePath`, bearer tokens, and nested arbitrary values; assert none appears in the serialized record.

- [ ] **Step 2: Run the focused test and confirm it fails for the missing projection**

  Run from `apps/desktop/`:

  ```bash
  node --experimental-strip-types --test tests/rendererDiagnostic.test.ts
  ```

  Expected: FAIL because the allowlist projection is not yet exported.

- [ ] **Step 3: Implement the minimal projection and wire main.ts**

  Define a narrow record union and copy only validated primitive fields. Use the existing `rendererOrigin` and `sanitizeRendererDiagnosticMessage` helpers for load error descriptions. For console-message, retain only the event type. Replace the three diagnostic `console.*` calls in `createWindow()` with `console.error`/`console.warn` on the projected record.

- [ ] **Step 4: Run the focused test and source-boundary checks**

  Run the focused test again and inspect the diagnostic handlers to confirm no raw `details`, `message`, or event object is logged.

- [ ] **Step 5: Commit the diagnostic change**

  ```bash
  git add apps/desktop/electron/rendererDiagnosticLog.ts apps/desktop/electron/main.ts apps/desktop/tests/rendererDiagnostic.test.ts
  git commit -m "fix: restrict renderer diagnostic logging"
  ```

### Task 2: Add cancellable CenterPanel backend recovery

**Files:**
- Create: `apps/desktop/src/terminalBackendRecovery.ts`
- Modify: `apps/desktop/src/components/CenterPanel.tsx`
- Test: `apps/desktop/tests/terminalBackendRecovery.test.ts`

**Interfaces:**
- `terminalBackendLookupResult(cancelled, error)` returns `{ kind: "cancelled" }` when the effect is disposed and `{ kind: "unavailable", message }` otherwise; successful lookups remain handled by the effect.
- CenterPanel renders `EmptyState` with an unavailable message and a Retry action that invokes `window.orkworks.retryBackend()` when its URL lookup fails while the backend status is connected.

- [ ] **Step 1: Add failing behavior tests for cancellation and retry action selection**

  Test that a rejected lookup becomes unavailable when active and becomes cancelled after cleanup. Test that the unavailable state’s retry callback calls `window.orkworks.retryBackend()` and that an already-cancelled lookup cannot change the state. Keep the tests independent of React and xterm by testing the pure helper and the callback contract.

- [ ] **Step 2: Run the focused test and confirm it fails**

  ```bash
  node --experimental-strip-types --test tests/terminalBackendRecovery.test.ts
  ```

  Expected: FAIL because the helper does not exist.

- [ ] **Step 3: Implement the helper and CenterPanel state path**

  Add `const [backendUnavailable, setBackendUnavailable] = useState(false)`. In the attach effect, clear the local flag for a new active lookup, set `cancelled = true` in cleanup, handle success only while active, and handle rejection through the pure helper. Return an unavailable `EmptyState` with `Retry` that clears the local flag, calls `window.orkworks.retryBackend()`, and restores the unavailable state if that explicit retry rejects. Preserve terminal disposal on backend-status changes and avoid logging the rejected error payload.

- [ ] **Step 4: Run focused tests and type-check**

  ```bash
  node --experimental-strip-types --test tests/terminalBackendRecovery.test.ts tests/backendPollingGate.test.ts
  npx tsc --noEmit
  ```

  Expected: all focused tests pass and TypeScript reports no errors.

- [ ] **Step 5: Commit the renderer recovery change**

  ```bash
  git add apps/desktop/src/terminalBackendRecovery.ts apps/desktop/src/components/CenterPanel.tsx apps/desktop/tests/terminalBackendRecovery.test.ts
  git commit -m "fix: recover CenterPanel from backend lookup failures"
  ```

### Task 3: Preserve the post-stability failure attempt and replace lifecycle regex coverage

**Files:**
- Modify: `apps/desktop/electron/sidecarLifecycle.ts`
- Modify: `apps/desktop/tests/sidecarLifecycle.test.ts`
- Modify: `apps/desktop/tests/electronSidecarWiring.test.ts`
- Test: `apps/desktop/tests/backendLifecycleWiring.test.ts`

**Interfaces:**
- `SidecarLifecycleOptions` no longer accepts an unused `fetch` member.
- A stable ready generation leaves the automatic retry budget positioned so its later failure schedules the first configured retry delay, not an immediate zero-delay retry.

- [ ] **Step 1: Add a delay assertion that exposes the reset bug**

  Extend the fake timers to record scheduled delays. Start a generation, publish a port, advance through the stability window, fail the ready process, and assert the first recovery timer uses `retryDelaysMs[0]`. Keep the existing pre-stability exhaustion test and assert its delays are `[1, 2]`.

- [ ] **Step 2: Run the lifecycle test and confirm the new assertion fails**

  ```bash
  node --experimental-strip-types --test tests/sidecarLifecycle.test.ts
  ```

  Expected: FAIL because the current reset sets `attempts` to zero and schedules a zero-delay recovery.

- [ ] **Step 3: Implement the minimal retry-accounting fix and remove fetch injection**

  Reset stable attempts to one retained failure-capable attempt, or equivalent logic that causes the next failure to select `retryDelaysMs[0]`. Remove `fetch` from the options interface, delete `void options.fetch`, and remove it from `main.ts` and test fixtures.

- [ ] **Step 4: Replace practical lifecycle source-regex tests with behavior tests**

  Move state/readiness/retry assertions into pure controller tests. Reduce `electronSidecarWiring.test.ts` to boundary checks that cannot be expressed without Electron module setup, and use exported pure lifecycle event behavior where possible. Do not add assertions that merely inspect implementation spelling.

- [ ] **Step 5: Run focused lifecycle and wiring tests**

  ```bash
  node --experimental-strip-types --test tests/sidecarLifecycle.test.ts tests/backendLifecycleEvent.test.ts tests/backendLifecycleWiring.test.ts tests/electronSidecarWiring.test.ts
  npx tsc --noEmit
  ```

- [ ] **Step 6: Commit the lifecycle change**

  ```bash
  git add apps/desktop/electron/sidecarLifecycle.ts apps/desktop/electron/main.ts apps/desktop/tests/sidecarLifecycle.test.ts apps/desktop/tests/electronSidecarWiring.test.ts apps/desktop/tests/backendLifecycleWiring.test.ts
  git commit -m "fix: preserve sidecar retry backoff after stability"
  ```

### Task 4: Add real macOS provider-spawn crash regression coverage

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `docs/agents/architecture.md`

**Interfaces:**
- `ProcessRunner::run` remains plain `Command::spawn()` with piped stdin/stdout/stderr and reports missing-command errors through `InvocationResult`.
- On macOS, a test invokes that real runner from multiple concurrent threads and confirms all calls complete without aborting the parent.

- [ ] **Step 1: Add the macOS multithreaded failing regression test**

  Add a `#[cfg(target_os = "macos")]` test that uses `std::thread::scope` and several threads to call `ProcessRunner::run` with a platform command that exits successfully. Assert every result is successful. Keep the existing platform-neutral missing-command test as the fallback invariant on other platforms.

- [ ] **Step 2: Run the focused provider tests before implementation changes**

  ```bash
  cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests
  ```

  Expected: the real-runner tests compile and the macOS regression protects the removed fork callback when run on macOS.

- [ ] **Step 3: Update architecture prose**

  Replace the claim that Unix `ProcessRunner` calls `setsid()` and closes inherited descriptors with the accurate invariant: provider commands use piped stdio and `Command`’s normal child descriptor handling; spawn errors are returned as failed invocation results. Do not claim stronger PTY isolation than the implementation provides.

- [ ] **Step 4: Run focused and complete Rust tests**

  ```bash
  cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests
  cargo test --manifest-path crates/orkworksd/Cargo.toml
  ```

- [ ] **Step 5: Commit provider and documentation changes**

  ```bash
  git add crates/orkworksd/src/providers.rs docs/agents/architecture.md
  git commit -m "test: cover concurrent provider spawning on macOS"
  ```

### Task 5: Cleanup, full verification, and fix-wave report

**Files:**
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/backendRestoration.ts`
- Modify: `apps/desktop/tests/externalLinks.test.ts` or the applicable recovery test
- Create: `.superpowers/sdd/task-fix-wave-report.md`

- [ ] **Step 1: Align recovery wording and fix whitespace**

  Ensure the recovery document text says Retry uses `location.replace(originalUrl)`. Remove the extra blank line at the end of `backendRestoration.ts` and any other touched-file whitespace errors. Leave `.superpowers/sdd/task-1-report.md` unchanged.

- [ ] **Step 2: Run the full desktop checks**

  ```bash
  cd apps/desktop
  npx tsc --noEmit
  node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
  pnpm build
  cd ../..
  ```

- [ ] **Step 3: Run repository verification**

  ```bash
  cargo test --manifest-path crates/orkworksd/Cargo.toml
  cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
  bash .claude/hooks/doc-check.sh
  bash .claude/hooks/worktree-check.sh
  git diff --check
  ```

- [ ] **Step 4: Inspect the final diff and write the report**

  Confirm the only pre-existing change remains `.superpowers/sdd/task-1-report.md`; confirm all fix-wave files are staged or committed. Write the report with summary, test commands/results, the final commit SHA, and remaining concerns such as macOS-only execution availability.

- [ ] **Step 5: Commit the report and final cleanup**

  ```bash
  git add .superpowers/sdd/task-fix-wave-report.md
  git commit -m "docs: report runtime recovery fix wave"
  git status --short --branch
  ```
