//! Startup-sealed four-slot persistent custody worker pool for macOS qualification.
//!
//! Dormant during normal sidecar startup. Allocates and seals all four split
//! unidirectional channel pairs (AF_UNIX request + OS pipe response) during
//! exclusive startup before any controlled child launches, spawns workers
//! serially via `posix_spawn` with `POSIX_SPAWN_CLOEXEC_DEFAULT` under one
//! shared three-second admission budget, permanently retires failed slots
//! without replacement, and retains explicit child ownership for cleanup.

use std::ffi::CString;
use std::io::{self, ErrorKind};
use std::os::fd::{AsFd, AsRawFd, OwnedFd, RawFd};
use std::os::unix::{ffi::OsStrExt, net::UnixStream, process::ExitStatusExt};
use std::path::Path;
use std::process::ExitStatus;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use super::bootstrap_frame::{self, Direction};
use super::channels::{self, SlotChannels};
use super::framing::{self, ControlFrame};
use super::worker::WORKER_ARG;

pub(crate) const MAX_SLOTS: usize = 4;
pub(crate) const STARTUP_BUDGET: Duration = Duration::from_secs(3);
const DEFAULT_CLEANUP_WAIT: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PoolCounts {
    pub ready: usize,
    pub retired: usize,
    pub cleanup_unconfirmed: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShutdownReport {
    pub clean_exits: usize,
    pub forced_kills: usize,
    pub cleanup_unconfirmed: usize,
}

#[derive(Clone, Copy)]
enum ChildState {
    Owned,
    Reaped(ExitStatus),
    Lost,
}

/// Owned child handle spawned via `posix_spawn`. Confirmed exits remain
/// cached. A lost identity permanently stops operations on the stored PID.
pub(crate) struct OwnedChild {
    pid: libc::pid_t,
    state: ChildState,
    #[cfg(test)]
    wait_allowed: Option<Arc<std::sync::atomic::AtomicBool>>,
}

impl OwnedChild {
    pub(crate) fn try_from_pid(pid: libc::pid_t) -> io::Result<Self> {
        if pid <= 0 {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "owned child pid must be positive",
            ));
        }
        Ok(Self {
            pid,
            state: ChildState::Owned,
            #[cfg(test)]
            wait_allowed: None,
        })
    }

    pub(crate) fn id(&self) -> u32 {
        self.pid as u32
    }

    pub(crate) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        match self.state {
            ChildState::Owned => {}
            ChildState::Reaped(status) => return Ok(Some(status)),
            ChildState::Lost => return Err(io::Error::from_raw_os_error(libc::ECHILD)),
        }
        #[cfg(test)]
        if self
            .wait_allowed
            .as_ref()
            .is_some_and(|allowed| !allowed.load(std::sync::atomic::Ordering::Acquire))
        {
            return Ok(None);
        }
        let mut status: libc::c_int = 0;
        loop {
            // SAFETY: `status` is valid writable storage and `self.pid > 0` is our unreaped child.
            let ret = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) };
            if ret == 0 {
                return Ok(None);
            }
            if ret == self.pid {
                let exit = ExitStatus::from_raw(status);
                self.state = ChildState::Reaped(exit);
                return Ok(Some(exit));
            }
            let err = io::Error::last_os_error();
            if err.kind() == ErrorKind::Interrupted {
                continue;
            }
            self.record_wait_error(&err);
            return Err(err);
        }
    }

    pub(crate) fn kill(&mut self) -> io::Result<()> {
        match self.state {
            ChildState::Owned => {}
            ChildState::Reaped(_) => return Ok(()),
            ChildState::Lost => return Err(io::Error::from_raw_os_error(libc::ECHILD)),
        }
        // SAFETY: `self.pid > 0` is our owned, unreaped child process.
        if unsafe { libc::kill(self.pid, libc::SIGKILL) } < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                return Ok(());
            }
            return Err(err);
        }
        Ok(())
    }

    pub(crate) fn wait(&mut self) -> io::Result<ExitStatus> {
        match self.state {
            ChildState::Owned => {}
            ChildState::Reaped(status) => return Ok(status),
            ChildState::Lost => return Err(io::Error::from_raw_os_error(libc::ECHILD)),
        }
        let mut status: libc::c_int = 0;
        loop {
            // SAFETY: `status` is valid writable storage and `self.pid > 0` is our unreaped child.
            let ret = unsafe { libc::waitpid(self.pid, &mut status, 0) };
            if ret == self.pid {
                let exit = ExitStatus::from_raw(status);
                self.state = ChildState::Reaped(exit);
                return Ok(exit);
            }
            let err = io::Error::last_os_error();
            if err.kind() == ErrorKind::Interrupted {
                continue;
            }
            self.record_wait_error(&err);
            return Err(err);
        }
    }
    fn record_wait_error(&mut self, error: &io::Error) {
        if error.raw_os_error() == Some(libc::ECHILD) {
            self.state = ChildState::Lost;
        }
    }

    fn identity_lost(&self) -> bool {
        matches!(self.state, ChildState::Lost)
    }
}

struct ReadyWorker {
    child: OwnedChild,
    request_tx: UnixStream,
    response_rx: OwnedFd,
    pid: u32,
    boot_nonce: [u8; 16],
}

