//! One-shot native startup probe. The production lease stream does not use EOF framing.

use std::io::{self, ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::{
    fs::MetadataExt,
    net::UnixStream,
    process::{CommandExt, ExitStatusExt},
};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::assert_closed;
use super::bootstrap_frame::{self, Direction, FRAME_SIZE};

const SELECTED: &str = "ORKWORKS_CUSTODY_FIXTURE_SELECTED";
const REGISTERED: &str = "ORKWORKS_CUSTODY_FIXTURE_REGISTERED";
const BUILD: [u8; 32] = [91; 32];
const CHECKED: i32 = 42;
const STARTUP: Duration = Duration::from_secs(3);

pub(super) fn check_owned_workers(pairs: &[(UnixStream, UnixStream); 4]) {
    let registered: Vec<_> = pairs
        .iter()
        .flat_map(|(parent, worker)| [parent.as_raw_fd(), worker.as_raw_fd()])
        .collect();
    let deadline = Instant::now() + STARTUP;

    for (slot, (controller, worker)) in pairs.iter().enumerate() {
        remaining(deadline).expect("one shared startup deadline");
        let nonce = *uuid::Uuid::new_v4().as_bytes();
        let selected = worker.as_raw_fd();
        let mut child = spawn(selected, &registered).expect("start owned fixture worker");
        remaining(deadline).expect("child was acquired within the shared startup deadline");
        if slot == 3 {
            // Withhold the challenge. An owned process alone cannot publish readiness.
            let response_deadline = deadline.min(Instant::now() + Duration::from_millis(60));
            let error = await_ready(controller, &nonce, response_deadline)
                .expect_err("silent worker cannot be ready");
            assert_eq!(error.kind(), ErrorKind::TimedOut);
            let status = child
                .terminate()
                .expect("kill and reap silent fixture child");
            assert_eq!(status.signal(), Some(libc::SIGKILL));
            continue;
        }

        let challenge = bootstrap_frame::encode(Direction::Challenge, &nonce, &BUILD);
        write_frame(controller, &challenge, deadline).expect("deliver startup challenge");
        if slot == 2 {
            let mut different_nonce = nonce;
            different_nonce[0] ^= 1;
            let error = await_ready(controller, &different_nonce, deadline)
                .expect_err("response cannot confirm a different nonce");
            assert_eq!(error.kind(), ErrorKind::InvalidData);
        } else {
            await_ready(controller, &nonce, deadline).expect("confirm matching readiness");
        }
        let status = child.wait_until(deadline).expect("reap fixture worker");
        assert_eq!(
            status.code(),
            Some(CHECKED),
            "worker checks did not complete"
        );
    }

    for fd in registered {
        // SAFETY: every endpoint is still owned by pairs.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert!(flags >= 0 && flags & libc::FD_CLOEXEC != 0);
    }
}

fn await_ready(channel: &UnixStream, nonce: &[u8; 16], deadline: Instant) -> io::Result<()> {
    let ready = read_frame(channel, deadline)?;
    if !bootstrap_frame::matches(&ready, Direction::Ready, nonce, &BUILD) {
        return Err(ErrorKind::InvalidData.into());
    }
    Ok(())
}

fn spawn(selected: RawFd, registered: &[RawFd]) -> io::Result<FixtureChild> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", "check_selected_worker_channel", "--ignored"])
        .env_clear()
        .env(SELECTED, selected.to_string())
        .env(
            REGISTERED,
            registered
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: the caller retains the selected endpoint across spawn. All
    // registered endpoints were sealed before controlled exec. Only the
    // intended child clears its selected flag, using an async-signal-safe fcntl.
    unsafe {
        command.pre_exec(move || {
            if libc::fcntl(selected, libc::F_SETFD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn().map(FixtureChild)
}

pub(super) fn run_child() {
    assert_eq!(std::env::vars_os().count(), 2);
    let selected: RawFd = std::env::var(SELECTED).unwrap().parse().unwrap();
    let registered: Vec<RawFd> = std::env::var(REGISTERED)
        .unwrap()
        .split(',')
        .map(|fd| fd.parse().unwrap())
        .collect();
    assert_eq!(registered.len(), 8);
    assert!(registered.iter().all(|fd| *fd > 2));
    assert_eq!(registered.iter().filter(|&&fd| fd == selected).count(), 1);
    for fd in registered.into_iter().filter(|&fd| fd != selected) {
        assert_closed(fd);
    }

    let null = std::fs::metadata("/dev/null").expect("inspect the null device");
    for fd in 0..=2 {
        // SAFETY: fstat writes to the supplied native stat storage on success.
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        assert_eq!(unsafe { libc::fstat(fd, stat.as_mut_ptr()) }, 0);
        // SAFETY: fstat succeeded.
        let stat = unsafe { stat.assume_init() };
        assert_eq!((stat.st_dev as u64, stat.st_ino), (null.dev(), null.ino()));
    }

    let stream = duplicate_endpoint(selected).expect("adopt verified worker endpoint");
    stream.peer_addr().expect("selected Unix channel");
    let deadline = Instant::now() + STARTUP;
    let frame = read_frame(&stream, deadline).expect("read the challenge");
    let nonce: [u8; 16] = frame[6..22].try_into().expect("fixed nonce range");
    assert!(bootstrap_frame::matches(
        &frame,
        Direction::Challenge,
        &nonce,
        &BUILD
    ));
    let ready = bootstrap_frame::encode(Direction::Ready, &nonce, &BUILD);
    write_frame(&stream, &ready, deadline).expect("return matching readiness");
    std::process::exit(CHECKED);
}

fn duplicate_endpoint(selected: RawFd) -> io::Result<UnixStream> {
    // SAFETY: fcntl reports a closed descriptor as EBADF and returns a new,
    // close-on-exec descriptor on success.
    let duplicate = unsafe { libc::fcntl(selected, libc::F_DUPFD_CLOEXEC, 3) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the successful duplicate is fresh and has no other owner.
    Ok(UnixStream::from(unsafe { OwnedFd::from_raw_fd(duplicate) }))
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    let time = deadline.saturating_duration_since(Instant::now());
    if time.is_zero() {
        return Err(io::Error::new(
            ErrorKind::TimedOut,
            "fixture deadline expired",
        ));
    }
    Ok(time)
}

fn wait_io(stream: &UnixStream, events: libc::c_short, deadline: Instant) -> io::Result<()> {
    let mut interest = libc::pollfd {
        fd: stream.as_raw_fd(),
        events,
        revents: 0,
    };
    loop {
        let millis = remaining(deadline)?.as_millis().clamp(1, i32::MAX as u128) as i32;
        // SAFETY: interest is one writable pollfd and the stream remains open.
        match unsafe { libc::poll(&mut interest, 1, millis) } {
            value if value > 0 => return Ok(()),
            0 => {}
            _ => {
                let error = io::Error::last_os_error();
                if error.kind() != ErrorKind::Interrupted {
                    return Err(error);
                }
            }
        }
    }
}

fn write_frame(mut stream: &UnixStream, mut bytes: &[u8], deadline: Instant) -> io::Result<()> {
    stream.set_nonblocking(true)?;
    while !bytes.is_empty() {
        wait_io(stream, libc::POLLOUT, deadline)?;
        match stream.write(bytes) {
            Ok(0) => return Err(ErrorKind::WriteZero.into()),
            Ok(count) => bytes = &bytes[count..],
            Err(error)
                if matches!(error.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) => {}
            Err(error) => return Err(error),
        }
    }
    remaining(deadline)?;
    stream.shutdown(Shutdown::Write)
}

fn read_frame(mut stream: &UnixStream, deadline: Instant) -> io::Result<[u8; FRAME_SIZE]> {
    stream.set_nonblocking(true)?;
    let mut bytes = [0; FRAME_SIZE + 1];
    let mut length = 0;
    while length < bytes.len() {
        wait_io(stream, libc::POLLIN, deadline)?;
        match stream.read(&mut bytes[length..]) {
            Ok(0) if length == FRAME_SIZE => {
                remaining(deadline)?;
                return Ok(bytes[..FRAME_SIZE].try_into().expect("fixed frame size"));
            }
            Ok(0) => return Err(ErrorKind::UnexpectedEof.into()),
            Ok(count) => length += count,
            Err(error)
                if matches!(error.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) => {}
            Err(error) => return Err(error),
        }
    }
    Err(ErrorKind::InvalidData.into())
}

pub(super) fn verify_exact_stream_frames() {
    let (closed, _peer) = UnixStream::pair().expect("closed endpoint test");
    let fd = closed.as_raw_fd();
    drop(closed);
    assert_eq!(
        duplicate_endpoint(fd)
            .expect_err("closed endpoint cannot be adopted")
            .raw_os_error(),
        Some(libc::EBADF)
    );

    let frame = [0; FRAME_SIZE + 1];
    for (length, expected) in [
        (FRAME_SIZE - 1, Some(ErrorKind::UnexpectedEof)),
        (FRAME_SIZE, None),
        (FRAME_SIZE + 1, Some(ErrorKind::InvalidData)),
    ] {
        let (tx, rx) = UnixStream::pair().expect("local frame test");
        let deadline = Instant::now() + STARTUP;
        write_frame(&tx, &frame[..length], deadline).unwrap();
        assert_eq!(
            read_frame(&rx, deadline).err().map(|error| error.kind()),
            expected
        );
    }
}

struct FixtureChild(Child);

impl FixtureChild {
    fn wait_until(&mut self, deadline: Instant) -> io::Result<ExitStatus> {
        loop {
            let wait = remaining(deadline)?;
            if let Some(status) = self.0.try_wait()? {
                remaining(deadline)?;
                return Ok(status);
            }
            thread::sleep(wait.min(Duration::from_millis(5)));
        }
    }

    fn terminate(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.0.try_wait()? {
            return Ok(status);
        }
        self.0.kill()?;
        self.0.wait()
    }
}

impl Drop for FixtureChild {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
