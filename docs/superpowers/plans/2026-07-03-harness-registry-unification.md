# Harness Registry Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify three separate harness registries into one — adding a new harness with peon inference support requires touching a single struct instead of three files.

**Architecture:** Add `HarnessPeonConfig` to `HarnessConfig` (instance level); derive `ProviderDefinition`s from harness configs at startup; give aider/gemini their own adapter types; change `limit_patterns` to `Vec<String>` so disk-configured harnesses can specify detection patterns.

**Tech Stack:** Rust, serde, existing crate modules (`harness.rs`, `harness_registry.rs`, `providers.rs`, `peon.rs`, `http/session_handlers.rs`, `main.rs`)

**Spec:** `docs/superpowers/specs/2026-07-03-harness-registry-unification-design.md`

---

## File map

| File | What changes |
|------|-------------|
| `crates/orkworksd/src/harness.rs` | `limit_patterns: Vec<String>`, update constructor + `HarnessAdapterConfig` |
| `crates/orkworksd/src/peon.rs` | Make detect functions generic over `AsRef<str>` |
| `crates/orkworksd/src/harness_registry.rs` | Add `HarnessPeonConfig`, update `HarnessConfig`, update `builtin_harness_configs`, add aider/gemini to `builtin_adapters`, update call sites |
| `crates/orkworksd/src/providers.rs` | `ProviderDefinition` owned strings, add `derive_from_harness_configs()`, shrink `builtin_provider_registry()` to ollama only, wire into `ProviderManager::new` and `for_tests` |
| `crates/orkworksd/src/http/session_handlers.rs` | Add `&` before `adapter.limit_patterns` at 4 call sites |

---

## Task 1: `limit_patterns` type cascade

Changes `limit_patterns` from `&'static [&'static str]` to `Vec<String>` and makes peon detect functions generic so both `&[&str]` and `&[String]` work at call sites.

**Files:**
- Modify: `crates/orkworksd/src/peon.rs:146-198`
- Modify: `crates/orkworksd/src/harness.rs:16-131`
- Modify: `crates/orkworksd/src/harness_registry.rs:182-283` (`builtin_adapters`)
- Modify: `crates/orkworksd/src/http/session_handlers.rs:661-665`

- [ ] **Step 1: Make peon detect functions generic**

In `peon.rs`, replace the four detect functions (lines 146–199) with generic versions:

```rust
pub fn detect_usage_limit<S: AsRef<str>>(patterns: &[S], lines: &[String]) -> bool {
    if patterns.is_empty() { return false; }
    lines.iter().any(|line| {
        let lower = strip_ansi(line).to_lowercase();
        patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str()))
    })
}

pub fn detect_usage_limit_raw<S: AsRef<str>>(patterns: &[S], text: &str) -> bool {
    if patterns.is_empty() { return false; }
    let lower = strip_ansi(text).to_lowercase();
    patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str()))
}

pub fn detect_usage_limit_hint_raw<S: AsRef<str>>(patterns: &[S], text: &str) -> Option<String> {
    if patterns.is_empty() { return None; }
    let plain = strip_ansi(text);
    let lower = plain.to_lowercase();
    if !patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str())) {
        return None;
    }
    let idx = lower.find("resets in").or_else(|| lower.find("reset in")).or_else(|| lower.find("try again at"))?;
    let fragment = &plain[idx..];
    let end = fragment.find(['.', '\n']).unwrap_or(fragment.len());
    Some(fragment[..end].trim().to_string())
}

pub fn detect_usage_limit_hint<S: AsRef<str>>(patterns: &[S], lines: &[String]) -> Option<String> {
    if patterns.is_empty() { return None; }
    lines.iter().rev().find_map(|line| {
        let plain = strip_ansi(line);
        let lower = plain.to_lowercase();
        if !patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str())) {
            return None;
        }
        let idx = lower.find("resets in").or_else(|| lower.find("reset in")).or_else(|| lower.find("try again at"))?;
        let fragment = &plain[idx..];
        let end = fragment.find(['.', '\n']).unwrap_or(fragment.len());
        Some(fragment[..end].trim().to_string())
    })
}
```

