//! Supervisor-owned launch admission and direct-root observation.

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::Duration;

use thiserror::Error;

use crate::observation::{
    ContainmentMembership, ExitState, ObservationSnapshot, ProcessObservation,
};
use crate::protocol::{
    GenerationId, LaunchTicket, NativeIdentity, PreparedGeneration, ProtocolError, Role,
    SupervisorProtocol,
};
use crate::targets::{self, TargetBehavior};

pub use crate::targets::RELEASE_EXEC_BYTE;

/// Closed launch description assembled for the fixture executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    executable: PathBuf,
    args: Vec<OsString>,
    behavior: TargetBehavior,
    role: Role,
}

impl LaunchSpec {
    /// Builds a fixture-owned executable invocation with no arbitrary command string.
    ///
    /// # Errors
    ///
    /// Returns [`SpawnError::InvalidTicket`] when the path does not name the
    /// fixture executable or the marker path is empty.
    pub fn fixture(
        executable: PathBuf,
        role: Role,
        behavior: TargetBehavior,
        marker: PathBuf,
        lifetime: Duration,
    ) -> Result<Self, SpawnError> {
        if !is_fixture_executable(&executable) || marker.as_os_str().is_empty() {
            return Err(SpawnError::InvalidTicket);
        }
        let args = targets::arguments(role, behavior, &marker, lifetime);
        Ok(Self {
            executable,
            args,
            behavior,
            role,
        })
    }

    /// Returns the validated fixture executable path.
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Returns the closed target argument vector.
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    /// Returns the closed target behavior represented by this launch.
    pub const fn behavior(&self) -> TargetBehavior {
        self.behavior
    }
}

/// Owned handle for a fixture root that is paused behind its release gate.
#[derive(Debug)]
pub struct OwnedProcessHandle {
    child: Child,
    release_gate: Option<ChildStdin>,
}

impl OwnedProcessHandle {
    fn diagnostic_pid(&self) -> u32 {
        self.child.id()
    }

    fn terminate(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        self.release_gate.take();
    }
}

impl Drop for OwnedProcessHandle {
    fn drop(&mut self) {
        self.terminate();
    }
}

/// Typestate value proving a root has identity and an owned paused handle.
#[derive(Debug)]
pub struct PausedRoot {
    /// Native identity captured before target behavior can run.
    pub identity: NativeIdentity,
    /// Retained root process handle and its unopened release gate.
    pub process_handle: OwnedProcessHandle,
}

/// Launch admission failures exposed by the fixture supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SpawnError {
    /// The ticket or closed launch specification was invalid or already used.
    #[error("invalid launch ticket")]
    InvalidTicket,
    /// Owner loss or cleanup already closed launch admission.
    #[error("launch admission is closed")]
    AdmissionClosed,
    /// Paused creation, containment attachment, or release failed.
    #[error("process containment failed")]
    ContainmentFailed,
    /// A native birth identity could not be captured.
    #[error("native process identity is unavailable")]
    IdentityUnavailable,
    /// Registration or independent observation was unavailable.
    #[error("process observer is unavailable")]
    ObserverUnavailable,
}

/// Common process-launch seam implemented by later native platform adapters.
pub trait PlatformAdapter {
    /// Creates a process root whose target behavior is blocked on a release gate.
    fn create_paused_root(&mut self, spec: &LaunchSpec) -> Result<OwnedProcessHandle, SpawnError>;

    /// Captures a birth identity independently of target diagnostics.
    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError>;

    /// Attaches the still-paused root to this generation's containment boundary.
    fn attach_containment(
        &mut self,
        generation: GenerationId,
        process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError>;

    /// Releases only a fully identified, attached, and registered paused root.
    fn release_exec(&mut self, paused_root: PausedRoot) -> Result<OwnedProcessHandle, SpawnError>;

    /// Observes direct-root birth identity and liveness through retained OS state.
    fn observe_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError>;
}

/// Safe portable foundation for the common admission sequence.
///
/// This adapter retains a process handle and uses an exec gate. Its
/// `Registered` membership is deliberately not a native containment proof;
/// Tasks 3 and 4 supply the platform-specific adapters and descendant census.
#[derive(Debug, Default)]
pub struct HostPlatformAdapter;

impl PlatformAdapter for HostPlatformAdapter {
    fn create_paused_root(&mut self, spec: &LaunchSpec) -> Result<OwnedProcessHandle, SpawnError> {
        let mut child = Command::new(&spec.executable)
            .args(&spec.args)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| SpawnError::ContainmentFailed)?;
        let release_gate = child.stdin.take().ok_or_else(|| {
            let _ = child.kill();
            let _ = child.wait();
            SpawnError::ContainmentFailed
        })?;
        Ok(OwnedProcessHandle {
            child,
            release_gate: Some(release_gate),
        })
    }

    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError> {
        let diagnostic_pid = process_handle.diagnostic_pid();
        let birth_identity =
            query_birth_identity(diagnostic_pid).map_err(|_| SpawnError::IdentityUnavailable)?;
        Ok(NativeIdentity {
            birth_identity,
            diagnostic_pid,
        })
    }

