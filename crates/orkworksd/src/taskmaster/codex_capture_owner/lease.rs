//! Reusable four-slot private worker lease accounting, descriptor quota
//! ledger, and kernel-bound root transfer for macOS custody qualification.
//!
//! Dormant during normal sidecar startup. Wraps `CustodyPool` to enforce
//! four-slot actor accounting (`Booting`, `Idle`, `Bound`, `Busy`, `Draining`,
//! `CleanupUnconfirmed`, `Retired`), per-slot 16-descriptor reservations
//! (4 controller + up to 12 resource FDs, no cross-slot borrowing), the global
//! 256-descriptor ceiling, nonblocking acquisition without queues or eviction,
//! exact live-worker lease references, checked `u64` sequence exhaustion,
//! complete close-before-ACK validation, and fail-closed quarantine on stale
//! ACKs, timeouts, or protocol violations.

use std::ffi::CString;
use std::io::{self, ErrorKind};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::{Duration, Instant};

use super::framing::{
    self, BindStatus, ControlFrame, ProbeCommitment, RevalidateVerdict, RootIdentity,
    ROOT_ROLE_COUNT,
};
use super::pool::{CustodyPool, ShutdownReport, MAX_SLOTS};

pub(crate) const SLOT_DESCRIPTOR_RESERVATION: usize = 16;
pub(crate) const CONTROLLER_DESCRIPTORS_PER_SLOT: usize = 4;
pub(crate) const MAX_RESOURCE_DESCRIPTORS_PER_SLOT: usize =
    SLOT_DESCRIPTOR_RESERVATION - CONTROLLER_DESCRIPTORS_PER_SLOT;
pub(crate) const GLOBAL_DESCRIPTOR_CAP: usize = 256;

pub(crate) const DEFAULT_LEASE_TTL: Duration = Duration::from_secs(30);
pub(crate) const DEFAULT_OP_DEADLINE: Duration = Duration::from_secs(3);

