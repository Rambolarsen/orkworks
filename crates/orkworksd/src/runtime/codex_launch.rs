//! Preserve the owning OrkWorks environment when Codex supports a shared daemon.

use super::terminal_runtime::should_forward_terminal_env;
use crate::harness::CommandSpec;
use crate::startup_failure::{StartupError, StartupFailureCode as Code};
use std::time::Duration;

const MAX_HELP_BYTES: u64 = 64 * 1024;
const PROBE_ERROR: &str =
    "Could not check Codex session isolation support. Check the configured executable and retry.";

/// Probe before PTY spawn so launch and exact resume use the same isolation.
/// Older CLIs without the option retain their existing arguments.
pub(crate) async fn isolate_session(command: &mut CommandSpec) -> Result<(), StartupError> {
    let tool = crate::harness::detect::probe_installed_tool(&command.program).ok_or_else(|| {
        tracing::warn!(
            reason = "executable not found",
            "Codex session-isolation probe failed"
        );
        StartupError::classified(Code::CodexHelpFailed, PROBE_ERROR)
    })?;
    let mut probe = std::process::Command::new(tool.executable);
    // Help wrappers must receive neither launcher authority nor an ambient session capability.
    probe
        .env_clear()
        .envs(std::env::vars_os().filter(|(key, _)| {
            key.to_str()
                .map(|key| {
                    let key = key.to_ascii_uppercase();
                    should_forward_terminal_env(&key) && !key.starts_with("ORKWORKS_")
                })
                .unwrap_or(true)
        }));
    probe
        .arg("--help")
        .current_dir(&command.cwd)
        .stdin(std::process::Stdio::null());
    let output = crate::probe_process::run(
        &mut probe,
        crate::probe_process::ProbePolicy {
            max_bytes: MAX_HELP_BYTES as usize,
            reject_overflow: true,
            require_success: true,
        },
        Duration::from_secs(3),
    )
    .await
    .map_err(|error| {
        let code = match error {
            crate::probe_process::ProbeError::Timeout => Code::CodexHelpTimeout,
            crate::probe_process::ProbeError::Overflow => Code::CodexHelpOversized,
            _ => Code::CodexHelpFailed,
        };
        tracing::warn!(stage=?error, "Codex session-isolation probe failed");
        StartupError::classified(code, PROBE_ERROR)
    })?;
    let help = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let help = crate::peon::strip_ansi(&help);
    if help
        .lines()
        .any(|line| line.split_whitespace().next() == Some("--no-daemon"))
        && !command
            .args
            .iter()
            .take_while(|arg| arg.as_str() != "--")
            .any(|arg| arg == "--no-daemon")
    {
        command.args.insert(0, "--no-daemon".into());
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    async fn prepare(script: &str, args: &[&str]) -> Result<Vec<String>, StartupError> {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("codex-help-fixture");
        std::fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
        crate::test_support::make_test_executable(&executable);
        let mut command = CommandSpec {
            program: executable.display().to_string(),
            args: args.iter().map(|arg| (*arg).into()).collect(),
            cwd: dir.path().display().to_string(),
        };
        isolate_session(&mut command).await?;
        Ok(command.args)
    }

    // Removing the stream separator would hide an option behind stdout's banner.
    #[tokio::test]
    async fn mixed_help_streams_preserve_the_advertised_isolation_option() {
        let args = prepare(
            "printf 'warning'; printf '  --no-daemon  independent\\n' >&2",
            &["resume", "saved-id"],
        )
        .await
        .unwrap();
        assert_eq!(args, ["--no-daemon", "resume", "saved-id"]);
    }

    #[tokio::test]
    async fn styled_help_preserves_the_advertised_isolation_option() {
        let args = prepare(
            "printf '  \\033[1;32m--no-daemon\\033[0m  independent\\n'",
            &["resume", "saved-id"],
        )
        .await
        .unwrap();
        assert_eq!(args, ["--no-daemon", "resume", "saved-id"]);
    }

    // Run with controlled ambient values in another test process, never mutate
    // the parallel runner's environment. Removing the filter exposes a capability.
    #[tokio::test]
    async fn help_probe_filters_environment_in_an_isolated_process() {
        const CHILD: &str = "CODEX_PROBE_FILTER_TEST_CHILD";
        const BLOCKED: &[&str] = &[
            "ORKWORKS_SESSION_ID",
            "ORKWORKS_REPORT_TOKEN",
            "ORKWORKS_PORT",
            "ORKWORKS_OPEN_PLAN_TOKEN",
            "ORKWORKS_CODEX_SESSION_REPORT_DIR",
            "OrKwOrKs_RePoRt_ToKeN",
            "NODE_OPTIONS",
            "VSCODE_INSPECTOR_OPTIONS",
            "VSCODE_PID",
            "ELECTRON_RUN_AS_NODE",
        ];
        if std::env::var_os(CHILD).is_some() {
            let script = format!(
                "for key in {}; do if printenv \"$key\" >/dev/null; then exit 17; fi; done\ntest \"$CODEX_PROBE_ALLOWED\" = kept || exit 18\nprintf '  --no-daemon\\n'",
                BLOCKED.join(" ")
            );
            let args = prepare(&script, &[]).await.unwrap();
            assert_eq!(args, ["--no-daemon"]);
            return;
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "runtime::codex_launch::tests::help_probe_filters_environment_in_an_isolated_process", "--nocapture"])
            .env(CHILD, "1")
            .env("CODEX_PROBE_ALLOWED", "kept")
            .envs(BLOCKED.iter().map(|key| (*key, "fixture-only")))
            .output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[tokio::test]
    async fn older_codex_and_similar_option_names_preserve_arguments() {
        let args = prepare(
            "printf 'Options:\n  --no-daemon-extra  unrelated\n'",
            &["resume", "saved-id"],
        )
        .await
        .unwrap();
        assert_eq!(args, ["resume", "saved-id"]);
    }

    #[tokio::test]
    async fn help_on_stderr_and_an_existing_flag_do_not_duplicate_isolation() {
        let args = prepare(
            "printf '  --no-daemon  independent\n' >&2",
            &["--no-daemon", "resume", "saved-id"],
        )
        .await
        .unwrap();
        assert_eq!(args, ["--no-daemon", "resume", "saved-id"]);
    }

    #[tokio::test]
    async fn a_prompt_after_the_option_separator_does_not_count_as_a_flag() {
        let args = prepare(
            "printf '  --no-daemon  independent\n'",
            &["--", "--no-daemon"],
        )
        .await
        .unwrap();
        assert_eq!(args, ["--no-daemon", "--", "--no-daemon"]);
    }

    #[tokio::test]
    async fn failed_help_cannot_authorize_a_shared_launch() {
        let error = prepare("printf '\\033[31msecret-token\\n'; exit 1", &[])
            .await
            .unwrap_err();
        assert_eq!(error.code, Some(Code::CodexHelpFailed));
        assert!(!error.to_string().contains("secret-token"));
    }

    #[tokio::test]
    async fn oversized_help_cannot_hide_an_isolation_option() {
        assert_eq!(
            prepare("head -c 65537 /dev/zero", &[])
                .await
                .unwrap_err()
                .code,
            Some(Code::CodexHelpOversized)
        );
    }

    #[tokio::test]
    async fn a_hung_help_probe_is_bounded() {
        let start = std::time::Instant::now();
        assert_eq!(
            prepare("exec sleep 30", &[]).await.unwrap_err().code,
            Some(Code::CodexHelpTimeout)
        );
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
