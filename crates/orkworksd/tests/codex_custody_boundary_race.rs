#![cfg(target_os = "macos")]
//! Native macOS Step 3 boundary race qualification suite for #1981.
//!
//! Proves descriptor non-inheritance at the actual implemented macOS custody
//! source boundary across all three audited production launch families
//! (`std::process::Command`, `probe_process::run`, and `portable_pty`), in both
//! receive-first and launch-first/concurrent orders, matching actual macOS
//! kernel object identities (`vnode (dev, ino)`, `socket soi_so/soi_pcb`, and
//! `pipe pipe_handle`) across the full `PROC_PIDLISTFDS` table rather than an
//! assumed FD number. Also proves positive controls detect real inheritance,
//! sealed channels are not inherited during active startup, parent never
//! installs resource rights, workers never fork after receipt, and independent
//! cleanup, expired lease refusal, sidecar replacement, and lost replies fail
//! closed without descriptor leakage.

#[path = "../src/probe_process.rs"]
#[allow(dead_code)]
mod probe_process;

#[path = "../src/taskmaster/codex_capture_owner/mod.rs"]
mod codex_capture_owner;

use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use codex_capture_owner::framing::{self, BindStatus, ControlFrame, ProbeCommitment, RootIdentity};
use codex_capture_owner::lease::{
    LeaseError, LeasePool, RevalidateOutcome, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE,
};
use codex_capture_owner::pool::{
    spawn_worker_process_for_test, CustodyPool, OwnedChild, MAX_SLOTS, STARTUP_BUDGET,
};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use probe_process::{ProbePolicy, ProbeService};

const ENV_VNODES: &str = "ORKWORKS_RACE_INSPECT_VNODES";
const ENV_SOCKETS: &str = "ORKWORKS_RACE_INSPECT_SOCKETS";
const ENV_PIPES: &str = "ORKWORKS_RACE_INSPECT_PIPES";
const ENV_WAIT_STDIN: &str = "ORKWORKS_RACE_WAIT_STDIN";

const PROC_PIDFDVNODEINFO: libc::c_int = 1;
const PROC_PIDFDSOCKETINFO: libc::c_int = 3;
const PROC_PIDFDPIPEINFO: libc::c_int = 6;

#[repr(C)]
struct ProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: libc::off_t,
    fi_type: i32,
    fi_guardflags: u32,
}

#[repr(C)]
struct VnodeFdInfo {
    pfi: ProcFileInfo,
    pvi: libc::vnode_info,
}

#[repr(C)]
struct PipeFdInfo {
    pfi: ProcFileInfo,
    pipe_stat: libc::vinfo_stat,
    pipe_handle: u64,
    pipe_peerhandle: u64,
    pipe_status: i32,
    rfu_1: i32,
}

#[repr(C)]
struct SocketFdInfoBuf {
    pfi: ProcFileInfo,
    soi_stat: libc::vinfo_stat,
    soi_so: u64,
    soi_pcb: u64,
    _rest: [u8; 792 - 176],
}

const _: () = {
    assert!(std::mem::size_of::<VnodeFdInfo>() == 176);
    assert!(std::mem::size_of::<PipeFdInfo>() == 184);
    assert!(std::mem::size_of::<SocketFdInfoBuf>() == 792);
};

fn fd_table_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn orkworksd_bin() -> &'static Path {
    static WARMED: OnceLock<()> = OnceLock::new();
    let bin = Path::new(env!("CARGO_BIN_EXE_orkworksd"));
    WARMED.get_or_init(|| {
        let _ = Command::new(bin)
            .arg("--codex-custody-worker")
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    });
    bin
}

fn tokio_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build test tokio runtime")
}

struct TempRoots {
    base: PathBuf,
    workspace: PathBuf,
    home: PathBuf,
    packaged: PathBuf,
}

impl TempRoots {
    fn create(label: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "orkworks-custody-race-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        let workspace = base.join("workspace");
        let home = base.join("home");
        let packaged = base.join("packaged");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&packaged).unwrap();
        Self {
            base,
            workspace,
            home,
            packaged,
        }
    }

    fn write_workspace_probe(&self, contents: &[u8]) {
        fs::write(self.workspace.join("custody-probe.txt"), contents).unwrap();
    }

    fn vnode_targets(&self) -> Vec<(u64, u64)> {
        let mut out = vec![
            file_identity(&self.workspace),
            file_identity(&self.home),
            file_identity(&self.packaged),
        ];
        let probe = self.workspace.join("custody-probe.txt");
        if probe.exists() {
            out.push(file_identity(&probe));
        }
        out
    }

    fn identities(&self) -> [RootIdentity; 3] {
        let (d0, i0) = file_identity(&self.workspace);
        let (d1, i1) = file_identity(&self.home);
        let (d2, i2) = file_identity(&self.packaged);
        [
            RootIdentity { dev: d0, ino: i0 },
            RootIdentity { dev: d1, ino: i1 },
            RootIdentity { dev: d2, ino: i2 },
        ]
    }
}

impl Drop for TempRoots {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn file_identity(path: &Path) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    let meta = fs::symlink_metadata(path).unwrap();
    (meta.dev(), meta.ino())
}

fn process_open_fds(pid: i32) -> io::Result<Vec<RawFd>> {
    // SAFETY: proc_pidinfo with null buffer queries required byte length.
    // On macOS proc_pidinfo returns 0 (with errno set) on error, never -1.
    let bytes_needed =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    if bytes_needed <= 0 {
        return Err(io::Error::last_os_error());
    }
    let entry_size = std::mem::size_of::<libc::proc_fdinfo>();
    let capacity = (bytes_needed as usize / entry_size).saturating_add(16);
    let mut entries = vec![
        libc::proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0,
        };
        capacity
    ];
    let buf_bytes = (entries.len() * entry_size) as libc::c_int;
    // SAFETY: entries is valid writable storage of `buf_bytes`.
    let written = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            entries.as_mut_ptr().cast(),
            buf_bytes,
        )
    };
    if written <= 0 {
        return Err(io::Error::last_os_error());
    }
    let count = (written as usize) / entry_size;
    let mut fds: Vec<RawFd> = entries[..count].iter().map(|e| e.proc_fd).collect();
    fds.sort_unstable();
    Ok(fds)
}