/// Parent-retained root duplicates (3) + worker copies (3) + peak transient
/// traversal handle (1) during synthetic probe observation.
const SYNTHETIC_BIND_PEAK_RESOURCE_FDS: usize = 7;
/// Steady-state bound charge: parent-retained roots (3) + worker copies (3).
const SYNTHETIC_BOUND_RESOURCE_FDS: usize = 6;
/// Worker-held roots (3) retained in ledger if parent roots are closed while
/// worker cleanup remains unconfirmed.
const WORKER_HELD_ROOT_FDS: usize = ROOT_ROLE_COUNT;
const QUARANTINE_CLEANUP_WAIT: Duration = Duration::from_millis(250);
const RETIRE_SHUTDOWN_BUDGET: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeaseError {
    Unavailable,
    SlotDescriptorQuotaExceeded,
    GlobalDescriptorCapExceeded,
    RootOpenFailed,
    InvalidHandle,
    Expired,
    ObservationRejected,
    OperationTimedOut,
    StaleReleaseAck,
    SequenceExhausted,
    WorkerExited,
    ProtocolViolation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RevalidateOutcome {
    Fresh,
    Changed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PoolAccounting {
    pub booting: usize,
    pub idle: usize,
    pub bound: usize,
    pub busy: usize,
    pub draining: usize,
    pub cleanup_unconfirmed: usize,
    pub retired: usize,
    pub charged_slots: usize,
    pub controller_fds: usize,
    pub resource_fds: usize,
    pub total_fds: usize,
}

/// Tracks per-slot 16-descriptor reservations (4 controller + <= 12 resource,
/// never borrowing across slots) under the separate 256 global ceiling.
#[derive(Clone, Debug)]
pub(crate) struct DescriptorLedger {
    controller_charged: [bool; MAX_SLOTS],
    slot_resources: [usize; MAX_SLOTS],
    external_charge: usize,
}

impl DescriptorLedger {
    pub(crate) fn new() -> Self {
        Self {
            controller_charged: [false; MAX_SLOTS],
            slot_resources: [0; MAX_SLOTS],
            external_charge: 0,
        }
    }

    pub(crate) fn controller_fds(&self) -> usize {
        self.controller_charged.iter().filter(|&&c| c).count() * CONTROLLER_DESCRIPTORS_PER_SLOT
    }

    pub(crate) fn resource_fds(&self) -> usize {
        self.slot_resources.iter().sum()
    }

    pub(crate) fn total_fds(&self) -> usize {
        self.controller_fds() + self.resource_fds() + self.external_charge
    }

    pub(crate) fn charge_controller_slot(&mut self, slot: usize) -> Result<(), LeaseError> {
        if slot >= MAX_SLOTS {
            return Err(LeaseError::InvalidHandle);
        }
        if self.controller_charged[slot] {
            return Ok(());
        }
        if self
            .total_fds()
            .checked_add(CONTROLLER_DESCRIPTORS_PER_SLOT)
            .is_none_or(|n| n > GLOBAL_DESCRIPTOR_CAP)
        {
            return Err(LeaseError::GlobalDescriptorCapExceeded);
        }
        self.controller_charged[slot] = true;
        Ok(())
    }

    pub(crate) fn release_controller_slot(&mut self, slot: usize) {
        if slot < MAX_SLOTS {
            self.controller_charged[slot] = false;
        }
    }

    pub(crate) fn try_charge_slot_resources(
        &mut self,
        slot: usize,
        delta: usize,
    ) -> Result<(), LeaseError> {
        if slot >= MAX_SLOTS {
            return Err(LeaseError::InvalidHandle);
        }
        let next_slot = self.slot_resources[slot]
            .checked_add(delta)
            .ok_or(LeaseError::SlotDescriptorQuotaExceeded)?;
        if next_slot > MAX_RESOURCE_DESCRIPTORS_PER_SLOT {
            return Err(LeaseError::SlotDescriptorQuotaExceeded);
        }
        let next_total = self
            .total_fds()
            .checked_add(delta)
            .ok_or(LeaseError::GlobalDescriptorCapExceeded)?;
        if next_total > GLOBAL_DESCRIPTOR_CAP {
            return Err(LeaseError::GlobalDescriptorCapExceeded);
        }
        self.slot_resources[slot] = next_slot;
        Ok(())
    }

    pub(crate) fn set_slot_resources_down(&mut self, slot: usize, new_count: usize) {
        if slot < MAX_SLOTS && new_count <= self.slot_resources[slot] {
            self.slot_resources[slot] = new_count;
        }
    }

    pub(crate) fn release_slot_resources(&mut self, slot: usize) {
        if slot < MAX_SLOTS {
            self.slot_resources[slot] = 0;
        }
    }

    pub(crate) fn set_external_charge(&mut self, count: usize) -> Result<(), LeaseError> {
        let base = self.controller_fds() + self.resource_fds();
        if base
            .checked_add(count)
            .is_none_or(|n| n > GLOBAL_DESCRIPTOR_CAP)
        {
            return Err(LeaseError::GlobalDescriptorCapExceeded);
        }
        self.external_charge = count;
        Ok(())
    }
}

/// Parent-retained live root directory handles (`O_RDONLY | O_DIRECTORY | O_CLOEXEC`)
/// for the three fixed code-selected roles (`workspace`, `home`, `packaged`).
pub(crate) struct LeaseRoots {
    fds: [OwnedFd; ROOT_ROLE_COUNT],
    identities: [RootIdentity; ROOT_ROLE_COUNT],
}

impl LeaseRoots {
    pub(crate) fn open(workspace: &Path, home: &Path, packaged: &Path) -> io::Result<Self> {
        let (ws_fd, ws_id) = open_root_dir(workspace)?;
        let (home_fd, home_id) = open_root_dir(home)?;
        let (pkg_fd, pkg_id) = open_root_dir(packaged)?;
        Ok(Self {
            fds: [ws_fd, home_fd, pkg_fd],
            identities: [ws_id, home_id, pkg_id],
        })
    }

    pub(crate) fn identities(&self) -> [RootIdentity; ROOT_ROLE_COUNT] {
        self.identities
    }

    fn raw_fds(&self) -> [libc::c_int; ROOT_ROLE_COUNT] {
        [
            self.fds[0].as_raw_fd(),
            self.fds[1].as_raw_fd(),
            self.fds[2].as_raw_fd(),
        ]
    }
}

fn open_root_dir(path: &Path) -> io::Result<(OwnedFd, RootIdentity)> {
    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "interior NUL in root path"))?;
    // SAFETY: `c_path` is a valid NUL-terminated path; O_CLOEXEC seals the parent descriptor.
    let raw = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `raw` is newly opened with O_CLOEXEC and owned exclusively here.
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: fstat writes only to `stat` on success.
    if unsafe { libc::fstat(fd.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fstat succeeded.
    let stat = unsafe { stat.assume_init() };
    if (stat.st_mode & libc::S_IFMT) != libc::S_IFDIR {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "root path is not a directory",
        ));
    }
    Ok((
        fd,
        RootIdentity {
            dev: stat.st_dev as u64,
            ino: stat.st_ino,
        },
    ))
}