struct SpawnedWorker {
    request_tx: UnixStream,
    response_rx: OwnedFd,
    child: OwnedChild,
}

type PendingSpawn = mpsc::Receiver<io::Result<SpawnedWorker>>;

enum SlotState {
    Ready(ReadyWorker),
    Retired,
    CleanupUnconfirmed(OwnedChild),
    PendingSpawn(PendingSpawn),
    LaunchUnconfirmed,
}

type UnreapedChildren = [Option<OwnedChild>; MAX_SLOTS];

pub(crate) struct CustodyPool {
    slots: [SlotState; MAX_SLOTS],
    drop_cleanup: Option<mpsc::SyncSender<UnreapedChildren>>,
}

impl CustodyPool {
    pub(crate) fn start_for_executable(
        executable: &Path,
        expected_build: [u8; 32],
        budget: Duration,
    ) -> Self {
        let executable = executable.to_owned();
        Self::start_with_slot_hook(
            expected_build,
            budget,
            DEFAULT_CLEANUP_WAIT,
            move |_slot, request_rx_fd, response_tx_fd| {
                spawn_worker_process(&executable, request_rx_fd, response_tx_fd)
            },
        )
    }

    /// A successful hook hands off its child. An error must leave no live child.
    pub(crate) fn start_with_slot_hook(
        expected_build: [u8; 32],
        budget: Duration,
        cleanup_wait: Duration,
        spawn_slot: impl Fn(usize, RawFd, RawFd) -> io::Result<OwnedChild> + Send + Sync + 'static,
    ) -> Self {
        let mut slots = [
            SlotState::Retired,
            SlotState::Retired,
            SlotState::Retired,
            SlotState::Retired,
        ];

        if budget.is_zero() {
            return Self {
                slots,
                drop_cleanup: None,
            };
        }
        let deadline = Instant::now() + budget;

        let Ok(channel_set) = channels::seal_split_channels() else {
            return Self {
                slots,
                drop_cleanup: None,
            };
        };
        // If channel sealing itself missed the shared deadline, retire the
        // entire bootstrap outcome and start no workers.
        if Instant::now() >= deadline {
            drop(channel_set);
            return Self {
                slots,
                drop_cleanup: None,
            };
        }

        // Reserve a cleanup owner before any native child can start.
        let Ok(drop_cleanup) = start_drop_cleanup() else {
            return Self {
                slots,
                drop_cleanup: None,
            };
        };
        let spawn_slot = Arc::new(spawn_slot);
        for (slot, channels) in channel_set.into_iter().enumerate() {
            if Instant::now() >= deadline {
                break;
            }

            let spawn_slot = Arc::clone(&spawn_slot);
            let Ok(pending) = supervise_spawn(channels, deadline, move |request, response| {
                spawn_slot(slot, request, response)
            }) else {
                continue;
            };
            let spawned =
                match pending.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(Ok(spawned)) => spawned,
                    Ok(Err(_)) => continue,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        slots[slot] = SlotState::PendingSpawn(pending);
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        slots[slot] = SlotState::LaunchUnconfirmed;
                        break;
                    }
                };
            let SpawnedWorker {
                request_tx,
                response_rx,
                child,
            } = spawned;
            if Instant::now() >= deadline {
                drop((request_tx, response_rx));
                slots[slot] = terminate_bounded(child, Duration::ZERO);
                break;
            }

