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
            while self.is_alive() && Instant::now() < deadline {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            self.signal(libc::SIGKILL);
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
#[cfg(unix)]
async fn probe(executable: &Path, cwd: &str, args: &[&str]) -> Option<(String, String)> {
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
    tokio::time::timeout(Duration::from_secs(3), read)
        .await
        .ok()
        .flatten()
}
#[cfg(not(unix))]
async fn probe(_: &Path, _: &str, _: &[&str]) -> Option<(String, String)> {
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
pub(super) async fn version(
    executable: &Path,
    cwd: &str,
    identity: &ExecutableIdentity,
    cache: &VersionProbeCache,
) -> Option<String> {
    cache
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
                let (out, err) = probe(executable, cwd, &["--version"]).await?;
                canonical_version(&out, &err)
            },
        )
        .await
}
pub(super) async fn features(executable: &Path, cwd: &str) -> bool {
    let Some((help, err)) = probe(executable, cwd, &["--help"]).await else {
        return false;
    };
    if !err.is_empty() || !help.starts_with("Codex CLI") {
        return false;
    }
    let Some((server, err)) = probe(executable, cwd, &["app-server", "--help"]).await else {
        return false;
    };
    if !err.is_empty() || !server.contains("Usage: codex app-server") {
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
    if !cfg!(unix) {
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
        let server = OwnedProcess::spawn(command)?;
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
                plan.revalidate()?;
                return Ok(OwnedNativeRuntime {
                    plan,
                    server,
                    endpoint,
                    token,
                    tui_args,
                    observer: Some(observer),
                    retry_at: Instant::now(),
                    backoff: Duration::from_millis(250),
                });
            }
            Ok(Err(_)) => drop(server),
            Err(_) => return Err(NativeError::Timeout),
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(NativeError::Timeout);
        }
    }
    Err(NativeError::Unavailable)
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    fn fixture_plan(dir: &Path, cache: &VersionProbeCache, resume: bool) -> NativeLaunchPlan {
        let executable = dir.join("codex");
        std::fs::write(&executable,r#"#!/bin/sh
printf '%s\n' "$@" > "$FIXTURE_DIR/args"
pwd > "$FIXTURE_DIR/cwd"
printf '%s\n' "$ORKWORKS_REPORT_TOKEN" "$ORKWORKS_CODEX_SESSION_REPORT_DIR" "$CODEX_HOME" "${ORKWORKS_CODEX_NATIVE_AUTH-absent}" > "$FIXTURE_DIR/env"
echo $$ > "$FIXTURE_DIR/pid"
if [ "$FIXTURE_MODE" = fail ]; then exit 1; fi
if [ "$FIXTURE_MODE" = hang ]; then exec sleep 30; fi
while [ "$#" -gt 0 ]; do
 case "$1" in --listen) export FIXTURE_LISTEN="$2"; shift;; --ws-token-sha256) export FIXTURE_DIGEST="$2"; shift;; esac
 shift
done
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
                "resume",
                "saved-id",
                "--disable",
                "other",
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
        }
    }
    fn fixture_env(dir: &Path, mode: &str) -> Vec<(String, String)> {
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
    async fn native_server_fixture() {
        let Ok(endpoint) = std::env::var("FIXTURE_LISTEN") else {
            return;
        };
        let listener = tokio::net::TcpListener::bind(endpoint.trim_start_matches("ws://"))
            .await
            .unwrap();
        loop {
            let (tcp, _) = listener.accept().await.unwrap();
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
                    let result = match value["method"].as_str().unwrap() {
                        "initialize" => {
                            json!({"userAgent":"fixture/0.160.0","codexHome":"/fixture","platformOs":std::env::consts::OS,"platformFamily":"unix"})
                        }
                        "thread/loaded/list" => json!({"data":["root"],"nextCursor":null}),
                        "thread/read" => {
                            json!({"thread":{"id":"root","sessionId":"root","source":"mcp","cliVersion":"0.160.0","status":{"type":"active","activeFlags":[]}}})
                        }
                        _ => panic!("forbidden fixture method"),
                    };
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
