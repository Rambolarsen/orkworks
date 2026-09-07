# Peon Codex Model and Effort Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Peon's Codex model picker follow the installed Codex CLI catalog and expose each selected model's supported reasoning effort.

**Architecture:** Extend the resolved harness capability registry with a dedicated Codex app-server model-discovery kind. The sidecar performs the short-lived JSON-RPC catalog probe and returns normalized model metadata through the existing Peon verification route; the existing Electron transaction carries that metadata into the renderer, where Codex selections persist optional `model` and `effort` values. Discovery failures leave the provider usable with Codex defaults and manual entry, while Electron retains the last successful catalog for stale-cache fallback.

**Tech Stack:** Rust, Axum, serde/serde_json, std::process JSONL subprocess I/O, Electron main/preload, React/TypeScript, Node test runner.

**Spec:** Approved chat design, [ADR 0049](../../adr/0049-codex-app-server-peon-model-discovery.md), and issue #482.

## Global Constraints

- Keep Peon observer-only; it must not type into or control the user’s coding session.
- Preserve the existing provider-first verify → Apply → Save transaction.
- Preserve backward compatibility for persisted Peon selections and provider settings.
- Keep Electron main and renderer types duplicated across the preload boundary.
- Use pnpm for desktop package commands and run Rust formatting/tests from the repository root.
- Do not add a hardcoded current Codex model catalog; the installed Codex app-server is the source of truth.

---

### Task 1: Add the Codex app-server model capability and parser

**Files:**
- Modify: `crates/orkworksd/src/harness/definition.rs`
- Modify: `crates/orkworksd/src/harness/registry.rs`
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/resources/harnesses-v2.json`
- Modify: `apps/desktop/src/harnessTypes.ts`
- Test: Rust unit tests in `crates/orkworksd/src/providers.rs` and `crates/orkworksd/src/harness/definition.rs`

**Interfaces:**
- `ModelCapability::CodexAppServer` is a declarative harness model capability with no command or model-list fields.
- `ProviderModelOption` serializes as `{ id, displayName, reasoningEfforts, defaultReasoningEffort }`.
- `ProviderReasoningEffort` serializes as `{ id, description }`.
- `ProviderDefinition` records whether model discovery uses the Codex app-server protocol.

- [ ] **Step 1: Write failing tests for the new capability and response mapping.**

  Add tests that parse a built-in Codex definition with `models.kind == "codex-app-server"`, project it into a provider definition, and parse a representative `model/list` JSON-RPC response containing `model`, `displayName`, `supportedReasoningEfforts`, and `defaultReasoningEffort`.

- [ ] **Step 2: Run the focused Rust tests and confirm they fail for the missing capability.**

  Run `cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests::codex -- --nocapture` and the relevant harness definition test filter. The failure must identify the missing enum/field or parser behavior.

- [ ] **Step 3: Implement the declarative capability and normalized model types.**

  Add the `codex-app-server` serde variant, permit it in strict Rust and TypeScript harness validation, project it through `provider_from_harness`, and change the built-in Codex model capability from the stale static list to the new kind. Map generic static/command model results into the same normalized model-option shape with no effort options.

- [ ] **Step 4: Implement the bounded JSON-RPC probe.**

  Spawn the provider command with `app-server --stdio`, send `initialize`, `initialized`, and `model/list` JSON lines, read only the matching responses through a bounded channel with the provider timeout, parse visible models, terminate/reap the child, and return a structured error on malformed, timed-out, or failed responses. Drain or discard stderr so a noisy CLI cannot deadlock the probe.

- [ ] **Step 5: Run the focused Rust tests and confirm they pass.**

  Run the same focused filters plus `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.

- [ ] **Step 6: Commit the isolated capability/parser slice.**

  Use `git add` for only the capability, resource, parser, and tests, then commit with `feat: discover Codex Peon models from app-server`.

