//! Exclusive-startup channel construction for the macOS custody qualification.

use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

type ChannelPair = (UnixStream, UnixStream);
type ChannelPairs = [ChannelPair; 4];

pub(super) fn seal_empty_channels() -> io::Result<ChannelPairs> {
    seal_empty_channels_using(UnixStream::pair)
}

pub(super) fn seal_empty_channels_using(
    mut create_pair: impl FnMut() -> io::Result<ChannelPair>,
) -> io::Result<ChannelPairs> {
    let mut sealed_pair = || {
        let (controller, worker) = create_pair()?;
        verify_sealed(&controller)?;
        verify_sealed(&worker)?;
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

fn verify_sealed(endpoint: &UnixStream) -> io::Result<()> {
    // SAFETY: endpoint remains open for this call and F_GETFD takes no
    // pointer or additional argument.
    let flags = unsafe { libc::fcntl(endpoint.as_raw_fd(), libc::F_GETFD) };
    if flags < 0 {
        Err(io::Error::last_os_error())
    } else if flags & libc::FD_CLOEXEC == 0 {
        Err(io::Error::other("custody channel is missing close-on-exec"))
    } else {
        Ok(())
    }
}
