use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Resolves the current working directory of a running process by pid.
/// Cross-platform (Linux/macOS/Windows) via `sysinfo`; returns `None` if the
/// process is gone, the probe is denied, or the platform doesn't support it.
pub fn live_cwd(pid: u32) -> Option<String> {
    let target = Pid::from_u32(pid);
    let mut system = System::new();
    // `cwd` isn't collected by default; opt in explicitly rather than paying
    // for a full-process refresh.
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[target]),
        true,
        ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
    );
    system
        .process(target)
        .and_then(|process| process.cwd())
        .map(|path| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn live_cwd_resolves_a_running_child_process() {
        let dir = std::env::temp_dir();
        let mut child = Command::new(sleep_command())
            .arg(sleep_arg())
            .current_dir(&dir)
            .spawn()
            .expect("spawn short-lived child");

        let resolved = live_cwd(child.id());

        child.kill().ok();
        child.wait().ok();

        let resolved = std::path::PathBuf::from(resolved.expect("live_cwd should resolve a running process"));
        let expected = dir.canonicalize().unwrap_or(dir);
        assert_eq!(resolved.canonicalize().unwrap_or(resolved), expected);
    }

    #[test]
    fn live_cwd_returns_none_once_the_process_has_exited() {
        let mut child = Command::new(sleep_command())
            .arg(sleep_arg())
            .spawn()
            .expect("spawn short-lived child");
        let pid = child.id();
        child.kill().expect("kill child");
        child.wait().expect("wait for child exit");

        assert_eq!(live_cwd(pid), None);
    }

    #[cfg(unix)]
    fn sleep_command() -> &'static str {
        "sleep"
    }

    #[cfg(unix)]
    fn sleep_arg() -> &'static str {
        "5"
    }

    #[cfg(windows)]
    fn sleep_command() -> &'static str {
        "timeout"
    }

    #[cfg(windows)]
    fn sleep_arg() -> &'static str {
        "5"
    }
}
