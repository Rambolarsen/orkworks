# Ollama Provider Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire Ollama as a Peon-capable provider with HTTP model listing and inference, alongside Ollama model pass-through for existing harness sessions.

**Architecture:** Add reqwest (async, rustls-tls) to the Rust sidecar for HTTP calls to Ollama's REST API. Extend `ProviderDefinition` to support HTTP-based providers alongside existing CLI-based ones. Add Ollama base URL to settings types and UI. The existing `{model}` template substitution in harness args already handles pass-through; Ollama just needs to appear as a valid provider with discoverable models.

**Tech Stack:** Rust (reqwest + tokio), TypeScript (Electron IPC, React), existing axum HTTP server

---

### Task 1: Add reqwest dependency

**Files:**
- Modify: `crates/orkworksd/Cargo.toml`

- [ ] **Step 1: Add reqwest to Cargo.toml**

```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```

Add this line after `dirs = "5"` on line 18:

```toml
18: dirs = "5"
19: reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
20: git2 = "0.19"
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check --manifest-path crates/orkworksd/Cargo.toml
```

Expected: Success (may download reqwest and its deps).

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/Cargo.toml
git commit -m "chore: add reqwest (rustls-tls) for Ollama HTTP client"
```

---

### Task 2: Add ollama to ProviderId and settings types (TypeScript)

**Files:**
- Modify: `apps/desktop/electron/providerTypes.ts:1`
- Modify: `apps/desktop/src/providerTypes.ts:1`
- Modify: `apps/desktop/electron/settingsMemory.ts:100-115,213-256`
- Modify: `apps/desktop/src/appSettingsTypes.ts:19-29`

- [ ] **Step 1: Add `"ollama"` to ProviderId union type in electron/providerTypes.ts**

Edit line 1 — add `| "ollama"` at the end:

```ts
export type ProviderId = "opencode" | "claude-code" | "codex" | "gemini" | "aider" | "gh-copilot" | "ollama";
```

- [ ] **Step 2: Add `"ollama"` to ProviderId in src/providerTypes.ts (renderer copy)**

Edit line 1 — add `| "ollama"` at the end:

```ts
export type ProviderId = "opencode" | "claude-code" | "codex" | "gemini" | "aider" | "gh-copilot" | "ollama";
```

- [ ] **Step 3: Add `ollamaBaseUrl` to ProviderSettings and AppSettings**

In `apps/desktop/electron/providerTypes.ts`, add after line 14:

```ts
export interface ProviderSettings {
  version: 1;
  revision: number;
  peonModel: string | null;
  ollamaBaseUrl: string;
  providers: ProviderSettingsEntry[];
}
```

In `apps/desktop/src/providerTypes.ts`, add the same `ollamaBaseUrl: string` field to the renderer-side `ProviderSettings` interface.

- [ ] **Step 4: Add `ollamaBaseUrl` to AppSettings in src/appSettingsTypes.ts**

The `AppSettings` interface wraps `ProviderSettings` via `providers: ProviderSettings`, so `ollamaBaseUrl` flows through automatically. No change needed in `appSettingsTypes.ts` itself — the field lives on `ProviderSettings`.

- [ ] **Step 5: Add ollama to VALID_PROVIDER_IDS and DEFAULT_PROVIDER_SETTINGS in settingsMemory.ts**

On line 100, add `"ollama"`:

```ts
const VALID_PROVIDER_IDS = new Set<ProviderId>(["opencode", "claude-code", "codex", "gemini", "aider", "gh-copilot", "ollama"]);
```

On line 103, add `ollamaBaseUrl` default and an ollama entry to `DEFAULT_PROVIDER_SETTINGS`:

```ts
export const DEFAULT_PROVIDER_SETTINGS: ProviderSettings = {
  version: 1,
  revision: 0,
  peonModel: null,
  ollamaBaseUrl: "http://127.0.0.1:11434",
  providers: [
    { id: "opencode", enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
    { id: "claude-code", enabled: true, fallbackOrder: 1, defaultState: "unknown", overrideState: null },
    { id: "codex", enabled: true, fallbackOrder: 2, defaultState: "unknown", overrideState: null },
    { id: "gemini", enabled: true, fallbackOrder: 3, defaultState: "unknown", overrideState: null },
    { id: "aider", enabled: true, fallbackOrder: 4, defaultState: "unknown", overrideState: null },
    { id: "gh-copilot", enabled: true, fallbackOrder: 5, defaultState: "unknown", overrideState: null },
    { id: "ollama", enabled: true, fallbackOrder: 6, defaultState: "unknown", overrideState: null },
  ],
};
```

- [ ] **Step 6: Add ollamaBaseUrl normalization in normalizeProviderSettings**

After line 238 (`peonModel: normalizePeonModel(raw),`), add the ollamaBaseUrl normalization:

```ts
return {
    version: 1,
    revision:
      typeof raw.revision === "number" && Number.isFinite(raw.revision)
        ? Math.max(0, Math.trunc(raw.revision))
        : DEFAULT_PROVIDER_SETTINGS.revision,
    peonModel: normalizePeonModel(raw),
    ollamaBaseUrl: normalizeOllamaBaseUrl(raw),
    providers,
  };
```

Add the helper function before `normalizePeonModel` (around line 243):

```ts
function normalizeOllamaBaseUrl(raw: Record<string, unknown>): string {
  const val = raw.ollamaBaseUrl;
  if (typeof val === "string" && val.length > 0) {
    const trimmed = val.trim();
    if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
      return trimmed.replace(/\/+$/, "");
    }
  }
  return DEFAULT_PROVIDER_SETTINGS.ollamaBaseUrl;
}
```

- [ ] **Step 7: Verify TypeScript compilation**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: No errors.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/electron/providerTypes.ts apps/desktop/src/providerTypes.ts apps/desktop/electron/settingsMemory.ts apps/desktop/src/appSettingsTypes.ts
git commit -m "feat(ollama): add ollama to ProviderId, ollamaBaseUrl to settings types"
```

