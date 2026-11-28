//! Persistent non-EOF byte framing for the macOS custody channels.
//!
//! Operates on `BorrowedFd` so requests traverse the unidirectional `AF_UNIX`
//! request channel while responses traverse the unidirectional OS `pipe()`
//! response channel (where `SCM_RIGHTS` transfer is kernel-impossible).

use std::io::{self, ErrorKind};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::time::{Duration, Instant};

use super::bootstrap_frame::FRAME_SIZE;

const MAGIC: [u8; 4] = *b"OWCO";
const PROTOCOL_VERSION: u8 = 1;
const HEADER_LEN: usize = 10;
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;

const KIND_PING: u8 = 1;
const KIND_PONG: u8 = 2;
const KIND_SHUTDOWN: u8 = 3;
const KIND_SHUTDOWN_ACK: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ControlFrame {
    Ping([u8; 16]),
    Pong([u8; 16]),
    Shutdown,
    ShutdownAck,
}

pub(crate) fn encode_control_frame(frame: &ControlFrame) -> Vec<u8> {
    let (kind, payload): (u8, &[u8]) = match frame {
        ControlFrame::Ping(nonce) => (KIND_PING, nonce),
        ControlFrame::Pong(nonce) => (KIND_PONG, nonce),
        ControlFrame::Shutdown => (KIND_SHUTDOWN, &[]),
        ControlFrame::ShutdownAck => (KIND_SHUTDOWN_ACK, &[]),
    };
    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(&MAGIC);
    out.push(PROTOCOL_VERSION);
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

pub(crate) fn write_bootstrap_frame(
    fd: BorrowedFd<'_>,
    frame: &[u8; FRAME_SIZE],
    deadline: Instant,
) -> io::Result<()> {
    write_all_deadline(fd, frame, deadline)
}

pub(crate) fn read_bootstrap_frame(
    fd: BorrowedFd<'_>,
    deadline: Instant,
) -> io::Result<[u8; FRAME_SIZE]> {
    let mut buf = [0u8; FRAME_SIZE];
    read_exact_deadline(fd, &mut buf, Some(deadline))?;
    Ok(buf)
}

pub(crate) fn write_control_frame(
    fd: BorrowedFd<'_>,
    frame: &ControlFrame,
    deadline: Instant,
) -> io::Result<()> {
    let encoded = encode_control_frame(frame);
    write_all_deadline(fd, &encoded, deadline)
}

pub(crate) fn read_control_frame(
    fd: BorrowedFd<'_>,
    deadline: Instant,
) -> io::Result<ControlFrame> {
    match read_control_frame_or_eof(fd, Some(deadline))? {
        Some(frame) => Ok(frame),
        None => Err(ErrorKind::UnexpectedEof.into()),
    }
}

/// Reads the next persistent control frame, returning `Ok(None)` only on clean
/// EOF at a frame boundary (zero bytes read before EOF).
pub(crate) fn read_control_frame_or_eof(
    fd: BorrowedFd<'_>,
    deadline: Option<Instant>,
) -> io::Result<Option<ControlFrame>> {
    set_nonblocking(fd.as_raw_fd())?;
    let mut header = [0u8; HEADER_LEN];
    let first = read_chunk_deadline(fd, &mut header[..1], deadline)?;
    if first == 0 {
        return Ok(None);
    }
    read_exact_deadline(fd, &mut header[1..], deadline)?;

    if header[0..4] != MAGIC || header[4] != PROTOCOL_VERSION {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid custody control header",
        ));
    }
    let kind = header[5];
    let len = u32::from_be_bytes([header[6], header[7], header[8], header[9]]) as usize;
    if len > MAX_PAYLOAD_BYTES {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "custody control payload exceeds cap",
        ));
    }

    match kind {
        KIND_PING | KIND_PONG => {
            if len != 16 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid ping/pong payload length",
                ));
            }
            let mut nonce = [0u8; 16];
            read_exact_deadline(fd, &mut nonce, deadline)?;
            Ok(Some(if kind == KIND_PING {
                ControlFrame::Ping(nonce)
            } else {
                ControlFrame::Pong(nonce)
            }))
        }
        KIND_SHUTDOWN | KIND_SHUTDOWN_ACK => {
            if len != 0 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "shutdown frame must carry empty payload",
                ));
            }
            Ok(Some(if kind == KIND_SHUTDOWN {
                ControlFrame::Shutdown
            } else {
                ControlFrame::ShutdownAck
            }))
        }
        _ => Err(io::Error::new(
            ErrorKind::InvalidData,
            "unknown custody control frame kind",
        )),
    }
}

