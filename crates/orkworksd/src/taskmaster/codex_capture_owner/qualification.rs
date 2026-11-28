//! Opt-in early executable entry for the macOS custody qualification.

use std::io::{self, Read};
use std::mem::{size_of, size_of_val, MaybeUninit};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt, net::UnixStream};
use std::time::{Duration, Instant};

#[path = "bootstrap_frame.rs"]
mod bootstrap_frame;
#[path = "deadline_io.rs"]
mod deadline_io;
#[path = "qualification_contract.rs"]
mod qualification_contract;

use bootstrap_frame::{Direction, FRAME_SIZE};
use qualification_contract::WORKER_ARG;

pub(crate) fn run_if_selected() -> Option<i32> {
    if !qualification_contract::enabled() {
        return None;
    }

    let mut args = std::env::args_os().skip(1);
    if args.next()?.as_os_str() != WORKER_ARG {
        return None;
    }
    let fd = args
        .next()
        .and_then(|arg| arg.to_str()?.parse::<RawFd>().ok());
    if args.next().is_some() || !has_only_platform_locale() {
        return Some(2);
    }
    Some(match fd.filter(|fd| *fd > 2) {
        Some(fd) if run_worker(fd).is_ok() => 0,
        _ => 2,
    })
}

fn has_only_platform_locale() -> bool {
    let mut vars = std::env::vars_os();
    let Some((key, value)) = vars.next() else {
        return true;
    };
    if key != "__CF_USER_TEXT_ENCODING" || vars.next().is_some() {
        return false;
    }
    // The actual sidecar's macOS framework startup supplies a numeric locale
    // tuple even when exec was given an empty environment. The worker never
    // reads it for capture, and accepts no other environment entry.
    let mut fields = value.as_bytes().split(|byte| *byte == b':');
    (0..3).all(|_| {
        fields.next().is_some_and(|field| {
            let digits = field.strip_prefix(b"0x").unwrap_or(field);
            (1..=8).contains(&digits.len()) && digits.iter().all(u8::is_ascii_hexdigit)
        })
    }) && fields.next().is_none()
}

fn run_worker(fd: RawFd) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(3);
    verify_inventory(fd)?;
    verify_null_stdio()?;

    // SAFETY: fcntl fails for an unavailable descriptor and returns a fresh
    // descriptor with close-on-exec on success.
    let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the successful duplicate is owned only by this worker.
    let mut stream = UnixStream::from(unsafe { OwnedFd::from_raw_fd(duplicate) });
    // SAFETY: the original endpoint belongs to this exec worker, and no
    // Rust object owns it. All subsequent I/O uses the fresh duplicate.
    if unsafe { libc::close(fd) } != 0 {
        return Err(io::Error::last_os_error());
    }
    verify_stream_kind(duplicate)?;
    stream.peer_addr()?;
    stream.set_nonblocking(true)?;

    let mut challenge = [0; FRAME_SIZE];
    deadline_io::read_exact(&mut stream, &mut challenge, deadline)?;
    let nonce: [u8; 16] = challenge[6..22].try_into().expect("fixed nonce range");
    let tag = qualification_contract::qualification_tag();
    if !bootstrap_frame::matches(&challenge, Direction::Challenge, &nonce, &tag) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    require_quiet_channel(&stream)?;
    deadline_io::write_all(
        &mut stream,
        &bootstrap_frame::encode(Direction::Ready, &nonce, &tag),
        deadline,
    )?;

    // This qualification worker remains idle until controller EOF. There is
    // no lease operation connected to the stream yet.
    stream.set_nonblocking(false)?;
    if stream.read(&mut [0])? == 0 {
        Ok(())
    } else {
        Err(io::ErrorKind::InvalidData.into())
    }
}

fn require_quiet_channel(stream: &UnixStream) -> io::Result<()> {
    let mut byte = 0_u8;
    // SAFETY: the stream stays open and the writable buffer is one byte.
    // MSG_PEEK leaves the bounded startup stream unconsumed.
    let count = unsafe {
        libc::recv(
            stream.as_raw_fd(),
            (&mut byte as *mut u8).cast(),
            1,
            libc::MSG_PEEK,
        )
    };
    if count >= 0 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock {
        Ok(())
    } else {
        Err(error)
    }
}

fn verify_inventory(selected: RawFd) -> io::Result<()> {
    let mut rows = [const {
        libc::proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0,
        }
    }; 8];
    // SAFETY: the buffer comprises initialized, aligned SDK-layout records.
    // Its fixed byte capacity is representable as c_int. Inspect only self.
    let bytes = unsafe {
        libc::proc_pidinfo(
            libc::getpid(),
            libc::PROC_PIDLISTFDS,
            0,
            rows.as_mut_ptr().cast(),
            size_of_val(&rows) as libc::c_int,
        )
    };
    if bytes != (4 * size_of::<libc::proc_fdinfo>()) as libc::c_int {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let rows = &mut rows[..4];
    rows.sort_unstable_by_key(|row| row.proc_fd);
    if rows.iter().map(|row| row.proc_fd).ne([0, 1, 2, selected])
        || rows[3].proc_fdtype != libc::PROX_FDTYPE_SOCKET as u32
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(())
}

fn verify_stream_kind(fd: RawFd) -> io::Result<()> {
    let mut kind: libc::c_int = 0;
    let mut len = size_of_val(&kind) as libc::socklen_t;
    // SAFETY: fd remains owned by the caller and the option storage matches
    // the integer and length arguments required by getsockopt.
    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&mut kind as *mut libc::c_int).cast(),
            &mut len,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    if len as usize != size_of_val(&kind) || kind != libc::SOCK_STREAM {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(())
}

fn verify_null_stdio() -> io::Result<()> {
    let null = std::fs::metadata("/dev/null")?;
    for fd in 0..=2 {
        let mut stat = MaybeUninit::<libc::stat>::uninit();
        // SAFETY: fstat initializes the supplied stat record on success.
        if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fstat succeeded for this descriptor.
        let stat = unsafe { stat.assume_init() };
        if (stat.st_dev as u64, stat.st_ino) != (null.dev(), null.ino()) {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
    Ok(())
}
