//! Private persistent non-spawning worker entry for the macOS custody pool.
//!
//! Dispatched before normal sidecar runtime or Git helper startup. Validates
//! capability-free environment, complete `PROC_PIDLISTFDS` inherited-descriptor
//! inventory (`{0, 1, 2, request_rx, response_tx}`), `/dev/null` standard
//! streams, unidirectional `AF_UNIX` request socket + OS `pipe()` response
//! descriptor, and exact build/nonce bootstrap handshake before entering a
//! zero-CPU idle loop.

use std::ffi::OsStr;
use std::io;
use std::net::Shutdown;
use std::os::fd::{AsFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::{fs::MetadataExt, net::UnixStream};
use std::time::{Duration, Instant};

use super::bootstrap_frame::{self, Direction};
use super::channels::{enable_fd_nosigpipe, enable_socket_nosigpipe, set_cloexec};
use super::framing::{self, ControlFrame};

pub(crate) const WORKER_ARG: &str = "--codex-custody-worker";

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);
const REPLY_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_FD_INVENTORY_BYTES: usize = 1024 * 1024;

const EXIT_BAD_ARGS: i32 = 64;
const EXIT_NON_EMPTY_ENV: i32 = 65;
const EXIT_BAD_FD_INVENTORY: i32 = 66;
const EXIT_BAD_STDIO: i32 = 67;
const EXIT_BAD_CHANNEL: i32 = 68;
const EXIT_HANDSHAKE_FAILED: i32 = 69;
const EXIT_PROTOCOL_ERROR: i32 = 70;

/// Early private-worker dispatch seam invoked at the start of `main()`.
///
/// Returns `None` for normal invocations so regular startup proceeds unchanged.
/// Returns `Some(exit_code)` whenever the private worker selector is present,
/// ensuring a custody worker never falls through to normal sidecar runtime or
/// Git helper code.
pub(crate) fn run_private_worker() -> Option<i32> {
    let mut args = std::env::args_os();
    let _exe = args.next();
    if args.next().as_deref() != Some(OsStr::new(WORKER_ARG)) {
        return None;
    }
    let request_arg = args.next();
    let response_arg = args.next();
    let extra_arg = args.next();
    match run_worker(
        request_arg.as_deref(),
        response_arg.as_deref(),
        extra_arg.is_some(),
    ) {
        Ok(()) => Some(0),
        Err(code) => Some(code),
    }
}

fn run_worker(
    request_arg: Option<&OsStr>,
    response_arg: Option<&OsStr>,
    has_extra_args: bool,
) -> Result<(), i32> {
    if has_extra_args {
        return Err(EXIT_BAD_ARGS);
    }
    let request_fd = parse_strict_fd(request_arg).ok_or(EXIT_BAD_ARGS)?;
    let response_fd = parse_strict_fd(response_arg).ok_or(EXIT_BAD_ARGS)?;
    if request_fd == response_fd {
        return Err(EXIT_BAD_ARGS);
    }

    verify_capability_free_env().map_err(|_| EXIT_NON_EMPTY_ENV)?;
    verify_exact_inherited_fds(request_fd, response_fd).map_err(|_| EXIT_BAD_FD_INVENTORY)?;
    verify_null_stdio().map_err(|_| EXIT_BAD_STDIO)?;

    let request_rx = adopt_request_socket(request_fd).map_err(|_| EXIT_BAD_CHANNEL)?;
    let response_tx = adopt_response_pipe(response_fd).map_err(|_| EXIT_BAD_CHANNEL)?;

    perform_handshake(&request_rx, &response_tx).map_err(|_| EXIT_HANDSHAKE_FAILED)?;
    run_idle_loop(&request_rx, &response_tx).map_err(|_| EXIT_PROTOCOL_ERROR)
}

