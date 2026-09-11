//! Login-preserving recommendation profiles, separate from interactive/Peon argv.
//! Optional integrations are disabled; administrator-managed policy remains
//! authoritative (ADR 0054). This is not a guarantee of zero CLI side effects.
use super::{ProviderDefinition, ProviderOperationError, ProviderOperationErrorCode};
use serde_json::{json, Value};
use std::{fs, io::Read, path::Path, process::Command};

#[cfg(all(test, unix))]
mod transport_tests;

#[derive(Clone, Copy)]
enum Profile {
    Codex,
    Claude,
}

fn profile(definition: &ProviderDefinition) -> Option<Profile> {
    // An arbitrary command wrapper cannot claim the safety properties of a CLI.
    match (definition.id.as_str(), definition.command.as_str()) {
        ("codex", "codex") => Some(Profile::Codex),
        ("claude-code", "claude") => Some(Profile::Claude),
        _ => None,
    }
}

pub(super) fn supports(definition: &ProviderDefinition) -> bool {
    profile(definition).is_some()
}

fn invalid(message: &str) -> ProviderOperationError {
    ProviderOperationError {
        code: ProviderOperationErrorCode::Malformed,
        message: message.into(),
    }
}

pub(super) struct PreparedInference {
    pub(super) command: Command,
    pub(super) stdin: String,
    profile: Profile,
    // Own all temporary configuration until the process and pipe readers finish.
    _directory: tempfile::TempDir,
}

