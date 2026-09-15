//! Portable Unix ownership candidates used by the process-ownership proof.

use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::net::TcpListener;
use std::os::fd::AsRawFd;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use thiserror::Error;

use crate::observation::{
    ContainmentMembership, ExitState, ObservationSnapshot, ProcessObservation,
};
use crate::protocol::{GenerationId, NativeIdentity};
use crate::supervisor::{
    ExecutableImage, LaunchSpec, OwnedProcessHandle, PausedRoot, PlatformAdapter, SpawnError,
    RELEASE_EXEC_BYTE,
};

use crate::platform::macos_launchd::LaunchdDomain;

const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_CENSUS_PROCESSES: usize = 128;
const MAX_ANCESTRY_DEPTH: usize = 64;

/// Unix ownership operation failures.
#[derive(Debug, Error)]
pub enum PlatformError {
    /// The requested native mechanism is not available on this host.
    #[error("Unix candidate is unsupported on this platform: {candidate}")]
    UnsupportedPlatform { candidate: &'static str },
    /// A native process operation failed.
    #[error("Unix process ownership operation `{operation}` failed: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: io::Error,
    },
    /// Birth identity or ancestry could not be established safely.
    #[error("Unix process identity is unavailable or ambiguous")]
    IdentityUnavailable,
    /// A process no longer matches its retained birth identity.
    #[error("Unix process identity changed while it was owned")]
    PidReuse,
    /// The bounded process census could not be completed.
    #[error("Unix process census exceeded the bound or was incomplete")]
    CensusOverflow,
    /// Owned termination did not produce a complete independent census.
    #[error("Unix owned-process cleanup did not complete before the deadline")]
    CompletionTimeout,
}

/// The Unix candidate mechanisms compared by the fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnixCandidateKind {
    /// Kill the process group created for each admitted root.
    ProcessGroup,
    /// Track roots and descendants using native birth identity and ancestry.
    RegisteredRoot,
    /// Use the macOS service manager as a control candidate.
    Launchd,
}

impl UnixCandidateKind {
    /// Stable evidence spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProcessGroup => "process-group",
            Self::RegisteredRoot => "registered-root",
            Self::Launchd => "launchd",
        }
    }
}

/// A candidate ownership domain for one supervisor generation.
#[derive(Debug)]
pub enum UnixCandidate {
    /// Portable process-group experiment.
    ProcessGroup(ProcessGroupDomain),
    /// Portable registered-root experiment.
    RegisteredRoot(RegisteredRootDomain),
    /// macOS launchd control candidate.
    Launchd(LaunchdDomain),
}

impl UnixCandidate {
    /// Creates a candidate for one generation.
    pub fn for_generation(
        kind: UnixCandidateKind,
        generation: GenerationId,
    ) -> Result<Self, PlatformError> {
        let _ = generation;
        let candidate = match kind {
            UnixCandidateKind::ProcessGroup => {
                "portable Unix process-group cleanup has no kernel-bound identity"
            }
            UnixCandidateKind::RegisteredRoot => {
                "portable Unix registered-root per-process cleanup has no kernel-bound identity"
            }
            UnixCandidateKind::Launchd => "launchd lifecycle is not implemented by this fixture",
        };
        Err(PlatformError::UnsupportedPlatform { candidate })
    }

    /// Returns the candidate's stable evidence spelling.
    #[must_use]
    pub const fn kind(&self) -> UnixCandidateKind {
        match self {
            Self::ProcessGroup(_) => UnixCandidateKind::ProcessGroup,
            Self::RegisteredRoot(_) => UnixCandidateKind::RegisteredRoot,
            Self::Launchd(_) => UnixCandidateKind::Launchd,
        }
    }

