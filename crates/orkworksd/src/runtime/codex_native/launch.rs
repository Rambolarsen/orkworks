use super::*;
use crate::harness::probe_cache::{VersionProbeCache, VersionProbeCacheKey};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use tokio::io::AsyncReadExt;

pub(super) const TOKEN_ENV: &str = "ORKWORKS_CODEX_NATIVE_AUTH";
pub(super) struct Capability(String);
impl Capability {
    fn fresh() -> Result<Self, NativeError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| NativeError::Unavailable)?;
        Ok(Self(hex::encode(bytes)))
    }
    pub(super) fn value(&self) -> &str {
        &self.0
    }
    fn digest(&self) -> String {
        hex::encode(Sha256::digest(self.0.as_bytes()))
    }
}
#[derive(PartialEq, Eq)]
pub(super) struct ExecutableIdentity {
    size: u64,
    modified: std::time::SystemTime,
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(unix)]
    ctime: i64,
    #[cfg(unix)]
    ctime_nsec: i64,
}
impl ExecutableIdentity {
    pub(super) fn read(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Some(Self {
            size: metadata.len(),
            modified: metadata.modified().ok()?,
            #[cfg(unix)]
            dev: metadata.dev(),
            #[cfg(unix)]
            ino: metadata.ino(),
            #[cfg(unix)]
            ctime: metadata.ctime(),
            #[cfg(unix)]
            ctime_nsec: metadata.ctime_nsec(),
        })
    }
    fn key(&self) -> String {
        #[cfg(unix)]
        {
            format!(
                "{}:{}:{}:{}:{}:{:?}",
                self.dev, self.ino, self.ctime, self.ctime_nsec, self.size, self.modified
            )
        }
        #[cfg(not(unix))]
        {
            format!("{}:{:?}", self.size, self.modified)
        }
    }
}
pub(super) fn resolve(program: &str) -> Option<PathBuf> {
    if program != "codex"
        && (!Path::new(program).is_absolute()
            || Path::new(program).file_name()?.to_str() != Some("codex"))
    {
        return None;
    }
    let path = crate::harness::detect::probe_installed_tool(program)?
        .executable
        .canonicalize()
        .ok()?;
    if path.file_name()?.to_str() != Some("codex") {
        return None;
    }
    Some(path)
}
pub(super) fn canonical_binary(path: &Path) -> bool {
    let mut prefix = [0u8; 4];
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    if file.read_exact(&mut prefix).is_err() {
        return false;
    }
    // Reject script/npm wrappers even if they advertise the same version and options.
    prefix == [0x7f, b'E', b'L', b'F']
        || matches!(
            prefix,
            [0xcf, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xcf]
                | [0xce, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xce]
                | [0xca, 0xfe, 0xba, 0xbe]
                | [0xbe, 0xba, 0xfe, 0xca]
        )
}

// The leader is never reaped before its process group is torn down. Its unreaped
// PID reserves the group identity even when the leader exits before descendants.
pub(super) struct OwnedProcess {
    child: Option<Child>,
}
#[cfg(unix)]
fn signal_probe_indicates_process_exists(result: libc::c_int, errno: libc::c_int) -> bool {
    result == 0 || errno == libc::EPERM
}
#[cfg(unix)]
fn process_group_exists(group_id: libc::pid_t) -> bool {
    // SAFETY: signal zero only probes a process group and does not deliver a
    // signal or change process state.
    let result = unsafe { libc::kill(-group_id, 0) };
    let errno = std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or_default();
    signal_probe_indicates_process_exists(result, errno)
}
impl OwnedProcess {
    #[cfg(unix)]
    fn spawn(mut command: Command) -> Result<Self, NativeError> {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
        let child = command.spawn().map_err(|_| NativeError::Unavailable)?;
        Ok(Self { child: Some(child) })
    }
    #[cfg(not(unix))]
    fn spawn(_: Command) -> Result<Self, NativeError> {
        Err(NativeError::Unavailable)
    }
    #[cfg(unix)]
    fn exit_state(&self) -> Option<bool> {
        let child = self.child.as_ref()?;
        // SAFETY: waitid uses this owned, unreaped child only. WNOWAIT preserves
        // the leader PID until all owned group signaling has finished.
        unsafe {
            let mut info: libc::siginfo_t = std::mem::zeroed();
            if libc::waitid(
                libc::P_PID,
                child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            ) != 0
            {
                return Some(false);
            }
            #[cfg(target_os = "linux")]
            let (pid, status) = (info.si_pid(), info.si_status());
            #[cfg(not(target_os = "linux"))]
            let (pid, status) = (info.si_pid, info.si_status);
            if pid == 0 {
                None
            } else {
                Some(info.si_code == libc::CLD_EXITED && status == 0)
            }
        }
    }
    pub(super) fn is_alive(&self) -> bool {
        #[cfg(unix)]
        {
            self.child.is_some() && self.exit_state().is_none()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }
    #[cfg(unix)]
    fn retains_identity(&self) -> bool {
        let Some(child) = self.child.as_ref() else {
            return false;
        };
        // SAFETY: read-only waitid on our child. Failure means ownership can no
        // longer be proved, so subsequent group signaling must be skipped.
        unsafe {
            let mut info: libc::siginfo_t = std::mem::zeroed();
            libc::waitid(
                libc::P_PID,
                child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            ) == 0
        }
    }
    #[cfg(unix)]
    fn group_has_live_processes(&self) -> bool {
        if !self.retains_identity() {
            return false;
        }
        let Some(child) = self.child.as_ref() else {
            return false;
        };
        let Ok(pid) = libc::pid_t::try_from(child.id()) else {
            return false;
        };
        if self.exit_state().is_none() {
            return true;
        }
        #[cfg(target_os = "linux")]
        {
            linux_group_has_live_descendants(pid, pid)
        }
        #[cfg(target_os = "macos")]
        {
            macos_group_has_live_descendants(pid, pid)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            // Native launch is disabled where we cannot distinguish live
            // descendants from zombies. The leader is still signaled above,
            // but group existence alone is not evidence of live work.
            false
        }
    }
    #[cfg(unix)]
    fn signal(&self, signal: libc::c_int) {
        if !self.retains_identity() {
            return;
        }
        if let Some(child) = &self.child {
            if let Ok(pid) = libc::pid_t::try_from(child.id()) {
                // SAFETY: process_group(0) made this child's PID the group ID.
                // It has not been reaped, so the group ID cannot have been reused.
                unsafe {
                    libc::kill(-pid, signal);
                }
            }
        }
    }
    pub(super) async fn shutdown(&mut self) {
        #[cfg(unix)]
        {
            self.signal(libc::SIGTERM);
            let deadline = Instant::now() + Duration::from_secs(2);
            while self.group_has_live_processes() && Instant::now() < deadline {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            if self.group_has_live_processes() {
                self.signal(libc::SIGKILL);
            }
            self.reap();
        }
    }
    fn reap(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Waiting after SIGKILL is independent of async startup cancellation.
            // A stuck OS process does not block the runtime's bounded cleanup.
            let _ = std::thread::Builder::new()
                .name("codex-native-reaper".into())
                .spawn(move || {
                    let _ = child.wait();
                });
        }
    }
}
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        #[cfg(unix)]
        self.signal(libc::SIGKILL);
        self.reap();
    }
}

