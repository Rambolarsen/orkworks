use crate::harness;
use crate::session_view::derive_memory_state;
use crate::session_types::MemoryState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex as StdMutex, OnceLock};

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
            is_builtin: true,
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
        },
        HarnessConfig {
            id: "codex".into(),
            name: "Codex".into(),
            harness: "generic-shell".into(),
            command: "codex".into(),
            args: vec![],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
        },
        HarnessConfig {
            id: "gemini".into(),
            name: "Gemini CLI".into(),
            harness: "generic-shell".into(),
            command: "gemini".into(),
            args: vec![],
            default_model: String::new(),
            model_prefix: String::new(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
        },
        HarnessConfig {
            id: "aider".into(),
            name: "Aider".into(),
            harness: "generic-shell".into(),
            command: "aider".into(),
            args: vec!["--model".into(), "{model}".into()],
            default_model: "claude-sonnet-4-20250514".into(),
            model_prefix: "ollama_chat/".into(),
            capabilities: HarnessVoiceCapabilities::default(),
            is_builtin: true,
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
        &[],
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
        &["usage limit reached"],
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
        &[],
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

    fn with_fake_home<T>(home: &std::path::Path, f: impl FnOnce() -> T) -> T {
        static HOME_LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
        let lock = HOME_LOCK.get_or_init(|| StdMutex::new(()));
        let _guard = lock.lock().unwrap();
        let previous = std::env::var_os("HOME");
        std::env::set_var("HOME", home);
        let result = f();
        if let Some(value) = previous {
            std::env::set_var("HOME", value);
        } else {
            std::env::remove_var("HOME");
        }
        result
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
                is_builtin: false,
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
}