    fn attach_containment(
        &mut self,
        _generation: GenerationId,
        _process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError> {
        Ok(ContainmentMembership::Registered)
    }

    fn release_exec(
        &mut self,
        mut paused_root: PausedRoot,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        let Some(mut release_gate) = paused_root.process_handle.release_gate.take() else {
            return Err(SpawnError::ContainmentFailed);
        };
        release_gate
            .write_all(&[RELEASE_EXEC_BYTE])
            .map_err(|_| SpawnError::ContainmentFailed)?;
        drop(release_gate);
        Ok(paused_root.process_handle)
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
        let observed = query_birth_identity(process_handle.diagnostic_pid())
            .map_err(|_| SpawnError::ObserverUnavailable)?;
        if observed != expected_identity.birth_identity {
            return Err(SpawnError::ObserverUnavailable);
        }
        Ok(ExitState::Running)
    }
}

#[derive(Debug)]
struct RegisteredRoot {
    identity: NativeIdentity,
    process_handle: OwnedProcessHandle,
    containment_membership: ContainmentMembership,
}

/// Live launch and observation authority for one prepared generation.
#[derive(Debug)]
pub struct Supervisor<A: PlatformAdapter = HostPlatformAdapter> {
    generation: GenerationId,
    protocol: SupervisorProtocol,
    adapter: A,
    admission_open: bool,
    registered_identities: HashSet<NativeIdentity>,
    roots: Vec<RegisteredRoot>,
    fail_registration_once: bool,
    fail_observer_once: bool,
    mismatch_birth_identity_once: bool,
}

impl Supervisor<HostPlatformAdapter> {
    /// Prepares one generation without starting any target process.
    ///
    /// # Errors
    ///
    /// Returns the Task 1 protocol error when OS capability randomness is
    /// unavailable.
    pub fn prepare(generation: GenerationId) -> Result<(Self, PreparedGeneration), ProtocolError> {
        Self::prepare_with_adapter(generation, HostPlatformAdapter)
    }
}

impl<A: PlatformAdapter> Supervisor<A> {
    /// Prepares a supervisor with a platform adapter and starts no target.
    ///
    /// # Errors
    ///
    /// Returns the Task 1 protocol error when OS capability randomness is
    /// unavailable.
    pub fn prepare_with_adapter(
        generation: GenerationId,
        adapter: A,
    ) -> Result<(Self, PreparedGeneration), ProtocolError> {
        let (protocol, prepared) = SupervisorProtocol::prepare(generation)?;
        Ok((
            Self {
                generation,
                protocol,
                adapter,
                admission_open: true,
                registered_identities: HashSet::new(),
                roots: Vec::new(),
                fail_registration_once: false,
                fail_observer_once: false,
                mismatch_birth_identity_once: false,
            },
            prepared,
        ))
    }

    /// Issues one Task 1 ticket while launch admission remains open.
    ///
    /// # Errors
    ///
    /// Returns [`SpawnError::AdmissionClosed`] after owner loss, otherwise
    /// [`SpawnError::InvalidTicket`] for an invalid closed binding.
    pub fn issue_launch_ticket(
        &mut self,
        role: Role,
        executable_identity: impl Into<String>,
        request_nonce: impl Into<String>,
    ) -> Result<LaunchTicket, SpawnError> {
        if !self.admission_open {
            return Err(SpawnError::AdmissionClosed);
        }
        self.protocol
            .issue_launch_ticket(role, executable_identity, request_nonce)
            .map_err(|_| SpawnError::InvalidTicket)
    }

