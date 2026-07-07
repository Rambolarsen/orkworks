# Peon Ollama Settings Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Settings-visible Ollama verification flow for Peon that can verify both draft and saved URLs, surface concrete verification state, and show a visible list of Peon candidate models.

**Architecture:** Keep Ollama verification logic in the Rust sidecar as a dedicated settings endpoint so both saved and unsaved URLs flow through one code path. Electron main exposes a narrow IPC bridge to the renderer, and `SettingsModal` owns transient verification UI state without mutating persisted settings until the existing save path runs.

**Tech Stack:** Rust + Axum + reqwest on the sidecar, Electron IPC in `apps/desktop/electron`, React + TypeScript in `apps/desktop/src`, Node built-in test runner for desktop tests, `cargo test` for sidecar tests.

---

### Task 1: Add Rust Verification Primitives

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`
- Test: `crates/orkworksd/src/providers.rs`

- [ ] **Step 1: Write the failing Rust unit tests for URL normalization and model filtering**

```rust
#[test]
fn normalize_ollama_base_url_trims_and_strips_trailing_slash() {
    let normalized = normalize_ollama_base_url(" http://127.0.0.1:11434/ ").unwrap();
    assert_eq!(normalized, "http://127.0.0.1:11434");
}

#[test]
fn normalize_ollama_base_url_rejects_non_origin_urls() {
    let err = normalize_ollama_base_url("http://127.0.0.1:11434/api/tags").unwrap_err();
    assert!(err.contains("origin-only"));
}

#[test]
fn filter_peon_candidate_models_excludes_embedding_names_case_insensitively() {
    let (models, excluded) = filter_peon_candidate_models(vec![
        "llama3.1:latest".into(),
        "nomic-embed-text".into(),
        "BGE-EMBED-M3:latest".into(),
    ]);

    assert_eq!(models, vec!["llama3.1:latest"]);
    assert_eq!(excluded, vec!["BGE-EMBED-M3:latest", "nomic-embed-text"]);
}
```

- [ ] **Step 2: Run the targeted Rust tests and confirm they fail for missing symbols**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml normalize_ollama_base_url_`

Expected: FAIL with errors for `normalize_ollama_base_url` and `filter_peon_candidate_models` not existing yet.

- [ ] **Step 3: Add the shared verification types and helpers in `providers.rs`**

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OllamaVerificationStatus {
    Connected,
    ConnectedEmpty,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OllamaVerificationReasonCode {
    Connected,
    NoModelsReturned,
    AllModelsFiltered,
    InvalidUrl,
    Unreachable,
    Timeout,
    HttpError,
    ParseError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OllamaVerificationResponse {
    pub ok: bool,
    #[serde(rename = "normalizedBaseUrl")]
    pub normalized_base_url: String,
    pub status: OllamaVerificationStatus,
    #[serde(rename = "reasonCode")]
    pub reason_code: OllamaVerificationReasonCode,
    #[serde(rename = "httpStatus")]
    pub http_status: Option<u16>,
    pub models: Vec<String>,
    #[serde(rename = "excludedModels")]
    pub excluded_models: Vec<String>,
    pub diagnostic: Option<String>,
}

pub(crate) fn normalize_ollama_base_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| "invalid Ollama URL".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Ollama URL must start with http:// or https://".to_string());
    }
    if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("Ollama URL must be origin-only with no path, query, or fragment".to_string());
    }
    Ok(parsed.origin().unicode_serialization())
}

pub(crate) fn filter_peon_candidate_models(mut models: Vec<String>) -> (Vec<String>, Vec<String>) {
    models.sort();
    let (excluded, included): (Vec<_>, Vec<_>) = models
        .into_iter()
        .partition(|name| {
            let lower = name.to_ascii_lowercase();
            lower.contains("embed") || lower.contains("embedding")
        });
    (included, excluded)
}
```

- [ ] **Step 4: Re-run the targeted Rust tests and confirm they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml normalize_ollama_base_url_`

Expected: PASS

- [ ] **Step 5: Commit the helper/types slice**

```bash
git add crates/orkworksd/src/providers.rs
git commit -m "feat: add ollama verification primitives"
```