### Task 2: Carry optional model and effort through Peon persistence and invocation

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/resources/harnesses-v2.json`
- Modify: `apps/desktop/electron/providerTypes.ts`
- Modify: `apps/desktop/src/providerTypes.ts`
- Modify: `apps/desktop/electron/settingsMemory.ts`
- Modify: `apps/desktop/electron/peonSelectionTransaction.ts`
- Test: `crates/orkworksd/src/providers.rs`, `apps/desktop/tests/electronSettingsMemory.test.ts`, `apps/desktop/tests/peonSelectionTransaction.test.ts`, and `apps/desktop/tests/peonModelPicker.test.ts`

**Interfaces:**
- `PeonSelection` has `model: string | null` and `effort: string | null`.
- `PeonProviderVerificationResponse.models` is `ProviderModelOption[]`.
- `PeonAppliedState` exposes the applied effort alongside provider/model.
- Codex's Peon capability declares `reasoningEffortArgs: ["--config", "model_reasoning_effort={effort}"]`.

- [ ] **Step 1: Write failing tests for backward-compatible normalization and argument rendering.**

  Cover legacy string-model selections, null/default selections, effort trimming, rejection of empty effort values, preservation of effort through Electron read-normalize-save, and Codex argv containing the selected model plus the config effort pair while default selections omit both.

- [ ] **Step 2: Run the focused desktop and Rust tests to verify the new assertions fail.**

  Run `node --experimental-strip-types --test tests/electronSettingsMemory.test.ts tests/peonSelectionTransaction.test.ts tests/peonModelPicker.test.ts` from `apps/desktop`, then the focused Rust provider tests.

- [ ] **Step 3: Implement optional selection normalization and applied-state matching.**

  Treat missing, null, and whitespace-only model/effort as defaults; keep Ollama’s model required; preserve provider/model/effort in the verification fingerprint and applied state; and make save confirmation compare effort as well as provider/model/URL.

- [ ] **Step 4: Implement capability-driven invocation arguments.**

  Render the optional model template only when a model is selected, render effort argument templates only when a non-empty effort is selected, and pass the resulting args through the existing process runner. Codex verification and Apply must be valid with no explicit model.

- [ ] **Step 5: Run the focused tests and confirm they pass.**

  Re-run the desktop and Rust focused commands, then run the full Rust provider test module.

- [ ] **Step 6: Commit the persistence/invocation slice.**

  Commit with `feat: support optional Peon model effort`.

### Task 3: Add stale-aware model metadata to the Electron and Settings UI

**Files:**
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/electron/providerTypes.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Modify: `apps/desktop/src/providerTypes.ts`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: relevant provider/settings CSS if needed
- Test: `apps/desktop/tests/peonModelPicker.test.ts`, `apps/desktop/tests/providersPanel.test.ts`, and settings-memory tests

**Interfaces:**
- `ProviderModelsResponse` returns normalized model options plus `stale` and `observedAt` metadata.
- `getProviderModels(providerId)` returns the process-local last successful catalog and refreshes it on explicit request or cache miss.
- Settings displays `Codex default`, model-specific effort choices, a manual model input, and a stale/unavailable status without disabling the default path.

- [ ] **Step 1: Write failing renderer/source tests for the new controls.**

  Assert that the Settings modal renders default-model and effort controls, uses the selected model’s effort list, omits unsupported efforts, shows stale discovery status, and passes effort through Apply and Save.

- [ ] **Step 2: Run the focused desktop tests and confirm they fail.**

  Run the Peon model picker and provider panel test files from `apps/desktop`.

- [ ] **Step 3: Implement the Electron metadata cache and duplicated preload contracts.**

  Store the last successful normalized Codex catalog with its observation timestamp, return stale data when refresh fails, update both main/preload and renderer declarations, and keep non-Codex string-list compatibility mapped into model options.

- [ ] **Step 4: Implement Settings model/effort behavior.**

  Initialize from the persisted selection, keep `Codex default` selectable, derive the effort options from the selected model, clear an invalid effort when the model changes, support manual model entry, mark Apply dirty when either model or effort changes, and keep Save gated on matching applied state.

- [ ] **Step 5: Run focused TypeScript checks and tests.**

  Run `npx tsc --noEmit` and the focused test files until the new controls and contract checks pass.

- [ ] **Step 6: Commit the Electron/UI slice.**

  Commit with `feat: expose Codex Peon model effort controls`.

### Task 4: Documentation, review, and full verification

**Files:**
- Modify: `docs/agents/architecture.md`
- Modify: issue #482 through the GitHub issue workflow
- Test: repository Rust and desktop validation commands

- [ ] **Step 1: Update architecture documentation.**

  Document that `providers.rs` projects Codex app-server model metadata, that Peon selections carry optional effort, and that model discovery failure falls back to the Codex default path.

- [ ] **Step 2: Run the complete verification suite.**

  Run `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`, `cargo test --manifest-path crates/orkworksd/Cargo.toml`, `npx tsc --noEmit`, `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`, and `git diff --check`.

- [ ] **Step 3: Request the required lightweight code review.**

  Review the complete branch diff at low effort, because the change touches both desktop and Rust code but does not change session lifecycle, security boundaries, or persisted metadata files outside app settings.

- [ ] **Step 4: Address review findings and re-run affected tests.**

  Fix findings that change the diff’s correctness, document intentional deviations in the issue, and repeat the relevant verification commands.

- [ ] **Step 5: Update issue #482 with implementation and verification evidence.**

  Add the changed files, test commands, and any known limitation such as Codex app-server availability on older installed CLI versions.
