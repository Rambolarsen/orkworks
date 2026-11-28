#![cfg(target_os = "macos")]

use std::io::{self, ErrorKind};
use std::os::fd::{AsFd, AsRawFd, RawFd};
use std::os::unix::{fs::MetadataExt, net::UnixStream, process::CommandExt};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// Serializes tests in this integration binary that inspect process-wide raw FD
/// closure or spawn subprocesses so POSIX FD-number reuse across parallel
/// libtest threads cannot cause spurious collisions.
fn fd_table_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[path = "../src/taskmaster/codex_capture_owner/mod.rs"]
mod codex_capture_owner;

use codex_capture_owner::{
    build_identity,
    channels::{self, SlotChannels},
    framing::{self, ControlFrame},
    pool::{CustodyPool, OwnedChild, MAX_SLOTS, STARTUP_BUDGET},
    worker::WORKER_ARG,
};

fn sidecar_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_orkworksd"))
}

#[test]
fn build_identity_is_32_non_zero_source_bound_bytes() {
    let id = build_identity();
    assert_eq!(id.len(), 32);
    assert_ne!(id, [0u8; 32]);
    assert_eq!(id, build_identity());
}

#[test]
fn split_channels_seal_all_16_endpoints_and_roll_back_partial_allocation() {
    let _guard = fd_table_lock();
    let mut opened_sockets = Vec::new();
    let mut opened_pipes = Vec::new();
    let failed = channels::seal_split_channels_using(
        || {
            if opened_sockets.len() == 4 {
                return Err(io::Error::other("injected third-slot socket failure"));
            }
            let (tx, rx) = UnixStream::pair()?;
            opened_sockets.extend([tx.as_raw_fd(), rx.as_raw_fd()]);
            Ok((tx, rx))
        },
        || {
            let (rx, tx) = channels::create_sealed_pipe()?;
            opened_pipes.extend([rx.as_raw_fd(), tx.as_raw_fd()]);
            Ok((rx, tx))
        },
    );
    assert_eq!(
        failed
            .err()
            .expect("partial allocation must fail")
            .to_string(),
        "injected third-slot socket failure"
    );
    assert_eq!(opened_sockets.len(), 4);
    assert_eq!(opened_pipes.len(), 4);
    for fd in opened_sockets.into_iter().chain(opened_pipes) {
        assert_closed(fd);
    }

    let slots = channels::seal_split_channels().expect("seal all 4 split slots");
    let fds: Vec<RawFd> = slots
        .iter()
        .flat_map(|slot| {
            [
                slot.request_tx.as_raw_fd(),
                slot.request_rx.as_raw_fd(),
                slot.response_rx.as_raw_fd(),
                slot.response_tx.as_raw_fd(),
            ]
        })
        .collect();
    assert_eq!(fds.len(), 16);
    for fd in fds {
        // SAFETY: `slots` owns every endpoint for this loop.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert!(flags >= 0 && (flags & libc::FD_CLOEXEC) != 0);
    }
}

#[test]
fn split_channels_block_worker_scm_rights_at_kernel_boundary() {
    let _guard = fd_table_lock();
    let temp = tempfile::tempfile().expect("temporary target file");
    let target_meta = temp.metadata().expect("temp metadata");
    let target_id = (target_meta.dev(), target_meta.ino());

    let slots = channels::seal_split_channels().expect("seal split channels");
    let SlotChannels {
        request_tx,
        request_rx,
        response_rx,
        response_tx,
    } = &slots[0];

    // 1. Worker cannot send SCM_RIGHTS back over the AF_UNIX request channel
    //    because both reverse directions are shut down (SHUT_RD / SHUT_WR).
    let req_err = send_with_scm_rights(request_rx.as_raw_fd(), b"x", temp.as_raw_fd())
        .expect_err("reverse sendmsg on unidirectional request socket must fail");
    assert!(
        matches!(
            req_err.raw_os_error(),
            Some(libc::EPIPE) | Some(libc::EINVAL) | Some(libc::ESHUTDOWN)
        ),
        "unexpected reverse sendmsg error: {req_err:?}"
    );

    // 2. Worker cannot send SCM_RIGHTS over the OS pipe response channel
    //    because a pipe is not a socket (ENOTSOCK).
    let resp_err = send_with_scm_rights(response_tx.as_raw_fd(), b"x", temp.as_raw_fd())
        .expect_err("sendmsg on OS pipe response channel must fail");
    assert_eq!(resp_err.raw_os_error(), Some(libc::ENOTSOCK));

    // 3. Parent response pipe still delivers normal pure-byte frames cleanly
    //    without installing any descriptor matching `temp`.
    let deadline = Instant::now() + Duration::from_secs(2);
    framing::write_control_frame(response_tx.as_fd(), &ControlFrame::ShutdownAck, deadline)
        .expect("write pure-byte response on pipe");
    assert_eq!(
        framing::read_control_frame(response_rx.as_fd(), deadline).expect("read response"),
        ControlFrame::ShutdownAck
    );
    let _ = request_tx;

    let leaked: Vec<RawFd> = list_own_open_fds()
        .into_iter()
        .filter(|&fd| fd != temp.as_raw_fd())
        .filter(|&fd| fd_matches_object(fd, target_id))
        .collect();
    assert!(
        leaked.is_empty(),
        "sent file object must never appear in parent descriptors: {leaked:?}"
    );
}

