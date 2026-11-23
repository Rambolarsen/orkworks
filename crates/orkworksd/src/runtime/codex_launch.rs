//! Preserve the owning OrkWorks environment when Codex supports a shared daemon.

use super::terminal_runtime::should_forward_terminal_env;
use crate::harness::CommandSpec;
use crate::startup_failure::{StartupError, StartupFailureCode as Code};
use std::time::Duration;

const MAX_HELP_BYTES: u64 = 64 * 1024;
const PROBE_ERROR: &str =
    "Could not check Codex session isolation support. Check the configured executable and retry.";

/// Successful help classification only; not native eligibility or launch authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IsolationAdvertisement {
    Advertised,
    NotAdvertised,
}

/// Probe before PTY spawn so launch and exact resume use the same isolation.
/// Older CLIs without the option retain their existing arguments.
pub(crate) async fn isolate_session(command: &mut CommandSpec) -> Result<(), StartupError> {
    let advertisement = probe_isolation_advertisement(command).await?;
    command.args = render_isolated_arguments(&command.args, advertisement);
    Ok(())
}

pub(crate) async fn probe_isolation_advertisement(
    command: &CommandSpec,
) -> Result<IsolationAdvertisement, StartupError> {
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
    Ok(isolation_advertisement(&output.stdout, &output.stderr))
}

