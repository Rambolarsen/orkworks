# Peon Model Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add per-provider peon model selection dropdowns to the Settings modal, populated from a new sidecar endpoint that queries each provider's CLI for available models.

**Architecture:** New `GET /providers/:id/models` sidecar endpoint → Electron caches model lists at startup → Renderer fetches from cache when Settings opens → SettingsModal renders per-provider model `<select>` dropdowns → auto-save on change via existing `save-provider-settings` IPC.

**Tech Stack:** Rust (axum sidecar), Electron main process, React/TypeScript renderer

---

### Task 1: Sidecar — add model-listing fields to ProviderDefinition

**Files:**
- Modify: `crates/orkworksd/src/providers.rs:102-111`

- [ ] **Step 1: Add `list_models_command` and `list_models_args` fields to ProviderDefinition**

```rust
// In providers.rs, modify the ProviderDefinition struct (lines 102-111):
pub struct ProviderDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub command: &'static str,
    pub default_args: &'static [&'static str],
    pub model_arg_template: Option<&'static str>,
    pub supports_model: bool,
    pub timeout_secs: u64,
    pub list_models_command: Option<&'static str>,
    pub list_models_args: &'static [&'static str],
}
```

- [ ] **Step 2: Populate new fields in `builtin_provider_registry()`**

```rust
// In providers.rs, modify the builtin_provider_registry function (lines 113-134):
pub fn builtin_provider_registry() -> Vec<ProviderDefinition> {
    vec![
        ProviderDefinition {
            id: "opencode",
            label: "OpenCode",
            command: "opencode",
            default_args: &["run", "--pure"],
            model_arg_template: Some("--model={model}"),
            supports_model: true,
            timeout_secs: 30,
            list_models_command: Some("opencode"),
            list_models_args: &["list-models"],
        },
        ProviderDefinition {
            id: "claude-code",
            label: "Claude Code",
            command: "claude",
            default_args: &["-p"],
            model_arg_template: Some("--model={model}"),
            supports_model: true,
            timeout_secs: 30,
            list_models_command: Some("claude"),
            list_models_args: &["models", "--list"],
        },
    ]
}
```

- [ ] **Step 3: Build and verify compilation**

```bash
cargo build --manifest-path crates/orkworksd/Cargo.toml 2>&1
```
Expected: successful compile (may have "unused" warnings for new fields — resolved in Task 2).

- [ ] **Step 4: Commit**

```bash
git add crates/orkworksd/src/providers.rs
git commit -m "feat(sidecar): add list_models_command/args to ProviderDefinition"
```

---

### Task 2: Sidecar — add list_models method to ProviderManager

**Files:**
- Modify: `crates/orkworksd/src/providers.rs` (add method to impl block)

- [ ] **Step 1: Add `list_models` method to `impl ProviderManager`**

Insert after the `get_providers_response` method (after line 335 in providers.rs):

```rust
    pub fn list_models(&self, provider_id: &str) -> Result<Vec<String>, String> {
        let definition = self.registry.iter()
            .find(|d| d.id == provider_id)
            .ok_or_else(|| format!("unknown provider: {provider_id}"))?;

        let (command, args) = match (definition.list_models_command, definition.list_models_args) {
            (Some(cmd), args) if !args.is_empty() => (cmd, args),
            _ => return Ok(Vec::new()),
        };

        let output = std::process::Command::new(command)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|e| format!("failed to run {command}: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                format!("{command} exited with status {}", output.status)
            } else {
                stderr
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        let models: Vec<String> = if trimmed.starts_with('[') {
            serde_json::from_str::<Vec<String>>(trimmed)
                .map_err(|e| format!("failed to parse JSON model list: {e}"))?
        } else {
            trimmed.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
        };

        Ok(models)
    }
```

- [ ] **Step 2: Build and verify**