            let nonce = *uuid::Uuid::new_v4().as_bytes();
            match handshake_worker(
                request_tx,
                response_rx,
                child,
                &nonce,
                &expected_build,
                deadline,
            ) {
                Ok(ready) => {
                    slots[slot] = SlotState::Ready(ready);
                }
                Err(child) => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    slots[slot] = terminate_bounded(child, cleanup_wait.min(remaining));
                }
            }
        }

        Self {
            slots,
            drop_cleanup: Some(drop_cleanup),
        }
    }

    pub(crate) fn counts(&self) -> PoolCounts {
        let mut counts = PoolCounts {
            ready: 0,
            retired: 0,
            cleanup_unconfirmed: 0,
        };
        for slot in &self.slots {
            match slot {
                SlotState::Ready(_) => counts.ready += 1,
                SlotState::Retired => counts.retired += 1,
                SlotState::CleanupUnconfirmed(_)
                | SlotState::PendingSpawn(_)
                | SlotState::LaunchUnconfirmed => {
                    counts.cleanup_unconfirmed += 1;
                }
            }
        }
        counts
    }

    pub(crate) fn slot_is_ready(&self, slot: usize) -> bool {
        matches!(self.slots.get(slot), Some(SlotState::Ready(_)))
    }

    pub(crate) fn slot_is_pending_spawn(&self, slot: usize) -> bool {
        matches!(self.slots.get(slot), Some(SlotState::PendingSpawn(_)))
    }

    pub(crate) fn slot_is_cleanup_unconfirmed(&self, slot: usize) -> bool {
        matches!(
            self.slots.get(slot),
            Some(SlotState::CleanupUnconfirmed(_) | SlotState::LaunchUnconfirmed)
        )
    }

    pub(crate) fn slot_identity(&self, slot: usize) -> Option<(u32, [u8; 16])> {
        match self.slots.get(slot) {
            Some(SlotState::Ready(worker)) => Some((worker.pid, worker.boot_nonce)),
            _ => None,
        }
    }

    pub(crate) fn with_ready_worker<R>(
        &mut self,
        slot: usize,
        f: impl FnOnce(&mut UnixStream, &mut OwnedFd, &mut OwnedChild) -> R,
    ) -> Option<R> {
        let SlotState::Ready(worker) = self.slots.get_mut(slot)? else {
            return None;
        };
        Some(f(
            &mut worker.request_tx,
            &mut worker.response_rx,
            &mut worker.child,
        ))
    }

    /// Gracefully shuts down a single ready slot (used on checked counter
    /// exhaustion) and returns `true` if reaped (`Retired`), or `false` if
    /// cleanup remains unconfirmed. Non-`Ready` states are preserved intact.
    pub(crate) fn retire_ready_slot_gracefully(&mut self, slot: usize, budget: Duration) -> bool {
        let Some(entry) = self.slots.get_mut(slot) else {
            return true;
        };
        let mut worker = match std::mem::replace(entry, SlotState::Retired) {
            SlotState::Ready(worker) => worker,
            other => {
                *entry = other;
                return matches!(entry, SlotState::Retired);
            }
        };
        let deadline = Instant::now() + budget;
        let sent = framing::write_control_frame(
            worker.request_tx.as_fd(),
            &ControlFrame::Shutdown,
            deadline,
        )
        .is_ok();
        let acked = sent
            && framing::read_control_frame(worker.response_rx.as_fd(), deadline)
                .is_ok_and(|frame| frame == ControlFrame::ShutdownAck);
        drop(worker.request_tx);
        drop(worker.response_rx);
        if acked && wait_for_clean_exit(&mut worker.child, deadline) {
            return true;
        }
        *entry = terminate_bounded(worker.child, DEFAULT_CLEANUP_WAIT);
        matches!(entry, SlotState::Retired)
    }

    /// Immediately quarantines and terminates a ready slot, returning `true` if
    /// confirmed reaped (`Retired`), or `false` if `CleanupUnconfirmed`.
    pub(crate) fn quarantine_slot(&mut self, slot: usize, cleanup_wait: Duration) -> bool {
        let Some(entry) = self.slots.get_mut(slot) else {
            return true;
        };
        match std::mem::replace(entry, SlotState::Retired) {
            SlotState::Ready(worker) => {
                drop(worker.request_tx);
                drop(worker.response_rx);
                *entry = terminate_bounded(worker.child, cleanup_wait);
                matches!(entry, SlotState::Retired)
            }
            SlotState::CleanupUnconfirmed(child) => {
                *entry = terminate_bounded(child, cleanup_wait);
                matches!(entry, SlotState::Retired)
            }
            other => {
                *entry = other;
                matches!(entry, SlotState::Retired)
            }
        }
    }

    /// Nonblockingly polls `CleanupUnconfirmed` children and late `PendingSpawn`
    /// handoffs, transitioning any confirmed exits to `Retired`.
    pub(crate) fn reconcile_unconfirmed(&mut self) -> usize {
        let mut reconciled = 0;
        for slot in &mut self.slots {
            match std::mem::replace(slot, SlotState::Retired) {
                SlotState::CleanupUnconfirmed(mut child) => {
                    if matches!(child.try_wait(), Ok(Some(_))) {
                        reconciled += 1;
                    } else {
                        *slot = SlotState::CleanupUnconfirmed(child);
                    }
                }
                SlotState::PendingSpawn(pending) => match pending.try_recv() {
                    Ok(Ok(spawned)) => {
                        drop((spawned.request_tx, spawned.response_rx));
                        *slot = terminate_bounded(spawned.child, Duration::ZERO);
                        if matches!(slot, SlotState::Retired) {
                            reconciled += 1;
                        }
                    }
                    Ok(Err(_)) => {
                        reconciled += 1;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        *slot = SlotState::PendingSpawn(pending);
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        *slot = SlotState::LaunchUnconfirmed;
                    }
                },
                other => {
                    *slot = other;
                }
            }
        }
        reconciled
    }

    pub(crate) fn ready_pids(&self) -> Vec<u32> {
        self.slots
            .iter()
            .filter_map(|slot| match slot {
                SlotState::Ready(worker) => Some(worker.pid),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn ping_slot(&mut self, slot: usize, deadline: Instant) -> io::Result<()> {
        let Some(entry) = self.slots.get_mut(slot) else {
            return Err(ErrorKind::InvalidInput.into());
        };
        let SlotState::Ready(worker) = entry else {
            return Err(ErrorKind::NotConnected.into());
        };
        let nonce = *uuid::Uuid::new_v4().as_bytes();
        let outcome = (|| -> io::Result<()> {
            framing::write_control_frame(
                worker.request_tx.as_fd(),
                &ControlFrame::Ping(nonce),
                deadline,
            )?;
            match framing::read_control_frame(worker.response_rx.as_fd(), deadline)? {
                ControlFrame::Pong(reply) if reply == nonce => {}
                _ => {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        "mismatched pong frame",
                    ));
                }
            }
            framing::ensure_no_trailing_bytes(worker.response_rx.as_fd())?;
            if worker.child.try_wait()?.is_some() {
                return Err(io::Error::new(
                    ErrorKind::BrokenPipe,
                    "custody worker exited unexpectedly",
                ));
            }
            Ok(())
        })();

        if let Err(err) = outcome {
            let owned = std::mem::replace(entry, SlotState::Retired);
            if let SlotState::Ready(worker) = owned {
                drop(worker.request_tx);
                drop(worker.response_rx);
                *entry = terminate_bounded(worker.child, DEFAULT_CLEANUP_WAIT);
            }
            return Err(err);
        }
        Ok(())
    }

    pub(crate) fn shutdown(&mut self, budget: Duration) -> ShutdownReport {
        let deadline = Instant::now() + budget;
        let mut report = ShutdownReport {
            clean_exits: 0,
            forced_kills: 0,
            cleanup_unconfirmed: 0,
        };

        // First, snapshot pre-existing CleanupUnconfirmed children separately
        // and send Shutdown to every Ready worker concurrently so a single
        // stalled slot cannot starve healthy siblings or be visited twice.
        let mut pending_graceful: [Option<(ReadyWorker, bool)>; MAX_SLOTS] =
            [None, None, None, None];
        let mut pending_unconfirmed: [Option<OwnedChild>; MAX_SLOTS] = [None, None, None, None];
        let mut ready_remaining = 0usize;
        for (idx, slot) in self.slots.iter_mut().enumerate() {
            match std::mem::replace(slot, SlotState::Retired) {
                SlotState::Retired => {}
                SlotState::Ready(worker) => {
                    let write_deadline = deadline.min(Instant::now() + Duration::from_millis(100));
                    let sent = framing::write_control_frame(
                        worker.request_tx.as_fd(),
                        &ControlFrame::Shutdown,
                        write_deadline,
                    )
                    .is_ok();
                    pending_graceful[idx] = Some((worker, sent));
                    ready_remaining += 1;
                }
                SlotState::CleanupUnconfirmed(child) => {
                    pending_unconfirmed[idx] = Some(child);
                }
                SlotState::PendingSpawn(pending) => match pending.try_recv() {
                    Ok(Ok(spawned)) => pending_unconfirmed[idx] = Some(spawned.child),
                    Ok(Err(_)) => {}
                    Err(mpsc::TryRecvError::Empty) => {
                        *slot = SlotState::PendingSpawn(pending);
                        report.cleanup_unconfirmed += 1;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        *slot = SlotState::LaunchUnconfirmed;
                        report.cleanup_unconfirmed += 1;
                    }
                },
                SlotState::LaunchUnconfirmed => {
                    *slot = SlotState::LaunchUnconfirmed;
                    report.cleanup_unconfirmed += 1;
                }
            }
        }

        // Wait for each worker's ShutdownAck and clean exit with a fair slice
        // of the remaining graceful budget, then give any straggler a bounded
        // DEFAULT_CLEANUP_WAIT SIGKILL reap window so one stuck child cannot
        // starve siblings or leave them unconfirmed.
        for (idx, item) in pending_graceful.into_iter().enumerate() {
            let Some((mut worker, sent)) = item else {
                continue;
            };
            let per_slot_budget = deadline
                .saturating_duration_since(Instant::now())
                .checked_div(ready_remaining.max(1) as u32)
                .unwrap_or(Duration::ZERO);
            let slot_deadline = Instant::now() + per_slot_budget;
            ready_remaining = ready_remaining.saturating_sub(1);

            let acked = sent
                && framing::read_control_frame(worker.response_rx.as_fd(), slot_deadline)
                    .is_ok_and(|frame| frame == ControlFrame::ShutdownAck);
            drop(worker.request_tx);
            drop(worker.response_rx);

            if acked && wait_for_clean_exit(&mut worker.child, slot_deadline) {
                report.clean_exits += 1;
            } else {
                match terminate_bounded(worker.child, DEFAULT_CLEANUP_WAIT) {
                    SlotState::Retired => report.forced_kills += 1,
                    SlotState::CleanupUnconfirmed(child) => {
                        report.cleanup_unconfirmed += 1;
                        self.slots[idx] = SlotState::CleanupUnconfirmed(child);
                    }
                    _ => unreachable!(),
                }
            }
        }

        // Reconcile only pre-existing CleanupUnconfirmed slots; newly
        // unconfirmed slots from the Ready loop are never visited twice.
        for (idx, item) in pending_unconfirmed.into_iter().enumerate() {
            let Some(child) = item else {
                continue;
            };
            match terminate_bounded(child, DEFAULT_CLEANUP_WAIT) {
                SlotState::Retired => {}
                SlotState::CleanupUnconfirmed(child) => {
                    report.cleanup_unconfirmed += 1;
                    self.slots[idx] = SlotState::CleanupUnconfirmed(child);
                }
                _ => unreachable!(),
            }
        }
        report
    }
}