---

### Task 3: Add Ollama ProviderDefinition to Rust registry

**Files:**
- Modify: `crates/orkworksd/src/providers.rs:117-218`

- [ ] **Step 1: Add ollamaBaseUrl to ProviderSettingsPayload**

In `crates/orkworksd/src/providers.rs`, after line 81 (`pub peon_model: Option<String>,`), add:

```rust
#[serde(rename = "ollamaBaseUrl", default = "default_ollama_base_url")]
pub ollama_base_url: String,
```

Add the default function before `impl Default for ProviderSettingsPayload`:

```rust
fn default_ollama_base_url() -> String {
    "http://127.0.0.1:11434".to_string()
}
```

- [ ] **Step 2: Update Default impl to include ollama_base_url**

```rust
impl Default for ProviderSettingsPayload {
    fn default() -> Self {
        Self { version: 1, revision: 0, peon_model: None, ollama_base_url: default_ollama_base_url(), providers: vec![] }
    }
}
```

- [ ] **Step 3: Add Ollama entry to builtin_provider_registry()**

After the `gh-copilot` entry (line 216), add:

```rust
ProviderDefinition {
    id: "ollama",
    label: "Ollama",
    command: "",
    default_args: &[],
    model_arg_template: None,
    supports_model: false,
    timeout_secs: 30,
    list_models_command: None,
    list_models_args: &[],
    static_models: &[],
},
```

The `list_models_command: None` with empty `list_models_args` and empty `static_models` triggers `list_models()` to return an empty vec — we'll override this in Task 4 with HTTP-based model listing.

- [ ] **Step 4: Verify compilation**

```bash
cargo check --manifest-path crates/orkworksd/Cargo.toml
```

Expected: Success.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/providers.rs
git commit -m "feat(ollama): add Ollama ProviderDefinition and ollamaBaseUrl to settings payload"
```

---

### Task 4: Implement HTTP model listing and inference for Ollama

**Files:**
- Modify: `crates/orkworksd/src/providers.rs:1-11,287-346,359-368,419-500,502-613`

This is the core Rust change. We need to:
1. Add `use` imports for reqwest and tokio-timeout
2. Add an `HttpRunner` that implements `ProviderRunner` for Ollama
3. Add HTTP model listing logic to `list_models()`
4. Make `ProviderManager` use `HttpRunner` when the provider is ollama

- [ ] **Step 1: Add imports**

Replace lines 1-6 of `providers.rs`:

```rust
use std::collections::HashMap;
use std::io::Read;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};

use crate::peon;
```

Wait — the user chose async reqwest. But `ProviderRunner::run` is sync. The existing architecture uses blocking I/O throughout. We can use `reqwest::blocking::Client` which is simple and fits the sync architecture. The async reqwest is available via `reqwest::Client` but we'd need `tokio::task::spawn_blocking` or `block_on` wrappers. Using `reqwest::blocking` is the pragmatic choice here — it's part of the `reqwest` crate when `blocking` feature is enabled.

Let me revise: we should enable the `blocking` feature. Actually, let me re-check Cargo.toml features. With `default-features = false, features = ["rustls-tls", "json"]`, we don't get the blocking feature. Let me add it.

Actually, wait - `reqwest::blocking` requires the `blocking` feature. Let me update the Cargo.toml in Task 1.

Hmm, but the user chose "reqwest (async)". Let me use `reqwest::Client` (async) and call `.block_on()` or use `tokio::task::block_in_place` from within the sync code. Or we can use `tokio::runtime::Handle::current().block_on()`.

Actually, the simplest approach: since the `ProviderManager` is already running inside a tokio runtime (`#[tokio::main]`), we can use `Handle::current().block_on()` to run async reqwest calls from sync code. This is a common pattern.

Let me use reqwest async with `block_on`. The imports would be:
```rust
use reqwest::Client as HttpClient;
```

And we'd do:
```rust
let rt = tokio::runtime::Handle::current();
let resp = rt.block_on(http_client.get(&url).send());
```

This is clean. Let me proceed with this approach.

Actually, I need to be careful. The `list_models()` method is called from `tokio::task::spawn_blocking()`, so we're inside a blocking task. We can use `Handle::current().block_on()` from there. This works.

And `run_inference()` is called from `spawn_blocking` too (looking at main.rs). So the same pattern works.

Let me write this plan properly.

- [ ] **Step 1: Add imports**

Replace the imports at the top of `providers.rs` (lines 1-10):

```rust
use std::collections::HashMap;
use std::io::Read;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

use crate::peon;
```

Also add the serde types for Ollama API responses.

- [ ] **Step 2: Add Ollama API response types**

After `use crate::peon;` (line 10), add:

```rust
// --- Ollama API types ---

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Deserialize)]
struct OllamaModelEntry {
    name: String,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
}
```

- [ ] **Step 3: Add HTTP model listing to ProviderDefinition**