pub(super) fn prepare(
    definition: &ProviderDefinition,
    model: &str,
    effort: Option<&str>,
    prompt: String,
) -> Result<PreparedInference, ProviderOperationError> {
    let preferences = if matches!(profile(definition), Some(Profile::Codex)) {
        let auth_root = std::env::var_os("CODEX_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".codex")))
            .ok_or_else(|| invalid("could not locate existing Codex configuration"))?;
        if !auth_root.is_absolute() {
            return Err(invalid("Codex authentication directory must be absolute"));
        }
        read_codex_login_preferences(&auth_root.join("config.toml"))?
    } else {
        Vec::new()
    };
    prepare_with_preferences(definition, model, effort, prompt, &preferences)
}

fn prepare_with_preferences(
    definition: &ProviderDefinition,
    model: &str,
    effort: Option<&str>,
    prompt: String,
    preferences: &[String],
) -> Result<PreparedInference, ProviderOperationError> {
    let profile = profile(definition).ok_or_else(|| ProviderOperationError {
        code: ProviderOperationErrorCode::UnsupportedCapability,
        message: "provider has no verified inference-only transport".into(),
    })?;
    if model.is_empty()
        || model.len() > 256
        || !model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._/:@+".contains(c))
    {
        return Err(invalid("model contains unsupported characters"));
    }
    if effort.is_some_and(|value| !matches!(value, "low" | "medium" | "high")) {
        return Err(invalid(
            "unsupported reasoning effort for background inference",
        ));
    }
    if prompt.len() > 256 * 1024 {
        return Err(invalid("inference prompt exceeds 256 KiB"));
    }
    let directory = tempfile::Builder::new()
        .prefix("orkworks-inference-")
        .tempdir()
        .map_err(|_| invalid("could not create private inference directory"))?;
    let mut command = Command::new(&definition.command);
    command.current_dir(directory.path());
    super::set_inference_environment(&mut command);
    match profile {
        Profile::Claude => {
            command.args([
                "-p",
                "--safe-mode",
                "--tools",
                "",
                "--no-session-persistence",
                "--output-format",
                "json",
            ]);
            // Safe mode leaves managed policy settings/hooks authoritative.
            // Do not add --strict-mcp-config: Claude rejects that override when
            // a managed MCP file exists, even though no optional MCP is needed.
            command.arg(format!("--model={model}"));
            if let Some(effort) = effort {
                command.args(["--effort", effort]);
            }
        }
        Profile::Codex => {
            for setting in preferences {
                command.args(["--config", setting]);
            }
            let catalog = directory.path().join("models.json");
            let instructions = directory.path().join("instructions.txt");
            fs::write(&catalog, codex_catalog(model).to_string())
                .and_then(|()| fs::write(&instructions, "Honor administrator-managed policy. This task requests recommendations only: analyze the supplied reference data and return the requested JSON. Do not invoke tools, execute commands, edit files, or start coding sessions."))
                .and_then(|()| fs::write(directory.path().join(".orkworks-inference-root"), ""))
                .map_err(|_| invalid("could not prepare inference configuration"))?;
            command.args([
                "exec",
                "--ignore-user-config",
                "--ignore-rules",
                "--strict-config",
                "--ephemeral",
                "--skip-git-repo-check",
                "--sandbox",
                "read-only",
                "--json",
                "--color",
                "never",
            ]);
            command.arg(format!("--model={model}"));
            for setting in [
                "approval_policy=\"never\"",
                "web_search=\"disabled\"",
                // Empty tables do not erase inherited managed MCP entries.
                "mcp_servers={}",
                "notify=[]",
                "project_doc_max_bytes=0",
                "project_root_markers=[\".orkworks-inference-root\"]",
                "include_environment_context=false",
                "model_reasoning_summary=\"none\"",
                "tools.update_plan.enabled=false",
                "tools.experimental_request_user_input.enabled=false",
                "features.skip_host_skill_discovery=true",
            ] {
                command.args(["--config", setting]);
            }
            for feature in [
                "shell_tool",
                "unified_exec",
                "shell_snapshot",
                "code_mode",
                "code_mode_host",
                "apps",
                "plugins",
                "remote_plugin",
                "hooks",
                "memories",
                "external_agent_memory_import",
                "multi_agent",
                "multi_agent_v2",
                "goals",
                "skill_search",
                "workspace_dependencies",
                "browser_use",
                "browser_use_external",
                "in_app_browser",
                "computer_use",
                "image_generation",
                "view_image",
                "search_tool",
                "tool_search",
                "tool_suggest",
                "request_permissions",
                "request_permissions_tool",
                "deferred_executor",
            ] {
                command.args(["--config", &format!("features.{feature}=false")]);
            }
            for (key, path) in [
                ("model_catalog_json", catalog),
                ("model_instructions_file", instructions),
            ] {
                // JSON quoted strings are valid TOML basic strings for these paths.
                let path = path
                    .to_str()
                    .ok_or_else(|| invalid("inference directory is not UTF-8"))?;
                command.args(["--config", &format!("{key}={}", json!(path))]);
            }
            if let Some(effort) = effort {
                command.args([
                    "--config",
                    &format!("model_reasoning_effort={}", json!(effort)),
                ]);
            }
            command.arg("-");
        }
    }
    Ok(PreparedInference {
        command,
        stdin: prompt,
        profile,
        _directory: directory,
    })
}

fn codex_catalog(model: &str) -> Value {
    // Explicit selection only: no other model is present to route/fall back to.
    // This describes an inference task, not a coding-agent tool configuration.
    json!({"models": [{
        "slug": model, "display_name": model, "description": null,
        "base_instructions": "Analyze only the supplied reference data and return the requested JSON. Do not take actions.",
        "supported_reasoning_levels": [], "shell_type": "disabled",
        "visibility": "list", "supported_in_api": true, "priority": 0,
        "support_verbosity": false, "default_verbosity": null,
        "apply_patch_tool_type": null, "experimental_supported_tools": [],
        "truncation_policy": {"mode": "tokens", "limit": 10000},
        "include_skills_usage_instructions": false, "include_plugin_usage_instructions": false,
        "include_apps_usage_instructions": false, "supports_reasoning_summary_parameter": false,
        "supports_search_tool": false, "node_repl_disabled": true, "tool_mode": "direct",
        "input_modalities": ["text"]
    }]})
}

fn read_codex_login_preferences(path: &Path) -> Result<Vec<String>, ProviderOperationError> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(invalid("could not read existing Codex login preferences")),
    };
    let mut config = String::new();
    file.take(64 * 1024 + 1)
        .read_to_string(&mut config)
        .map_err(|_| invalid("could not read existing Codex login preferences"))?;
    if config.len() > 64 * 1024 {
        return Err(invalid("Codex configuration exceeds 64 KiB"));
    }
    parse_codex_login_preferences(&config)
}

