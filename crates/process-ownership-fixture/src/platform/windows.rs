//! Windows Job Object ownership adapter for the process-ownership proof.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpListener;
use std::os::windows::io::{AsRawHandle, AsRawSocket, BorrowedHandle, FromRawHandle, OwnedHandle};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use windows_sys::Win32::Foundation::{
    GetHandleInformation, ERROR_ACCESS_DENIED, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob,
    JobObjectBasicAccountingInformation, JobObjectBasicProcessIdList,
    JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
    TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{
    OpenThread, ResumeThread, TerminateProcess, WaitForSingleObject, CREATE_SUSPENDED,
    THREAD_SUSPEND_RESUME,
};

use crate::observation::{is_complete, ContainmentMembership, ExitState};
use crate::protocol::{GenerationId, NativeIdentity, Role};
use crate::supervisor::{
    ExecutableImage, LaunchSpec, OwnedProcessHandle, PausedRoot, PlatformAdapter, SpawnError,
    RELEASE_EXEC_BYTE,
};
use crate::targets::TargetBehavior;

const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const BREAKAWAY_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_FIXTURE_JOB_PROCESSES: usize = 64;
const ELECTRON_PARENT_FLAG: &str = "--windows-electron-parent";
const OWNER_SUPERVISOR_FLAG: &str = "--windows-owner-supervisor";

/// Windows Job Object operation failure.
#[derive(Debug, Error)]
pub enum PlatformError {
    /// A named Win32 operation failed.
    #[error("Windows process ownership operation `{operation}` failed: {source}")]
    Windows {
        /// Operation whose Win32 call failed.
        operation: &'static str,
        /// Last operating-system error reported by the failed call.
        #[source]
        source: io::Error,
    },
    /// The shared owner state could not be inspected.
    #[error("Windows owner state is unavailable")]
    StateUnavailable,
    /// The verified fixture executable could not be prepared.
    #[error("verified fixture executable is unavailable")]
    ExecutableUnavailable,
    /// The Job Object still reported active processes after termination.
    #[error("Windows Job Object completion was not observed before the deadline")]
    CompletionTimeout,
    /// The Job Object exceeded the deliberately bounded fixture census.
    #[error("Windows Job Object process census exceeded the fixture bound")]
    ProcessCensusOverflow,
    /// A Windows-only helper command did not have its exact closed shape.
    #[error("invalid Windows process ownership helper arguments")]
    InvalidHelperArguments,
}

/// Outcome of attempting an unapproved `CREATE_BREAKAWAY_FROM_JOB` launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakawayResult {
    /// Windows rejected the attempted breakaway.
    Rejected,
    /// A descendant escaped despite the no-breakaway Job Object.
    Escaped,
    /// The probe could not establish either result.
    Unresolved,
}

/// Evidence published after the Electron-like process has launched the real supervisor.
#[derive(Debug, Deserialize, Serialize)]
pub struct ForcedParentReady {
    /// PID of the out-of-process supervisor retaining Job Object authority.
    pub supervisor_pid: u32,
    /// PID of the target launched and admitted by that supervisor.
    pub owned_target_pid: u32,
    /// Whether all retained authority and parent-channel handles reject inheritance.
    pub handles_non_inheritable: bool,
}

/// Evidence published by the surviving supervisor after parent loss and cleanup.
#[derive(Debug, Deserialize, Serialize)]
pub struct ForcedParentCompletion {
    /// PID of the supervisor that observed parent loss and performed cleanup.
    pub supervisor_pid: u32,
    /// True only when EOF was observed on the Electron-owned channel.
    pub owner_loss_observed: bool,
    /// Whether the endpoint could be rebound while the owned target was still live.
    pub endpoint_rebound_while_target_live: bool,
    /// Independent Job Object census after `TerminateJobObject` completion.
    pub active_processes_after_termination: usize,
}