fn fd_vnode_identity(pid: i32, fd: RawFd) -> Option<(u64, u64)> {
    let mut info = std::mem::MaybeUninit::<VnodeFdInfo>::zeroed();
    // SAFETY: proc_pidfdinfo populates VnodeFdInfo for vnode descriptors.
    let ret = unsafe {
        libc::proc_pidfdinfo(
            pid,
            fd,
            PROC_PIDFDVNODEINFO,
            info.as_mut_ptr().cast(),
            std::mem::size_of::<VnodeFdInfo>() as libc::c_int,
        )
    };
    if ret == std::mem::size_of::<VnodeFdInfo>() as libc::c_int {
        // SAFETY: proc_pidfdinfo returned the full VnodeFdInfo size.
        let info = unsafe { info.assume_init() };
        Some((info.pvi.vi_stat.vst_dev as u64, info.pvi.vi_stat.vst_ino))
    } else {
        None
    }
}

fn fd_socket_identity(pid: i32, fd: RawFd) -> Option<(u64, u64)> {
    let mut info = std::mem::MaybeUninit::<SocketFdInfoBuf>::zeroed();
    // SAFETY: proc_pidfdinfo populates SocketFdInfoBuf (792 bytes) for socket descriptors.
    let ret = unsafe {
        libc::proc_pidfdinfo(
            pid,
            fd,
            PROC_PIDFDSOCKETINFO,
            info.as_mut_ptr().cast(),
            std::mem::size_of::<SocketFdInfoBuf>() as libc::c_int,
        )
    };
    if ret == std::mem::size_of::<SocketFdInfoBuf>() as libc::c_int {
        // SAFETY: proc_pidfdinfo returned the full SocketFdInfoBuf size.
        let info = unsafe { info.assume_init() };
        if info.soi_so != 0 || info.soi_pcb != 0 {
            return Some((info.soi_so, info.soi_pcb));
        }
    }
    None
}

fn fd_pipe_identity(pid: i32, fd: RawFd) -> Option<u64> {
    let mut info = std::mem::MaybeUninit::<PipeFdInfo>::zeroed();
    // SAFETY: proc_pidfdinfo populates PipeFdInfo (184 bytes) for pipe descriptors.
    let ret = unsafe {
        libc::proc_pidfdinfo(
            pid,
            fd,
            PROC_PIDFDPIPEINFO,
            info.as_mut_ptr().cast(),
            std::mem::size_of::<PipeFdInfo>() as libc::c_int,
        )
    };
    if ret == std::mem::size_of::<PipeFdInfo>() as libc::c_int {
        // SAFETY: proc_pidfdinfo returned the full PipeFdInfo size.
        let info = unsafe { info.assume_init() };
        if info.pipe_handle != 0 {
            return Some(info.pipe_handle);
        }
    }
    None
}

fn process_has_vnode(pid: i32, target: (u64, u64)) -> bool {
    let fds = process_open_fds(pid).expect("query PROC_PIDLISTFDS for vnode scan");
    assert!(
        !fds.is_empty(),
        "expected non-empty descriptor table for pid {pid}"
    );
    fds.into_iter()
        .any(|fd| fd_vnode_identity(pid, fd) == Some(target))
}

fn snapshot_socket_and_pipe_identities(pid: i32) -> (HashSet<(u64, u64)>, HashSet<u64>) {
    let fds = process_open_fds(pid).expect("query PROC_PIDLISTFDS for channel snapshot");
    assert!(!fds.is_empty());
    let mut sockets = HashSet::new();
    let mut pipes = HashSet::new();
    for fd in fds {
        if let Some(pair) = fd_socket_identity(pid, fd) {
            sockets.insert(pair);
        }
        if let Some(handle) = fd_pipe_identity(pid, fd) {
            pipes.insert(handle);
        }
    }
    (sockets, pipes)
}

fn child_process_count(ppid: u32) -> io::Result<usize> {
    let mut pids = [0 as libc::pid_t; 16];
    // SAFETY: proc_listchildpids writes child pid_t entries into `pids`.
    let ret = unsafe {
        libc::proc_listchildpids(
            ppid as libc::pid_t,
            pids.as_mut_ptr().cast(),
            std::mem::size_of_val(&pids) as libc::c_int,
        )
    };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(ret as usize)
}

fn process_reaped_echild(pid: u32) -> bool {
    // SAFETY: WNOWAIT non-destructively checks if our child PID is still waitable.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let rc = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    rc != 0 && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD)
}

#[derive(Clone, Debug, Default)]
struct InspectTargets {
    vnodes: Vec<(u64, u64)>,
    sockets: Vec<u64>,
    pipes: Vec<u64>,
}

