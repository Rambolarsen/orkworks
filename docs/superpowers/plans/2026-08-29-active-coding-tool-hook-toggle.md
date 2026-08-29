# Active Coding Tool Hook Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make each coding-tool toggle own OrkWorks integration setup, status, retry, and user-action feedback while preserving ownership-safe hook mutations and subsection-local Settings actions.

**Architecture:** Keep the sidecar as the authority for workspace active-tool persistence and integration mutations, but add one Electron-main orchestration seam for the Tools subsection. The renderer loads status for every resolved harness capability, derives a per-tool visual state, and receives structured per-tool outcomes. Remove the global Settings footer; keep command-path editing as a separate per-tool control.

**Tech Stack:** Electron main/preload IPC, React/TypeScript, Axum/Rust sidecar, existing integration handlers and `IntegrationStatus`, Node’s built-in test runner, Rust unit tests, CSS custom properties.

## Global Constraints

- Use `pnpm` for Node package-management tasks.
- Preserve the Electron-main/renderer boundary; renderer code never gains filesystem or hook-mutation authority.
- Use the existing `--attention-needs-you` token for conditions requiring user action; do not duplicate its color value.
- Preserve foreign and unrelated coding-tool configuration; remove only OrkWorks-owned entries.
- Unsupported integrations are skipped without claiming hook health; limited integrations, including Aider, remain supported with limited-coverage messaging.
- A hook mutation failure preserves the requested active-tool state and produces a needs-you result; active-tool persistence failure prevents hook mutations.
- Run `bash scripts/doc-check.sh` and `bash .claude/hooks/worktree-check.sh` before handoff.

---

## File Map

- Modify `apps/desktop/electron/main.ts`: add the privileged Tools Save orchestration IPC handler and typed result mapping.
- Modify `apps/desktop/electron/preload.ts`: expose the narrow orchestration method.
- Modify `apps/desktop/src/orkworksWindow.d.ts`: mirror the preload contract.
- Modify `apps/desktop/src/api.ts`: add typed sidecar helpers for active-tool persistence and integration status/mutations used by the main-process orchestration seam.
- Modify `apps/desktop/src/App.tsx`: return structured active-tool save results instead of swallowing failures; refresh active tools after successful persistence.
- Modify `apps/desktop/src/components/SettingsModal.tsx`: own all subsection drafts/actions, load per-tool statuses, remove the global footer, and remove the integration section mount.
- Modify `apps/desktop/src/components/Toggle.tsx`: support an explicit visual state, status description, tooltip, and non-color glyph while preserving `role="switch"`.
- Create `apps/desktop/src/components/HarnessCommandPathControl.tsx`: preserve custom command-path Save/Clear behavior after hook controls are removed.
- Create or modify `apps/desktop/src/harnessIntegrationPresentation.ts`: derive capability-aware display state, warning precedence, and accessible copy.
- Modify `apps/desktop/src/App.css`: add needs-you, healthy, neutral-spinner, and status-glyph styles using tokens in both themes.
- Modify `apps/desktop/tests/providersPanel.test.ts`, `apps/desktop/tests/harnessIntegrationSection.test.ts`, and add focused presentation/controller tests for result mapping and state derivation.
- Modify `apps/desktop/tests/api.test.ts` or add an IPC contract test for the new typed operation.
- Modify `crates/orkworksd/src/http/integration_handlers.rs` and related sidecar application code only if workspace generation/active-state validation must be enforced server-side.
- Modify `crates/orkworksd/src/harness/registry.rs` tests if capability-derived participation or status enabled-state behavior changes.
- Update `docs/agents/architecture.md` if the new main-process orchestration seam becomes a durable architecture contract.

## Task 1: Establish the typed Save and status-state contracts

