//! Exclusive-startup channel construction for the macOS custody qualification.
//!
//! Per the 2026-11-28 amendment to ADR 0122, each persistent custody slot uses
//! two pre-sealed unidirectional channels:
//! - an `AF_UNIX` `SOCK_STREAM` request channel (`parent -> worker`), with the
//!   reverse direction shut down at both ends (`SHUT_RD` / `SHUT_WR`) so no
//!   worker can send `SCM_RIGHTS` back to the parent; and
//! - an OS `pipe()` response channel (`worker -> parent`), where `sendmsg`
//!   fails with `ENOTSOCK` so descriptor installation into the parent is
//!   structurally impossible in the kernel.

use std::io;
use std::net::Shutdown;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;

pub(super) type ChannelPair = (UnixStream, UnixStream);
pub(super) type ChannelPairs = [ChannelPair; 4];

#[allow(dead_code)] // Used by the macOS custody pool qualification path.
pub(crate) struct SlotChannels {
    pub(crate) request_tx: UnixStream,
    pub(crate) request_rx: UnixStream,
    pub(crate) response_rx: OwnedFd,
    pub(crate) response_tx: OwnedFd,
}

#[allow(dead_code)] // Used by the macOS custody pool qualification path.
pub(crate) type SplitChannelSet = [SlotChannels; 4];

#[allow(dead_code)] // Retained for the existing codex_custody_bootstrap test.
pub(super) fn seal_empty_channels() -> io::Result<ChannelPairs> {
    seal_empty_channels_using(UnixStream::pair)
}

#[allow(dead_code)] // Retained for the existing codex_custody_bootstrap test.
pub(super) fn seal_empty_channels_using(
    mut create_pair: impl FnMut() -> io::Result<ChannelPair>,
) -> io::Result<ChannelPairs> {
    let mut sealed_pair = || {
        let (controller, worker) = create_pair()?;
        verify_sealed_fd(controller.as_raw_fd())?;
        verify_sealed_fd(worker.as_raw_fd())?;
        enable_socket_nosigpipe(controller.as_raw_fd())?;
        enable_socket_nosigpipe(worker.as_raw_fd())?;
        Ok::<_, io::Error>((controller, worker))
    };

    // All four pairs must pass before the caller obtains any endpoint. The
    // streams in the array initializer close on early return.
    Ok([
        sealed_pair()?,
        sealed_pair()?,
        sealed_pair()?,
        sealed_pair()?,
    ])
}

#[allow(dead_code)] // Used by the macOS custody pool qualification path.
pub(crate) fn seal_split_channels() -> io::Result<SplitChannelSet> {
    seal_split_channels_using(UnixStream::pair, create_sealed_pipe)
}

#[allow(dead_code)] // Used by the macOS custody pool qualification path and rollback tests.
pub(crate) fn seal_split_channels_using(
    mut create_request_pair: impl FnMut() -> io::Result<ChannelPair>,
    mut create_response_pipe: impl FnMut() -> io::Result<(OwnedFd, OwnedFd)>,
) -> io::Result<SplitChannelSet> {
    let mut sealed_slot = || {
        let (request_tx, request_rx) = create_request_pair()?;
        verify_sealed_fd(request_tx.as_raw_fd())?;
        verify_sealed_fd(request_rx.as_raw_fd())?;
        enable_socket_nosigpipe(request_tx.as_raw_fd())?;
        enable_socket_nosigpipe(request_rx.as_raw_fd())?;
        // Permanently seal the reverse direction on the AF_UNIX request channel.
        request_tx.shutdown(Shutdown::Read)?;
        request_rx.shutdown(Shutdown::Write)?;

        let (response_rx, response_tx) = create_response_pipe()?;
        verify_sealed_fd(response_rx.as_raw_fd())?;
        verify_sealed_fd(response_tx.as_raw_fd())?;
        enable_fd_nosigpipe(response_tx.as_raw_fd())?;

        Ok::<_, io::Error>(SlotChannels {
            request_tx,
            request_rx,
            response_rx,
            response_tx,
        })
    };

    // All four slots (16 endpoints total) must be created and verified
    // close-on-exec before any endpoint is returned or any child starts.
    Ok([
        sealed_slot()?,
        sealed_slot()?,
        sealed_slot()?,
        sealed_slot()?,
    ])
}

#[allow(dead_code)] // Used by seal_split_channels.
pub(crate) fn create_sealed_pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds: [RawFd; 2] = [-1, -1];
    // SAFETY: `fds` is a valid 2-element array writable by `pipe`.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `pipe` succeeded and returned two freshly created, unowned descriptors.
    let read_end = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    // SAFETY: same as above.
    let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };

    set_cloexec(read_end.as_raw_fd())?;
    set_cloexec(write_end.as_raw_fd())?;
    verify_sealed_fd(read_end.as_raw_fd())?;
    verify_sealed_fd(write_end.as_raw_fd())?;
    Ok((read_end, write_end))
}

#[allow(dead_code)]
pub(super) fn set_cloexec(fd: RawFd) -> io::Result<()> {
    // SAFETY: `fd` is open and owned by caller; F_SETFD takes one integer flag.
    if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(super) fn verify_sealed_fd(fd: RawFd) -> io::Result<()> {
    if fd <= 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "custody channel descriptor cannot occupy standard stream 0..=2",
        ));
    }
    // SAFETY: `fd` remains open for this call and F_GETFD takes no pointer.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        Err(io::Error::last_os_error())
    } else if flags & libc::FD_CLOEXEC == 0 {
        Err(io::Error::other("custody channel is missing close-on-exec"))
    } else {
        Ok(())
    }
}

pub(super) fn enable_socket_nosigpipe(fd: RawFd) -> io::Result<()> {
    let enabled: libc::c_int = 1;
    // SAFETY: `fd` is an open socket and `enabled` is a valid c_int for SO_NOSIGPIPE.
    let status = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_NOSIGPIPE,
            (&enabled as *const libc::c_int).cast(),
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    if status < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

// macOS SDK `sys/fcntl.h`: `#define F_SETNOSIGPIPE 73` (not re-exported by libc 0.2).
const F_SETNOSIGPIPE: libc::c_int = 73;

pub(super) fn enable_fd_nosigpipe(fd: RawFd) -> io::Result<()> {
    // SAFETY: `fd` is an open descriptor and macOS F_SETNOSIGPIPE takes an int flag.
    if unsafe { libc::fcntl(fd, F_SETNOSIGPIPE, 1 as libc::c_int) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
