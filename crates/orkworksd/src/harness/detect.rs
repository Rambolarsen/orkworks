//! Detects whether a harness's configured launch command resolves to an
//! installed, executable binary — either on `PATH` or at an absolute
//! override path.

use std::env;
use std::path::Path;
use std::time::Duration;

use super::definition::VersionRequirement;
use super::integration::DetectedTool;

pub(crate) fn probe_installed_tool(command: &str) -> Option<DetectedTool> {
    if command.is_empty() {
        return None;
    }
    let path = Path::new(command);
    if path.is_absolute() {
        return is_executable_file(path).then(|| DetectedTool {
            executable: path.to_path_buf(),
            version: None,
            compatible: true,
        });
    }
    let path_var = env::var_os("PATH")?;
    let candidates = if cfg!(windows) {
        windows_candidate_names(command, &env::var("PATHEXT").unwrap_or_default())
    } else {
        vec![command.to_string()]
    };
    for dir in env::split_paths(&path_var) {
        for candidate in &candidates {
            let full = dir.join(candidate);
            if is_executable_file(&full) {
                return Some(DetectedTool {
                    executable: full,
                    version: None,
                    compatible: true,
                });
            }
        }
    }
    None
}

/// Filenames to try for `command` in one `PATH` directory on Windows: the
/// bare name plus each `PATHEXT` extension, unless `command` already ends
/// in one of them. `pathext` is the raw `PATHEXT` env var value (semicolon
/// separated, e.g. `".COM;.EXE;.BAT"`); an empty/blank string falls back to
/// the common default `exe;cmd;bat`. This is a plain function (not gated
/// behind `cfg(windows)`) so its logic is unit-testable on any host — the
/// OS decision about whether to call it at all lives in
/// `probe_installed_tool` via `cfg!(windows)`.
fn windows_candidate_names(command: &str, pathext: &str) -> Vec<String> {
    let pathext = if pathext.trim().is_empty() {
        "exe;cmd;bat"
    } else {
        pathext
    };
    let extensions: Vec<String> = pathext
        .split(';')
        .map(|ext| ext.trim().trim_start_matches('.').to_lowercase())
        .filter(|ext| !ext.is_empty())
        .collect();
    let lower_command = command.to_lowercase();
    if extensions
        .iter()
        .any(|ext| lower_command.ends_with(&format!(".{ext}")))
    {
        return vec![command.to_string()];
    }
    extensions
        .iter()
        .map(|ext| format!("{command}.{ext}"))
        .collect()
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && meta.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

#[cfg(windows)]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

/// Caps how much of a probed tool's stdout/stderr is buffered. A version
/// string is a handful of bytes; this is generous headroom for a noisy
/// banner while still bounding memory if the tool is broken or hostile and
/// writes continuously for the whole probe timeout. Each stream gets its
/// own cap, so total worst-case buffering is `2 * MAX_PROBE_OUTPUT_BYTES`.
const MAX_PROBE_OUTPUT_BYTES: usize = 64 * 1024;

/// Runs `<executable> --version`, bounded to 3 seconds and to
/// `MAX_PROBE_OUTPUT_BYTES` per stream, and returns the combined
/// stdout+stderr text (some CLIs print version info to stderr; non-UTF8
/// bytes are lossily converted rather than treated as an error). Returns
/// `None` on any spawn error or timeout.
///
/// Reads stdout and stderr through an explicit byte cap (`AsyncReadExt::
/// take`) rather than `Command::output()`, which buffers both streams
/// without any size limit — a broken or hostile tool that writes
/// continuously for the full timeout would otherwise let a single Settings
/// status request buffer unbounded memory. Once each stream's cap is hit,
/// the read stops there (not an error) and any further output the child
/// writes just backs up in the OS pipe buffer, which the child sees as
/// ordinary write-side backpressure — it does not need to be drained for
/// the timeout or `kill_on_drop` to still take effect.
pub(crate) async fn probe_tool_version(executable: &Path) -> Option<String> {
    use tokio::io::AsyncReadExt;

    let mut command = tokio::process::Command::new(executable);
    command
        .arg("--version")
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let read_capped_output = async {
        let mut child = command.spawn().ok()?;
        let stdout = child.stdout.take()?;
        let stderr = child.stderr.take()?;
        let mut stdout_capped = stdout.take(MAX_PROBE_OUTPUT_BYTES as u64);
        let mut stderr_capped = stderr.take(MAX_PROBE_OUTPUT_BYTES as u64);
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();
        let (stdout_result, stderr_result) = tokio::join!(
            stdout_capped.read_to_end(&mut stdout_buf),
            stderr_capped.read_to_end(&mut stderr_buf),
        );
        stdout_result.ok()?;
        stderr_result.ok()?;
        // Deliberately not awaiting child.wait() here: once each stream's
        // cap is hit (or the child exits first, whichever comes first), the
        // probe already has everything it needs. A gluttonous writer that
        // keeps producing past the cap may never exit on its own once we
        // stop draining it (kernel pipe backpressure blocks its next
        // write()), so waiting for exit here would just re-block on the
        // outer 3s timeout for no benefit. `command.kill_on_drop(true)`
        // cleans up `child` when it drops at the end of this scope, the
        // same as it does on the timeout-cancellation path.
        Some((stdout_buf, stderr_buf))
    };

    let (stdout_buf, stderr_buf) = tokio::time::timeout(Duration::from_secs(3), read_capped_output)
        .await
        .ok()??;
    Some(
        String::from_utf8_lossy(&stdout_buf).into_owned()
            + &String::from_utf8_lossy(&stderr_buf),
    )
}

/// Extracts the first `major.minor[.patch]`-shaped token from arbitrary CLI
/// output. A generic heuristic, not a real version parser — see the design
/// spec for the accepted limitations (e.g. a noisy banner with an unrelated
/// leading numeric token). A missing patch component defaults to 0.
///
/// A token immediately followed by `-` or `+` (semver's prerelease/build-
/// metadata separators, e.g. `0.114.0-alpha.1`) is treated as unparseable
/// rather than accepted as its numeric prefix — a prerelease must not
/// silently compare equal to its corresponding stable release.
pub(crate) fn parse_version_token(text: &str) -> Option<(u64, u64, u64)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !(bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }
        let followed_by_prerelease_marker = bytes.get(i).is_some_and(|&b| b == b'-' || b == b'+');
        if followed_by_prerelease_marker {
            continue;
        }
        let token = &text[start..i];
        let parts: Vec<&str> = token.split('.').filter(|part| !part.is_empty()).collect();
        if !(2..=3).contains(&parts.len()) {
            continue;
        }
        let Some(numbers) = parts
            .iter()
            .map(|part| part.parse::<u64>().ok())
            .collect::<Option<Vec<u64>>>()
        else {
            continue;
        };
        return Some((numbers[0], numbers[1], numbers.get(2).copied().unwrap_or(0)));
    }
    None
}

