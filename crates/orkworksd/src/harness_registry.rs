use crate::harness;
use crate::session_view::derive_memory_state;
use crate::session_types::MemoryState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct HarnessVoiceCapabilities {
    #[serde(rename = "nativeVoice", default)]
    pub(crate) native_voice: bool,
    #[serde(rename = "requiresMicrophonePermission", default)]
    pub(crate) requires_microphone_permission: bool,
    #[serde(rename = "orkworksDictation", default)]
    pub(crate) orkworks_dictation: bool,
    #[serde(rename = "orkworksVoiceCommands", default)]
    pub(crate) orkworks_voice_commands: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct HarnessAttentionCapabilities {
    #[serde(rename = "activeWorkHook", default)]
    pub(crate) active_work_hook: bool,
}

pub(crate) fn normalize_hook_attention_status(
    status: &str,
    supports_active_work: bool,
) -> Option<String> {
    match status {
        "working" | "thinking" | "reasoning" if supports_active_work => Some("working".into()),
        "waiting_for_input" | "blocked" | "failed" | "done" | "stale" | "idle" => {
            Some(status.into())
        }
        _ => None,
    }
}

/// Peon inference config for a harness instance. When present the harness
/// participates in model probing and provider capacity checks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessPeonConfig {
    /// Overrides the harness command for headless invocation (rarely needed).
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

fn default_peon_timeout() -> u64 { 30 }

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
    #[serde(default)]
    pub(crate) attention: HarnessAttentionCapabilities,
    #[serde(rename = "isBuiltin", default)]
    pub(crate) is_builtin: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) peon: Option<HarnessPeonConfig>,
}

fn shell_cmd() -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        (
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()),
            vec![],
        )
    } else {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        (shell, vec!["-i".into(), "-l".into()])
    }
}

