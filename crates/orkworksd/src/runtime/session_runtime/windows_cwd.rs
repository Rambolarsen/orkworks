//! Windows PTY launch directory handling for command shims.
use std::io;

pub(super) fn process_cwd(cwd: &str, _program: &str) -> io::Result<String> {
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
}
