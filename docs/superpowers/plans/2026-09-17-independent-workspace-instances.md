# Independent Workspace Instances Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the proposed single-Electron multi-sidecar model with independent one-workspace instances and installation-persistent recent workspace history.

**Architecture:** Electron main owns one workspace state machine per process. Installation-scoped path history is shared only as durable path data, protected by a short-lived history-file lock and atomic revisioned replacement; it never publishes live instance state. Each sidecar adopts one workspace through the existing lease, while crash-surviving cleanup and relaunch adoption remain blocked on the native ownership proof from #545.

**Tech Stack:** Electron main TypeScript/Node filesystem APIs, React renderer IPC contracts, Rust Axum sidecar and workspace lease, Node built-in tests, Cargo tests, pnpm.

**Spec:** `docs/superpowers/specs/2026-09-17-independent-workspace-instances-design.md`

## Global Constraints

- Concurrent workspaces are represented by independent OrkWorks instances, not one process managing multiple workspace sidecars.
- Recent-workspace history is tied to the OrkWorks installation and survives process exits and application restarts.
- The history file contains no process IDs, ports, leases, health flags, ownership claims, peer identifiers, or open/closed state.
- A successful open requires lease acquisition, sidecar readiness, and completed workspace restoration.
- A failed destination open leaves the instance in Picker with no adopted workspace; the previous workspace is never silently reopened.
- A cleanup or ownership-proof failure leaves the instance Unresolved and prevents replacement startup.
- No runtime may claim unavailable-sidecar cleanup or replacement adoption until the applicable platform ownership mechanism is selected and proven with native evidence.
- Persisted PIDs, process names, executable paths, and released metadata locks are not process-ownership proof.
- The implementation must use pnpm for Node tasks and preserve the Electron-main/renderer boundary.

---

### Task 1: Record the independent-instance decision as repository authority

**Files:**
- Create: `docs/adr/0060-independent-workspace-instances.md`
- Modify: `docs/adr/0056-one-sidecar-per-open-workspace.md`
- Modify: `docs/adr/README.md`
- Modify: `specs/multi-workspace.md`
- Modify: `docs/validation/multi-workspace.md`
- Modify: `docs/developers.md`
- Modify: GitHub issue `#541`

**Interfaces:**
- Produces the accepted/proposed ADR and updated issue/spec vocabulary used by all later implementation tasks.
- Retains ADR 0056 as a superseded historical record; it must not remain the active authority for registry, background-sidecar, aggregate-attention, or cross-workspace-quit behavior.

- [ ] **Step 1: Write the superseding ADR and the failing documentation assertions**

  Create ADR 0060 with the decision, alternatives, consequences, installation-history boundary, one-instance lifecycle, and explicit #545 ownership prerequisite. Mark ADR 0056 `superseded by 0060` without deleting its evidence amendment. Add a documentation test or shell assertions that the old spec and validation plan no longer claim one Electron registry, background sidecars, aggregate attention, or multi-workspace quit orchestration as implementation requirements.

- [ ] **Step 2: Run the documentation assertions to verify they fail**

  Run:

  ```bash
  rg -n "one Electron registry|background workspace|aggregate attention|cross-workspace quit|one sidecar per open workspace" specs/multi-workspace.md docs/validation/multi-workspace.md docs/adr/0056-one-sidecar-per-open-workspace.md
  ```

  Expected: existing multi-sidecar claims are found before the documents are updated.

- [ ] **Step 3: Update the authoritative documents and issue acceptance criteria**

  Rewrite the old proposed spec and validation matrix around independent instances, installation-scoped path history, deterministic close-then-open behavior, and per-instance cleanup. Update `docs/developers.md` links/status text. Edit issue #541 so its acceptance criteria require the new spec/ADR and no longer require a registry, background sidecars, or aggregate attention.

- [ ] **Step 4: Run link and contradiction checks**

  Run:

  ```bash
  git diff --check
  rg -n "one Electron registry|background workspaces|aggregate attention|cross-workspace quit" specs/multi-workspace.md docs/validation/multi-workspace.md docs/adr/0056-one-sidecar-per-open-workspace.md
  ```

  Expected: `git diff --check` exits 0; remaining matches are historical explanations explicitly marked superseded, not active requirements.

- [ ] **Step 5: Commit the authority transition**

  ```bash
  git add docs/adr/0060-independent-workspace-instances.md docs/adr/0056-one-sidecar-per-open-workspace.md docs/adr/README.md specs/multi-workspace.md docs/validation/multi-workspace.md docs/developers.md
  git commit -m "docs: record independent workspace instance decision"
  ```

### Task 2: Implement installation-scoped, restart-persistent workspace history

**Files:**
- Modify: `apps/desktop/electron/workspaceMemory.ts`
- Modify: `apps/desktop/tests/electronWorkspaceMemory.test.ts`
- Modify: `apps/desktop/electron/main.ts`