impl InspectTargets {
    fn encode_vnodes(&self) -> String {
        self.vnodes
            .iter()
            .map(|(d, i)| format!("{d}:{i}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn encode_u64s(items: &[u64]) -> String {
        items
            .iter()
            .map(|v| format!("{v:x}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn apply_to_std(&self, cmd: &mut Command) {
        cmd.env_clear()
            .env(ENV_VNODES, self.encode_vnodes())
            .env(ENV_SOCKETS, Self::encode_u64s(&self.sockets))
            .env(ENV_PIPES, Self::encode_u64s(&self.pipes));
    }

    fn apply_to_pty(&self, cmd: &mut CommandBuilder) {
        cmd.env_clear();
        cmd.env(ENV_VNODES, self.encode_vnodes());
        cmd.env(ENV_SOCKETS, Self::encode_u64s(&self.sockets));
        cmd.env(ENV_PIPES, Self::encode_u64s(&self.pipes));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InspectResult {
    vnode_matches: usize,
    socket_matches: usize,
    pipe_matches: usize,
    total_fds: usize,
}

impl InspectResult {
    fn assert_clean(&self, label: &str) {
        assert!(
            self.total_fds >= 3,
            "{label}: inspector must observe at least stdio descriptors ({self:?})"
        );
        assert_eq!(
            self.vnode_matches, 0,
            "{label}: inherited vnode descriptor detected ({self:?})"
        );
        assert_eq!(
            self.socket_matches, 0,
            "{label}: inherited custody socket descriptor detected ({self:?})"
        );
        assert_eq!(
            self.pipe_matches, 0,
            "{label}: inherited custody pipe descriptor detected ({self:?})"
        );
    }
}

fn parse_inspect_output(output: &str) -> InspectResult {
    for raw_line in output.lines() {
        let line = raw_line.trim().trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("INSPECT_RESULT ") {
            let mut vnode_matches = None;
            let mut socket_matches = None;
            let mut pipe_matches = None;
            let mut total_fds = None;
            for part in rest.split_whitespace() {
                if let Some((k, v)) = part.split_once('=') {
                    let parsed = v.parse::<usize>().ok();
                    match k {
                        "vnode_matches" => vnode_matches = parsed,
                        "socket_matches" => socket_matches = parsed,
                        "pipe_matches" => pipe_matches = parsed,
                        "total_fds" => total_fds = parsed,
                        _ => {}
                    }
                }
            }
            if let (Some(v), Some(s), Some(p), Some(t)) =
                (vnode_matches, socket_matches, pipe_matches, total_fds)
            {
                return InspectResult {
                    vnode_matches: v,
                    socket_matches: s,
                    pipe_matches: p,
                    total_fds: t,
                };
            }
        }
    }
    panic!("missing INSPECT_RESULT sentinel in child output:\n{output}");
}

fn parse_vnode_set(raw: &str) -> HashSet<(u64, u64)> {
    raw.split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|item| {
            let (d, i) = item.split_once(':')?;
            Some((d.parse::<u64>().ok()?, i.parse::<u64>().ok()?))
        })
        .collect()
}

fn parse_hex_set(raw: &str) -> HashSet<u64> {
    raw.split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|item| u64::from_str_radix(item, 16).ok())
        .collect()
}

fn inspect_self_now() -> InspectResult {
    let pid = std::process::id() as i32;
    let target_vnodes = parse_vnode_set(&std::env::var(ENV_VNODES).unwrap_or_default());
    let target_sockets = parse_hex_set(&std::env::var(ENV_SOCKETS).unwrap_or_default());
    let target_pipes = parse_hex_set(&std::env::var(ENV_PIPES).unwrap_or_default());

    let fds = process_open_fds(pid).expect("list own PROC_PIDLISTFDS");
    let mut vnode_matches = 0usize;
    let mut socket_matches = 0usize;
    let mut pipe_matches = 0usize;

    for &fd in &fds {
        if let Some(id) = fd_vnode_identity(pid, fd) {
            if target_vnodes.contains(&id) {
                vnode_matches += 1;
            }
        }
        if let Some((so, pcb)) = fd_socket_identity(pid, fd) {
            if target_sockets.contains(&so) || target_sockets.contains(&pcb) {
                socket_matches += 1;
            }
        }
        if let Some(handle) = fd_pipe_identity(pid, fd) {
            if target_pipes.contains(&handle) {
                pipe_matches += 1;
            }
        }
    }

    InspectResult {
        vnode_matches,
        socket_matches,
        pipe_matches,
        total_fds: fds.len(),
    }
}

/// Ignored helper test executed inside unrelated child processes launched via
/// `std::process::Command`, `probe_process::run`, and `portable_pty`.
#[test]
#[ignore = "child-process object-identity inspector launched by boundary race tests"]
fn inspect_inherited_objects_child() {
    if std::env::var(ENV_WAIT_STDIN).as_deref() == Ok("1") {
        println!("READY");
        let _ = io::stdout().flush();
        let mut line = String::new();
        io::stdin().read_line(&mut line).expect("read GO line");
        assert_eq!(line.trim(), "GO");
        // Spawn a grandchild via ordinary std::process::Command to verify neither
        // the launch-first parent nor its grandchild inherited target objects.
        let mut grand = Command::new(std::env::current_exe().unwrap());
        grand
            .args([
                "--exact",
                "inspect_inherited_objects_child",
                "--ignored",
                "--nocapture",
            ])
            .env_remove(ENV_WAIT_STDIN);
        let out = grand.output().expect("spawn grandchild inspector");
        assert!(out.status.success(), "grandchild inspector failed");
        let child_res = parse_inspect_output(&String::from_utf8_lossy(&out.stdout));
        let self_res = inspect_self_now();
        println!(
            "INSPECT_RESULT vnode_matches={} socket_matches={} pipe_matches={} total_fds={}",
            self_res.vnode_matches + child_res.vnode_matches,
            self_res.socket_matches + child_res.socket_matches,
            self_res.pipe_matches + child_res.pipe_matches,
            self_res.total_fds
        );
        let _ = io::stdout().flush();
        return;
    }

    let res = inspect_self_now();
    println!(
        "INSPECT_RESULT vnode_matches={} socket_matches={} pipe_matches={} total_fds={}",
        res.vnode_matches, res.socket_matches, res.pipe_matches, res.total_fds
    );
    let _ = io::stdout().flush();
}

fn launch_via_std_command(targets: &InspectTargets) -> InspectResult {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        "inspect_inherited_objects_child",
        "--ignored",
        "--nocapture",
    ]);
    targets.apply_to_std(&mut cmd);
    let output = cmd.output().expect("std::process::Command inspector");
    assert!(
        output.status.success(),
        "std inspector failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_inspect_output(&String::from_utf8_lossy(&output.stdout))
}

fn launch_via_probe_process(targets: &InspectTargets) -> InspectResult {
    let rt = tokio_rt();
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        "inspect_inherited_objects_child",
        "--ignored",
        "--nocapture",
    ]);
    targets.apply_to_std(&mut cmd);
    let policy = ProbePolicy {
        max_bytes: 64 * 1024,
        reject_overflow: true,
        require_success: true,
    };
    let captured = rt.block_on(async {
        let service = ProbeService::new().expect("create isolated ProbeService");
        let output = service
            .run(&mut cmd, policy, Duration::from_secs(5))
            .await
            .expect("ProbeService::run inspector");
        service.shutdown().await;
        output
    });
    parse_inspect_output(&String::from_utf8_lossy(&captured.stdout))
}

fn launch_via_portable_pty(targets: &InspectTargets) -> InspectResult {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");
    let mut cmd = CommandBuilder::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        "inspect_inherited_objects_child",
        "--ignored",
        "--nocapture",
    ]);
    targets.apply_to_pty(&mut cmd);

    let mut child = pair.slave.spawn_command(cmd).expect("pty spawn_command");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone pty reader");
    let reader_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    });

    let status = child.wait().expect("wait pty child");
    assert!(status.success(), "pty child failed: {status:?}");
    drop(pair.master);

    let stdout = reader_thread.join().expect("join pty reader");
    parse_inspect_output(&stdout)
}

