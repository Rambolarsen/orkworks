//! Detects whether a harness's configured launch command resolves to an
//! installed, executable binary — either on `PATH` or at an absolute
//! override path.

use std::env;
use std::path::Path;

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
}