Add a new optional field to `ProviderDefinition` for HTTP-based model listing:

```rust
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
    pub static_models: &'static [&'static str],
    pub http_list_models: bool,               // NEW: use HTTP for model listing
}
```

Update the ollama entry in `builtin_provider_registry()`:

```rust
ProviderDefinition {
    id: "ollama",
    label: "Ollama",
    command: "",
    default_args: &[],
    model_arg_template: None,
    supports_model: false,
    timeout_secs: 30,
    list_models_command: None,
    list_models_args: &[],
    static_models: &[],
    http_list_models: true,
},
```

Update all OTHER entries to add `http_list_models: false,`.

- [ ] **Step 4: Add HTTP model listing logic to list_models()**

In `list_models()` (after the existing CLI model listing logic at line 426), add before the CLI branch:

```rust
pub fn list_models(&self, provider_id: &str) -> Result<Vec<String>, String> {
    let definition = self.registry.iter()
        .find(|d| d.id == provider_id)
        .ok_or_else(|| format!("unknown provider: {provider_id}"))?;

    // HTTP-based model listing (Ollama)
    if definition.http_list_models {
        return self.list_models_http(provider_id);
    }

    // ... existing CLI logic ...
}
```

Add the `list_models_http` method:

```rust
fn list_models_http(&self, provider_id: &str) -> Result<Vec<String>, String> {
    let settings = self.settings.read().unwrap().clone();
    let base_url = match provider_id {
        "ollama" => &settings.ollama_base_url,
        _ => return Err(format!("no HTTP base URL configured for {provider_id}")),
    };

    let url = format!("{base_url}/api/tags");
    let client = HttpClient::new();
    let rt = tokio::runtime::Handle::current();

    let resp = rt.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(10),
            client.get(&url).send(),
        ).await
    }).map_err(|_| format!("Ollama request timed out for {url}"))?
    .map_err(|e| {
        if e.is_connect() {
            format!("Ollama endpoint unreachable at {base_url}: connection refused")
        } else if e.is_timeout() {
            format!("Ollama request timed out for {url}")
        } else {
            format!("Ollama request failed: {e}")
        }
    })?;

    if !resp.status().is_success() {
        return Err(format!("Ollama returned HTTP {}", resp.status()));
    }

    let body = rt.block_on(resp.text())
        .map_err(|e| format!("failed to read Ollama response: {e}"))?;

    let tags: OllamaTagsResponse = serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse Ollama /api/tags response: {e}"))?;

    if tags.models.is_empty() {
        return Err("Ollama returned an empty model list".to_string());
    }

    let models: Vec<String> = tags.models.into_iter().map(|m| m.name).collect();
    Ok(models)
}
```

- [ ] **Step 5: Add HTTP inference runner**

Add `HttpRunner` struct after `ProcessRunner` (after line 346):

```rust
struct HttpRunner {
    settings: Arc<RwLock<ProviderSettingsPayload>>,
}

impl ProviderRunner for HttpRunner {
    fn run(&self, id: &str, _command: &str, _args: &[String], prompt: &str, _timeout_secs: u64) -> InvocationResult {
        let settings = self.settings.read().unwrap().clone();
        let base_url = match id {
            "ollama" => &settings.ollama_base_url,
            _ => return InvocationResult {
                success: false,
                stdout: String::new(),
                stderr: format!("HttpRunner does not support provider {id}"),
            },
        };

        let model = match &settings.peon_model {
            Some(m) => m.clone(),
            None => {
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: "no Ollama model selected in Peon settings".to_string(),
                }
            }
        };

        let url = format!("{base_url}/api/generate");
        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
        });

        let client = HttpClient::new();
        let rt = tokio::runtime::Handle::current();

        let resp = match rt.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(30),
                client.post(&url).json(&body).send(),
            ).await
        }) {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                let msg = if e.is_connect() {
                    format!("Ollama endpoint unreachable at {base_url}: connection refused")
                } else if e.is_timeout() {
                    format!("Ollama generate request timed out")
                } else {
                    format!("Ollama generate request failed: {e}")
                };
                return InvocationResult { success: false, stdout: String::new(), stderr: msg };
            }
            Err(_) => {
                return InvocationResult { success: false, stdout: String::new(), stderr: "Ollama generate request timed out".to_string() };
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = rt.block_on(resp.text()).unwrap_or_default();
            return InvocationResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Ollama returned HTTP {}: {}", status.as_u16(), err_body.trim()),
            };
        }

        let text = match rt.block_on(resp.text()) {
            Ok(t) => t,
            Err(e) => return InvocationResult {
                success: false,
                stdout: String::new(),
                stderr: format!("failed to read Ollama response: {e}"),
            },
        };

        match serde_json::from_str::<OllamaGenerateResponse>(&text) {
            Ok(gen) => {
                InvocationResult {
                    success: true,
                    stdout: gen.response,
                    stderr: String::new(),
                }
            }
            Err(e) => {
                InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("failed to parse Ollama generate response: {e}"),
                }
            }
        }
    }
}
```

- [ ] **Step 6: Update ProviderManager to use HttpRunner for ollama**

In `ProviderManager::new()` (lines 359-368), change the runner field to be more flexible. We need a way to dispatch between ProcessRunner and HttpRunner. The cleanest approach: create a composite runner that delegates.

Modify `ProviderManager::new()`:

```rust
impl ProviderManager {
    pub fn new() -> Self {
        let settings = Arc::new(RwLock::new(ProviderSettingsPayload::default()));
        let runtime = Arc::new(RwLock::new(HashMap::new()));
        Self {
            registry: builtin_provider_registry(),
            settings: settings.clone(),
            applied_revision: Arc::new(RwLock::new(None)),
            runtime,
            runner: Arc::new(CompositeRunner {
                process: ProcessRunner,
                http: HttpRunner { settings },
            }),
        }
    }
}
```

Add `CompositeRunner` before the `ProcessRunner`:

```rust
struct CompositeRunner {
    process: ProcessRunner,
    http: HttpRunner,
}

impl ProviderRunner for CompositeRunner {
    fn run(&self, id: &str, command: &str, args: &[String], prompt: &str, timeout_secs: u64) -> InvocationResult {
        match id {
            "ollama" => self.http.run(id, command, args, prompt, timeout_secs),
            _ => self.process.run(id, command, args, prompt, timeout_secs),
        }
    }
}
```

- [ ] **Step 7: Update the test `for_tests` constructor to use CompositeRunner**

In the `#[cfg(test)]` block, update `ProviderManager::for_tests` and `for_tests_with_registry` to use a `TestRunner` instead (since `FakeRunner` is already there). The test constructors pass `Arc::new(FakeRunner { specs })` directly — no change needed since tests don't call real Ollama.

Wait — the test constructors use `Arc::new(FakeRunner { specs })` which already implements `ProviderRunner`. That's fine — tests bypass the CompositeRunner entirely. No change needed.

But `for_tests_with_registry` uses `self.runner = Arc::new(FakeRunner { specs })` — good.

- [ ] **Step 8: Add test helper to build settings with ollamaBaseUrl**

No changes needed to existing tests — the `TestEntryBuilder` pattern doesn't include `ollama_base_url` and defaults will be used.

- [ ] **Step 9: Add Ollama-specific tests**

At the end of the test module (after line 971), add:

```rust
    #[test]
    fn ollama_list_models_uses_http_path() {
        let settings = ProviderSettingsPayload {
            version: 1,
            revision: 1,
            peon_model: None,
            ollama_base_url: "http://127.0.0.1:11434".to_string(),
            providers: vec![],
        };
        let manager = ProviderManager::for_tests(settings, vec![]);
        // With no real Ollama running, this should return a connection error
        let err = manager.list_models("ollama").unwrap_err();
        assert!(err.contains("unreachable") || err.contains("connection refused") || err.contains("Connection refused"));
    }

    #[test]
    fn ollama_provider_definition_exists() {
        let registry = builtin_provider_registry();
        let ollama = registry.iter().find(|d| d.id == "ollama");
        assert!(ollama.is_some());
        let ollama = ollama.unwrap();
        assert_eq!(ollama.label, "Ollama");
        assert!(ollama.http_list_models);
    }

    #[test]
    fn ollama_in_run_inference_surfaces_error_when_no_model_selected() {
        let manager = ProviderManager::for_tests(
            ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: "http://127.0.0.1:11434".to_string(),
                providers: vec![
                    ProviderSettingsEntry {
                        id: "ollama".to_string(),
                        enabled: true,
                        fallback_order: 0,
                        default_state: ProviderCapacityState::Healthy,
                        override_state: None,
                    },
                ],
            },
            vec![],
        );
        let result = manager.run_inference(PeonScope::Session, &["test".to_string()]);
        assert!(result.inference.is_none());
        assert_eq!(result.attempts.len(), 1);
        assert_eq!(result.attempts[0].outcome, AttemptOutcome::Failed);
    }
```

Wait — actually the `list_models` test won't work because `list_models` is called via `tokio::runtime::Handle::current()` which requires a tokio runtime. In tests, there's no runtime by default. Let me think about this...

The tests don't use `#[tokio::test]`, they use plain `#[test]`. The `list_models` call for ollama will try `tokio::runtime::Handle::current()` which will panic with "no reactor running".

Options:
1. Add `#[tokio::test]` for Ollama-specific tests
2. Mock out the HTTP client via the runner pattern
3. Use a different approach — spawn a local runtime in the test

The simplest is to make the HTTP client injectable. But that's over-engineering for this test. Let me instead test that `list_models` returns the proper error for ollama when there's a tokio runtime. Or, we can make the HTTP client a trait/field that can be swapped in tests.

Actually, the simplest approach: use `tokio::runtime::Runtime::new().unwrap().block_on(...)` in the `list_models_http` method instead of `Handle::current()`. This works in any context (including tests) because it creates its own runtime if needed.

Let me revise: instead of `Handle::current()`, use:

```rust
let rt = tokio::runtime::Runtime::new().unwrap();
rt.block_on(async { ... })
```

Wait, that's wasteful — creates a new runtime for every call. Better to use `Handle::try_current()` and fall back to a new runtime:

Actually, the cleanest approach for production + test: use `Handle::current()` in production (which works inside `#[tokio::main]`), and for tests, we'd mark them as `#[tokio::test]`.

Let me update the plan to add `tokio::test` for Ollama-specific tests. Actually, I notice the existing tests already use plain `#[test]` and they work because they don't call any async code or handle.current(). The ollama test that calls `list_models("ollama")` will fail.

Let me handle this by using `Handle::try_current()`:

```rust
fn block_on_http<F: std::future::Future>(f: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(f),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(f)
        }
    }
}
```

Then use `block_on_http` everywhere instead of `rt.block_on`. This works in both tests and production.