impl Drop for CustodyPool {
    fn drop(&mut self) {
        let unresolved = std::array::from_fn(|index| {
            let child = match std::mem::replace(&mut self.slots[index], SlotState::Retired) {
                SlotState::Ready(worker) => {
                    drop((worker.request_tx, worker.response_rx));
                    worker.child
                }
                SlotState::CleanupUnconfirmed(child) => child,
                // Dropping a pending receiver lets its launch task own
                // cleanup of any child that subsequently arrives.
                SlotState::Retired | SlotState::PendingSpawn(_) | SlotState::LaunchUnconfirmed => {
                    return None;
                }
            };
            match terminate_bounded(child, DEFAULT_CLEANUP_WAIT) {
                SlotState::CleanupUnconfirmed(child) => Some(child),
                _ => None,
            }
        });
        if unresolved.iter().all(Option::is_none) {
            return;
        }
        let unresolved = match self.drop_cleanup.take() {
            Some(cleanup) => match cleanup.try_send(unresolved) {
                Ok(()) => return,
                Err(
                    mpsc::TrySendError::Full(children) | mpsc::TrySendError::Disconnected(children),
                ) => children,
            },
            None => unresolved,
        };
        // A live pool has a prestarted receiver for its single batch. If that
        // invariant is broken, keep ownership even at the cost of blocking.
        reap_unresolved(unresolved);
    }
}

