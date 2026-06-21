# Provider Ops and Peon Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an app-wide Providers control surface that persists Peon provider defaults and overrides, syncs them into the Rust sidecar, and lets Peon fall back from a failed primary provider instead of failing hard on OpenCode.

**Architecture:** Keep durable provider preferences in the existing Electron `settings.json`, with one `default` state and one optional `override` per provider. Move Peon execution choice into a small Rust provider registry plus a scope-aware fallback runner that records live runtime state and exposes it over `/providers`. The renderer merges Electron settings with live backend state, renders the existing `capacity` panel as `Providers`, and keeps the Settings modal as a thin entry point into the panel.

**Tech Stack:** Electron main/preload IPC, React 19 + TypeScript + Dockview, Rust + Axum + Tokio + Serde, Node built-in test runner, Cargo tests

---

## Execution Notes

- Existing user edits are already present in `apps/desktop/package.json`, `apps/desktop/pnpm-lock.yaml`, `apps/desktop/src/App.css`, `apps/desktop/src/components/SessionListPanel.tsx`, `apps/desktop/src/components/SettingsModal.tsx`, and `crates/orkworksd/src/main.rs`. Preserve them while implementing this plan.
- Keep the internal Dockview/menu panel id as `capacity`. Change only visible labels and rendered content to `Providers`.
- Keep the first pass narrow: app-wide settings only, no repo-scoped provider preferences, no auto-expiring overrides, no run-now button, no provider marketplace.
- The current tree only wires the session Peon loop. Implement the fallback runner with a shared `PeonScope` enum and wire the existing session caller first; do not invent a second repo-Peon loop in this feature.

## File Structure

### Create

- `docs/adr/0015-provider-ops-peon-fallback.md` — records the architectural decision to move Peon provider choice into main-process settings plus sidecar-owned runtime fallback.
- `apps/desktop/src/providerTypes.ts` — shared TypeScript unions and interfaces for provider ids, states, persisted settings, and `/providers` responses.
- `apps/desktop/electron/providerSettingsSync.ts` — main-process helper that pushes saved provider settings to the sidecar on startup, workspace switch, and save.
- `apps/desktop/tests/providerSettingsSync.test.ts` — unit tests for startup/reconnect/save sync behavior without driving the full Electron process.
- `apps/desktop/src/providerPresentation.ts` — pure renderer helpers for effective state derivation, fallback-order sorting, stale-revision checks, and view-model shaping.
- `apps/desktop/tests/providersPanel.test.ts` — Node tests for provider presentation helpers and server-side rendered panel output.
- `crates/orkworksd/src/providers.rs` — fixed provider registry, applied-settings store, runtime state, fallback runner, and `/providers` response types.

### Modify

- `docs/adr/README.md` — add ADR 0015 to the index.
- `README.md` — document the new Providers panel and app-wide provider fallback behavior.
- `AGENTS.md` — note the new ADR and provider-settings behavior so agent docs stay current.
- `docs/agents/architecture.md` — describe the new provider-settings sync and `/providers` data flow.
- `apps/desktop/src/appSettingsTypes.ts` — extend the renderer settings contract with provider settings.
- `apps/desktop/src/api.ts` — add provider runtime response types and `getProviders`.
- `apps/desktop/src/App.tsx` — load app settings eagerly, poll `/providers`, save provider settings, and pass provider state into Dockview/Settings.
- `apps/desktop/src/components/DockviewApp.tsx` — pass provider data through context and rename the visible panel title to `Providers`.
- `apps/desktop/src/components/CapacityPanel.tsx` — replace the placeholder with the real Providers panel UI.
- `apps/desktop/src/components/SettingsModal.tsx` — add the thin Providers entry point button.
- `apps/desktop/src/orkworksWindow.d.ts` — type new preload APIs for provider settings saves.
- `apps/desktop/electron/settingsMemory.ts` — normalize, persist, and default provider settings in `settings.json`.
- `apps/desktop/electron/main.ts` — expose `save-provider-settings`, trigger startup/reconnect sync, and keep current settings in memory.
- `apps/desktop/electron/preload.ts` — expose `saveProviderSettings`.
- `apps/desktop/electron/menuTemplate.ts` — rename visible `Capacity` labels to `Providers` while preserving the `capacity` id and hotkey slot.
- `apps/desktop/tests/electronSettingsMemory.test.ts` — cover provider-settings defaults, normalization, ordering, and revision behavior.
- `apps/desktop/tests/dockview.test.ts` — cover the visible Providers label, modal entry point, and shared panel wiring.
- `crates/orkworksd/src/peon.rs` — expose prompt/parsing helpers needed by the provider runner and retire single-harness-only execution.
- `crates/orkworksd/src/main.rs` — register `providers` module, add `/providers` and `/settings/providers`, store provider runtime state, and route the session Peon loop through the fallback runner.