fn verify_capability_free_env() -> Result<(), ()> {
    // `orkworksd` links CoreFoundation; its pre-`main` initializer populates
    // `__CF_USER_TEXT_ENCODING` (`0x<uid>:0x<enc>:0x<lang>`) even when spawned
    // with an empty environment. Reject every other environment variable.
    for (key, value) in std::env::vars_os() {
        if key != OsStr::new("__CF_USER_TEXT_ENCODING") {
            return Err(());
        }
        let text = value.to_str().ok_or(())?;
        if text.is_empty()
            || text.len() > 64
            || !text
                .bytes()
                .all(|b| b.is_ascii_hexdigit() || b == b'x' || b == b':')
        {
            return Err(());
        }
    }
    Ok(())
}

fn parse_strict_fd(arg: Option<&OsStr>) -> Option<RawFd> {
    let text = arg?.to_str()?;
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let fd: RawFd = text.parse().ok()?;
    (fd > 2).then_some(fd)
}

fn verify_exact_inherited_fds(request_fd: RawFd, response_fd: RawFd) -> io::Result<()> {
    let pid = std::process::id() as libc::c_int;
    let entry_size = std::mem::size_of::<libc::proc_fdinfo>();
    // SAFETY: querying the byte length with a null buffer is the documented macOS
    // PROC_PIDLISTFDS sizing pattern.
    let estimate =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    if estimate <= 0 {
        return Err(io::Error::last_os_error());
    }
    let estimate = estimate as usize;
    if estimate > MAX_FD_INVENTORY_BYTES || estimate % entry_size != 0 {
        return Err(io::Error::other("invalid descriptor inventory size"));
    }

    // Reserve spare entries so we never trust a truncated full buffer.
    let capacity = estimate
        .checked_add(16 * entry_size)
        .ok_or_else(|| io::Error::other("descriptor inventory overflow"))?
        .min(MAX_FD_INVENTORY_BYTES)
        / entry_size;
    let mut rows = vec![
        libc::proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0,
        };
        capacity
    ];
    let byte_len =
        libc::c_int::try_from(capacity * entry_size).map_err(|_| io::Error::other("size cap"))?;
    // SAFETY: rows is aligned and has `byte_len` writable bytes.
    let actual = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            rows.as_mut_ptr().cast(),
            byte_len,
        )
    };
    if actual <= 0 {
        return Err(io::Error::last_os_error());
    }
    let actual = actual as usize;
    if actual >= capacity * entry_size || actual % entry_size != 0 {
        return Err(io::Error::other("truncated descriptor inventory"));
    }

    let mut fds: Vec<RawFd> = rows[..(actual / entry_size)]
        .iter()
        .map(|row| row.proc_fd)
        .collect();
    fds.sort_unstable();

    let mut expected = [0, 1, 2, request_fd, response_fd];
    expected.sort_unstable();
    if fds.as_slice() != expected.as_slice() {
        return Err(io::Error::other("unexpected inherited descriptor set"));
    }
    Ok(())
}

fn verify_null_stdio() -> io::Result<()> {
    let null_meta = std::fs::metadata("/dev/null")?;
    let expected = (null_meta.dev(), null_meta.ino());
    for fd in 0..=2 {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: fstat writes only to `stat` on success.
        if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fstat succeeded.
        let stat = unsafe { stat.assume_init() };
        if (stat.st_dev as u64, stat.st_ino) != expected {
            return Err(io::Error::other("standard stream is not /dev/null"));
        }
    }
    Ok(())
}