fn spawn_waiting_std_parent(
    targets: &InspectTargets,
) -> (std::process::Child, BufReader<std::process::ChildStdout>) {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        "inspect_inherited_objects_child",
        "--ignored",
        "--nocapture",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    targets.apply_to_std(&mut cmd);
    cmd.env(ENV_WAIT_STDIN, "1");

    let mut child = cmd.spawn().expect("spawn waiting std parent");
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).expect("read READY line");
        assert!(n > 0, "waiting child exited before READY");
        if line.trim() == "READY" {
            break;
        }
    }
    (child, reader)
}

fn finish_waiting_std_parent(
    mut child: std::process::Child,
    mut reader: BufReader<std::process::ChildStdout>,
) -> InspectResult {
    {
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "GO").expect("write GO");
    }
    let mut rest = String::new();
    reader.read_to_string(&mut rest).expect("read rest stdout");
    let status = child.wait().expect("wait waiting child");
    assert!(status.success(), "waiting child failed: {status:?}");
    parse_inspect_output(&rest)
}

fn receive_inheritable_fd_in_parent(source_fd: RawFd) -> OwnedFd {
    let (tx, rx) = UnixStream::pair().expect("UnixStream::pair");
    let fds = [source_fd];
    let fds_bytes = std::mem::size_of_val(&fds) as libc::c_uint;
    // SAFETY: CMSG_SPACE computes aligned ancillary buffer size for 1 RawFd.
    let cmsg_space = unsafe { libc::CMSG_SPACE(fds_bytes) } as usize;
    let mut send_buf = vec![0u64; cmsg_space.div_ceil(std::mem::size_of::<u64>())];
    let mut byte = [b'X'; 1];
    let mut iov = libc::iovec {
        iov_base: byte.as_mut_ptr().cast(),
        iov_len: 1,
    };
    // SAFETY: msghdr initialized before sendmsg.
    let mut smsg: libc::msghdr = unsafe { std::mem::zeroed() };
    smsg.msg_iov = &mut iov;
    smsg.msg_iovlen = 1;
    smsg.msg_control = send_buf.as_mut_ptr().cast();
    smsg.msg_controllen = cmsg_space as libc::socklen_t;
    // SAFETY: send_buf has cmsg_space writable aligned bytes.
    unsafe {
        let cmsg = libc::CMSG_FIRSTHDR(&smsg);
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(fds_bytes);
        std::ptr::copy_nonoverlapping(
            fds.as_ptr().cast::<u8>(),
            libc::CMSG_DATA(cmsg),
            fds_bytes as usize,
        );
        assert!(libc::sendmsg(tx.as_raw_fd(), &smsg, 0) == 1);
    }

    let mut recv_buf = vec![0u64; cmsg_space.div_ceil(std::mem::size_of::<u64>())];
    let mut rbyte = [0u8; 1];
    let mut riov = libc::iovec {
        iov_base: rbyte.as_mut_ptr().cast(),
        iov_len: 1,
    };
    // SAFETY: msghdr initialized before recvmsg.
    let mut rmsg: libc::msghdr = unsafe { std::mem::zeroed() };
    rmsg.msg_iov = &mut riov;
    rmsg.msg_iovlen = 1;
    rmsg.msg_control = recv_buf.as_mut_ptr().cast();
    rmsg.msg_controllen = cmsg_space as libc::socklen_t;
    // SAFETY: recvmsg delivers an inheritable FD in parent (reproducing XNU unp_externalize).
    let received_raw = unsafe {
        assert!(libc::recvmsg(rx.as_raw_fd(), &mut rmsg, 0) == 1);
        assert_eq!(rmsg.msg_flags & libc::MSG_CTRUNC, 0);
        let cmsg = libc::CMSG_FIRSTHDR(&rmsg);
        assert!(!cmsg.is_null());
        assert_eq!((*cmsg).cmsg_level, libc::SOL_SOCKET);
        assert_eq!((*cmsg).cmsg_type, libc::SCM_RIGHTS);
        let mut raw_fd: RawFd = -1;
        std::ptr::copy_nonoverlapping(
            libc::CMSG_DATA(cmsg),
            (&mut raw_fd as *mut RawFd).cast::<u8>(),
            std::mem::size_of::<RawFd>(),
        );
        raw_fd
    };
    assert!(received_raw > 2);
    // Confirm macOS XNU installed it WITHOUT FD_CLOEXEC.
    // SAFETY: received_raw is an open descriptor.
    let flags = unsafe { libc::fcntl(received_raw, libc::F_GETFD) };
    assert_eq!(
        flags & libc::FD_CLOEXEC,
        0,
        "expected inheritable SCM_RIGHTS FD on macOS"
    );
    // SAFETY: received_raw is owned exclusively here.
    unsafe { OwnedFd::from_raw_fd(received_raw) }
}