## Task 1: Record the Architecture Decision First

**Files:**
- Create: `docs/adr/0015-provider-ops-peon-fallback.md`
- Modify: `docs/adr/README.md`

- [ ] **Step 1: Verify that ADR 0015 is the next free slot**

Run: `rtk rg -n "0015|provider-ops-peon-fallback" docs/adr docs/adr/README.md`

Expected: no matches for `0015-provider-ops-peon-fallback`.

- [ ] **Step 2: Write the ADR before touching implementation code**

```md
# Provider ops panel and app-wide Peon fallback

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-06-21

## Context

Peon currently executes a single harness command, defaulting to OpenCode. When OpenCode is capped or otherwise unavailable, Peon fails silently and the app loses the observer that should have explained the failure. The desktop app also lacks one place to inspect provider availability, set fallback order, or apply a manual cap override.

## Decision

Store provider preferences as app-wide Electron settings with one `defaultState` and one optional `overrideState` per provider. Keep executable provider definitions in the Rust sidecar, where a fixed registry owns labels, argv conventions, timeout policy, and runtime error classification. Expose live provider runtime state over `/providers`, and render the existing `capacity` panel slot as a Providers operations surface.

## Consequences

- Peon can fall back from one provider to another without storing arbitrary commands in user settings.
- The app gets a single provider-control model that future recommendation work can reuse.
- Electron and the sidecar must stay in sync on saved provider revisions, so startup and reconnect flows now include a settings push.
```

- [ ] **Step 3: Add ADR 0015 to the index**

```md
| [0015](./0015-provider-ops-peon-fallback.md) | Provider ops panel and app-wide Peon fallback | accepted |
```

- [ ] **Step 4: Commit the ADR change immediately**

```bash
git add docs/adr/0015-provider-ops-peon-fallback.md docs/adr/README.md
git commit -m "docs: record provider ops fallback architecture"
```

## Task 2: Add Durable Provider Settings to Electron Settings

**Files:**
- Create: `apps/desktop/src/providerTypes.ts`
- Modify: `apps/desktop/src/appSettingsTypes.ts`
- Modify: `apps/desktop/electron/settingsMemory.ts`
- Test: `apps/desktop/tests/electronSettingsMemory.test.ts`

- [ ] **Step 1: Write the failing settings-memory tests**

```ts
test("settings memory seeds default provider settings", () => {
  const settings = readSettings(dir);
  assert.deepEqual(settings.providers, {
    version: 1,
    revision: 0,
    providers: [
      {
        id: "opencode",
        enabled: true,
        fallbackOrder: 0,
        peonModel: null,
        defaultState: "healthy",
        overrideState: null,
      },
      {
        id: "claude-code",
        enabled: true,
        fallbackOrder: 1,
        peonModel: null,
        defaultState: "unknown",
        overrideState: null,
      },
    ],
  });
});

test("settings memory normalizes malformed provider payloads", () => {
  writeFileSync(
    settingsPath(dir),
    JSON.stringify({
      version: 1,
      providers: {
        version: 99,
        revision: 4.7,
        providers: [
          { id: "claude-code", enabled: "yes", fallbackOrder: -10, peonModel: 42, defaultState: "bad", overrideState: "capped" },
          { id: "unknown-provider", enabled: true, fallbackOrder: 0, peonModel: null, defaultState: "healthy", overrideState: null },
        ],
      },
    }),
  );

  const settings = readSettings(dir);
  assert.equal(settings.providers.version, 1);
  assert.equal(settings.providers.revision, 4);
  assert.deepEqual(settings.providers.providers.map((entry) => entry.id), ["claude-code", "opencode"]);
  assert.equal(settings.providers.providers[0].enabled, true);
  assert.equal(settings.providers.providers[0].fallbackOrder, 0);
  assert.equal(settings.providers.providers[0].peonModel, null);
  assert.equal(settings.providers.providers[0].defaultState, "unknown");
  assert.equal(settings.providers.providers[0].overrideState, "capped");
});

test("settings memory preserves provider revisions and canonical fallback order on write", () => {
  writeSettings(dir, {
    ...DEFAULT_SETTINGS,
    providers: {
      version: 1,
      revision: 7,
      providers: [
        { id: "claude-code", enabled: true, fallbackOrder: 9, peonModel: "sonnet", defaultState: "healthy", overrideState: null },
        { id: "opencode", enabled: false, fallbackOrder: 2, peonModel: null, defaultState: "capped", overrideState: null },
      ],
    },
  });

  const persisted = JSON.parse(readFileSync(settingsPath(dir), "utf8"));
  assert.equal(persisted.providers.revision, 7);
  assert.deepEqual(
    persisted.providers.providers.map((entry: { id: string; fallbackOrder: number }) => [entry.id, entry.fallbackOrder]),
    [["opencode", 0], ["claude-code", 1]],
  );
});
```

