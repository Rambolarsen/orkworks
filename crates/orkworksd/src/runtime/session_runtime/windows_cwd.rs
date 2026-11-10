//! Windows PTY launch directory handling for command shims.
use std::io;

pub(super) fn process_cwd(cwd: &str, _program: &str) -> io::Result<String> {
    Ok(cwd.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npm_cmd_shim_starts_in_selected_workspace() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("repo with spaces");
        std::fs::create_dir(&workspace).unwrap();
        let shim = root.path().join("codex.cmd");
        std::fs::write(&shim, "@echo off\r\ncd\r\n").unwrap();

        let canonical = workspace.canonicalize().unwrap().to_string_lossy().into_owned();
        assert!(canonical.starts_with(r"\\?\"), "Windows canonicalization must exercise the device path");
        let launch_cwd = process_cwd(&canonical, shim.to_str().unwrap()).unwrap();
        let output = std::process::Command::new(&shim)
            .current_dir(&launch_cwd)
            .output()
            .unwrap();
        assert!(output.status.success());
        let actual = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let expected = workspace.to_string_lossy();
        assert!(
            actual.eq_ignore_ascii_case(&expected),
            "cmd shim started in {actual:?}, expected {expected:?}; stderr={:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