```bash
cargo build --manifest-path crates/orkworksd/Cargo.toml 2>&1
```
Expected: successful compile.

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/providers.rs
git commit -m "feat(sidecar): add list_models method to ProviderManager"
```

---

### Task 3: Sidecar — add GET /providers/:id/models route + handler

**Files:**
- Modify: `crates/orkworksd/src/main.rs` (route table near line 216, handler near line 933)

- [ ] **Step 1: Add the route to the router**

In `main.rs`, add after the `/providers` route (line 218):

```rust
        .route("/providers/:id/models", get(get_provider_models))
```

- [ ] **Step 2: Add the handler function**

Insert after the `set_provider_settings` handler (after line 943):

```rust
#[derive(Serialize)]
struct ProviderModelsResponse {
    models: Vec<String>,
}

async fn get_provider_models(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> impl IntoResponse {
    match state.providers.list_models(&provider_id) {
        Ok(models) => axum::Json(ProviderModelsResponse { models }).into_response(),
        Err(msg) => {
            if msg.starts_with("unknown provider") {
                (axum::http::StatusCode::NOT_FOUND, msg).into_response()
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
            }
        }
    }
}
```

- [ ] **Step 3: Verify the import for Path is present**

Check that `use axum::extract::Path;` is in the imports at the top of `main.rs`. If not, add it alongside the other `use` statements.

- [ ] **Step 4: Verify the import for Serialize**

Check that `use serde::Serialize;` is in the imports. If not, add it. (It likely already is.)

- [ ] **Step 5: Build and verify**

```bash
cargo build --manifest-path crates/orkworksd/Cargo.toml 2>&1
```
Expected: successful compile, no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "feat(sidecar): add GET /providers/:id/models endpoint"
```

---

### Task 4: Sidecar — add tests for list_models

**Files:**
- Modify: `crates/orkworksd/src/providers.rs` (test module at bottom)

- [ ] **Step 1: Add test for unknown provider**

Add at the end of the `mod tests` block (before the closing `}`):

```rust
    #[test]
    fn list_models_returns_empty_for_provider_without_list_command() {
        let manager = ProviderManager::for_tests(
            sample_settings(vec![
                entry("opencode"),
            ]),
            vec![],
        );

        // Remove list_models_command from the registry entry
        // We test via the built-in registry which has list_models_command set.
        // Instead, test that a provider without list_models_command returns empty.
        let result = manager.list_models("opencode");
        // With list_models_command set, this tries to run the real command.
        // In a pure unit test we can't test without mocking, but we can test
        // the unknown-provider path.
        let err = manager.list_models("nonexistent").unwrap_err();
        assert!(err.contains("unknown provider"));
    }
```

Actually, proper unit testing of `list_models` requires a test-only constructor that accepts a custom registry. Let's add a `for_tests_with_registry` constructor:

```rust
#[cfg(test)]
impl ProviderManager {
    pub fn for_tests_with_registry(
        registry: Vec<ProviderDefinition>,
        settings: ProviderSettingsPayload,
        fakes: Vec<FakeProvider>,
    ) -> Self {
        let specs: HashMap<String, FakeProvider> =
            fakes.into_iter().map(|f| (f.id.to_string(), f)).collect();
        Self {
            registry,
            settings: Arc::new(RwLock::new(settings)),
            applied_revision: Arc::new(RwLock::new(None)),
            runtime: Arc::new(RwLock::new(HashMap::new())),
            runner: Arc::new(FakeRunner { specs }),
        }
    }
}
```

Then add these tests at the end of the `mod tests` block:

```rust
    #[test]
    fn list_models_returns_empty_when_no_list_command_configured() {
        let manager = ProviderManager::for_tests_with_registry(
            vec![ProviderDefinition {
                id: "test-provider",
                label: "Test",
                command: "test",
                default_args: &[],
                model_arg_template: None,
                supports_model: false,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: &[],
            }],
            sample_settings(vec![entry("test-provider")]),
            vec![],
        );

        let result = manager.list_models("test-provider").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_models_returns_error_for_unknown_provider() {
        let manager = ProviderManager::for_tests(
            sample_settings(vec![]),
            vec![],
        );

        let err = manager.list_models("nonexistent").unwrap_err();
        assert!(err.contains("unknown provider"));
    }
```

- [ ] **Step 2: Run Rust tests**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1
```
Expected: all tests pass including new ones.

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/providers.rs
git commit -m "test(sidecar): add list_models unit tests"
```

---

### Task 5: Renderer — add ProviderModelsResponse type + window declaration

**Files:**
- Modify: `apps/desktop/src/providerTypes.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`

- [ ] **Step 1: Add type to providerTypes.ts**

At the end of `apps/desktop/src/providerTypes.ts`, add:

```typescript
export interface ProviderModelsResponse {
  models: string[];
}
```

- [ ] **Step 2: Add method declaration to orkworksWindow.d.ts**

In `apps/desktop/src/orkworksWindow.d.ts`, add the import and method:

```typescript
import type { ProviderSettings, ProviderModelsResponse } from "./providerTypes";
```

And inside the `orkworks` object, add:

```typescript
      getProviderModels: (providerId: string) => Promise<ProviderModelsResponse>;
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1
```
Expected: may show an error on preload.ts since the IPC handler isn't wired yet. That's expected — resolved in Task 7.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/providerTypes.ts apps/desktop/src/orkworksWindow.d.ts
git commit -m "feat(renderer): add ProviderModelsResponse type and window method"
```

---

### Task 6: Electron — add provider models cache + IPC handler

**Files:**
- Modify: `apps/desktop/electron/main.ts`

- [ ] **Step 1: Add models cache variable and population function**

In `apps/desktop/electron/main.ts`, add after `let currentSettings: AppSettings | null = null;` (line 24):

```typescript
let providerModels: Map<string, string[]> = new Map();

async function refreshProviderModels(): Promise<void> {
  const port = await portPromise;
  const registry = ["opencode", "claude-code"];
  const next = new Map<string, string[]>();
  for (const id of registry) {
    try {
      const resp = await fetch(`http://127.0.0.1:${port}/providers/${id}/models`);
      if (resp.ok) {
        const data = await resp.json() as { models: string[] };
        next.set(id, data.models);
      } else {
        next.set(id, []);
      }
    } catch {
      next.set(id, []);
    }
  }
  providerModels = next;
}
```

- [ ] **Step 2: Call refreshProviderModels on startup**

After `syncSavedProviderSettings()` in the startup sequence (after the `portPromise.then(...)` block around line 339), add a call to `refreshProviderModels()`. Find the end of that `.then()` block and add:

```typescript
    await refreshProviderModels();
```

It should go right after the existing `await syncSavedProviderSettings();` call.

- [ ] **Step 3: Add IPC handler for get-provider-models**

Add after the `save-provider-settings` handler (after line 232):

```typescript
  ipcMain.handle("get-provider-models", async (_event, providerId: string) => {
    return { models: providerModels.get(providerId) ?? [] };
  });
```

- [ ] **Step 4: Verify TypeScript compilation**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1
```
Expected: no errors (may still show preload.ts error if not yet wired).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/electron/main.ts
git commit -m "feat(electron): add provider models cache and get-provider-models IPC handler"
```

---

### Task 7: Electron — expose getProviderModels in preload bridge

**Files:**
- Modify: `apps/desktop/electron/preload.ts`

- [ ] **Step 1: Add getProviderModels to the exposed API**

In `apps/desktop/electron/preload.ts`, add inside the `contextBridge.exposeInMainWorld("orkworks", {` object (after `saveProviderSettings` on line 12):

```typescript
  getProviderModels: (providerId: string): Promise<unknown> => ipcRenderer.invoke("get-provider-models", providerId),
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/electron/preload.ts
git commit -m "feat(electron): expose getProviderModels via preload bridge"
```

---

### Task 8: SettingsModal — add Providers section with model dropdowns

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx`

- [ ] **Step 1: Add imports and state for provider models**

In `SettingsModal.tsx`, extend the imports:

```typescript
import { useEffect, useState } from "react";
import { acceleratorFromKeyboardEvent } from "../hotkeyCapture";
import type { AppSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "../appSettingsTypes";
import type { ProviderSettings, ProviderSettingsEntry, ProviderModelsResponse } from "../providerTypes";
```

Add state variables after the existing `useState` declarations (after line 31):

```typescript
  const [providerDraft, setProviderDraft] = useState<ProviderSettings>(initialSettings.providers);
  const [providerModels, setProviderModels] = useState<Record<string, string[]>>({});
  const [providerSaveStatus, setProviderSaveStatus] = useState<string | null>(null);
```

- [ ] **Step 2: Add model fetching effect**

Add a `useEffect` to fetch models on mount (after the existing `useEffect` for `capturing` block):

```typescript
  useEffect(() => {
    const ids = providerDraft.providers.map((p) => p.id);
    async function load() {
      const map: Record<string, string[]> = {};
      for (const id of ids) {
        try {
          const resp: ProviderModelsResponse = await window.orkworks.getProviderModels(id);
          map[id] = resp.models;
        } catch {
          map[id] = [];
        }
      }
      setProviderModels(map);
    }
    load();
  }, []);
```

- [ ] **Step 3: Add saveProviderSettings handler**

Add after the `save` function (before the `return` statement):

```typescript
  async function saveProviderDraft(entry: ProviderSettingsEntry) {
    setProviderSaveStatus(null);
    const next = {
      ...providerDraft,
      providers: providerDraft.providers.map((p) =>
        p.id === entry.id ? entry : p,
      ),
    };
    setProviderDraft(next);
    try {
      await window.orkworks.saveProviderSettings(next);
      setProviderSaveStatus("Saved");
    } catch {
      setProviderSaveStatus("Couldn't save provider settings.");
    }
  }
```

- [ ] **Step 4: Add Providers section to the JSX**

Insert the Providers section above the Hotkeys section (before `<div className="settings-section">` with `<h3>Hotkeys</h3>` at line 111):

```tsx
        <div className="settings-section">
          <h3>Providers</h3>
          <p className="settings-section-copy">
            Choose which model peon uses for each provider. Changes apply immediately.
          </p>

          <div className="provider-list">
            {[...providerDraft.providers]
              .sort((a, b) => a.fallbackOrder - b.fallbackOrder)
              .map((entry) => (
                <div className="provider-card" key={entry.id}>
                  <div className="provider-label">{entry.id === "opencode" ? "OpenCode" : entry.id === "claude-code" ? "Claude Code" : entry.id}</div>
                  <select
                    className="provider-model-select"
                    value={entry.peonModel ?? ""}
                    onChange={(e) => {
                      const val = e.target.value;
                      saveProviderDraft({ ...entry, peonModel: val || null });
                    }}
                  >
                    <option value="">default</option>
                    {(providerModels[entry.id] ?? []).map((m) => (
                      <option key={m} value={m}>{m}</option>
                    ))}
                  </select>
                </div>
              ))}
          </div>

          {providerSaveStatus && (
            <div className={`retention-status ${providerSaveStatus === "Saved" ? "retention-status--ok" : ""}`}>
              {providerSaveStatus}
            </div>
          )}
        </div>
```

- [ ] **Step 5: Verify TypeScript compilation**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/components/SettingsModal.tsx
git commit -m "feat(ui): add Providers section with peon model dropdowns to Settings modal"
```

---

### Task 9: Update existing tests for SettingsModal change

**Files:**
- Modify: `apps/desktop/tests/providersPanel.test.ts`
- Modify: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Remove the negative-assertion test from providersPanel.test.ts**

The test "SettingsModal does not render an app-wide Providers section" (lines 59-65) is now invalid since SettingsModal DOES render a Providers section. Replace it with a test that verifies the Providers section IS present:

```typescript
test("SettingsModal renders a Providers section", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Providers/);
  assert.match(source, /providerDraft/);
  assert.match(source, /provider-model-select/);
  assert.match(source, /getProviderModels/);
});
```

- [ ] **Step 2: Remove the negative-assertion test from dockview.test.ts**

The test "SettingsModal does not include an app-wide Providers section" (lines 344-350) is now invalid. Replace it with a test that verifies the Providers section IS present:

```typescript
test("SettingsModal includes a Providers section above Hotkeys", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Providers/);
  assert.match(source, /providerDraft/);
  assert.match(source, /provider-model-select/);
  assert.match(source, /getProviderModels/);
});
```

- [ ] **Step 3: Run existing frontend tests**

```bash
cd apps/desktop && node --experimental-strip-types --test tests/providersPanel.test.ts tests/dockview.test.ts 2>&1
```
Expected: all tests pass (old failing test now passes with new assertion).

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/tests/providersPanel.test.ts apps/desktop/tests/dockview.test.ts
git commit -m "test: update SettingsModal tests for Providers section"
```