/// Exact live-worker lease reference bound to a specific pool instance, slot,
/// worker boot nonce, worker PID, lease UUID, checked positive sequence, and
/// original parent-authoritative expiry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LeaseHandle {
    pool_id: [u8; 16],
    slot: usize,
    worker_boot_nonce: [u8; 16],
    worker_pid: u32,
    lease_id: [u8; 16],
    lease_seq: u64,
    expires_at: Instant,
    root_identities: [RootIdentity; ROOT_ROLE_COUNT],
    commitments: [ProbeCommitment; ROOT_ROLE_COUNT],
}

impl LeaseHandle {
    pub(crate) fn slot(&self) -> usize {
        self.slot
    }

    pub(crate) fn worker_pid(&self) -> u32 {
        self.worker_pid
    }

    pub(crate) fn lease_seq(&self) -> u64 {
        self.lease_seq
    }

    pub(crate) fn expires_at(&self) -> Instant {
        self.expires_at
    }

    pub(crate) fn root_identities(&self) -> [RootIdentity; ROOT_ROLE_COUNT] {
        self.root_identities
    }

    pub(crate) fn commitments(&self) -> [ProbeCommitment; ROOT_ROLE_COUNT] {
        self.commitments
    }

    pub(crate) fn corrupt_lease_id_for_test(&mut self) {
        self.lease_id[0] ^= 0xFF;
    }

    pub(crate) fn corrupt_lease_seq_for_test(&mut self) {
        self.lease_seq = self.lease_seq.wrapping_add(1);
    }

    pub(crate) fn corrupt_boot_nonce_for_test(&mut self) {
        self.worker_boot_nonce[0] ^= 0xFF;
    }
}

struct BoundLease {
    worker_boot_nonce: [u8; 16],
    worker_pid: u32,
    lease_id: [u8; 16],
    lease_seq: u64,
    last_op_seq: u64,
    expires_at: Instant,
    /// Parent-retained live root directory descriptors owned for the exact
    /// lifetime of this lease and closed before releasing their ledger charge.
    _roots: LeaseRoots,
}

enum SlotLeaseState {
    Booting,
    Idle,
    Bound(BoundLease),
    Busy,
    Draining,
    CleanupUnconfirmed,
    Retired,
}

pub(crate) struct LeasePool {
    pool_id: [u8; 16],
    custody: CustodyPool,
    states: [SlotLeaseState; MAX_SLOTS],
    next_lease_seq: [Option<u64>; MAX_SLOTS],
    ledger: DescriptorLedger,
}

impl LeasePool {
    pub(crate) fn start_for_executable(
        executable: &Path,
        expected_build: [u8; 32],
        budget: Duration,
    ) -> Self {
        let custody = CustodyPool::start_for_executable(executable, expected_build, budget);
        Self::from_custody_pool(custody)
    }

    pub(crate) fn from_custody_pool(custody: CustodyPool) -> Self {
        let mut states = [
            SlotLeaseState::Retired,
            SlotLeaseState::Retired,
            SlotLeaseState::Retired,
            SlotLeaseState::Retired,
        ];
        let mut ledger = DescriptorLedger::new();
        for (slot, state) in states.iter_mut().enumerate() {
            if custody.slot_is_ready(slot) {
                let _ = ledger.charge_controller_slot(slot);
                *state = SlotLeaseState::Idle;
            } else if custody.slot_is_pending_spawn(slot) {
                let _ = ledger.charge_controller_slot(slot);
                *state = SlotLeaseState::Booting;
            } else if custody.slot_is_cleanup_unconfirmed(slot) {
                let _ = ledger.charge_controller_slot(slot);
                *state = SlotLeaseState::CleanupUnconfirmed;
            }
        }
        Self {
            pool_id: *uuid::Uuid::new_v4().as_bytes(),
            custody,
            states,
            next_lease_seq: [Some(1); MAX_SLOTS],
            ledger,
        }
    }

