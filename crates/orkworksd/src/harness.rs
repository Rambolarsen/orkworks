use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandTemplate {
    pub command: String,
    pub args: Vec<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarnessCapabilities {
    pub launch: bool,
    pub resume_exact: bool,
    pub resume_latest_in_cwd: bool,
    pub resume_latest_in_repo: bool,
    pub detect_session_id: bool,
    pub detect_model: bool,
    pub detect_context_usage: bool,
    pub detect_capacity: bool,
    pub native_voice: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResumeState {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResumeStrategy {
    Exact,
    LatestCwd,
    LatestRepo,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeMemory {
    pub state: ResumeState,
    #[serde(rename = "preferredStrategy")]
    pub preferred_strategy: ResumeStrategy,
    #[serde(rename = "harnessSessionId", skip_serializing_if = "Option::is_none")]
    pub harness_session_id: Option<String>,
    #[serde(rename = "latestFallback")]
    pub latest_fallback: bool,
    #[serde(rename = "lastSeenAt", skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeRequest {
    pub strategy: ResumeStrategy,
    pub cwd: String,
    pub repo_root: Option<String>,
    pub harness_session_id: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct LaunchRequest {
    pub cwd: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HarnessAdapter {
    pub id: String,
    pub display_name: String,
    pub capabilities: HarnessCapabilities,
    pub limit_patterns: Vec<String>,
    launch_template: CommandTemplate,
    exact_resume_template: Option<CommandTemplate>,
    latest_cwd_resume_template: Option<CommandTemplate>,
    latest_repo_resume_template: Option<CommandTemplate>,
}

impl HarnessAdapter {
    #[allow(dead_code)]
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

    pub fn build_resume_command(&self, request: &ResumeRequest) -> Option<CommandSpec> {
        let template = match request.strategy {
            ResumeStrategy::Exact => self.exact_resume_template.as_ref()?,
            ResumeStrategy::LatestCwd => self.latest_cwd_resume_template.as_ref()?,
            ResumeStrategy::LatestRepo => self.latest_repo_resume_template.as_ref()?,
            ResumeStrategy::None => return None,
        };
        Some(render_template(template, request))
    }

    #[allow(dead_code)]
    pub fn build_launch_command(&self, request: &LaunchRequest) -> CommandSpec {
        render_launch_template(&self.launch_template, request)
    }
}

pub fn select_resume_strategy(memory: &ResumeMemory, capabilities: &HarnessCapabilities) -> ResumeStrategy {
    if memory.state != ResumeState::Available {
        return ResumeStrategy::None;
    }
    if capabilities.resume_exact && memory.harness_session_id.is_some() {
        return ResumeStrategy::Exact;
    }
    if memory.latest_fallback && capabilities.resume_latest_in_cwd {
        return ResumeStrategy::LatestCwd;
    }
    if memory.latest_fallback && capabilities.resume_latest_in_repo {
        return ResumeStrategy::LatestRepo;
    }
    ResumeStrategy::None
}

fn render_template(template: &CommandTemplate, request: &ResumeRequest) -> CommandSpec {
    let session_id = request.harness_session_id.as_deref().unwrap_or("");
    let repo_root = request.repo_root.as_deref().unwrap_or(&request.cwd);
    let model = request.model.as_deref().unwrap_or("");
    let args = template
        .args
        .iter()
        .map(|arg| {
            arg.replace("{harnessSessionId}", session_id)
                .replace("{cwd}", &request.cwd)
                .replace("{repoRoot}", repo_root)
                .replace("{model}", model)
        })
        .collect();

    CommandSpec {
        program: template.command.clone(),
        args,
        cwd: request.cwd.clone(),
    }
}

#[allow(dead_code)]
fn render_launch_template(template: &CommandTemplate, request: &LaunchRequest) -> CommandSpec {
    let model = request.model.as_deref().unwrap_or("");
    let args = template
        .args
        .iter()
        .map(|arg| arg.replace("{cwd}", &request.cwd).replace("{model}", model))
        .collect();

    CommandSpec {
        program: template.command.clone(),
        args,
        cwd: request.cwd.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps() -> HarnessCapabilities {
        HarnessCapabilities {
            launch: true,
            resume_exact: true,
            resume_latest_in_cwd: true,
            resume_latest_in_repo: true,
            detect_session_id: true,
            detect_model: true,
            detect_context_usage: true,
            detect_capacity: true,
            native_voice: false,
        }
    }

    #[test]
    fn exact_resume_wins_when_session_id_exists() {
        let memory = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("sess-123".into()),
            latest_fallback: true,
            last_seen_at: Some("2026-06-17T12:00:00Z".into()),
        };

        assert_eq!(select_resume_strategy(&memory, &caps()), ResumeStrategy::Exact);
    }

    #[test]
    fn latest_cwd_is_fallback_without_exact_id() {
        let mut capabilities = caps();
        capabilities.resume_exact = true;
        let memory = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: None,
        };

        assert_eq!(
            select_resume_strategy(&memory, &capabilities),
            ResumeStrategy::LatestCwd,
        );
    }

    #[test]
    fn unsupported_resume_returns_none_strategy() {
        let mut capabilities = caps();
        capabilities.resume_exact = false;
        capabilities.resume_latest_in_cwd = false;
        capabilities.resume_latest_in_repo = false;
        let memory = ResumeMemory {
            state: ResumeState::Unavailable,
            preferred_strategy: ResumeStrategy::None,
            harness_session_id: Some("sess-123".into()),
            latest_fallback: false,
            last_seen_at: None,
        };

        assert_eq!(select_resume_strategy(&memory, &capabilities), ResumeStrategy::None);
    }

    #[test]
    fn template_adapter_builds_exact_resume_command() {
        let adapter = HarnessAdapter::template(
            "custom",
            "Custom Harness",
            HarnessCapabilities {
                launch: true,
                resume_exact: true,
                resume_latest_in_cwd: true,
                resume_latest_in_repo: false,
                detect_session_id: false,
                detect_model: false,
                detect_context_usage: false,
                detect_capacity: false,
                native_voice: false,
            },
            vec![],
            CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--start".into()],
            },
            Some(CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--resume".into(), "{harnessSessionId}".into()],
            }),
            Some(CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--continue".into(), "--cwd".into(), "{cwd}".into()],
            }),
        );
        let request = ResumeRequest {
            strategy: ResumeStrategy::Exact,
            cwd: "/repo".into(),
            repo_root: Some("/repo".into()),
            harness_session_id: Some("sess-123".into()),
            model: Some("model-a".into()),
        };

        let command = adapter.build_resume_command(&request).unwrap();

        assert_eq!(command.program, "custom-ai");
        assert_eq!(command.args, vec!["--resume", "sess-123"]);
        assert_eq!(command.cwd, "/repo");
    }

    #[test]
    fn template_adapter_builds_launch_command() {
        let adapter = HarnessAdapter::template(
            "custom",
            "Custom Harness",
            HarnessCapabilities {
                launch: true,
                resume_exact: false,
                resume_latest_in_cwd: false,
                resume_latest_in_repo: false,
                detect_session_id: false,
                detect_model: false,
                detect_context_usage: false,
                detect_capacity: false,
                native_voice: false,
            },
            vec![],
            CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--model".into(), "{model}".into()],
            },
            None,
            None,
        );

        let command = adapter.build_launch_command(&LaunchRequest {
            cwd: "/repo".into(),
            model: Some("model-a".into()),
        });

        assert_eq!(command.program, "custom-ai");
        assert_eq!(command.args, vec!["--model", "model-a"]);
        assert_eq!(command.cwd, "/repo");
    }

    #[test]
    fn adapter_config_creates_template_adapter_without_code_changes() {
        let config = HarnessAdapterConfig {
            id: "custom".into(),
            display_name: "Custom Harness".into(),
            capabilities: caps(),
            limit_patterns: vec![],
            launch: CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--run".into()],
            },
            resume_exact: Some(CommandTemplate {
                command: "custom-ai".into(),
                args: vec!["--resume".into(), "{harnessSessionId}".into()],
            }),
            resume_latest_cwd: None,
            resume_latest_repo: None,
        };

        let adapter = HarnessAdapter::from_config(config);

        assert_eq!(adapter.id, "custom");
        assert_eq!(adapter.display_name, "Custom Harness");
        assert!(adapter.capabilities.resume_exact);
    }
}
