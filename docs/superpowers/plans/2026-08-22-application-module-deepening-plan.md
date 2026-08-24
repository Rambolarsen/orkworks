# Application Module Deepening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move session, workspace, and settings orchestration behind deep, testable interfaces while preserving current product behavior and process contracts.

**Architecture:** Freeze existing behavior and exact contracts first. Then extract the Rust session application seam, renderer workspace/session controller, and renderer settings workflow in sequence. HTTP, preload, metadata persistence, and terminal transport remain existing Adapters or deep modules; no second state owner is introduced.

**Tech Stack:** Rust, Axum, Tokio, TypeScript, React, Electron IPC, Node test runner, Cargo tests.

## Global Constraints

- Use pnpm for Node package-management tasks.
- `apps/desktop/electron/` and `apps/desktop/src/` must never import from each other.
- Preserve REST routes, JSON payloads, preload security, persisted settings format, and the single-active-session model.
- Read root `AGENTS.md` plus the scoped desktop/Rust instructions before editing.
- Use a branch/worktree for code changes; use TDD and verification before completion.
- Run doc-currency and worktree-currency checks before ending.
- Do not add a generic frontend service layer, state-management dependency, or speculative Adapter.

## File Map

- `crates/orkworksd/src/session_application.rs`: session/workspace commands and stable errors.
- `crates/orkworksd/src/http/session_handlers.rs`: HTTP request/response Adapter after extraction.
- `apps/desktop/src/workspaceSessionController.ts`: renderer session/workspace orchestration.
- `apps/desktop/src/settingsController.ts`: renderer settings draft/commit workflow.
- `apps/desktop/src/App.tsx`: React state/view wiring after extraction.
- `apps/desktop/src/components/SettingsModal.tsx`: settings composition after extraction.

### Task 1: Freeze behavior and cross-seam contracts — complete

**Files:**
- Create: `docs/superpowers/specs/2026-08-22-application-module-contracts.md`
- Test: existing Rust session-handler tests and desktop API/polling/pending-create/settings tests

**Interfaces:**
- Produces the exact symbols, constructors, ownership rules, error mappings, and acceptance criteria consumed by Tasks 2–4.

- [ ] **Step 1: Inventory current behavior.**

Record current symbols and status/body behavior for `set_workspace`, `create_session`, `resume_session`, `report_attention`, plan selection, `delete_session`, and `forget_session`. Record preload settings handlers and their normalization/default behavior.

- [ ] **Step 2: Freeze the Rust contract.**

Define exact command/result/error types. `SessionApplication` must wrap `Arc<AppState>` or another explicit reference to existing state and must not own a second session map. Document workspace lookup, concurrency/admission behavior, sync versus async work, side-effect ordering, compensation, and the HTTP mapping table.

- [ ] **Step 3: Freeze the renderer contract.**

Require `openWorkspace(path: string)`. Define one polling owner, an operation generation or cancellation token for every async operation, stale-result rejection, active-session restoration/deletion transitions, pending-create correlation by returned session ID, terminal-pruning order, notification suppression, and post-disposal behavior.

- [ ] **Step 4: Freeze the settings contract.**

Define `load`, `updateDraft`, `discard`, `verifyOllama`, `resetHotkey`, and `commit` for hotkeys, retention, debug, providers, and integrations. Electron remains the authority for defaults. Provider verification must not mutate saved settings. A failed domain save retains the renderer draft.

- [ ] **Step 5: Add characterization assertions and run them.**