    pub(crate) fn slot_pid(&self, slot: usize) -> Option<u32> {
        self.custody.slot_identity(slot).map(|(pid, _)| pid)
    }

    pub(crate) fn accounting(&self) -> PoolAccounting {
        let mut acct = PoolAccounting {
            booting: 0,
            idle: 0,
            bound: 0,
            busy: 0,
            draining: 0,
            cleanup_unconfirmed: 0,
            retired: 0,
            charged_slots: 0,
            controller_fds: self.ledger.controller_fds(),
            resource_fds: self.ledger.resource_fds(),
            total_fds: self.ledger.total_fds(),
        };
        for state in &self.states {
            match state {
                SlotLeaseState::Booting => acct.booting += 1,
                SlotLeaseState::Idle => acct.idle += 1,
                SlotLeaseState::Bound(_) => acct.bound += 1,
                SlotLeaseState::Busy => acct.busy += 1,
                SlotLeaseState::Draining => acct.draining += 1,
                SlotLeaseState::CleanupUnconfirmed => acct.cleanup_unconfirmed += 1,
                SlotLeaseState::Retired => acct.retired += 1,
            }
        }
        acct.charged_slots = MAX_SLOTS - acct.retired;
        acct
    }

    pub(crate) fn is_handle_usable(&self, handle: &LeaseHandle) -> bool {
        if handle.pool_id != self.pool_id || handle.slot >= MAX_SLOTS {
            return false;
        }
        let SlotLeaseState::Bound(bound) = &self.states[handle.slot] else {
            return false;
        };
        bound.worker_boot_nonce == handle.worker_boot_nonce
            && bound.worker_pid == handle.worker_pid
            && bound.lease_id == handle.lease_id
            && bound.lease_seq == handle.lease_seq
            && Instant::now() < bound.expires_at
    }

