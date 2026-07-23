use serde::{Deserialize, Serialize};

pub(crate) mod definition;
pub(crate) mod registry;
pub(crate) mod store;

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

pub(crate) fn default_shell_command(cwd: impl Into<String>) -> CommandSpec {
    let (program, args) = if cfg!(target_os = "windows") {
        (
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()),
            vec![],
        )
    } else {
        (
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
            vec!["-i".into(), "-l".into()],
        )
    };
    CommandSpec {
        program,
        args,
        cwd: cwd.into(),
    }
}

pub(crate) fn render_command_template(
    template: &CommandTemplate,
    cwd: &str,
    repo_root: Option<&str>,
    harness_session_id: Option<&str>,
    model: Option<&str>,
) -> CommandSpec {
    let repo_root = repo_root.unwrap_or(cwd);
    let harness_session_id = harness_session_id.unwrap_or_default();
    let model = model.unwrap_or_default();
    CommandSpec {
        program: template.command.clone(),
        args: template
            .args
            .iter()
            .map(|arg| {
                arg.replace("{harnessSessionId}", harness_session_id)
                    .replace("{cwd}", cwd)
                    .replace("{repoRoot}", repo_root)
                    .replace("{model}", model)
            })
            .collect(),
        cwd: cwd.into(),
    }
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