fn start_drop_cleanup() -> io::Result<mpsc::SyncSender<UnreapedChildren>> {
    let (cleanup, unresolved) = mpsc::sync_channel(1);
    thread::Builder::new().spawn(move || {
        if let Ok(children) = unresolved.recv() {
            reap_unresolved(children);
        }
    })?;
    Ok(cleanup)
}

fn reap_unresolved(mut children: UnreapedChildren) {
    while children.iter().any(Option::is_some) {
        for slot in &mut children {
            let Some(child) = slot else { continue };
            match child.try_wait() {
                Ok(Some(_)) => *slot = None,
                Ok(None) => {
                    let _ = child.kill();
                }
                // An errored wait has not confirmed this pid is still ours.
                Err(_) => {}
            }
        }
        if children
            .iter()
            .flatten()
            .any(|child| !child.identity_lost())
        {
            thread::sleep(Duration::from_millis(25));
        } else if children.iter().any(Option::is_some) {
            // Keep unresolved identities until sidecar exit without polling
            // or signaling any of their former PIDs.
            loop {
                thread::park();
            }
        }
    }
}

fn supervise_spawn(
    channels: SlotChannels,
    deadline: Instant,
    spawn: impl FnOnce(RawFd, RawFd) -> io::Result<OwnedChild> + Send + 'static,
) -> io::Result<PendingSpawn> {
    // No buffered result can be abandoned on receiver drop. A failed
    // rendezvous leaves the exact process owned by this background task.
    let (complete, pending) = mpsc::sync_channel(0);
    thread::Builder::new().spawn(move || {
        if Instant::now() >= deadline {
            let _ = complete.send(Err(ErrorKind::TimedOut.into()));
            return;
        }
        let SlotChannels {
            request_tx,
            request_rx,
            response_rx,
            response_tx,
        } = channels;
        let child = spawn(request_rx.as_raw_fd(), response_tx.as_raw_fd());
        drop((request_rx, response_tx));
        let result = child.map(|child| SpawnedWorker {
            request_tx,
            response_rx,
            child,
        });
        if let Err(mpsc::SendError(Ok(spawned))) = complete.send(result) {
            // The pool is gone. Close the channels and reap off the caller's
            // thread, even if the operating system takes longer to settle.
            let SpawnedWorker {
                request_tx,
                response_rx,
                mut child,
            } = spawned;
            drop((request_tx, response_rx));
            let _ = child.kill();
            let _ = child.wait();
        }
    })?;
    Ok(pending)
}

struct SpawnFileActions(libc::posix_spawn_file_actions_t);

impl SpawnFileActions {
    fn new() -> io::Result<Self> {
        let mut raw = std::mem::MaybeUninit::<libc::posix_spawn_file_actions_t>::uninit();
        // SAFETY: `raw` is writable storage initialized by posix_spawn_file_actions_init.
        let rc = unsafe { libc::posix_spawn_file_actions_init(raw.as_mut_ptr()) };
        if rc != 0 {
            return Err(io::Error::from_raw_os_error(rc));
        }
        // SAFETY: posix_spawn_file_actions_init returned 0.
        Ok(Self(unsafe { raw.assume_init() }))
    }

    fn add_open(&mut self, fd: RawFd, path: &CString, oflag: libc::c_int) -> io::Result<()> {
        // SAFETY: `self.0` is initialized and `path` is a valid NUL-terminated C string.
        let rc = unsafe {
            libc::posix_spawn_file_actions_addopen(&mut self.0, fd, path.as_ptr(), oflag, 0)
        };
        if rc != 0 {
            return Err(io::Error::from_raw_os_error(rc));
        }
        Ok(())
    }

    fn add_inherit(&mut self, fd: RawFd) -> io::Result<()> {
        // macOS SDK `spawn.h` (libSystem.B.dylib); not re-exported by libc 0.2.
        extern "C" {
            fn posix_spawn_file_actions_addinherit_np(
                file_actions: *mut libc::posix_spawn_file_actions_t,
                filedes: libc::c_int,
            ) -> libc::c_int;
        }
        // SAFETY: `self.0` is initialized and macOS `posix_spawn_file_actions_addinherit_np`
        // preserves `fd` across exec under `POSIX_SPAWN_CLOEXEC_DEFAULT`.
        let rc = unsafe { posix_spawn_file_actions_addinherit_np(&mut self.0, fd) };
        if rc != 0 {
            return Err(io::Error::from_raw_os_error(rc));
        }
        Ok(())
    }
}