### Task 2: Add the Sidecar Verify Endpoint

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/src/http/provider_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/providers.rs`
- Test: `crates/orkworksd/src/http/provider_handlers.rs`

- [ ] **Step 1: Write failing tests for invalid input, reachable-empty success, and unreachable failure**

```rust
#[tokio::test]
async fn verify_ollama_returns_bad_request_for_invalid_url() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let response = verify_ollama_settings(
        State(state),
        axum::Json(OllamaVerifyRequest { base_url: "http://127.0.0.1:11434/api".into() }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[test]
fn verify_ollama_all_models_filtered_is_connected_empty() {
    let response = build_ollama_verification_response(
        "http://127.0.0.1:11434".into(),
        vec!["nomic-embed-text".into()],
    );

    assert!(response.ok);
    assert_eq!(response.status, OllamaVerificationStatus::ConnectedEmpty);
    assert!(response.models.is_empty());
    assert_eq!(response.reason_code, OllamaVerificationReasonCode::AllModelsFiltered);
}

#[test]
fn verify_ollama_unreachable_maps_to_failed_response() {
    let manager = ProviderManager::for_tests(
        ProviderSettingsPayload::default(),
        vec![],
    );

    let response = manager.verify_ollama("http://127.0.0.1:49999");
    assert!(!response.ok);
    assert_eq!(response.status, OllamaVerificationStatus::Failed);
    assert_eq!(response.reason_code, OllamaVerificationReasonCode::Unreachable);
}
```

- [ ] **Step 2: Run the focused Rust tests and verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml verify_ollama_`

Expected: FAIL because `verify_ollama_settings`, `OllamaVerifyRequest`, and `ProviderManager::verify_ollama` are not implemented yet.

- [ ] **Step 3: Implement `ProviderManager::verify_ollama`, the request payload, and the handler**

```rust
#[derive(Deserialize)]
pub struct OllamaVerifyRequest {
    #[serde(rename = "baseUrl")]
    pub base_url: String,
}

fn build_ollama_verification_response(
    normalized_base_url: String,
    raw_models: Vec<String>,
) -> OllamaVerificationResponse {
    let (models, excluded_models) = filter_peon_candidate_models(raw_models);
    let reason_code = if models.is_empty() {
        if excluded_models.is_empty() {
            OllamaVerificationReasonCode::NoModelsReturned
        } else {
            OllamaVerificationReasonCode::AllModelsFiltered
        }
    } else {
        OllamaVerificationReasonCode::Connected
    };
    let status = if models.is_empty() {
        OllamaVerificationStatus::ConnectedEmpty
    } else {
        OllamaVerificationStatus::Connected
    };

    OllamaVerificationResponse {
        ok: true,
        normalized_base_url,
        status,
        reason_code,
        http_status: Some(200),
        models,
        excluded_models,
        diagnostic: None,
    }
}

fn failed_ollama_verification(normalized_base_url: String, error: reqwest::Error) -> OllamaVerificationResponse {
    let (reason_code, diagnostic) = if error.is_connect() {
        (
            OllamaVerificationReasonCode::Unreachable,
            format!("Ollama endpoint unreachable at {normalized_base_url}"),
        )
    } else if error.is_timeout() {
        (
            OllamaVerificationReasonCode::Timeout,
            "Ollama request timed out".to_string(),
        )
    } else {
        (
            OllamaVerificationReasonCode::HttpError,
            format!("Ollama request failed: {error}"),
        )
    };

    OllamaVerificationResponse {
        ok: false,
        normalized_base_url,
        status: OllamaVerificationStatus::Failed,
        reason_code,
        http_status: None,
        models: vec![],
        excluded_models: vec![],
        diagnostic: Some(diagnostic),
    }
}

impl ProviderManager {
    pub fn verify_ollama(&self, base_url: &str) -> OllamaVerificationResponse {
        let normalized = normalize_ollama_base_url(base_url)
            .expect("verify_ollama caller must validate the URL first");
        let client = HttpClient::new();
        let url = format!("{normalized}/api/tags");

        let response = match block_on_http(async {
            tokio::time::timeout(Duration::from_secs(10), client.get(&url).send()).await
        }) {
            Ok(Ok(resp)) => resp,
            Ok(Err(err)) => return failed_ollama_verification(normalized, err),
            Err(_) => {
                return OllamaVerificationResponse {
                    ok: false,
                    normalized_base_url: normalized,
                    status: OllamaVerificationStatus::Failed,
                    reason_code: OllamaVerificationReasonCode::Timeout,
                    http_status: None,
                    models: vec![],
                    excluded_models: vec![],
                    diagnostic: Some("Ollama request timed out".into()),
                };
            }
        };

        let status = response.status();
        if !status.is_success() {
            let diagnostic = format!("Ollama returned HTTP {}", status.as_u16());
            return OllamaVerificationResponse {
                ok: false,
                normalized_base_url: normalized,
                status: OllamaVerificationStatus::Failed,
                reason_code: OllamaVerificationReasonCode::HttpError,
                http_status: Some(status.as_u16()),
                models: vec![],
                excluded_models: vec![],
                diagnostic: Some(diagnostic),
            };
        }

        let body = match block_on_http(response.text()) {
            Ok(text) => text,
            Err(error) => {
                return OllamaVerificationResponse {
                    ok: false,
                    normalized_base_url: normalized,
                    status: OllamaVerificationStatus::Failed,
                    reason_code: OllamaVerificationReasonCode::ParseError,
                    http_status: Some(status.as_u16()),
                    models: vec![],
                    excluded_models: vec![],
                    diagnostic: Some(format!("failed to read Ollama response: {error}")),
                };
            }
        };

        let tags: OllamaTagsResponse = match serde_json::from_str(&body) {
            Ok(parsed) => parsed,
            Err(error) => {
                return OllamaVerificationResponse {
                    ok: false,
                    normalized_base_url: normalized,
                    status: OllamaVerificationStatus::Failed,
                    reason_code: OllamaVerificationReasonCode::ParseError,
                    http_status: Some(status.as_u16()),
                    models: vec![],
                    excluded_models: vec![],
                    diagnostic: Some(format!("failed to parse Ollama /api/tags response: {error}")),
                };
            }
        };

        build_ollama_verification_response(
            normalized,
            tags.models.into_iter().map(|model| model.name).collect(),
        )
    }
}

pub(crate) async fn verify_ollama_settings(
    State(state): State<Arc<AppState>>,
    axum::Json(payload): axum::Json<providers::OllamaVerifyRequest>,
) -> impl IntoResponse {
    let normalized = match providers::normalize_ollama_base_url(&payload.base_url) {
        Ok(value) => value,
        Err(error) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(ErrorResponse { error }),
            )
                .into_response();
        }
    };

    let providers = state.providers.clone();
    match tokio::task::spawn_blocking(move || providers.verify_ollama(&normalized)).await {
        Ok(result) => axum::Json(result).into_response(),
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(ErrorResponse { error: "internal error".into() }),
        )
            .into_response(),
    }
}
```

- [ ] **Step 4: Wire the new route in `main.rs` and re-run the focused Rust tests**

```rust
use crate::http::provider_handlers::{
    get_provider_models,
    get_providers,
    set_provider_settings,
    verify_ollama_settings,
};

let app = Router::new()
    .route("/health", get(health_check))
    .route("/providers", get(get_providers))
    .route("/providers/:id/models", get(get_provider_models))
    .route("/settings/providers", post(set_provider_settings))
    .route("/settings/providers/ollama/verify", post(verify_ollama_settings))
    .route("/workspace", post(set_workspace));
```

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml verify_ollama_`

Expected: PASS

- [ ] **Step 5: Commit the sidecar endpoint slice**

```bash
git add crates/orkworksd/src/providers.rs crates/orkworksd/src/http/provider_handlers.rs crates/orkworksd/src/main.rs
git commit -m "feat: add ollama verification endpoint"
```

### Task 3: Add the Electron IPC Bridge and Shared Types

**Files:**
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Modify: `apps/desktop/src/providerTypes.ts`
- Create: `apps/desktop/tests/ollamaVerification.test.ts`

- [ ] **Step 1: Write failing desktop tests for the new type and bridge API**

```ts
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("provider types include an Ollama verification response", () => {
  const source = readFileSync(new URL("../src/providerTypes.ts", import.meta.url), "utf8");
  assert.match(source, /export interface OllamaVerificationResponse/);
  assert.match(source, /normalizedBaseUrl/);
  assert.match(source, /reasonCode/);
  assert.match(source, /excludedModels/);
});

test("preload and window typing expose verifyOllama", () => {
  const preload = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
  const types = readFileSync(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");
  assert.match(preload, /verifyOllama:\s*\(baseUrl: string\)/);
  assert.match(types, /verifyOllama:\s*\(baseUrl: string\)\s*=>\s*Promise<OllamaVerificationResponse>/);
});
```

- [ ] **Step 2: Run the desktop test file and confirm it fails**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/ollamaVerification.test.ts`

Expected: FAIL because `OllamaVerificationResponse` and `verifyOllama` do not exist yet.

- [ ] **Step 3: Implement the typed bridge in `providerTypes.ts`, `preload.ts`, `orkworksWindow.d.ts`, and `main.ts`**

```ts
export interface OllamaVerificationResponse {
  ok: boolean;
  normalizedBaseUrl: string;
  status: "connected" | "connected_empty" | "failed";
  reasonCode:
    | "connected"
    | "no_models_returned"
    | "all_models_filtered"
    | "invalid_url"
    | "unreachable"
    | "timeout"
    | "http_error"
    | "parse_error";
  httpStatus: number | null;
  models: string[];
  excludedModels: string[];
  diagnostic: string | null;
}
```

```ts
verifyOllama: (baseUrl: string): Promise<unknown> => ipcRenderer.invoke("verify-ollama", baseUrl),
```

```ts
ipcMain.handle("verify-ollama", async (_event, baseUrl: string) => {
  const port = await portPromise;
  const response = await fetch(`http://127.0.0.1:${port}/settings/providers/ollama/verify`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ baseUrl }),
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: "Couldn't verify Ollama." }));
    throw new Error(error.error ?? "Couldn't verify Ollama.");
  }
  return await response.json();
});
```

- [ ] **Step 4: Re-run the desktop bridge tests and confirm they pass**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/ollamaVerification.test.ts`

