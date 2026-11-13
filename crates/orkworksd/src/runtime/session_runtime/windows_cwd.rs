//! Windows PTY launch directory handling for command shims.
use std::io;
use std::path::Path;

/// Windows canonicalization returns device paths (\\?\C:\...), but cmd shims
/// started through ConPTY cannot use them as a working directory. Keep the
/// canonical path in session metadata and simplify only the child's cwd.
pub(super) fn process_cwd(cwd: &str, program: &str) -> io::Result<String> {
    let is_cmd_shim = Path::new(program)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
        });
    if !is_cmd_shim {
        return Ok(cwd.to_owned());
    }

    if let Some(path) = cwd.strip_prefix(r"\\?\") {
        let bytes = path.as_bytes();
        if bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'\\'
        {
            return Ok(path.to_owned());
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Windows command shims require a local drive workspace",
        ));
    }
    if cwd.starts_with(r"\\") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Windows command shims require a local drive workspace",
        ));
    }
    Ok(cwd.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    #[test]
    fn npm_cmd_shim_starts_in_selected_workspace() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("repo with spaces");
        std::fs::create_dir(&workspace).unwrap();
        let record = root.path().join("launched-cwd.txt");
        let shim = root.path().join("codex.cmd");
        std::fs::write(
            &shim,
            format!("@echo off\r\ncd > \"{}\"\r\n", record.display()),
        )
        .unwrap();

        let canonical = workspace.canonicalize().unwrap();
        let canonical_text = canonical.to_str().unwrap();
        assert!(
            canonical_text.starts_with(r"\\?\"),
            "Windows canonicalization must exercise the device path"
        );
        let launch_cwd = process_cwd(canonical_text, shim.to_str().unwrap()).unwrap();
        assert_eq!(launch_cwd, canonical_text.trim_start_matches(r"\\?\"));
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut command = CommandBuilder::new(shim.to_str().unwrap());
        command.cwd(&launch_cwd);
        let mut child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);
        child.wait().unwrap();

        let actual = std::fs::read_to_string(&record).unwrap();
        assert_eq!(
            std::path::Path::new(actual.trim()).canonicalize().unwrap(),
            canonical,
            "cmd shim did not start in the selected workspace"
        );
    }

    #[test]
    fn only_command_shims_need_the_simplified_cwd() {
        let cwd = r"\\?\C:\repo";
        assert_eq!(process_cwd(cwd, "codex.exe").unwrap(), cwd);
        assert_eq!(process_cwd(cwd, "codex.CMD").unwrap(), r"C:\repo");
        assert_eq!(process_cwd(cwd, "codex.bat").unwrap(), r"C:\repo");
    }

    #[test]
    fn ordinary_drive_workspace_is_preserved() {
        let cwd = r"C:\repo with spaces";
        assert_eq!(process_cwd(cwd, "codex.cmd").unwrap(), cwd);
    }

    #[test]
    fn command_shims_reject_unc_workspaces() {
        assert_eq!(
            process_cwd(r"\\?\UNC\server\share\repo", "codex.cmd")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            process_cwd(r"\\server\share\repo", "codex.cmd")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