    /// Runs paused creation, identity capture, containment, registration, then release.
    ///
    /// Every failure after ticket acceptance terminates the still-paused root;
    /// the one-use ticket remains discarded and target behavior does not run.
    ///
    /// # Errors
    ///
    /// Returns a specific [`SpawnError`] for invalid admission, containment,
    /// identity, registration, or observer failures.
    pub fn spawn(
        &mut self,
        ticket: LaunchTicket,
        launch_spec: LaunchSpec,
    ) -> Result<NativeIdentity, SpawnError> {
        if !self.admission_open {
            return Err(SpawnError::AdmissionClosed);
        }
        self.protocol
            .accept_ticket(
                ticket.clone(),
                launch_spec.role,
                launch_spec.role.executable_identity(),
                &ticket.request_nonce,
            )
            .map_err(|_| SpawnError::InvalidTicket)?;

        let mut process_handle = self.adapter.create_paused_root(&launch_spec)?;
        let identity = self.adapter.capture_native_identity(&process_handle);
        let identity = match identity {
            Ok(identity) => identity,
            Err(error) => {
                process_handle.terminate();
                return Err(error);
            }
        };
        let containment_membership = self
            .adapter
            .attach_containment(self.generation, &process_handle);
        let containment_membership = match containment_membership {
            Ok(membership) => membership,
            Err(error) => {
                process_handle.terminate();
                return Err(error);
            }
        };
        if self.take_registration_failure() {
            process_handle.terminate();
            return Err(SpawnError::ObserverUnavailable);
        }
        if !self.registered_identities.insert(identity.clone()) {
            process_handle.terminate();
            return Err(SpawnError::ObserverUnavailable);
        }

        let paused_root = PausedRoot {
            identity: identity.clone(),
            process_handle,
        };
        let process_handle = match self.adapter.release_exec(paused_root) {
            Ok(process_handle) => process_handle,
            Err(error) => {
                self.registered_identities.remove(&identity);
                return Err(error);
            }
        };
        self.roots.push(RegisteredRoot {
            identity: identity.clone(),
            process_handle,
            containment_membership,
        });
        Ok(identity)
    }

    /// Closes admission permanently for this supervisor generation.
    pub fn owner_lost(&mut self) {
        self.admission_open = false;
    }

    /// Independently observes every registered direct root.
    ///
    /// Observer failure and birth-identity mismatch are represented as
    /// `Unresolved`; neither can collapse to an empty process set.
    #[must_use]
    pub fn observe(&mut self) -> ObservationSnapshot {
        let mut unresolved_survivors = Vec::new();
        let fail_observer = std::mem::take(&mut self.fail_observer_once);
        let mismatch_identity = std::mem::take(&mut self.mismatch_birth_identity_once);
        let mut roots = Vec::with_capacity(self.roots.len());

        for (index, root) in self.roots.iter_mut().enumerate() {
            let exit_state = if index == 0 && (fail_observer || mismatch_identity) {
                ExitState::Unresolved
            } else {
                self.adapter
                    .observe_root(&mut root.process_handle, &root.identity)
                    .unwrap_or(ExitState::Unresolved)
            };
            if exit_state == ExitState::Unresolved {
                unresolved_survivors.push(root.identity.clone());
            }
            roots.push(ProcessObservation {
                identity: root.identity.clone(),
                containment_membership: root.containment_membership,
                exit_state,
            });
        }

        ObservationSnapshot {
            generation: self.generation,
            roots,
            descendants: Vec::new(),
            unresolved_survivors,
        }
    }

    /// Causes the next registration attempt to fail before release.
    pub fn inject_registration_failure_once(&mut self) {
        self.fail_registration_once = true;
    }

    /// Causes the next observation to return an unresolved identity.
    pub fn inject_observer_failure_once(&mut self) {
        self.fail_observer_once = true;
    }

    /// Causes the next observation to model a PID/birth-identity mismatch.
    pub fn inject_birth_identity_mismatch_once(&mut self) {
        self.mismatch_birth_identity_once = true;
    }

    fn take_registration_failure(&mut self) -> bool {
        std::mem::take(&mut self.fail_registration_once)
    }
}

fn is_fixture_executable(executable: &Path) -> bool {
    executable
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| {
            name == "process-ownership-fixture" || name == "process-ownership-fixture.exe"
        })
}

#[cfg(unix)]
fn query_birth_identity(pid: u32) -> Result<String, ()> {
    let output = Command::new("ps")
        .args(["-o", "lstart=", "-p"])
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .output()
        .map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }
    let started = String::from_utf8(output.stdout).map_err(|_| ())?;
    let started = started.trim();
    if started.is_empty() {
        return Err(());
    }
    Ok(format!("unix:{pid}:{started}"))
}

#[cfg(windows)]
fn query_birth_identity(pid: u32) -> Result<String, ()> {
    let script = format!("(Get-Process -Id {pid}).StartTime.ToUniversalTime().Ticks");
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .output()
        .map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }
    let started = String::from_utf8(output.stdout).map_err(|_| ())?;
    let started = started.trim();
    if started.is_empty() {
        return Err(());
    }
    Ok(format!("windows:{pid}:{started}"))
}
