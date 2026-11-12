//! Short piped discovery probes with invocation-owned native cleanup.
use std::{
    io,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncReadExt,
    sync::{Notify, OwnedSemaphorePermit, Semaphore},
};

const OWNER_LIMIT: usize = 64;
const POLL: Duration = Duration::from_millis(10);
const CLEANUP_WAIT: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProbePolicy {
    pub(crate) max_bytes: usize,
    pub(crate) reject_overflow: bool,
    pub(crate) require_success: bool,
}
#[derive(Debug)]
pub(crate) struct CapturedOutput {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProbeError {
    Busy,
    Setup,
    Io,
    Timeout,
    Overflow,
    Unsuccessful,
    CleanupUnconfirmed,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CleanupState {
    Accepted,
    Rejected,
    #[cfg(target_os = "macos")]
    PendingExit,
}

struct Owner {
    child: Child,
    #[cfg(windows)]
    job: Option<crate::process_job::ProcessJob>,
    cleanup: Option<CleanupState>,
    reaped: bool,
    _permit: OwnedSemaphorePermit,
    #[cfg(test)]
    faults: Arc<tests::Faults>,
}
impl Owner {
    fn peek_exit(&self) -> io::Result<Option<bool>> {
        #[cfg(all(test, target_os = "macos"))]
        if self.faults.pending_exit.load(Ordering::Relaxed) {
            self.faults.pending_checks.fetch_add(1, Ordering::Relaxed);
            return Ok(None);
        }
        #[cfg(unix)]
        {
            // This owner is the sole waiter. WNOWAIT keeps the leader's PID
            // reserved until the final group signal, even after natural exit.
            let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
            let result = unsafe {
                libc::waitid(
                    libc::P_PID,
                    self.child.id() as libc::id_t,
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if result != 0 {
                return Err(io::Error::last_os_error());
            }
            if unsafe { info.si_pid() } == 0 {
                return Ok(None);
            }
            Ok(Some(
                info.si_code == libc::CLD_EXITED && unsafe { info.si_status() } == 0,
            ))
        }
        #[cfg(windows)]
        {
            crate::process_job::peek_exit(&self.child)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "probe ownership unsupported",
            ))
        }
    }
    fn terminate(&mut self) -> bool {
        if let Some(state) = self.cleanup {
            return state == CleanupState::Accepted;
        }
        #[cfg(unix)]
        let state = {
            // ECHILD means identity is no longer anchored. Never target a
            // remembered numeric PGID after that loss, or after reaping.
            match self.peek_exit() {
                Err(_) => CleanupState::Rejected,
                Ok(_) => {
                    #[cfg(all(test, target_os = "macos"))]
                    if let Some(before_kill) = self.faults.before_group_kill.lock().unwrap().take()
                    {
                        before_kill(self);
                    }
                    #[cfg(all(test, target_os = "macos"))]
                    self.faults.group_signals.fetch_add(1, Ordering::Relaxed);
                    let result =
                        unsafe { libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL) };
                    let error = io::Error::last_os_error().raw_os_error();
                    let mut state = if result == 0 || error == Some(libc::ESRCH) {
                        CleanupState::Accepted
                    } else {
                        CleanupState::Rejected
                    };
                    #[cfg(target_os = "macos")]
                    if state == CleanupState::Rejected && error == Some(libc::EPERM) {
                        // Darwin can reject the signal before the exiting
                        // leader is waitable. Pending is not cleanup evidence.
                        state = exited_leader_cleanup(self);
                    }
                    state
                }
            }
        };
        #[cfg(windows)]
        let state = {
            let job_ok = self.job.as_ref().is_some_and(|job| job.terminate().is_ok());
            // Covers a suspended child whose Job assignment failed. This is an
            // owned process handle, not a PID reconstructed after exit.
            let _ = self.child.kill();
            self.job.take(); // final kill-on-close handle, including failure
            if job_ok {
                CleanupState::Accepted
            } else {
                CleanupState::Rejected
            }
        };
        #[cfg(not(any(unix, windows)))]
        let state = CleanupState::Rejected;
        self.record_cleanup(state)
    }
    fn record_cleanup(&mut self, state: CleanupState) -> bool {
        #[cfg(test)]
        let state = if self.faults.cleanup.load(Ordering::Relaxed) {
            CleanupState::Rejected
        } else {
            state
        };
        self.cleanup = Some(state);
        if state == CleanupState::Rejected {
            tracing::warn!(
                stage = "probe_cleanup",
                code = "unconfirmed",
                "probe cleanup unconfirmed"
            );
        }
        state == CleanupState::Accepted
    }
    fn confirm_cleanup(&mut self) {
        #[cfg(target_os = "macos")]
        if self.cleanup == Some(CleanupState::PendingExit) {
            let state = exited_leader_cleanup(self);
            self.record_cleanup(state);
        }
    }
    fn cancel_cleanup(&mut self) {
        self.terminate();
        #[cfg(target_os = "macos")]
        if self.cleanup == Some(CleanupState::PendingExit) {
            self.record_cleanup(CleanupState::Rejected);
        }
    }
    fn reap(&mut self) {
        #[cfg(test)]
        if self.faults.reap.load(Ordering::Relaxed) {
            return;
        }
        if !matches!(
            self.cleanup,
            Some(CleanupState::Accepted | CleanupState::Rejected)
        ) || self.reaped
        {
            return;
        }
        if let Ok(Some(_)) = self.child.try_wait() {
            self.reaped = true;
        }
    }
}

#[cfg(target_os = "macos")]
fn exited_leader_cleanup(owner: &Owner) -> CleanupState {
    match owner.peek_exit() {
        Ok(None) => return CleanupState::PendingExit,
        Err(_) => return CleanupState::Rejected,
        Ok(Some(_)) => {}
    }
    let leader = owner.child.id();
    const PROC_PGRP_ONLY: u32 = 2; // SDK sys/proc_info.h
    let mut members = [0i32; 2];
    // Identity is supplied by the unreaped original child, not this query.
    // Two entries suffice: only the leader may remain; any second PID fails.
    let bytes = unsafe {
        libc::proc_listpids(
            PROC_PGRP_ONLY,
            leader,
            members.as_mut_ptr().cast(),
            std::mem::size_of_val(&members) as i32,
        )
    };
    #[cfg(test)]
    let bytes = if let Some((bytes, injected)) = *owner.faults.membership_result.lock().unwrap() {
        members = injected;
        bytes
    } else {
        bytes
    };
    if bytes < 0
        || bytes as usize > std::mem::size_of_val(&members)
        || bytes as usize % std::mem::size_of::<i32>() != 0
    {
        return CleanupState::Rejected;
    }
    if members[..bytes as usize / std::mem::size_of::<i32>()]
        .iter()
        .all(|pid| *pid == leader as i32)
    {
        CleanupState::Accepted
    } else {
        CleanupState::Rejected
    }
}

#[cfg(all(test, target_os = "macos"))]
fn only_exited_leader(owner: &Owner) -> bool {
    exited_leader_cleanup(owner) == CleanupState::Accepted
}

async fn capture_stream<R: tokio::io::AsyncRead + Unpin>(
    stream: R,
    policy: ProbePolicy,
) -> Result<Vec<u8>, ProbeError> {
    let limit = policy.max_bytes + usize::from(policy.reject_overflow);
    let mut stream = stream.take(limit as u64);
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| ProbeError::Io)?;
    if bytes.len() > policy.max_bytes {
        return Err(ProbeError::Overflow);
    }
    Ok(bytes)
}

