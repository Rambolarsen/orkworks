# Peon Model Picker Design

- Date: 2026-06-22
- Status: proposed

## Summary

Add per-provider peon model selection to the Settings modal. Users pick a model for each provider from a dropdown populated by a sidecar endpoint that queries each provider's CLI for available models. Models are cached in the Electron main process at startup and served from memory.

## Problem

Peon runs inference through providers (opencode, claude-code) and each provider needs a model string passed as a CLI argument (`--model={model}`). Currently the `peonModel` field exists in the data model (`ProviderSettingsEntry.peonModel`) but the UI has no way to set it — `ProviderSettingsSection` displays it read-only and isn't rendered anywhere. Users cannot choose which model peon should use per provider.

## Design Goals

- Let users configure `peonModel` per provider from the Settings modal
- Populate model options from the provider's own CLI (not hardcoded)
- Cache model lists eagerly in the Electron main process
- Keep the Settings modal as the single settings surface
- Preserve the existing provider fallback architecture

## Sidecar Changes

### New endpoint: `GET /providers/:id/models`

Returns available models for a provider by running its model-listing command.

**Response (200):**
```json
{ "models": ["claude-sonnet-4-20250514", "claude-opus-4-20250514", "gpt-4o"] }
```

**Errors:**
- `404` if provider ID not in the built-in registry
- `500` if the model-listing command fails, with `{ "error": "<message>" }`

### ProviderDefinition extensions

Add two optional fields to `ProviderDefinition` in `providers.rs`:

```rust
pub struct ProviderDefinition {
    // ... existing fields ...
    pub list_models_command: Option<&'static str>,   // e.g., "opencode" (use same command, different args)
    pub list_models_args: &'static [&'static str],   // e.g., ["list-models", "--json"]
}
```

When both fields are `Some`, the handler runs the command and parses its stdout as a JSON array or newline-delimited model names. When `None`, the endpoint returns an empty model list with 200 (provider exists but doesn't support model listing).

The model-listing commands for each provider must be verified against the actual provider CLIs during implementation. Placeholder values:
- `opencode`: `list_models_command: Some("opencode")`, `list_models_args: &["list-models"]`
- `claude-code`: `list_models_command: Some("claude")`, `list_models_args: &["models", "--list"]`

## Frontend UI Changes

### SettingsModal

A new **Providers** section is added to the Settings modal, rendered **above Hotkeys** (first section). On modal open:
1. Fetch `ProviderSettings` via `window.orkworks.getSettings()`
2. Fetch models for each provider via `window.orkworks.getProviderModels(providerId)` (served from Electron cache)
3. Maintain a draft of `ProviderSettings` (same pattern as hotkeys draft)
4. Pass draft + models map into `ProviderSettingsSection`

### ProviderSettingsSection

The existing component is modified:
- **Model field**: The read-only `<div>Model: {row.peonModel ?? "default"}</div>` is replaced with a `<select>` dropdown
- Dropdown options:
  - First option: `"default"` (value `""`, no model arg passed to provider)
  - Remaining options: each model from the cached list
- `onChange` → updates draft immediately + calls `saveProviderSettings(draft)` for auto-save
- Existing reorder (up/down), enable/disable, and state override controls remain unchanged

## IPC and Preload Bridge

### New IPC channel: `get-provider-models`

| Channel | Exposed as | Payload |
|---|---|---|
| `get-provider-models` | `window.orkworks.getProviderModels(providerId)` | Takes `string`, returns `{ models: string[] }` |

### Electron handler

- Cached in memory: `Map<string, string[]>` (provider ID → model list)
- Populated at startup: fetches from sidecar `GET /providers/:id/models` for each registered provider
- Refreshed on workspace switch (alongside `syncSavedProviderSettings`)
- IPC handler returns from cache directly — no sidecar proxy on every renderer call
- Cache misses (unknown provider ID) return `{ models: [] }`

### Renderer type extensions

`providerTypes.ts` gains:
```typescript
export interface ProviderModelsResponse {
  models: string[];
}
```

`orkworksWindow.d.ts` gains:
```typescript
getProviderModels(providerId: string): Promise<ProviderModelsResponse>;
```

## Component State Flow

```
Settings modal opens
  ├─ getSettings() → ProviderSettings
  ├─ getProviderModels("opencode") → cached models
  └─ getProviderModels("claude-code") → cached models
        │
        ▼
  ProviderSettingsSection renders
    ├─ opencode card: <select> with models
    └─ claude-code card: <select> with models
        │
        ▼
  User changes dropdown → onChange
    ├─ update draft
    └─ saveProviderSettings(draft) → sidecar POST /settings/providers
```

## ADR Impact

This design overrides ADR 0017 (removal of provider editing from Settings). Provider model selection is re-added to the Settings modal. The session-scoped read-only provider context in the Details panel remains unchanged.

## Testing

- **Sidecar**: Test `GET /providers/:id/models` with valid provider, unknown provider, command failure
- **Electron**: Test cache population at startup, cache serving, cache refresh on workspace switch
- **Frontend**: Test dropdown population, auto-save on change, default option behavior
- **Integration**: Test end-to-end flow: open Settings → pick model → save → verify peon uses the model

## Non-Goals

- Model catalog / discovery for session creation (still free-text input)
- Global peon model (still per-provider)
- Provider enable/disable exposed in Settings (these remain unchanged from ADR 0017)
- Adding or removing providers (provider registry remains hardcoded)