Expected: PASS

- [ ] **Step 5: Commit the bridge/types slice**

```bash
git add apps/desktop/electron/main.ts apps/desktop/electron/preload.ts apps/desktop/src/orkworksWindow.d.ts apps/desktop/src/providerTypes.ts apps/desktop/tests/ollamaVerification.test.ts
git commit -m "feat: expose ollama verification to settings"
```

### Task 4: Implement the Settings UI, Status Handling, and Candidate List

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/App.css`
- Modify: `apps/desktop/tests/peonModelPicker.test.ts`
- Modify: `apps/desktop/tests/providersPanel.test.ts`
- Modify: `apps/desktop/tests/ollamaVerification.test.ts`

- [ ] **Step 1: Add failing renderer tests for the verify button, status region, and candidate list**

```ts
test("SettingsModal renders verify affordance and status region for Ollama", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Verify Ollama/);
  assert.match(source, /role="status"/);
  assert.match(source, /window\.orkworks\.verifyOllama/);
});

test("SettingsModal renders a visible candidate model list with a use action", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Use this model/);
  assert.match(source, /ollama-candidate-list/);
  assert.match(source, /selected-model/);
});

test("SettingsModal guards against stale verification results", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /verifyRequestRef/);
  assert.match(source, /normalizedBaseUrl/);
});
```

- [ ] **Step 2: Run the renderer-focused tests and confirm they fail**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/peonModelPicker.test.ts tests/providersPanel.test.ts tests/ollamaVerification.test.ts`

