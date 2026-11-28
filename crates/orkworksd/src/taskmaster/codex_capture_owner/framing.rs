//! Persistent non-EOF byte and unidirectional `SCM_RIGHTS` framing for the
//! macOS custody channels.
//!
//! Requests traverse the unidirectional `AF_UNIX` request channel (`parent ->
//! worker`, where `BindAndObserve` transfers root directory descriptors via
//! `SCM_RIGHTS`), while responses traverse the unidirectional OS `pipe()`
//! response channel (`worker -> parent`, where `SCM_RIGHTS` is kernel-impossible).

use std::io::{self, ErrorKind};
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::time::{Duration, Instant};

use super::bootstrap_frame::FRAME_SIZE;
use super::channels::set_cloexec;

const MAGIC: [u8; 4] = *b"OWCO";
const PROTOCOL_VERSION: u8 = 1;
const HEADER_LEN: usize = 10;
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
const MAX_RECV_CMSG_FDS: usize = 8;

const KIND_PING: u8 = 1;
const KIND_PONG: u8 = 2;
const KIND_SHUTDOWN: u8 = 3;
const KIND_SHUTDOWN_ACK: u8 = 4;
const KIND_BIND_AND_OBSERVE: u8 = 10;
const KIND_BIND_OBSERVED: u8 = 11;
const KIND_REVALIDATE: u8 = 12;
const KIND_REVALIDATED: u8 = 13;
const KIND_RELEASE: u8 = 14;
const KIND_RELEASE_ACK: u8 = 15;