#[cfg(target_os = "linux")]
fn linux_group_has_live_descendants(group_id: libc::pid_t, leader_id: libc::pid_t) -> bool {
    let entries = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return true,
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return true;
        };
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<libc::pid_t>().ok())
        else {
            continue;
        };
        if pid == leader_id {
            continue;
        }
        let stat = match std::fs::read_to_string(entry.path().join("stat")) {
            Ok(stat) => stat,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return true,
        };
        let Some(close_paren) = stat.rfind(')') else {
            return true;
        };
        let mut fields = stat[close_paren + 1..].split_whitespace();
        let Some(state) = fields.next() else {
            return true;
        };
        let Some(_parent) = fields.next() else {
            return true;
        };
        let Some(process_group) = fields.next().and_then(|value| value.parse().ok()) else {
            return true;
        };
        if process_group == group_id && !matches!(state, "Z" | "X" | "x") {
            return true;
        }
    }
    false
}

#[cfg(target_os = "macos")]
fn macos_group_has_live_descendants(group_id: libc::pid_t, leader_id: libc::pid_t) -> bool {
    use std::mem::size_of;

    let mut pids = vec![0 as libc::pid_t; 64];
    loop {
        let count = unsafe {
            // SAFETY: libproc writes at most the supplied byte length into the
            // initialized PID buffer; the process-group ID remains protected
            // by the unreaped leader while it is queried.
            libc::proc_listpgrppids(
                group_id,
                pids.as_mut_ptr().cast(),
                (pids.len() * size_of::<libc::pid_t>()) as libc::c_int,
            )
        };
        if count <= 0 {
            return count < 0 || {
                // libproc reports zero on failure as well as an empty list.
                // A still-existing group is therefore treated conservatively.
                process_group_exists(group_id)
            };
        }
        let count = count as usize;
        if count >= pids.len() {
            if pids.len() >= 4096 {
                return true;
            }
            pids.resize((pids.len() * 2).min(4096), 0);
            continue;
        }
        for pid in pids.iter().take(count).copied() {
            if pid <= 0 || pid == leader_id {
                continue;
            }
            let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
            let read = unsafe {
                // SAFETY: `info` has the exact size expected by this flavor of
                // proc_pidinfo and is initialized before reading.
                libc::proc_pidinfo(
                    pid,
                    libc::PROC_PIDTBSDINFO,
                    0,
                    info.as_mut_ptr().cast(),
                    size_of::<libc::proc_bsdinfo>() as libc::c_int,
                )
            };
            if read != size_of::<libc::proc_bsdinfo>() as libc::c_int {
                // The process may have exited after enumeration; any other
                // uncertainty retains the full grace period.
                // SAFETY: signal zero only probes the enumerated PID.
                let result = unsafe { libc::kill(pid, 0) };
                let errno = std::io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or_default();
                if signal_probe_indicates_process_exists(result, errno) {
                    return true;
                }
                continue;
            }
            let info = unsafe { info.assume_init() };
            if info.pbi_status != libc::SZOMB {
                return true;
            }
        }
        return false;
    }
}