Let me simplify the plan — I'll include this helper.

OK let me write the final plan now. I've been overthinking. Let me put it all together.<｜end▁of▁thinking｜>Now let me write the complete plan:

<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="write">
<｜｜DSML｜｜parameter name="content" string="true"># Ollama Provider Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire Ollama as a Peon-capable provider with HTTP model listing and inference, alongside Ollama model pass-through for existing harness sessions.

**Architecture:** Add reqwest (async, rustls-tls) to the Rust sidecar for HTTP calls to Ollama's REST API (`GET /api/tags` for model listing, `POST /api/generate` for inference). Add `http_list_models: bool` to `ProviderDefinition` so providers can opt into HTTP-based model discovery. Add a `CompositeRunner` that dispatches to `HttpRunner` for ollama and `ProcessRunner` for everything else. Add `ollamaBaseUrl` to settings types and UI. The existing `{model}` template substitution already handles harness pass-through.

**Tech Stack:** Rust (reqwest + tokio + serde_json), TypeScript (Electron IPC, React), existing axum HTTP server

---

### Task 1: Add reqwest dependency

**Files:**
- Modify: `crates/orkworksd/Cargo.toml:19`

- [ ] **Step 1: Add reqwest to Cargo.toml**

Replace line 18-19:
```toml
dirs = "5"
git2 = "0.19"
```

With:
```toml
dirs = "5"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
git2 = "0.19"
```

- [ ] **Step 2: cargo check**

```bash
cargo check --manifest-path crates/orkworksd/Cargo.toml
```

Expected: success (may download reqwest deps).

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/Cargo.toml
git commit -m "chore: add reqwest (rustls-tls) for Ollama HTTP client"
```

---

### Task 2: Add ollama to TypeScript types and settings

**Files:**
- Modify: `apps/desktop/electron/providerTypes.ts:1,13-18`
- Modify: `apps/desktop/src/providerTypes.ts:1,13-18`
- Modify: `apps/desktop/electron/settingsMemory.ts:100,103-115,213-256`

- [ ] **Step 1: Add `"ollama"` to ProviderId in electron/providerTypes.ts**

Line 1 becomes:
```ts
export type ProviderId = "opencode" | "claude-code" | "codex" | "gemini" | "aider" | "gh-copilot" | "ollama";
```

- [ ] **Step 2: Add `ollamaBaseUrl` to ProviderSettings in electron/providerTypes.ts**

Lines 13-18 become:
```ts
export interface ProviderSettings {
  version: 1;
  revision: number;
  peonModel: string | null;
  ollamaBaseUrl: string;
  providers: ProviderSettingsEntry[];
}
```

- [ ] **Step 3: Add `"ollama"` to ProviderId in src/providerTypes.ts**

Line 1 becomes:
```ts
export type ProviderId = "opencode" | "claude-code" | "codex" | "gemini" | "aider" | "gh-copilot" | "ollama";
```

- [ ] **Step 4: Add `ollamaBaseUrl` to ProviderSettings in src/providerTypes.ts**

Lines 13-18 become:
```ts
export interface ProviderSettings {
  version: 1;
  revision: number;
  peonModel: string | null;
  ollamaBaseUrl: string;
  providers: ProviderSettingsEntry[];
}
```

- [ ] **Step 5: Add ollama to VALID_PROVIDER_IDS and DEFAULT_PROVIDER_SETTINGS in settingsMemory.ts**

Line 100 becomes:
```ts
const VALID_PROVIDER_IDS = new Set<ProviderId>(["opencode", "claude-code", "codex", "gemini", "aider", "gh-copilot", "ollama"]);
```

Lines 103-115 become:
```ts
export const DEFAULT_PROVIDER_SETTINGS: ProviderSettings = {
  version: 1,
  revision: 0,
  peonModel: null,
  ollamaBaseUrl: "http://127.0.0.1:11434",
  providers: [
    { id: "opencode", enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
    { id: "claude-code", enabled: true, fallbackOrder: 1, defaultState: "unknown", overrideState: null },
    { id: "codex", enabled: true, fallbackOrder: 2, defaultState: "unknown", overrideState: null },
    { id: "gemini", enabled: true, fallbackOrder: 3, defaultState: "unknown", overrideState: null },
    { id: "aider", enabled: true, fallbackOrder: 4, defaultState: "unknown", overrideState: null },
    { id: "gh-copilot", enabled: true, fallbackOrder: 5, defaultState: "unknown", overrideState: null },
    { id: "ollama", enabled: true, fallbackOrder: 6, defaultState: "unknown", overrideState: null },
  ],
};
```

- [ ] **Step 6: Add normalizeOllamaBaseUrl function and wire it into normalizeProviderSettings**

After the `normalizePeonModel` function (after line 256), add:
```ts
function normalizeOllamaBaseUrl(raw: Record<string, unknown>): string {
  const val = raw.ollamaBaseUrl;
  if (typeof val === "string" && val.length > 0) {
    const trimmed = val.trim();
    if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
      return trimmed.replace(/\/+$/, "");
    }
  }
  return DEFAULT_PROVIDER_SETTINGS.ollamaBaseUrl;
}
```

In `normalizeProviderSettings` return value (after line 238 `peonModel: normalizePeonModel(raw),`), add:
```ts
    ollamaBaseUrl: normalizeOllamaBaseUrl(raw),