/// Builds the exact command line for the Electron-like topology helper.
#[must_use]
pub fn electron_parent_arguments(
    ready: &Path,
    completion: &Path,
    target_marker: &Path,
) -> Vec<OsString> {
    vec![
        OsString::from(ELECTRON_PARENT_FLAG),
        ready.as_os_str().to_owned(),
        completion.as_os_str().to_owned(),
        target_marker.as_os_str().to_owned(),
    ]
}

/// Handles the Windows-only parent/supervisor fixture helper command lines.
///
/// Returns `Ok(false)` when the arguments belong to the ordinary target entry
/// point. The Electron-like helper owns a pipe writer and launches the separate
/// supervisor helper; force-terminating Electron closes that writer, allowing
/// the supervisor to observe owner loss without inheriting Electron lifetime.
///
/// # Errors
///
/// Returns [`PlatformError`] when helper arguments, process launch, ownership,
/// evidence publication, or cleanup observation fails.
pub fn run_helper_from_args(args: &[OsString]) -> Result<bool, PlatformError> {
    let Some(flag) = args.first() else {
        return Ok(false);
    };
    if flag == OsStr::new(ELECTRON_PARENT_FLAG) {
        let [_, ready, completion, target_marker] = args else {
            return Err(PlatformError::InvalidHelperArguments);
        };
        run_electron_parent(
            Path::new(ready),
            Path::new(completion),
            Path::new(target_marker),
        )?;
        return Ok(true);
    }
    if flag == OsStr::new(OWNER_SUPERVISOR_FLAG) {
        let [_, ready, completion, target_marker] = args else {
            return Err(PlatformError::InvalidHelperArguments);
        };
        run_owner_supervisor(
            Path::new(ready),
            Path::new(completion),
            Path::new(target_marker),
        )?;
        return Ok(true);
    }
    Ok(false)
}

