//! Preserve the owning OrkWorks environment when Codex supports a shared daemon.

use super::terminal_runtime::should_forward_terminal_env;
use crate::harness::CommandSpec;
use std::time::Duration;
use tokio::io::AsyncReadExt;

const MAX_HELP_BYTES: u64 = 64 * 1024;
const PROBE_ERROR: &str =
    "Could not check Codex session isolation support. Check the configured executable and retry.";

/// Probe before PTY spawn so launch and exact resume use the same isolation.
/// Older CLIs without the option retain their existing arguments.
pub(crate) async fn isolate_session(command: &mut CommandSpec) -> Result<(), String> {
    let tool = crate::harness::detect::probe_installed_tool(&command.program)
        .ok_or_else(|| PROBE_ERROR.to_string())?;
    let mut probe = tokio::process::Command::new(tool.executable);
    // Help wrappers receive the same authority boundary as the PTY child.
    probe
        .env_clear()
        .envs(std::env::vars_os().filter(|(key, _)| {
            key.to_str()
                .map(should_forward_terminal_env)
                .unwrap_or(true)
        }));
    probe
        .arg("--help")
        .current_dir(&command.cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let read_help = async {
        let mut child = probe.spawn().ok()?;
        let mut stdout = child.stdout.take()?.take(MAX_HELP_BYTES + 1);
        let mut stderr = child.stderr.take()?.take(MAX_HELP_BYTES + 1);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let (out_result, err_result) =
            tokio::join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut err),);
        out_result.ok()?;
        err_result.ok()?;
        // Do not wait on an oversized writer: dropping the child kills it.
        if out.len() as u64 > MAX_HELP_BYTES || err.len() as u64 > MAX_HELP_BYTES {
            return None;
        }
        if !child.wait().await.ok()?.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out).into_owned() + &String::from_utf8_lossy(&err))
    };
    let help = tokio::time::timeout(Duration::from_secs(3), read_help)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| PROBE_ERROR.to_string())?;
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

    async fn prepare(script: &str, args: &[&str]) -> Result<Vec<String>, String> {
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
        assert!(prepare("printf '  --no-daemon\n'; exit 1", &[])
            .await
            .is_err());
    }

    #[tokio::test]
    async fn oversized_help_cannot_hide_an_isolation_option() {
        assert!(prepare("head -c 65537 /dev/zero", &[]).await.is_err());
    }

    #[tokio::test]
    async fn a_hung_help_probe_is_bounded() {
        let start = std::time::Instant::now();
        assert!(prepare("exec sleep 30", &[]).await.is_err());
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