    /// Launches, registers, and releases a paused fixture root.
    pub fn launch_paused(&mut self, spec: LaunchSpec) -> Result<PausedRoot, PlatformError> {
        #[cfg(target_os = "macos")]
        {
            let _ = spec;
            Err(PlatformError::UnsupportedPlatform {
                candidate: "macOS handle-based executable launch",
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            match self {
                Self::ProcessGroup(domain) => domain.launch_paused(spec),
                Self::RegisteredRoot(domain) => domain.launch_paused(spec),
                Self::Launchd(domain) => domain.launch_paused(spec),
            }
        }
    }

    /// Independently observes roots and descendants in this generation.
    pub fn observe(&mut self) -> Result<ObservationSnapshot, PlatformError> {
        match self {
            Self::ProcessGroup(domain) => domain.observe(),
            Self::RegisteredRoot(domain) => domain.observe(),
            Self::Launchd(domain) => domain.observe(),
        }
    }

    /// Terminates only the identities admitted to this generation.
    pub fn terminate_owned(&mut self) -> Result<(), PlatformError> {
        match self {
            Self::ProcessGroup(domain) => domain.terminate_owned(),
            Self::RegisteredRoot(domain) => domain.terminate_owned(),
            Self::Launchd(domain) => domain.kill_and_wait(),
        }
    }

    /// Injects one identity mismatch for negative PID-reuse coverage.
    pub fn inject_identity_mismatch_once(&mut self) -> Result<(), PlatformError> {
        match self {
            Self::RegisteredRoot(domain) => {
                domain.identity_mismatch_once = true;
                Ok(())
            }
            Self::ProcessGroup(domain) => {
                domain.identity_mismatch_once = true;
                Ok(())
            }
            Self::Launchd(_) => Err(PlatformError::UnsupportedPlatform {
                candidate: "identity mismatch probe for launchd",
            }),
        }
    }

    /// Injects one observer failure and keeps the result unresolved.
    pub fn inject_observer_failure_once(&mut self) -> Result<(), PlatformError> {
        match self {
            Self::RegisteredRoot(domain) => {
                domain.observer_failure_once = true;
                Ok(())
            }
            Self::ProcessGroup(_) => Err(PlatformError::UnsupportedPlatform {
                candidate: "observer failure probe for process group",
            }),
            Self::Launchd(_) => Err(PlatformError::UnsupportedPlatform {
                candidate: "observer failure probe for launchd",
            }),
        }
    }
}

/// Evidence retained for each registered root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredRootEvidence {
    /// Birth identity captured before release.
    pub identity: NativeIdentity,
    /// Parent PID captured at registration, for diagnostics only.
    pub parent_pid: u32,
    /// Process group captured at registration.
    pub process_group: u32,
    /// Session captured at registration.
    pub session: u32,
    /// Parent ancestry captured at registration.
    pub ancestry: Vec<NativeIdentity>,
}

#[derive(Debug, Clone)]
struct ProcessInfo {
    identity: NativeIdentity,
    parent_pid: u32,
    parent_identity: Option<NativeIdentity>,
    process_group: u32,
    session: u32,
}

#[derive(Debug, Clone)]
struct TrackedRoot {
    evidence: RegisteredRootEvidence,
    known_descendants: HashMap<NativeIdentity, ProcessInfo>,
    was_observed_live: bool,
}

/// Process-group ownership experiment.
#[derive(Debug)]
pub struct ProcessGroupDomain {
    generation: GenerationId,
    roots: Vec<TrackedRoot>,
    pending: Option<ProcessInfo>,
    identity_mismatch_once: bool,
    revalidation_failed: bool,
}

impl ProcessGroupDomain {
    #[cfg(not(target_os = "macos"))]
    fn launch_paused(&mut self, spec: LaunchSpec) -> Result<PausedRoot, PlatformError> {
        let executable =
            ExecutableImage::discover().map_err(|_| PlatformError::IdentityUnavailable)?;
        let handle = self
            .create_paused_root(&executable, &spec)
            .map_err(spawn_error)?;
        let identity = self.capture_native_identity(&handle).map_err(spawn_error)?;
        self.attach_containment(self.generation, &handle)
            .map_err(spawn_error)?;
        let mut root = PausedRoot {
            identity,
            process_handle: handle,
        };
        self.release_exec(&mut root).map_err(spawn_error)?;
        Ok(root)
    }