    /// Nonblocking acquisition of an `Idle` worker slot. Reserves slot and global
    /// descriptor quota before opening the three parent root directories,
    /// transfers them via `SCM_RIGHTS`, binds ownership of the parent roots into
    /// the live lease, and reads pure commitment bytes back from the slot's OS `pipe()`.
    pub(crate) fn bind_and_observe(
        &mut self,
        workspace: &Path,
        home: &Path,
        packaged: &Path,
        ttl: Duration,
        op_deadline: Duration,
    ) -> Result<LeaseHandle, LeaseError> {
        let ttl_ms = u32::try_from(ttl.as_millis())
            .ok()
            .filter(|&ms| ms >= 1 && ms <= 30_000)
            .ok_or(LeaseError::ProtocolViolation)?;

        // Nonblocking scan for the first Idle slot with remaining sequence capacity.
        let mut chosen = None;
        for slot in 0..MAX_SLOTS {
            if matches!(self.states[slot], SlotLeaseState::Idle)
                && self.next_lease_seq[slot].is_some()
            {
                chosen = Some(slot);
                break;
            }
        }
        let Some(slot) = chosen else {
            return Err(LeaseError::Unavailable);
        };

        // Reserve peak synthetic resource descriptors under slot (<=12) and global (<=256)
        // caps BEFORE opening any parent root descriptors.
        self.ledger
            .try_charge_slot_resources(slot, SYNTHETIC_BIND_PEAK_RESOURCE_FDS)?;

        let Some((worker_pid, worker_boot_nonce)) = self.custody.slot_identity(slot) else {
            self.ledger.release_slot_resources(slot);
            self.states[slot] = SlotLeaseState::Retired;
            self.ledger.release_controller_slot(slot);
            return Err(LeaseError::Unavailable);
        };

        let roots = match LeaseRoots::open(workspace, home, packaged) {
            Ok(roots) => roots,
            Err(_) => {
                self.ledger.release_slot_resources(slot);
                return Err(LeaseError::RootOpenFailed);
            }
        };

        let lease_seq = self.next_lease_seq[slot].expect("checked non-None");
        self.next_lease_seq[slot] = lease_seq.checked_add(1);
        let lease_id = *uuid::Uuid::new_v4().as_bytes();
        let op_seq = 1u64;

        self.states[slot] = SlotLeaseState::Busy;
        let deadline = Instant::now() + op_deadline;
        let expires_at = Instant::now() + ttl;
        let raw_fds = roots.raw_fds();

        let rpc = self
            .custody
            .with_ready_worker(slot, |req_tx, res_rx, child| {
                if !matches!(child.try_wait(), Ok(None)) {
                    return Err(LeaseError::WorkerExited);
                }
                let frame = ControlFrame::BindAndObserve {
                    lease_id,
                    lease_seq,
                    op_seq,
                    ttl_ms,
                };
                framing::send_control_frame_with_fds(req_tx.as_fd(), &frame, &raw_fds, deadline)
                    .map_err(map_io_error)?;
                let reply =
                    framing::read_control_frame(res_rx.as_fd(), deadline).map_err(map_io_error)?;
                framing::ensure_no_trailing_bytes(res_rx.as_fd()).map_err(map_io_error)?;
                if !matches!(child.try_wait(), Ok(None)) {
                    return Err(LeaseError::WorkerExited);
                }
                Ok(reply)
            });

        let reply = match rpc {
            Some(Ok(frame)) => frame,
            Some(Err(err)) => {
                drop(roots);
                self.quarantine_and_record(slot);
                return Err(err);
            }
            None => {
                drop(roots);
                self.quarantine_and_record(slot);
                return Err(LeaseError::WorkerExited);
            }
        };

        match reply {
            ControlFrame::BindObserved {
                lease_id: ack_id,
                lease_seq: ack_lease_seq,
                op_seq: ack_op_seq,
                status,
                root_identities,
                probe_commitments,
            } if ack_id == lease_id && ack_lease_seq == lease_seq && ack_op_seq == op_seq => {
                if root_identities != roots.identities() {
                    drop(roots);
                    self.quarantine_and_record(slot);
                    return Err(LeaseError::ProtocolViolation);
                }
                match status {
                    BindStatus::Observed => {
                        self.ledger
                            .set_slot_resources_down(slot, SYNTHETIC_BOUND_RESOURCE_FDS);
                        self.states[slot] = SlotLeaseState::Bound(BoundLease {
                            worker_boot_nonce,
                            worker_pid,
                            lease_id,
                            lease_seq,
                            last_op_seq: op_seq,
                            expires_at,
                            _roots: roots,
                        });
                        Ok(LeaseHandle {
                            pool_id: self.pool_id,
                            slot,
                            worker_boot_nonce,
                            worker_pid,
                            lease_id,
                            lease_seq,
                            expires_at,
                            root_identities,
                            commitments: probe_commitments,
                        })
                    }
                    BindStatus::Rejected => {
                        // Close parent root descriptors before releasing their ledger charge.
                        drop(roots);
                        self.ledger.release_slot_resources(slot);
                        if self.next_lease_seq[slot].is_none() {
                            self.retire_exhausted_slot(slot);
                        } else {
                            self.states[slot] = SlotLeaseState::Idle;
                        }
                        Err(LeaseError::ObservationRejected)
                    }
                }
            }
            _ => {
                drop(roots);
                self.quarantine_and_record(slot);
                Err(LeaseError::ProtocolViolation)
            }
        }
    }