- [ ] **Step 2: Run the settings-memory tests to confirm the gap**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts`

Expected: FAIL because `AppSettings` has no `providers` section and normalization helpers do not exist yet.

- [ ] **Step 3: Add shared provider types and settings defaults**

```ts
// apps/desktop/src/providerTypes.ts
export type ProviderId = "opencode" | "claude-code";
export type ProviderCapacityState = "healthy" | "degraded" | "capped" | "unknown";
export type ProviderEffectiveState = ProviderCapacityState | "disabled";

export interface ProviderSettingsEntry {
  id: ProviderId;
  enabled: boolean;
  fallbackOrder: number;
  peonModel: string | null;
  defaultState: ProviderCapacityState;
  overrideState: ProviderCapacityState | null;
}

export interface ProviderSettings {
  version: 1;
  revision: number;
  providers: ProviderSettingsEntry[];
}

export interface ProviderApplyStatus {
  appliedRevision: number | null;
  appliedAt: string | null;
  lastApplyError: string | null;
}
```

```ts
// apps/desktop/electron/settingsMemory.ts
export const DEFAULT_PROVIDER_SETTINGS: ProviderSettings = {
  version: 1,
  revision: 0,
  providers: [
    {
      id: "opencode",
      enabled: true,
      fallbackOrder: 0,
      peonModel: null,
      defaultState: "healthy",
      overrideState: null,
    },
    {
      id: "claude-code",
      enabled: true,
      fallbackOrder: 1,
      peonModel: null,
      defaultState: "unknown",
      overrideState: null,
    },
  ],
};

export function normalizeProviderSettings(value: unknown): ProviderSettings {
  const raw = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  const entries = Array.isArray(raw.providers) ? raw.providers : [];
  const normalizedById = new Map<ProviderId, ProviderSettingsEntry>();

  for (const entry of entries) {
    if (!entry || typeof entry !== "object") continue;
    const candidate = normalizeProviderEntry(entry as Record<string, unknown>);
    if (candidate) normalizedById.set(candidate.id, candidate);
  }

  for (const defaultEntry of DEFAULT_PROVIDER_SETTINGS.providers) {
    if (!normalizedById.has(defaultEntry.id)) normalizedById.set(defaultEntry.id, { ...defaultEntry });
  }

  const providers = Array.from(normalizedById.values())
    .sort((a, b) => a.fallbackOrder - b.fallbackOrder || a.id.localeCompare(b.id))
    .map((entry, index) => ({ ...entry, fallbackOrder: index }));

  return {
    version: 1,
    revision: clampInt(raw.revision, 0, Number.MAX_SAFE_INTEGER, DEFAULT_PROVIDER_SETTINGS.revision),
    providers,
  };
}
```

```ts
// apps/desktop/src/appSettingsTypes.ts
import type { ProviderSettings } from "./providerTypes";

export interface AppSettings {
  [key: string]: unknown;
  version: 1;
  hotkeys: HotkeySettings;
  defaultHotkeys: HotkeySettings;
  retention: RetentionSettings;
  providers: ProviderSettings;
}
```

- [ ] **Step 4: Run the settings-memory tests again**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts`

Expected: PASS for the new provider-settings cases and the existing hotkey/retention cases.

- [ ] **Step 5: Commit the durable settings work**

```bash
git add apps/desktop/src/providerTypes.ts apps/desktop/src/appSettingsTypes.ts apps/desktop/electron/settingsMemory.ts apps/desktop/tests/electronSettingsMemory.test.ts
git commit -m "feat: persist provider settings"
```

## Task 3: Add Main-Process Provider Sync and Save IPC

**Files:**
- Create: `apps/desktop/electron/providerSettingsSync.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Test: `apps/desktop/tests/providerSettingsSync.test.ts`

- [ ] **Step 1: Write the failing sync tests**

```ts
test("pushProviderSettings posts saved settings to the sidecar and records success", async () => {
  const calls: Array<{ url: string; body: unknown }> = [];
  const fetchImpl = async (url: string, init?: RequestInit) => {
    calls.push({ url, body: JSON.parse(String(init?.body)) });
    return new Response(JSON.stringify({ appliedRevision: 3, appliedAt: "2026-06-21T10:00:00Z", lastApplyError: null }), { status: 200 });
  };

  const result = await pushProviderSettings("http://127.0.0.1:4444", sampleProviderSettings(3), fetchImpl);

  assert.equal(result.appliedRevision, 3);
  assert.equal(result.lastApplyError, null);
  assert.deepEqual(calls.map((call) => call.url), ["http://127.0.0.1:4444/settings/providers"]);
});