---

### Task 10: New frontend test for peon model dropdown behavior

**Files:**
- Create: `apps/desktop/tests/peonModelPicker.test.ts`

- [ ] **Step 1: Write the test file**

```typescript
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import type { ProviderSettings } from "../src/providerTypes.ts";

function sampleProviderSettings(): ProviderSettings {
  return {
    version: 1,
    revision: 1,
    providers: [
      { id: "opencode", enabled: true, fallbackOrder: 0, peonModel: null, defaultState: "healthy", overrideState: null },
      { id: "claude-code", enabled: true, fallbackOrder: 1, peonModel: "sonnet", defaultState: "unknown", overrideState: null },
    ],
  };
}

test("ProviderSettingsEntry peonModel is nullable string", () => {
  const entry = { id: "opencode", enabled: true, fallbackOrder: 0, peonModel: null, defaultState: "healthy", overrideState: null };
  assert.equal(entry.peonModel, null);
});

test("ProviderSettings peonModel can be set to a model string", () => {
  const entry = { id: "opencode", enabled: true, fallbackOrder: 0, peonModel: "claude-sonnet-4-20250514", defaultState: "healthy", overrideState: null };
  assert.equal(entry.peonModel, "claude-sonnet-4-20250514");
});

test("SettingsModal has a <select> for each provider model", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /<select/);
  assert.match(source, /provider-model-select/);
  assert.match(source, /value=\{entry\.peonModel/);
  assert.match(source, /saveProviderDraft/);
});

test("SettingsModal renders sorted by fallbackOrder", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /fallbackOrder/);
  assert.match(source, /\.sort/);
});

test("SettingsModal auto-saves on model change", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /saveProviderDraft/);
  assert.match(source, /saveProviderSettings/);
});

test("ProviderSettings default option is empty string (default)", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /value=""/);
  assert.match(source, /default/);
});
```

- [ ] **Step 2: Run the new test file**

```bash
cd apps/desktop && node --experimental-strip-types --test tests/peonModelPicker.test.ts 2>&1
```
Expected: all 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/tests/peonModelPicker.test.ts
git commit -m "test: add peon model picker frontend tests"
```

---

### Task 11: Verification — run all tests and type-check

**Files:** None (verification only)

- [ ] **Step 1: Run Rust tests**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1
```
Expected: all tests pass.

- [ ] **Step 2: TypeScript type-check**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1
```
Expected: no errors.

- [ ] **Step 3: Run all frontend tests**

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs 2>&1
```
Expected: all tests pass.

- [ ] **Step 4: Run doc currency check**

```bash
bash .claude/hooks/doc-check.sh
```
Address any flagged files.

- [ ] **Step 5: Commit any remaining changes (if doc updates needed)**

```bash
git add -A && git diff --cached --stat
git commit -m "chore: final verification and doc updates"
```
