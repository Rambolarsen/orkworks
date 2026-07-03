# Harness Registry Unification

**Date:** 2026-07-03
**Status:** Approved

## Problem

Adding a new harness currently requires touching three separate places:

1. `builtin_harness_configs()` in `harness_registry.rs` — user-visible config (name, launch command, args, model prefix)
2. `builtin_adapters()` in `harness_registry.rs` — adapter type behavior (resume templates, capability flags, limit patterns)
3. `builtin_provider_registry()` in `providers.rs` — peon inference config (headless command/args, model listing, static models)

These three registries describe the same tools with overlapping fields (id, command). They must be kept in sync manually. There is no runtime enforcement.

Additionally, `HarnessAdapter.limit_patterns` is `&'static [&'static str]` — disk-configured harnesses can never specify usage-limit detection patterns.

## Decision

Keep the **instance / type** separation — it is the right model. Multiple instances can share one adapter type (e.g. a custom shell wrapping generic-shell). The problem is not the conceptual split; it is that the **type definition** is spread across two files and the **instance definition** is missing its peon config.

## Design

### 1. Peon config moves to the instance (`HarnessConfig`)

Add an optional `peon: Option<PeonConfig>` sub-struct to `HarnessConfig`. When present, the harness participates in peon inference. When absent, it does not.

```rust
// harness_registry.rs

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeonConfig {
    /// Overrides HarnessConfig.command for headless invocation (rarely needed).
    #[serde(rename = "commandOverride", skip_serializing_if = "Option::is_none")]
    pub command_override: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(rename = "modelArgTemplate", skip_serializing_if = "Option::is_none")]
    pub model_arg_template: Option<String>,
    #[serde(rename = "supportsModel", default)]
    pub supports_model: bool,
    #[serde(rename = "timeoutSecs", default = "default_peon_timeout")]
    pub timeout_secs: u64,
    #[serde(rename = "listModelsCommand", skip_serializing_if = "Option::is_none")]
    pub list_models_command: Option<String>,
    #[serde(rename = "listModelsArgs", default)]
    pub list_models_args: Vec<String>,
    #[serde(rename = "staticModels", default)]
    pub static_models: Vec<String>,
    #[serde(rename = "httpListModels", default)]
    pub http_list_models: bool,
}

pub struct HarnessConfig {
    // ... existing fields unchanged ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peon: Option<PeonConfig>,   // NEW
}
```

`builtin_harness_configs()` is updated to embed peon configs for all peon-capable built-in harnesses (claude-code, opencode, codex, gemini, aider, gh-copilot).

### 2. `ProviderDefinition` changes to owned strings

`ProviderDefinition` currently uses `&'static str` throughout. Change all fields to `String` / `Vec<String>` so definitions can be derived at runtime from `HarnessConfig`.

### 3. `builtin_provider_registry()` shrinks to ollama only

Ollama is HTTP-based and has no corresponding harness. It remains as the one standalone `ProviderDefinition`. All other provider definitions are derived from harness configs at startup:

```rust
// harness_registry.rs
pub fn derive_provider_definitions(harnesses: &[HarnessConfig]) -> Vec<ProviderDefinition> {
    harnesses.iter().filter_map(|h| {
        let peon = h.peon.as_ref()?;
        Some(ProviderDefinition {
            id: h.id.clone(),
            label: h.name.clone(),
            command: peon.command_override.clone().unwrap_or_else(|| h.command.clone()),
            default_args: peon.args.clone(),
            model_arg_template: peon.model_arg_template.clone(),
            supports_model: peon.supports_model,
            timeout_secs: peon.timeout_secs,
            list_models_command: peon.list_models_command.clone(),
            list_models_args: peon.list_models_args.clone(),
            static_models: peon.static_models.clone(),
            http_list_models: peon.http_list_models,
        })
    }).collect()
}
```

`ProviderManager` is initialized with the derived definitions merged with the ollama standalone entry.

### 4. `limit_patterns` becomes `Vec<String>`

Change `HarnessAdapter.limit_patterns` from `&'static [&'static str]` to `Vec<String>`. Update `HarnessAdapterConfig` (the disk-deserializable form) to include `limit_patterns: Vec<String>` so disk-configured adapters can specify detection patterns.

```rust
// harness.rs
pub struct HarnessAdapter {
    pub id: String,
    pub display_name: String,
    pub capabilities: HarnessCapabilities,
    pub limit_patterns: Vec<String>,   // was &'static [&'static str]
    // ... templates unchanged ...
}
```

### 5. Aider and gemini get proper adapter types

Currently aider and gemini use `generic-shell` as their adapter type. They should each have their own entry in `builtin_adapters()` with accurate capability flags and limit patterns for their tools. This unblocks per-tool usage-limit detection for those harnesses without needing to widen the generic-shell adapter.

## File changes

| File | Change |
|------|--------|
| `harness.rs` | `limit_patterns: Vec<String>`, update constructor + `from_config` |
| `harness_registry.rs` | Add `PeonConfig`, add `peon` field to `HarnessConfig`, update `builtin_harness_configs()`, add aider/gemini to `builtin_adapters()`, add `derive_provider_definitions()` |
| `providers.rs` | `ProviderDefinition` fields → owned strings, `builtin_provider_registry()` → ollama only, `ProviderManager::new()` accepts derived definitions |
| `main.rs` | Wire `derive_provider_definitions()` into `ProviderManager` construction |

## Non-goals

- No disk format for adapter types (users can override instances via `harnesses.json`; adapter types remain code-only)
- No changes to how peon inference runs — only where the config comes from
- No changes to `HarnessCapabilities` or the resume protocol

## Result

Adding a new harness (with peon support): one `HarnessConfig` entry with a `peon` sub-struct.
Adding a new harness without peon support: one `HarnessConfig` entry, `peon: None`.
Adding a new adapter type (new resume/detection behavior): one entry in `builtin_adapters()`.
Disk-configured harnesses can now specify `limit_patterns`.