test("pushProviderSettings keeps the last error on non-fatal sidecar failures", async () => {
  const fetchImpl = async () => new Response("boom", { status: 500 });

  const result = await pushProviderSettings("http://127.0.0.1:4444", sampleProviderSettings(9), fetchImpl);

  assert.equal(result.appliedRevision, null);
  assert.match(result.lastApplyError ?? "", /500/);
});
```

- [ ] **Step 2: Run the new sync tests**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/providerSettingsSync.test.ts`

Expected: FAIL because `providerSettingsSync.ts` and `pushProviderSettings` do not exist yet.

- [ ] **Step 3: Implement a small sync helper and wire it into Electron**

```ts
// apps/desktop/electron/providerSettingsSync.ts
import type { ProviderApplyStatus, ProviderSettings } from "../src/providerTypes";

export async function pushProviderSettings(
  baseUrl: string,
  settings: ProviderSettings,
  fetchImpl: typeof fetch = fetch,
): Promise<ProviderApplyStatus> {
  try {
    const response = await fetchImpl(`${baseUrl}/settings/providers`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(settings),
    });

    if (!response.ok) {
      return {
        appliedRevision: null,
        appliedAt: null,
        lastApplyError: `settings push failed: ${response.status}`,
      };
    }

    const payload = (await response.json()) as ProviderApplyStatus;
    return {
      appliedRevision: payload.appliedRevision,
      appliedAt: payload.appliedAt,
      lastApplyError: payload.lastApplyError,
    };
  } catch (error) {
    return {
      appliedRevision: null,
      appliedAt: null,
      lastApplyError: error instanceof Error ? error.message : "settings push failed",
    };
  }
}
```

```ts
// apps/desktop/electron/preload.ts
saveProviderSettings: (providers: ProviderSettings): Promise<{ ok: true; settings: AppSettings }> =>
  ipcRenderer.invoke("save-provider-settings", providers),
```

```ts
// apps/desktop/src/orkworksWindow.d.ts
saveProviderSettings: (providers: ProviderSettings) => Promise<{ ok: true; settings: AppSettings }>;
```

```ts
// apps/desktop/electron/main.ts
ipcMain.handle("save-provider-settings", async (_event, providers: ProviderSettings) => {
  const baseSettings = currentSettings ?? readSettings(app.getPath("userData"));
  const nextSettings: AppSettings = {
    ...baseSettings,
    version: 1,
    providers: normalizeProviderSettings({
      ...providers,
      revision: Math.max(baseSettings.providers.revision + 1, providers.revision),
    }),
  };

  writeSettings(app.getPath("userData"), nextSettings);
  currentSettings = nextSettings;

  const port = await portPromise;
  await pushProviderSettings(`http://127.0.0.1:${port}`, nextSettings.providers);

  return { ok: true, settings: rendererSettings(currentSettings) };
});
```

- [ ] **Step 4: Push saved provider settings on startup and after workspace-driven sidecar restarts**

```ts
// apps/desktop/electron/main.ts
async function syncSavedProviderSettings(): Promise<void> {
  const settings = currentSettings ?? readSettings(app.getPath("userData"));
  const port = await portPromise;
  const result = await pushProviderSettings(`http://127.0.0.1:${port}`, settings.providers);
  if (result.lastApplyError) {
    console.warn(`[main] failed to push provider settings: ${result.lastApplyError}`);
  }
}

portPromise.then(() => {
  syncSavedProviderSettings().catch(() => {
    // Non-fatal; the next save or reconnect retries.
  });
});
```

- [ ] **Step 5: Run the sync tests and the existing settings tests**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/providerSettingsSync.test.ts tests/electronSettingsMemory.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit the Electron sync work**

```bash
git add apps/desktop/electron/providerSettingsSync.ts apps/desktop/electron/main.ts apps/desktop/electron/preload.ts apps/desktop/src/orkworksWindow.d.ts apps/desktop/tests/providerSettingsSync.test.ts
git commit -m "feat: sync provider settings to sidecar"
```

## Task 4: Add the Sidecar Provider Registry, Runtime State, and Fallback Runner

**Files:**
- Create: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/src/peon.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/providers.rs`
- Test: `crates/orkworksd/src/peon.rs`

- [ ] **Step 1: Write the failing Rust tests for fallback behavior**