fn parse_codex_login_preferences(config: &str) -> Result<Vec<String>, ProviderOperationError> {
    #[derive(serde::Deserialize, Default)]
    struct Features {
        secret_auth_storage: Option<bool>,
    }
    #[derive(serde::Deserialize)]
    struct Preferences {
        cli_auth_credentials_store: Option<String>,
        forced_login_method: Option<String>,
        forced_chatgpt_workspace_id: Option<String>,
        model_provider: Option<String>,
        chatgpt_base_url: Option<String>,
        #[serde(default)]
        features: Features,
    }
    let preferences: Preferences =
        toml::from_str(config).map_err(|_| invalid("invalid existing Codex configuration"))?;
    if preferences
        .model_provider
        .as_deref()
        .is_some_and(|provider| provider != "openai")
        || preferences.chatgpt_base_url.is_some()
    {
        return Err(invalid(
            "custom Codex backend requires a verified inference profile",
        ));
    }
    let mut settings = Vec::new();
    if let Some(store) = preferences.cli_auth_credentials_store {
        if !matches!(store.as_str(), "file" | "keyring" | "auto") {
            return Err(invalid("unsupported Codex login storage"));
        }
        settings.push(format!("cli_auth_credentials_store={}", json!(store)));
    }
    if let Some(method) = preferences.forced_login_method {
        if !matches!(method.as_str(), "chatgpt" | "api") {
            return Err(invalid("unsupported Codex login method"));
        }
        settings.push(format!("forced_login_method={}", json!(method)));
    }
    if let Some(workspace) = preferences.forced_chatgpt_workspace_id {
        if workspace.len() > 256 || workspace.chars().any(char::is_control) {
            return Err(invalid("invalid Codex login workspace"));
        }
        settings.push(format!("forced_chatgpt_workspace_id={}", json!(workspace)));
    }
    if let Some(enabled) = preferences.features.secret_auth_storage {
        settings.push(format!("features.secret_auth_storage={enabled}"));
    }
    Ok(settings)
}

impl PreparedInference {
    pub(super) fn check_version(
        &self,
        runner: &dyn super::ProviderRunner,
        id: &str,
    ) -> Result<(), ProviderOperationError> {
        let mut probe = Command::new(self.command.get_program());
        probe.arg("--version").current_dir(self._directory.path());
        probe.env_clear();
        for (key, value) in self.command.get_envs() {
            if let Some(value) = value {
                probe.env(key, value);
            } else {
                probe.env_remove(key);
            }
        }
        let result = runner.run_prepared(id, &mut probe, "", 5, None);
        if !result.success {
            return Err(ProviderOperationError {
                code: super::classify_invocation_error(&result.stderr),
                message: "CLI compatibility check failed; check the installed coding tool".into(),
            });
        }
        let version = match self.profile {
            Profile::Codex => result.stdout.trim().strip_prefix("codex-cli "),
            Profile::Claude => result.stdout.trim().strip_suffix(" (Claude Code)"),
        };
        let parsed = version.and_then(|version| {
            let parts = version
                .split('.')
                .map(str::parse::<u64>)
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            <[u64; 3]>::try_from(parts).ok()
        });
        let compatible = parsed.is_some_and(|version| match self.profile {
            Profile::Codex => version[0] == 0 && version >= [0, 153, 4],
            Profile::Claude => version[0] == 2 && version >= [2, 1, 236],
        });
        if compatible {
            Ok(())
        } else {
            Err(ProviderOperationError {
                code: ProviderOperationErrorCode::UnsupportedCapability,
                message: "background analysis requires Codex >=0.153.4 (<1.0) or Claude Code >=2.1.236 (<3.0); unrecognized versions are unsupported".into(),
            })
        }
    }

