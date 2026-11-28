//! Bounded I/O over a nonblocking qualification channel.

use std::io::{self, ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

fn remaining(deadline: Instant) -> io::Result<Duration> {
    let time = deadline.saturating_duration_since(Instant::now());
    if time.is_zero() {
        return Err(ErrorKind::TimedOut.into());
    }
    Ok(time)
}

fn wait(stream: &UnixStream, events: libc::c_short, deadline: Instant) -> io::Result<()> {
    let mut interest = libc::pollfd {
        fd: stream.as_raw_fd(),
        events,
        revents: 0,
    };
    loop {
        let millis = remaining(deadline)?.as_millis().clamp(1, i32::MAX as u128) as i32;
        // SAFETY: interest points to one writable pollfd and the stream stays open.
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

pub(super) fn read(
    mut stream: &UnixStream,
    bytes: &mut [u8],
    deadline: Instant,
) -> io::Result<usize> {
    loop {
        wait(stream, libc::POLLIN, deadline)?;
        match stream.read(bytes) {
            Err(error)
                if matches!(error.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) => {}
            result => {
                remaining(deadline)?;
                return result;
            }
        }
    }
}

pub(super) fn read_exact(
    stream: &UnixStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> io::Result<()> {
    while !bytes.is_empty() {
        let count = read(stream, bytes, deadline)?;
        if count == 0 {
            return Err(ErrorKind::UnexpectedEof.into());
        }
        bytes = &mut bytes[count..];
    }
    Ok(())
}

pub(super) fn write_all(
    mut stream: &UnixStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> io::Result<()> {
    while !bytes.is_empty() {
        wait(stream, libc::POLLOUT, deadline)?;
        match stream.write(bytes) {
            Ok(0) => return Err(ErrorKind::WriteZero.into()),
            Ok(count) => bytes = &bytes[count..],
            Err(error)
                if matches!(error.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) => {}
            Err(error) => return Err(error),
        }
    }
    remaining(deadline)?;
    Ok(())
}