#[test]
fn persistent_control_frames_round_trip_and_reject_malformed_inputs() {
    let _guard = fd_table_lock();
    let (rx, tx) = channels::create_sealed_pipe().expect("pipe");
    let deadline = Instant::now() + Duration::from_secs(2);
    let nonce = [0x5Au8; 16];

    framing::write_control_frame(tx.as_fd(), &ControlFrame::Ping(nonce), deadline)
        .expect("write ping");
    assert_eq!(
        framing::read_control_frame(rx.as_fd(), deadline).expect("read ping"),
        ControlFrame::Ping(nonce)
    );
    framing::ensure_no_trailing_bytes(rx.as_fd()).expect("no trailing bytes after ping");

    // Same pipe stays open for subsequent frames without EOF.
    framing::write_control_frame(tx.as_fd(), &ControlFrame::Pong(nonce), deadline)
        .expect("write pong");
    assert_eq!(
        framing::read_control_frame(rx.as_fd(), deadline).expect("read pong"),
        ControlFrame::Pong(nonce)
    );
    framing::write_control_frame(tx.as_fd(), &ControlFrame::Shutdown, deadline)
        .expect("write shutdown");
    assert_eq!(
        framing::read_control_frame(rx.as_fd(), deadline).expect("read shutdown"),
        ControlFrame::Shutdown
    );
    framing::write_control_frame(tx.as_fd(), &ControlFrame::ShutdownAck, deadline)
        .expect("write ack");
    assert_eq!(
        framing::read_control_frame(rx.as_fd(), deadline).expect("read ack"),
        ControlFrame::ShutdownAck
    );

    // Reject unsolicited trailing bytes immediately following a complete frame.
    let mut with_trailing = framing::encode_control_frame(&ControlFrame::ShutdownAck);
    with_trailing.push(0xFF);
    let (trail_rx, trail_tx) = channels::create_sealed_pipe().expect("trailing pipe");
    write_raw_all(trail_tx.as_raw_fd(), &with_trailing);
    assert_eq!(
        framing::read_control_frame(trail_rx.as_fd(), deadline).expect("read frame"),
        ControlFrame::ShutdownAck
    );
    let trail_err = framing::ensure_no_trailing_bytes(trail_rx.as_fd())
        .expect_err("trailing byte must be rejected");
    assert_eq!(trail_err.kind(), ErrorKind::InvalidData);

    // Reject bad magic, bad version, bad kind, bad length, oversized length.
    let invalid_headers: &[&[u8]] = &[
        b"NOPE\x01\x03\x00\x00\x00\x00",
        b"OWCO\x02\x03\x00\x00\x00\x00",
        b"OWCO\x01\x7f\x00\x00\x00\x00",
        b"OWCO\x01\x03\x00\x00\x00\x01\x00",
        b"OWCO\x01\x01\x00\x00\x00\x0f123456789012345",
        b"OWCO\x01\x01\x00\x20\x00\x00",
    ];
    for raw in invalid_headers {
        let (bad_rx, bad_tx) = channels::create_sealed_pipe().expect("bad pipe");
        write_raw_all(bad_tx.as_raw_fd(), raw);
        drop(bad_tx);
        let err = framing::read_control_frame(bad_rx.as_fd(), deadline)
            .expect_err("malformed frame must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    // Truncated stream hits UnexpectedEof.
    let (trunc_rx, trunc_tx) = channels::create_sealed_pipe().expect("trunc pipe");
    write_raw_all(trunc_tx.as_raw_fd(), b"OWCO\x01\x01\x00\x00");
    drop(trunc_tx);
    let err = framing::read_control_frame(trunc_rx.as_fd(), deadline)
        .expect_err("truncated frame must fail");
    assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
}

#[test]
fn real_sidecar_persistent_worker_pool_starts_pings_measures_cost_and_shuts_down() {
    let _guard = fd_table_lock();
    let started = Instant::now();
    let mut pool =
        CustodyPool::start_for_executable(sidecar_bin(), build_identity(), STARTUP_BUDGET);
    let startup_elapsed = started.elapsed();

    assert!(
        startup_elapsed < STARTUP_BUDGET,
        "startup exceeded budget: {startup_elapsed:?}"
    );
    let counts = pool.counts();
    assert_eq!(counts.ready, MAX_SLOTS);
    assert_eq!(counts.retired, 0);
    assert_eq!(counts.cleanup_unconfirmed, 0);

    let pids = pool.ready_pids();
    assert_eq!(pids.len(), MAX_SLOTS);

    // Every persistent worker must handle multiple back-to-back pings on the same open channels.
    for round in 0..3 {
        let deadline = Instant::now() + Duration::from_secs(2);
        for slot in 0..MAX_SLOTS {
            pool.ping_slot(slot, deadline)
                .unwrap_or_else(|err| panic!("slot {slot} round {round} ping failed: {err}"));
        }
    }

    // Measure idle resident memory and CPU delta over a 300ms idle interval.
    let before_samples: Vec<_> = pids.iter().map(|&pid| sample_task_cost(pid)).collect();
    thread::sleep(Duration::from_millis(300));
    let after_samples: Vec<_> = pids.iter().map(|&pid| sample_task_cost(pid)).collect();

    let total_rss_bytes: u64 = after_samples.iter().map(|s| s.resident_bytes).sum();
    let total_cpu_delta_ns: u64 = before_samples
        .iter()
        .zip(&after_samples)
        .map(|(b, a)| a.total_cpu_ns.saturating_sub(b.total_cpu_ns))
        .sum();
    assert!(total_rss_bytes > 0, "idle workers must report positive RSS");
    assert!(
        total_cpu_delta_ns < 50_000_000,
        "four idle workers used excessive CPU over 300ms: {total_cpu_delta_ns} ns"
    );

    eprintln!(
        "CUSTODY_POOL_COST startup_ms={} workers={} total_rss_kib={} per_worker_rss_kib={:?} idle_300ms_cpu_us={}",
        startup_elapsed.as_millis(),
        pids.len(),
        total_rss_bytes / 1024,
        after_samples
            .iter()
            .map(|s| s.resident_bytes / 1024)
            .collect::<Vec<_>>(),
        total_cpu_delta_ns / 1000
    );

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, MAX_SLOTS);
    assert_eq!(report.forced_kills, 0);
    assert_eq!(report.cleanup_unconfirmed, 0);
    assert_eq!(pool.counts().ready, 0);
    assert_eq!(pool.counts().retired, MAX_SLOTS);
}

#[test]
fn real_sidecar_worker_fails_closed_on_invalid_invocation_env_extra_fd_or_wrong_build() {
    let _guard = fd_table_lock();
    // 1. Missing descriptor arguments exit non-zero and never start HTTP server.
    let output = Command::new(sidecar_bin())
        .arg(WORKER_ARG)
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .expect("run invalid worker arg");
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("ORKWORKSD_PORT="));

    // 2. Non-empty capability environment variable is rejected.
    let (_req_tx, req_rx) = UnixStream::pair().expect("req pair");
    let (_resp_rx, resp_tx) = channels::create_sealed_pipe().expect("resp pipe");
    let req_fd = req_rx.as_raw_fd();
    let resp_fd = resp_tx.as_raw_fd();
    let mut env_cmd = Command::new(sidecar_bin());
    env_cmd
        .args([WORKER_ARG, &req_fd.to_string(), &resp_fd.to_string()])
        .env_clear()
        .env("UNEXPECTED_CAPABILITY", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: descriptors remain open across spawn; clearing CLOEXEC is async-signal-safe.
    unsafe {
        env_cmd.pre_exec(move || {
            if libc::fcntl(req_fd, libc::F_SETFD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::fcntl(resp_fd, libc::F_SETFD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let env_status = env_cmd.status().expect("run worker with env");
    assert_eq!(
        env_status.code(),
        Some(65),
        "worker must reject unexpected env var with EXIT_NON_EMPTY_ENV"
    );

    // 3. Extra unexpected inherited file descriptor is rejected by full PROC_PIDLISTFDS inventory.
    let extra_file = tempfile::tempfile().expect("extra inherited file");
    let extra_fd = extra_file.as_raw_fd();
    let mut fd_cmd = Command::new(sidecar_bin());
    fd_cmd
        .args([WORKER_ARG, &req_fd.to_string(), &resp_fd.to_string()])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: all descriptors remain open across spawn; fcntl(F_SETFD) is async-signal-safe.
    unsafe {
        fd_cmd.pre_exec(move || {
            if libc::fcntl(req_fd, libc::F_SETFD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::fcntl(resp_fd, libc::F_SETFD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::fcntl(extra_fd, libc::F_SETFD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let fd_status = fd_cmd.status().expect("run worker with extra fd");
    assert_eq!(
        fd_status.code(),
        Some(66),
        "worker must reject unknown inherited descriptor with EXIT_BAD_FD_INVENTORY"
    );

    // 4. Passing a Unix socket as the response pipe is rejected by worker channel validation.
    let (_sock_a, sock_b) = UnixStream::pair().expect("wrong response type");
    let mut wrong_resp =
        codex_capture_owner::pool::spawn_worker_process(sidecar_bin(), req_fd, sock_b.as_raw_fd())
            .expect("spawn worker with socket response fd");
    let wrong_status = wrong_resp.wait().expect("reap wrong-response worker");
    assert_eq!(
        wrong_status.code(),
        Some(68),
        "worker must reject non-pipe response descriptor with EXIT_BAD_CHANNEL"
    );

    // 5. Passing a read-only pipe end as response_tx is rejected with EXIT_BAD_CHANNEL.
    let (ro_rx, _ro_tx) = channels::create_sealed_pipe().expect("read-only pipe");
    let mut ro_worker =
        codex_capture_owner::pool::spawn_worker_process(sidecar_bin(), req_fd, ro_rx.as_raw_fd())
            .expect("spawn worker with read-only pipe");
    let ro_status = ro_worker.wait().expect("reap read-only pipe worker");
    assert_eq!(
        ro_status.code(),
        Some(68),
        "worker must reject read-only response pipe with EXIT_BAD_CHANNEL"
    );

    // 6. Mismatched build identity retires all slots cleanly.
    let mut wrong_build = build_identity();
    wrong_build[0] ^= 0xFF;
    let pool = CustodyPool::start_for_executable(sidecar_bin(), wrong_build, STARTUP_BUDGET);
    assert_eq!(pool.counts().ready, 0);
    assert_eq!(pool.counts().retired, MAX_SLOTS);
    assert_eq!(pool.counts().cleanup_unconfirmed, 0);
}

#[test]
fn startup_deadline_expiry_before_seal_starts_no_workers() {
    let _guard = fd_table_lock();
    let pool = CustodyPool::start_for_executable(sidecar_bin(), build_identity(), Duration::ZERO);
    assert_eq!(pool.counts().ready, 0);
    assert_eq!(pool.counts().retired, MAX_SLOTS);
    assert_eq!(pool.counts().cleanup_unconfirmed, 0);
}

#[test]
fn pool_returns_at_deadline_with_spawn_attempt_still_pending() {
    let _guard = fd_table_lock();
    let (release, gate) = mpsc::channel();
    let (entered, entry) = mpsc::channel();
    let gate = Mutex::new(gate);
    let (returned, result) = mpsc::channel();
    let (late_pid, spawned_pid) = mpsc::channel();

    let startup = thread::spawn(move || {
        let started = Instant::now();
        let pool = CustodyPool::start_with_slot_hook(
            build_identity(),
            STARTUP_BUDGET,
            Duration::from_millis(250),
            move |slot, request_rx_fd, response_tx_fd| {
                assert!(
                    slot < 2,
                    "startup launched another slot while the hook was pending"
                );
                if slot == 1 {
                    entered.send(()).unwrap();
                    gate.lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(10))
                        .map_err(io::Error::other)?;
                }
                let child = codex_capture_owner::pool::spawn_worker_process(
                    sidecar_bin(),
                    request_rx_fd,
                    response_tx_fd,
                )?;
                if slot == 1 {
                    let _ = late_pid.send(child.id());
                }
                Ok(child)
            },
        );
        returned.send((pool, started.elapsed())).unwrap();
    });

    entry.recv_timeout(STARTUP_BUDGET).unwrap();
    let mut pool = match result.recv_timeout(STARTUP_BUDGET + Duration::from_secs(1)) {
        Ok((pool, elapsed)) => {
            assert!(elapsed < STARTUP_BUDGET + Duration::from_secs(1));
            pool
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            release.send(()).unwrap();
            let (mut pool, _) = result.recv_timeout(Duration::from_secs(3)).unwrap();
            pool.shutdown(Duration::from_secs(2));
            startup.join().unwrap();
            panic!("startup waited for a spawn operation after the shared deadline");
        }
        Err(error) => panic!("startup result lost: {error}"),
    };
    startup.join().unwrap();
    assert_eq!(pool.counts().ready, 1);
    assert_eq!(pool.counts().cleanup_unconfirmed, 1);
    assert_eq!(pool.counts().retired, 2);
    pool.ping_slot(0, Instant::now() + Duration::from_secs(1))
        .unwrap();
    assert!(pool
        .ping_slot(1, Instant::now() + Duration::from_secs(1))
        .is_err());

    release.send(()).unwrap();
    assert!(spawned_pid.recv_timeout(Duration::from_secs(2)).unwrap() > 0);
    let cleanup_deadline = Instant::now() + Duration::from_secs(3);
    let report = pool.shutdown(Duration::from_secs(1));
    assert_eq!(report.clean_exits, 1);
    while pool.counts().cleanup_unconfirmed > 0 {
        assert!(Instant::now() < cleanup_deadline);
        pool.shutdown(Duration::ZERO);
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(pool.counts().retired, MAX_SLOTS);
}

#[test]
fn dropping_pool_leaves_late_child_cleanup_with_its_launch_task() {
    let _guard = fd_table_lock();
    let (release, gate) = mpsc::channel();
    let gate = Mutex::new(gate);
    let (child_id, observed_child) = mpsc::channel();
    let pool = CustodyPool::start_with_slot_hook(
        build_identity(),
        Duration::from_millis(500),
        Duration::ZERO,
        move |slot, request, response| {
            assert_eq!(slot, 0);
            gate.lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .map_err(io::Error::other)?;
            let child =
                codex_capture_owner::pool::spawn_worker_process(sidecar_bin(), request, response)?;
            let _ = child_id.send(child.id());
            Ok(child)
        },
    );
    assert_eq!(pool.counts().cleanup_unconfirmed, 1);
    drop(pool);
    release.send(()).unwrap();

    let pid = observed_child.recv_timeout(Duration::from_secs(2)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        // SAFETY: The reported pid is our exact freshly spawned child.
        // WNOWAIT observes it without competing with the owner's reap.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let status = unsafe {
            libc::waitid(
                libc::P_PID,
                pid as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if status != 0 && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD) {
            break;
        }
        assert!(Instant::now() < deadline, "late child remained unreaped");
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn partial_readiness_and_late_startup_quarantine_retire_failed_slots_without_replacement() {
    let _guard = fd_table_lock();
    let attempts = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&attempts);
    let mut pool = CustodyPool::start_with_slot_hook(
        build_identity(),
        STARTUP_BUDGET,
        Duration::from_millis(250),
        move |slot, request_rx_fd, response_tx_fd| {
            seen.fetch_add(1, Ordering::Relaxed);
            if slot == 2 {
                // Spawn a real silent process that holds both channel endpoints
                // open so the challenge write succeeds and readiness genuinely
                // stalls until the shared deadline expires.
                return spawn_silent_holder(request_rx_fd, response_tx_fd);
            }
            codex_capture_owner::pool::spawn_worker_process(
                sidecar_bin(),
                request_rx_fd,
                response_tx_fd,
            )
        },
    );

    assert_eq!(
        attempts.load(Ordering::Relaxed),
        3,
        "slot 3 must never start after shared deadline expires"
    );
    assert_eq!(pool.counts().ready, 2);
    assert_eq!(pool.counts().retired + pool.counts().cleanup_unconfirmed, 2);

    // Ready slots 0 and 1 remain usable; unavailable slots 2 and 3 fail closed.
    let deadline = Instant::now() + Duration::from_secs(2);
    pool.ping_slot(0, deadline).expect("slot 0 ready");
    pool.ping_slot(1, deadline).expect("slot 1 ready");
    assert!(pool.ping_slot(2, deadline).is_err());
    assert!(pool.ping_slot(3, deadline).is_err());

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, 2);
    assert_eq!(report.forced_kills, 0);
    assert_eq!(report.cleanup_unconfirmed, 0);
}

#[test]
fn unconfirmed_cleanup_remains_charged_to_slot_until_reaped() {
    let _guard = fd_table_lock();
    let attempts = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&attempts);
    let mut pool = CustodyPool::start_with_slot_hook(
        build_identity(),
        Duration::from_millis(200),
        Duration::ZERO,
        move |slot, request_rx_fd, response_tx_fd| {
            seen.fetch_add(1, Ordering::Relaxed);
            if slot == 0 {
                return spawn_silent_holder(request_rx_fd, response_tx_fd);
            }
            codex_capture_owner::pool::spawn_worker_process(
                sidecar_bin(),
                request_rx_fd,
                response_tx_fd,
            )
        },
    );
    assert_eq!(
        attempts.load(Ordering::Relaxed),
        1,
        "no additional slot may start after slot 0 exhausts the shared deadline"
    );
    let counts = pool.counts();
    assert_eq!(counts.ready, 0);
    assert_eq!(counts.retired + counts.cleanup_unconfirmed, MAX_SLOTS);
    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.cleanup_unconfirmed, 0);
    assert_eq!(pool.counts().retired, MAX_SLOTS);
}

#[test]
fn unexpected_worker_death_retires_slot_permanently_without_replacement() {
    let _guard = fd_table_lock();
    let mut pool =
        CustodyPool::start_for_executable(sidecar_bin(), build_identity(), STARTUP_BUDGET);
    assert_eq!(pool.counts().ready, MAX_SLOTS);

    let pids = pool.ready_pids();
    let victim_pid = pids[1] as libc::pid_t;
    // SAFETY: victim_pid is an owned worker child launched by this pool.
    assert_eq!(unsafe { libc::kill(victim_pid, libc::SIGKILL) }, 0);
    wait_until_zombie(victim_pid, Duration::from_secs(1));

    let deadline = Instant::now() + Duration::from_secs(2);
    assert!(
        pool.ping_slot(1, deadline).is_err(),
        "pinging a dead worker must fail closed and retire its slot"
    );
    let counts = pool.counts();
    assert_eq!(counts.ready, MAX_SLOTS - 1);
    assert_eq!(counts.retired, 1);
    assert_eq!(counts.cleanup_unconfirmed, 0);

    for slot in [0, 2, 3] {
        pool.ping_slot(slot, deadline)
            .unwrap_or_else(|err| panic!("surviving slot {slot} should stay healthy: {err}"));
    }

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, MAX_SLOTS - 1);
    assert_eq!(report.forced_kills, 0);
    assert_eq!(report.cleanup_unconfirmed, 0);
}

fn spawn_silent_holder(request_rx_fd: RawFd, response_tx_fd: RawFd) -> io::Result<OwnedChild> {
    let mut cmd = Command::new("/bin/sleep");
    cmd.arg("5")
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: caller keeps both FDs open across spawn; clearing CLOEXEC on
    // both endpoints is async-signal-safe so `/bin/sleep` keeps the channels
    // open without responding until the parent kills it on deadline expiry.
    unsafe {
        cmd.pre_exec(move || {
            if libc::fcntl(request_rx_fd, libc::F_SETFD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::fcntl(response_tx_fd, libc::F_SETFD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = cmd.spawn()?;
    let pid = child.id() as libc::pid_t;
    std::mem::forget(child);
    OwnedChild::try_from_pid(pid)
}

fn wait_until_zombie(pid: libc::pid_t, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        // SAFETY: siginfo_t is zeroed and waitid with WNOWAIT | WNOHANG
        // observes whether `pid` has exited without reaping it.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let rc = unsafe {
            libc::waitid(
                libc::P_PID,
                pid as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if rc == 0 && info.si_pid == pid {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("victim worker pid {pid} did not enter zombie state within {timeout:?}");
}

struct TaskCost {
    resident_bytes: u64,
    total_cpu_ns: u64,
}

fn sample_task_cost(pid: u32) -> TaskCost {
    let mut info = std::mem::MaybeUninit::<libc::proc_taskinfo>::zeroed();
    let expected = std::mem::size_of::<libc::proc_taskinfo>() as libc::c_int;
    // SAFETY: proc_taskinfo buffer is properly aligned and sized for PROC_PIDTASKINFO.
    let written = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTASKINFO,
            0,
            info.as_mut_ptr().cast(),
            expected,
        )
    };
    assert_eq!(written, expected, "PROC_PIDTASKINFO failed for pid {pid}");
    // SAFETY: proc_pidinfo wrote the entire proc_taskinfo struct.
    let info = unsafe { info.assume_init() };
    TaskCost {
        resident_bytes: info.pti_resident_size,
        total_cpu_ns: info.pti_total_user.saturating_add(info.pti_total_system),
    }
}

fn assert_closed(fd: RawFd) {
    // SAFETY: F_GETFD takes no pointer and reports closed descriptors as EBADF.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    let error = io::Error::last_os_error();
    assert_eq!(flags, -1, "descriptor {fd} remains open");
    assert_eq!(error.raw_os_error(), Some(libc::EBADF));
}

fn write_raw_all(fd: RawFd, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        // SAFETY: `fd` is an open test pipe and `bytes` is a valid slice.
        let n = unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len()) };
        assert!(
            n > 0,
            "write_raw_all failed: {}",
            io::Error::last_os_error()
        );
        bytes = &bytes[n as usize..];
    }
}

fn list_own_open_fds() -> Vec<RawFd> {
    let pid = std::process::id() as libc::c_int;
    // SAFETY: querying required byte size with null buffer is documented for PROC_PIDLISTFDS.
    let estimate =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    assert!(estimate > 0);
    let entry_size = std::mem::size_of::<libc::proc_fdinfo>();
    let capacity = (estimate as usize / entry_size) + 16;
    let mut entries = vec![
        libc::proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0,
        };
        capacity
    ];
    let byte_len = (capacity * entry_size) as libc::c_int;
    // SAFETY: entries is aligned and has capacity * entry_size writable bytes.
    let actual = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            entries.as_mut_ptr().cast(),
            byte_len,
        )
    };
    assert!(actual > 0 && (actual as usize) < capacity * entry_size);
    let mut fds: Vec<RawFd> = entries[..(actual as usize / entry_size)]
        .iter()
        .map(|entry| entry.proc_fd)
        .collect();
    fds.sort_unstable();
    fds
}

fn fd_matches_object(fd: RawFd, target: (u64, u64)) -> bool {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: fstat writes only into stat on success.
    if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
        return false;
    }
    // SAFETY: fstat succeeded.
    let stat = unsafe { stat.assume_init() };
    (stat.st_dev as u64, stat.st_ino) == target
}

fn send_with_scm_rights(fd: RawFd, bytes: &[u8], fd_to_send: RawFd) -> io::Result<()> {
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr() as *mut libc::c_void,
        iov_len: bytes.len(),
    };
    // SAFETY: CMSG_SPACE computes required control buffer length for one RawFd.
    let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<RawFd>() as u32) } as usize;
    // Allocate as u64 words so alignment (>= 8) strictly satisfies align_of::<libc::cmsghdr>().
    let mut control = vec![0u64; cmsg_space.div_ceil(std::mem::size_of::<u64>())];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = cmsg_space as libc::socklen_t;

    // SAFETY: control buffer is 8-byte aligned and sized by CMSG_SPACE for cmsghdr.
    unsafe {
        let cmsg = libc::CMSG_FIRSTHDR(&msg);
        assert!(!cmsg.is_null());
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<RawFd>() as u32) as libc::socklen_t;
        std::ptr::copy_nonoverlapping(
            &fd_to_send as *const RawFd as *const u8,
            libc::CMSG_DATA(cmsg),
            std::mem::size_of::<RawFd>(),
        );
        let sent = libc::sendmsg(fd, &msg, 0);
        if sent < 0 {
            return Err(io::Error::last_os_error());
        }
        if sent as usize != bytes.len() {
            return Err(io::Error::new(ErrorKind::WriteZero, "short sendmsg"));
        }
    }
    Ok(())
}