impl Drop for SpawnFileActions {
    fn drop(&mut self) {
        // SAFETY: `self.0` was initialized in `new()` and is destroyed once on Drop.
        unsafe {
            libc::posix_spawn_file_actions_destroy(&mut self.0);
        }
    }
}

struct SpawnAttr(libc::posix_spawnattr_t);

impl SpawnAttr {
    fn new_cloexec_default() -> io::Result<Self> {
        let mut raw = std::mem::MaybeUninit::<libc::posix_spawnattr_t>::uninit();
        // SAFETY: `raw` is writable storage initialized by posix_spawnattr_init.
        let rc = unsafe { libc::posix_spawnattr_init(raw.as_mut_ptr()) };
        if rc != 0 {
            return Err(io::Error::from_raw_os_error(rc));
        }
        // SAFETY: posix_spawnattr_init returned 0.
        let mut attr = Self(unsafe { raw.assume_init() });
        // SAFETY: `attr.0` is initialized; POSIX_SPAWN_CLOEXEC_DEFAULT is a valid macOS flag.
        let rc = unsafe {
            libc::posix_spawnattr_setflags(&mut attr.0, libc::POSIX_SPAWN_CLOEXEC_DEFAULT as i16)
        };
        if rc != 0 {
            return Err(io::Error::from_raw_os_error(rc));
        }
        Ok(attr)
    }
}

impl Drop for SpawnAttr {
    fn drop(&mut self) {
        // SAFETY: `self.0` was initialized and is destroyed once on Drop.
        unsafe {
            libc::posix_spawnattr_destroy(&mut self.0);
        }
    }
}

/// Spawns one custody worker using macOS `posix_spawn` with
/// `POSIX_SPAWN_CLOEXEC_DEFAULT` so only `/dev/null` stdio (`0..=2`),
/// `request_rx_fd`, and `response_tx_fd` survive into the child, leaving all
/// parent descriptor flags untouched and closing any ambient host FDs.
pub(crate) fn spawn_worker_process(
    executable: &Path,
    request_rx_fd: RawFd,
    response_tx_fd: RawFd,
) -> io::Result<OwnedChild> {
    if request_rx_fd <= 2 || response_tx_fd <= 2 || request_rx_fd == response_tx_fd {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "worker endpoints must be distinct descriptors > 2",
        ));
    }
    let exe = CString::new(executable.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "executable path contains NUL"))?;
    let flag = CString::new(WORKER_ARG).expect("static worker arg");
    let req_arg = CString::new(request_rx_fd.to_string()).expect("numeric fd");
    let resp_arg = CString::new(response_tx_fd.to_string()).expect("numeric fd");
    let dev_null = CString::new("/dev/null").expect("static path");

    let mut actions = SpawnFileActions::new()?;
    actions.add_open(0, &dev_null, libc::O_RDONLY)?;
    actions.add_open(1, &dev_null, libc::O_WRONLY)?;
    actions.add_open(2, &dev_null, libc::O_WRONLY)?;
    actions.add_inherit(request_rx_fd)?;
    actions.add_inherit(response_tx_fd)?;

    let attr = SpawnAttr::new_cloexec_default()?;
    let argv: [*mut libc::c_char; 5] = [
        exe.as_ptr().cast_mut(),
        flag.as_ptr().cast_mut(),
        req_arg.as_ptr().cast_mut(),
        resp_arg.as_ptr().cast_mut(),
        std::ptr::null_mut(),
    ];
    let envp: [*mut libc::c_char; 1] = [std::ptr::null_mut()];
    let mut pid: libc::pid_t = 0;

    // SAFETY: all CStrings, argv, envp, actions, and attr remain alive for the
    // duration of `posix_spawn`; caller retains both inherited FDs across spawn.
    let rc = unsafe {
        libc::posix_spawn(
            &mut pid,
            exe.as_ptr(),
            &actions.0,
            &attr.0,
            argv.as_ptr(),
            envp.as_ptr(),
        )
    };
    if rc != 0 {
        return Err(io::Error::from_raw_os_error(rc));
    }
    OwnedChild::try_from_pid(pid)
}

fn handshake_worker(
    request_tx: UnixStream,
    response_rx: OwnedFd,
    mut child: OwnedChild,
    nonce: &[u8; 16],
    expected_build: &[u8; 32],
    deadline: Instant,
) -> Result<ReadyWorker, OwnedChild> {
    let challenge = bootstrap_frame::encode(Direction::Challenge, nonce, expected_build);
    if framing::write_bootstrap_frame(request_tx.as_fd(), &challenge, deadline).is_err() {
        return Err(child);
    }
    let Ok(ready) = framing::read_bootstrap_frame(response_rx.as_fd(), deadline) else {
        return Err(child);
    };
    if !bootstrap_frame::matches(&ready, Direction::Ready, nonce, expected_build) {
        return Err(child);
    }
    if framing::ensure_no_trailing_bytes(response_rx.as_fd()).is_err() {
        return Err(child);
    }
    if Instant::now() >= deadline {
        return Err(child);
    }
    if !matches!(child.try_wait(), Ok(None)) {
        return Err(child);
    }
    let pid = child.id();
    Ok(ReadyWorker {
        child,
        request_tx,
        response_rx,
        pid,
        boot_nonce: *nonce,
    })
}