**Interfaces:**
- `AppWorkspaceMemory` gains `version: 1` and `revision: number` while retaining `lastWorkspacePath: string | null` and `recentWorkspacePaths: string[]`.
- `readWorkspaceMemory(userDataPath: string): AppWorkspaceMemory` reads the installation-scoped file without publishing live state.
- `rememberWorkspacePath(userDataPath: string, workspacePath: string): AppWorkspaceMemory` and `forgetWorkspacePath(userDataPath: string, workspacePath: string): AppWorkspaceMemory` perform locked, reread-under-lock, bounded, atomic updates.
- Corrupt-file diagnostics are returned to main-process startup and picker UI without overwriting the corrupt source file.

- [ ] **Step 1: Add failing history tests**

  Extend `electronWorkspaceMemory.test.ts` with tests for version/revision round-trip, a 20-entry bound, a 64 KiB serialized bound, restart persistence from the same application-data directory, duplicate canonical paths, explicit removal without deleting the underlying workspace, corrupt-file preservation, and two simulated writers merging against the current revision while holding the history lock.

- [ ] **Step 2: Run the focused tests to verify the new cases fail**

  From `apps/desktop/`, run:

  ```bash
  node --experimental-strip-types --test tests/electronWorkspaceMemory.test.ts
  ```

  Expected: the new schema, bound, corruption, and concurrent-writer cases fail against the current direct-truncating writer.

- [ ] **Step 3: Implement the locked atomic history store**

  Keep `workspace-memory.json` under `app.getPath("userData")`. Add an installation-local lock directory with bounded acquisition retries; while holding it, reread the file, increment `revision`, merge the requested path mutation, evict oldest entries until both the 20-entry and 64 KiB limits hold, write a same-directory temporary file, flush/close it, and atomically replace the target. Leave malformed input untouched and return a diagnostic state instead of replacing it. Treat history-write failure as a convenience-state error; do not terminate an already-ready sidecar.

- [ ] **Step 4: Wire startup and successful-open ordering**

  In `main.ts`, read the installation history as a startup hint, remove the development-repository/home fallback when no valid hint exists, and remember a path only after sidecar readiness and workspace restoration complete. Keep lease conflicts and inaccessible paths in the picker. Update `forgetWorkspacePath` calls so rejecting a stale path does not erase unrelated recent entries.

- [ ] **Step 5: Run focused tests and type-check**

  ```bash
  node --experimental-strip-types --test tests/electronWorkspaceMemory.test.ts
  npx tsc --noEmit -p tsconfig.node.json
  ```

  Expected: all focused history tests pass and the Electron main-process TypeScript check exits 0.

- [ ] **Step 6: Commit the history store**

  ```bash
  git add apps/desktop/electron/workspaceMemory.ts apps/desktop/tests/electronWorkspaceMemory.test.ts apps/desktop/electron/main.ts
  git commit -m "feat: persist installation workspace history safely"
  ```

### Task 3: Make one-instance switching deterministic