pub(crate) fn isolation_advertisement(stdout: &[u8], stderr: &[u8]) -> IsolationAdvertisement {
    let help = format!(
        "{}\n{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    );
    let help = crate::peon::strip_ansi(&help);
    if help
        .lines()
        .any(|line| line.split_whitespace().next() == Some("--no-daemon"))
    {
        IsolationAdvertisement::Advertised
    } else {
        IsolationAdvertisement::NotAdvertised
    }
}

pub(crate) fn render_isolated_arguments(
    args: &[String],
    advertisement: IsolationAdvertisement,
) -> Vec<String> {
    let mut rendered = args.to_vec();
    if advertisement == IsolationAdvertisement::Advertised
        && !args
            .iter()
            .take_while(|arg| arg.as_str() != "--")
            .any(|arg| arg == "--no-daemon")
    {
        rendered.insert(0, "--no-daemon".into());
    }
    rendered
}

#[cfg(test)]
mod pure_tests {
    use super::*;

    // Losing the stream separator or broadening option matching changes the command.
    #[test]
    fn only_an_exact_first_token_advertises_isolation() {
        use IsolationAdvertisement::{Advertised, NotAdvertised};
        let cases: &[(&[u8], &[u8], IsolationAdvertisement)] = &[
            (b"", b"", NotAdvertised),
            (b"--no-daemon", b"", Advertised),
            (b"warning", b"  --no-daemon  independent\n", Advertised),
            (
                b"  \x1b[1;32m--no-daemon\x1b[0m  independent\n",
                b"",
                Advertised,
            ),
            (b"\xff\n --no-daemon\n", b"", Advertised),
            (b"--no-\xffdaemon", b"", NotAdvertised),
            (b"usage: codex --no-daemon", b"", NotAdvertised),
            (b"--no-daemon-extra", b"", NotAdvertised),
            (b"prefix--no-daemon", b"", NotAdvertised),
        ];
        for &(stdout, stderr, expected) in cases {
            assert_eq!(isolation_advertisement(stdout, stderr), expected);
        }
    }

    // A flag after `--` is prompt text; existing duplicates/empty entries stay exact.
    #[test]
    fn rendering_preserves_arguments_and_only_inserts_a_missing_option() {
        use IsolationAdvertisement::{Advertised, NotAdvertised};
        let cases: &[(&[&str], IsolationAdvertisement, &[&str])] = &[
            (&[], Advertised, &["--no-daemon"]),
            (&[""], Advertised, &["--no-daemon", ""]),
            (
                &["resume", "saved-id"],
                Advertised,
                &["--no-daemon", "resume", "saved-id"],
            ),
            (
                &["--no-daemon", "--no-daemon", "resume"],
                Advertised,
                &["--no-daemon", "--no-daemon", "resume"],
            ),
            (
                &["--", "--no-daemon"],
                Advertised,
                &["--no-daemon", "--", "--no-daemon"],
            ),
            (
                &["--no-daemon-extra"],
                Advertised,
                &["--no-daemon", "--no-daemon-extra"],
            ),
            (&[], NotAdvertised, &[]),
            (
                &["resume", "", "saved-id"],
                NotAdvertised,
                &["resume", "", "saved-id"],
            ),
            (
                &["--", "--no-daemon"],
                NotAdvertised,
                &["--", "--no-daemon"],
            ),
        ];
        for &(args, advertisement, expected) in cases {
            let args: Vec<String> = args.iter().map(|arg| (*arg).into()).collect();
            assert_eq!(render_isolated_arguments(&args, advertisement), expected);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn help_fixture(script: &str, args: &[&str]) -> (tempfile::TempDir, CommandSpec) {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("codex-help-fixture");
        std::fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
        crate::test_support::make_test_executable(&executable);
        let command = CommandSpec {
            program: executable.display().to_string(),
            args: args.iter().map(|arg| (*arg).into()).collect(),
            cwd: dir.path().display().to_string(),
        };
        (dir, command)
    }

    async fn prepare(script: &str, args: &[&str]) -> Result<Vec<String>, StartupError> {
        let (_dir, mut command) = help_fixture(script, args);
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
            "ORKWORKS_RENDERER_TOKEN",
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
        let (_dir, mut command) = help_fixture(
            "printf 'Options:\n  --no-daemon-extra  unrelated\n'",
            &["resume", "", "saved-id"],
        );
        let before = command.clone();
        isolate_session(&mut command).await.unwrap();
        assert_eq!(command, before);
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
        let (_dir, mut command) = help_fixture(
            "printf '\\033[31msecret-token\\n'; exit 1",
            &["resume", "", "saved-id"],
        );
        let before = command.clone();
        let error = isolate_session(&mut command).await.unwrap_err();
        assert_eq!(command, before);
        assert_eq!(error.code, Some(Code::CodexHelpFailed));
        assert!(!error.to_string().contains("secret-token"));
    }

    #[tokio::test]
    async fn oversized_help_cannot_hide_an_isolation_option() {
        let script = "head -c 65537 /dev/zero";
        let (_dir, mut command) = help_fixture(script, &[]);
        #[cfg(target_os = "macos")]
        {
            // Payload bounds need an executable fixture, independently of first
            // script startup under load. Later writes preserve this same inode.
            std::fs::write(&command.program, "#!/bin/sh\nexit 0\n").unwrap();
            let mut preparation = std::process::Command::new(&command.program);
            preparation.env_clear().current_dir(&command.cwd);
            let output = crate::probe_process::run(
                &mut preparation,
                crate::probe_process::ProbePolicy {
                    max_bytes: 1,
                    reject_overflow: true,
                    require_success: true,
                },
                Duration::from_secs(30),
            )
            .await
            .expect("synthetic oversized-help fixture preparation failed");
            assert!(output.stdout.is_empty() && output.stderr.is_empty());
            std::fs::write(&command.program, format!("#!/bin/sh\n{script}\n")).unwrap();
        }
        let before = command.clone();
        assert_eq!(
            isolate_session(&mut command).await.unwrap_err().code,
            Some(Code::CodexHelpOversized)
        );
        assert_eq!(command, before);
    }

    #[tokio::test]
    async fn a_hung_help_probe_is_bounded() {
        let (_dir, mut command) = help_fixture("exec sleep 30", &["resume", "saved-id"]);
        let before = command.clone();
        let start = std::time::Instant::now();
        assert_eq!(
            isolate_session(&mut command).await.unwrap_err().code,
            Some(Code::CodexHelpTimeout)
        );
        assert_eq!(command, before);
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