pub(crate) const ROOT_ROLE_COUNT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RootIdentity {
    pub dev: u64,
    pub ino: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProbeCommitment {
    pub present: bool,
    pub byte_len: u32,
    pub sha256: [u8; 32],
}

impl ProbeCommitment {
    pub(crate) const fn absent() -> Self {
        Self {
            present: false,
            byte_len: 0,
            sha256: [0u8; 32],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindStatus {
    Observed = 0,
    Rejected = 1,
}

impl BindStatus {
    fn from_u8(v: u8) -> io::Result<Self> {
        match v {
            0 => Ok(Self::Observed),
            1 => Ok(Self::Rejected),
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                "invalid bind status",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RevalidateVerdict {
    Fresh = 0,
    Changed = 1,
    Expired = 2,
    Invalid = 3,
}

impl RevalidateVerdict {
    fn from_u8(v: u8) -> io::Result<Self> {
        match v {
            0 => Ok(Self::Fresh),
            1 => Ok(Self::Changed),
            2 => Ok(Self::Expired),
            3 => Ok(Self::Invalid),
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                "invalid revalidate verdict",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ControlFrame {
    Ping([u8; 16]),
    Pong([u8; 16]),
    Shutdown,
    ShutdownAck,
    BindAndObserve {
        lease_id: [u8; 16],
        lease_seq: u64,
        op_seq: u64,
        ttl_ms: u32,
    },
    BindObserved {
        lease_id: [u8; 16],
        lease_seq: u64,
        op_seq: u64,
        status: BindStatus,
        root_identities: [RootIdentity; ROOT_ROLE_COUNT],
        probe_commitments: [ProbeCommitment; ROOT_ROLE_COUNT],
    },
    Revalidate {
        lease_id: [u8; 16],
        lease_seq: u64,
        op_seq: u64,
    },
    Revalidated {
        lease_id: [u8; 16],
        lease_seq: u64,
        op_seq: u64,
        verdict: RevalidateVerdict,
    },
    Release {
        lease_id: [u8; 16],
        lease_seq: u64,
        op_seq: u64,
    },
    ReleaseAck {
        lease_id: [u8; 16],
        lease_seq: u64,
        op_seq: u64,
    },
}

pub(crate) fn encode_control_frame(frame: &ControlFrame) -> Vec<u8> {
    let mut payload = Vec::new();
    let kind = match frame {
        ControlFrame::Ping(nonce) => {
            payload.extend_from_slice(nonce);
            KIND_PING
        }
        ControlFrame::Pong(nonce) => {
            payload.extend_from_slice(nonce);
            KIND_PONG
        }
        ControlFrame::Shutdown => KIND_SHUTDOWN,
        ControlFrame::ShutdownAck => KIND_SHUTDOWN_ACK,
        ControlFrame::BindAndObserve {
            lease_id,
            lease_seq,
            op_seq,
            ttl_ms,
        } => {
            payload.extend_from_slice(lease_id);
            payload.extend_from_slice(&lease_seq.to_be_bytes());
            payload.extend_from_slice(&op_seq.to_be_bytes());
            payload.extend_from_slice(&ttl_ms.to_be_bytes());
            KIND_BIND_AND_OBSERVE
        }
        ControlFrame::BindObserved {
            lease_id,
            lease_seq,
            op_seq,
            status,
            root_identities,
            probe_commitments,
        } => {
            payload.extend_from_slice(lease_id);
            payload.extend_from_slice(&lease_seq.to_be_bytes());
            payload.extend_from_slice(&op_seq.to_be_bytes());
            payload.push(*status as u8);
            for root in root_identities {
                payload.extend_from_slice(&root.dev.to_be_bytes());
                payload.extend_from_slice(&root.ino.to_be_bytes());
            }
            for commitment in probe_commitments {
                payload.push(u8::from(commitment.present));
                payload.extend_from_slice(&commitment.byte_len.to_be_bytes());
                payload.extend_from_slice(&commitment.sha256);
            }
            KIND_BIND_OBSERVED
        }
        ControlFrame::Revalidate {
            lease_id,
            lease_seq,
            op_seq,
        } => {
            payload.extend_from_slice(lease_id);
            payload.extend_from_slice(&lease_seq.to_be_bytes());
            payload.extend_from_slice(&op_seq.to_be_bytes());
            KIND_REVALIDATE
        }
        ControlFrame::Revalidated {
            lease_id,
            lease_seq,
            op_seq,
            verdict,
        } => {
            payload.extend_from_slice(lease_id);
            payload.extend_from_slice(&lease_seq.to_be_bytes());
            payload.extend_from_slice(&op_seq.to_be_bytes());
            payload.push(*verdict as u8);
            KIND_REVALIDATED
        }
        ControlFrame::Release {
            lease_id,
            lease_seq,
            op_seq,
        } => {
            payload.extend_from_slice(lease_id);
            payload.extend_from_slice(&lease_seq.to_be_bytes());
            payload.extend_from_slice(&op_seq.to_be_bytes());
            KIND_RELEASE
        }
        ControlFrame::ReleaseAck {
            lease_id,
            lease_seq,
            op_seq,
        } => {
            payload.extend_from_slice(lease_id);
            payload.extend_from_slice(&lease_seq.to_be_bytes());
            payload.extend_from_slice(&op_seq.to_be_bytes());
            KIND_RELEASE_ACK
        }
    };

    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(&MAGIC);
    out.push(PROTOCOL_VERSION);
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(&payload);
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

/// Sends a control frame over an `AF_UNIX` `SOCK_STREAM` request channel with
/// optional `SCM_RIGHTS` descriptors attached to the first chunk.
pub(crate) fn send_control_frame_with_fds(
    fd: BorrowedFd<'_>,
    frame: &ControlFrame,
    fds: &[RawFd],
    deadline: Instant,
) -> io::Result<()> {
    let encoded = encode_control_frame(frame);
    if fds.is_empty() {
        return write_all_deadline(fd, &encoded, deadline);
    }
    if fds.len() > MAX_RECV_CMSG_FDS {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "descriptor count exceeds control message cap",
        ));
    }
    let raw = fd.as_raw_fd();
    set_nonblocking(raw)?;

    let fds_bytes = std::mem::size_of_val(fds);
    let fds_bytes_u32 = libc::c_uint::try_from(fds_bytes)
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "descriptor list too large"))?;
    // SAFETY: CMSG_SPACE computes the aligned ancillary buffer size for `fds`.
    let cmsg_space = unsafe { libc::CMSG_SPACE(fds_bytes_u32) } as usize;
    let cmsg_words = cmsg_space.div_ceil(std::mem::size_of::<u64>());
    let mut cmsg_buf = vec![0u64; cmsg_words];

    let mut iov = libc::iovec {
        iov_base: encoded.as_ptr() as *mut libc::c_void,
        iov_len: encoded.len(),
    };
    // SAFETY: zero-initialized msghdr populated before sendmsg.
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr().cast();
    msg.msg_controllen = libc::socklen_t::try_from(cmsg_space)
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "cmsg length overflow"))?;

    // SAFETY: msg_control has `cmsg_space` aligned writable bytes; CMSG_FIRSTHDR returns valid header.
    unsafe {
        let cmsg = libc::CMSG_FIRSTHDR(&msg);
        if cmsg.is_null() {
            return Err(io::Error::other("CMSG_FIRSTHDR returned null"));
        }
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(fds_bytes_u32);
        std::ptr::copy_nonoverlapping(fds.as_ptr().cast::<u8>(), libc::CMSG_DATA(cmsg), fds_bytes);
    }

    let first_written = loop {
        wait_io(raw, libc::POLLOUT, Some(deadline))?;
        // SAFETY: `raw` is an open AF_UNIX stream and `msg` points to live buffers.
        let sent = unsafe { libc::sendmsg(raw, &msg, 0) };
        if sent == 0 {
            return Err(ErrorKind::WriteZero.into());
        }
        if sent < 0 {
            let err = io::Error::last_os_error();
            if matches!(err.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) {
                continue;
            }
            return Err(err);
        }
        break sent as usize;
    };

    if first_written < encoded.len() {
        write_all_deadline(fd, &encoded[first_written..], deadline)?;
    } else {
        remaining(deadline)?;
    }
    Ok(())
}

/// Reads the next pure-byte response control frame from the parent's `pipe()`.
pub(crate) fn read_control_frame(
    fd: BorrowedFd<'_>,
    deadline: Instant,
) -> io::Result<ControlFrame> {
    match read_control_frame_or_eof(fd, Some(deadline))? {
        Some(frame) => Ok(frame),
        None => Err(ErrorKind::UnexpectedEof.into()),
    }
}

/// Reads the next persistent control frame via pure `read()`, returning
/// `Ok(None)` only on clean EOF at a frame boundary (zero bytes read).
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
    let (kind, len) = parse_header(&header)?;
    let mut payload = vec![0u8; len];
    if !payload.is_empty() {
        read_exact_deadline(fd, &mut payload, deadline)?;
    }
    decode_control_frame(kind, &payload).map(Some)
}

/// Worker-side `AF_UNIX` request reader using `recvmsg` on the first chunk so
/// any `SCM_RIGHTS` descriptors attached to `BindAndObserve` are captured
/// atomically, immediately marked `FD_CLOEXEC`, and owned for automatic close
/// on any error. Returns `Ok(None)` only on clean EOF at a frame boundary.
pub(crate) fn recv_request_frame_or_eof(
    fd: BorrowedFd<'_>,
    deadline: Option<Instant>,
) -> io::Result<Option<(ControlFrame, Vec<OwnedFd>)>> {
    set_nonblocking(fd.as_raw_fd())?;
    let mut header = [0u8; HEADER_LEN];
    let (first, received_fds) = recvmsg_chunk(fd, &mut header, deadline, true)?;
    if first == 0 {
        if !received_fds.is_empty() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "unexpected ancillary descriptors on EOF",
            ));
        }
        return Ok(None);
    }
    if first < HEADER_LEN {
        recvmsg_exact_no_fds(fd, &mut header[first..], deadline)?;
    }
    let (kind, len) = parse_header(&header)?;
    let mut payload = vec![0u8; len];
    if !payload.is_empty() {
        recvmsg_exact_no_fds(fd, &mut payload, deadline)?;
    }
    let frame = decode_control_frame(kind, &payload)?;
    Ok(Some((frame, received_fds)))
}

fn parse_header(header: &[u8; HEADER_LEN]) -> io::Result<(u8, usize)> {
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
    Ok((kind, len))
}

fn decode_control_frame(kind: u8, payload: &[u8]) -> io::Result<ControlFrame> {
    match kind {
        KIND_PING | KIND_PONG => {
            if payload.len() != 16 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid ping/pong payload length",
                ));
            }
            let nonce: [u8; 16] = payload.try_into().expect("checked 16 bytes");
            Ok(if kind == KIND_PING {
                ControlFrame::Ping(nonce)
            } else {
                ControlFrame::Pong(nonce)
            })
        }
        KIND_SHUTDOWN | KIND_SHUTDOWN_ACK => {
            if !payload.is_empty() {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "shutdown frame must carry empty payload",
                ));
            }
            Ok(if kind == KIND_SHUTDOWN {
                ControlFrame::Shutdown
            } else {
                ControlFrame::ShutdownAck
            })
        }
        KIND_BIND_AND_OBSERVE => {
            if payload.len() != 36 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid BindAndObserve payload length",
                ));
            }
            let (lease_id, lease_seq, op_seq) = decode_lease_triplet(&payload[0..32]);
            let ttl_ms = u32::from_be_bytes(payload[32..36].try_into().expect("4 bytes"));
            Ok(ControlFrame::BindAndObserve {
                lease_id,
                lease_seq,
                op_seq,
                ttl_ms,
            })
        }
        KIND_BIND_OBSERVED => {
            const EXPECTED_LEN: usize = 32 + 1 + ROOT_ROLE_COUNT * 16 + ROOT_ROLE_COUNT * 37;
            if payload.len() != EXPECTED_LEN {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid BindObserved payload length",
                ));
            }
            let (lease_id, lease_seq, op_seq) = decode_lease_triplet(&payload[0..32]);
            let status = BindStatus::from_u8(payload[32])?;
            let mut offset = 33;
            let mut root_identities = [RootIdentity { dev: 0, ino: 0 }; ROOT_ROLE_COUNT];
            for slot in &mut root_identities {
                let dev =
                    u64::from_be_bytes(payload[offset..offset + 8].try_into().expect("8 bytes"));
                let ino = u64::from_be_bytes(
                    payload[offset + 8..offset + 16]
                        .try_into()
                        .expect("8 bytes"),
                );
                *slot = RootIdentity { dev, ino };
                offset += 16;
            }
            let mut probe_commitments = [ProbeCommitment::absent(); ROOT_ROLE_COUNT];
            for slot in &mut probe_commitments {
                let present = match payload[offset] {
                    0 => false,
                    1 => true,
                    _ => {
                        return Err(io::Error::new(
                            ErrorKind::InvalidData,
                            "invalid probe presence flag",
                        ));
                    }
                };
                let byte_len = u32::from_be_bytes(
                    payload[offset + 1..offset + 5].try_into().expect("4 bytes"),
                );
                let sha256: [u8; 32] = payload[offset + 5..offset + 37]
                    .try_into()
                    .expect("32 bytes");
                *slot = ProbeCommitment {
                    present,
                    byte_len,
                    sha256,
                };
                offset += 37;
            }
            Ok(ControlFrame::BindObserved {
                lease_id,
                lease_seq,
                op_seq,
                status,
                root_identities,
                probe_commitments,
            })
        }
        KIND_REVALIDATE => {
            if payload.len() != 32 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid Revalidate payload length",
                ));
            }
            let (lease_id, lease_seq, op_seq) = decode_lease_triplet(payload);
            Ok(ControlFrame::Revalidate {
                lease_id,
                lease_seq,
                op_seq,
            })
        }
        KIND_REVALIDATED => {
            if payload.len() != 33 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid Revalidated payload length",
                ));
            }
            let (lease_id, lease_seq, op_seq) = decode_lease_triplet(&payload[0..32]);
            let verdict = RevalidateVerdict::from_u8(payload[32])?;
            Ok(ControlFrame::Revalidated {
                lease_id,
                lease_seq,
                op_seq,
                verdict,
            })
        }
        KIND_RELEASE => {
            if payload.len() != 32 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid Release payload length",
                ));
            }
            let (lease_id, lease_seq, op_seq) = decode_lease_triplet(payload);
            Ok(ControlFrame::Release {
                lease_id,
                lease_seq,
                op_seq,
            })
        }
        KIND_RELEASE_ACK => {
            if payload.len() != 32 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid ReleaseAck payload length",
                ));
            }
            let (lease_id, lease_seq, op_seq) = decode_lease_triplet(payload);
            Ok(ControlFrame::ReleaseAck {
                lease_id,
                lease_seq,
                op_seq,
            })
        }
        _ => Err(io::Error::new(
            ErrorKind::InvalidData,
            "unknown custody control frame kind",
        )),
    }
}

