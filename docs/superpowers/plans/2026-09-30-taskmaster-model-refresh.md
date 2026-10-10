---
type: "Implementation Plan"
title: "Taskmaster Model Refresh Implementation Plan"
description: "Implementation plan: Taskmaster Model Refresh Implementation Plan."
tags: ["orkworks", "plans"]
---

# Taskmaster Model Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Taskmaster Settings refresh Codex and Ollama model suggestions on explicit request while keeping Taskmaster discovery separate from Peon configuration and custom provider commands.

**Architecture:** Electron main exposes a narrow IPC operation that uses the existing Taskmaster token. The sidecar validates a code-owned built-in provider profile and calls a typed discovery operation: fixed Codex app-server discovery or Ollama at the Taskmaster draft URL. Settings keeps manual model IDs and discards results after provider or URL changes.

**Tech Stack:** Rust/Axum sidecar, `ProviderManager`, Electron IPC, React/TypeScript, Node's built-in test runner.

**Spec:** `docs/superpowers/specs/2026-09-29-taskmaster-model-refresh-status-design.md`, “Model suggestions and refresh”.

**Prerequisite:** ADR 0071 is recorded in the companion run-status plan before implementation begins.

## Global Constraints

- Model refresh is explicit and does not run when Settings opens.
- Codex uses the fixed `codex app-server --stdio` model-list operation.
- Ollama uses the Taskmaster selection's draft URL.
- Claude Code uses its built-in static catalog.
- Custom providers keep static suggestions and manual entry; Taskmaster does not execute custom model-list commands.
- Refresh performs discovery only: no inference, Apply, settings persistence, or Peon state mutation.
- A response for a provider or URL that changed while the request was pending is discarded.
- Provider suggestions do not remove the editable custom model-ID entry.

---

### Task 1: Add typed provider discovery for Taskmaster

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`
- Test: `crates/orkworksd/src/providers.rs` provider tests

**Interface:**
- Add `ProviderManager::discover_taskmaster_models(profile: NativeProfile, ollama_base_url: Option<&str>) -> Result<Vec<String>, ProviderOperationError>`.
- `NativeProfile::Codex` uses a code-owned command and arguments. `NativeProfile::Ollama` uses the supplied URL. `NativeProfile::Claude` returns `UnsupportedCapability` without starting a process or making a request.

- [ ] **Step 1: Write the failing discovery override test.** Create a fake `codex` executable that answers the app-server handshake and a separate executable that represents a mutable `models` override. Call the new Taskmaster method and assert it returns the fake Codex catalog while the override marker file is absent.

```rust
let models = manager
    .discover_taskmaster_models(NativeProfile::Codex, None)
    .expect("fixed Codex discovery succeeds");
assert_eq!(models, vec!["fixed-codex-model"]);
assert!(!override_marker.exists());
```

- [ ] **Step 2: Run the focused test and confirm it fails because the typed method is absent.**

Run from the repository root:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_discovery_ignores_models_override
```

Expected: compilation fails because `discover_taskmaster_models` is not defined.

- [ ] **Step 3: Implement the typed operation.** Reuse the existing Codex response parser and Ollama verification transport, but supply fixed Codex command data directly instead of reading `ProviderDefinition.list_models_command`.

```rust
match profile {
    NativeProfile::Codex => self.discover_codex_models_with_command("codex", &["app-server", "--stdio"]),
    NativeProfile::Ollama => self.discover_ollama_models(ollama_base_url),
    NativeProfile::Claude => Err(unsupported_taskmaster_discovery()),
}
```

- [ ] **Step 4: Run the focused test and the existing Codex model-list tests.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_discovery_ignores_models_override
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml codex_app_server_discovery
```

Expected: both pass; the mutable command marker remains absent.

### Task 2: Add the authorized sidecar discovery route

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/http/mod.rs`
- Modify: `crates/orkworksd/src/http/taskmaster_settings_handlers.rs` or add `crates/orkworksd/src/http/taskmaster_model_handlers.rs`
- Test: focused handler tests beside the handler