**Files:**
- Create: `apps/desktop/electron/workspaceSwitchCoordinator.ts`
- Create: `apps/desktop/tests/workspaceSwitchCoordinator.test.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/backendRestoration.ts`
- Modify: `apps/desktop/electron/sidecarLifecycle.ts`
- Modify: `apps/desktop/tests/backendRestoration.test.ts`
- Modify: `apps/desktop/tests/electronSidecarWiring.test.ts`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`

**Interfaces:**
- `WorkspaceInstanceState = "picker" | "opening" | "ready" | "closing" | "unresolved"`.
- `WorkspaceSwitchCoordinator` serializes switch, retry, close, and quit operations for one Electron process; it never stores or queries peer-instance state.
- A switch operation accepts a validated destination and returns either a ready restored workspace or a typed failure state; it does not return the previous workspace as an implicit fallback.

- [ ] **Step 1: Write the coordinator state-machine tests**

  Cover validation failure preserving the current workspace, cleanup failure producing `unresolved`, successful close followed by destination lease conflict producing `picker`, readiness failure cleaning only the attempted runtime, restoration failure producing `picker`, successful restoration recording history, and repeated switch/quit requests being serialized rather than interleaved.

- [ ] **Step 2: Run the coordinator tests to verify they fail**

  ```bash
  node --experimental-strip-types --test tests/workspaceSwitchCoordinator.test.ts
  ```

  Expected: the coordinator module and its deterministic outcome assertions are absent or failing.

- [ ] **Step 3: Implement the coordinator around existing generation guards**

  Move the close-then-open ordering out of `switchWorkspaceBackend`. Gate new session/resume/foreground/analysis admissions, stop and await the current generation's bounded cleanup, enter `picker` only after cleanup is acknowledged, start the destination unadopted, await lease-backed restoration, then publish `ready` and update installation history. Reject late lifecycle/restoration events from obsolete generations.

- [ ] **Step 4: Expose picker, opening, closing, and unresolved states through the existing preload boundary**

  Update the duplicated main/renderer contract types independently. Make `App.tsx` retain the picker and diagnostic when no workspace is adopted, show an explicit retry path for destination failures, and never silently restore the previous workspace after a failed destination open. Preserve the one visible terminal/session-context rule.

- [ ] **Step 5: Replace obsolete persistence-before-start assertions**

  Update `backendRestoration.test.ts` and `electronSidecarWiring.test.ts` so they assert history is written after completed restoration, the old generation is not restarted by a late event, and no `sidecarLifecycle.stop()` or replacement path can bypass the coordinator's cleanup gate.

- [ ] **Step 6: Run the desktop focused suite and type-check**

  ```bash
  node --experimental-strip-types --test tests/workspaceSwitchCoordinator.test.ts tests/backendRestoration.test.ts tests/electronSidecarWiring.test.ts tests/sidecarLifecycle.test.ts tests/backendLifecycleWiring.test.ts
  npx tsc --noEmit
  ```

  Expected: the focused lifecycle suite passes with no Electron-main/renderer boundary violations.

- [ ] **Step 7: Commit the deterministic switch**

  ```bash
  git add apps/desktop/electron/workspaceSwitchCoordinator.ts apps/desktop/tests/workspaceSwitchCoordinator.test.ts apps/desktop/electron/main.ts apps/desktop/electron/backendRestoration.ts apps/desktop/electron/sidecarLifecycle.ts apps/desktop/tests/backendRestoration.test.ts apps/desktop/tests/electronSidecarWiring.test.ts apps/desktop/src/App.tsx apps/desktop/src/orkworksWindow.d.ts
  git commit -m "feat: make workspace switching single-instance and deterministic"
  ```

### Task 4: Bind sidecar adoption to canonical workspace identity and lease outcomes

**Files:**
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/workspaceRestore.ts`
- Modify: `crates/orkworksd/src/workspace_runtime.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Test: `crates/orkworksd/src/http/session_handlers.rs` tests and `apps/desktop/tests/workspaceRestore.test.ts`

**Interfaces:**
- Electron sends the canonical destination identity and display path separately.
- The sidecar validates the expected identity before metadata loading or reconciliation, acquires the existing workspace lease before adoption, and returns a typed external-owner conflict without inspecting the owner.
- Equivalent aliases resolve to one lease; distinct Git worktrees remain distinct workspace identities.

- [ ] **Step 1: Add failing alias, replacement, and conflict tests**

  Add Rust tests for equivalent path aliases, distinct worktrees, directory replacement before adoption, and a foreign lease owner with a surviving sentinel. Add desktop tests that map the sidecar conflict to picker/retry without deleting unrelated history.

- [ ] **Step 2: Run the focused Rust and desktop tests to verify they fail**

  ```bash
  cargo test --locked --manifest-path crates/orkworksd/Cargo.toml workspace --lib
  cd apps/desktop && node --experimental-strip-types --test tests/workspaceRestore.test.ts
  ```

  Expected: the new identity-bound adoption assertions fail against path-only restoration.

- [ ] **Step 3: Implement identity-bound adoption**

  Resolve the directory using platform filesystem semantics, pass the expected identity with the open request, verify it again in the sidecar before metadata/lease adoption, and preserve the existing `WorkspaceLease` conflict semantics. Clean up only an attempted runtime after a conflict; never wait for or terminate the foreign owner.

- [ ] **Step 4: Run Rust formatting, focused tests, and desktop checks**

  ```bash
  cargo fmt --all -- --check
  cargo test --locked --manifest-path crates/orkworksd/Cargo.toml workspace --lib
  cd apps/desktop && node --experimental-strip-types --test tests/workspaceRestore.test.ts tests/backendLifecycleWiring.test.ts
  ```

  Expected: formatting and focused tests pass; the sidecar reports conflicts without adopting foreign metadata.

- [ ] **Step 5: Commit canonical adoption**

  ```bash
  git add apps/desktop/electron/main.ts apps/desktop/electron/workspaceRestore.ts crates/orkworksd/src/workspace_runtime.rs crates/orkworksd/src/main.rs crates/orkworksd/src/http
  git commit -m "feat: bind workspace adoption to canonical identity"
  ```

### Task 5: Complete the #545 ownership prerequisite before enabling crash relaunch

**Files:**
- Modify only after native mechanism selection: `apps/desktop/electron/main.ts`, `apps/desktop/electron/sidecarLifecycle.ts`, `crates/orkworksd/src/runtime/session_runtime.rs`, `crates/orkworksd/src/runtime/terminal_runtime.rs`, `crates/orkworksd/src/providers.rs`, and `crates/orkworksd/src/session_application.rs`
- Extend: `crates/process-ownership-fixture/` or the platform-specific native fixture used by #545
- Test: native crash/relaunch integration fixtures and the existing sidecar lifecycle tests

**Interfaces:**
- Every process family that can outlive its launch request is registered before execution or is rejected fail-closed.
- Cleanup enumerates and terminates only the generation's registered descendants, acknowledges complete exit, and reports unresolved survivors within bounded deadlines.
- Relaunch adoption waits for ownership proof; a released lease or persisted PID is never sufficient.

- [ ] **Step 1: Write the native evidence matrix before production integration**

  Cover PTY, long-lived inference, model discovery, version probes, Git, shell, and harness helper launch families on Windows, macOS, and portable Linux. For each family record registration timing, detach/spawn behavior, crash cleanup, foreign-sentinel survival, and relaunch adoption.

- [ ] **Step 2: Run the existing ownership fixture and record the missing evidence**

  ```bash
  cargo test --locked --manifest-path crates/process-ownership-fixture/Cargo.toml --test evidence
  ```

  Expected: the existing fixture result is recorded as evidence only; no unsupported platform or production descendant claim is made.

- [ ] **Step 3: Add native fixture coverage for every production root**

  Add a fixture row for each process family that the production sidecar can launch. Require suspended registration before target execution where the platform supports it, retained creation-time identity, bounded cleanup, and a surviving foreign sentinel that cleanup must not touch.

- [ ] **Step 4: Integrate only the proven mechanism and keep unsupported paths fail-closed**

  Route each production spawn seam through the proven owner boundary. If registration or platform support is unavailable, reject the spawn and leave the instance `unresolved`; do not fall back to PID/name/path matching or lease-release heuristics.

- [ ] **Step 5: Run the crash/relaunch gate**

  Force-terminate Electron with one focused and one independent instance, immediately relaunch, and verify all old owned descendants are gone or safely contained before adoption. Verify the foreign sentinel remains alive and only the installation's last selected path is offered as a startup hint.

- [ ] **Step 6: Commit only after evidence is attached to the ADR and issue**

  ```bash
  git add apps/desktop/electron/sidecarLifecycle.ts crates/process-ownership-fixture docs/adr/0060-independent-workspace-instances.md
  git commit -m "feat: enforce proven crash-surviving process ownership"
  ```

### Task 6: Run the independent-instance acceptance matrix

**Files:**
- Modify: `docs/validation/multi-workspace.md`
- Test: desktop integration fixtures and the native ownership fixtures from Task 5

- [ ] **Step 1: Add installation-history scenarios**

  Verify persistence through restart, concurrent path-history updates, corrupt-file preservation, bounded eviction, explicit removal, and no live peer-state fields.

- [ ] **Step 2: Add one-instance switch scenarios**

  Verify A→B close-then-open, cleanup survivor, destination lease conflict, spawn/readiness/restoration failure, picker recovery, no implicit A reopen, and late-generation rejection.

- [ ] **Step 3: Add independent-instance isolation scenarios**

  Run two real OrkWorks processes against distinct workspaces and verify they share only installation path history/global settings contracts; neither can focus, inspect, or terminate the other. Run the same-workspace foreign-owner case and verify the lease conflict is opaque and retryable.

- [ ] **Step 4: Run the full proportionate verification set**

  ```bash
  cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts
  npx tsc --noEmit
  cd ../..
  cargo test --locked --manifest-path crates/orkworksd/Cargo.toml
  cargo test --locked --manifest-path crates/process-ownership-fixture/Cargo.toml --test evidence
  git diff --check
  ```

  Expected: all available local tests pass. Native platform gates that cannot run in the current environment remain explicitly reported as unverified rather than inferred from mocks.

- [ ] **Step 5: Commit the validation contract and request review**

  ```bash
  git add docs/validation/multi-workspace.md
  git commit -m "test: validate independent workspace instances"
  ```

  Run the required `/code-review low` gate for the code changes and attach the native evidence before requesting merge.

## Plan self-review

- Spec coverage: installation history is Task 2; instance state and switch failure semantics are Task 3; lease identity and conflicts are Task 4; quit/crash/relaunch and descendant ownership are Task 5; authority migration and acceptance evidence are Tasks 1 and 6.
- Scope boundary: Tasks 2–4 can produce independently testable desktop/sidecar behavior; Task 5 remains a hard prerequisite for crash-surviving cleanup and must not be weakened to unblock the earlier tasks.
- Boundary check: Electron main and renderer contracts remain duplicated and independently maintained; no renderer import crosses into `electron/`.
- Placeholder scan: no step relies on an unspecified implementation; unsupported native mechanisms fail closed and are explicitly tested as such.