/// Runs the full version-detection-and-gating pipeline: probes whether
/// `command` resolves to an installed binary, and if `min_version` is set
/// and the binary resolves, spawns `<executable> --version` to check the
/// version requirement. Returns `None` when the command is not found at all;
/// returns `Some(DetectedTool { compatible: false, .. })` when the binary
/// exists but no version string can be extracted, or when the extracted
/// version is below the requirement, or when the version is a prerelease.
pub(crate) async fn resolve_tool_gate(
    command: &str,
    min_version: Option<&VersionRequirement>,
) -> Option<DetectedTool> {
    let mut tool = probe_installed_tool(command)?;
    let Some(requirement) = min_version else {
        return Some(tool);
    };
    let version_output = probe_tool_version(&tool.executable).await;
    match version_output.as_deref().and_then(parse_version_token) {
        Some(parsed) => {
            tool.compatible = parsed >= requirement.min;
            tool.version = version_output;
        }
        None => {
            tool.compatible = false;
            tool.version = None;
        }
    }
    Some(tool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakePath;

    #[test]
    fn empty_command_returns_none() {
        assert!(probe_installed_tool("").is_none());
    }

    #[test]
    fn absolute_path_that_does_not_exist_returns_none() {
        assert!(probe_installed_tool("/definitely/not/a/real/path/xyz").is_none());
    }

    #[test]
    fn bare_command_not_found_on_path_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let _fake_path = FakePath::prepend(dir.path());
        assert!(probe_installed_tool("definitely-not-a-real-binary-xyz").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn bare_command_found_and_executable_on_path_returns_detected_tool() {
        use crate::test_support::make_test_executable;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("toolfake");
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        make_test_executable(&bin);
        let _fake_path = FakePath::prepend(dir.path());

        let detected = probe_installed_tool("toolfake").expect("should be detected");
        assert_eq!(detected.executable, bin);
        assert_eq!(detected.version, None);
        assert!(detected.compatible);
    }

    #[cfg(unix)]
    #[test]
    fn bare_command_found_but_not_executable_returns_none() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("toolfake");
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        // Deliberately leave default (non-executable) permissions.
        let _fake_path = FakePath::prepend(dir.path());

        assert!(probe_installed_tool("toolfake").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn absolute_path_that_exists_and_is_executable_returns_detected_tool() {
        use crate::test_support::make_test_executable;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("toolfake");
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        make_test_executable(&bin);

        let detected =
            probe_installed_tool(bin.to_str().unwrap()).expect("should be detected");
        assert_eq!(detected.executable, bin);
    }

    #[cfg(unix)]
    #[test]
    fn absolute_path_that_exists_but_is_not_executable_returns_none() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("toolfake");
        fs::write(&bin, "#!/bin/sh\n").unwrap();

        assert!(probe_installed_tool(bin.to_str().unwrap()).is_none());
    }

    #[test]
    fn windows_candidate_names_appends_each_pathext_extension() {
        assert_eq!(
            windows_candidate_names("claude", "EXE;CMD;BAT"),
            vec!["claude.exe", "claude.cmd", "claude.bat"]
        );
    }

    #[test]
    fn windows_candidate_names_does_not_double_append_a_known_extension() {
        assert_eq!(
            windows_candidate_names("claude.cmd", "EXE;CMD;BAT"),
            vec!["claude.cmd"]
        );
    }

    #[test]
    fn windows_candidate_names_falls_back_to_default_extensions_when_pathext_is_empty() {
        assert_eq!(
            windows_candidate_names("claude", ""),
            vec!["claude.exe", "claude.cmd", "claude.bat"]
        );
    }

    #[test]
    fn parse_version_token_finds_a_three_component_version() {
        assert_eq!(parse_version_token("codex-cli 0.114.2\n"), Some((0, 114, 2)));
    }

    #[test]
    fn parse_version_token_accepts_a_missing_patch_component() {
        assert_eq!(parse_version_token("gemini version 2.9\n"), Some((2, 9, 0)));
    }

    #[test]
    fn parse_version_token_picks_the_first_numeric_token_in_a_noisy_banner() {
        // Known, accepted limitation (documented in the design spec): this is
        // a generic heuristic, not a real parser, so an unrelated leading
        // numeric token (a bundled dependency version here) wins over the
        // actual tool version later in the same banner.
        assert_eq!(
            parse_version_token("built against libfoo 6.2.1, tool 0.114.2"),
            Some((6, 2, 1))
        );
    }

    #[test]
    fn parse_version_token_returns_none_for_output_with_no_numeric_token() {
        assert_eq!(parse_version_token("no version information available"), None);
    }

    #[test]
    fn parse_version_token_returns_none_for_empty_output() {
        assert_eq!(parse_version_token(""), None);
    }

    #[test]
    fn parse_version_token_treats_a_prerelease_suffix_as_unparseable() {
        // 0.114.0-alpha.1 must not compare equal to the stable 0.114.0 —
        // prereleases sort below their corresponding stable release and may
        // lack the finalized feature a min_version threshold is gating on.
        // Returning None here (rather than silently stripping the suffix)
        // makes the caller's existing "can't confirm" -> compatible: false
        // path do the right thing without needing full semver ordering.
        assert_eq!(parse_version_token("codex-cli 0.114.0-alpha.1"), None);
    }

    #[test]
    fn parse_version_token_treats_a_build_metadata_suffix_as_unparseable() {
        assert_eq!(parse_version_token("tool 1.2.3+build.5"), None);
    }

    #[tokio::test]
    async fn probe_tool_version_returns_combined_stdout_and_stderr() {
        use crate::test_support::make_test_executable;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("fake-tool");
        std::fs::write(&bin, "#!/bin/sh\necho 'fake-tool 1.2.3'\n").unwrap();
        make_test_executable(&bin);

        let output = probe_tool_version(&bin).await.expect("should run");
        assert_eq!(parse_version_token(&output), Some((1, 2, 3)));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_tool_version_terminates_within_the_timeout_for_a_binary_that_never_stops_writing() {
        use crate::test_support::make_test_executable;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("firehose");
        // Writes ~3MB of filler — many times over MAX_PROBE_OUTPUT_BYTES
        // and any real OS pipe buffer. Once the capped stdout reader stops
        // draining past its limit, this binary blocks on write-side
        // backpressure and never exits on its own; since its stderr is
        // still open (the process hasn't exited), the read on that stream
        // only ever completes via child-exit-driven EOF, which never
        // happens here. So this case legitimately runs out the full probe
        // timeout before probe_tool_version returns None — that's the
        // correct, accepted worst case for a genuinely adversarial writer.
        // The guarantee this fix provides is bounded *memory* (verified
        // indirectly here: nothing about this test's assertions could pass
        // if the read attempted to buffer the full ~3MB before giving up)
        // and bounded *latency* — still bounded by the existing timeout,
        // just not below it. A well-behaved tool's stdout and stderr both
        // close together when it exits normally, long before either the
        // cap or the timeout matters — see the sibling test using a small,
        // real version string for that path.
        std::fs::write(
            &bin,
            "#!/bin/sh\ni=0\nwhile [ $i -lt 3000 ]; do printf '%1000s' x; i=$((i+1)); done\n",
        )
        .unwrap();
        make_test_executable(&bin);

        let start = std::time::Instant::now();
        let output = probe_tool_version(&bin).await;
        let elapsed = start.elapsed();

        assert_eq!(
            output, None,
            "a writer that never exits must time out, not return partial output"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(4),
            "must terminate via the existing 3s probe timeout rather than hanging forever, \
             took {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn probe_tool_version_returns_empty_text_for_a_silent_nonzero_exit() {
        use crate::test_support::make_test_executable;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("fake-tool");
        std::fs::write(&bin, "#!/bin/sh\nexit 1\n").unwrap();
        make_test_executable(&bin);

        // .output() succeeds regardless of exit code — it only fails if the
        // process can't be spawned at all. A silent nonzero exit is a
        // parse-failure case for the *caller*, not a spawn failure here.
        let output = probe_tool_version(&bin).await;
        assert_eq!(output, Some(String::new()));
        assert_eq!(parse_version_token(&output.unwrap()), None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_tool_version_kills_a_hanging_binary_when_the_timeout_fires() {
        use crate::test_support::make_test_executable;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("hangs-forever");
        let pidfile = dir.path().join("child.pid");
        // `exec` replaces the shell process image with `sleep`, so there is
        // exactly one process for kill_on_drop's SIGKILL to reach — without
        // `exec`, `sh` would fork `sleep` as a *grandchild* that
        // kill_on_drop's signal to the immediate child (`sh`) would never
        // reach, leaking an orphaned `sleep` even with the fix in place.
        std::fs::write(
            &bin,
            format!("#!/bin/sh\necho $$ > {}\nexec sleep 30\n", pidfile.display()),
        )
        .unwrap();
        make_test_executable(&bin);

        let start = std::time::Instant::now();
        let result = probe_tool_version(&bin).await;
        assert!(result.is_none(), "a timed-out probe must not return output");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "must not wait past the 3s timeout"
        );

        let pid: i32 = std::fs::read_to_string(&pidfile)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        // SIGKILL delivery/reaping is asynchronous — tokio reaps the killed
        // child via a background task on the *same* current-thread runtime
        // this test runs on, so the poll must yield with `tokio::time::sleep`
        // rather than block the thread with `std::thread::sleep`; blocking
        // here would starve that reaping task and make this loop spin until
        // timeout even though the kill succeeded.
        for _ in 0..20 {
            if unsafe { libc::kill(pid, 0) } != 0 {
                return; // ESRCH: process no longer exists.
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("child process {pid} is still alive after kill_on_drop should have killed it");
    }
}