- [ ] **Step 2: Update `HarnessAdapter` and `HarnessAdapterConfig` in `harness.rs`**

Change the `limit_patterns` field and both constructors:

```rust
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HarnessAdapter {
    pub id: String,
    pub display_name: String,
    pub capabilities: HarnessCapabilities,
    pub limit_patterns: Vec<String>,   // was &'static [&'static str]
    launch_template: CommandTemplate,
    exact_resume_template: Option<CommandTemplate>,
    latest_cwd_resume_template: Option<CommandTemplate>,
    latest_repo_resume_template: Option<CommandTemplate>,
}
```

Add `limit_patterns` to `HarnessAdapterConfig` (after `capabilities` field):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct HarnessAdapterConfig {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub capabilities: HarnessCapabilities,
    #[serde(rename = "limitPatterns", default)]
    pub limit_patterns: Vec<String>,
    pub launch: CommandTemplate,
    #[serde(rename = "resumeExact", skip_serializing_if = "Option::is_none")]
    pub resume_exact: Option<CommandTemplate>,
    #[serde(rename = "resumeLatestCwd", skip_serializing_if = "Option::is_none")]
    pub resume_latest_cwd: Option<CommandTemplate>,
    #[serde(rename = "resumeLatestRepo", skip_serializing_if = "Option::is_none")]
    pub resume_latest_repo: Option<CommandTemplate>,
}
```

Update `from_config`:

```rust
pub fn from_config(config: HarnessAdapterConfig) -> Self {
    Self {
        id: config.id,
        display_name: config.display_name,
        capabilities: config.capabilities,
        limit_patterns: config.limit_patterns,
        launch_template: config.launch,
        exact_resume_template: config.resume_exact,
        latest_cwd_resume_template: config.resume_latest_cwd,
        latest_repo_resume_template: config.resume_latest_repo,
    }
}
```

Update `template`:

```rust
pub fn template(
    id: impl Into<String>,
    display_name: impl Into<String>,
    capabilities: HarnessCapabilities,
    limit_patterns: Vec<String>,
    launch_template: CommandTemplate,
    exact_resume_template: Option<CommandTemplate>,
    latest_cwd_resume_template: Option<CommandTemplate>,
) -> Self {
    Self {
        id: id.into(),
        display_name: display_name.into(),
        capabilities,
        limit_patterns,
        launch_template,
        exact_resume_template,
        latest_cwd_resume_template,
        latest_repo_resume_template: None,
    }
}
```

- [ ] **Step 3: Update `builtin_adapters()` call sites in `harness_registry.rs`**

Replace every `&[]` with `vec![]` and every `&["pattern"]` with `vec!["pattern".to_string()]` in all `HarnessAdapter::template(...)` calls inside `builtin_adapters()`.

`generic` adapter: `vec![]`
`opencode` adapter: `vec!["usage limit reached".to_string()]`
`claude` adapter: `vec![]`
`codex` adapter: `vec!["you've hit your usage limit".to_string()]`

- [ ] **Step 4: Update `session_handlers.rs` call sites**

At lines 661–665 add `&` before `adapter.limit_patterns`:

```rust
merged.at_usage_limit = limit_adapter
    .map(|adapter| prev_latch
        || peon::detect_usage_limit(&adapter.limit_patterns, &snapshot)
        || peon::detect_usage_limit_raw(&adapter.limit_patterns, &scan_buf));
merged.usage_limit_reset_hint = limit_adapter
    .and_then(|adapter| peon::detect_usage_limit_hint(&adapter.limit_patterns, &snapshot)
        .or_else(|| peon::detect_usage_limit_hint_raw(&adapter.limit_patterns, &scan_buf)));