#[cfg(unix)]
async fn probe_fenced(
    executable: &Path,
    cwd: &str,
    args: &[&str],
    current: &impl Fn() -> bool,
) -> Option<(String, String)> {
    if !current() {
        return None;
    }
    let mut command = Command::new(executable);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(std::env::vars_os().filter(|(key, _)| {
            key.to_str().is_some_and(|key| {
                let key = key.to_ascii_uppercase();
                !key.starts_with("ORKWORKS_")
                    && super::super::terminal_runtime::should_forward_terminal_env(&key)
            })
        }))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !current() {
        return None;
    }
    let mut owner = OwnedProcess::spawn(command).ok()?;
    let stdout = owner.child.as_mut()?.stdout.take()?;
    let stderr = owner.child.as_mut()?.stderr.take()?;
    let mut out = tokio::net::unix::pipe::Receiver::from_owned_fd(stdout.into())
        .ok()?
        .take(65537);
    let mut err = tokio::net::unix::pipe::Receiver::from_owned_fd(stderr.into())
        .ok()?
        .take(65537);
    let read = async {
        let mut output = Vec::new();
        let mut error = Vec::new();
        let (a, b) = tokio::join!(out.read_to_end(&mut output), err.read_to_end(&mut error));
        a.ok()?;
        b.ok()?;
        if output.len() > 65536 || error.len() > 65536 {
            return None;
        }
        while owner.is_alive() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        if owner.exit_state() != Some(true) {
            return None;
        }
        Some((
            String::from_utf8(output).ok()?,
            String::from_utf8(error).ok()?,
        ))
    };
    let result = tokio::time::timeout(Duration::from_secs(3), read)
        .await
        .ok()
        .flatten();
    if !current() {
        return None;
    }
    result
}
#[cfg(not(unix))]
async fn probe_fenced(
    _: &Path,
    _: &str,
    _: &[&str],
    _: &impl Fn() -> bool,
) -> Option<(String, String)> {
    None
}
fn canonical_version(stdout: &str, stderr: &str) -> Option<String> {
    if !stderr.is_empty() {
        return None;
    }
    let text = stdout.strip_suffix('\n').unwrap_or(stdout);
    let version = text.strip_prefix("codex-cli ")?;
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || (p.len() > 1 && p.starts_with('0'))
                || p.parse::<u64>().is_err()
        })
    {
        return None;
    }
    Some(version.to_owned())
}
pub(super) async fn version_fenced(
    executable: &Path,
    cwd: &str,
    identity: &ExecutableIdentity,
    cache: &VersionProbeCache,
    current: &impl Fn() -> bool,
) -> Option<String> {
    if !current() {
        return None;
    }
    let result = cache
        .probe_or_get(
            VersionProbeCacheKey {
                harness_id: "codex-native/v1".into(),
                launch_command: identity.key(),
                executable: executable.to_path_buf(),
            },
            Instant::now(),
            Duration::from_secs(30),
            Duration::from_secs(5),
            || async {
                let (out, err) = probe_fenced(executable, cwd, &["--version"], current).await?;
                canonical_version(&out, &err)
            },
        )
        .await;
    if !current() {
        return None;
    }
    result
}
pub(super) async fn features_fenced(
    executable: &Path,
    cwd: &str,
    current: &impl Fn() -> bool,
) -> bool {
    if !current() {
        return false;
    }
    let Some((help, err)) = probe_fenced(executable, cwd, &["--help"], current).await else {
        return false;
    };
    if !current() || !err.is_empty() || !help.starts_with("Codex CLI") {
        return false;
    }
    if !current() {
        return false;
    }
    let Some((server, err)) =
        probe_fenced(executable, cwd, &["app-server", "--help"], current).await
    else {
        return false;
    };
    if !current() || !err.is_empty() || !server.contains("Usage: codex app-server") {
        return false;
    }
    let has = |text: &str, flag: &str| {
        text.lines()
            .any(|line| line.split_whitespace().next() == Some(flag))
    };
    has(&help, "--remote")
        && has(&help, "--remote-auth-token-env")
        && has(&server, "--listen")
        && has(&server, "--ws-auth")
        && has(&server, "--ws-token-sha256")
}
// Existing lower-level probe fixtures inject their own executable without
// claiming a production compatibility entry. No unfenced wrapper is shipped.
#[cfg(test)]
async fn version(
    executable: &Path,
    cwd: &str,
    identity: &ExecutableIdentity,
    cache: &VersionProbeCache,
) -> Option<String> {
    version_fenced(executable, cwd, identity, cache, &|| true).await
}
#[cfg(test)]
async fn features(executable: &Path, cwd: &str) -> bool {
    features_fenced(executable, cwd, &|| true).await
}
#[cfg(test)]
async fn probe(executable: &Path, cwd: &str, args: &[&str]) -> Option<(String, String)> {
    probe_fenced(executable, cwd, args, &|| true).await
}
fn arguments(
    plan: &NativeLaunchPlan,
    endpoint: &str,
    token: &Capability,
) -> (Vec<String>, Vec<String>) {
    let mut server = plan.route.shared.clone();
    server.extend([
        "app-server".into(),
        "--listen".into(),
        endpoint.into(),
        "--ws-auth".into(),
        "capability-token".into(),
        "--ws-token-sha256".into(),
        token.digest(),
    ]);
    let mut tui = plan.route.shared.clone();
    tui.extend([
        "--remote".into(),
        endpoint.into(),
        "--remote-auth-token-env".into(),
        TOKEN_ENV.into(),
    ]);
    if let Some(id) = &plan.route.resume {
        tui.extend(["resume".into(), id.clone()]);
    }
    (server, tui)
}
// Only this module's controlled fake-native constructor can mint fixture provenance.
// It is absent from non-test builds and cannot be obtained from environment or records.
#[cfg(test)]
pub(super) struct ControlledFixtureListener {
    _private: (),
}
pub(super) async fn start(
    plan: NativeLaunchPlan,
    execution_env: &[(String, String)],
) -> Result<OwnedNativeRuntime, NativeError> {
    start_with_port_hook(plan, execution_env, |_| {}).await
}
// Private injection seam for deterministic fixture port-race tests. Production
// always supplies the no-op hook above; eligibility has no environment bypass.
async fn start_with_port_hook<F: FnMut(&str)>(
    plan: NativeLaunchPlan,
    execution_env: &[(String, String)],
    mut port_released: F,
) -> Result<OwnedNativeRuntime, NativeError> {
    // Neither port reservation, child liveness nor initialize shape proves listener ownership.
    // #763 must establish the owned-listener handoff before any non-fixture bearer delivery.
    #[cfg(test)]
    let controlled_fixture = plan.controlled_fixture.is_some();
    #[cfg(not(test))]
    let controlled_fixture = false;
    if !controlled_fixture || !cfg!(unix) {
        return Err(NativeError::Unavailable);
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    for _ in 0..3 {
        plan.revalidate()?;
        let reservation = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .map_err(|_| NativeError::Unavailable)?;
        let endpoint = format!(
            "ws://{}",
            reservation
                .local_addr()
                .map_err(|_| NativeError::Unavailable)?
        );
        let token = Capability::fresh()?;
        let (server_args, tui_args) = arguments(&plan, &endpoint, &token);
        let mut command = Command::new(&plan.executable);
        command
            .args(server_args)
            .current_dir(&plan.cwd)
            .env_clear()
            .envs(
                execution_env
                    .iter()
                    .filter(|(key, _)| !key.eq_ignore_ascii_case(TOKEN_ENV))
                    .map(|(key, value)| (key, value)),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        plan.revalidate()?;
        drop(reservation);
        port_released(&endpoint);
        let mut server = OwnedProcess::spawn(command)?;
        let readiness = async {
            loop {
                if !server.is_alive() {
                    return Err(NativeError::Unavailable);
                }
                match super::protocol::Client::connect(&endpoint, token.value(), plan.record).await
                {
                    Ok(client) if server.is_alive() => return Ok(client),
                    Ok(_) => return Err(NativeError::Unavailable),
                    Err(NativeError::Disconnected) => {
                        tokio::time::sleep(Duration::from_millis(25)).await
                    }
                    Err(error) => return Err(error),
                }
            }
        };
        match tokio::time::timeout_at(deadline, readiness).await {
            Ok(Ok(observer)) => {
                if let Err(error) = plan.revalidate() {
                    drop(observer);
                    server.shutdown().await;
                    return Err(error);
                }
                return Ok(OwnedNativeRuntime {
                    plan,
                    server,
                    endpoint,
                    token,
                    tui_args,
                    observer: Some(observer),
                    retry_at: Instant::now(),
                    backoff: Duration::from_millis(250),
                    permanent_error: None,
                });
            }
            Ok(Err(_)) => server.shutdown().await,
            Err(_) => {
                server.shutdown().await;
                return Err(NativeError::Timeout);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(NativeError::Timeout);
        }
    }
    Err(NativeError::Unavailable)
}
#[cfg(all(test, unix))]
pub(super) mod tests {
    use super::*;
    pub(crate) fn fixture_plan(
        dir: &Path,
        cache: &VersionProbeCache,
        resume: bool,
    ) -> NativeLaunchPlan {
        let executable = dir.join("codex");
        std::fs::write(&executable,r#"#!/bin/sh
case " $* " in
 *" --remote "*)
  printf '%s\n' "$@" > "$FIXTURE_DIR/tui-args"
  echo $$ > "$FIXTURE_DIR/tui-pid"
  if [ -n "${ORKWORKS_CODEX_NATIVE_AUTH-}" ]; then echo present; else echo absent; fi > "$FIXTURE_DIR/tui-auth-presence"
  echo fake-tui-ready
  if [ "${FIXTURE_TUI_EXIT-}" = yes ]; then exit 0; fi
  if [ "${FIXTURE_TUI_EXIT_ON_SERVER-}" = yes ]; then
   while [ ! -e "$FIXTURE_DIR/server-exited" ]; do sleep 0.005; done
   exit 0
  fi
  while :; do sleep 1; done
  ;;
esac
printf '%s\n' "$@" > "$FIXTURE_DIR/args"
pwd > "$FIXTURE_DIR/cwd"
if [ "${FIXTURE_REDACT_REPORT-}" = yes ]; then
 printf '%s\n' present "$ORKWORKS_CODEX_SESSION_REPORT_DIR" "$CODEX_HOME" "${ORKWORKS_CODEX_NATIVE_AUTH-absent}" > "$FIXTURE_DIR/env"
else
 printf '%s\n' "$ORKWORKS_REPORT_TOKEN" "$ORKWORKS_CODEX_SESSION_REPORT_DIR" "$CODEX_HOME" "${ORKWORKS_CODEX_NATIVE_AUTH-absent}" > "$FIXTURE_DIR/env"
fi
echo $$ > "$FIXTURE_DIR/pid"
if [ "$FIXTURE_MODE" = fail ]; then exit 1; fi
if [ "$FIXTURE_MODE" = hang ]; then exec sleep 30; fi
if [ "$FIXTURE_MODE" = refuse ]; then
 trap 'echo stopped > "$FIXTURE_DIR/shutdown-ack"; exit 0' TERM
 while :; do sleep 0.05; done
fi
while [ "$#" -gt 0 ]; do
 case "$1" in --listen) export FIXTURE_LISTEN="$2"; shift;; --ws-token-sha256) export FIXTURE_DIGEST="$2"; shift;; esac
 shift
done
if [ "${FIXTURE_TUI_EXIT_ON_SERVER-}" = yes ]; then
 "$FIXTURE_TEST_EXE" --exact runtime::codex_native::launch::tests::native_server_fixture --nocapture &
 fixture_server_pid=$!
 trap 'kill -TERM "$fixture_server_pid" 2>/dev/null || true; wait "$fixture_server_pid" 2>/dev/null || true; touch "$FIXTURE_DIR/server-exited"; exit 0' TERM
 wait "$fixture_server_pid"
 exit $?
fi
exec "$FIXTURE_TEST_EXE" --exact runtime::codex_native::launch::tests::native_server_fixture --nocapture
"#).unwrap();
        crate::test_support::make_test_executable(&executable);
        let executable = executable.canonicalize().unwrap();
        let args = if resume {
            vec![
                "-c",
                "model=\"configured\"",
                "--enable",
                "feature",
                "--disable",
                "other",
                "resume",
                "saved-id",
            ]
        } else {
            vec![]
        };
        NativeLaunchPlan {
            configured_program: executable.display().to_string(),
            identity: ExecutableIdentity::read(&executable).unwrap(),
            executable,
            epoch: cache.epoch(),
            cwd: dir.to_path_buf(),
            route: parse_arguments(&args.into_iter().map(String::from).collect::<Vec<_>>())
                .unwrap(),
            record: CompatibilityRecord {
                version: "0.160.0",
                os: std::env::consts::OS,
                arch: std::env::consts::ARCH,
                user_agent_prefix: "fixture/0.160.0",
                protocol: "v2-thread-status-0.160",
                root_proof: "fixture only",
                evidence: "fixture only",
            },
            controlled_fixture: Some(ControlledFixtureListener { _private: () }),
        }
    }
    pub(crate) fn fixture_env(dir: &Path, mode: &str) -> Vec<(String, String)> {
        [
            ("PATH", "/usr/bin:/bin".into()),
            ("FIXTURE_DIR", dir.display().to_string()),
            (
                "FIXTURE_TEST_EXE",
                std::env::current_exe().unwrap().display().to_string(),
            ),
            ("FIXTURE_MODE", mode.into()),
            ("ORKWORKS_REPORT_TOKEN", format!("report-{}", dir.display())),
            (
                "ORKWORKS_CODEX_SESSION_REPORT_DIR",
                dir.join("mailbox").display().to_string(),
            ),
            ("CODEX_HOME", dir.join("codex-home").display().to_string()),
            (TOKEN_ENV, "caller-native-sentinel".into()),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v))
        .collect()
    }
    #[tokio::test]
    async fn failed_readiness_acknowledges_owned_shutdown_before_returning() {
        let dir = tempfile::tempdir().unwrap();
        let cache = VersionProbeCache::new();
        let result = fixture_plan(dir.path(), &cache, false)
            .start(&fixture_env(dir.path(), "refuse"))
            .await;
        assert!(result.is_err());
        assert!(
            dir.path().join("shutdown-ack").exists(),
            "startup returned before graceful owner shutdown acknowledgement"
        );
    }

    #[tokio::test]
    async fn native_server_fixture() {
        use std::io::Write as _;

        let Ok(endpoint) = std::env::var("FIXTURE_LISTEN") else {
            return;
        };
        let listener = tokio::net::TcpListener::bind(endpoint.trim_start_matches("ws://"))
            .await
            .unwrap();
        let observation_requests = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        loop {
            let (tcp, _) = listener.accept().await.unwrap();
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(
                    std::path::PathBuf::from(std::env::var("FIXTURE_DIR").unwrap())
                        .join("server-connections"),
                )
                .unwrap()
                .write_all(b"connected\n")
                .unwrap();
            let observation_requests = observation_requests.clone();
            tokio::spawn(async move {
                use futures_util::{SinkExt, StreamExt};
                use serde_json::{json, Value};
                use tokio_tungstenite::tungstenite::Message;
                let Ok(mut socket)=tokio_tungstenite::accept_hdr_async(tcp,|request:&tokio_tungstenite::tungstenite::handshake::server::Request,response:tokio_tungstenite::tungstenite::handshake::server::Response| {
                    let valid=request.headers().get("Authorization").and_then(|v|v.to_str().ok()).and_then(|v|v.strip_prefix("Bearer ")).is_some_and(|token|hex::encode(Sha256::digest(token.as_bytes()))==std::env::var("FIXTURE_DIGEST").unwrap());
                    if !valid{return Err(tokio_tungstenite::tungstenite::http::Response::builder().status(401).body(None).unwrap());}Ok(response)
                }).await else{return;};
                while let Some(Ok(Message::Text(text))) = socket.next().await {
                    let value: Value = serde_json::from_str(&text).unwrap();
                    if value["method"] == "initialized" {
                        continue;
                    }
                    let mut close_observation = false;
                    let result = match value["method"].as_str().unwrap() {
                        "initialize" => {
                            json!({"userAgent":"fixture/0.160.0","codexHome":"/fixture","platformOs":std::env::consts::OS,"platformFamily":"unix"})
                        }
                        "thread/loaded/list" => {
                            let request = observation_requests
                                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                                + 1;
                            let disconnects = std::env::var("FIXTURE_DISCONNECT_OBSERVATIONS")
                                .ok()
                                .and_then(|value| value.parse::<usize>().ok())
                                .unwrap_or(0);
                            let closes = std::env::var("FIXTURE_CLOSE_OBSERVATIONS")
                                .ok()
                                .and_then(|value| value.parse::<usize>().ok())
                                .unwrap_or(0);
                            if request <= disconnects {
                                return;
                            }
                            if request <= closes {
                                close_observation = true;
                            }
                            if std::env::var("FIXTURE_PROTOCOL_FAILURE").ok().as_deref()
                                == Some("shape")
                            {
                                json!({"data":"wrong-shape","nextCursor":null})
                            } else {
                                json!({"data":["root"],"nextCursor":null})
                            }
                        }
                        "thread/read" => {
                            json!({"thread":{"id":"root","sessionId":"root","source":"mcp","cliVersion":"0.160.0","status":{"type":"active","activeFlags":[]}}})
                        }
                        _ => panic!("forbidden fixture method"),
                    };
                    if close_observation {
                        let _ = socket.send(Message::Close(None)).await;
                        return;
                    }
                    if socket
                        .send(Message::Text(
                            json!({"id":value["id"],"result":result}).to_string(),
                        ))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            });
        }
    }
    #[tokio::test]
    async fn two_owned_servers_keep_capabilities_arguments_and_execution_environments_separate() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let cache = VersionProbeCache::new();
        let mut first = fixture_plan(a.path(), &cache, true)
            .start(&fixture_env(a.path(), "serve"))
            .await
            .unwrap();
        let mut second = fixture_plan(b.path(), &cache, false)
            .start(&fixture_env(b.path(), "serve"))
            .await
            .unwrap();
        assert!(first.is_alive() && second.is_alive());
        assert!(first.tui_environment().1 != second.tui_environment().1);
        assert!(first
            .tui_args()
            .ends_with(&["resume".into(), "saved-id".into()]));
        assert!(first.tui_args().starts_with(&[
            "-c".into(),
            "model=\"configured\"".into(),
            "--enable".into(),
            "feature".into(),
            "--disable".into(),
            "other".into()
        ]));
        for (dir, runtime) in [(a.path(), &first), (b.path(), &second)] {
            let args = std::fs::read_to_string(dir.join("args")).unwrap();
            assert!(!args.contains(runtime.tui_environment().1));
            assert!(!runtime
                .tui_args()
                .join(" ")
                .contains(runtime.tui_environment().1));
            let environment = std::fs::read_to_string(dir.join("env")).unwrap();
            assert_eq!(
                environment,
                format!(
                    "report-{}\n{}\n{}\nabsent\n",
                    dir.display(),
                    dir.join("mailbox").display(),
                    dir.join("codex-home").display()
                )
            );
            assert_eq!(
                std::fs::read_to_string(dir.join("cwd")).unwrap().trim(),
                dir.canonicalize().unwrap().display().to_string()
            );
            runtime.revalidate_tui_executable().unwrap();
        }
        assert_eq!(
            first.observe("root").await.unwrap().status,
            NativeStatus::Active
        );
        first.observer = None;
        assert_eq!(
            first.observe("root").await.err(),
            Some(NativeError::Disconnected)
        );
        assert_eq!(
            first.observe("root").await.unwrap().status,
            NativeStatus::Active
        );
        first.shutdown().await;
        first.shutdown().await;
        assert!(!first.is_alive());
        assert!(second.is_alive());
        second.shutdown().await;
    }
    #[tokio::test]
    async fn startup_failure_cancellation_and_registry_change_never_leave_a_server() {
        let dir = tempfile::tempdir().unwrap();
        let cache = VersionProbeCache::new();
        assert!(fixture_plan(dir.path(), &cache, false)
            .start(&fixture_env(dir.path(), "fail"))
            .await
            .is_err());
        let plan = fixture_plan(dir.path(), &cache, false);
        let env = fixture_env(dir.path(), "hang");
        let task = tokio::spawn(async move { plan.start(&env).await });
        let file = dir.path().join("pid");
        for _ in 0..100 {
            if file.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        let pid = std::fs::read_to_string(&file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        task.abort();
        let _ = task.await;
        for _ in 0..100 {
            if !process_exists(pid) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(!process_exists(pid));
        let plan = fixture_plan(dir.path(), &cache, false);
        cache.bump_generation();
        assert!(plan.start(&fixture_env(dir.path(), "serve")).await.is_err());
    }
    #[tokio::test]
    async fn post_readiness_executable_or_registry_change_blocks_the_tui() {
        let dir = tempfile::tempdir().unwrap();
        let cache = VersionProbeCache::new();
        let mut runtime = fixture_plan(dir.path(), &cache, false)
            .start(&fixture_env(dir.path(), "serve"))
            .await
            .unwrap();
        cache.bump_generation();
        assert!(runtime.revalidate_tui_executable().is_err());
        runtime.shutdown().await;
        let mut runtime = fixture_plan(dir.path(), &cache, false)
            .start(&fixture_env(dir.path(), "serve"))
            .await
            .unwrap();
        std::fs::write(dir.path().join("codex"), "replaced executable").unwrap();
        assert!(runtime.revalidate_tui_executable().is_err());
        runtime.shutdown().await;
    }
    // Mirrors ordinary/installed construction, without controlled-fixture provenance.
    fn unproven_plan(dir: &Path, cache: &VersionProbeCache) -> NativeLaunchPlan {
        let fixture = fixture_plan(dir, cache, false);
        NativeLaunchPlan {
            executable: fixture.executable,
            configured_program: fixture.configured_program,
            identity: fixture.identity,
            epoch: fixture.epoch,
            cwd: fixture.cwd,
            route: fixture.route,
            record: fixture.record,
            controlled_fixture: None,
        }
    }

    struct CompetitorBearerReceipt(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl tokio_tungstenite::tungstenite::handshake::server::Callback for CompetitorBearerReceipt {
        fn on_request(
            self,
            request: &tokio_tungstenite::tungstenite::handshake::server::Request,
            response: tokio_tungstenite::tungstenite::handshake::server::Response,
        ) -> Result<
            tokio_tungstenite::tungstenite::handshake::server::Response,
            tokio_tungstenite::tungstenite::handshake::server::ErrorResponse,
        > {
            // Keep only the boolean; never retain, print, or persist the capability.
            if request.headers().contains_key("Authorization") {
                self.0.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            Ok(response)
        }
    }

    fn accepting_competitor(
        listener: std::net::TcpListener,
        bearer_received: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> tokio::task::JoinHandle<()> {
        listener.set_nonblocking(true).unwrap();
        let listener = tokio::net::TcpListener::from_std(listener).unwrap();
        tokio::spawn(async move {
            use futures_util::{SinkExt, StreamExt};
            use serde_json::{json, Value};
            use tokio_tungstenite::tungstenite::Message;
            loop {
                let (tcp, _) = listener.accept().await.unwrap();
                let Ok(mut socket) = tokio_tungstenite::accept_hdr_async(
                    tcp,
                    CompetitorBearerReceipt(bearer_received.clone()),
                )
                .await
                else {
                    continue;
                };
                while let Some(Ok(Message::Text(text))) = socket.next().await {
                    let value: Value = serde_json::from_str(&text).unwrap();
                    if value["method"] == "initialize" {
                        let result = json!({"userAgent":"fixture/0.160.0","codexHome":"/fixture","platformOs":std::env::consts::OS,"platformFamily":"unix"});
                        if socket
                            .send(Message::Text(
                                json!({"id":value["id"],"result":result}).to_string(),
                            ))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        })
    }

    #[tokio::test]
    async fn unproven_listener_never_delivers_bearer_to_accepting_competitor() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        let dir = tempfile::tempdir().unwrap();
        let cache = VersionProbeCache::new();
        let received = Arc::new(AtomicBool::new(false));
        // A competitor also survives rejection before the reservation hook is reached.
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let control_endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let mut competitors = vec![accepting_competitor(listener, received.clone())];
        let result = tokio::time::timeout(
            Duration::from_secs(7),
            start_with_port_hook(
                unproven_plan(dir.path(), &cache),
                &fixture_env(dir.path(), "hang"),
                |endpoint| {
                    let listener =
                        std::net::TcpListener::bind(endpoint.trim_start_matches("ws://")).unwrap();
                    competitors.push(accepting_competitor(listener, received.clone()));
                },
            ),
        )
        .await
        .expect("startup rejection must remain bounded");
        let accepted_readiness = result.is_ok();
        let accepted_live_child = result.as_ref().is_ok_and(|runtime| runtime.is_alive());
        if let Ok(mut runtime) = result {
            runtime.shutdown().await;
        }
        // Independently exercise the already-live competitor, without an Authorization header.
        // This proves survival is not just a hook that the early guard never invokes.
        tokio::time::timeout(Duration::from_secs(2), async {
            use futures_util::{SinkExt, StreamExt};
            use serde_json::{json, Value};
            use tokio_tungstenite::tungstenite::Message;
            let (mut socket, _) = tokio_tungstenite::connect_async(&control_endpoint)
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    json!({"id":1,"method":"initialize","params":{}}).to_string(),
                ))
                .await
                .unwrap();
            let frame = socket.next().await.unwrap().unwrap();
            let Message::Text(text) = frame else {
                panic!("competitor control response absent");
            };
            let value: Value = serde_json::from_str(&text).unwrap();
            let shape_correct = value["result"]
                == json!({"userAgent":"fixture/0.160.0","codexHome":"/fixture","platformOs":std::env::consts::OS,"platformFamily":"unix"});
            assert!(
                shape_correct,
                "competitor control initialize shape mismatch"
            );
            socket.close(None).await.unwrap();
        }).await.expect("surviving competitor control must remain usable");
        let competitor_survived = competitors.iter().all(|task| !task.is_finished());
        for task in competitors {
            task.abort();
            let _ = task.await;
        }
        assert!(
            !received.load(Ordering::SeqCst),
            "unproven listener received an Authorization capability"
        );
        assert!(
            !accepted_readiness,
            "shape-correct competitor was accepted while owned child alive={accepted_live_child}"
        );
        assert!(competitor_survived);
        assert!(
            !dir.path().join("pid").exists(),
            "rejection must precede native child spawn"
        );
        assert!(
            !dir.path().join("tui-args").exists(),
            "rejection must precede TUI augmentation"
        );
    }

    #[tokio::test]
    async fn installed_diagnostic_cannot_obtain_fixture_listener_provenance_or_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let cache = VersionProbeCache::new();
        let fixture = fixture_plan(dir.path(), &cache, false);
        let command = CommandSpec {
            program: fixture.configured_program,
            args: vec![],
            cwd: dir.path().display().to_string(),
        };
        let result = installed_diagnostic_plan(&command, &cache).await;
        assert!(matches!(result, Err(NativeError::Unavailable)));
        assert!(!dir.path().join("args").exists());
        assert!(!dir.path().join("pid").exists());
        assert!(!dir.path().join("tui-args").exists());
    }

    #[tokio::test]
    async fn port_competition_retries_bounded_attempts_without_attaching_to_the_competitor() {
        let dir = tempfile::tempdir().unwrap();
        let cache = VersionProbeCache::new();
        let mut tasks = Vec::new();
        let mut ports = Vec::new();
        let result=start_with_port_hook(fixture_plan(dir.path(),&cache,false),&fixture_env(dir.path(),"serve"),|endpoint| {
            let listener=std::net::TcpListener::bind(endpoint.trim_start_matches("ws://")).unwrap();
            ports.push(listener.local_addr().unwrap().port());listener.set_nonblocking(true).unwrap();
            let listener=tokio::net::TcpListener::from_std(listener).unwrap();
            tasks.push(tokio::spawn(async move {
                loop {let (tcp,_)=listener.accept().await.unwrap();
                    let _:Result<_,_>=tokio_tungstenite::accept_hdr_async(tcp,|_:&tokio_tungstenite::tungstenite::handshake::server::Request,_:tokio_tungstenite::tungstenite::handshake::server::Response| {
                        Err(tokio_tungstenite::tungstenite::http::Response::builder().status(401).body(None).unwrap())
                    }).await;
                }
            }));
        }).await;
        assert!(result.is_err());
        assert_eq!(ports.len(), 3);
        assert!(tasks.iter().all(|t| !t.is_finished()));
        for task in tasks {
            task.abort();
        }
    }
    #[tokio::test]
    async fn probes_filter_all_session_capabilities_and_native_secret_in_an_isolated_process() {
        const CHILD: &str = "NATIVE_PROBE_ENV_TEST_CHILD";
        const KEYS: &[&str] = &[
            "ORKWORKS_REPORT_TOKEN",
            "ORKWORKS_PORT",
            "ORKWORKS_SESSION_ID",
            "ORKWORKS_CODEX_NATIVE_AUTH",
            "OrKwOrKs_SeCrEt",
            "NODE_OPTIONS",
        ];
        if std::env::var_os(CHILD).is_some() {
            let dir = tempfile::tempdir().unwrap();
            let executable = dir.path().join("codex");
            let script = format!(
                r#"#!/bin/sh
for key in {}; do if printenv "$key" >/dev/null; then exit 9;fi;done
test "$NATIVE_PROBE_ALLOWED" = kept || exit 10
printf 'codex-cli 0.160.0\n'
"#,
                KEYS.join(" ")
            );
            std::fs::write(&executable, script).unwrap();
            crate::test_support::make_test_executable(&executable);
            let identity = ExecutableIdentity::read(&executable).unwrap();
            assert_eq!(
                version(
                    &executable,
                    dir.path().to_str().unwrap(),
                    &identity,
                    &VersionProbeCache::new()
                )
                .await
                .as_deref(),
                Some("0.160.0")
            );
            return;
        }
        let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","runtime::codex_native::launch::tests::probes_filter_all_session_capabilities_and_native_secret_in_an_isolated_process","--nocapture"]).env(CHILD,"1").env("NATIVE_PROBE_ALLOWED","kept").envs(KEYS.iter().map(|k|(*k,"fixture-only"))).output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[tokio::test]
    async fn exact_version_platform_and_protocol_records_are_required_even_with_cached_probe_data()
    {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("codex");
        std::fs::copy("/bin/sh", &executable).unwrap();
        let executable = executable.canonicalize().unwrap();
        let identity = ExecutableIdentity::read(&executable).unwrap();
        assert!(canonical_binary(&executable));
        let command = CommandSpec {
            program: executable.display().to_string(),
            args: vec![],
            cwd: dir.path().display().to_string(),
        };
        let record = CompatibilityRecord {
            version: "0.160.0",
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            user_agent_prefix: "fixture/0.160.0",
            protocol: "v2-thread-status-0.160",
            root_proof: "fixture",
            evidence: "fixture",
        };
        for version_text in ["0.159.0", "0.160.1", "1.0.0"] {
            let cache = VersionProbeCache::new();
            cache
                .probe_or_get(
                    VersionProbeCacheKey {
                        harness_id: "codex-native/v1".into(),
                        launch_command: identity.key(),
                        executable: executable.clone(),
                    },
                    Instant::now(),
                    Duration::from_secs(30),
                    Duration::from_secs(5),
                    || async { Some(version_text.into()) },
                )
                .await;
            assert!(
                super::super::eligible_with_records(&command, &cache, &[record])
                    .await
                    .unwrap()
                    .is_none()
            );
        }
        for unsupported in [
            CompatibilityRecord {
                arch: "other-architecture",
                ..record
            },
            CompatibilityRecord {
                protocol: "unreviewed-protocol",
                ..record
            },
            CompatibilityRecord {
                evidence: "",
                ..record
            },
        ] {
            assert!(super::super::eligible_with_records(
                &command,
                &VersionProbeCache::new(),
                &[unsupported]
            )
            .await
            .unwrap()
            .is_none());
        }
    }
    #[tokio::test]
    async fn inter_probe_epoch_change_during_version_stops_help_child() {
        assert_probe_mutation_stops_next_child("version", false).await;
    }
    #[tokio::test]
    async fn inter_probe_executable_replacement_during_version_stops_help_child() {
        assert_probe_mutation_stops_next_child("version", true).await;
    }
    #[tokio::test]
    async fn inter_probe_epoch_change_during_cli_help_stops_server_help_child() {
        assert_probe_mutation_stops_next_child("cli-help", false).await;
    }
    #[tokio::test]
    async fn inter_probe_executable_replacement_during_cli_help_stops_server_help_child() {
        assert_probe_mutation_stops_next_child("cli-help", true).await;
    }
    #[tokio::test]
    async fn inter_probe_epoch_change_during_server_help_revokes_final_result() {
        assert_probe_mutation_stops_next_child("server-help", false).await;
    }
    #[tokio::test]
    async fn inter_probe_executable_replacement_during_server_help_revokes_final_result() {
        assert_probe_mutation_stops_next_child("server-help", true).await;
    }
    async fn assert_probe_mutation_stops_next_child(stage: &str, replace: bool) {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("codex");
        let script = format!(
            r#"#!/bin/sh
case "$1" in --version) stage=version;; --help) stage=cli-help;; *) stage=server-help;; esac
printf '%s\n' "$stage" >> '{dir}/children'
if [ "$stage" = '{stage}' ]; then
 touch '{dir}/entered'
 while [ ! -f '{dir}/release' ]; do sleep 0.01; done
fi
case "$stage" in
 version) printf 'codex-cli 0.160.0\n';;
 cli-help) printf 'Codex CLI\n --remote endpoint\n --remote-auth-token-env name\n';;
 server-help) printf 'Usage: codex app-server\n --listen endpoint\n --ws-auth auth\n --ws-token-sha256 digest\n';;
esac
"#,
            dir = dir.path().display(),
            stage = stage
        );
        std::fs::write(&executable, &script).unwrap();
        crate::test_support::make_test_executable(&executable);
        let executable = executable.canonicalize().unwrap();
        let identity = ExecutableIdentity::read(&executable).unwrap();
        let cache = std::sync::Arc::new(VersionProbeCache::new());
        let epoch = cache.epoch();
        let command = CommandSpec {
            program: executable.display().to_string(),
            args: vec![],
            cwd: dir.path().display().to_string(),
        };
        let record = CompatibilityRecord {
            version: "0.160.0",
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            user_agent_prefix: "fixture/0.160.0",
            protocol: "v2-thread-status-0.160",
            root_proof: "fixture",
            evidence: "fixture",
        };
        let task = {
            let cache = cache.clone();
            let executable = executable.clone();
            tokio::spawn(async move {
                super::super::probe_record(
                    &command,
                    &cache,
                    &[record],
                    &executable,
                    &identity,
                    &epoch,
                )
                .await
            })
        };
        // Earlier bounded probes may take up to three seconds each under the
        // parallel fixture load. Wait on the actual barrier or task completion,
        // not a shorter one-second scheduling assumption.
        let barrier_deadline = Instant::now() + Duration::from_secs(8);
        while !dir.path().join("entered").exists()
            && !task.is_finished()
            && Instant::now() < barrier_deadline
        {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            dir.path().join("entered").exists(),
            "probe fixture did not reach {stage} barrier; finished={}, children={}",
            task.is_finished(),
            std::fs::read_to_string(dir.path().join("children")).unwrap_or_default()
        );
        if replace {
            let replacement = dir.path().join("replacement");
            std::fs::write(&replacement, &script).unwrap();
            crate::test_support::make_test_executable(&replacement);
            std::fs::rename(replacement, &executable).unwrap();
        } else {
            cache.bump_generation();
        }
        std::fs::write(dir.path().join("release"), "").unwrap();
        let result = task.await.unwrap();
        let children = std::fs::read_to_string(dir.path().join("children")).unwrap();
        let expected = match stage {
            "version" => "version\n",
            "cli-help" => "version\ncli-help\n",
            _ => "version\ncli-help\nserver-help\n",
        };
        assert_eq!(
            children, expected,
            "invalidated/replaced {stage} spawned a subsequent child"
        );
        assert!(
            result.is_none(),
            "invalidated/replaced probe retained eligibility"
        );
    }
    #[tokio::test]
    async fn native_cache_never_reuses_integration_outputs_or_replaced_executable_identity() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("codex");
        let cache = VersionProbeCache::new();
        std::fs::write(&executable, "#!/bin/sh\nprintf 'codex-cli 0.160.0\\n'\n").unwrap();
        crate::test_support::make_test_executable(&executable);
        cache
            .probe_or_get(
                VersionProbeCacheKey {
                    harness_id: "codex".into(),
                    launch_command: "codex".into(),
                    executable: executable.clone(),
                },
                Instant::now(),
                Duration::from_secs(30),
                Duration::from_secs(5),
                || async { Some("warning codex-cli 9.9.9".into()) },
            )
            .await;
        let identity = ExecutableIdentity::read(&executable).unwrap();
        assert_eq!(
            version(&executable, dir.path().to_str().unwrap(), &identity, &cache)
                .await
                .as_deref(),
            Some("0.160.0")
        );
        std::fs::write(&executable, "#!/bin/sh\nprintf 'codex-cli 0.160.1\\n'\n").unwrap();
        let changed = ExecutableIdentity::read(&executable).unwrap();
        assert!(identity != changed);
        assert_eq!(
            version(&executable, dir.path().to_str().unwrap(), &changed, &cache)
                .await
                .as_deref(),
            Some("0.160.1")
        );
        cache.bump_generation();
        assert_eq!(
            version(&executable, dir.path().to_str().unwrap(), &changed, &cache)
                .await
                .as_deref(),
            Some("0.160.1")
        );
    }
    #[tokio::test]
    async fn timeout_and_noisy_failed_truncated_help_cannot_authorize_features() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("codex");
        for script in [
            "printf 'warning';exit 1",
            "head -c 65537 /dev/zero",
            "printf 'Codex CLI\\n --remote-extra\\n --remote-auth-token-env\\n'",
        ] {
            std::fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
            crate::test_support::make_test_executable(&executable);
            assert!(!features(&executable, dir.path().to_str().unwrap()).await);
        }
        std::fs::write(&executable, "#!/bin/sh\nexec sleep 30\n").unwrap();
        let now = Instant::now();
        assert!(
            probe(&executable, dir.path().to_str().unwrap(), &["--version"])
                .await
                .is_none()
        );
        assert!(now.elapsed() < Duration::from_secs(5));
    }
    #[test]
    fn capabilities_are_distinct_and_never_put_plaintext_in_digest() {
        let a = Capability::fresh().unwrap();
        let b = Capability::fresh().unwrap();
        assert_eq!(a.value().len(), 64);
        assert_ne!(a.value(), b.value());
        assert_ne!(a.value(), a.digest());
    }
    #[test]
    fn permission_denied_signal_probe_keeps_owned_process_live() {
        assert!(signal_probe_indicates_process_exists(-1, libc::EPERM));
        assert!(!signal_probe_indicates_process_exists(-1, libc::ESRCH));
    }
    #[tokio::test]
    async fn shutdown_waits_for_owned_descendants_after_leader_exits() {
        let dir = tempfile::tempdir().unwrap();
        let ready = dir.path().join("descendant-ready");
        let completed = dir.path().join("descendant-term-handled");
        let child = r#"
import pathlib, signal, sys, time
def finish(signum, frame):
    time.sleep(0.35)
    pathlib.Path(sys.argv[2]).write_text("graceful")
    raise SystemExit(0)
signal.signal(signal.SIGTERM, finish)
pathlib.Path(sys.argv[1]).write_text("ready")
while True:
    time.sleep(0.02)
"#;
        let mut command = std::process::Command::new("/bin/sh");
        command.args([
            "-c",
            "trap 'exit 0' TERM; python3 -c \"$1\" \"$2\" \"$3\" & wait",
            "fixture",
            child,
            ready.to_str().unwrap(),
            completed.to_str().unwrap(),
        ]);
        let mut owner = OwnedProcess::spawn(command).unwrap();
        for _ in 0..200 {
            if ready.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(ready.exists(), "descendant fixture did not start");

        let shutdown_started = Instant::now();
        owner.shutdown().await;

        assert!(
            completed.exists(),
            "shutdown force-killed the descendant before its TERM handler completed"
        );
        assert!(
            shutdown_started.elapsed() < Duration::from_secs(1),
            "shutdown waited on the exited leader after its descendant finished"
        );
    }

    #[tokio::test]
    async fn owned_descendants_are_cleaned_even_after_leader_exit_and_cancellation() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("pid");
        let mut command = std::process::Command::new("/bin/sh");
        command
            .args(["-c", "sleep 30 & echo $! > \"$1\"; exit 0", "fixture"])
            .arg(&file);
        let owner = OwnedProcess::spawn(command).unwrap();
        for _ in 0..100 {
            if file.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let pid: libc::pid_t = std::fs::read_to_string(file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(process_exists(pid));
        drop(owner);
        for _ in 0..100 {
            if !process_exists(pid) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("owned descendant survived owner cancellation");
    }
    fn process_exists(pid: libc::pid_t) -> bool {
        // SAFETY: signal zero only checks this fixture PID; it never changes process state.
        unsafe { libc::kill(pid, 0) == 0 }
    }
    #[tokio::test]
    async fn probe_requires_canonical_successful_bounded_version_output() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("codex");
        for (body, expected) in [
            ("printf 'codex-cli 0.160.0\\n'", Some("0.160.0")),
            ("printf 'warning codex-cli 0.160.0\\n'", None),
            ("printf 'codex-cli 0.160.0-alpha\\n'", None),
            ("printf 'codex-cli 0.160.0\\n';exit 1", None),
            ("head -c 65537 /dev/zero", None),
        ] {
            std::fs::write(&executable, format!("#!/bin/sh\n{body}\n")).unwrap();
            crate::test_support::make_test_executable(&executable);
            let identity = ExecutableIdentity::read(&executable).expect("identity");
            let result = version(
                &executable,
                dir.path().to_str().unwrap(),
                &identity,
                &VersionProbeCache::new(),
            )
            .await;
            assert_eq!(result.as_deref(), expected);
        }
    }
}

// Installed diagnostics remain unavailable until #763 proves listener ownership.
// This constructor cannot mint controlled-fixture provenance or start probes/children.
#[cfg(all(test, unix))]
pub(crate) async fn installed_diagnostic_plan(
    _command: &CommandSpec,
    _cache: &VersionProbeCache,
) -> Result<NativeLaunchPlan, NativeError> {
    Err(NativeError::Unavailable)
}