fn decode_lease_triplet(slice: &[u8]) -> ([u8; 16], u64, u64) {
    let lease_id: [u8; 16] = slice[0..16].try_into().expect("16 bytes");
    let lease_seq = u64::from_be_bytes(slice[16..24].try_into().expect("8 bytes"));
    let op_seq = u64::from_be_bytes(slice[24..32].try_into().expect("8 bytes"));
    (lease_id, lease_seq, op_seq)
}

fn recvmsg_exact_no_fds(
    fd: BorrowedFd<'_>,
    buf: &mut [u8],
    deadline: Option<Instant>,
) -> io::Result<()> {
    let mut offset = 0;
    while offset < buf.len() {
        let (count, fds) = recvmsg_chunk(fd, &mut buf[offset..], deadline, false)?;
        if !fds.is_empty() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "unexpected ancillary descriptors in continuation chunk",
            ));
        }
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

fn recvmsg_chunk(
    fd: BorrowedFd<'_>,
    buf: &mut [u8],
    deadline: Option<Instant>,
    allow_fds: bool,
) -> io::Result<(usize, Vec<OwnedFd>)> {
    let raw = fd.as_raw_fd();
    let max_bytes = (MAX_RECV_CMSG_FDS * std::mem::size_of::<RawFd>()) as libc::c_uint;
    // SAFETY: CMSG_SPACE computes aligned control buffer size for up to MAX_RECV_CMSG_FDS descriptors.
    let cmsg_space = unsafe { libc::CMSG_SPACE(max_bytes) } as usize;
    let cmsg_words = cmsg_space.div_ceil(std::mem::size_of::<u64>());
    let mut cmsg_buf = vec![0u64; cmsg_words];

    loop {
        wait_io(raw, libc::POLLIN, deadline)?;
        cmsg_buf.fill(0);
        let mut iov = libc::iovec {
            iov_base: buf.as_mut_ptr().cast(),
            iov_len: buf.len(),
        };
        // SAFETY: zero-initialized msghdr populated before recvmsg.
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = cmsg_buf.as_mut_ptr().cast();
        msg.msg_controllen = cmsg_space as libc::socklen_t;

        // SAFETY: `raw` is open and `msg` points to live writable iov/cmsg buffers.
        let count = unsafe { libc::recvmsg(raw, &mut msg, 0) };
        if count < 0 {
            let err = io::Error::last_os_error();
            if matches!(err.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock) {
                continue;
            }
            return Err(err);
        }

        // Immediately wrap EVERY genuinely delivered descriptor (> 2) within
        // the bounded control buffer into OwnedFd so any error path drops and
        // closes them with zero leaks and never claims stdio (0..=2) or reads
        // past msg_controllen.
        let mut owned_fds = Vec::new();
        let mut unexpected_cmsg = false;
        let mut malformed_cmsg = false;
        let control_base = msg.msg_control as usize;
        let returned_control_len = (msg.msg_controllen as usize).min(cmsg_space);
        let control_end = control_base.saturating_add(returned_control_len);
        // SAFETY: walking CMSG headers returned by kernel bounded to `[control_base, control_end]`.
        unsafe {
            let hdr_len = libc::CMSG_LEN(0) as usize;
            let mut cmsg = libc::CMSG_FIRSTHDR(&msg);
            while !cmsg.is_null() {
                let cmsg_addr = cmsg as usize;
                if cmsg_addr < control_base || cmsg_addr.saturating_add(hdr_len) > control_end {
                    malformed_cmsg = true;
                    break;
                }
                let total_len = (*cmsg).cmsg_len as usize;
                if total_len < hdr_len {
                    malformed_cmsg = true;
                    break;
                }
                let declared_end = cmsg_addr.saturating_add(total_len);
                let safe_end = declared_end.min(control_end);
                if declared_end > control_end {
                    malformed_cmsg = true;
                }
                if (*cmsg).cmsg_level == libc::SOL_SOCKET && (*cmsg).cmsg_type == libc::SCM_RIGHTS {
                    let data_ptr = libc::CMSG_DATA(cmsg);
                    let data_addr = data_ptr as usize;
                    if data_addr > safe_end {
                        malformed_cmsg = true;
                        break;
                    }
                    let data_len = safe_end - data_addr;
                    if data_len % std::mem::size_of::<RawFd>() != 0 {
                        malformed_cmsg = true;
                    }
                    let fd_count = data_len / std::mem::size_of::<RawFd>();
                    for idx in 0..fd_count {
                        let mut raw_fd: RawFd = -1;
                        std::ptr::copy_nonoverlapping(
                            data_ptr.add(idx * std::mem::size_of::<RawFd>()),
                            (&mut raw_fd as *mut RawFd).cast::<u8>(),
                            std::mem::size_of::<RawFd>(),
                        );
                        if raw_fd > 2 {
                            owned_fds.push(OwnedFd::from_raw_fd(raw_fd));
                        } else {
                            malformed_cmsg = true;
                        }
                    }
                } else {
                    unexpected_cmsg = true;
                }
                if declared_end > control_end {
                    break;
                }
                cmsg = libc::CMSG_NXTHDR(&msg, cmsg);
            }
        }

        for owned in &owned_fds {
            set_cloexec(owned.as_raw_fd())?;
        }
        if (msg.msg_flags & libc::MSG_CTRUNC) != 0 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "ancillary control message was truncated",
            ));
        }
        if unexpected_cmsg || malformed_cmsg {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "invalid or unexpected ancillary control message",
            ));
        }
        if !allow_fds && !owned_fds.is_empty() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "unexpected ancillary descriptors on request frame",
            ));
        }
        return Ok((count as usize, owned_fds));
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