```rust
#[test]
fn skips_disabled_and_capped_providers_before_spawn() {
    let manager = ProviderManager::for_tests(
        sample_settings(vec![
            entry("opencode").enabled(false).default_state(ProviderCapacityState::Healthy),
            entry("claude-code").override_state(Some(ProviderCapacityState::Capped)),
        ]),
        registry_with(vec![
            fake_provider("opencode"),
            fake_provider("claude-code"),
        ]),
    );

    let result = manager.run_inference(PeonScope::Session, &["terminal line".to_string()]);

    assert!(result.inference.is_none());
    assert_eq!(result.attempts.len(), 2);
    assert_eq!(result.attempts[0].outcome, AttemptOutcome::SkippedDisabled);
    assert_eq!(result.attempts[1].outcome, AttemptOutcome::SkippedCapped);
}

#[test]
fn falls_back_to_second_provider_after_primary_quota_failure() {
    let manager = ProviderManager::for_tests(
        sample_settings(vec![
            entry("opencode"),
            entry("claude-code"),
        ]),
        registry_with(vec![
            fake_provider("opencode").stderr("usage limit reached, resets in 2h").exit_code(1),
            fake_provider("claude-code").stdout(r#"{"observedStatus":"working","confidence":0.9}"#),
        ]),
    );

    let result = manager.run_inference(PeonScope::Session, &["terminal line".to_string()]);

    assert!(result.inference.is_some());
    assert_eq!(result.runtime["opencode"].last_error_summary.as_deref(), Some("usage limit reached"));
    assert_eq!(result.runtime["opencode"].reset_hint.as_deref(), Some("resets in 2h"));
    assert_eq!(result.runtime["claude-code"].fallback_step, Some(2));
}
```

- [ ] **Step 2: Run the Rust test suite to confirm the missing backend**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml providers -- --nocapture`

Expected: FAIL because `providers.rs`, provider runtime types, and fallback logic do not exist.

- [ ] **Step 3: Add the provider registry and applied-settings store**

```rust
// crates/orkworksd/src/providers.rs
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapacityState {
    Healthy,
    Degraded,
    Capped,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderSettingsEntry {
    pub id: String,
    pub enabled: bool,
    #[serde(rename = "fallbackOrder")]
    pub fallback_order: usize,
    #[serde(rename = "peonModel")]
    pub peon_model: Option<String>,
    #[serde(rename = "defaultState")]
    pub default_state: ProviderCapacityState,
    #[serde(rename = "overrideState")]
    pub override_state: Option<ProviderCapacityState>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderSettingsPayload {
    pub version: u8,
    pub revision: u64,
    pub providers: Vec<ProviderSettingsEntry>,
}

#[derive(Clone, Debug)]
pub struct ProviderDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub command: &'static str,
    pub default_args: &'static [&'static str],
    pub model_arg_template: Option<&'static str>,
    pub supports_model: bool,
    pub timeout_secs: u64,
}

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
        },
        ProviderDefinition {
            id: "claude-code",
            label: "Claude Code",
            command: "claude",
            default_args: &["-p"],
            model_arg_template: Some("--model={model}"),
            supports_model: true,
            timeout_secs: 30,
        },
    ]
}
```

- [ ] **Step 4: Replace single-provider Peon execution with a scope-aware fallback runner**

```rust
// crates/orkworksd/src/providers.rs
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PeonScope {
    Session,
    Repo,
}

pub struct ProviderRunResult {
    pub inference: Option<peon::PeonInference>,
    pub winning_provider_id: Option<String>,
}

impl ProviderManager {
    pub fn run_inference(&self, scope: PeonScope, output: &[String]) -> ProviderRunResult {
        let prompt = peon::build_prompt(output);

        for (index, joined) in self.enabled_providers().into_iter().enumerate() {
            let effective = joined.effective_state();
            if !joined.settings.enabled {
                self.record_skip(&joined.definition.id, scope, AttemptOutcome::SkippedDisabled, index + 1);
                continue;
            }
            if effective == ProviderEffectiveState::Capped {
                self.record_skip(&joined.definition.id, scope, AttemptOutcome::SkippedCapped, index + 1);
                continue;
            }

            match self.invoke_provider(&joined, &prompt, scope, index + 1) {
                Ok(inference) => {
                    self.record_success(&joined.definition.id, scope, index + 1);
                    return ProviderRunResult {
                        inference: Some(inference),
                        winning_provider_id: Some(joined.definition.id.to_string()),
                    };
                }
                Err(error) => {
                    self.record_failure(&joined.definition.id, scope, index + 1, &error);
                }
            }
        }

        ProviderRunResult { inference: None, winning_provider_id: None }
    }
}
```

```rust
// crates/orkworksd/src/peon.rs
pub fn build_prompt(output: &[String]) -> String { /* move current private function to pub */ }

pub fn parse_inference(stdout: &str) -> Option<PeonInference> {
    let json_str = extract_json(stdout)?;
    let inference: PeonInference = serde_json::from_str(&json_str).ok()?;
    validate_inference(&inference).ok()?;
    Some(inference)
}
```

- [ ] **Step 5: Add `/settings/providers` and `/providers`, then route the session Peon loop through the new manager**

```rust
// crates/orkworksd/src/main.rs
mod providers;