pub(crate) fn spawn_worker_process_for_test(
    executable: &Path,
    request_rx_fd: RawFd,
    response_tx_fd: RawFd,
) -> io::Result<OwnedChild> {
    spawn_worker_process(executable, request_rx_fd, response_tx_fd)
}

fn wait_for_clean_exit(child: &mut OwnedChild, deadline: Instant) -> bool {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return false;
                }
                thread::sleep(remaining.min(Duration::from_millis(5)));
            }
            Err(_) => return false,
        }
    }
}

fn terminate_bounded(mut child: OwnedChild, wait_budget: Duration) -> SlotState {
    match child.try_wait() {
        Ok(Some(_)) => return SlotState::Retired,
        Ok(None) => {
            let _ = child.kill();
        }
        Err(_) if child.identity_lost() => return SlotState::CleanupUnconfirmed(child),
        Err(_) => {}
    }
    let deadline = Instant::now() + wait_budget;
    loop {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return SlotState::Retired;
        }
        if child.identity_lost() {
            return SlotState::CleanupUnconfirmed(child);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return SlotState::CleanupUnconfirmed(child);
        }
        thread::sleep(remaining.min(Duration::from_millis(5)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn wait_identity_loss_stays_unconfirmed() {
        let mut child = OwnedChild::try_from_pid(std::process::id() as libc::pid_t).unwrap();
        child.record_wait_error(&io::Error::from(ErrorKind::Interrupted));
        assert!(!child.identity_lost());

        child.record_wait_error(&io::Error::from_raw_os_error(libc::ECHILD));
        assert!(child.identity_lost());
        let SlotState::CleanupUnconfirmed(mut child) = terminate_bounded(child, Duration::ZERO)
        else {
            panic!("lost identity reported as reaped");
        };
        assert!(child.identity_lost());
        assert_eq!(
            child.try_wait().unwrap_err().raw_os_error(),
            Some(libc::ECHILD)
        );
        assert_eq!(child.wait().unwrap_err().raw_os_error(), Some(libc::ECHILD));
    }

    #[test]
    fn dropping_pool_preserves_an_already_owned_unconfirmed_child() {
        use std::io::Read;
        use std::sync::atomic::{AtomicBool, Ordering};

        const FIXTURE: &str = "ORKWORKS_CUSTODY_DROP_FIXTURE";
        const MARKER: &str = "running isolated custody drop fixture";
        if std::env::var(FIXTURE).as_deref() != Ok("1") {
            let module = module_path!().split_once("::").unwrap().1;
            let name =
                format!("{module}::dropping_pool_preserves_an_already_owned_unconfirmed_child");
            let mut fixture = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", &name, "--nocapture"])
                .env(FIXTURE, "1")
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(12);
            let status = loop {
                if let Some(status) = fixture.try_wait().unwrap() {
                    break status;
                }
                if Instant::now() >= deadline {
                    let _ = fixture.kill();
                    let _ = fixture.wait();
                    panic!("isolated custody drop fixture timed out");
                }
                thread::sleep(Duration::from_millis(5));
            };
            let mut output = String::new();
            fixture
                .stdout
                .take()
                .unwrap()
                .read_to_string(&mut output)
                .unwrap();
            assert!(status.success() && output.contains(MARKER), "{output}");
            return;
        }
        println!("{MARKER}");

        let wait_allowed = [
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ];
        let gates = wait_allowed.clone();
        let (process, observed) = mpsc::channel();
        let mut pool = CustodyPool::start_with_slot_hook(
            [0; 32],
            Duration::from_secs(2),
            Duration::ZERO,
            move |slot, _, _| {
                let gate = gates.get(slot).ok_or(ErrorKind::NotFound)?;
                let child = Command::new("/bin/sleep")
                    .arg("5")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
                let mut owned = OwnedChild::try_from_pid(child.id() as libc::pid_t)?;
                owned.wait_allowed = Some(Arc::clone(gate));
                let _ = process.send(child);
                Ok(owned)
            },
        );
        let first = observed.recv_timeout(Duration::from_secs(1)).unwrap();
        let second = observed.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(pool.shutdown(Duration::ZERO).cleanup_unconfirmed, 2);
        assert_eq!(pool.counts().cleanup_unconfirmed, 2);

        let (returned, complete) = mpsc::channel();
        let dropping = thread::spawn(move || {
            drop(pool);
            let _ = returned.send(());
        });
        let drop_bounded = complete.recv_timeout(Duration::from_secs(2)).is_ok();

        // Release the later slot first. A blocked first slot must not stall
        // the second slot's reap. Release both gates before any assertion.
        wait_allowed[1].store(true, Ordering::Release);
        let second_reaped = drop_bounded && observe_reap(second.id());
        let first_waitable = child_still_waitable(first.id()).is_ok_and(|waitable| waitable);
        wait_allowed[0].store(true, Ordering::Release);
        dropping.join().unwrap();
        let first_reaped = observe_reap(first.id());
        assert!(drop_bounded, "pool drop waited for an unresolved child");
        assert!(first_waitable, "first child reaped before its gate opened");
        assert!(
            second_reaped,
            "later child could not reap while first was held"
        );
        assert!(
            first_reaped,
            "pool drop discarded the unresolved child owner"
        );
    }

    fn observe_reap(pid: u32) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if child_still_waitable(pid).is_ok_and(|waitable| !waitable) {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        false
    }

    fn child_still_waitable(pid: u32) -> io::Result<bool> {
        // SAFETY: This only observes our exact native children. WNOWAIT
        // leaves the real reap exclusively with their cleanup owner.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                pid as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result == 0 {
            return Ok(true);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ECHILD) {
            Ok(false)
        } else {
            Err(error)
        }
    }

    #[test]
    fn expired_queued_spawn_skips_native_launch() {
        let [channels, _one, _two, _three] = channels::seal_split_channels().unwrap();
        let pending = supervise_spawn(channels, Instant::now(), |_, _| {
            panic!("expired launch entered the native operation")
        })
        .unwrap();
        let result = pending.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(result.err().unwrap().kind(), ErrorKind::TimedOut);
    }

    #[test]
    fn owned_child_rejects_non_positive_pids_and_preserves_real_exit_status() {
        assert_eq!(
            OwnedChild::try_from_pid(0).err().map(|e| e.kind()),
            Some(ErrorKind::InvalidInput)
        );
        assert_eq!(
            OwnedChild::try_from_pid(-1).err().map(|e| e.kind()),
            Some(ErrorKind::InvalidInput)
        );

        let child = Command::new("/usr/bin/false")
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn /usr/bin/false");
        let pid = child.id() as libc::pid_t;
        std::mem::forget(child);
        let mut owned = OwnedChild::try_from_pid(pid).expect("positive pid");
        let first = owned.wait().expect("first wait");
        assert!(!first.success());
        let second = owned.wait().expect("second wait");
        assert_eq!(first, second);
        assert_eq!(owned.try_wait().expect("try_wait after wait"), Some(first));
    }

    #[test]
    fn spawn_worker_process_rejects_stdio_and_duplicate_fds() {
        let exe = Path::new("/usr/bin/true");
        for (req, resp) in [(0, 4), (4, 2), (5, 5)] {
            assert_eq!(
                spawn_worker_process(exe, req, resp).err().map(|e| e.kind()),
                Some(ErrorKind::InvalidInput)
            );
        }
    }

    #[test]
    fn cleanup_unconfirmed_slot_is_counted_refuses_ping_and_reaps_on_shutdown() {
        let child = Command::new("/bin/sleep")
            .arg("5")
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn live sleep child");
        let pid = child.id() as libc::pid_t;
        std::mem::forget(child);
        let owned = OwnedChild::try_from_pid(pid).expect("positive pid");

        let mut pool = CustodyPool {
            drop_cleanup: Some(start_drop_cleanup().unwrap()),
            slots: [
                SlotState::CleanupUnconfirmed(owned),
                SlotState::Retired,
                SlotState::Retired,
                SlotState::Retired,
            ],
        };
        assert_eq!(
            pool.counts(),
            PoolCounts {
                ready: 0,
                retired: MAX_SLOTS - 1,
                cleanup_unconfirmed: 1,
            }
        );
        let err = pool
            .ping_slot(0, Instant::now() + Duration::from_secs(1))
            .expect_err("unconfirmed slot cannot accept pings");
        assert_eq!(err.kind(), ErrorKind::NotConnected);

        let report = pool.shutdown(Duration::from_secs(2));
        assert_eq!(report.cleanup_unconfirmed, 0);
        assert_eq!(
            pool.counts(),
            PoolCounts {
                ready: 0,
                retired: MAX_SLOTS,
                cleanup_unconfirmed: 0,
            }
        );
    }

    #[test]
    fn shutdown_with_hung_slot_still_reaps_hung_slot_and_preserves_budget_for_siblings() {
        // Slot 0 is a hung /bin/sleep process masquerading as Ready; slots 1..3 are Retired.
        let (req_tx, req_rx) = UnixStream::pair().expect("req pair");
        let (resp_rx, resp_tx) = channels::create_sealed_pipe().expect("resp pipe");
        let child = Command::new("/bin/sleep")
            .arg("5")
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn hung child");
        let pid = child.id() as libc::pid_t;
        std::mem::forget(child);
        let owned = OwnedChild::try_from_pid(pid).expect("positive pid");

        // Keep peer ends open so slot 0 stalls until its graceful slice expires.
        let _keep_open = (req_rx, resp_tx);
        let mut pool = CustodyPool {
            drop_cleanup: Some(start_drop_cleanup().unwrap()),
            slots: [
                SlotState::Ready(ReadyWorker {
                    child: owned,
                    request_tx: req_tx,
                    response_rx: resp_rx,
                    pid: pid as u32,
                    boot_nonce: [0; 16],
                }),
                SlotState::Retired,
                SlotState::Retired,
                SlotState::Retired,
            ],
        };

        let report = pool.shutdown(Duration::from_millis(150));
        assert_eq!(report.clean_exits, 0);
        assert_eq!(report.forced_kills, 1);
        assert_eq!(report.cleanup_unconfirmed, 0);
        assert_eq!(pool.counts().retired, MAX_SLOTS);
    }
}