**Interface:**
- Add `POST /settings/taskmaster/models` with `{ "provider": "codex" | "ollama", "ollamaBaseUrl"?: string }` and response `{ "models": string[] }`.
- Require `authorize_taskmaster_request`; validate the provider with `NativeProfile::resolve` against the current harness snapshot before dispatching discovery.

- [ ] **Step 1: Add failing handler tests for missing authorization, Codex with a mutable `models` override, and rejection of Claude/custom IDs.** Assert the Codex test returns models from the code-owned command and leaves Peon state unchanged.
- [ ] **Step 2: Run the focused handler test and confirm the route is missing or rejects the request.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_models_route
```

- [ ] **Step 3: Add the typed request/response, authorization check, profile resolution, blocking-task dispatch, and router registration.** Keep the route independent of saved Peon provider settings and do not call `discover_provider_models(provider_id, ...)`.
- [ ] **Step 4: Run the focused tests and existing Taskmaster authorization tests.**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_models_route
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster_request
```

Expected: unauthorized calls fail before discovery; custom and Claude requests are rejected; Codex and Ollama use their typed paths.

### Task 3: Bridge model refresh through Electron IPC

**Files:**
- Modify: `apps/desktop/electron/taskmasterSettings.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Test: `apps/desktop/tests/taskmasterSettings.test.ts`

- [ ] **Step 1: Add a failing transport test for the `models` resource.** Assert it sends `POST http://127.0.0.1:<port>/settings/taskmaster/models`, includes `x-orkworks-open-plan-token`, and rejects oversized or invalid responses.
- [ ] **Step 2: Run the focused Node test and confirm the resource is not accepted.**

```bash
cd apps/desktop && rtk node --experimental-strip-types --test tests/taskmasterSettings.test.ts
```

- [ ] **Step 3: Add the bounded `models` resource and IPC method `refreshTaskmasterModels(provider, ollamaBaseUrl?)`.** Electron main validates the provider to `codex` or `ollama`, obtains the active sidecar readiness/authority, and calls the fixed loopback resource. The renderer receives only model IDs and cannot select a URL or token.
- [ ] **Step 4: Run the focused Node test and TypeScript check.**

```bash
cd apps/desktop && rtk node --experimental-strip-types --test tests/taskmasterSettings.test.ts && rtk pnpm exec tsc --noEmit
```

### Task 4: Add the Settings refresh control

**Files:**
- Modify: `apps/desktop/src/components/TaskmasterSettings.tsx`
- Modify: `apps/desktop/src/App.css`
- Test: `apps/desktop/tests/taskmasterSettingsComponent.test.mjs`

- [ ] **Step 1: Add failing component tests.** Verify Codex/Ollama show a refresh button, Claude/custom do not, Settings mount makes zero refresh calls, the button shows progress and errors, and the manual model value remains editable.
- [ ] **Step 2: Run the focused component test and confirm the refresh assertions fail.**

```bash
cd apps/desktop && rtk node --experimental-strip-types --test tests/taskmasterSettingsComponent.test.mjs
```

- [ ] **Step 3: Implement ephemeral refreshed-model state keyed by provider and Ollama URL.** Keep static catalogs as the baseline; display refreshed IDs after success; ignore both success and error responses if the key or request generation is stale. Add `aria-live="polite"` progress and error text. Do not persist the refreshed list or change Peon state.
- [ ] **Step 4: Run the component test, Taskmaster settings tests, and TypeScript check.**

```bash
cd apps/desktop && rtk node --experimental-strip-types --test tests/taskmasterSettingsComponent.test.mjs tests/taskmasterSettings.test.ts && rtk pnpm exec tsc --noEmit
```

Expected: model discovery occurs only after an explicit click; stale responses never replace current suggestions.

### Task 5: Verify the model-refresh slice

- [ ] Run `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- [ ] Run the focused Rust and desktop commands above.
- [ ] Run the complete Rust and desktop test suites after the slice is integrated.
- [ ] Run `rtk git diff --check` and `rtk bash scripts/doc-check.sh`.
- [ ] Confirm no settings write, inference invocation, Peon endpoint call, or Peon state change is reachable from Taskmaster refresh.