Expected: FAIL because the new verification UI and stale-result guard do not exist yet.

- [ ] **Step 3: Implement verification state, manual verify, auto re-verify on saved URL change, and candidate selection in `SettingsModal.tsx`**

```tsx
type OllamaVerificationViewState =
  | { phase: "idle" }
  | { phase: "checking"; requestedBaseUrl: string }
  | { phase: "done"; result: OllamaVerificationResponse };

const [ollamaVerification, setOllamaVerification] = useState<OllamaVerificationViewState>({ phase: "idle" });
const verifyRequestRef = useRef(0);

async function verifyOllamaDraft(baseUrl: string) {
  const requestId = ++verifyRequestRef.current;
  setOllamaVerification({ phase: "checking", requestedBaseUrl: baseUrl });
  try {
    const result = await window.orkworks.verifyOllama(baseUrl);
    if (requestId !== verifyRequestRef.current) return;
    setOllamaVerification({ phase: "done", result });
  } catch (error) {
    if (requestId !== verifyRequestRef.current) return;
    setOllamaVerification({
      phase: "done",
      result: {
        ok: false,
        normalizedBaseUrl: baseUrl.trim().replace(/\/+$/, ""),
        status: "failed",
        reasonCode: "invalid_url",
        httpStatus: null,
        models: [],
        excludedModels: [],
        diagnostic: error instanceof Error ? error.message : "Couldn't verify Ollama.",
      },
    });
  }
}
```