```

- [ ] **Step 5: Update `harness.rs` tests**

In the `tests` module, every `HarnessAdapter::template(...)` call passes `&[]` as the `limit_patterns` argument. Change each one to `vec![]`:

```rust
// before
&[],
// after
vec![],
```

- [ ] **Step 6: Verify it compiles and tests pass**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/orkworksd/src/peon.rs \
        crates/orkworksd/src/harness.rs \
        crates/orkworksd/src/harness_registry.rs \
        crates/orkworksd/src/http/session_handlers.rs
git commit -m "refactor: limit_patterns as Vec<String>, peon detect fns generic over AsRef<str>"
```

---

## Task 2: Add `HarnessPeonConfig` and `peon` field to `HarnessConfig`

Adds the peon inference config sub-struct to each built-in harness that participates in peon inference. Also adds `gh-copilot` as a proper `HarnessConfig` entry.

**Files:**
- Modify: `crates/orkworksd/src/harness_registry.rs`

- [ ] **Step 1: Add `HarnessPeonConfig` struct**

Add after the existing `HarnessVoiceCapabilities` struct:

```rust
fn default_peon_timeout() -> u64 { 30 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessPeonConfig {
    #[serde(rename = "commandOverride", skip_serializing_if = "Option::is_none")]
    pub(crate) command_override: Option<String>,
    #[serde(default)]
    pub(crate) args: Vec<String>,
    #[serde(rename = "modelArgTemplate", skip_serializing_if = "Option::is_none")]
    pub(crate) model_arg_template: Option<String>,
    #[serde(rename = "supportsModel", default)]
    pub(crate) supports_model: bool,
    #[serde(rename = "timeoutSecs", default = "default_peon_timeout")]
    pub(crate) timeout_secs: u64,
    #[serde(rename = "listModelsCommand", skip_serializing_if = "Option::is_none")]
    pub(crate) list_models_command: Option<String>,
    #[serde(rename = "listModelsArgs", default)]
    pub(crate) list_models_args: Vec<String>,
    #[serde(rename = "staticModels", default)]
    pub(crate) static_models: Vec<String>,
    #[serde(rename = "httpListModels", default)]
    pub(crate) http_list_models: bool,
}
```