struct Shared {
    permits: Arc<Semaphore>,
    owners: Mutex<Vec<Arc<Mutex<Owner>>>>,
    closed: AtomicBool,
    closing_at: Mutex<Option<Instant>>,
    cancelled: Notify,
    #[cfg(test)]
    faults: Arc<tests::Faults>,
}
impl Shared {
    fn close(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            *self.closing_at.lock().unwrap() = Some(Instant::now());
        }
        self.cancelled.notify_waiters();
        let owners = self.owners.lock().unwrap().clone();
        for owner in owners {
            owner.lock().unwrap().cancel_cleanup();
        }
    }
    fn register(&self, owner: Arc<Mutex<Owner>>) -> bool {
        let mut owners = self.owners.lock().unwrap();
        // Shares the publication lock with shutdown's owner snapshot. If close
        // won, the local lease still terminates/reaps this just-spawned child.
        let open = !self.closed.load(Ordering::Acquire);
        owners.push(owner);
        open
    }
    fn sweep(&self) -> bool {
        // No registry lock around native status operations.
        let owners = self.owners.lock().unwrap().clone();
        for owner in owners {
            owner.lock().unwrap().reap();
        }
        let mut owners = self.owners.lock().unwrap();
        owners.retain(|owner| !owner.lock().unwrap().reaped);
        owners.is_empty()
    }
}