fn clear_cloexec(fd: RawFd) {
    // SAFETY: `fd` is open; clearing FD_CLOEXEC solely for positive-control inheritance proof.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    assert!(flags >= 0);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) },
        0
    );
}

fn collect_pool_channel_targets(pool: &LeasePool) -> (Vec<u64>, Vec<u64>) {
    let pid = std::process::id() as i32;
    let mut sockets = Vec::new();
    let mut pipes = Vec::new();
    for (req_fd, res_fd) in pool.controller_fds() {
        if let Some((so, pcb)) = fd_socket_identity(pid, req_fd) {
            if so != 0 {
                sockets.push(so);
            }
            if pcb != 0 {
                sockets.push(pcb);
            }
        }
        if let Some(handle) = fd_pipe_identity(pid, res_fd) {
            pipes.push(handle);
        }
    }
    (sockets, pipes)
}

#[test]
fn positive_controls_detect_inherited_vnode_socket_and_pipe_by_object_identity() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("positive");
    temp.write_workspace_probe(b"positive-control-vnode");
    let probe_path = temp.workspace.join("custody-probe.txt");
    let probe_id = file_identity(&probe_path);

    // 1. Vnode positive control: receive the probe file via SCM_RIGHTS in parent without CLOEXEC.
    let source = fs::File::open(&probe_path).unwrap();
    let inherited_vnode = receive_inheritable_fd_in_parent(source.as_raw_fd());
    drop(source);

    // 2. Socket & pipe positive controls: deliberately unseal one AF_UNIX socket and one OS pipe.
    let (sock_a, _sock_b) = UnixStream::pair().unwrap();
    clear_cloexec(sock_a.as_raw_fd());
    let mut pipe_fds = [-1 as RawFd; 2];
    // SAFETY: pipe writes two descriptors into pipe_fds.
    assert_eq!(unsafe { libc::pipe(pipe_fds.as_mut_ptr()) }, 0);
    // SAFETY: pipe succeeded; wrap both immediately.
    let pipe_r = unsafe { OwnedFd::from_raw_fd(pipe_fds[0]) };
    let _pipe_w = unsafe { OwnedFd::from_raw_fd(pipe_fds[1]) };
    clear_cloexec(pipe_r.as_raw_fd());

    let self_pid = std::process::id() as i32;
    let (sock_so, sock_pcb) = fd_socket_identity(self_pid, sock_a.as_raw_fd()).unwrap();
    let pipe_handle = fd_pipe_identity(self_pid, pipe_r.as_raw_fd()).unwrap();

    let targets = InspectTargets {
        vnodes: vec![probe_id],
        sockets: vec![sock_so, sock_pcb],
        pipes: vec![pipe_handle],
    };

    // Both std::process::Command and probe_process::run MUST detect all three inherited objects!
    let std_res = launch_via_std_command(&targets);
    assert!(
        std_res.vnode_matches >= 1 && std_res.socket_matches >= 1 && std_res.pipe_matches >= 1,
        "std positive control must detect inherited vnode, socket, and pipe: {std_res:?}"
    );

    let probe_res = launch_via_probe_process(&targets);
    assert!(
        probe_res.vnode_matches >= 1 && probe_res.socket_matches >= 1 && probe_res.pipe_matches >= 1,
        "probe_process positive control must detect inherited vnode, socket, and pipe: {probe_res:?}"
    );

    // Confirm portable-pty also executes cleanly (its pre_exec close_random_fds closes non-stdio FDs,
    // which is why the spawn audit notes PTY alone cannot substitute for std/probe positive controls).
    let _pty_res = launch_via_portable_pty(&targets);

    drop((inherited_vnode, sock_a, pipe_r));
}

#[test]
fn sealed_channels_are_not_inherited_during_startup_across_std_and_probe() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let bin = orkworksd_bin().to_path_buf();
    let build = codex_capture_owner::build_identity();
    let self_pid = std::process::id() as i32;
    let (baseline_sockets, baseline_pipes) = snapshot_socket_and_pipe_identities(self_pid);
    let startup_checks = Arc::new(Mutex::new(Vec::<InspectResult>::new()));
    let startup_checks_hook = Arc::clone(&startup_checks);

    // Use a generous hook-test budget since the hook synchronously runs 6
    // unrelated child launches (std, probe_process, portable_pty) inside startup;
    // the strict 3s admission budget is qualified separately without injected spawns.
    let custody = CustodyPool::start_with_slot_hook(
        build,
        Duration::from_secs(15),
        Duration::from_millis(250),
        move |slot, request_rx_fd, response_tx_fd| {
            // Execute unrelated launches DURING active startup right after channel sealing
            // (before spawning slot 0, while all 16 pre-sealed endpoints are open in the
            // parent, and again at slot 1) and verify ZERO sealed channel kernel objects
            // appear in any child's PROC_PIDLISTFDS table.
            if slot == 0 || slot == 1 {
                let (cur_sockets, cur_pipes) = snapshot_socket_and_pipe_identities(self_pid);
                let new_sockets: Vec<(u64, u64)> =
                    cur_sockets.difference(&baseline_sockets).copied().collect();
                let new_pipes: Vec<u64> = cur_pipes.difference(&baseline_pipes).copied().collect();
                if slot == 0 {
                    // Before slot 0 spawns, all 16 sealed endpoints (8 AF_UNIX socket ends
                    // and 8 OS pipe ends across 4 slots) must be open in the parent.
                    assert!(
                        new_sockets.len() >= MAX_SLOTS * 2,
                        "expected at least 8 newly sealed socket endpoints at slot 0, got {}",
                        new_sockets.len()
                    );
                    assert!(
                        new_pipes.len() >= MAX_SLOTS * 2,
                        "expected at least 8 newly sealed pipe endpoints at slot 0, got {}",
                        new_pipes.len()
                    );
                }
                let mut socket_targets = Vec::new();
                for (so, pcb) in new_sockets {
                    if so != 0 {
                        socket_targets.push(so);
                    }
                    if pcb != 0 {
                        socket_targets.push(pcb);
                    }
                }
                let targets = InspectTargets {
                    vnodes: vec![],
                    sockets: socket_targets,
                    pipes: new_pipes,
                };
                let std_res = launch_via_std_command(&targets);
                let probe_res = launch_via_probe_process(&targets);
                let pty_res = launch_via_portable_pty(&targets);
                let mut log = startup_checks_hook.lock().unwrap();
                log.push(std_res);
                log.push(probe_res);
                log.push(pty_res);
            }
            spawn_worker_process_for_test(&bin, request_rx_fd, response_tx_fd)
        },
    );

    let checks = startup_checks.lock().unwrap().clone();
    assert_eq!(checks.len(), 6, "expected 6 startup-window launch checks");
    for (idx, res) in checks.iter().enumerate() {
        res.assert_clean(&format!("startup-window check #{idx}"));
    }

    let mut pool = LeasePool::from_custody_pool(custody);
    assert_eq!(pool.accounting().idle, MAX_SLOTS);
    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, MAX_SLOTS);
    assert_eq!(report.cleanup_unconfirmed, 0);
}