    pub(crate) fn revalidate(
        &mut self,
        handle: &LeaseHandle,
        op_deadline: Duration,
    ) -> Result<RevalidateOutcome, LeaseError> {
        let slot = self.validate_bound_handle(handle)?;

        // Check parent-authoritative expiry FIRST; invalidate usability before draining.
        let expired = match &self.states[slot] {
            SlotLeaseState::Bound(bound) => Instant::now() >= bound.expires_at,
            _ => return Err(LeaseError::InvalidHandle),
        };
        if expired {
            let bound = self.take_bound_into_draining(slot);
            let _ = self.execute_release(slot, bound, op_deadline);
            return Err(LeaseError::Expired);
        }

        // Check transient traversal FD headroom (+1 above steady-state 6 = 7 <= 12).
        self.ledger.try_charge_slot_resources(slot, 1)?;

        let mut bound = self.take_bound_into_busy(slot);
        let Some(op_seq) = bound.last_op_seq.checked_add(1) else {
            drop(bound);
            self.quarantine_and_record(slot);
            return Err(LeaseError::SequenceExhausted);
        };
        bound.last_op_seq = op_seq;

        let deadline = Instant::now() + op_deadline;
        let rpc = self
            .custody
            .with_ready_worker(slot, |req_tx, res_rx, child| {
                if !matches!(child.try_wait(), Ok(None)) {
                    return Err(LeaseError::WorkerExited);
                }
                let frame = ControlFrame::Revalidate {
                    lease_id: bound.lease_id,
                    lease_seq: bound.lease_seq,
                    op_seq,
                };
                framing::write_control_frame(req_tx.as_fd(), &frame, deadline)
                    .map_err(map_io_error)?;
                let reply =
                    framing::read_control_frame(res_rx.as_fd(), deadline).map_err(map_io_error)?;
                framing::ensure_no_trailing_bytes(res_rx.as_fd()).map_err(map_io_error)?;
                if !matches!(child.try_wait(), Ok(None)) {
                    return Err(LeaseError::WorkerExited);
                }
                Ok(reply)
            });

        let reply = match rpc {
            Some(Ok(frame)) => frame,
            Some(Err(err)) => {
                drop(bound);
                self.quarantine_and_record(slot);
                return Err(err);
            }
            None => {
                drop(bound);
                self.quarantine_and_record(slot);
                return Err(LeaseError::WorkerExited);
            }
        };

        self.ledger
            .set_slot_resources_down(slot, SYNTHETIC_BOUND_RESOURCE_FDS);

        match reply {
            ControlFrame::Revalidated {
                lease_id,
                lease_seq,
                op_seq: ack_op_seq,
                verdict,
            } if lease_id == bound.lease_id
                && lease_seq == bound.lease_seq
                && ack_op_seq == op_seq =>
            {
                match verdict {
                    RevalidateVerdict::Fresh => {
                        // Recheck parent-authoritative expiry AFTER receiving the response
                        // so a reply arriving past expiry never restores Bound or reports Fresh.
                        if Instant::now() >= bound.expires_at {
                            self.states[slot] = SlotLeaseState::Draining;
                            let remaining = deadline.saturating_duration_since(Instant::now());
                            let _ = self.execute_release(slot, bound, remaining);
                            return Err(LeaseError::Expired);
                        }
                        self.states[slot] = SlotLeaseState::Bound(bound);
                        Ok(RevalidateOutcome::Fresh)
                    }
                    RevalidateVerdict::Changed => {
                        // Invalidate preview usability first on change, then
                        // drain/release within the remaining operation budget.
                        self.states[slot] = SlotLeaseState::Draining;
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        let _ = self.execute_release(slot, bound, remaining);
                        Ok(RevalidateOutcome::Changed)
                    }
                    RevalidateVerdict::Expired => {
                        // Invalidate usability immediately, drain worker cleanly
                        // within the remaining operation budget.
                        self.states[slot] = SlotLeaseState::Draining;
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        let _ = self.execute_release(slot, bound, remaining);
                        Err(LeaseError::Expired)
                    }
                    RevalidateVerdict::Invalid => {
                        drop(bound);
                        self.quarantine_and_record(slot);
                        Err(LeaseError::ProtocolViolation)
                    }
                }
            }
            _ => {
                drop(bound);
                self.quarantine_and_record(slot);
                Err(LeaseError::ProtocolViolation)
            }
        }
    }

    pub(crate) fn release(
        &mut self,
        handle: LeaseHandle,
        op_deadline: Duration,
    ) -> Result<(), LeaseError> {
        let slot = self.validate_bound_handle(&handle)?;
        // Invalidate preview usability FIRST before sending Release.
        let bound = self.take_bound_into_draining(slot);
        self.execute_release(slot, bound, op_deadline)
    }

    pub(crate) fn cancel_and_drain(
        &mut self,
        handle: LeaseHandle,
        op_deadline: Duration,
    ) -> Result<(), LeaseError> {
        self.release(handle, op_deadline)
    }

    /// Invalidates all live leases first, then shuts down and drains the exact
    /// owned worker children, retaining charges only for unconfirmed slots.
    pub(crate) fn shutdown(&mut self, budget: Duration) -> ShutdownReport {
        for state in &mut self.states {
            if matches!(state, SlotLeaseState::Bound(_)) {
                *state = SlotLeaseState::Draining;
            }
        }
        let report = self.custody.shutdown(budget);
        self.sync_after_shutdown();
        report
    }