struct AppState {
    sessions: Mutex<HashMap<String, SessionHandle>>,
    workspace: Mutex<Option<WorkspaceState>>,
    peon: PeonState,
    providers: providers::ProviderManager,
    adapters: HashMap<String, harness::HarnessAdapter>,
    retention_config: tokio::sync::RwLock<RetentionConfig>,
}

let app = Router::new()
    .route("/health", get(health_check))
    .route("/providers", get(get_providers))
    .route("/settings/providers", post(set_provider_settings))
    .route("/workspace", post(set_workspace))
    .route("/workspace/active-session", post(set_active_session))
    .route("/sessions", post(create_session))
    .route("/sessions", get(list_sessions))
    .route("/sessions/:id", delete(delete_session))
    .route("/sessions/:id/forget", delete(forget_session))
    .route("/sessions/:id/resume", post(resume_session))
    .route("/settings/retention", post(set_retention))
    .route("/sessions/:id/terminal", get(session_terminal_handler))
    .route("/sessions/:id/terminal-output", get(get_terminal_output))
    .layer(cors)
    .with_state(state);
```

```rust
// crates/orkworksd/src/main.rs inside peon_loop
let provider_result = state_clone.providers.run_inference(providers::PeonScope::Session, &output_snapshot);
let inference = provider_result.inference;
```

- [ ] **Step 6: Run the focused Rust tests, then the full sidecar suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml providers peon -- --nocapture`

Expected: PASS for provider ordering, skip logic, quota/reset-hint capture, and prompt/parsing helpers.

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS.

- [ ] **Step 7: Commit the sidecar fallback work**

```bash
git add crates/orkworksd/src/providers.rs crates/orkworksd/src/peon.rs crates/orkworksd/src/main.rs
git commit -m "feat: add sidecar provider fallback for peon"
```

## Task 5: Build the Providers Panel and Renderer Data Flow

**Files:**
- Create: `apps/desktop/src/providerPresentation.ts`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/components/CapacityPanel.tsx`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/electron/menuTemplate.ts`
- Test: `apps/desktop/tests/providersPanel.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Write the failing renderer tests**

```ts
test("deriveEffectiveState prefers disabled, then override, then default", () => {
  assert.equal(deriveEffectiveState({ enabled: false, defaultState: "healthy", overrideState: null }), "disabled");
  assert.equal(deriveEffectiveState({ enabled: true, defaultState: "healthy", overrideState: "capped" }), "capped");
  assert.equal(deriveEffectiveState({ enabled: true, defaultState: "degraded", overrideState: null }), "degraded");
});

test("buildProviderViewModel sorts by fallback order and marks stale applied revisions", () => {
  const model = buildProviderViewModel(sampleSettings(), sampleRuntime({ appliedRevision: 1 }), "claude-code");
  assert.deepEqual(model.rows.map((row) => row.id), ["opencode", "claude-code"]);
  assert.equal(model.isStale, true);
  assert.equal(model.summary.currentProviderLabel, "Claude Code");
});

test("CapacityPanel renders Providers labels and runtime details", async () => {
  const html = renderToStaticMarkup(
    <CapacityPanel
      providerSettings={sampleSettings()}
      providerRuntime={sampleRuntime()}
      onSaveProviderSettings={async () => {}}
    />,
  );

  assert.match(html, /Providers/);
  assert.match(html, /Default/);
  assert.match(html, /Override/);
  assert.match(html, /Effective/);
  assert.match(html, /usage limit reached/);
  assert.match(html, /OpenCode/);
});
```

- [ ] **Step 2: Run the renderer tests to show the missing panel model**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/providersPanel.test.ts tests/dockview.test.ts`

Expected: FAIL because the provider presentation helpers, panel props, and visible Providers labels do not exist yet.

- [ ] **Step 3: Add pure presentation helpers and provider API types**

```ts
// apps/desktop/src/providerPresentation.ts
import type {
  ProviderCapacityState,
  ProviderEffectiveState,
  ProviderRuntimeResponse,
  ProviderSettings,
  ProviderSettingsEntry,
} from "./providerTypes";

export function deriveEffectiveState(entry: Pick<ProviderSettingsEntry, "enabled" | "defaultState" | "overrideState">): ProviderEffectiveState {
  if (!entry.enabled) return "disabled";
  return entry.overrideState ?? entry.defaultState;
}

export function sortProviderEntries(entries: ProviderSettingsEntry[]): ProviderSettingsEntry[] {
  return [...entries].sort((a, b) => a.fallbackOrder - b.fallbackOrder || a.id.localeCompare(b.id));
}

export function isAppliedRevisionStale(settings: ProviderSettings, runtime: ProviderRuntimeResponse | null): boolean {
  if (!runtime) return true;
  return runtime.appliedRevision !== settings.revision;
}
```