    fn observe(&mut self) -> Result<ObservationSnapshot, PlatformError> {
        let force_identity_mismatch = std::mem::take(&mut self.identity_mismatch_once);
        if force_identity_mismatch {
            self.revalidation_failed = true;
        }
        observe_roots(
            self.generation,
            &mut self.roots,
            force_identity_mismatch,
            true,
        )
    }

    fn terminate_owned(&mut self) -> Result<(), PlatformError> {
        if self.revalidation_failed {
            return Err(PlatformError::IdentityUnavailable);
        }
        for root in &self.roots {
            let info = process_info(root.evidence.identity.diagnostic_pid)?;
            if info.identity != root.evidence.identity {
                return Err(PlatformError::PidReuse);
            }
            if info.process_group != root.evidence.process_group {
                return Err(PlatformError::IdentityUnavailable);
            }
            if root.evidence.process_group == 0 || root.evidence.process_group == std::process::id()
            {
                return Err(PlatformError::IdentityUnavailable);
            }
            signal_process_group(root.evidence.process_group)?;
        }
        wait_for_complete(|| self.observe())
    }
}

/// Registered-root ownership experiment with PID-reuse-resistant observation.
#[derive(Debug)]
pub struct RegisteredRootDomain {
    generation: GenerationId,
    roots: Vec<TrackedRoot>,
    pending: Option<ProcessInfo>,
    identity_mismatch_once: bool,
    observer_failure_once: bool,
}

impl RegisteredRootDomain {
    /// Returns retained birth identity and ancestry evidence for each root.
    #[must_use]
    pub fn registered_roots(&self) -> Vec<RegisteredRootEvidence> {
        self.roots
            .iter()
            .map(|root| root.evidence.clone())
            .collect()
    }

    #[cfg(not(target_os = "macos"))]
    fn launch_paused(&mut self, spec: LaunchSpec) -> Result<PausedRoot, PlatformError> {
        let executable =
            ExecutableImage::discover().map_err(|_| PlatformError::IdentityUnavailable)?;
        let handle = self
            .create_paused_root(&executable, &spec)
            .map_err(spawn_error)?;
        let identity = self.capture_native_identity(&handle).map_err(spawn_error)?;
        self.attach_containment(self.generation, &handle)
            .map_err(spawn_error)?;
        let mut root = PausedRoot {
            identity,
            process_handle: handle,
        };
        self.release_exec(&mut root).map_err(spawn_error)?;
        Ok(root)
    }

    fn observe(&mut self) -> Result<ObservationSnapshot, PlatformError> {
        if self.observer_failure_once {
            self.observer_failure_once = false;
            let mut snapshot = observe_roots(self.generation, &mut self.roots, false, false)?;
            for root in &snapshot.roots {
                if root.exit_state != ExitState::Exited {
                    snapshot.unresolved_survivors.push(root.identity.clone());
                }
            }
            return Ok(snapshot);
        }
        observe_roots(
            self.generation,
            &mut self.roots,
            std::mem::take(&mut self.identity_mismatch_once),
            false,
        )
    }

    fn terminate_owned(&mut self) -> Result<(), PlatformError> {
        let deadline = Instant::now() + CLEANUP_TIMEOUT;
        loop {
            let snapshot = self.observe()?;
            let mut targets = Vec::new();
            for root in &snapshot.roots {
                if root.exit_state == ExitState::Running {
                    targets.push(root.identity.clone());
                }
            }
            for descendant in &snapshot.descendants {
                if descendant.exit_state == ExitState::Running {
                    targets.push(descendant.identity.clone());
                }
            }
            for identity in targets {
                terminate_identity(&identity)?;
            }
            if let Ok(snapshot) = self.observe() {
                if crate::observation::is_complete(&snapshot) {
                    return Ok(());
                }
            }
            if Instant::now() >= deadline {
                return Err(PlatformError::CompletionTimeout);
            }
            thread::sleep(POLL_INTERVAL);
        }
    }
}

impl PlatformAdapter for UnixCandidate {
    fn secure_control_endpoint(&mut self, endpoint: &TcpListener) -> Result<(), SpawnError> {
        set_close_on_exec(endpoint).map_err(|_| SpawnError::ContainmentFailed)
    }

    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        match self {
            Self::ProcessGroup(domain) => domain.create_paused_root(executable, spec),
            Self::RegisteredRoot(domain) => domain.create_paused_root(executable, spec),
            Self::Launchd(_) => Err(SpawnError::ContainmentFailed),
        }
    }

    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError> {
        let info = process_info(process_handle.diagnostic_pid())
            .map_err(|_| SpawnError::IdentityUnavailable)?;
        match self {
            Self::ProcessGroup(domain) => domain.pending = Some(info.clone()),
            Self::RegisteredRoot(domain) => domain.pending = Some(info.clone()),
            Self::Launchd(_) => return Err(SpawnError::ContainmentFailed),
        }
        Ok(info.identity)
    }