    pub(crate) fn reconcile_unconfirmed(&mut self) -> usize {
        let reconciled = self.custody.reconcile_unconfirmed();
        for slot in 0..MAX_SLOTS {
            match self.states[slot] {
                SlotLeaseState::Booting | SlotLeaseState::CleanupUnconfirmed => {
                    if self.custody.slot_is_pending_spawn(slot) {
                        self.states[slot] = SlotLeaseState::Booting;
                    } else if self.custody.slot_is_cleanup_unconfirmed(slot) {
                        self.states[slot] = SlotLeaseState::CleanupUnconfirmed;
                    } else {
                        self.states[slot] = SlotLeaseState::Retired;
                        self.ledger.release_slot_resources(slot);
                        self.ledger.release_controller_slot(slot);
                    }
                }
                _ => {}
            }
        }
        reconciled
    }

    pub(crate) fn quarantine_slot_for_test(&mut self, slot: usize, cleanup_wait: Duration) {
        if matches!(self.states.get(slot), Some(SlotLeaseState::Bound(_))) {
            // Drop parent-retained roots immediately; unconfirmed worker retains its 3 roots.
            self.states[slot] = SlotLeaseState::Draining;
        }
        if self.custody.quarantine_slot(slot, cleanup_wait) {
            self.states[slot] = SlotLeaseState::Retired;
            self.ledger.release_slot_resources(slot);
            self.ledger.release_controller_slot(slot);
        } else {
            self.ledger
                .set_slot_resources_down(slot, WORKER_HELD_ROOT_FDS);
            self.states[slot] = SlotLeaseState::CleanupUnconfirmed;
        }
    }

    pub(crate) fn set_slot_next_lease_seq_for_test(&mut self, slot: usize, next: Option<u64>) {
        self.next_lease_seq[slot] = next;
    }

    pub(crate) fn set_slot_last_op_seq_for_test(&mut self, slot: usize, last_op_seq: u64) {
        if let SlotLeaseState::Bound(bound) = &mut self.states[slot] {
            bound.last_op_seq = last_op_seq;
        }
    }

    pub(crate) fn extend_parent_expiry_for_test(&mut self, slot: usize, extra: Duration) {
        if let SlotLeaseState::Bound(bound) = &mut self.states[slot] {
            bound.expires_at += extra;
        }
    }

    pub(crate) fn set_external_fd_charge_for_test(
        &mut self,
        count: usize,
    ) -> Result<(), LeaseError> {
        self.ledger.set_external_charge(count)
    }

    fn validate_bound_handle(&self, handle: &LeaseHandle) -> Result<usize, LeaseError> {
        if handle.pool_id != self.pool_id || handle.slot >= MAX_SLOTS {
            return Err(LeaseError::InvalidHandle);
        }
        let SlotLeaseState::Bound(bound) = &self.states[handle.slot] else {
            return Err(LeaseError::InvalidHandle);
        };
        if bound.worker_boot_nonce != handle.worker_boot_nonce
            || bound.worker_pid != handle.worker_pid
            || bound.lease_id != handle.lease_id
            || bound.lease_seq != handle.lease_seq
        {
            return Err(LeaseError::InvalidHandle);
        }
        Ok(handle.slot)
    }

    fn take_bound_into_busy(&mut self, slot: usize) -> BoundLease {
        match std::mem::replace(&mut self.states[slot], SlotLeaseState::Busy) {
            SlotLeaseState::Bound(bound) => bound,
            _ => unreachable!("caller verified Bound state"),
        }
    }

    fn take_bound_into_draining(&mut self, slot: usize) -> BoundLease {
        match std::mem::replace(&mut self.states[slot], SlotLeaseState::Draining) {
            SlotLeaseState::Bound(bound) => bound,
            _ => unreachable!("caller verified Bound state"),
        }
    }