**Files:**
- Modify: `apps/desktop/src/harnessIntegrationPresentation.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Test: `apps/desktop/tests/harnessIntegrationSection.test.ts`
- Test: `apps/desktop/tests/providersPanel.test.ts`

**Interfaces:**
- Produce `IntegrationDisplayState` with `appearance: "off" | "neutral" | "healthy" | "needs-you" | "error" | "in-progress"`, `label`, `description`, `tooltip`, and `glyph`.
- Produce `ActiveHarnessSaveResult` with active persistence outcome and per-harness operation/outcome/registration/activation/coverage/diagnostic fields as defined in the spec.
- Expose `saveActiveHarnessesWithIntegrations(ids: string[]): Promise<ActiveHarnessSaveResult>` through `window.orkworks`.

- [ ] **Step 1: Write failing state-derivation tests.** Cover healthy installed, enabled/absent, needs trust, disabled/owned, unsupported, limited Aider, status unavailable, operation failure, and neutral in-progress states. Assert that needs-you uses the literal semantic state `needs-you`, not an amber warning state.

- [ ] **Step 2: Run the focused tests and verify failure.**

  Run from `apps/desktop/`:

  ```bash
  node --experimental-strip-types --test tests/harnessIntegrationSection.test.ts tests/providersPanel.test.ts
  ```

  Expected: FAIL because the display-state helper and combined IPC type do not yet exist.

- [ ] **Step 3: Implement the pure presentation contract.** Add a single capability-aware mapping that applies this precedence: operation failure, current diagnostic, trust/ownership/registration condition, then healthy/unsupported. Use `--attention-needs-you` only through the presentation state; do not embed hex colors in TypeScript.

- [ ] **Step 4: Add the preload/window declarations.** Add the method to `preload.ts` using `ipcRenderer.invoke`, and mirror the exact return type in `orkworksWindow.d.ts`. Keep the existing direct status/path methods until later tasks remove their consumers.

- [ ] **Step 5: Run focused tests and type-check.**

  ```bash
  node --experimental-strip-types --test tests/harnessIntegrationSection.test.ts tests/providersPanel.test.ts
  npx tsc --noEmit
  ```

  Expected: the new presentation tests pass; unrelated UI tests may still fail until Task 4 removes the old section.

- [ ] **Step 6: Commit the contract seam.**

  ```bash
  git add apps/desktop/src/harnessIntegrationPresentation.ts apps/desktop/src/orkworksWindow.d.ts apps/desktop/electron/preload.ts apps/desktop/tests/harnessIntegrationSection.test.ts apps/desktop/tests/providersPanel.test.ts
  git commit -m "feat: define coding tool integration toggle states"
  ```

## Task 2: Implement main-process orchestration with workspace safety

**Files:**
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/tests/api.test.ts`
- Test: `apps/desktop/tests/backendRestoration.test.ts` or a new `apps/desktop/tests/activeHarnessSave.test.ts`
- Modify: `crates/orkworksd/src/http/integration_handlers.rs` only if the backend must add generation validation.

**Interfaces:**
- `saveActiveHarnessesWithIntegrations(ids: string[]): Promise<ActiveHarnessSaveResult>` is the only renderer-facing combined operation.
- The main-process operation first calls `PUT /workspace/active-harnesses`; on failure it returns `activeHarnesses.outcome = "failed"` and performs no integration mutation.
- On success it obtains current status/capability for all harnesses, installs/repairs enabled capable integrations, uninstalls disabled owned integrations, skips unsupported tools, and returns one per-tool result.

- [ ] **Step 1: Write orchestration tests.** Use injected fake sidecar/IPC dependencies to assert: active persistence failure prevents all mutations; multiple tools return independent outcomes; enabled/absent chooses install; installed/drifted chooses repair/install; disabled owned chooses uninstall; unsupported chooses skipped; Codex `needs_trust` remains non-healthy; one failure does not erase another tool’s success.

- [ ] **Step 2: Add workspace-generation guards.** Capture workspace path and sidecar lifecycle generation before the active write. Check them before each mutation and after the batch. Abort remaining work with `stale_workspace`, never report success for the old workspace, and ignore late results after a workspace switch.

- [ ] **Step 3: Implement the operation in Electron main.** Reuse the existing privileged integration status/install/uninstall routes through main-process helpers. Preserve the current confirmation/ownership behavior. Map expected per-tool failures into structured results; reserve thrown/rejected errors for invalid IPC input or unavailable backend infrastructure.

