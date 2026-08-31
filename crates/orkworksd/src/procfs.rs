use std::collections::HashMap;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Resolves the current working directory of every given pid in one
/// `sysinfo` scan, cross-platform (Linux/macOS/Windows) — so a poll covering
/// N sessions costs one OS process-table probe rather than N. Pids that are
/// gone, denied, or unsupported are simply absent from the returned map.
pub fn live_cwds(pids: &[u32]) -> HashMap<u32, String> {
    if pids.is_empty() {
        return HashMap::new();
    }
    let targets: Vec<Pid> = pids.iter().copied().map(Pid::from_u32).collect();
    let mut system = System::new();
    // `cwd` isn't collected by default; opt in explicitly rather than paying
    // for a full-process refresh.
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&targets),
        true,
        ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
    );
    targets
        .into_iter()
        .filter_map(|pid| {
            system
                .process(pid)
                .and_then(|process| process.cwd())
                .map(|path| (pid.as_u32(), path.display().to_string()))
        })
        .collect()
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

        let resolved = live_cwds(&[child.id()]).remove(&child.id());

        child.kill().ok();
        child.wait().ok();

        let resolved =
            std::path::PathBuf::from(resolved.expect("live_cwds should resolve a running process"));
        let expected = dir.canonicalize().unwrap_or(dir);
        assert_eq!(resolved.canonicalize().unwrap_or(resolved), expected);
    }

    #[test]
    fn live_cwds_resolves_multiple_pids_in_one_batched_scan() {
        let dir_a = std::env::temp_dir();
        let dir_b = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()));
        let mut child_a = Command::new(sleep_command())
            .arg(sleep_arg())
            .current_dir(&dir_a)
            .spawn()
            .expect("spawn short-lived child a");
        let mut child_b = Command::new(sleep_command())
            .arg(sleep_arg())
            .current_dir(&dir_b)
            .spawn()
            .expect("spawn short-lived child b");

        let resolved = live_cwds(&[child_a.id(), child_b.id()]);

        child_a.kill().ok();
        child_a.wait().ok();
        child_b.kill().ok();
        child_b.wait().ok();

        assert_eq!(resolved.len(), 2);
        assert!(resolved.contains_key(&child_a.id()));
        assert!(resolved.contains_key(&child_b.id()));
    }

    #[test]
    fn live_cwds_of_empty_slice_does_no_probing_and_returns_empty() {
        assert!(live_cwds(&[]).is_empty());
    }

    #[test]
    fn live_cwds_omits_a_pid_once_the_process_has_exited() {
        let mut child = Command::new(sleep_command())
            .arg(sleep_arg())
            .spawn()
            .expect("spawn short-lived child");
        let pid = child.id();
        child.kill().expect("kill child");
        child.wait().expect("wait for child exit");

        assert!(!live_cwds(&[pid]).contains_key(&pid));
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