Pin current REST responses, pending-create resolution by exact session ID, active-session restoration after session loading, settings defaults, and provider-verification non-mutation.

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/sessionPolling.test.ts tests/pendingCreate.test.ts tests/electronSettingsMemory.test.ts
```

Expected: existing tests pass and the contract document has no undefined type, path, ownership, or compatibility decision.

- [ ] **Step 6: Commit.**

```bash
git add docs/superpowers/specs/2026-08-22-application-module-contracts.md crates/orkworksd/src/http/session_handlers.rs apps/desktop/tests
git commit -m "test: freeze application module contracts"
```

### Task 2: Extract the Rust session application seam — complete

**Files:**
- Create: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Test: `session_application.rs` and existing handler tests

**Interfaces:**
- Consumes the exact contract from Task 1.
- Produces `SessionApplication::open_workspace`, `create_session`, `resume_session`, `report_attention`, plan selection, and delete/forget operations with documented types.

- [ ] **Step 1: Add failing application-interface tests** for workspace reconciliation, creating persistence, resume admission conflicts, attention priority, validated plan selection, and delete-versus-forget semantics.
- [ ] **Step 2: Implement the application object and stable errors** while keeping metadata, observed-status, plan-handoff, and runtime modules as internal dependencies.
- [ ] **Step 3: Preserve side-effect ordering and existing compensation behavior.** Add tests for partial-failure recovery rather than claiming atomicity the current system does not provide.
- [ ] **Step 4: Thin the handlers** to extraction, authorization, application invocation, compatibility mapping, and serialization.
- [ ] **Step 5: Verify.**

```bash
cargo fmt --all -- --check
cargo test --manifest-path crates/orkworksd/Cargo.toml
cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings
```

- [ ] **Step 6: Commit** with `git commit -m "refactor: deepen session application module"`.

### Task 3: Extract the renderer workspace/session controller — complete

**Files:**
- Create: `apps/desktop/src/workspaceSessionController.ts`
- Create: `apps/desktop/tests/workspaceSessionController.test.ts`
- Modify: `apps/desktop/src/App.tsx`
- Reuse: `api.ts`, `sessionPolling.ts`, `pendingCreate.ts`, and `sessionSort.ts`

**Interfaces:**
- Consumes existing `api.ts` and the Task 1 REST contract.
- Produces `openWorkspace(path)`, `refreshSessions()`, `createSession()`, `resumeSession(id)`, `selectSession(id)`, `deleteSession(id, forget)`, and `dispose()`.
- Uses a monotonically increasing operation generation; stale/disposed operations cannot invoke state callbacks.

- [ ] **Step 1: Add failing tests** for one polling loop, stale workspace rejection, disposal, pending-create correlation, restoration ordering, active deletion, and terminal-pruning order.
- [ ] **Step 2: Implement generation checks** and make `dispose()` invalidate all pending work.
- [ ] **Step 3: Move polling ownership** into the controller; `App.tsx` must not start a second loop. Suppress repeated errors by error key until a successful refresh.
- [ ] **Step 4: Move session workflows** into the controller while keeping React state in `App.tsx`. The controller emits accepted snapshots/results through callbacks; it does not become a second store.
- [ ] **Step 5: Verify.**

```bash
cd apps/desktop
npx tsc --noEmit
node --experimental-strip-types --test tests/workspaceSessionController.test.ts tests/sessionPolling.test.ts tests/pendingCreate.test.ts tests/api.test.ts
```

- [ ] **Step 6: Commit** with `git commit -m "refactor: deepen workspace session controller"`.

### Task 4: Extract the renderer settings workflow

**Files:**
- Create: `apps/desktop/src/settingsController.ts`
- Create: `apps/desktop/tests/settingsController.test.ts`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Test: existing Electron settings, provider sync, and providers-panel tests

**Interfaces:**
- Consumes the existing typed `window.orkworks` settings methods.
- Produces `load`, `updateDraft(domain, value)`, `discard`, `verifyOllama(baseUrl)`, `resetHotkey(action)`, and `commit`.
- `commit` sends changed domains in deterministic order, retains the draft on failure, and returns the latest renderer-facing settings on success.

- [ ] **Step 1: Add failing tests** for draft isolation, discard, Electron-provided defaults, reset, verification without save, and failed commit recovery.
- [ ] **Step 2: Implement separate durable snapshot and draft state.** Use Electron for normalization/defaults; do not duplicate `DEFAULT_SETTINGS` in the renderer.
- [ ] **Step 3: Serialize domain saves** through the existing main-process `currentSettings` behavior. Do not claim cross-domain atomicity unavailable in the current IPC contract.
- [ ] **Step 4: Make `SettingsModal` a composition layer** for hotkeys, providers, integrations, retention, and debug settings.
- [ ] **Step 5: Verify.**

```bash
cd apps/desktop
npx tsc --noEmit
node --experimental-strip-types --test tests/settingsController.test.ts tests/electronSettingsMemory.test.ts tests/providerSettingsSync.test.ts tests/providersPanel.test.ts
```

- [ ] **Step 6: Commit** with `git commit -m "refactor: deepen settings workflow"`.

### Task 5: Cross-seam compatibility and documentation

**Files:**
- Modify: `docs/agents/architecture.md` if final seams differ from documented architecture
- Modify: `docs/agents/domain-entities.md` only if lifecycle or metadata vocabulary changes

- [ ] **Step 1: Run complete Rust and desktop validation.**
- [ ] **Step 2: Confirm no renderer/Electron cross-imports, REST payload changes, preload relaxation, second session owner, or multi-terminal behavior.**
- [ ] **Step 3: Run `git diff --check`, `bash .claude/hooks/doc-check.sh`, and `bash .claude/hooks/worktree-check.sh`.**
- [ ] **Step 4: Commit documentation updates** only when the architecture or domain docs actually changed.

## Follow-up backlog from the application pass

These items are intentionally tracked separately so the current seam work does not lose them:

- **Rust AppState narrowing:** after Tasks 2–5, identify the remaining direct `AppState` access in `session_handlers.rs`, `session_runtime.rs`, `terminal_runtime.rs`, and `peon_runtime.rs`; introduce a narrower internal session-context interface only where it removes duplicated invariants.
- **Terminal lifecycle module:** reassess `apps/desktop/src/terminalStore.ts` after the controller work. Extract a transport/lifecycle seam only if new tests demonstrate that xterm construction, WebSocket attachment, replay fallback, input buffering, and teardown cannot remain cohesive.
- **Settings section extraction:** split `SettingsModal.tsx` into focused section files only where the controller extraction leaves a real rendering responsibility; do not perform a mechanical component split.
- **Cross-application contract audit:** after the three seams stabilize, compare Rust response types, `api.ts`, preload settings types, and Electron settings types for drift without removing intentional process-boundary duplication.

## Verification and compatibility debt discovered during implementation

These are known follow-ups, not reasons to reopen the completed seams:

- **Rust formatting baseline:** `cargo fmt --all -- --check` currently reports
  pre-existing formatting drift across untouched sidecar files. Run a scoped
  cleanup only when prepared to review the resulting broad mechanical diff.
- **Rust strict-clippy baseline:** strict clippy still reports existing
  unrelated warnings, and shared-target runs can be blocked by target-directory
  permissions. Do not attribute that baseline debt to the application seam;
  isolate and burn it down separately.
- **Live-forget wire assertion:** add an explicit HTTP test for the response
  body when forgetting a live session returns `409`, preserving the exact
  compatibility message rather than checking status only.
- **Renderer test-shape migration:** source-shape tests that asserted
  orchestration in `App.tsx` now belong at the controller seam. Keep future
  characterization tests focused on observable behavior or the owning module,
  not incidental placement.
- **Polling lifecycle regression coverage:** the controller now has separate
  foreground generations and polling epochs. Preserve both invariants when
  changing polling or workspace lifecycle code; a poll must not cancel a
  foreground operation, and a disabled loop must not publish after re-enable.
- **Settings partial-save semantics:** Task 4 must retain the renderer draft
  and represent pending/stale sidecar application when Electron persistence
  succeeds but the sidecar push fails; do not accidentally promise atomic
  cross-domain settings saves.

## Execution refinement for Rust Task 2

The Rust seam is intentionally executed as four reviewable slices rather than one large relocation:

1. Move workspace opening and reconciliation completely behind typed application results (already complete in the initial scaffold).
2. Move create/resume lifecycle workflows behind typed application results, preserving current resume startup/error behavior and generation-aware admission.
3. Move attention and plan-selection workflows behind typed application results, preserving token authorization in HTTP and metadata priority rules.
4. Move delete/forget workflows, then remove all `*_legacy` handlers and add the final handler compatibility tests.

The temporary scaffold from the initial Task 2 attempt is not an accepted endpoint: no later slice may leave an application method returning Axum `Response`, depending on HTTP request DTOs, or delegating to a `*_legacy` handler.