fn adopt_request_socket(selected: RawFd) -> io::Result<UnixStream> {
    set_cloexec(selected)?;
    let mut sock_type: libc::c_int = 0;
    let mut sock_type_len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    // SAFETY: sock_type is writable and sized by sock_type_len.
    if unsafe {
        libc::getsockopt(
            selected,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&mut sock_type as *mut libc::c_int).cast(),
            &mut sock_type_len,
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    if sock_type != libc::SOCK_STREAM {
        return Err(io::Error::other(
            "selected request descriptor is not SOCK_STREAM",
        ));
    }

    // SAFETY: `selected` was verified present in our descriptor inventory and
    // has no other Rust owner.
    let owned = unsafe { OwnedFd::from_raw_fd(selected) };
    let stream = UnixStream::from(owned);
    // Confirm it is a connected Unix domain socket.
    stream.peer_addr()?;
    enable_socket_nosigpipe(selected)?;
    // Defense-in-depth: keep worker write direction shut down on request socket.
    let _ = stream.shutdown(Shutdown::Write);
    Ok(stream)
}

fn adopt_response_pipe(selected: RawFd) -> io::Result<OwnedFd> {
    set_cloexec(selected)?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: fstat writes only to `stat` on success.
    if unsafe { libc::fstat(selected, stat.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fstat succeeded.
    let stat = unsafe { stat.assume_init() };
    if (stat.st_mode & libc::S_IFMT) != libc::S_IFIFO {
        return Err(io::Error::other(
            "selected response descriptor is not a pipe",
        ));
    }
    // Verify it is not a socket (ENOTSOCK) and is open for writing.
    let mut sock_type: libc::c_int = 0;
    let mut sock_type_len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    // SAFETY: checking getsockopt must fail with ENOTSOCK on an OS pipe.
    if unsafe {
        libc::getsockopt(
            selected,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&mut sock_type as *mut libc::c_int).cast(),
            &mut sock_type_len,
        )
    } == 0
    {
        return Err(io::Error::other(
            "selected response descriptor cannot be a socket",
        ));
    }
    if io::Error::last_os_error().raw_os_error() != Some(libc::ENOTSOCK) {
        return Err(io::Error::other(
            "selected response descriptor did not report ENOTSOCK",
        ));
    }
    // SAFETY: `selected` is open and F_GETFL takes no pointer argument.
    let status_flags = unsafe { libc::fcntl(selected, libc::F_GETFL) };
    if status_flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if (status_flags & libc::O_ACCMODE) == libc::O_RDONLY {
        return Err(io::Error::other(
            "selected response pipe descriptor is not writable",
        ));
    }
    enable_fd_nosigpipe(selected)?;

    // SAFETY: `selected` was verified present in our descriptor inventory and
    // has no other Rust owner.
    Ok(unsafe { OwnedFd::from_raw_fd(selected) })
}

fn perform_handshake(request_rx: &UnixStream, response_tx: &OwnedFd) -> io::Result<()> {
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let frame = framing::read_bootstrap_frame(request_rx.as_fd(), deadline)?;
    let nonce: [u8; 16] = frame[6..22]
        .try_into()
        .map_err(|_| io::Error::other("nonce slice"))?;
    let expected_build = super::build_identity();
    if !bootstrap_frame::matches(&frame, Direction::Challenge, &nonce, &expected_build) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bootstrap challenge mismatch",
        ));
    }
    framing::ensure_no_trailing_bytes(request_rx.as_fd())?;
    let ready = bootstrap_frame::encode(Direction::Ready, &nonce, &expected_build);
    framing::write_bootstrap_frame(response_tx.as_fd(), &ready, deadline)
}

fn run_idle_loop(request_rx: &UnixStream, response_tx: &OwnedFd) -> io::Result<()> {
    loop {
        match framing::read_control_frame_or_eof(request_rx.as_fd(), None)? {
            None => return Ok(()),
            Some(ControlFrame::Ping(nonce)) => {
                let deadline = Instant::now() + REPLY_TIMEOUT;
                framing::write_control_frame(
                    response_tx.as_fd(),
                    &ControlFrame::Pong(nonce),
                    deadline,
                )?;
            }
            Some(ControlFrame::Shutdown) => {
                let deadline = Instant::now() + REPLY_TIMEOUT;
                framing::write_control_frame(
                    response_tx.as_fd(),
                    &ControlFrame::ShutdownAck,
                    deadline,
                )?;
                return Ok(());
            }
            Some(ControlFrame::Pong(_) | ControlFrame::ShutdownAck) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unexpected response frame received by worker",
                ));
            }
        }
    }
}