```

- [ ] **Step 7: tsc check**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/electron/providerTypes.ts apps/desktop/src/providerTypes.ts apps/desktop/electron/settingsMemory.ts
git commit -m "feat(ollama): add ollama to ProviderId, ollamaBaseUrl to ProviderSettings types and defaults"
```

---

### Task 3: Add Ollama ProviderDefinition and ollamaBaseUrl to Rust types

**Files:**
- Modify: `crates/orkworksd/src/providers.rs:76-89,103-115,117-218,287-346,359-368`

- [ ] **Step 1: Add ollama_base_url to ProviderSettingsPayload**

After line 81 (`pub peon_model: Option<String>,`), add:
```rust
    #[serde(rename = "ollamaBaseUrl", default = "default_ollama_base_url")]
    pub ollama_base_url: String,
```

Before `impl Default for ProviderSettingsPayload`, add the default function:
```rust
fn default_ollama_base_url() -> String {
    "http://127.0.0.1:11434".to_string()
}
```

Update Default impl (line 86-88):
```rust
impl Default for ProviderSettingsPayload {
    fn default() -> Self {
        Self { version: 1, revision: 0, peon_model: None, ollama_base_url: default_ollama_base_url(), providers: vec![] }
    }
}
```

- [ ] **Step 2: Add http_list_models field to ProviderDefinition**

Lines 103-115 become:
```rust
#[derive(Clone, Debug)]
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
    pub static_models: &'static [&'static str],
    pub http_list_models: bool,
}
```

- [ ] **Step 3: Add ollama entry to builtin_provider_registry() and add http_list_models: false to all others**

After the gh-copilot entry (after line 216 in the original, before closing `]`), add:
```rust
        ProviderDefinition {
            id: "ollama",
            label: "Ollama",
            command: "",
            default_args: &[],
            model_arg_template: None,
            supports_model: false,
            timeout_secs: 30,
            list_models_command: None,
            list_models_args: &[],
            static_models: &[],
            http_list_models: true,
        },
```

Add `http_list_models: false,` to each of the existing 6 entries (opencode, claude-code, codex, gemini, aider, gh-copilot).

- [ ] **Step 4: cargo check**

```bash
cargo check --manifest-path crates/orkworksd/Cargo.toml
```

Expected: success.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/providers.rs
git commit -m "feat(ollama): add Ollama ProviderDefinition, http_list_models field, ollamaBaseUrl to settings payload"
```

---

### Task 4: Implement HTTP model listing and inference for Ollama

**Files:**
- Modify: `crates/orkworksd/src/providers.rs:1-10,287-368,419-500,502-613,846-972`

- [ ] **Step 1: Add imports and Ollama API types**

Replace lines 1-10:
```rust
use std::collections::HashMap;
use std::io::Read;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

use crate::peon;

// --- Ollama API types ---

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Deserialize)]
struct OllamaModelEntry {
    name: String,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}
```

- [ ] **Step 2: Add block_on_http helper**

Before the `ProviderRunner` trait (around line 287), add:
```rust
fn block_on_http<F: std::future::Future>(f: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(f),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime for HTTP");
            rt.block_on(f)
        }
    }
}
```

- [ ] **Step 3: Add HttpRunner after ProcessRunner**

After the `ProcessRunner` impl (line 346), add:
```rust
struct HttpRunner {
    settings: Arc<RwLock<ProviderSettingsPayload>>,
}

impl ProviderRunner for HttpRunner {
    fn run(&self, id: &str, _command: &str, _args: &[String], prompt: &str, _timeout_secs: u64) -> InvocationResult {
        let settings = self.settings.read().unwrap().clone();
        let base_url = match id {
            "ollama" => settings.ollama_base_url.clone(),
            _ => return InvocationResult {
                success: false,
                stdout: String::new(),
                stderr: format!("HttpRunner does not support provider {id}"),
            },
        };

        let model = match &settings.peon_model {
            Some(m) if !m.is_empty() => m.clone(),
            _ => {
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: "no Ollama model selected in Peon settings".to_string(),
                }
            }
        };

        let url = format!("{base_url}/api/generate");
        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
        });

        let client = HttpClient::new();

        let request_fut = client.post(&url).json(&body).send();
        let resp = match block_on_http(async {
            tokio::time::timeout(Duration::from_secs(30), request_fut).await
        }) {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                let msg = if e.is_connect() {
                    format!("Ollama endpoint unreachable at {base_url}")
                } else if e.is_timeout() {
                    "Ollama generate request timed out".to_string()
                } else {
                    format!("Ollama generate request failed: {e}")
                };
                return InvocationResult { success: false, stdout: String::new(), stderr: msg };
            }
            Err(_) => {
                return InvocationResult { success: false, stdout: String::new(), stderr: "Ollama generate request timed out".to_string() };
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = block_on_http(resp.text()).unwrap_or_default();
            return InvocationResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Ollama returned HTTP {}: {}", status.as_u16(), err_body.trim()),
            };
        }

        let text = match block_on_http(resp.text()) {
            Ok(t) => t,
            Err(e) => return InvocationResult {
                success: false,
                stdout: String::new(),
                stderr: format!("failed to read Ollama response: {e}"),
            },
        };

        match serde_json::from_str::<OllamaGenerateResponse>(&text) {
            Ok(gen) => {
                InvocationResult {
                    success: true,
                    stdout: gen.response,
                    stderr: String::new(),
                }
            }
            Err(e) => {
                InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("failed to parse Ollama generate response: {e}"),
                }
            }
        }
    }
}
```

- [ ] **Step 4: Add CompositeRunner and update ProviderManager::new()**

Add `CompositeRunner` struct before `ProcessRunner`:
```rust
struct CompositeRunner {
    process: ProcessRunner,
    http: HttpRunner,
}