    fn attach_containment(
        &mut self,
        generation: GenerationId,
        process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError> {
        let pid = process_handle.diagnostic_pid();
        match self {
            Self::ProcessGroup(domain) if domain.generation == generation => {
                let info = domain
                    .pending
                    .take()
                    .ok_or(SpawnError::IdentityUnavailable)?;
                if info.process_group == 0 || info.process_group == std::process::id() {
                    return Err(SpawnError::ContainmentFailed);
                }
                domain
                    .roots
                    .push(tracked_root(info, pid).map_err(|_| SpawnError::ObserverUnavailable)?);
                Ok(ContainmentMembership::Confirmed)
            }
            Self::RegisteredRoot(domain) if domain.generation == generation => {
                let info = domain
                    .pending
                    .take()
                    .ok_or(SpawnError::IdentityUnavailable)?;
                domain
                    .roots
                    .push(tracked_root(info, pid).map_err(|_| SpawnError::ObserverUnavailable)?);
                Ok(ContainmentMembership::Confirmed)
            }
            _ => Err(SpawnError::ContainmentFailed),
        }
    }

    fn release_exec(&mut self, paused_root: &mut PausedRoot) -> Result<(), SpawnError> {
        let Some(mut gate) = paused_root.process_handle.release_gate.take() else {
            return Err(SpawnError::ContainmentFailed);
        };
        gate.write_all(&[RELEASE_EXEC_BYTE])
            .map_err(|_| SpawnError::ContainmentFailed)
    }

    fn observe_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError> {
        if process_handle
            .child
            .try_wait()
            .map_err(|_| SpawnError::ObserverUnavailable)?
            .is_some()
        {
            return Ok(ExitState::Exited);
        }
        let observed = process_info(process_handle.diagnostic_pid())
            .map_err(|_| SpawnError::ObserverUnavailable)?;
        if observed.identity != *expected_identity {
            return Err(SpawnError::ObserverUnavailable);
        }
        Ok(ExitState::Running)
    }
}

impl PlatformAdapter for ProcessGroupDomain {
    fn secure_control_endpoint(&mut self, endpoint: &TcpListener) -> Result<(), SpawnError> {
        set_close_on_exec(endpoint).map_err(|_| SpawnError::ContainmentFailed)
    }

    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        create_unix_root(executable, spec, true)
    }

    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError> {
        let info = process_info(process_handle.diagnostic_pid())
            .map_err(|_| SpawnError::IdentityUnavailable)?;
        self.pending = Some(info.clone());
        Ok(info.identity)
    }