/// Confirms no unsolicited trailing bytes are buffered and the peer has not
/// unexpectedly closed a persistent channel immediately after a complete frame.
pub(crate) fn ensure_no_trailing_bytes(fd: BorrowedFd<'_>) -> io::Result<()> {
    let mut interest = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        // SAFETY: interest is one valid pollfd and `fd` remains open.
        match unsafe { libc::poll(&mut interest, 1, 0) } {
            0 => return Ok(()),
            value if value > 0 => {
                if (interest.revents & libc::POLLNVAL) != 0 {
                    return Err(io::Error::new(
                        ErrorKind::BrokenPipe,
                        "custody channel descriptor is invalid",
                    ));
                }
                let mut readable: libc::c_int = 0;
                // SAFETY: FIONREAD writes one c_int byte count on macOS pipes and sockets.
                let ioctl_ok =
                    unsafe { libc::ioctl(fd.as_raw_fd(), libc::FIONREAD, &mut readable) };
                if ioctl_ok == 0 && readable > 0 {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        "unexpected trailing bytes on custody channel",
                    ));
                }
                if (interest.revents & (libc::POLLHUP | libc::POLLERR)) != 0 {
                    return Err(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "custody channel closed unexpectedly",
                    ));
                }
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "unexpected readable state on custody channel",
                ));
            }
            _ => {
                let err = io::Error::last_os_error();
                if err.kind() != ErrorKind::Interrupted {
                    return Err(err);
                }
            }
        }
    }
}

pub(crate) fn remaining(deadline: Instant) -> io::Result<Duration> {
    let time = deadline.saturating_duration_since(Instant::now());
    if time.is_zero() {
        return Err(io::Error::new(
            ErrorKind::TimedOut,
            "custody operation deadline expired",
        ));
    }
    Ok(time)
}

fn write_all_deadline(fd: BorrowedFd<'_>, mut bytes: &[u8], deadline: Instant) -> io::Result<()> {
    let raw = fd.as_raw_fd();
    set_nonblocking(raw)?;
    while !bytes.is_empty() {
        wait_io(raw, libc::POLLOUT, Some(deadline))?;
        // SAFETY: `raw` is open for `fd`'s lifetime and `bytes` is a valid readable buffer.
        let written = unsafe { libc::write(raw, bytes.as_ptr().cast(), bytes.len()) };
        if written == 0 {
            return Err(ErrorKind::WriteZero.into());
        }
        if written < 0 {
            let err = io::Error::last_os_error();
            if matches!(err.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) {
                continue;
            }
            return Err(err);
        }
        bytes = &bytes[written as usize..];
    }
    remaining(deadline)?;
    Ok(())
}

fn read_exact_deadline(
    fd: BorrowedFd<'_>,
    buf: &mut [u8],
    deadline: Option<Instant>,
) -> io::Result<()> {
    set_nonblocking(fd.as_raw_fd())?;
    let mut offset = 0;
    while offset < buf.len() {
        let count = read_chunk_deadline(fd, &mut buf[offset..], deadline)?;
        if count == 0 {
            return Err(ErrorKind::UnexpectedEof.into());
        }
        offset += count;
    }
    if let Some(limit) = deadline {
        remaining(limit)?;
    }
    Ok(())
}

fn read_chunk_deadline(
    fd: BorrowedFd<'_>,
    buf: &mut [u8],
    deadline: Option<Instant>,
) -> io::Result<usize> {
    let raw = fd.as_raw_fd();
    loop {
        wait_io(raw, libc::POLLIN, deadline)?;
        // SAFETY: `raw` is open for `fd`'s lifetime and `buf` is a valid writable buffer.
        let count = unsafe { libc::read(raw, buf.as_mut_ptr().cast(), buf.len()) };
        if count < 0 {
            let err = io::Error::last_os_error();
            if matches!(err.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) {
                continue;
            }
            return Err(err);
        }
        return Ok(count as usize);
    }
}

fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    // SAFETY: `fd` is open and F_GETFL takes no pointer argument.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if (flags & libc::O_NONBLOCK) == 0 {
        // SAFETY: `fd` is open and F_SETFL sets O_NONBLOCK cleanly.
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

fn wait_io(fd: RawFd, events: libc::c_short, deadline: Option<Instant>) -> io::Result<()> {
    let mut interest = libc::pollfd {
        fd,
        events,
        revents: 0,
    };
    loop {
        let millis = match deadline {
            Some(limit) => remaining(limit)?.as_millis().clamp(1, i32::MAX as u128) as i32,
            None => -1,
        };
        // SAFETY: interest is one valid pollfd and `fd` remains open for this call.
        match unsafe { libc::poll(&mut interest, 1, millis) } {
            value if value > 0 => {
                if (interest.revents & libc::POLLNVAL) != 0 {
                    return Err(io::Error::new(
                        ErrorKind::BrokenPipe,
                        "custody channel descriptor is invalid",
                    ));
                }
                return Ok(());
            }
            0 => {}
            _ => {
                let err = io::Error::last_os_error();
                if err.kind() != ErrorKind::Interrupted {
                    return Err(err);
                }
            }
        }
    }
}