impl ProviderRunner for CompositeRunner {
    fn run(&self, id: &str, command: &str, args: &[String], prompt: &str, timeout_secs: u64) -> InvocationResult {
        match id {
            "ollama" => self.http.run(id, command, args, prompt, timeout_secs),
            _ => self.process.run(id, command, args, prompt, timeout_secs),
        }
    }
}
```

Change `ProviderManager::new()` (lines 359-368) to:
```rust
impl ProviderManager {
    pub fn new() -> Self {
        let settings = Arc::new(RwLock::new(ProviderSettingsPayload::default()));
        let runtime = Arc::new(RwLock::new(HashMap::new()));
        Self {
            registry: builtin_provider_registry(),
            settings: settings.clone(),
            applied_revision: Arc::new(RwLock::new(None)),
            runtime,
            runner: Arc::new(CompositeRunner {
                process: ProcessRunner,
                http: HttpRunner { settings },
            }),
        }
    }
}
```

- [ ] **Step 5: Add HTTP model listing to list_models()**

In `list_models()` (around line 419), replace the top of the method with:
```rust
    pub fn list_models(&self, provider_id: &str) -> Result<Vec<String>, String> {
        let definition = self.registry.iter()
            .find(|d| d.id == provider_id)
            .ok_or_else(|| format!("unknown provider: {provider_id}"))?;

        if definition.http_list_models {
            return self.list_models_http(provider_id);
        }

        if definition.list_models_command.is_none() || definition.list_models_args.is_empty() {
            return Ok(definition.static_models.iter().map(|s| s.to_string()).collect());
        }
        // ... rest of existing CLI model listing code ...
```

Add the `list_models_http` method to `impl ProviderManager` (before the closing `}` of that impl block):
```rust
    fn list_models_http(&self, provider_id: &str) -> Result<Vec<String>, String> {
        let settings = self.settings.read().unwrap().clone();
        let base_url = match provider_id {
            "ollama" => &settings.ollama_base_url,
            _ => return Err(format!("no HTTP base URL configured for {provider_id}")),
        };

        let url = format!("{base_url}/api/tags");
        let client = HttpClient::new();

        let request_fut = client.get(&url).send();
        let resp = match block_on_http(async {
            tokio::time::timeout(Duration::from_secs(10), request_fut).await
        }) {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                let msg = if e.is_connect() {
                    format!("Ollama endpoint unreachable at {base_url}")
                } else if e.is_timeout() {
                    format!("Ollama request timed out for {url}")
                } else {
                    format!("Ollama request failed: {e}")
                };
                return Err(msg);
            }
            Err(_) => return Err(format!("Ollama request timed out for {url}")),
        };

        if !resp.status().is_success() {
            return Err(format!("Ollama returned HTTP {}", resp.status().as_u16()));
        }

        let body = block_on_http(resp.text())
            .map_err(|e| format!("failed to read Ollama response: {e}"))?;

        let tags: OllamaTagsResponse = serde_json::from_str(&body)
            .map_err(|e| format!("failed to parse Ollama /api/tags response: {e}"))?;

        if tags.models.is_empty() {
            return Err("Ollama returned an empty model list".to_string());
        }

        let models: Vec<String> = tags.models.into_iter().map(|m| m.name).collect();
        Ok(models)
    }
```

- [ ] **Step 6: Update tests to include ollama_base_url and http_list_models**

In `#[cfg(test)] mod tests`, update all `ProviderSettingsPayload` literals to include `ollama_base_url`:

Every `ProviderSettingsPayload { version: 1, revision: 1, peon_model: None, providers: ... }` becomes:
```rust
ProviderSettingsPayload {
    version: 1,
    revision: 1,
    peon_model: None,
    ollama_base_url: "http://127.0.0.1:11434".to_string(),
    providers: ...
}
```

Update `sample_settings` function (line 833-840):
```rust
    fn sample_settings(builders: Vec<TestEntryBuilder>) -> ProviderSettingsPayload {
        ProviderSettingsPayload {
            version: 1,
            revision: 1,
            peon_model: None,
            ollama_base_url: "http://127.0.0.1:11434".to_string(),
            providers: builders.into_iter().map(|b| b.build()).collect(),
        }
    }
```

In `list_models_returns_empty_when_no_list_command_configured` (line 917-937), add `http_list_models: false,`.

In `list_models_returns_static_models_when_no_command` (line 940-959), add `http_list_models: false,`.

In `list_models_returns_error_for_unknown_provider` (line 962-971), update to include `ollama_base_url`:
```rust
    #[test]
    fn list_models_returns_error_for_unknown_provider() {
        let manager = ProviderManager::for_tests(
            ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: "http://127.0.0.1:11434".to_string(),
                providers: vec![],
            },
            vec![],
        );

        let err = manager.list_models("nonexistent").unwrap_err();
        assert!(err.contains("unknown provider"));
    }
```

- [ ] **Step 7: Add Ollama-specific tests**