```tsx
<div className="provider-card">
  <div className="provider-label">Ollama verification</div>
  <button
    type="button"
    onClick={() => verifyOllamaDraft(ollamaBaseUrlDraft)}
    disabled={ollamaVerification.phase === "checking"}
  >
    {ollamaVerification.phase === "checking" ? "Verifying…" : "Verify Ollama"}
  </button>
  <div role="status" aria-live="polite" className="provider-verify-status">
    {renderOllamaVerificationStatus(ollamaVerification)}
  </div>
  <ul className="ollama-candidate-list">
    {candidateModels.map((model) => (
      <li key={model}>
        <span className={model === peonModelDraft ? "selected-model" : undefined}>{model}</span>
        <button type="button" aria-label={`Use ${model} for Peon`} onClick={() => setPeonModelDraft(model)}>
          Use this model
        </button>
      </li>
    ))}
  </ul>
</div>
```

- [ ] **Step 4: Add the CSS for the bounded candidate list and re-run the renderer tests**

```css
.provider-verify-status {
  margin-top: 8px;
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.provider-verify-status--ok {
  color: var(--state-ok);
}

.provider-verify-status--error {
  color: var(--state-error);
}

.ollama-candidate-list {
  list-style: none;
  margin: 10px 0 0;
  padding: 0;
  max-height: 180px;
  overflow: auto;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
}

.selected-model {
  color: var(--text-primary);
  font-weight: 600;
}
```

Run: `cd apps/desktop && node --experimental-strip-types --test tests/peonModelPicker.test.ts tests/providersPanel.test.ts tests/ollamaVerification.test.ts`

Expected: PASS

- [ ] **Step 5: Commit the Settings UI slice**

```bash
git add apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/App.css apps/desktop/tests/peonModelPicker.test.ts apps/desktop/tests/providersPanel.test.ts apps/desktop/tests/ollamaVerification.test.ts
git commit -m "feat: show ollama verification in settings"
```

### Task 5: Update Architecture Notes and Run Full Verification

**Files:**
- Modify: `docs/agents/architecture.md`

- [ ] **Step 1: Update the architecture doc so the new endpoint and IPC bridge are current**

```md
`electron/settingsMemory.ts` owns app-level settings in Electron `userData`, including hotkey validation, default hotkeys, a persisted `debug.showSessionIds` flag for gating internal session identifiers in the Details panel, persisted menu accelerators, and durable provider settings (`ProviderSettings`). Electron settings now push both retention and provider settings into the sidecar after port discovery. Provider model lists are fetched from `GET /providers/:id/models` and cached in memory, while draft Ollama verification in Settings bypasses that cache through `POST /settings/providers/ollama/verify` so unsaved URLs can be checked before persistence.
```

```md
Key endpoints: `POST /workspace`, `POST /workspace/active-session`, `PUT /workspace/active-harnesses`, `GET /providers`, `GET /providers/:id/models`, `POST /settings/providers`, `POST /settings/providers/ollama/verify`, `POST /settings/retention`, `GET/POST /sessions`, `DELETE /sessions/:id`, `POST /sessions/:id/resume`, `GET/POST /harnesses`, `PUT/DELETE /harnesses/:id`, and `WS /sessions/:id/terminal`.
```

- [ ] **Step 2: Run the full verification commands**

```bash
cd apps/desktop && npx tsc --noEmit
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
cargo test --manifest-path crates/orkworksd/Cargo.toml
bash .claude/hooks/doc-check.sh
```

Expected:
- `npx tsc --noEmit`: PASS
- desktop node tests: PASS
- `cargo test`: PASS
- `doc-check.sh`: no required doc follow-up beyond the already-updated architecture note, or explicit flagged files addressed before handoff

- [ ] **Step 3: Commit the doc + verification sweep**

```bash
git add docs/agents/architecture.md
git commit -m "docs: record ollama verification settings flow"
```