```ts
// apps/desktop/src/api.ts
export interface ProviderRuntimeEntry {
  id: string;
  label: string;
  effectiveState: ProviderEffectiveState;
  activePeonScopes: Array<"session" | "repo">;
  fallbackStep: number | null;
  lastAttemptAt: string | null;
  lastSuccessAt: string | null;
  lastErrorAt: string | null;
  lastErrorSummary: string | null;
  resetHint: string | null;
}

export interface ProviderRuntimeResponse {
  appliedRevision: number | null;
  appliedAt: string | null;
  lastApplyError: string | null;
  overallStatus: "healthy" | "unhealthy";
  currentProviderId: string | null;
  lastChainResult: string | null;
  providers: ProviderRuntimeEntry[];
}

export async function getProviders(baseUrl: string): Promise<ProviderRuntimeResponse> {
  const resp = await fetch(`${baseUrl}/providers`);
  if (!resp.ok) throw new Error(`get providers failed: ${resp.status}`);
  return resp.json();
}
```

- [ ] **Step 4: Load settings eagerly, poll `/providers`, and surface a real Providers panel**

```tsx
// apps/desktop/src/App.tsx
const [settings, setSettings] = useState<AppSettings | null>(null);
const [providerRuntime, setProviderRuntime] = useState<ProviderRuntimeResponse | null>(null);

useEffect(() => {
  window.orkworks.getSettings().then(setSettings).catch(() => {
    pushToast("error", "Couldn't load app settings.");
  });
}, []);

const refreshProviders = useCallback(async () => {
  try {
    const baseUrl = await window.orkworks.getBackendUrl();
    setProviderRuntime(await getProviders(baseUrl));
  } catch {
    // Silent polling failure; stale badge handles visibility.
  }
}, []);

async function saveProviderSettings(providers: ProviderSettings) {
  const result = await window.orkworks.saveProviderSettings(providers);
  setSettings(result.settings);
  await refreshProviders();
}
```

```tsx
// apps/desktop/src/components/DockviewApp.tsx
export const PANEL_DEFAULTS: Record<string, PanelDefault> = {
  terminal: { component: "terminal", title: "Terminal" },
  sessions: { component: "sessions", title: "Sessions", position: { referencePanel: "terminal", direction: "left" } },
  detail: { component: "detail", title: "Detail", position: { referencePanel: "sessions", direction: "below" } },
  capacity: { component: "capacity", title: "Providers", position: { referencePanel: "terminal", direction: "right" } },
  recommendations: { component: "recommendations", title: "Recommendations", position: { referencePanel: "capacity", direction: "below" } },
};
```

```tsx
// apps/desktop/src/components/SettingsModal.tsx
<div className="settings-section">
  <h3>Providers</h3>
  <p className="settings-section-copy">
    Provider defaults, overrides, fallback order, and Peon models live in the Providers panel.
  </p>
  <button type="button" onClick={onOpenProviders}>
    Open Providers Panel
  </button>
</div>
```

```ts
// apps/desktop/electron/menuTemplate.ts
const panelTitles: Record<(typeof panelIds)[number], string> = {
  sessions: "Sessions",
  detail: "Detail",
  terminal: "Terminal",
  capacity: "Providers",
  recommendations: "Recommendations",
};
```

- [ ] **Step 5: Replace the placeholder content with the real Providers UI**

```tsx
// apps/desktop/src/components/CapacityPanel.tsx
function CapacityPanel({ providerSettings, providerRuntime, onSaveProviderSettings }: CapacityPanelProps) {
  const viewModel = buildProviderViewModel(providerSettings, providerRuntime);

  return (
    <section className="providers-panel">
      <header className="providers-summary">
        <div>
          <h2>Providers</h2>
          <p>App-wide defaults and overrides for Peon fallback.</p>
        </div>
        <div className={`providers-health providers-health--${viewModel.summary.overallStatus}`}>
          {viewModel.summary.overallStatus}
        </div>
      </header>

      {viewModel.isStale && (
        <div className="providers-stale-banner">
          Saved settings revision {providerSettings.revision} is not yet applied to the sidecar.
        </div>
      )}

      {viewModel.rows.map((row) => (
        <article key={row.id} className="provider-card">
          <header>
            <h3>{row.label}</h3>
            <span>Step {row.fallbackOrder + 1}</span>
          </header>
          <div>Default: {row.defaultState}</div>
          <div>Override: {row.overrideState ?? "none"}</div>
          <div>Effective: {row.effectiveState}</div>
          <div>Model: {row.peonModel ?? "default"}</div>
          <div>Last error: {row.lastErrorSummary ?? "none"}</div>
          <div>Reset hint: {row.resetHint ?? "none"}</div>
          <button type="button" onClick={() => onMove(row.id, "up")} disabled={row.fallbackOrder === 0}>Move up</button>
          <button type="button" onClick={() => onMove(row.id, "down")} disabled={row.fallbackOrder === viewModel.rows.length - 1}>Move down</button>
          <button type="button" onClick={() => onClearOverride(row.id)} disabled={!row.overrideState}>Clear override</button>
        </article>
      ))}
    </section>
  );
}
```