    fn attach_containment(
        &mut self,
        generation: GenerationId,
        process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError> {
        if generation != self.generation {
            return Err(SpawnError::ContainmentFailed);
        }
        let info = self.pending.take().ok_or(SpawnError::IdentityUnavailable)?;
        if info.process_group == 0 || info.process_group == std::process::id() {
            return Err(SpawnError::ContainmentFailed);
        }
        self.roots.push(
            tracked_root(info, process_handle.diagnostic_pid())
                .map_err(|_| SpawnError::ObserverUnavailable)?,
        );
        Ok(ContainmentMembership::Confirmed)
    }

    fn release_exec(&mut self, paused_root: &mut PausedRoot) -> Result<(), SpawnError> {
        let Some(mut gate) = paused_root.process_handle.release_gate.take() else {
            return Err(SpawnError::ContainmentFailed);
        };
        gate.write_all(&[RELEASE_EXEC_BYTE])
            .map_err(|_| SpawnError::ContainmentFailed)
    }

    fn observe_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError> {
        observe_root(process_handle, expected_identity)
    }
}

impl PlatformAdapter for RegisteredRootDomain {
    fn secure_control_endpoint(&mut self, endpoint: &TcpListener) -> Result<(), SpawnError> {
        set_close_on_exec(endpoint).map_err(|_| SpawnError::ContainmentFailed)
    }

    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        create_unix_root(executable, spec, false)
    }

    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError> {
        let info = process_info(process_handle.diagnostic_pid())
            .map_err(|_| SpawnError::IdentityUnavailable)?;
        self.pending = Some(info.clone());
        Ok(info.identity)
    }