    pub(super) fn decode(&self, stdout: &str) -> Result<String, ProviderOperationError> {
        match self.profile {
            Profile::Claude => {
                let value: Value = serde_json::from_str(stdout)
                    .map_err(|_| invalid("invalid Claude inference response"))?;
                if value["type"] != "result"
                    || value["subtype"] != "success"
                    || value["is_error"] != false
                {
                    return Err(invalid("Claude did not complete inference successfully"));
                }
                value["result"]
                    .as_str()
                    .filter(|text| !text.trim().is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("Claude returned no inference text"))
            }
            Profile::Codex => decode_codex(stdout),
        }
    }
}

fn decode_codex(stdout: &str) -> Result<String, ProviderOperationError> {
    let mut answer = None;
    let mut complete = false;
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let event: Value =
            serde_json::from_str(line).map_err(|_| invalid("invalid Codex inference event"))?;
        if complete {
            return Err(invalid("unexpected event after Codex completion"));
        }
        match event["type"].as_str() {
            Some("thread.started" | "turn.started") => {}
            Some("item.started" | "item.updated" | "item.completed") => {
                match event["item"]["type"].as_str() {
                    Some("reasoning") => {}
                    Some("agent_message") => {
                        if event["type"] == "item.completed" {
                            answer = event["item"]["text"].as_str().map(str::to_owned);
                        }
                    }
                    _ => return Err(invalid("unexpected tool event in Codex inference")),
                }
            }
            Some("turn.completed") => complete = true,
            _ => return Err(invalid("Codex did not complete inference successfully")),
        }
    }
    answer
        .filter(|text| complete && !text.trim().is_empty())
        .ok_or_else(|| invalid("Codex returned no completed inference text"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{
        ProcessRunner, ProviderManager, ProviderRunner, ProviderSettingsPayload,
    };

    // Pure command construction tests never inspect the developer's CLI config.
    fn prepare(
        definition: &ProviderDefinition,
        model: &str,
        effort: Option<&str>,
        prompt: String,
    ) -> Result<PreparedInference, ProviderOperationError> {
        prepare_with_preferences(definition, model, effort, prompt, &[])
    }

    fn definition(id: &str) -> ProviderDefinition {
        ProviderManager::for_tests(ProviderSettingsPayload::default(), vec![])
            .definition(id)
            .unwrap()
    }

    #[test]
    fn profiles_preserve_auth_and_never_reuse_interactive_arguments() {
        for id in ["codex", "claude-code"] {
            let mut definition = definition(id);
            definition.default_args = vec![
                "--dangerously-bypass-approvals-and-sandbox".into(),
                "--resume=old-session".into(),
            ];
            let invocation = prepare(
                &definition,
                "chosen-model",
                Some("high"),
                "supplied context".into(),
            )
            .unwrap();
            let args: Vec<_> = invocation
                .command
                .get_args()
                .map(|arg| arg.to_str().unwrap())
                .collect();
            assert!(!args
                .iter()
                .any(|arg| arg.contains("dangerously") || arg.contains("resume")));
            assert!(args.contains(&"--model=chosen-model"));
            assert_eq!(invocation.stdin, "supplied context");
            assert!(invocation.command.get_current_dir().unwrap().is_dir());
            for (key, value) in invocation.command.get_envs() {
                if [
                    "HOME",
                    "CODEX_HOME",
                    "CLAUDE_CONFIG_DIR",
                    "ANTHROPIC_API_KEY",
                    "OPENAI_API_KEY",
                ]
                .contains(&key.to_str().unwrap())
                {
                    assert!(
                        value == std::env::var_os(key).as_deref(),
                        "login environment changed"
                    );
                }
            }
            if id == "claude-code" {
                assert!(args.contains(&"--safe-mode"));
                assert!(args.windows(2).any(|pair| pair == ["--tools", ""]));
            } else {
                assert!(args.contains(&"--ignore-user-config"));
                assert!(args.contains(&"--ignore-rules"));
                assert!(args.contains(&"features.hooks=false"));
                assert!(args.contains(&"features.plugins=false"));
                let catalog: Value = serde_json::from_str(
                    &fs::read_to_string(invocation._directory.path().join("models.json")).unwrap(),
                )
                .unwrap();
                assert_eq!(catalog["models"].as_array().unwrap().len(), 1);
                assert_eq!(catalog["models"][0]["slug"], "chosen-model");
                assert_eq!(catalog["models"][0]["shell_type"], "disabled");
                assert!(catalog["models"][0]["apply_patch_tool_type"].is_null());
                assert_eq!(
                    catalog["models"][0]["experimental_supported_tools"],
                    json!([])
                );
            }
            let path = invocation._directory.path().to_path_buf();
            drop(invocation);
            assert!(!path.exists());
        }
    }

    #[test]
    fn rejects_command_wrappers_and_argument_injection_before_dispatch() {
        for (id, command) in [("codex", "/tmp/codex"), ("claude-code", "./claude")] {
            let mut wrapper = definition(id);
            wrapper.command = command.into();
            assert!(profile(&wrapper).is_none());
        }
        let mut wrapper = definition("codex");
        wrapper.command = "sh".into();
        assert!(!supports(&wrapper));
        assert!(prepare(&wrapper, "chosen", None, "context".into()).is_err());
        for model in ["", "a\n--config=notify", "a;touch /tmp/x", "$(id)"] {
            assert!(prepare(&definition("codex"), model, None, "context".into()).is_err());
        }
        assert!(prepare(
            &definition("codex"),
            "chosen",
            Some("high;run"),
            "context".into()
        )
        .is_err());
    }

    #[test]
    fn decoder_rejects_tool_events_errors_and_incomplete_results() {
        for output in [
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"file_change\"}}\n{\"type\":\"turn.completed\"}",
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"{}\"}}",
            "{\"type\":\"turn.failed\"}",
            "{\"type\":\"turn.completed\"}", "not json",
        ] { assert!(decode_codex(output).is_err(), "{output}"); }
        let claude = prepare(&definition("claude-code"), "chosen", None, "context".into()).unwrap();
        assert!(claude.decode("{\"type\":\"result\",\"subtype\":\"error_max_turns\",\"is_error\":true,\"result\":\"{}\"}").is_err());
    }

    #[test]
    #[ignore = "requires installed Codex; validates local configuration only, never runs inference"]
    fn installed_codex_accepts_inference_configuration_without_new_login() {
        let invocation =
            super::prepare(&definition("codex"), "probe-model", None, "".into()).unwrap();
        let args: Vec<_> = invocation.command.get_args().collect();
        let mut command = Command::new("codex");
        command.args(["features", "list"]);
        command.current_dir(invocation._directory.path());
        for pair in args.windows(2) {
            if pair[0] == "--config" {
                command.args(pair);
            }
        }
        let result = ProcessRunner.run_prepared("codex", &mut command, "", 10, None);
        assert!(result.success, "{}", result.stderr);
        for feature in [
            "shell_tool",
            "plugins",
            "hooks",
            "view_image",
            "multi_agent",
        ] {
            let fields: Vec<_> = result
                .stdout
                .lines()
                .find(|line| line.split_whitespace().next() == Some(feature))
                .unwrap()
                .split_whitespace()
                .collect();
            assert_eq!(fields.last(), Some(&"false"), "{feature}");
        }
    }

    #[test]
    fn oversized_prompt_is_rejected_before_any_cli_can_run() {
        assert!(prepare(
            &definition("codex"),
            "chosen",
            None,
            "x".repeat(256 * 1024 + 1)
        )
        .is_err());
    }

    #[test]
    fn preserves_only_nonsecret_codex_login_preferences() {
        let settings = parse_codex_login_preferences("cli_auth_credentials_store = 'keyring'\nforced_login_method = 'chatgpt'\nforced_chatgpt_workspace_id = 'existing-workspace'\nnotify = ['untrusted-command']\n[features]\nsecret_auth_storage = true\n[model_providers.custom]\nexperimental_bearer_token = 'must-not-copy'\n").unwrap();
        assert_eq!(
            settings,
            vec![
                "cli_auth_credentials_store=\"keyring\"",
                "forced_login_method=\"chatgpt\"",
                "forced_chatgpt_workspace_id=\"existing-workspace\"",
                "features.secret_auth_storage=true",
            ]
        );
        assert!(parse_codex_login_preferences("model_provider='custom'").is_err());
        assert!(parse_codex_login_preferences("cli_auth_credentials_store='unsupported'").is_err());
    }
}