- [ ] **Step 6: Run the renderer test suite**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/providersPanel.test.ts tests/dockview.test.ts tests/electronSettingsMemory.test.ts tests/providerSettingsSync.test.ts`

Expected: PASS.

- [ ] **Step 7: Commit the renderer work**

```bash
git add apps/desktop/src/providerPresentation.ts apps/desktop/src/api.ts apps/desktop/src/App.tsx apps/desktop/src/components/DockviewApp.tsx apps/desktop/src/components/CapacityPanel.tsx apps/desktop/src/components/SettingsModal.tsx apps/desktop/electron/menuTemplate.ts apps/desktop/tests/providersPanel.test.ts apps/desktop/tests/dockview.test.ts
git commit -m "feat: add providers operations panel"
```

## Task 6: Finish the Docs, Verify Everything, and Close the Loop

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/agents/architecture.md`

- [ ] **Step 1: Update the user and agent docs**

```md
<!-- README.md -->
- Providers panel: app-wide Peon provider order, manual overrides, and last runtime errors.
- If OpenCode is capped, Peon can fall back to the next enabled provider instead of failing hard.
```

```md
<!-- AGENTS.md -->
- ADR 0015 records the provider ops panel and app-wide Peon fallback model.
- The Dockview `capacity` slot now renders the visible `Providers` panel; keep the internal id stable for layout compatibility.
```

```md
<!-- docs/agents/architecture.md -->
- Electron settings now push both retention and provider settings into the sidecar after port discovery.
- The sidecar exposes `GET /providers` for live provider runtime state and `POST /settings/providers` for durable settings application.
```

- [ ] **Step 2: Type-check the desktop app**

Run: `cd apps/desktop && npx tsc --noEmit`

Expected: PASS.

- [ ] **Step 3: Run the frontend tests together**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts tests/providerSettingsSync.test.ts tests/providersPanel.test.ts tests/dockview.test.ts`

Expected: PASS.

- [ ] **Step 4: Run the Rust tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Run the required doc currency check**

Run: `bash .claude/hooks/doc-check.sh`

Expected: no unaddressed doc warnings.

- [ ] **Step 6: Commit the final docs and verification pass**

```bash
git add README.md AGENTS.md docs/agents/architecture.md
git commit -m "docs: document providers panel and peon fallback"
```

## Self-Review

### Spec Coverage

- The plan covers the app-wide-only control model through `ProviderSettings` in Electron and the Providers panel in the renderer.
- The plan covers `default`, `override`, and derived `effective` state explicitly in both the persisted schema and renderer helpers.
- The plan covers ordered fallback, skip-before-spawn for `enabled=false` and `capped`, structured runtime errors, and reset hints in the Rust provider manager.
- The plan covers startup/reconnect sync with `pushProviderSettings`, `/settings/providers`, and the stale applied-revision banner from `/providers`.
- The plan covers the thin Settings entry point and the visible rename from `Capacity` to `Providers` while preserving internal panel id `capacity`.
- The plan covers the required docs work, including the ADR, README, AGENTS, architecture doc, and `doc-check.sh`.

### Placeholder Scan

- No `TBD`, `TODO`, or “similar to task N” placeholders remain.
- Every implementation task names exact files, concrete commands, and concrete code shapes.
- Every verification step has an explicit command and expected result.

### Type Consistency

- Provider ids are consistently `opencode | claude-code` in the TypeScript and Rust snippets.
- Persisted settings consistently use `defaultState`, `overrideState`, `fallbackOrder`, and `peonModel`.
- Runtime response consistently uses `appliedRevision`, `appliedAt`, `lastApplyError`, `activePeonScopes`, `fallbackStep`, `lastErrorSummary`, and `resetHint`.
- `capacity` remains the internal panel id everywhere; `Providers` is the visible label everywhere.

### Intentional Constraint

- The current codebase only exposes the session Peon loop. This plan still builds a shared `PeonScope`-aware fallback runner and runtime response so the repo Peon path can call the same code when that caller exists, but it does not expand this feature into building a second repo-scope loop from scratch.