pub(crate) struct ProbeService {
    shared: Arc<Shared>,
}
impl ProbeService {
    pub(crate) fn new() -> io::Result<Self> {
        let shared = Arc::new(Shared {
            permits: Arc::new(Semaphore::new(OWNER_LIMIT)),
            owners: Mutex::new(Vec::with_capacity(OWNER_LIMIT)),
            closed: AtomicBool::new(false),
            closing_at: Mutex::new(None),
            cancelled: Notify::new(),
            #[cfg(test)]
            faults: Arc::new(tests::Faults::default()),
        });
        let reaper = shared.clone();
        std::thread::Builder::new()
            .name("probe-reaper".into())
            .spawn(move || loop {
                let empty = reaper.sweep();
                if reaper.closed.load(Ordering::Acquire) {
                    let expired = reaper
                        .closing_at
                        .lock()
                        .unwrap()
                        .is_some_and(|at| at.elapsed() >= CLEANUP_WAIT);
                    if (empty && reaper.permits.available_permits() == OWNER_LIMIT) || expired {
                        break;
                    }
                }
                std::thread::sleep(POLL);
            })?;
        Ok(Self { shared })
    }

    pub(crate) async fn run(
        &self,
        command: &mut Command,
        policy: ProbePolicy,
        timeout: Duration,
    ) -> Result<CapturedOutput, ProbeError> {
        let deadline = tokio::time::Instant::now() + timeout;
        if self.shared.closed.load(Ordering::Acquire) {
            return Err(ProbeError::Closed);
        }
        let permit = self
            .shared
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| ProbeError::Busy)?;
        if policy.max_bytes == 0 || policy.max_bytes.checked_add(1).is_none() {
            return Err(ProbeError::Setup);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(ProbeError::Timeout);
        }
        #[cfg(not(any(unix, windows)))]
        return Err(ProbeError::Setup);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(windows)]
        let job = {
            use std::os::windows::process::CommandExt;
            command.creation_flags(windows_sys::Win32::System::Threading::CREATE_SUSPENDED);
            crate::process_job::ProcessJob::new().map_err(|_| ProbeError::Setup)?
        };
        command.stdin(Stdio::null());
        #[cfg(not(windows))]
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)]
        let (mut stdout, mut stderr) = {
            let (stdout, out_writer) =
                windows_pipe::capture_pipe().map_err(|_| ProbeError::Setup)?;
            let (stderr, err_writer) =
                windows_pipe::capture_pipe().map_err(|_| ProbeError::Setup)?;
            command.stdout(out_writer).stderr(err_writer);
            (stdout, stderr)
        };
        let child = command.spawn().map_err(|_| ProbeError::Setup)?;
        // Install the drop guard before any fallible post-spawn operation.
        let owner = Arc::new(Mutex::new(Owner {
            child,
            cleanup: None,
            reaped: false,
            _permit: permit,
            #[cfg(test)]
            faults: self.shared.faults.clone(),
            #[cfg(windows)]
            job: Some(job),
        }));
        let lease = Lease(owner.clone());
        let open = self.shared.register(owner.clone());
        if !open {
            return Err(ProbeError::Closed);
        }
        // Command retains configured Windows writer handles across spawn. Close
        // those parent copies so EOF depends only on the invocation's writers.
        #[cfg(windows)]
        command.stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(test)]
        if self
            .shared
            .faults
            .close_after_register
            .load(Ordering::Relaxed)
        {
            self.shared.close();
        }
        #[cfg(not(windows))]
        let (stdout, stderr) = {
            let mut owned = owner.lock().unwrap();

            (
                owned.child.stdout.take().ok_or(ProbeError::Io)?,
                owned.child.stderr.take().ok_or(ProbeError::Io)?,
            )
        };
        #[cfg(windows)]
        {
            let owned = owner.lock().unwrap();
            // Shutdown and attachment serialize on this owner. If shutdown won,
            // it already terminated the suspended child and closed the Job.
            if owned.cleanup.is_some() {
                return Err(ProbeError::Closed);
            }
            owned
                .job
                .as_ref()
                .ok_or(ProbeError::Setup)?
                .attach_and_resume(&owned.child)
                .map_err(|_| ProbeError::Setup)?;
        }
        #[cfg(test)]
        if self.shared.faults.pipe_setup.load(Ordering::Relaxed) {
            return Err(ProbeError::Io);
        }
        #[cfg(not(windows))]
        let mut stdout =
            tokio::process::ChildStdout::from_std(stdout).map_err(|_| ProbeError::Io)?;
        #[cfg(not(windows))]
        let mut stderr =
            tokio::process::ChildStderr::from_std(stderr).map_err(|_| ProbeError::Io)?;
        let capture = async {
            #[cfg(windows)]
            tokio::try_join!(stdout.connect(), stderr.connect()).map_err(|_| ProbeError::Io)?;
            let (out, err) = tokio::try_join!(
                capture_stream(&mut stdout, policy),
                capture_stream(&mut stderr, policy)
            )?;
            if policy.require_success {
                loop {
                    let exited = owner
                        .lock()
                        .unwrap()
                        .peek_exit()
                        .map_err(|_| ProbeError::CleanupUnconfirmed)?;
                    match exited {
                        Some(true) => break,
                        Some(false) => return Err(ProbeError::Unsuccessful),
                        None => tokio::time::sleep(POLL).await,
                    }
                }
            }
            Ok(CapturedOutput {
                stdout: out,
                stderr: err,
            })
        };
        let result = tokio::select! {
            biased;
            _ = self.shared.cancelled.notified() => Err(ProbeError::Closed),
            result = tokio::time::timeout_at(deadline, capture) => result.unwrap_or(Err(ProbeError::Timeout)),
        };
        self.settle(&owner, deadline, result.as_ref().err().copied())
            .await?;
        drop(lease);
        result
    }

    async fn settle(
        &self,
        owner: &Arc<Mutex<Owner>>,
        deadline: tokio::time::Instant,
        capture_error: Option<ProbeError>,
    ) -> Result<(), ProbeError> {
        // Confirmation and reaping share this single settlement endpoint.
        let settle_until = std::cmp::min(deadline, tokio::time::Instant::now() + CLEANUP_WAIT);
        {
            let mut owned = owner.lock().unwrap();
            owned.terminate();
            if self.shared.closed.load(Ordering::Acquire) {
                return Err(ProbeError::Closed);
            }
            if owned.cleanup == Some(CleanupState::Rejected) {
                return Err(ProbeError::CleanupUnconfirmed);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(capture_error.unwrap_or(ProbeError::Timeout));
            }
        }
        loop {
            if self.shared.closed.load(Ordering::Acquire) {
                return Err(ProbeError::Closed);
            }
            {
                let mut owned = owner.lock().unwrap();
                if owned.cleanup == Some(CleanupState::Rejected) {
                    return Err(ProbeError::CleanupUnconfirmed);
                }
                if tokio::time::Instant::now() >= settle_until {
                    return Err(ProbeError::CleanupUnconfirmed);
                }
                owned.confirm_cleanup();
                if owned.cleanup == Some(CleanupState::Rejected) {
                    return Err(ProbeError::CleanupUnconfirmed);
                }
                owned.reap();
                if owned.reaped {
                    if tokio::time::Instant::now() >= settle_until {
                        return Err(ProbeError::CleanupUnconfirmed);
                    }
                    break;
                }
            }
            tokio::time::sleep_until(std::cmp::min(
                settle_until,
                tokio::time::Instant::now() + POLL,
            ))
            .await;
        }
        if self.shared.closed.load(Ordering::Acquire) {
            return Err(ProbeError::Closed);
        }
        Ok(())
    }

    pub(crate) async fn shutdown(&self) {
        self.shared.close();
        let deadline = tokio::time::Instant::now() + CLEANUP_WAIT;
        while tokio::time::Instant::now() < deadline {
            if self.shared.sweep() && self.shared.permits.available_permits() == OWNER_LIMIT {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    }
}
impl Drop for ProbeService {
    fn drop(&mut self) {
        self.shared.close();
    }
}
struct Lease(Arc<Mutex<Owner>>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.lock().unwrap().cancel_cleanup();
    }
}
static SERVICE: OnceLock<Result<ProbeService, ()>> = OnceLock::new();
pub(crate) async fn run(
    command: &mut Command,
    policy: ProbePolicy,
    timeout: Duration,
) -> Result<CapturedOutput, ProbeError> {
    SERVICE
        .get_or_init(|| ProbeService::new().map_err(|_| ()))
        .as_ref()
        .map_err(|_| ProbeError::Setup)?
        .run(command, policy, timeout)
        .await
}
pub(crate) async fn shutdown() {
    // Publish a closed service even if no probe has initialized it yet. A
    // queued request reaching run during shutdown must not open fresh admission.
    if let Ok(service) = SERVICE.get_or_init(|| ProbeService::new().map_err(|_| ())) {
        service.shutdown().await;
    }
}

#[cfg(test)]
#[path = "probe_process/tests.rs"]
mod tests;
#[cfg(all(test, windows))]
#[path = "probe_process/windows_tests.rs"]
mod windows_tests;

#[cfg(windows)]
#[path = "probe_process/windows_pipe.rs"]
mod windows_pipe;