#[test]
fn both_receive_and_launch_orders_isolate_descriptors_across_std_probe_and_pty() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("both-orders");
    temp.write_workspace_probe(b"boundary-race-payload-v1");
    let probe_id = file_identity(&temp.workspace.join("custody-probe.txt"));

    let mut pool = LeasePool::start_for_executable(
        orkworksd_bin(),
        codex_capture_owner::build_identity(),
        STARTUP_BUDGET,
    );
    assert_eq!(pool.accounting().idle, MAX_SLOTS);

    let (sockets, pipes) = collect_pool_channel_targets(&pool);
    assert!(
        sockets.len() >= MAX_SLOTS && pipes.len() == MAX_SLOTS,
        "must capture actual kernel identities for all 4 slots (sockets={}, pipes={})",
        sockets.len(),
        pipes.len()
    );
    let targets = InspectTargets {
        vnodes: temp.vnode_targets(),
        sockets,
        pipes,
    };

    // --- Order A: Bind/Receive-First, then launch across std, probe_process, and portable_pty ---
    let handle = pool
        .bind_and_observe(
            &temp.workspace,
            &temp.home,
            &temp.packaged,
            DEFAULT_LEASE_TTL,
            DEFAULT_OP_DEADLINE,
        )
        .expect("Order A bind_and_observe");
    let worker_pid = handle.worker_pid();

    // Parent must NEVER hold the synthetic resource file descriptor; worker must NEVER fork.
    assert!(
        !process_has_vnode(std::process::id() as i32, probe_id),
        "parent must never install synthetic resource descriptor"
    );
    assert_eq!(
        child_process_count(worker_pid).unwrap(),
        0,
        "custody worker must never fork after receipt"
    );

    launch_via_std_command(&targets).assert_clean("Order A std::process::Command");
    launch_via_probe_process(&targets).assert_clean("Order A probe_process::run");
    launch_via_portable_pty(&targets).assert_clean("Order A portable_pty");

    assert_eq!(
        pool.revalidate(&handle, DEFAULT_OP_DEADLINE).unwrap(),
        RevalidateOutcome::Fresh
    );
    assert_eq!(child_process_count(worker_pid).unwrap(), 0);
    pool.release(handle, DEFAULT_OP_DEADLINE).unwrap();

    // --- Order B1: Two-stage Launch-First witness (spawn parent before bind, grandchild after) ---
    let (waiting_child, waiting_reader) = spawn_waiting_std_parent(&targets);
    let handle_b1 = pool
        .bind_and_observe(
            &temp.workspace,
            &temp.home,
            &temp.packaged,
            DEFAULT_LEASE_TTL,
            DEFAULT_OP_DEADLINE,
        )
        .expect("Order B1 bind_and_observe while waiting child is live");
    assert_eq!(child_process_count(handle_b1.worker_pid()).unwrap(), 0);
    finish_waiting_std_parent(waiting_child, waiting_reader)
        .assert_clean("Order B1 launch-first parent + grandchild");
    pool.release(handle_b1, DEFAULT_OP_DEADLINE).unwrap();

    // --- Order B2: Barrier-synchronized concurrent overlap across std, probe, and PTY ---
    // Pre-initialize runtime/ProbeService and PTY pair BEFORE the barrier so the barrier
    // aligns directly with the actual child spawn, and loop bind/revalidate/release continuously
    // on the custody thread until the launcher completes so both sides genuinely overlap.
    for round in 0..6 {
        struct SetOnDrop(Arc<AtomicBool>);
        impl Drop for SetOnDrop {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        let barrier = Arc::new(Barrier::new(2));
        let launcher_done = Arc::new(AtomicBool::new(false));
        let b_launch = Arc::clone(&barrier);
        let done_launch = Arc::clone(&launcher_done);
        let t_launch = targets.clone();

        let launcher = thread::spawn(move || {
            let _mark_done = SetOnDrop(done_launch);
            match round % 3 {
                0 => {
                    let mut cmd = Command::new(std::env::current_exe().unwrap());
                    cmd.args([
                        "--exact",
                        "inspect_inherited_objects_child",
                        "--ignored",
                        "--nocapture",
                    ]);
                    t_launch.apply_to_std(&mut cmd);
                    b_launch.wait();
                    let output = cmd.output().expect("concurrent std inspector");
                    assert!(output.status.success());
                    parse_inspect_output(&String::from_utf8_lossy(&output.stdout))
                }
                1 => {
                    let rt = tokio_rt();
                    let service = ProbeService::new().expect("create isolated ProbeService");
                    let mut cmd = Command::new(std::env::current_exe().unwrap());
                    cmd.args([
                        "--exact",
                        "inspect_inherited_objects_child",
                        "--ignored",
                        "--nocapture",
                    ]);
                    t_launch.apply_to_std(&mut cmd);
                    let policy = ProbePolicy {
                        max_bytes: 64 * 1024,
                        reject_overflow: true,
                        require_success: true,
                    };
                    b_launch.wait();
                    let captured = rt.block_on(async {
                        let out = service
                            .run(&mut cmd, policy, Duration::from_secs(5))
                            .await
                            .expect("concurrent ProbeService::run inspector");
                        service.shutdown().await;
                        out
                    });
                    parse_inspect_output(&String::from_utf8_lossy(&captured.stdout))
                }
                _ => {
                    let pty_system = native_pty_system();
                    let pair = pty_system
                        .openpty(PtySize {
                            rows: 24,
                            cols: 120,
                            pixel_width: 0,
                            pixel_height: 0,
                        })
                        .expect("openpty");
                    let mut cmd = CommandBuilder::new(std::env::current_exe().unwrap());
                    cmd.args([
                        "--exact",
                        "inspect_inherited_objects_child",
                        "--ignored",
                        "--nocapture",
                    ]);
                    t_launch.apply_to_pty(&mut cmd);
                    let mut reader = pair.master.try_clone_reader().expect("clone pty reader");
                    b_launch.wait();
                    let mut child = pair.slave.spawn_command(cmd).expect("concurrent pty spawn");
                    drop(pair.slave);
                    let reader_thread = thread::spawn(move || {
                        let mut buf = Vec::new();
                        let _ = reader.read_to_end(&mut buf);
                        String::from_utf8_lossy(&buf).into_owned()
                    });
                    let status = child.wait().expect("wait concurrent pty child");
                    assert!(status.success());
                    drop(pair.master);
                    let stdout = reader_thread.join().expect("join pty reader");
                    parse_inspect_output(&stdout)
                }
            }
        });

        barrier.wait();
        let mut custody_cycles = 0usize;
        while custody_cycles == 0 || !launcher_done.load(Ordering::Acquire) {
            let h = pool
                .bind_and_observe(
                    &temp.workspace,
                    &temp.home,
                    &temp.packaged,
                    DEFAULT_LEASE_TTL,
                    DEFAULT_OP_DEADLINE,
                )
                .expect("concurrent bind_and_observe");
            assert!(!process_has_vnode(std::process::id() as i32, probe_id));
            assert_eq!(child_process_count(h.worker_pid()).unwrap(), 0);
            assert_eq!(
                pool.revalidate(&h, DEFAULT_OP_DEADLINE).unwrap(),
                RevalidateOutcome::Fresh
            );
            pool.release(h, DEFAULT_OP_DEADLINE).unwrap();
            custody_cycles += 1;
        }

        let res = launcher.join().expect("join concurrent launcher");
        res.assert_clean(&format!(
            "Order B2 concurrent overlap round {round} ({custody_cycles} overlapping cycles)"
        ));
    }

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, MAX_SLOTS);
    assert_eq!(report.cleanup_unconfirmed, 0);
}