pub(crate) fn global_harnesses_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".orkworks").join("harnesses.json"))
}

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
            attention: HarnessAttentionCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec!["-p".into()],
                model_arg_template: Some("--model={model}".into()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "claude-sonnet-4-6".into(),
                    "claude-opus-4-20250514".into(),
                    "claude-opus-4-1-20250805".into(),
                    "claude-sonnet-4-5-20250929".into(),
                    "claude-haiku-3-5-20241022".into(),
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
            attention: HarnessAttentionCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec!["run".into(), "--pure".into()],
                model_arg_template: Some("--model={model}".into()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: Some("opencode".into()),
                list_models_args: vec!["models".into()],
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
            attention: HarnessAttentionCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec!["exec".into()],
                model_arg_template: Some("--model={model}".into()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "gpt-5-codex".into(),
                    "gpt-5".into(),
                    "gpt-5-mini".into(),
                    "gpt-5-nano".into(),
                ],
                http_list_models: false,
            }),
        },
        HarnessConfig {
            id: "gemini".into(),
            name: "Gemini CLI".into(),
            harness: "gemini".into(),
            command: "gemini".into(),
            args: vec![],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            attention: HarnessAttentionCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec![],
                model_arg_template: Some("--model={model}".into()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "gemini-2.5-pro".into(),
                    "gemini-2.5-flash".into(),
                    "gemini-2.0-flash".into(),
                ],
                http_list_models: false,
            }),
        },
        HarnessConfig {
            id: "aider".into(),
            name: "Aider".into(),
            harness: "aider".into(),
            command: "aider".into(),
            args: vec!["--model".into(), "{model}".into()],
            default_model: "claude-sonnet-4-20250514".into(),
            model_prefix: "ollama_chat/".into(),
            capabilities: HarnessVoiceCapabilities::default(),
            attention: HarnessAttentionCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec![],
                model_arg_template: Some("--model={model}".into()),
                supports_model: true,
                timeout_secs: 60,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "claude-sonnet-4-6".into(),
                    "claude-opus-4-20250514".into(),
                    "gpt-4o".into(),
                    "gpt-5".into(),
                    "gemini-2.5-pro".into(),
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
            attention: HarnessAttentionCapabilities::default(),
            is_builtin: true,
            peon: Some(HarnessPeonConfig {
                command_override: None,
                args: vec!["copilot".into(), "suggest".into()],
                model_arg_template: Some("--model={model}".into()),
                supports_model: true,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![
                    "gpt-4o".into(),
                    "gpt-5".into(),
                    "claude-sonnet-4-6".into(),
                    "gemini-2.5-pro".into(),
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
            attention: HarnessAttentionCapabilities::default(),
            is_builtin: true,
            peon: None,
        },
    ]
}

pub(crate) fn load_harnesses() -> Vec<HarnessConfig> {
    let built_ins = builtin_harness_configs();
    let Some(path) = global_harnesses_path() else { return built_ins; };
    let Ok(data) = std::fs::read_to_string(&path) else { return built_ins; };
    let Ok(disk): serde_json::Result<Vec<HarnessConfig>> = serde_json::from_str(&data) else {
        tracing::warn!("failed to parse ~/.orkworks/harnesses.json; using built-ins");
        return built_ins;
    };
    let mut result = built_ins;
    for disk_entry in disk {
        if let Some(pos) = result.iter().position(|h| h.id == disk_entry.id) {
            let is_builtin = result[pos].is_builtin;
            result[pos] = HarnessConfig { is_builtin, ..disk_entry };
        } else {
            result.push(disk_entry);
        }
    }
    result
}

pub(crate) fn save_harnesses(harnesses: &[HarnessConfig]) {
    let Some(path) = global_harnesses_path() else { return; };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(harnesses) {
        Ok(json) => {
            let _ = std::fs::write(&path, json);
        }
        Err(e) => tracing::error!(error = %e, "failed to serialize harnesses"),
    }
}

pub(crate) fn default_shell_command(cwd: String) -> harness::CommandSpec {
    let (program, args) = shell_cmd();
    harness::CommandSpec { program, args, cwd }
}

pub(crate) fn default_capabilities() -> harness::HarnessCapabilities {
    harness::HarnessCapabilities {
        launch: true,
        resume_exact: false,
        resume_latest_in_cwd: false,
        resume_latest_in_repo: false,
        detect_session_id: false,
        detect_model: false,
        detect_context_usage: false,
        detect_capacity: false,
        native_voice: false,
    }
}

pub(crate) fn builtin_adapters() -> HashMap<String, harness::HarnessAdapter> {
    let (program, args) = shell_cmd();
    let mut map = HashMap::new();

    let generic = harness::HarnessAdapter::template(
        "generic-shell",
        "Generic Shell",
        default_capabilities(),
        vec![],
        harness::CommandTemplate {
            command: program.clone(),
            args: args.clone(),
        },
        None,
        None,
    );
    map.insert("generic-shell".into(), generic);

    let opencode_caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: true,
        resume_latest_in_cwd: true,
        resume_latest_in_repo: true,
        detect_session_id: true,
        detect_model: true,
        detect_context_usage: true,
        detect_capacity: true,
        native_voice: false,
    };
    let opencode = harness::HarnessAdapter::template(
        "opencode",
        "OpenCode",
        opencode_caps.clone(),
        vec!["usage limit reached".to_string()],
        harness::CommandTemplate {
            command: "opencode".into(),
            args: vec![],
        },
        Some(harness::CommandTemplate {
            command: "opencode".into(),
            args: vec!["--session".into(), "{harnessSessionId}".into()],
        }),
        Some(harness::CommandTemplate {
            command: "opencode".into(),
            args: vec!["--continue".into()],
        }),
    );
    map.insert("opencode".into(), opencode);

    let claude_caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: true,
        resume_latest_in_cwd: true,
        resume_latest_in_repo: true,
        detect_session_id: true,
        detect_model: true,
        detect_context_usage: true,
        detect_capacity: true,
        native_voice: false,
    };
    let claude = harness::HarnessAdapter::template(
        "claude-code",
        "Claude Code",
        claude_caps.clone(),
        vec!["you've hit your session limit".to_string()],
        harness::CommandTemplate {
            command: "claude".into(),
            args: vec![],
        },
        Some(harness::CommandTemplate {
            command: "claude".into(),
            args: vec!["--resume".into(), "{harnessSessionId}".into()],
        }),
        Some(harness::CommandTemplate {
            command: "claude".into(),
            args: vec!["--continue".into()],
        }),
    );
    map.insert("claude-code".into(), claude);

    let codex_caps = harness::HarnessCapabilities {
        launch: true,
        resume_exact: false,
        resume_latest_in_cwd: false,
        resume_latest_in_repo: false,
        detect_session_id: false,
        detect_model: false,
        detect_context_usage: false,
        detect_capacity: true,
        native_voice: false,
    };
    let codex = harness::HarnessAdapter::template(
        "codex",
        "Codex",
        codex_caps,
        vec!["you've hit your usage limit".to_string()],
        harness::CommandTemplate {
            command: "codex".into(),
            args: vec![],
        },
        None,
        None,
    );
    map.insert("codex".into(), codex);

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
            args: vec![],
        },
        None,
        None,
    );
    map.insert("aider".into(), aider);

    map
}

pub(crate) fn capabilities_for_harness(
    adapters: &HashMap<String, harness::HarnessAdapter>,
    harness_id: Option<&str>,
) -> harness::HarnessCapabilities {
    match harness_id {
        Some(h) if !h.is_empty() => adapters
            .get(h)
            .map(|a| a.capabilities.clone())
            .unwrap_or_else(default_capabilities),
        _ => default_capabilities(),
    }
}

pub(crate) fn adapter_for_harness<'a>(
    adapters: &'a HashMap<String, harness::HarnessAdapter>,
    harness_id: Option<&str>,
) -> &'a harness::HarnessAdapter {
    match harness_id {
        Some(h) if !h.is_empty() => adapters.get(h),
        _ => None,
    }
    .unwrap_or_else(|| adapters.get("generic-shell").unwrap())
}

