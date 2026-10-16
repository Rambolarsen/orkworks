//! Dormant connected-stream proof; never authorizes native startup by itself.
//! This guard retains the exact stream and borrows its unreaped owned leader.
//! Future integration must separately fence executable/generation identity and
//! prove every native TUI connection; this helper supplies neither authority.
use super::{launch::OwnedProcess, NativeError};
use std::net::{SocketAddr, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
#[cfg(any(test, all(target_os = "macos", target_arch = "aarch64")))]
use std::time::Duration;
use std::time::Instant;
#[derive(Clone)]
struct InspectionBudget {
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}
impl InspectionBudget {
    fn new(deadline: Instant) -> Self {
        Self {
            deadline,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
    fn check(&self) -> Result<(), NativeError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(NativeError::Stale);
        }
        if Instant::now() >= self.deadline {
            return Err(NativeError::Timeout);
        }
        Ok(())
    }
}
struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

pub(super) struct ProvedStream<'a> {
    owner: &'a OwnedProcess,
    stream: TcpStream,
}
impl ProvedStream<'_> {
    /// Recheck the retained leader before accessing this same proved stream.
    pub(super) fn stream(&self) -> Result<&TcpStream, NativeError> {
        if !self.owner.is_alive() {
            return Err(NativeError::Unavailable);
        }
        Ok(&self.stream)
    }
}
/// Inspect without sending bytes. Cancellation/error drops the retained stream.
/// The deadline must also be bounded by the caller's shared startup budget.
pub(super) async fn prove(
    owner: &OwnedProcess,
    stream: TcpStream,
    deadline: Instant,
) -> Result<ProvedStream<'_>, NativeError> {
    let tuple = SocketTuple::reversed(&stream)?;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        let pid = owner
            .listener_process_id()
            .ok_or(NativeError::Unavailable)?;
        let deadline = deadline.min(Instant::now() + Duration::from_secs(3));
        if deadline <= Instant::now() {
            return Err(NativeError::Timeout);
        }
        let budget = InspectionBudget::new(deadline);
        let _cancel_on_drop = CancelOnDrop(budget.cancelled.clone());
        let worker_budget = budget.clone();
        let task = tokio::task::spawn_blocking(move || {
            inspect_with_budget(&MacKernel, pid, tuple, &worker_budget)
        });
        tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), task)
            .await
            .map_err(|_| NativeError::Timeout)?
            .map_err(|_| NativeError::Unavailable)??;
        budget.check()?;
        if owner.listener_process_id() != Some(pid) {
            return Err(NativeError::Unavailable);
        }
        Ok(ProvedStream { owner, stream })
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let _ = (owner, stream, deadline, tuple);
        Err(NativeError::Unavailable)
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Descriptor {
    fd: i32,
    kind: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SocketTuple {
    local_address: u32,
    peer_address: u32,
    local_port: u16,
    peer_port: u16,
}
impl SocketTuple {
    fn reversed(stream: &TcpStream) -> Result<Self, NativeError> {
        let (SocketAddr::V4(local), SocketAddr::V4(peer)) = (
            stream.local_addr().map_err(|_| NativeError::Unavailable)?,
            stream.peer_addr().map_err(|_| NativeError::Unavailable)?,
        ) else {
            return Err(NativeError::Unavailable);
        };
        if !local.ip().is_loopback()
            || !peer.ip().is_loopback()
            || local.port() == 0
            || peer.port() == 0
        {
            return Err(NativeError::Unavailable);
        }
        Ok(Self {
            local_address: u32::from(*peer.ip()),
            peer_address: u32::from(*local.ip()),
            local_port: peer.port(),
            peer_port: local.port(),
        })
    }
}
const MAX_INVENTORY_BYTES: usize = 1024 * 1024;
const MAX_INVENTORY_ATTEMPTS: usize = 3;
const SOCKET_KIND: u32 = 2;
trait Kernel {
    fn descriptors(&self, pid: i32, buffer: &mut [Descriptor]) -> Result<usize, NativeError>;
    fn socket(&self, pid: i32, fd: i32) -> Result<Option<SocketTuple>, NativeError>;
}
fn inspect_with_budget(
    kernel: &impl Kernel,
    pid: i32,
    tuple: SocketTuple,
    budget: &InspectionBudget,
) -> Result<(), NativeError> {
    budget.check()?;
    if pid <= 0 {
        return Err(NativeError::Unavailable);
    }
    let size = std::mem::size_of::<Descriptor>();
    for _ in 0..MAX_INVENTORY_ATTEMPTS {
        budget.check()?;
        let estimate = kernel.descriptors(pid, &mut [])?;
        if estimate == 0 || estimate > MAX_INVENTORY_BYTES || estimate % size != 0 {
            return Err(NativeError::Unavailable);
        }
        // Spare entries distinguish a complete inventory from a buffer filled
        // during concurrent descriptor growth. A full buffer is never trusted.
        let capacity = estimate
            .checked_add(8 * size)
            .ok_or(NativeError::Unavailable)?
            .min(MAX_INVENTORY_BYTES)
            / size;
        let mut rows = vec![Descriptor::default(); capacity];
        budget.check()?;
        let bytes = kernel.descriptors(pid, &mut rows)?;
        if bytes > capacity * size || bytes % size != 0 {
            return Err(NativeError::Unavailable);
        }
        if bytes == capacity * size {
            continue;
        }
        let mut matches = 0;
        for row in rows.iter().take(bytes / size) {
            budget.check()?;
            if row.fd < 0 {
                return Err(NativeError::Unavailable);
            }
            if row.kind == SOCKET_KIND && kernel.socket(pid, row.fd)? == Some(tuple) {
                matches += 1;
            }
        }
        budget.check()?;
        return if matches == 1 {
            Ok(())
        } else {
            Err(NativeError::Unavailable)
        };
    }
    Err(NativeError::Unavailable)
}
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
struct MacKernel;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
impl Kernel for MacKernel {
    fn descriptors(&self, pid: i32, buffer: &mut [Descriptor]) -> Result<usize, NativeError> {
        let bytes = std::mem::size_of_val(buffer);
        let length = i32::try_from(bytes).map_err(|_| NativeError::Unavailable)?;
        let ptr = if buffer.is_empty() {
            std::ptr::null_mut()
        } else {
            buffer.as_mut_ptr().cast()
        };
        // SAFETY: initialized, aligned repr(C) integer records with the supplied
        // byte capacity. The SDK adapter asserts proc_fdinfo's matching layout.
        let count = unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, ptr, length) };
        usize::try_from(count)
            .ok()
            .filter(|count| *count > 0)
            .ok_or(NativeError::Unavailable)
    }
    fn socket(&self, pid: i32, fd: i32) -> Result<Option<SocketTuple>, NativeError> {
        extern "C" {
            fn orkworks_native_socket(pid: i32, fd: i32, out: *mut SocketTuple) -> i32;
        }
        let mut tuple = SocketTuple::default();
        // SAFETY: the SDK-compiled adapter writes only this repr(C) tuple, whose
        // integer field layout matches its C declaration; no pointers escape.
        match unsafe { orkworks_native_socket(pid, fd, &mut tuple) } {
            1 => Ok(Some(tuple)),
            0 => Ok(None),
            _ => Err(NativeError::Unavailable),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    fn inspect(kernel: &impl Kernel, pid: i32, tuple: SocketTuple) -> Result<(), NativeError> {
        inspect_with_budget(
            kernel,
            pid,
            tuple,
            &InspectionBudget::new(Instant::now() + Duration::from_secs(3)),
        )
    }
    const TUPLE: SocketTuple = SocketTuple {
        local_address: 0x7f000001,
        peer_address: 0x7f000001,
        local_port: 10001,
        peer_port: 20002,
    };
    struct Fixture {
        rows: Vec<Option<SocketTuple>>,
        estimate: usize,
        returned: usize,
        denied: bool,
        socket_error: bool,
        full_reads: usize,
        cancel_on_socket: Option<Arc<AtomicBool>>,
        reads: Cell<usize>,
    }
    impl Fixture {
        fn new(rows: Vec<Option<SocketTuple>>) -> Self {
            Self {
                estimate: rows.len() * 8,
                returned: rows.len() * 8,
                rows,
                denied: false,
                socket_error: false,
                full_reads: 0,
                cancel_on_socket: None,
                reads: Cell::new(0),
            }
        }
    }
    impl Kernel for Fixture {
        fn descriptors(&self, pid: i32, buffer: &mut [Descriptor]) -> Result<usize, NativeError> {
            if self.denied || pid != 42 {
                return Err(NativeError::Unavailable);
            }
            if buffer.is_empty() {
                return Ok(self.estimate);
            }
            self.reads.set(self.reads.get() + 1);
            if self.reads.get() <= self.full_reads {
                return Ok(std::mem::size_of_val(buffer));
            }
            for (i, row) in buffer.iter_mut().enumerate().take(self.rows.len()) {
                *row = Descriptor {
                    fd: i as i32,
                    kind: SOCKET_KIND,
                };
            }
            Ok(self.returned)
        }
        fn socket(&self, _pid: i32, fd: i32) -> Result<Option<SocketTuple>, NativeError> {
            if self.socket_error {
                return Err(NativeError::Unavailable);
            }
            if let Some(cancelled) = &self.cancel_on_socket {
                cancelled.store(true, Ordering::Release);
            }
            Ok(self.rows[fd as usize])
        }
    }
    #[test]
    fn exact_established_tuple_is_required_and_ambiguity_is_rejected() {
        for (rows, accepted) in [
            (vec![Some(TUPLE)], true),
            (vec![], false),
            (vec![None], false),
            (
                vec![Some(SocketTuple {
                    peer_port: 20003,
                    ..TUPLE
                })],
                false,
            ),
            (vec![Some(TUPLE), Some(TUPLE)], false),
        ] {
            assert_eq!(inspect(&Fixture::new(rows), 42, TUPLE).is_ok(), accepted);
        }
    }
    #[test]
    fn expired_or_cancelled_inspection_never_returns_ownership() {
        let budget = InspectionBudget::new(Instant::now());
        assert!(inspect_with_budget(&Fixture::new(vec![Some(TUPLE)]), 42, TUPLE, &budget).is_err());
        let budget = InspectionBudget::new(Instant::now() + Duration::from_secs(3));
        budget.cancelled.store(true, Ordering::Release);
        assert!(inspect_with_budget(&Fixture::new(vec![Some(TUPLE)]), 42, TUPLE, &budget).is_err());
        let budget = InspectionBudget::new(Instant::now() + Duration::from_secs(3));
        let mut during = Fixture::new(vec![Some(TUPLE)]);
        during.cancel_on_socket = Some(budget.cancelled.clone());
        assert!(inspect_with_budget(&during, 42, TUPLE, &budget).is_err());
    }
    #[test]
    fn inventory_growth_retries_are_bounded_and_only_complete_results_qualify() {
        let mut grows = Fixture::new(vec![Some(TUPLE)]);
        grows.full_reads = 1;
        assert!(inspect(&grows, 42, TUPLE).is_ok());
        assert_eq!(grows.reads.get(), 2);
        let mut never_complete = Fixture::new(vec![Some(TUPLE)]);
        never_complete.full_reads = 4;
        assert!(inspect(&never_complete, 42, TUPLE).is_err());
        assert_eq!(never_complete.reads.get(), 3);
    }
    #[test]
    fn denied_malformed_truncated_and_oversized_inventory_fail_closed() {
        let mut cases = Vec::new();
        let mut denied = Fixture::new(vec![Some(TUPLE)]);
        denied.denied = true;
        cases.push(denied);
        let mut oversized = Fixture::new(vec![Some(TUPLE)]);
        oversized.estimate = MAX_INVENTORY_BYTES + 8;
        cases.push(oversized);
        let mut partial = Fixture::new(vec![Some(TUPLE)]);
        partial.returned = 7;
        cases.push(partial);
        let mut truncated = Fixture::new(vec![Some(TUPLE)]);
        truncated.returned = MAX_INVENTORY_BYTES;
        cases.push(truncated);
        let mut bad_socket = Fixture::new(vec![Some(TUPLE)]);
        bad_socket.socket_error = true;
        cases.push(bad_socket);
        for kernel in cases {
            assert!(inspect(&kernel, 42, TUPLE).is_err());
            assert!(kernel.reads.get() <= 3);
        }
        assert!(inspect(&Fixture::new(vec![Some(TUPLE)]), 43, TUPLE).is_err());
    }
}