- [ ] **Step 2: Add `peon` field to `HarnessConfig`**

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) harness: String,
    pub(crate) command: String,
    #[serde(default)]
    pub(crate) args: Vec<String>,
    #[serde(rename = "defaultModel", default)]
    pub(crate) default_model: String,
    #[serde(rename = "modelPrefix", default)]
    pub(crate) model_prefix: String,
    #[serde(default)]
    pub(crate) capabilities: HarnessVoiceCapabilities,
    #[serde(rename = "isBuiltin", default)]
    pub(crate) is_builtin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) peon: Option<HarnessPeonConfig>,
}
```

- [ ] **Step 3: Update `builtin_harness_configs()` with peon data**

Replace the entire function body with the following. Each peon-capable harness gets its `peon` field set; `generic-shell` stays `peon: None`.

```rust
pub(crate) fn builtin_harness_configs() -> Vec<HarnessConfig> {
    let (shell_program, shell_args) = shell_cmd();
    vec![
        HarnessConfig {
            id: "claude-code".into(),
            name: "Claude Code".into(),
            harness: "claude-code".into(),
            command: "claude".into(),
            args: vec![],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec!["-p".to_string()],
                model_arg_template: Some("--model={model}".to_string()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "claude-sonnet-4-6".to_string(),
                    "claude-opus-4-20250514".to_string(),
                    "claude-opus-4-1-20250805".to_string(),
                    "claude-sonnet-4-5-20250929".to_string(),
                    "claude-haiku-3-5-20241022".to_string(),
                ],
                http_list_models: false,
            }),
        },
        HarnessConfig {
            id: "opencode".into(),
            name: "OpenCode".into(),
            harness: "opencode".into(),
            command: "opencode".into(),
            args: vec!["--model".into(), "{model}".into()],
            default_model: String::new(),
            model_prefix: "ollama/".into(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec!["run".to_string(), "--pure".to_string()],
                model_arg_template: Some("--model={model}".to_string()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: Some("opencode".to_string()),
                list_models_args: vec!["models".to_string()],
                static_models: vec![],
                http_list_models: false,
            }),
        },
        HarnessConfig {
            id: "codex".into(),
            name: "Codex".into(),
            harness: "codex".into(),
            command: "codex".into(),
            args: vec![],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec!["exec".to_string()],
                model_arg_template: Some("--model={model}".to_string()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "gpt-5-codex".to_string(),
                    "gpt-5".to_string(),
                    "gpt-5-mini".to_string(),
                    "gpt-5-nano".to_string(),
                ],
                http_list_models: false,
            }),
        },
        HarnessConfig {
            id: "gemini".into(),
            name: "Gemini CLI".into(),
            harness: "gemini".into(),  // will be its own type after Task 3
            command: "gemini".into(),
            args: vec![],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec![],
                model_arg_template: Some("--model={model}".to_string()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "gemini-2.5-pro".to_string(),
                    "gemini-2.5-flash".to_string(),
                    "gemini-2.0-flash".to_string(),
                ],
                http_list_models: false,
            }),
        },
        HarnessConfig {
            id: "aider".into(),
            name: "Aider".into(),
            harness: "aider".into(),  // will be its own type after Task 3
            command: "aider".into(),
            args: vec!["--model".into(), "{model}".into()],
            default_model: "claude-sonnet-4-20250514".into(),
            model_prefix: "ollama_chat/".into(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec![],
                model_arg_template: Some("--model={model}".to_string()),
                supports_model: true,
                timeout_secs: 60,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "claude-sonnet-4-6".to_string(),
                    "claude-opus-4-20250514".to_string(),
                    "gpt-4o".to_string(),
                    "gpt-5".to_string(),
                    "gemini-2.5-pro".to_string(),
                ],
                http_list_models: false,
            }),
        },
        HarnessConfig {
            id: "gh-copilot".into(),
            name: "Copilot".into(),
            harness: "generic-shell".into(),
            command: "gh".into(),
            args: vec!["copilot".into(), "suggest".into()],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec!["copilot".to_string(), "suggest".to_string()],
                model_arg_template: Some("--model={model}".to_string()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "gpt-4o".to_string(),
                    "gpt-5".to_string(),
                    "claude-sonnet-4-6".to_string(),
                    "gemini-2.5-pro".to_string(),
                ],
                http_list_models: false,
            }),
        },
        HarnessConfig {
            id: "generic-shell".into(),
            name: "Shell".into(),
            harness: "generic-shell".into(),
            command: shell_program,
            args: shell_args,
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
            peon: None,
        },
    ]
}
```

- [ ] **Step 4: Write a test that verifies peon is set correctly**

Add to the `tests` module in `harness_registry.rs`:

```rust
#[test]
fn builtin_harness_configs_have_peon_set_for_peon_capable_harnesses() {
    let configs = builtin_harness_configs();
    let ids_with_peon: Vec<&str> = configs.iter()
        .filter(|h| h.peon.is_some())
        .map(|h| h.id.as_str())
        .collect();
    assert!(ids_with_peon.contains(&"claude-code"), "claude-code must have peon config");
    assert!(ids_with_peon.contains(&"opencode"), "opencode must have peon config");
    assert!(ids_with_peon.contains(&"codex"), "codex must have peon config");
    assert!(ids_with_peon.contains(&"gemini"), "gemini must have peon config");
    assert!(ids_with_peon.contains(&"aider"), "aider must have peon config");
    assert!(ids_with_peon.contains(&"gh-copilot"), "gh-copilot must have peon config");

    let shell = configs.iter().find(|h| h.id == "generic-shell").unwrap();
    assert!(shell.peon.is_none(), "generic-shell must not have peon config");
}