fn run_electron_parent(
    ready: &Path,
    completion: &Path,
    target_marker: &Path,
) -> Result<(), PlatformError> {
    let executable = std::env::current_exe().map_err(|source| PlatformError::Windows {
        operation: "locate Windows fixture helper executable",
        source,
    })?;
    let mut supervisor = Command::new(executable)
        .args([
            OsString::from(OWNER_SUPERVISOR_FLAG),
            ready.as_os_str().to_owned(),
            completion.as_os_str().to_owned(),
            target_marker.as_os_str().to_owned(),
        ])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|source| PlatformError::Windows {
            operation: "launch out-of-process Windows owner supervisor",
            source,
        })?;
    let _owner_channel = supervisor
        .stdin
        .take()
        .ok_or(PlatformError::StateUnavailable)?;
    loop {
        if supervisor
            .try_wait()
            .map_err(|source| PlatformError::Windows {
                operation: "observe out-of-process Windows owner supervisor",
                source,
            })?
            .is_some()
        {
            return Err(PlatformError::StateUnavailable);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn run_owner_supervisor(
    ready: &Path,
    completion: &Path,
    target_marker: &Path,
) -> Result<(), PlatformError> {
    let mut parent_channel = io::stdin();
    clear_inherit(
        parent_channel.as_raw_handle(),
        "SetHandleInformation(parent-loss channel)",
    )?;
    let parent_channel_non_inheritable = !is_inheritable(parent_channel.as_raw_handle())?;
    let domain = OwnerDomain::create(31)?;
    let (mut supervisor, _) =
        crate::supervisor::Supervisor::prepare_with_adapter(31, domain.clone())
            .map_err(|error| helper_error("prepare out-of-process Windows supervisor", error))?;
    let endpoint = supervisor
        .control_endpoint_addr()
        .ok_or(PlatformError::StateUnavailable)?;
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "windows-forced-parent-target")
        .map_err(platform_spawn_error)?;
    let spec = LaunchSpec::fixture(
        Role::Inference,
        TargetBehavior::Inference,
        target_marker.to_path_buf(),
        Duration::from_secs(30),
    )
    .map_err(platform_spawn_error)?;
    let target = supervisor
        .spawn(ticket, spec)
        .map_err(platform_spawn_error)?;
    let ready_evidence = ForcedParentReady {
        supervisor_pid: std::process::id(),
        owned_target_pid: target.diagnostic_pid,
        handles_non_inheritable: parent_channel_non_inheritable
            && domain.handles_are_non_inheritable()?,
    };
    write_json_evidence(ready, &ready_evidence, "publish supervisor ready evidence")?;

    let mut unexpected = [0_u8; 1];
    if parent_channel
        .read(&mut unexpected)
        .map_err(|source| PlatformError::Windows {
            operation: "observe Electron parent-loss channel",
            source,
        })?
        != 0
    {
        return Err(PlatformError::StateUnavailable);
    }

    supervisor.owner_lost();
    let target_was_live = domain
        .active_process_ids()?
        .contains(&target.diagnostic_pid);
    let endpoint_rebound = TcpListener::bind(endpoint).is_ok();
    domain.terminate_owned()?;
    let active_processes = domain.active_process_ids()?.len();
    let deadline = Instant::now() + COMPLETION_TIMEOUT;
    while !is_complete(&supervisor.observe()) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    if !is_complete(&supervisor.observe()) {
        return Err(PlatformError::CompletionTimeout);
    }
    let completion_evidence = ForcedParentCompletion {
        supervisor_pid: std::process::id(),
        owner_loss_observed: true,
        endpoint_rebound_while_target_live: target_was_live && endpoint_rebound,
        active_processes_after_termination: active_processes,
    };
    write_json_evidence(
        completion,
        &completion_evidence,
        "publish supervisor completion evidence",
    )
}

fn write_json_evidence<T: Serialize>(
    path: &Path,
    evidence: &T,
    operation: &'static str,
) -> Result<(), PlatformError> {
    let temporary = related_marker(path, &format!("{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec(evidence).map_err(|error| helper_error(operation, error))?;
    fs::write(&temporary, bytes).map_err(|source| PlatformError::Windows { operation, source })?;
    fs::rename(temporary, path).map_err(|source| PlatformError::Windows { operation, source })
}

fn helper_error(operation: &'static str, error: impl std::fmt::Display) -> PlatformError {
    PlatformError::Windows {
        operation,
        source: io::Error::other(error.to_string()),
    }
}

#[derive(Debug)]
struct DomainState {
    pending_identities: HashMap<u32, NativeIdentity>,
    registered_processes: HashMap<NativeIdentity, OwnedHandle>,
    endpoint_handle: Option<usize>,
    thread_handles_non_inheritable: bool,
    // Drop the retained process handles before kill-on-close releases the job.
    job: OwnedHandle,
}

/// Cloneable supervisor authority over one private Windows Job Object.
#[derive(Clone, Debug)]
pub struct OwnerDomain {
    generation: GenerationId,
    state: Arc<Mutex<DomainState>>,
}

impl OwnerDomain {
    /// Creates a private Job Object with kill-on-close and no breakaway allowance.
    ///
    /// # Errors
    ///
    /// Returns [`PlatformError`] if creation, handle isolation, or limit setup fails.
    pub fn create(generation: GenerationId) -> Result<Self, PlatformError> {
        // SAFETY: null security attributes and name request a private,
        // non-inheritable Job Object handle owned by this call.
        let raw_job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw_job.is_null() {
            return Err(last_error("CreateJobObjectW"));
        }
        // SAFETY: successful creation transfers one valid owned handle.
        let job = unsafe { OwnedHandle::from_raw_handle(raw_job) };
        clear_inherit(job.as_raw_handle(), "SetHandleInformation(job)")?;

        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: `job` is live and `limits` has the exact initialized layout
        // required by this information class.
        let configured = unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_u32(&limits),
            )
        };
        if configured == 0 {
            return Err(last_error("SetInformationJobObject"));
        }

        Ok(Self {
            generation,
            state: Arc::new(Mutex::new(DomainState {
                pending_identities: HashMap::new(),
                registered_processes: HashMap::new(),
                endpoint_handle: None,
                thread_handles_non_inheritable: true,
                job,
            })),
        })
    }

    /// Starts a root suspended, assigns and registers it, then resumes it.
    ///
    /// # Errors
    ///
    /// Returns [`PlatformError`] if any pre-release ownership step fails.
    pub fn launch_paused(&mut self, spec: LaunchSpec) -> Result<PausedRoot, PlatformError> {
        let executable =
            ExecutableImage::discover().map_err(|_| PlatformError::ExecutableUnavailable)?;
        let mut process_handle = self
            .create_paused_root(&executable, &spec)
            .map_err(platform_spawn_error)?;
        let identity = match self.capture_native_identity(&process_handle) {
            Ok(identity) => identity,
            Err(error) => {
                let _ = self.terminate_root(&mut process_handle, Duration::from_secs(2));
                return Err(platform_spawn_error(error));
            }
        };
        match self.attach_containment(self.generation, &process_handle) {
            Ok(ContainmentMembership::Confirmed) => {}
            Ok(ContainmentMembership::Unresolved) => {
                let _ = self.terminate_root(&mut process_handle, Duration::from_secs(2));
                return Err(platform_spawn_error(SpawnError::ContainmentFailed));
            }
            Err(error) => {
                let _ = self.terminate_root(&mut process_handle, Duration::from_secs(2));
                return Err(platform_spawn_error(error));
            }
        }
        let mut paused_root = PausedRoot {
            identity,
            process_handle,
        };
        if let Err(error) = self.release_exec(&mut paused_root) {
            let _ = self.terminate_root(&mut paused_root.process_handle, Duration::from_secs(2));
            return Err(platform_spawn_error(error));
        }
        Ok(paused_root)
    }

    /// Terminates the job and waits for an independently observed empty census.
    ///
    /// # Errors
    ///
    /// Returns [`PlatformError`] when termination or completion observation fails.
    pub fn terminate_owned(&self) -> Result<(), PlatformError> {
        let state = self.lock_state()?;
        // SAFETY: the private Job Object contains only roots admitted by this
        // owner and their inherited descendants.
        if unsafe { TerminateJobObject(state.job.as_raw_handle(), 137) } == 0 {
            return Err(last_error("TerminateJobObject"));
        }
        let deadline = Instant::now() + COMPLETION_TIMEOUT;
        loop {
            if active_process_count(state.job.as_raw_handle())? == 0 {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(PlatformError::CompletionTimeout);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Attempts a prohibited descendant breakaway from the private job.
    #[must_use]
    pub fn attempt_breakaway(&self) -> BreakawayResult {
        self.attempt_breakaway_inner()
            .unwrap_or(BreakawayResult::Unresolved)
    }

    /// Returns PIDs currently reported by the private Job Object.
    ///
    /// # Errors
    ///
    /// Returns [`PlatformError`] when the bounded job census cannot be queried.
    pub fn active_process_ids(&self) -> Result<Vec<u32>, PlatformError> {
        let state = self.lock_state()?;
        query_process_ids(state.job.as_raw_handle())
    }

    /// Terminates one registered root without directly terminating its descendants.
    ///
    /// # Errors
    ///
    /// Returns [`PlatformError`] if the exact registered identity is unavailable
    /// or cannot be terminated and observed exited.
    pub fn terminate_registered(&self, identity: &NativeIdentity) -> Result<(), PlatformError> {
        let state = self.lock_state()?;
        let process = state
            .registered_processes
            .get(identity)
            .ok_or(PlatformError::StateUnavailable)?;
        // SAFETY: the retained handle belongs to this exact registered birth
        // identity and grants PROCESS_TERMINATE access.
        if unsafe { TerminateProcess(process.as_raw_handle(), 137) } == 0 {
            return Err(last_error("TerminateProcess(registered root)"));
        }
        wait_for_process(process.as_raw_handle(), COMPLETION_TIMEOUT)
    }

    /// Reports liveness only while the retained handle matches the registered birth identity.
    ///
    /// # Errors
    ///
    /// Returns [`PlatformError`] if the identity is absent or cannot be observed.
    pub fn registered_identity_is_alive(
        &self,
        identity: &NativeIdentity,
    ) -> Result<bool, PlatformError> {
        let state = self.lock_state()?;
        let process = state
            .registered_processes
            .get(identity)
            .ok_or(PlatformError::StateUnavailable)?;
        if wait_result(process.as_raw_handle(), 0)? == WAIT_OBJECT_0 {
            return Ok(false);
        }
        Ok(capture_identity(process.as_raw_handle(), identity.diagnostic_pid)? == *identity)
    }

    /// Confirms retained job, process, thread, and endpoint handles are non-inheritable.
    ///
    /// # Errors
    ///
    /// Returns [`PlatformError`] if a live handle flag cannot be queried.
    pub fn handles_are_non_inheritable(&self) -> Result<bool, PlatformError> {
        let state = self.lock_state()?;
        if is_inheritable(state.job.as_raw_handle())? || !state.thread_handles_non_inheritable {
            return Ok(false);
        }
        if let Some(endpoint) = state.endpoint_handle {
            if is_inheritable(endpoint as HANDLE)? {
                return Ok(false);
            }
        }
        for process in state.registered_processes.values() {
            if is_inheritable(process.as_raw_handle())? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn attempt_breakaway_inner(&self) -> Result<BreakawayResult, PlatformError> {
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|_| PlatformError::StateUnavailable)?;
        let directory = std::env::temp_dir().join(format!(
            "orkworks-windows-breakaway-{}-{}",
            std::process::id(),
            hex::encode(random)
        ));
        fs::create_dir(&directory).map_err(|source| PlatformError::Windows {
            operation: "create breakaway probe directory",
            source,
        })?;
        let _cleanup = BreakawayDirectory(directory.clone());
        let marker = directory.join("probe");
        let result_marker = related_marker(&marker, "breakaway");
        let spec = LaunchSpec::fixture(
            Role::Sidecar,
            TargetBehavior::Daemonized,
            marker,
            Duration::from_secs(10),
        )
        .map_err(platform_spawn_error)?;
        let mut launcher = self.clone();
        let _root = launcher.launch_paused(spec)?;
        let deadline = Instant::now() + BREAKAWAY_TIMEOUT;
        while !result_marker.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let result =
            fs::read_to_string(&result_marker).map_err(|source| PlatformError::Windows {
                operation: "read breakaway probe result",
                source,
            })?;
        match result.trim() {
            rejected if rejected == format!("rejected:{}", ERROR_ACCESS_DENIED).as_str() => {
                Ok(BreakawayResult::Rejected)
            }
            escaped if escaped.starts_with("escaped-cleaned:") => Ok(BreakawayResult::Escaped),
            _ => Ok(BreakawayResult::Unresolved),
        }
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, DomainState>, PlatformError> {
        self.state
            .lock()
            .map_err(|_| PlatformError::StateUnavailable)
    }
}

impl PlatformAdapter for OwnerDomain {
    fn secure_control_endpoint(&mut self, endpoint: &TcpListener) -> Result<(), SpawnError> {
        let handle = endpoint.as_raw_socket() as usize as HANDLE;
        clear_inherit(handle, "SetHandleInformation(supervisor endpoint)")
            .map_err(|_| SpawnError::ContainmentFailed)?;
        self.lock_state()
            .map_err(|_| SpawnError::ContainmentFailed)?
            .endpoint_handle = Some(handle as usize);
        Ok(())
    }

    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        let mut child = Command::new(executable.path())
            .args(spec.args())
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED)
            .spawn()
            .map_err(|_| SpawnError::ContainmentFailed)?;
        clear_inherit(child.as_raw_handle(), "SetHandleInformation(process)").map_err(|_| {
            let _ = child.kill();
            SpawnError::ContainmentFailed
        })?;
        let release_gate = child.stdin.take();
        if release_gate.is_none() {
            let _ = child.kill();
            return Err(SpawnError::ContainmentFailed);
        }
        Ok(OwnedProcessHandle {
            child,
            release_gate,
        })
    }

    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError> {
        let pid = process_handle.diagnostic_pid();
        let identity = capture_identity(process_handle.child.as_raw_handle(), pid)
            .map_err(|_| SpawnError::IdentityUnavailable)?;
        self.lock_state()
            .map_err(|_| SpawnError::ObserverUnavailable)?
            .pending_identities
            .insert(pid, identity.clone());
        Ok(identity)
    }

    fn attach_containment(
        &mut self,
        generation: GenerationId,
        process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError> {
        if generation != self.generation {
            return Err(SpawnError::ContainmentFailed);
        }
        let mut state = self
            .lock_state()
            .map_err(|_| SpawnError::ObserverUnavailable)?;
        // SAFETY: both handles are live, and the child remains suspended until release.
        if unsafe {
            AssignProcessToJobObject(
                state.job.as_raw_handle(),
                process_handle.child.as_raw_handle(),
            )
        } == 0
        {
            return Err(SpawnError::ContainmentFailed);
        }
        let pid = process_handle.diagnostic_pid();
        let identity = state
            .pending_identities
            .remove(&pid)
            .ok_or(SpawnError::IdentityUnavailable)?;
        // SAFETY: the live child handle is borrowed only during duplication.
        let borrowed = unsafe { BorrowedHandle::borrow_raw(process_handle.child.as_raw_handle()) };
        let retained = borrowed
            .try_clone_to_owned()
            .map_err(|_| SpawnError::ObserverUnavailable)?;
        clear_inherit(
            retained.as_raw_handle(),
            "SetHandleInformation(retained process)",
        )
        .map_err(|_| SpawnError::ContainmentFailed)?;
        state.registered_processes.insert(identity, retained);
        Ok(ContainmentMembership::Confirmed)
    }

    fn release_exec(&mut self, paused_root: &mut PausedRoot) -> Result<(), SpawnError> {
        let Some(mut release_gate) = paused_root.process_handle.release_gate.take() else {
            return Err(SpawnError::ContainmentFailed);
        };
        release_gate
            .write_all(&[RELEASE_EXEC_BYTE])
            .map_err(|_| SpawnError::ContainmentFailed)?;
        drop(release_gate);
        let thread_non_inheritable = resume_initial_thread(&paused_root.process_handle)?;
        self.lock_state()
            .map_err(|_| SpawnError::ObserverUnavailable)?
            .thread_handles_non_inheritable &= thread_non_inheritable;
        Ok(())
    }

    fn observe_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError> {
        let wait = wait_result(process_handle.child.as_raw_handle(), 0)
            .map_err(|_| SpawnError::ObserverUnavailable)?;
        if wait == WAIT_OBJECT_0 {
            return Ok(ExitState::Exited);
        }
        let observed = capture_identity(
            process_handle.child.as_raw_handle(),
            process_handle.diagnostic_pid(),
        )
        .map_err(|_| SpawnError::ObserverUnavailable)?;
        if observed != *expected_identity {
            return Err(SpawnError::ObserverUnavailable);
        }
        let state = self
            .lock_state()
            .map_err(|_| SpawnError::ObserverUnavailable)?;
        let mut in_job = 0;
        // SAFETY: both handles are live and `in_job` is writable BOOL storage.
        if unsafe {
            IsProcessInJob(
                process_handle.child.as_raw_handle(),
                state.job.as_raw_handle(),
                &mut in_job,
            )
        } == 0
            || in_job == 0
        {
            return Err(SpawnError::ObserverUnavailable);
        }
        Ok(ExitState::Running)
    }
}

#[repr(C)]
struct FixtureProcessIdList {
    number_of_assigned_processes: u32,
    number_of_process_ids_in_list: u32,
    process_ids: [usize; MAX_FIXTURE_JOB_PROCESSES],
}

fn active_process_count(job: HANDLE) -> Result<u32, PlatformError> {
    let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
    // SAFETY: `accounting` has the exact initialized writable layout requested.
    if unsafe {
        QueryInformationJobObject(
            job,
            JobObjectBasicAccountingInformation,
            (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
            size_u32(&accounting),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(last_error("QueryInformationJobObject(accounting)"));
    }
    Ok(accounting.ActiveProcesses)
}

fn query_process_ids(job: HANDLE) -> Result<Vec<u32>, PlatformError> {
    let mut list = FixtureProcessIdList {
        number_of_assigned_processes: 0,
        number_of_process_ids_in_list: 0,
        process_ids: [0; MAX_FIXTURE_JOB_PROCESSES],
    };
    // SAFETY: `list` has the documented header followed by bounded PID storage.
    if unsafe {
        QueryInformationJobObject(
            job,
            JobObjectBasicProcessIdList,
            (&mut list as *mut FixtureProcessIdList).cast(),
            size_u32(&list),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(last_error("QueryInformationJobObject(process list)"));
    }
    if list.number_of_assigned_processes > MAX_FIXTURE_JOB_PROCESSES as u32
        || list.number_of_process_ids_in_list > MAX_FIXTURE_JOB_PROCESSES as u32
    {
        return Err(PlatformError::ProcessCensusOverflow);
    }
    list.process_ids[..list.number_of_process_ids_in_list as usize]
        .iter()
        .copied()
        .map(|pid| u32::try_from(pid).map_err(|_| PlatformError::ProcessCensusOverflow))
        .collect()
}

fn capture_identity(handle: HANDLE, pid: u32) -> Result<NativeIdentity, PlatformError> {
    let mut created = windows_sys::Win32::Foundation::FILETIME::default();
    let mut exited = windows_sys::Win32::Foundation::FILETIME::default();
    let mut kernel = windows_sys::Win32::Foundation::FILETIME::default();
    let mut user = windows_sys::Win32::Foundation::FILETIME::default();
    // SAFETY: the process handle is live and all FILETIME pointers are writable.
    if unsafe {
        windows_sys::Win32::System::Threading::GetProcessTimes(
            handle,
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return Err(last_error("GetProcessTimes"));
    }
    let created_ticks =
        (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
    Ok(NativeIdentity {
        birth_identity: format!("windows:{pid}:{created_ticks}"),
        diagnostic_pid: pid,
    })
}

fn resume_initial_thread(process_handle: &OwnedProcessHandle) -> Result<bool, SpawnError> {
    // SAFETY: TH32CS_SNAPTHREAD takes no pointers and ignores its process ID argument.
    let raw_snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if raw_snapshot == INVALID_HANDLE_VALUE {
        return Err(SpawnError::ContainmentFailed);
    }
    // SAFETY: successful snapshot creation transfers one owned kernel handle.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw_snapshot) };
    let mut entry = THREADENTRY32 {
        dwSize: u32::try_from(std::mem::size_of::<THREADENTRY32>())
            .expect("THREADENTRY32 size fits u32"),
        ..Default::default()
    };
    // SAFETY: `entry` is writable with the required size and `snapshot` is live.
    let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
    while found != 0 {
        if entry.th32OwnerProcessID == process_handle.diagnostic_pid() {
            // SAFETY: this is a thread of the still-suspended owned process;
            // zero explicitly requests a non-inheritable handle.
            let raw_thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if raw_thread.is_null() {
                return Err(SpawnError::ContainmentFailed);
            }
            // SAFETY: OpenThread returned one fresh owned handle.
            let thread_handle = unsafe { OwnedHandle::from_raw_handle(raw_thread) };
            clear_inherit(
                thread_handle.as_raw_handle(),
                "SetHandleInformation(initial thread)",
            )
            .map_err(|_| SpawnError::ContainmentFailed)?;
            let non_inheritable = !is_inheritable(thread_handle.as_raw_handle())
                .map_err(|_| SpawnError::ContainmentFailed)?;
            // SAFETY: this handle grants THREAD_SUSPEND_RESUME and was not resumed before.
            return match unsafe { ResumeThread(thread_handle.as_raw_handle()) } {
                1 => Ok(non_inheritable),
                _ => Err(SpawnError::ContainmentFailed),
            };
        }
        entry.dwSize = u32::try_from(std::mem::size_of::<THREADENTRY32>())
            .expect("THREADENTRY32 size fits u32");
        // SAFETY: `entry` and `snapshot` remain valid for enumeration.
        found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
    }
    Err(SpawnError::ContainmentFailed)
}

fn wait_for_process(handle: HANDLE, timeout: Duration) -> Result<(), PlatformError> {
    match wait_result(handle, duration_millis(timeout))? {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_TIMEOUT => Err(PlatformError::CompletionTimeout),
        _ => Err(last_error("WaitForSingleObject(process)")),
    }
}

fn wait_result(handle: HANDLE, timeout_ms: u32) -> Result<u32, PlatformError> {
    // SAFETY: the caller retains `handle` for the duration of this bounded wait.
    let result = unsafe { WaitForSingleObject(handle, timeout_ms) };
    if result == u32::MAX {
        Err(last_error("WaitForSingleObject"))
    } else {
        Ok(result)
    }
}

fn duration_millis(duration: Duration) -> u32 {
    u32::try_from(duration.as_millis()).unwrap_or(u32::MAX - 1)
}

fn size_u32<T>(value: &T) -> u32 {
    u32::try_from(std::mem::size_of_val(value)).expect("Win32 structure size fits u32")
}

fn clear_inherit(handle: HANDLE, operation: &'static str) -> Result<(), PlatformError> {
    // SAFETY: callers supply a live kernel handle; clearing the documented flag
    // does not invalidate the underlying object.
    if unsafe {
        windows_sys::Win32::Foundation::SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0)
    } == 0
    {
        Err(last_error(operation))
    } else {
        Ok(())
    }
}

fn is_inheritable(handle: HANDLE) -> Result<bool, PlatformError> {
    let mut flags = 0;
    // SAFETY: `handle` is live and `flags` points to writable u32 storage.
    if unsafe { GetHandleInformation(handle, &mut flags) } == 0 {
        Err(last_error("GetHandleInformation"))
    } else {
        Ok(flags & HANDLE_FLAG_INHERIT != 0)
    }
}

fn platform_spawn_error(error: SpawnError) -> PlatformError {
    let operation = match error {
        SpawnError::InvalidTicket => "validate launch specification",
        SpawnError::AdmissionClosed => "admit process root",
        SpawnError::ContainmentFailed => "contain process root",
        SpawnError::IdentityUnavailable => "capture process identity",
        SpawnError::ObserverUnavailable => "observe process root",
    };
    PlatformError::Windows {
        operation,
        source: io::Error::other(error.to_string()),
    }
}

fn last_error(operation: &'static str) -> PlatformError {
    PlatformError::Windows {
        operation,
        source: io::Error::last_os_error(),
    }
}

fn related_marker(root: &Path, suffix: &str) -> PathBuf {
    let mut marker = root.as_os_str().to_owned();
    marker.push(format!(".{suffix}"));
    marker.into()
}

struct BreakawayDirectory(PathBuf);

impl Drop for BreakawayDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