    fn attach_containment(
        &mut self,
        generation: GenerationId,
        process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError> {
        if generation != self.generation {
            return Err(SpawnError::ContainmentFailed);
        }
        let info = self.pending.take().ok_or(SpawnError::IdentityUnavailable)?;
        self.roots.push(
            tracked_root(info, process_handle.diagnostic_pid())
                .map_err(|_| SpawnError::ObserverUnavailable)?,
        );
        Ok(ContainmentMembership::Confirmed)
    }

    fn release_exec(&mut self, paused_root: &mut PausedRoot) -> Result<(), SpawnError> {
        let Some(mut gate) = paused_root.process_handle.release_gate.take() else {
            return Err(SpawnError::ContainmentFailed);
        };
        gate.write_all(&[RELEASE_EXEC_BYTE])
            .map_err(|_| SpawnError::ContainmentFailed)
    }

    fn observe_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError> {
        observe_root(process_handle, expected_identity)
    }
}

#[cfg(not(target_os = "macos"))]
fn spawn_error(error: SpawnError) -> PlatformError {
    match error {
        SpawnError::ContainmentFailed => PlatformError::Io {
            operation: "create or contain Unix root",
            source: io::Error::other(error.to_string()),
        },
        SpawnError::IdentityUnavailable | SpawnError::ObserverUnavailable => {
            PlatformError::IdentityUnavailable
        }
        SpawnError::InvalidTicket | SpawnError::AdmissionClosed => PlatformError::Io {
            operation: "launch Unix root",
            source: io::Error::other(error.to_string()),
        },
    }
}

fn observe_root(
    process_handle: &mut OwnedProcessHandle,
    expected_identity: &NativeIdentity,
) -> Result<ExitState, SpawnError> {
    if process_handle
        .child
        .try_wait()
        .map_err(|_| SpawnError::ObserverUnavailable)?
        .is_some()
    {
        return Ok(ExitState::Exited);
    }
    let observed = process_info(process_handle.diagnostic_pid())
        .map_err(|_| SpawnError::ObserverUnavailable)?;
    if observed.identity != *expected_identity {
        return Err(SpawnError::ObserverUnavailable);
    }
    Ok(ExitState::Running)
}

fn create_unix_root(
    executable: &ExecutableImage,
    spec: &LaunchSpec,
    process_group: bool,
) -> Result<OwnedProcessHandle, SpawnError> {
    let mut launch = executable_command(executable)?;
    if process_group {
        use std::os::unix::process::CommandExt;
        // SAFETY: this closure runs in the child between fork and exec and only
        // changes the child process group before the release gate is consumed.
        unsafe {
            launch.command.pre_exec(|| {
                if libc::setpgid(0, 0) == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    let mut child = launch
        .command
        .args(spec.args())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| SpawnError::ContainmentFailed)?;
    let release_gate = child.stdin.take();
    let mut process_handle = OwnedProcessHandle {
        child,
        release_gate,
    };
    if process_handle.release_gate.is_none() {
        let _ = process_handle.terminate_bounded(Duration::from_secs(2));
        return Err(SpawnError::ContainmentFailed);
    }
    Ok(process_handle)
}

fn executable_command(
    executable: &ExecutableImage,
) -> Result<crate::supervisor::VerifiedLaunchCommand, SpawnError> {
    executable.command()
}

fn tracked_root(info: ProcessInfo, _pid: u32) -> Result<TrackedRoot, PlatformError> {
    let ancestry = ancestry(info.parent_pid)?;
    Ok(TrackedRoot {
        evidence: RegisteredRootEvidence {
            identity: info.identity.clone(),
            parent_pid: info.parent_pid,
            process_group: info.process_group,
            session: info.session,
            ancestry,
        },
        known_descendants: HashMap::new(),
        was_observed_live: false,
    })
}

fn observe_roots(
    generation: GenerationId,
    roots: &mut [TrackedRoot],
    force_identity_mismatch: bool,
    validate_groups: bool,
) -> Result<ObservationSnapshot, PlatformError> {
    let all_processes = process_census()?;
    let mut observations = Vec::new();
    let mut descendants = Vec::new();
    let mut unresolved_survivors = Vec::new();
    for (index, root) in roots.iter_mut().enumerate() {
        let expected = if force_identity_mismatch && index == 0 {
            NativeIdentity {
                birth_identity: format!("{}:mismatch", root.evidence.identity.birth_identity),
                diagnostic_pid: root.evidence.identity.diagnostic_pid,
            }
        } else {
            root.evidence.identity.clone()
        };
        let current = all_processes.iter().find(|process| {
            process.identity.diagnostic_pid == root.evidence.identity.diagnostic_pid
        });
        let root_state = match current {
            Some(process)
                if process.identity == expected
                    && (!validate_groups
                        || process.process_group == root.evidence.process_group) =>
            {
                root.was_observed_live = true;
                ExitState::Running
            }
            Some(_) => {
                unresolved_survivors.push(root.evidence.identity.clone());
                ExitState::Unresolved
            }
            None if root.was_observed_live => ExitState::Exited,
            None => ExitState::Unresolved,
        };
        if root_state == ExitState::Unresolved {
            unresolved_survivors.push(root.evidence.identity.clone());
        }
        observations.push(ProcessObservation {
            identity: root.evidence.identity.clone(),
            containment_membership: ContainmentMembership::Confirmed,
            exit_state: root_state,
        });

        // A mismatched or otherwise unresolved root cannot authorize a
        // descendant traversal. The retained root identity is the trust root.
        if root_state != ExitState::Running {
            continue;
        }

        let root_pid = root.evidence.identity.diagnostic_pid;
        let root_identity = root.evidence.identity.clone();
        let mut descendants_by_pid: HashMap<u32, ProcessInfo> = HashMap::new();
        let mut changed = true;
        while changed {
            changed = false;
            for process in &all_processes {
                if process.identity.diagnostic_pid == root_pid {
                    continue;
                }
                let parent_known = (process.parent_pid == root_pid
                    && process.parent_identity.as_ref() == Some(&root_identity))
                    || descendants_by_pid.values().any(|parent| {
                        process.parent_pid == parent.identity.diagnostic_pid
                            && process.parent_identity.as_ref() == Some(&parent.identity)
                    });
                if parent_known
                    && descendants_by_pid
                        .insert(process.identity.diagnostic_pid, process.clone())
                        .is_none()
                {
                    changed = true;
                }
            }
        }
        for process in descendants_by_pid.values() {
            root.known_descendants
                .insert(process.identity.clone(), process.clone());
            descendants.push(ProcessObservation {
                identity: process.identity.clone(),
                containment_membership: ContainmentMembership::Confirmed,
                exit_state: ExitState::Running,
            });
        }
        for process in root.known_descendants.values() {
            if let Some(current) = all_processes
                .iter()
                .find(|current| current.identity.diagnostic_pid == process.identity.diagnostic_pid)
            {
                if current.identity != process.identity
                    || current.parent_identity != process.parent_identity
                {
                    unresolved_survivors.push(process.identity.clone());
                    descendants.push(ProcessObservation {
                        identity: process.identity.clone(),
                        containment_membership: ContainmentMembership::Confirmed,
                        exit_state: ExitState::Unresolved,
                    });
                }
            } else {
                descendants.push(ProcessObservation {
                    identity: process.identity.clone(),
                    containment_membership: ContainmentMembership::Confirmed,
                    exit_state: ExitState::Exited,
                });
            }
        }
    }
    if observations.len() + descendants.len() > MAX_CENSUS_PROCESSES {
        return Err(PlatformError::CensusOverflow);
    }
    Ok(ObservationSnapshot {
        generation,
        roots: observations,
        descendants,
        unidentified_roots: Vec::new(),
        unresolved_survivors,
    })
}

fn wait_for_complete(
    mut observe: impl FnMut() -> Result<ObservationSnapshot, PlatformError>,
) -> Result<(), PlatformError> {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    loop {
        match observe() {
            Ok(snapshot) if crate::observation::is_complete(&snapshot) => return Ok(()),
            Ok(_) => {}
            Err(error) => return Err(error),
        }
        if Instant::now() >= deadline {
            return Err(PlatformError::CompletionTimeout);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn terminate_identity(identity: &NativeIdentity) -> Result<(), PlatformError> {
    let Some(process) = process_census()?
        .into_iter()
        .find(|process| process.identity.diagnostic_pid == identity.diagnostic_pid)
    else {
        return Ok(());
    };
    if process.identity != *identity {
        return Err(PlatformError::PidReuse);
    }
    // SAFETY: identity was matched against a live process census immediately
    // before the signal; a reused PID is rejected by the birth-identity check.
    if unsafe { libc::kill(identity.diagnostic_pid as i32, libc::SIGKILL) } == -1 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(PlatformError::Io {
                operation: "terminate registered Unix process",
                source: error,
            });
        }
    }
    Ok(())
}

fn signal_process_group(process_group: u32) -> Result<(), PlatformError> {
    // SAFETY: a negative PID targets the explicitly captured process group.
    if unsafe { libc::kill(-(process_group as i32), libc::SIGKILL) } == -1 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(PlatformError::Io {
                operation: "terminate Unix process group",
                source: error,
            });
        }
    }
    Ok(())
}

fn ancestry(mut pid: u32) -> Result<Vec<NativeIdentity>, PlatformError> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    while pid != 0 {
        if !seen.insert(pid) {
            return Err(PlatformError::IdentityUnavailable);
        }
        if result.len() >= MAX_ANCESTRY_DEPTH {
            return Err(PlatformError::CensusOverflow);
        }
        let info = process_info(pid)?;
        result.push(info.identity);
        pid = info.parent_pid;
    }
    Ok(result)
}

fn set_close_on_exec(listener: &TcpListener) -> io::Result<()> {
    let fd = listener.as_raw_fd();
    // SAFETY: fd belongs to the live listener borrowed for this call.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fd and flags came from the preceding successful fcntl call.
    if unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn process_census() -> Result<Vec<ProcessInfo>, PlatformError> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir("/proc").map_err(|source| PlatformError::Io {
        operation: "enumerate Linux processes",
        source,
    })? {
        let entry = entry.map_err(|source| PlatformError::Io {
            operation: "read Linux process entry",
            source,
        })?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        result.push(process_info(pid)?);
    }
    populate_parent_identities(&mut result)?;
    Ok(result)
}

#[cfg(target_os = "macos")]
fn process_census() -> Result<Vec<ProcessInfo>, PlatformError> {
    let mut capacity = 4096_usize;
    let pids = loop {
        let mut pids = vec![0_i32; capacity];
        // SAFETY: pids points to writable storage and the size is expressed in bytes.
        let count = unsafe {
            libc::proc_listallpids(
                pids.as_mut_ptr().cast(),
                i32::try_from(pids.len() * std::mem::size_of::<i32>())
                    .map_err(|_| PlatformError::CensusOverflow)?,
            )
        };
        if count < 0 {
            return Err(PlatformError::Io {
                operation: "enumerate macOS processes",
                source: io::Error::last_os_error(),
            });
        }
        let count = usize::try_from(count).map_err(|_| PlatformError::CensusOverflow)?;
        if count < capacity {
            break pids.into_iter().take(count).collect::<Vec<_>>();
        }
        capacity = capacity
            .checked_mul(2)
            .ok_or(PlatformError::CensusOverflow)?;
        if capacity > MAX_CENSUS_PROCESSES * 64 {
            return Err(PlatformError::CensusOverflow);
        }
    };
    let mut result = Vec::new();
    for pid in pids.into_iter().filter(|pid| *pid > 0) {
        result.push(process_info(
            u32::try_from(pid).map_err(|_| PlatformError::IdentityUnavailable)?,
        )?);
    }
    populate_parent_identities(&mut result)?;
    Ok(result)
}

fn populate_parent_identities(processes: &mut [ProcessInfo]) -> Result<(), PlatformError> {
    let identities = processes
        .iter()
        .map(|process| (process.identity.diagnostic_pid, process.identity.clone()))
        .collect::<HashMap<_, _>>();
    for process in processes {
        process.parent_identity = if process.parent_pid == 0 {
            None
        } else {
            Some(
                identities
                    .get(&process.parent_pid)
                    .ok_or(PlatformError::IdentityUnavailable)?
                    .clone(),
            )
        };
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn process_info(pid: u32) -> Result<ProcessInfo, PlatformError> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|_| PlatformError::IdentityUnavailable)?;
    let (_, fields) = stat
        .rsplit_once(')')
        .ok_or(PlatformError::IdentityUnavailable)?;
    let fields = fields.split_whitespace().collect::<Vec<_>>();
    let start_ticks = fields
        .get(19)
        .ok_or(PlatformError::IdentityUnavailable)?
        .parse::<u64>()
        .map_err(|_| PlatformError::IdentityUnavailable)?;
    let parent_pid = fields
        .get(1)
        .ok_or(PlatformError::IdentityUnavailable)?
        .parse::<u32>()
        .map_err(|_| PlatformError::IdentityUnavailable)?;
    let process_group = fields
        .get(2)
        .ok_or(PlatformError::IdentityUnavailable)?
        .parse::<u32>()
        .map_err(|_| PlatformError::IdentityUnavailable)?;
    let session = fields
        .get(3)
        .ok_or(PlatformError::IdentityUnavailable)?
        .parse::<u32>()
        .map_err(|_| PlatformError::IdentityUnavailable)?;
    Ok(ProcessInfo {
        identity: NativeIdentity {
            birth_identity: format!("linux:{pid}:{start_ticks}"),
            diagnostic_pid: pid,
        },
        parent_pid,
        parent_identity: None,
        process_group,
        session,
    })
}

#[cfg(target_os = "macos")]
fn process_info(pid: u32) -> Result<ProcessInfo, PlatformError> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    // SAFETY: info is writable storage of the exact proc_pidinfo size.
    let bytes = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            std::mem::size_of::<libc::proc_bsdinfo>() as i32,
        )
    };
    if bytes != std::mem::size_of::<libc::proc_bsdinfo>() as i32 {
        return Err(PlatformError::IdentityUnavailable);
    }
    // SAFETY: proc_pidinfo returned the full structure size.
    let info = unsafe { info.assume_init() };
    let session = unsafe { libc::getsid(pid as i32) };
    if session < 0 {
        return Err(PlatformError::Io {
            operation: "read macOS process session",
            source: std::io::Error::last_os_error(),
        });
    }
    Ok(ProcessInfo {
        identity: NativeIdentity {
            birth_identity: format!(
                "macos:{pid}:{}:{}",
                info.pbi_start_tvsec, info.pbi_start_tvusec
            ),
            diagnostic_pid: pid,
        },
        parent_pid: info.pbi_ppid,
        parent_identity: None,
        process_group: info.pbi_pgid,
        session: u32::try_from(session).map_err(|_| PlatformError::IdentityUnavailable)?,
    })
}

impl Drop for ProcessGroupDomain {
    fn drop(&mut self) {
        let _ = self.terminate_owned();
    }
}

impl Drop for RegisteredRootDomain {
    fn drop(&mut self) {
        let _ = self.terminate_owned();
    }
}