    fn execute_release(
        &mut self,
        slot: usize,
        mut bound: BoundLease,
        op_deadline: Duration,
    ) -> Result<(), LeaseError> {
        let Some(op_seq) = bound.last_op_seq.checked_add(1) else {
            drop(bound);
            self.quarantine_and_record(slot);
            return Err(LeaseError::SequenceExhausted);
        };
        bound.last_op_seq = op_seq;

        let deadline = Instant::now() + op_deadline;
        let rpc = self
            .custody
            .with_ready_worker(slot, |req_tx, res_rx, child| {
                if !matches!(child.try_wait(), Ok(None)) {
                    return Err(LeaseError::WorkerExited);
                }
                let frame = ControlFrame::Release {
                    lease_id: bound.lease_id,
                    lease_seq: bound.lease_seq,
                    op_seq,
                };
                framing::write_control_frame(req_tx.as_fd(), &frame, deadline)
                    .map_err(map_io_error)?;
                let reply =
                    framing::read_control_frame(res_rx.as_fd(), deadline).map_err(map_io_error)?;
                framing::ensure_no_trailing_bytes(res_rx.as_fd()).map_err(map_io_error)?;
                if !matches!(child.try_wait(), Ok(None)) {
                    return Err(LeaseError::WorkerExited);
                }
                Ok(reply)
            });

        let reply = match rpc {
            Some(Ok(frame)) => frame,
            Some(Err(err)) => {
                drop(bound);
                self.quarantine_and_record(slot);
                return Err(err);
            }
            None => {
                drop(bound);
                self.quarantine_and_record(slot);
                return Err(LeaseError::WorkerExited);
            }
        };

        match reply {
            ControlFrame::ReleaseAck {
                lease_id,
                lease_seq,
                op_seq: ack_op_seq,
            } if lease_id == bound.lease_id
                && lease_seq == bound.lease_seq
                && ack_op_seq == op_seq =>
            {
                // Close parent-retained root descriptors BEFORE clearing their ledger charge.
                drop(bound);
                self.ledger.release_slot_resources(slot);
                if self.next_lease_seq[slot].is_none() {
                    self.retire_exhausted_slot(slot);
                } else {
                    self.states[slot] = SlotLeaseState::Idle;
                }
                Ok(())
            }
            ControlFrame::ReleaseAck { .. } => {
                drop(bound);
                self.quarantine_and_record(slot);
                Err(LeaseError::StaleReleaseAck)
            }
            _ => {
                drop(bound);
                self.quarantine_and_record(slot);
                Err(LeaseError::ProtocolViolation)
            }
        }
    }

    fn retire_exhausted_slot(&mut self, slot: usize) {
        if self
            .custody
            .retire_ready_slot_gracefully(slot, RETIRE_SHUTDOWN_BUDGET)
        {
            self.states[slot] = SlotLeaseState::Retired;
            self.ledger.release_slot_resources(slot);
            self.ledger.release_controller_slot(slot);
        } else {
            self.states[slot] = SlotLeaseState::CleanupUnconfirmed;
        }
    }

    fn quarantine_and_record(&mut self, slot: usize) {
        if self.custody.quarantine_slot(slot, QUARANTINE_CLEANUP_WAIT) {
            self.states[slot] = SlotLeaseState::Retired;
            self.ledger.release_slot_resources(slot);
            self.ledger.release_controller_slot(slot);
        } else {
            // Parent root descriptors have been closed; retain controller (4)
            // and unconfirmed worker root charges (3) until confirmed reap.
            self.ledger
                .set_slot_resources_down(slot, WORKER_HELD_ROOT_FDS);
            self.states[slot] = SlotLeaseState::CleanupUnconfirmed;
        }
    }

    fn sync_after_shutdown(&mut self) {
        for slot in 0..MAX_SLOTS {
            if self.custody.slot_is_pending_spawn(slot) {
                self.states[slot] = SlotLeaseState::Booting;
            } else if self.custody.slot_is_cleanup_unconfirmed(slot) {
                self.ledger
                    .set_slot_resources_down(slot, WORKER_HELD_ROOT_FDS);
                self.states[slot] = SlotLeaseState::CleanupUnconfirmed;
            } else {
                self.states[slot] = SlotLeaseState::Retired;
                self.ledger.release_slot_resources(slot);
                self.ledger.release_controller_slot(slot);
            }
        }
    }
}

fn map_io_error(err: io::Error) -> LeaseError {
    match err.kind() {
        ErrorKind::TimedOut => LeaseError::OperationTimedOut,
        ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe => LeaseError::WorkerExited,
        _ => LeaseError::ProtocolViolation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_rejects_external_overflow_and_preserves_unconfirmed_charges() {
        let mut ledger = DescriptorLedger::new();
        for slot in 0..MAX_SLOTS {
            ledger.charge_controller_slot(slot).unwrap();
        }
        // 16 controller FDs charged; 241 external FDs would exceed 256.
        assert_eq!(
            ledger.set_external_charge(241),
            Err(LeaseError::GlobalDescriptorCapExceeded)
        );
        ledger.set_external_charge(240).unwrap();
        assert_eq!(ledger.total_fds(), 256);
        assert_eq!(
            ledger.try_charge_slot_resources(0, 1),
            Err(LeaseError::GlobalDescriptorCapExceeded)
        );
    }
}