pub(crate) fn resolve_adapter_harness_id(
    harnesses: &[HarnessConfig],
    session_harness_id: Option<&str>,
) -> Option<String> {
    let harness_id = session_harness_id?;
    if harness_id.is_empty() {
        return None;
    }
    harnesses
        .iter()
        .find(|h| h.id == harness_id)
        .map(|h| h.harness.clone())
        .or_else(|| Some(harness_id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::with_fake_home;

    #[test]
    fn attention_capability_defaults_to_no_active_work_hook() {
        let parsed: HarnessConfig = serde_json::from_str(r#"{
            "id": "custom", "name": "Custom", "harness": "custom",
            "command": "custom"
        }"#)
        .unwrap();

        assert!(!parsed.attention.active_work_hook);
    }

    #[test]
    fn active_hook_aliases_normalize_only_for_capable_harnesses() {
        assert_eq!(
            normalize_hook_attention_status("thinking", true),
            Some("working".into())
        );
        assert_eq!(
            normalize_hook_attention_status("reasoning", true),
            Some("working".into())
        );
        assert_eq!(normalize_hook_attention_status("thinking", false), None);
        assert_eq!(
            normalize_hook_attention_status("waiting_for_input", false),
            Some("waiting_for_input".into())
        );
    }

    #[test]
    fn load_harnesses_merges_disk_overrides_with_builtins() {
        let dir = tempfile::tempdir().unwrap();

        with_fake_home(dir.path(), || {
            let mut override_entry = builtin_harness_configs()
                .into_iter()
                .find(|h| h.id == "opencode")
                .expect("expected opencode builtin");
            override_entry.command = "opencode-custom".into();
            override_entry.args = vec!["--sandbox".into()];
            override_entry.is_builtin = false;

            let harnesses_path = global_harnesses_path().unwrap();
            std::fs::create_dir_all(harnesses_path.parent().unwrap()).unwrap();
            std::fs::write(
                &harnesses_path,
                serde_json::to_string(&vec![override_entry]).unwrap(),
            )
            .unwrap();

            let harnesses = load_harnesses();
            let merged = harnesses
                .into_iter()
                .find(|h| h.id == "opencode")
                .expect("expected merged opencode harness");

            assert_eq!(merged.command, "opencode-custom");
            assert_eq!(merged.args, vec!["--sandbox"]);
            assert!(merged.is_builtin);
        });
    }

    #[test]
    fn load_harnesses_appends_custom_harnesses_after_builtins() {
        let dir = tempfile::tempdir().unwrap();

        with_fake_home(dir.path(), || {
            let custom = HarnessConfig {
                id: "custom-shell".into(),
                name: "Custom Shell".into(),
                harness: "generic-shell".into(),
                command: "/bin/sh".into(),
                args: vec!["-lc".into()],
                default_model: String::new(),
                model_prefix: String::new(),
                capabilities: HarnessVoiceCapabilities::default(),
                attention: HarnessAttentionCapabilities::default(),
                is_builtin: false,
                peon: None,
            };

            let harnesses_path = global_harnesses_path().unwrap();
            std::fs::create_dir_all(harnesses_path.parent().unwrap()).unwrap();
            std::fs::write(
                &harnesses_path,
                serde_json::to_string(&vec![custom.clone()]).unwrap(),
            )
            .unwrap();

            let harnesses = load_harnesses();
            let builtin_count = builtin_harness_configs().len();

            assert_eq!(harnesses.len(), builtin_count + 1);
            assert_eq!(harnesses.last().map(|h| h.id.as_str()), Some("custom-shell"));
            assert_eq!(harnesses.last().map(|h| h.is_builtin), Some(false));
        });
    }

    #[test]
    fn generic_shell_memory_state_is_not_resumable() {
        let capabilities = default_capabilities();
        let resume = harness::ResumeMemory {
            state: harness::ResumeState::Available,
            preferred_strategy: harness::ResumeStrategy::Exact,
            harness_session_id: Some("sess-1".into()),
            latest_fallback: true,
            last_seen_at: None,
        };

        let (memory_state, strategy) = derive_memory_state(false, Some(&resume), &capabilities);
        let command = builtin_adapters()
            .get("generic-shell")
            .unwrap()
            .build_resume_command(&harness::ResumeRequest {
                strategy: harness::ResumeStrategy::Exact,
                cwd: "/tmp".into(),
                repo_root: Some("/tmp".into()),
                harness_session_id: Some("sess-1".into()),
                model: None,
            });

        assert_eq!(memory_state, MemoryState::Unsupported);
        assert_eq!(strategy, harness::ResumeStrategy::None);
        assert!(command.is_none());
    }

    #[test]
    fn resolve_adapter_harness_id_treats_empty_string_as_no_harness() {
        let harnesses = builtin_harness_configs();
        let result = resolve_adapter_harness_id(&harnesses, Some(""));
        assert_eq!(result, None, "Some(\"\") should behave like None and return None");
    }

    #[test]
    fn builtin_claude_adapter_has_session_limit_pattern() {
        let adapters = builtin_adapters();
        let claude = adapters.get("claude-code").unwrap();
        assert!(claude
            .limit_patterns
            .contains(&"you've hit your session limit".to_string()));
    }
}