- [ ] **Step 4: Replace the swallowing App callback.** Make `handleSaveActiveHarnesses` return the structured result and update `activeHarnessIds` only when active persistence reports `persisted`. Do not emit a misleading generic “Saved” toast for partial integration failures.

- [ ] **Step 5: Run focused tests.**

  ```bash
  node --experimental-strip-types --test tests/activeHarnessSave.test.ts tests/api.test.ts
  npx tsc --noEmit
  ```

  Expected: PASS for persistence ordering, partial results, stale workspace, and typed IPC coverage.

- [ ] **Step 6: Commit.**

  ```bash
  git add apps/desktop/electron/main.ts apps/desktop/electron/preload.ts apps/desktop/src/api.ts apps/desktop/src/App.tsx apps/desktop/src/orkworksWindow.d.ts apps/desktop/tests/activeHarnessSave.test.ts apps/desktop/tests/api.test.ts crates/orkworksd/src/http/integration_handlers.rs
  git commit -m "feat: orchestrate coding tool integration saves"
  ```

## Task 3: Make integration participation capability-derived

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/harnessTypes.ts` if a typed integration capability projection is needed.
- Modify: `apps/desktop/src/newSessionDialogState.ts` only if retired-tool filtering is exposed to Settings.
- Test: `apps/desktop/tests/providersPanel.test.ts`
- Test: `apps/desktop/tests/newSessionDialogState.test.ts`

**Interfaces:**
- Settings loads integration status for every selectable harness whose resolved definition has an integration capability; no `INTEGRATION_HARNESS_IDS` allowlist remains.
- Aider participates through its limited integration status; unsupported/no-binding tools are represented as skipped/neutral.

- [ ] **Step 1: Write failing tests.** Assert that Settings does not use `INTEGRATION_HARNESS_IDS`, that Aider is not accidentally excluded, and that retired Gemini follows existing selectable-harness rules rather than a new hard-coded exception.

- [ ] **Step 2: Implement capability-derived status loading.** Load status independently of whether the tool is currently enabled. Keep detection status independent. Store a per-harness status map with cancellation on modal close and a generation bump after successful saves/path changes.

- [ ] **Step 3: Map status to display state.** Implement explicit enabled/absent, disabled/owned, limited, unsupported, status-unavailable, and Codex trust-pending outcomes. Green means installed/applied with no action-required diagnostic; it does not require a verified runtime execution for integrations whose activation is `unknown` by contract.

- [ ] **Step 4: Run focused tests.**

  ```bash
  node --experimental-strip-types --test tests/providersPanel.test.ts tests/newSessionDialogState.test.ts
  ```

  Expected: PASS with capability-derived participation and no stale allowlist.

- [ ] **Step 5: Commit.**

  ```bash
  git add apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/harnessTypes.ts apps/desktop/src/newSessionDialogState.ts apps/desktop/tests/providersPanel.test.ts apps/desktop/tests/newSessionDialogState.test.ts
  git commit -m "refactor: derive integration settings from capabilities"
  ```

## Task 4: Implement the colored toggle and remove the global Settings footer

**Files:**
- Modify: `apps/desktop/src/components/Toggle.tsx`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/App.css`
- Modify: `apps/desktop/src/styles/tokens.css` only if a missing semantic token is required.
- Modify: `apps/desktop/tests/providersPanel.test.ts`
- Add: `apps/desktop/tests/toggle.test.ts` if the existing test style supports pure source/DOM checks.

**Interfaces:**
- `Toggle` accepts `visualState`, `statusDescription`, `tooltip`, and `statusGlyph` while retaining `checked`, `onChange`, `disabled`, and `role="switch"`.
- The stable accessible name remains the tool name; `aria-describedby` points to visible status text. Native `title` mirrors the tooltip.