#[test]
fn opencode_peon_config_uses_run_pure_args() {
    let configs = builtin_harness_configs();
    let opencode = configs.iter().find(|h| h.id == "opencode").unwrap();
    let peon = opencode.peon.as_ref().unwrap();
    assert_eq!(peon.args, vec!["run", "--pure"]);
    assert_eq!(peon.list_models_command.as_deref(), Some("opencode"));
}
```

- [ ] **Step 5: Run and verify**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -20
```

Expected: all tests pass including the two new ones.

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/harness_registry.rs
git commit -m "feat: add HarnessPeonConfig to HarnessConfig, wire peon data for all builtin harnesses"
```

---

## Task 3: Add aider and gemini adapter types

Gives aider and gemini their own entries in `builtin_adapters()` so they can carry their own capabilities and limit patterns independent of `generic-shell`.

**Files:**
- Modify: `crates/orkworksd/src/harness_registry.rs` (`builtin_adapters`)

- [ ] **Step 1: Add aider and gemini adapters to `builtin_adapters()`**

After the `codex` entry and before `map`, add:

```rust
let aider_caps = harness::HarnessCapabilities {
    launch: true,
    resume_exact: false,
    resume_latest_in_cwd: false,
    resume_latest_in_repo: false,
    detect_session_id: false,
    detect_model: false,
    detect_context_usage: false,
    detect_capacity: false,
    native_voice: false,
};
let aider = harness::HarnessAdapter::template(
    "aider",
    "Aider",
    aider_caps,
    vec![],
    harness::CommandTemplate {
        command: "aider".into(),
        args: vec!["--model".into(), "{model}".into()],
    },
    None,
    None,
);
map.insert("aider".into(), aider);

let gemini_caps = harness::HarnessCapabilities {
    launch: true,
    resume_exact: false,
    resume_latest_in_cwd: false,
    resume_latest_in_repo: false,
    detect_session_id: false,
    detect_model: false,
    detect_context_usage: false,
    detect_capacity: false,
    native_voice: false,
};
let gemini = harness::HarnessAdapter::template(
    "gemini",
    "Gemini CLI",
    gemini_caps,
    vec![],
    harness::CommandTemplate {
        command: "gemini".into(),
        args: vec![],
    },
    None,
    None,
);
map.insert("gemini".into(), gemini);
```

- [ ] **Step 2: Write a test**

Add to the `tests` module:

```rust
#[test]
fn builtin_adapters_includes_aider_and_gemini_types() {
    let adapters = builtin_adapters();
    assert!(adapters.contains_key("aider"), "aider adapter must be registered");
    assert!(adapters.contains_key("gemini"), "gemini adapter must be registered");

    let aider_config = builtin_harness_configs();
    let aider = aider_config.iter().find(|h| h.id == "aider").unwrap();
    assert_eq!(aider.harness, "aider", "aider config must reference aider adapter type");
    let gemini = aider_config.iter().find(|h| h.id == "gemini").unwrap();
    assert_eq!(gemini.harness, "gemini", "gemini config must reference gemini adapter type");
}
```

- [ ] **Step 3: Run and verify**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/orkworksd/src/harness_registry.rs
git commit -m "feat: add aider and gemini as first-class adapter types"
```

---

## Task 4: `ProviderDefinition` owned strings + derive from harness configs