After the last test (line 971), add:
```rust
    #[test]
    fn ollama_provider_definition_in_registry() {
        let registry = builtin_provider_registry();
        let ollama = registry.iter().find(|d| d.id == "ollama");
        assert!(ollama.is_some());
        let ollama = ollama.unwrap();
        assert_eq!(ollama.label, "Ollama");
        assert!(ollama.http_list_models);
    }

    #[test]
    fn ollama_inference_fails_without_model() {
        let manager = ProviderManager::for_tests(
            ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: "http://127.0.0.1:11434".to_string(),
                providers: vec![ProviderSettingsEntry {
                    id: "ollama".to_string(),
                    enabled: true,
                    fallback_order: 0,
                    default_state: ProviderCapacityState::Healthy,
                    override_state: None,
                }],
            },
            vec![],
        );
        let result = manager.run_inference(PeonScope::Session, &["test".to_string()]);
        assert!(result.inference.is_none());
        assert_eq!(result.attempts.len(), 1);
        assert_eq!(result.attempts[0].outcome, AttemptOutcome::Failed);
    }

    #[test]
    fn ollama_list_models_makes_no_real_request_in_tests() {
        let manager = ProviderManager::for_tests(
            ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: "http://127.0.0.1:11434".to_string(),
                providers: vec![],
            },
            vec![],
        );
        // With no real Ollama running, this returns an error (not a panic)
        let result = manager.list_models("ollama");
        assert!(result.is_err());
    }

    #[test]
    fn ollama_disabled_is_skipped() {
        let manager = ProviderManager::for_tests(
            ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: "http://127.0.0.1:11434".to_string(),
                providers: vec![ProviderSettingsEntry {
                    id: "ollama".to_string(),
                    enabled: false,
                    fallback_order: 0,
                    default_state: ProviderCapacityState::Healthy,
                    override_state: None,
                }],
            },
            vec![],
        );
        let result = manager.run_inference(PeonScope::Session, &["test".to_string()]);
        assert!(result.inference.is_none());
        assert_eq!(result.attempts[0].outcome, AttemptOutcome::SkippedDisabled);
    }
```

- [ ] **Step 8: cargo check and cargo test**

```bash
cargo check --manifest-path crates/orkworksd/Cargo.toml
```

Expected: success.

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: all tests pass (Ollama-specific tests will attempt real HTTP calls to 127.0.0.1:11434 and fail gracefully if no Ollama is running — that's expected).

- [ ] **Step 9: Commit**

```bash
git add crates/orkworksd/src/providers.rs
git commit -m "feat(ollama): HTTP model listing, inference runner, CompositeRunner dispatch"
```

---

### Task 5: Add Ollama base URL input to SettingsModal UI

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx:42-47,188-196,244-271`

- [ ] **Step 1: Add ollamaBaseUrl to state in SettingsModal**

After line 46 (`const [peonModelDraft, setPeonModelDraft] = useState<string | null>(initialSettings.providers.peonModel);`), add:
```tsx
  const [ollamaBaseUrlDraft, setOllamaBaseUrlDraft] = useState<string>(initialSettings.providers.ollamaBaseUrl);
```

- [ ] **Step 2: Add Ollama base URL input in the Provider Settings section**

After the Peon Model card (lines 251-271), add:
```tsx
            <div className="provider-card">
              <div className="provider-label">Ollama Base URL</div>
              <input
                className="provider-model-select"
                type="text"
                placeholder="http://127.0.0.1:11434"
                value={ollamaBaseUrlDraft}
                onChange={(e) => setOllamaBaseUrlDraft(e.target.value.trim())}
                onBlur={() => {
                  const normalized = ollamaBaseUrlDraft.trim().replace(/\/+$/, "");
                  if (normalized !== providerDraft.ollamaBaseUrl && (normalized.startsWith("http://") || normalized.startsWith("https://"))) {
                    const next = { ...providerDraft, ollamaBaseUrl: normalized };
                    setProviderDraft(next);
                    persistProviderSettings(next);
                  }
                }}
              />
            </div>
```

- [ ] **Step 3: Update savePeonModel to preserve ollamaBaseUrl**

In `savePeonModel` (lines 165-170), ensure the next settings include the current ollamaBaseUrl:
```tsx
  async function savePeonModel(model: string | null) {
    setProviderSaveStatus(null);
    const next = { ...providerDraft, peonModel: model };
    setProviderDraft(next);
    await persistProviderSettings(next);
  }
```
(This already uses `...providerDraft` so ollamaBaseUrl is preserved — no change needed.)

- [ ] **Step 4: tsc check**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/components/SettingsModal.tsx
git commit -m "feat(ollama): add Ollama base URL input to SettingsModal"
```

---

### Task 6: Run full verification

**Files:** none (verification only)

- [ ] **Step 1: Run Rust tests**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: all tests pass. Ollama-specific tests will attempt real HTTP to localhost:11434 — if Ollama is not running, `list_models("ollama")` returns an error (connection refused) which the test expects.

- [ ] **Step 2: Run frontend tests**

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: all tests pass.

- [ ] **Step 3: TypeScript type-check**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 4: Run doc-check**

```bash
bash .claude/hooks/doc-check.sh
```

- [ ] **Step 5: Update issue #48 checklist**

Mark completed items in the issue. Any remaining:
- AC7 (pass-through validation on 1-2 harnesses) — infrastructure is ready, but actual validation requires running harnesses with Ollama models. This can be done manually post-merge.
- AC8 (document gaps) — no gaps expected in code; manual validation results should be documented.

- [ ] **Step 6: Commit any final docs**

```bash
git add docs/superpowers/plans/2026-06-23-ollama-provider-wiring.md
git commit -m "docs: add Ollama provider wiring implementation plan"
```