- [ ] **Step 1: Write failing UI/source tests.** Assert colored classes use semantic states, `--attention-needs-you` is used in CSS, status glyph/text is present, the Tools toggle preserves its draft on/off position during in-progress work, and the modal no longer renders the global Save/Cancel/Restore footer or `saveError`.

- [ ] **Step 2: Implement Toggle state props.** Render the stable switch label, a visible non-color glyph/status description for warning/error/in-progress states, `aria-describedby`, and `title`. Disable the switch during integration operations while leaving `aria-checked` tied to the draft.

- [ ] **Step 3: Add CSS.** Add semantic toggle modifier classes. Needs-you states must use `var(--attention-needs-you)` in both themes; healthy uses `var(--state-ok)`; status-query failure uses `var(--state-error)`; in-progress uses neutral track plus spinner. Preserve visible focus styles and contrast.

- [ ] **Step 4: Remove the global footer.** Delete the modal-level Save/Cancel/Restore controls and their global `saveError`/`saving` path. Keep the title-bar close action as discard-all-close, and move Hotkeys Restore defaults plus subsection Save/Cancel into the Hotkeys section.

- [ ] **Step 5: Run focused tests.**

  ```bash
  node --experimental-strip-types --test tests/toggle.test.ts tests/providersPanel.test.ts
  npx tsc --noEmit
  ```

  Expected: PASS for semantic colors, accessibility attributes, and subsection-only actions.

- [ ] **Step 6: Commit.**

  ```bash
  git add apps/desktop/src/components/Toggle.tsx apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/App.css apps/desktop/src/styles/tokens.css apps/desktop/tests/providersPanel.test.ts apps/desktop/tests/toggle.test.ts
  git commit -m "feat: scope settings actions and color tool toggles"
  ```

## Task 5: Preserve custom command-path editing as a separate control

**Files:**
- Add: `apps/desktop/src/components/HarnessCommandPathControl.tsx`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/components/HarnessIntegrationSection.tsx` only if extracting shared path logic before deleting the hook section.
- Modify: `apps/desktop/src/App.css`
- Test: `apps/desktop/tests/providersPanel.test.ts`
- Add: `apps/desktop/tests/harnessCommandPathControl.test.ts`

**Interfaces:**
- `HarnessCommandPathControl({ harnessId, harnessName, harness, disabled, onChanged })` preserves immediate Save/Clear behavior and reports path errors beside the path controls.

- [ ] **Step 1: Write failing tests.** Cover absolute-path validation, Save/Clear IPC calls, path errors, disabled state during integration operation, and status refresh after a successful path change.

- [ ] **Step 2: Extract the existing path logic.** Move `looksAbsolute`, draft state, Save/Clear handlers, and custom-path explanation from `HarnessIntegrationSection` into the new component without changing IPC methods.

- [ ] **Step 3: Mount the path control for command-template tools.** Do not gate it on hook capability; do not render it for platform-shell tools. Pass the integration-operation busy state so path changes cannot race reconciliation.

- [ ] **Step 4: Delete hook-section rendering and preserve only the path component.** Remove the old install/uninstall/reinstall UI and its status text. Keep status loading in Settings’ per-harness map.

- [ ] **Step 5: Run focused tests and commit.**

  ```bash
  node --experimental-strip-types --test tests/harnessCommandPathControl.test.ts tests/providersPanel.test.ts
  git add apps/desktop/src/components/HarnessCommandPathControl.tsx apps/desktop/src/components/HarnessIntegrationSection.tsx apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/App.css apps/desktop/tests/harnessCommandPathControl.test.ts apps/desktop/tests/providersPanel.test.ts
  git commit -m "refactor: preserve command path settings separately"
  ```

## Task 6: Finish subsection-local Save/revert behavior and status lifecycle

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/settingsController.ts`: retain only the subsection controller operations needed after the global commit path is removed.
- Modify: `apps/desktop/tests/settingsController.test.ts`
- Modify: `apps/desktop/tests/providersPanel.test.ts`