Removes the last static string registry duplication. `ProviderDefinition` becomes runtime-constructable; its entries are derived from `HarnessConfig.peon` at startup. `builtin_provider_registry()` shrinks to ollama only.

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`

- [ ] **Step 1: Change `ProviderDefinition` to owned strings**

Replace the struct definition (lines 133–145):

```rust
#[derive(Clone, Debug)]
pub struct ProviderDefinition {
    pub id: String,
    pub label: String,
    pub command: String,
    pub default_args: Vec<String>,
    pub model_arg_template: Option<String>,
    pub supports_model: bool,
    pub timeout_secs: u64,
    pub list_models_command: Option<String>,
    pub list_models_args: Vec<String>,
    pub static_models: Vec<String>,
    pub http_list_models: bool,
}
```

- [ ] **Step 2: Shrink `builtin_provider_registry()` to ollama only**

Replace the entire function body:

```rust
pub fn builtin_provider_registry() -> Vec<ProviderDefinition> {
    vec![
        ProviderDefinition {
            id: "ollama".to_string(),
            label: "Ollama".to_string(),
            command: String::new(),
            default_args: vec![],
            model_arg_template: None,
            supports_model: false,
            timeout_secs: 30,
            list_models_command: None,
            list_models_args: vec![],
            static_models: vec![],
            http_list_models: true,
        },
    ]
}
```

- [ ] **Step 3: Add `derive_from_harness_configs()` and update `ProviderManager::new`**

Add this private function after `builtin_provider_registry()`:

```rust
fn derive_from_harness_configs() -> Vec<ProviderDefinition> {
    crate::harness_registry::builtin_harness_configs()
        .into_iter()
        .filter_map(|h| {
            let peon = h.peon?;
            Some(ProviderDefinition {
                id: h.id,
                label: h.name,
                command: peon.command_override.unwrap_or(h.command),
                default_args: peon.args,
                model_arg_template: peon.model_arg_template,
                supports_model: peon.supports_model,
                timeout_secs: peon.timeout_secs,
                list_models_command: peon.list_models_command,
                list_models_args: peon.list_models_args,
                static_models: peon.static_models,
                http_list_models: peon.http_list_models,
            })
        })
        .collect()
}
```

Update `ProviderManager::new()` to use derived definitions:

```rust
pub fn new() -> Self {
    let settings = Arc::new(RwLock::new(ProviderSettingsPayload::default()));
    let runtime = Arc::new(RwLock::new(HashMap::new()));
    let mut registry = derive_from_harness_configs();
    registry.extend(builtin_provider_registry());
    Self {
        registry,
        settings: settings.clone(),
        applied_revision: Arc::new(RwLock::new(None)),
        runtime,
        runner: Arc::new(CompositeRunner {
            process: ProcessRunner,
            http: HttpRunner { settings },
        }),
        session_capped: Arc::new(RwLock::new(HashMap::new())),
        session_reset_hint: Arc::new(RwLock::new(HashMap::new())),
        session_checking: Arc::new(RwLock::new(HashSet::new())),
    }
}
```

- [ ] **Step 4: Update all `run_inference` and `list_models` string field accesses**

In `run_inference()` (around line 729), `definition.command` is now `String` — change the runner call:

```rust
let result = self.runner.run(&entry.id, &definition.command, &args, &prompt, definition.timeout_secs);
```

In `list_models()`, change:

```rust
// Before
return Ok(definition.static_models.iter().map(|s| s.to_string()).collect());
// After
return Ok(definition.static_models.clone());
```

```rust
// Before
let command = definition.list_models_command.unwrap();
let args = definition.list_models_args;
// After
let command = definition.list_models_command.as_deref().unwrap();
let args = &definition.list_models_args;
```

In `list_models_http()`, the provider match is on `id: &str` — no change needed there.

In `get_providers_response()`, the label lookup:

```rust
// Before
.map(|d| d.label.to_string())
// After
.map(|d| d.label.clone())
```

- [ ] **Step 5: Update `for_tests` to use derived definitions**

In the `#[cfg(test)]` `impl ProviderManager` block, update `for_tests`:

```rust
pub fn for_tests(settings: ProviderSettingsPayload, fakes: Vec<FakeProvider>) -> Self {
    let specs: HashMap<String, FakeProvider> =
        fakes.into_iter().map(|f| (f.id.to_string(), f)).collect();
    let mut registry = derive_from_harness_configs();
    registry.extend(builtin_provider_registry());
    Self {
        registry,
        settings: Arc::new(RwLock::new(settings)),
        applied_revision: Arc::new(RwLock::new(None)),
        runtime: Arc::new(RwLock::new(HashMap::new())),
        runner: Arc::new(FakeRunner { specs }),
        session_capped: Arc::new(RwLock::new(HashMap::new())),
        session_reset_hint: Arc::new(RwLock::new(HashMap::new())),
        session_checking: Arc::new(RwLock::new(HashSet::new())),
    }
}
```

- [ ] **Step 6: Update inline test `ProviderDefinition` literals**

Two tests in the `tests` module use `for_tests_with_registry` with literal `ProviderDefinition` structs. Replace both:

`list_models_returns_empty_when_no_list_command_configured`:

```rust
vec![ProviderDefinition {
    id: "test-provider".to_string(),
    label: "Test".to_string(),
    command: "test".to_string(),
    default_args: vec![],
    model_arg_template: None,
    supports_model: false,
    timeout_secs: 30,
    list_models_command: None,
    list_models_args: vec![],
    static_models: vec![],
    http_list_models: false,
}]
```

`list_models_returns_static_models_when_no_command`:

```rust
vec![ProviderDefinition {
    id: "claude-code".to_string(),
    label: "Claude Code".to_string(),
    command: "claude".to_string(),
    default_args: vec![],
    model_arg_template: None,
    supports_model: false,
    timeout_secs: 30,
    list_models_command: None,
    list_models_args: vec![],
    static_models: vec!["sonnet".to_string(), "opus".to_string(), "haiku".to_string()],
    http_list_models: false,
}]
```

- [ ] **Step 7: Add tests that verify the derive output**

`derive_from_harness_configs` is private but accessible from within the `tests` module in the same file. Test it directly; test `ProviderManager::new` indirectly via `list_models()` (which returns `Err("unknown provider: …")` for unknown ids, but `Ok(…)` for registered ones with static models):

```rust
#[test]
fn derived_registry_includes_all_peon_capable_harnesses() {
    let derived = derive_from_harness_configs();
    let ids: Vec<&str> = derived.iter().map(|d| d.id.as_str()).collect();
    for expected in &["claude-code", "opencode", "codex", "gemini", "aider", "gh-copilot"] {
        assert!(ids.contains(expected), "derived registry must include {expected}");
    }
    assert!(!ids.contains(&"generic-shell"), "generic-shell has no peon config");
    assert!(!ids.contains(&"ollama"), "ollama is standalone, not derived");
}

#[test]
fn provider_manager_new_includes_derived_harnesses_and_ollama() {
    let mgr = ProviderManager::new();
    // claude-code has static models → Ok without network
    let models = mgr.list_models("claude-code").expect("claude-code must be in registry");
    assert!(models.contains(&"claude-sonnet-4-6".to_string()));
    // ollama is in registry → Err is connection error, not "unknown provider"
    let err = mgr.list_models("ollama").unwrap_err();
    assert!(!err.contains("unknown provider"), "ollama must be registered: got {err}");
    // truly unknown provider → "unknown provider" error
    let err = mgr.list_models("nonexistent-xyz").unwrap_err();
    assert!(err.contains("unknown provider"));
}
```

- [ ] **Step 8: Run full test suite**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add crates/orkworksd/src/providers.rs
git commit -m "refactor: ProviderDefinition owned strings, derive provider registry from harness configs"
```

---

## Final: doc check and push

- [ ] **Step 1: Run the doc check**

```bash
bash .claude/hooks/doc-check.sh
```

Address any flagged files.

- [ ] **Step 2: Push to remote**

```bash
git push
```