#[test]
fn boundary_lifecycle_independent_cleanup_expired_refusal_sidecar_replacement_and_lost_reply() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("lifecycle-boundary");
    temp.write_workspace_probe(b"lifecycle-boundary-payload");
    let expected_ids = temp.identities();
    let probe_id = file_identity(&temp.workspace.join("custody-probe.txt"));
    let bin = orkworksd_bin().to_path_buf();
    let build = codex_capture_owner::build_identity();

    // 1. Lost reply + independent cleanup: slot 0 uses a synthetic peer that accepts
    //    BindAndObserve then drops its response pipe (lost reply) on Revalidate; slots 1..3
    //    run real orkworksd workers that stay healthy and clean up independently.
    let (peer_tx, peer_rx) = std::sync::mpsc::channel();
    let bin_hook = bin.clone();
    let custody = CustodyPool::start_with_slot_hook(
        build,
        STARTUP_BUDGET,
        Duration::from_millis(250),
        move |slot, req_rx, res_tx| {
            if slot != 0 {
                return spawn_worker_process_for_test(&bin_hook, req_rx, res_tx);
            }
            // SAFETY: req_rx and res_tx are open during hook invocation.
            let dup_req = unsafe { libc::fcntl(req_rx, libc::F_DUPFD_CLOEXEC, 3) };
            let dup_res = unsafe { libc::fcntl(res_tx, libc::F_DUPFD_CLOEXEC, 3) };
            assert!(dup_req >= 3 && dup_res >= 3);
            // SAFETY: dup_req and dup_res are newly owned FDs.
            let req_stream = UnixStream::from(unsafe { OwnedFd::from_raw_fd(dup_req) });
            let res_fd = unsafe { OwnedFd::from_raw_fd(dup_res) };

            let child = Command::new("/bin/sleep")
                .arg("30")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            let pid = child.id() as libc::pid_t;
            std::mem::forget(child);
            let owned = OwnedChild::try_from_pid(pid)?;

            let join = thread::spawn(move || {
                let hs_dl = Instant::now() + Duration::from_secs(10);
                let ch = framing::read_bootstrap_frame(req_stream.as_fd(), hs_dl).unwrap();
                let mut ready = ch;
                ready[5] = 2;
                framing::write_bootstrap_frame(res_fd.as_fd(), &ready, hs_dl).unwrap();

                let bind_dl = Instant::now() + Duration::from_secs(10);
                let (frame, fds) =
                    framing::recv_request_frame_or_eof(req_stream.as_fd(), Some(bind_dl))
                        .unwrap()
                        .unwrap();
                let ControlFrame::BindAndObserve {
                    lease_id,
                    lease_seq,
                    op_seq,
                    ..
                } = frame
                else {
                    panic!("expected BindAndObserve");
                };
                framing::write_control_frame(
                    res_fd.as_fd(),
                    &ControlFrame::BindObserved {
                        lease_id,
                        lease_seq,
                        op_seq,
                        status: BindStatus::Observed,
                        root_identities: expected_ids,
                        probe_commitments: [ProbeCommitment::absent(); 3],
                    },
                    bind_dl,
                )
                .unwrap();

                // Read Revalidate then drop response pipe without replying (lost reply).
                let rev_dl = Instant::now() + Duration::from_secs(10);
                let (rev_frame, _) =
                    framing::recv_request_frame_or_eof(req_stream.as_fd(), Some(rev_dl))
                        .unwrap()
                        .unwrap();
                assert!(
                    matches!(rev_frame, ControlFrame::Revalidate { .. }),
                    "expected Revalidate before simulating lost reply, got {rev_frame:?}"
                );
                drop((res_fd, req_stream, fds));
            });
            peer_tx.send(join).unwrap();
            Ok(owned)
        },
    );

    let mut sidecar_a = LeasePool::from_custody_pool(custody);
    assert_eq!(sidecar_a.accounting().idle, MAX_SLOTS);
    let (sockets_a, pipes_a) = collect_pool_channel_targets(&sidecar_a);
    let targets_a = InspectTargets {
        vnodes: temp.vnode_targets(),
        sockets: sockets_a,
        pipes: pipes_a,
    };

    let h0 = sidecar_a
        .bind_and_observe(
            &temp.workspace,
            &temp.home,
            &temp.packaged,
            DEFAULT_LEASE_TTL,
            DEFAULT_OP_DEADLINE,
        )
        .unwrap();
    let h1 = sidecar_a
        .bind_and_observe(
            &temp.workspace,
            &temp.home,
            &temp.packaged,
            Duration::from_millis(200),
            DEFAULT_OP_DEADLINE,
        )
        .unwrap();
    let h2 = sidecar_a
        .bind_and_observe(
            &temp.workspace,
            &temp.home,
            &temp.packaged,
            DEFAULT_LEASE_TTL,
            DEFAULT_OP_DEADLINE,
        )
        .unwrap();
    let lost_pid = h0.worker_pid();
    let sibling_pids = [
        h1.worker_pid(),
        h2.worker_pid(),
        sidecar_a.slot_pid(3).unwrap(),
    ];

    // Lost reply on slot 0 retires slot 0 permanently and reaps its child without affecting siblings.
    let lost_err = sidecar_a.revalidate(&h0, DEFAULT_OP_DEADLINE);
    assert_eq!(lost_err, Err(LeaseError::WorkerExited));
    // Join synthetic peer thread before launching child inspectors so its in-process
    // received FDs are deterministically closed.
    peer_rx.recv().unwrap().join().unwrap();
    assert!(!sidecar_a.is_handle_usable(&h0));
    assert!(
        process_reaped_echild(lost_pid),
        "lost-reply worker must be reaped"
    );
    assert_eq!(sidecar_a.accounting().retired, 1);
    assert_eq!(
        sidecar_a.revalidate(&h2, DEFAULT_OP_DEADLINE).unwrap(),
        RevalidateOutcome::Fresh
    );
    launch_via_std_command(&targets_a).assert_clean("after lost reply std");
    launch_via_probe_process(&targets_a).assert_clean("after lost reply probe");

    // 2. Expired lease refusal on slot 1: wait past h1's expiry, refuse Revalidate, drain cleanly.
    let wait_until = h1.expires_at() + Duration::from_millis(30);
    if let Some(remaining) = wait_until.checked_duration_since(Instant::now()) {
        thread::sleep(remaining);
    }
    assert_eq!(
        sidecar_a.revalidate(&h1, DEFAULT_OP_DEADLINE),
        Err(LeaseError::Expired)
    );
    assert!(!sidecar_a.is_handle_usable(&h1));
    launch_via_std_command(&targets_a).assert_clean("after expired lease refusal std");

    // 3. Sidecar replacement: shut down sidecar_a, confirm all old workers are reaped,
    //    start fresh sidecar_b, verify sidecar_b rejects h2 and adopts zero old workers.
    let report_a = sidecar_a.shutdown(Duration::from_secs(2));
    assert_eq!(report_a.clean_exits, 3);
    assert_eq!(report_a.cleanup_unconfirmed, 0);
    for old_pid in sibling_pids {
        assert!(
            process_reaped_echild(old_pid),
            "sidecar_a worker {old_pid} must be reaped on replacement"
        );
    }

    let mut sidecar_b = LeasePool::start_for_executable(&bin, build, STARTUP_BUDGET);
    assert_eq!(sidecar_b.accounting().idle, MAX_SLOTS);
    assert!(!sidecar_b.is_handle_usable(&h2));
    assert_eq!(
        sidecar_b.revalidate(&h2, DEFAULT_OP_DEADLINE),
        Err(LeaseError::InvalidHandle)
    );

    let (sockets_b, pipes_b) = collect_pool_channel_targets(&sidecar_b);
    let targets_b = InspectTargets {
        vnodes: temp.vnode_targets(),
        sockets: sockets_b,
        pipes: pipes_b,
    };
    let h_new = sidecar_b
        .bind_and_observe(
            &temp.workspace,
            &temp.home,
            &temp.packaged,
            DEFAULT_LEASE_TTL,
            DEFAULT_OP_DEADLINE,
        )
        .unwrap();
    assert!(!process_has_vnode(std::process::id() as i32, probe_id));
    assert_eq!(child_process_count(h_new.worker_pid()).unwrap(), 0);
    launch_via_std_command(&targets_b).assert_clean("sidecar_b std");
    launch_via_probe_process(&targets_b).assert_clean("sidecar_b probe");
    launch_via_portable_pty(&targets_b).assert_clean("sidecar_b pty");
    sidecar_b.release(h_new, DEFAULT_OP_DEADLINE).unwrap();

    let report_b = sidecar_b.shutdown(Duration::from_secs(2));
    assert_eq!(report_b.clean_exits, MAX_SLOTS);
    assert_eq!(report_b.cleanup_unconfirmed, 0);
}