**Interfaces:**
- Tools Save keeps the modal open on partial integration failures, updates only successful per-tool warning state, and retries all applicable reconciliation on the next Tools Save.
- Hotkeys Save/Cancel/Restore are local to Hotkeys; Providers retain Apply/Save; Retention/Debug remain field-level/immediate.

- [ ] **Step 1: Write failing lifecycle tests.** Cover title-bar close discarding all unsaved drafts, Hotkeys subsection revert, Tools partial success with one warning retained, operation failure taking precedence over status diagnostics, modal reopen reloading status diagnostics, and successful one-tool reconciliation not clearing another tool’s warning.

- [ ] **Step 2: Implement per-subsection state.** Remove the global `createSettingsController.commit()` path from modal Save handling. Keep only the controller methods needed by Providers, Hotkeys, Retention, and Debug, or split them into focused helpers without changing persisted settings formats.

- [ ] **Step 3: Implement warning precedence and refresh generations.** Store operation errors per harness, overlay them over status diagnostics, clear only on successful healthy reconciliation, and invalidate late status/mutation responses after close or workspace switch.

- [ ] **Step 4: Run the full desktop test suite.**

  ```bash
  node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
  npx tsc --noEmit
  ```

  Expected: PASS with no global Settings footer assumptions left in source tests.

- [ ] **Step 5: Commit.**

  ```bash
  git add apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/settingsController.ts apps/desktop/tests/settingsController.test.ts apps/desktop/tests/providersPanel.test.ts
  git commit -m "refactor: make settings subsection lifecycles explicit"
  ```

## Task 7: Sidecar correctness, documentation, and final verification

**Files:**
- Modify: `crates/orkworksd/src/http/integration_handlers.rs` and/or `crates/orkworksd/src/session_application.rs`: add the active-state/generation validation required by the orchestration contract.
- Modify: Rust integration/registry tests covering Aider, ownership ambiguity, foreign entries, and disabled cleanup.
- Modify: `docs/agents/architecture.md` if the orchestration contract is now part of the durable architecture description.
- Modify: `docs/agents/domain-entities.md` only if session metadata/status vocabulary changes (expected: no).

- [ ] **Step 1: Add or update sidecar tests.** Prove foreign entries survive, ambiguous ownership is never removed, Aider remains limited, unsupported tools are skipped, and status enabled/disabled values are not hardcoded incorrectly.

- [ ] **Step 2: Run Rust validation.**

  ```bash
  cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
  cargo test --manifest-path crates/orkworksd/Cargo.toml
  cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings
  ```

  Expected: PASS with all integration ownership tests green.

- [ ] **Step 3: Update architecture documentation.** Document the Tools-subsection Electron-main orchestration boundary, structured partial result, stale-workspace rejection, and subsection-local Settings actions.

- [ ] **Step 4: Run final verification.**

  ```bash
  cd apps/desktop
  npx tsc --noEmit
  node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
  cd ../..
  bash scripts/doc-check.sh
  bash .claude/hooks/worktree-check.sh
  git diff --check
  git status --short --branch
  ```

  Expected: type-check, desktop tests, Rust tests, documentation check, and diff check pass; only intentional branch changes remain.

- [ ] **Step 5: Request code review before merge.** Use the repo’s manual `/code-review` gate because this work changes `apps/desktop/` and potentially `crates/orkworksd/`.

## Self-review checklist

- Spec coverage: Tasks 1–2 cover contracts, partial failure, stale workspace, and Codex trust; Task 3 covers capability-derived participation and Aider; Tasks 4–5 cover colors, accessibility, footer removal, and command paths; Task 6 covers subsection lifecycle and warning precedence; Task 7 covers sidecar ownership and documentation.
- Placeholder scan: no `TBD`, `TODO`, “implement later,” or unbounded “handle edge cases” steps remain.
- Type consistency: `ActiveHarnessSaveResult` is introduced in Task 1 and consumed by Tasks 2, 4, and 6; `IntegrationDisplayState` is introduced in Task 1 and consumed by Tasks 3–5.
- Scope: the plan does not add new hook contracts, providers, or arbitrary configuration ownership; it composes the existing integration handlers behind one Settings operation.
