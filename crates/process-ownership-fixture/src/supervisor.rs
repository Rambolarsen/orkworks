//! Supervisor-owned launch admission and direct-root observation.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::observation::{
    is_complete, ContainmentMembership, ExitState, ObservationSnapshot, ProcessObservation,
    UnidentifiedProcessObservation,
};
use crate::protocol::{
    AdoptionRequest, AuthenticatedRendezvousReply, CleanupResult, CompleteExitReceipt,
    GenerationId, LaunchTicket, NativeIdentity, PreparedGeneration, ProtocolError,
    RendezvousRecord, Role, SupervisorProtocol,
};
use crate::targets::{self, TargetBehavior};

pub use crate::targets::RELEASE_EXEC_BYTE;

const GRACEFUL_CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
const OWNED_TERMINATION_TIMEOUT: Duration = Duration::from_secs(5);
const CLEANUP_POLL_INTERVAL: Duration = Duration::from_millis(10);
static LIVE_INFERENCE_GENERATIONS: OnceLock<Mutex<HashSet<GenerationId>>> = OnceLock::new();

/// Closed launch description assembled for the fixture executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    args: Vec<OsString>,
    behavior: TargetBehavior,
    role: Role,
}

impl LaunchSpec {
    /// Builds a fixture-owned executable invocation with no arbitrary command string.
    ///
    /// # Errors
    ///
    /// Returns [`SpawnError::InvalidTicket`] when the marker path is empty.
    pub fn fixture(
        role: Role,
        behavior: TargetBehavior,
        marker: PathBuf,
        lifetime: Duration,
    ) -> Result<Self, SpawnError> {
        if marker.as_os_str().is_empty() {
            return Err(SpawnError::InvalidTicket);
        }
        let args = targets::arguments(role, behavior, &marker, lifetime);
        Ok(Self {
            args,
            behavior,
            role,
        })
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
    pub(crate) child: Child,
    pub(crate) release_gate: Option<ChildStdin>,
}

impl OwnedProcessHandle {
    pub(crate) fn diagnostic_pid(&self) -> u32 {
        self.child.id()
    }

    /// Closes the pre-exec release gate without sending the release byte.
    pub fn close_release_gate(&mut self) {
        self.release_gate.take();
    }

    pub(crate) fn terminate_bounded(&mut self, timeout: Duration) -> Result<(), SpawnError> {
        self.close_release_gate();
        if self
            .child
            .try_wait()
            .map_err(|_| SpawnError::ObserverUnavailable)?
            .is_some()
        {
            return Ok(());
        }

        let kill_error = self.child.kill().err();
        let deadline = Instant::now() + timeout;
        loop {
            if self
                .child
                .try_wait()
                .map_err(|_| SpawnError::ObserverUnavailable)?
                .is_some()
            {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(if kill_error.is_some() {
                    SpawnError::ContainmentFailed
                } else {
                    SpawnError::ObserverUnavailable
                });
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn observe_exit(&mut self) -> Result<ExitState, SpawnError> {
        match self
            .child
            .try_wait()
            .map_err(|_| SpawnError::ObserverUnavailable)?
        {
            Some(_) => Ok(ExitState::Exited),
            None => Ok(ExitState::Running),
        }
    }
}

impl Drop for OwnedProcessHandle {
    fn drop(&mut self) {
        let _ = self.terminate_bounded(Duration::from_secs(2));
    }
}

/// Supervisor-owned executable image verified during generation preparation.
#[derive(Debug)]
pub struct ExecutableImage {
    path: PathBuf,
    file: File,
    digest: [u8; 32],
    ticket_binding: String,
    #[cfg(target_os = "linux")]
    staging: Option<PartialStaging>,
}

pub(crate) struct VerifiedLaunchCommand {
    pub(crate) command: Command,
    _image_handle: Option<File>,
}

impl ExecutableImage {
    pub(crate) fn discover() -> Result<Self, ProtocolError> {
        let path = Self::discover_path()?;
        Self::from_trusted_candidate(path)
    }

    fn from_trusted_candidate(candidate: PathBuf) -> Result<Self, ProtocolError> {
        let trusted = Self::discover_path()?;
        let trusted_file = open_executable(&trusted).map_err(|_| ProtocolError::InvalidValue {
            field: "fixture executable",
        })?;
        let trusted_digest =
            hash_executable(&trusted_file).map_err(|_| ProtocolError::InvalidValue {
                field: "fixture executable",
            })?;
        let candidate = candidate
            .canonicalize()
            .map_err(|_| ProtocolError::InvalidValue {
                field: "fixture executable",
            })?;
        let candidate_file =
            open_executable(&candidate).map_err(|_| ProtocolError::InvalidValue {
                field: "fixture executable",
            })?;
        let candidate_digest =
            hash_executable(&candidate_file).map_err(|_| ProtocolError::InvalidValue {
                field: "fixture executable",
            })?;
        if candidate_digest != trusted_digest {
            return Err(ProtocolError::InvalidValue {
                field: "fixture executable",
            });
        }
        #[cfg(target_os = "linux")]
        let (path, file, mut staging) =
            prepare_launch_image(&candidate, candidate_file).map_err(|_| {
                ProtocolError::InvalidValue {
                    field: "fixture executable",
                }
            })?;
        #[cfg(target_os = "macos")]
        let (path, file) = prepare_launch_image(&candidate, candidate_file).map_err(|_| {
            ProtocolError::InvalidValue {
                field: "fixture executable",
            }
        })?;
        #[cfg(windows)]
        let (path, file) = prepare_launch_image(&candidate, candidate_file).map_err(|_| {
            ProtocolError::InvalidValue {
                field: "fixture executable",
            }
        })?;
        let digest = hash_executable(&file).map_err(|_| ProtocolError::InvalidValue {
            field: "fixture executable",
        })?;
        if digest != candidate_digest {
            return Err(ProtocolError::InvalidValue {
                field: "fixture executable",
            });
        }
        #[cfg(target_os = "linux")]
        staging.commit();
        let ticket_binding = image_ticket_binding(&path, &digest);
        Ok(Self {
            #[cfg(target_os = "linux")]
            staging: Some(staging),
            path,
            file,
            digest,
            ticket_binding,
        })
    }

    fn discover_path() -> Result<PathBuf, ProtocolError> {
        let current = std::env::current_exe().map_err(|_| ProtocolError::InvalidValue {
            field: "fixture executable",
        })?;
        let candidate = if is_fixture_executable(&current) {
            current
        } else {
            let name = if cfg!(windows) {
                "process-ownership-fixture.exe"
            } else {
                "process-ownership-fixture"
            };
            current
                .parent()
                .and_then(Path::parent)
                .ok_or(ProtocolError::InvalidValue {
                    field: "fixture executable",
                })?
                .join(name)
        };
        candidate
            .canonicalize()
            .map_err(|_| ProtocolError::InvalidValue {
                field: "fixture executable",
            })
    }

    fn executable_identity(&self, role: Role) -> &'static str {
        role.executable_identity()
    }

    fn bind_request_nonce(&self, request_nonce: &str) -> String {
        format!("image:{}:{request_nonce}", self.ticket_binding)
    }

    fn ticket_matches(&self, ticket: &LaunchTicket, role: Role) -> bool {
        image_ticket_binding(&self.path, &self.digest) == self.ticket_binding
            && ticket.executable_identity == self.executable_identity(role)
            && ticket
                .request_nonce
                .starts_with(&format!("image:{}:", self.ticket_binding))
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn command(&self) -> Result<VerifiedLaunchCommand, SpawnError> {
        use std::os::fd::AsRawFd;
        use std::os::unix::process::CommandExt;

        let image_handle = self.try_clone_file()?;
        let descriptor = image_handle.as_raw_fd();
        // SAFETY: `descriptor` belongs to the live cloned file handle.
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
        if flags == -1 {
            return Err(SpawnError::ContainmentFailed);
        }
        let mut command = Command::new(format!("/proc/self/fd/{descriptor}"));
        // Keep the descriptor close-on-exec in the parent. Clear it only in
        // this child after fork, eliminating the concurrent launch race where
        // another child could inherit a sibling's verified image descriptor.
        // SAFETY: the closure runs before exec and uses only async-signal-safe
        // libc calls; `image_handle` remains live through Command::spawn.
        unsafe {
            command.pre_exec(move || {
                if libc::fcntl(descriptor, libc::F_SETFD, flags & !libc::FD_CLOEXEC) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        Ok(VerifiedLaunchCommand {
            command,
            _image_handle: Some(image_handle),
        })
    }

    #[cfg(windows)]
    pub(crate) fn command(&self) -> Result<VerifiedLaunchCommand, SpawnError> {
        Ok(VerifiedLaunchCommand {
            command: Command::new(&self.path),
            _image_handle: None,
        })
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    pub(crate) fn command(&self) -> Result<VerifiedLaunchCommand, SpawnError> {
        Err(SpawnError::ContainmentFailed)
    }

    /// Returns the canonical pathname retained for diagnostics and native adapters.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Clones the verified executable image handle for a native adapter.
    ///
    /// # Errors
    ///
    /// Returns [`SpawnError::ContainmentFailed`] if the OS cannot duplicate the handle.
    pub fn try_clone_file(&self) -> Result<File, SpawnError> {
        self.file
            .try_clone()
            .map_err(|_| SpawnError::ContainmentFailed)
    }
}

impl Drop for ExecutableImage {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some(staging) = &self.staging {
            cleanup_staging_directory(&staging.directory, &staging.path);
        }
    }
}

fn open_executable(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    options.share_mode(0x0000_0001);
    options.open(path)
}

#[cfg(target_os = "linux")]
fn prepare_launch_image(
    _source_path: &Path,
    source: File,
) -> std::io::Result<(PathBuf, File, PartialStaging)> {
    let staging = PartialStaging::create()?;
    let path = staging.path.clone();
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o500)
        .open(&path)?;
    let mut source = source;
    source.rewind()?;
    std::io::copy(&mut source, &mut destination)?;
    destination.sync_all()?;
    drop(destination);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o500))?;
    let file = open_executable(&path)?;
    std::fs::set_permissions(&staging.directory, std::fs::Permissions::from_mode(0o500))?;
    Ok((path, file, staging))
}

#[cfg(target_os = "macos")]
fn prepare_launch_image(source_path: &Path, source: File) -> std::io::Result<(PathBuf, File)> {
    Ok((source_path.to_path_buf(), source))
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct PartialStaging {
    directory: PathBuf,
    path: PathBuf,
    committed: bool,
}

#[cfg(target_os = "linux")]
impl PartialStaging {
    fn create() -> std::io::Result<Self> {
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|error| std::io::Error::other(error.to_string()))?;
        let directory = std::env::temp_dir().join(format!(
            "orkworks-process-ownership-image-{}-{}",
            std::process::id(),
            hex::encode(random)
        ));
        std::fs::create_dir(&directory)?;
        let path = directory.join("process-ownership-fixture");
        let staging = Self {
            directory,
            path,
            committed: false,
        };
        std::fs::set_permissions(&staging.directory, std::fs::Permissions::from_mode(0o700))?;
        Ok(staging)
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

#[cfg(target_os = "linux")]
impl Drop for PartialStaging {
    fn drop(&mut self) {
        if !self.committed {
            cleanup_staging_directory(&self.directory, &self.path);
        }
    }
}

#[cfg(target_os = "linux")]
fn cleanup_staging_directory(directory: &Path, path: &Path) {
    let _ = std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700));
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir(directory);
}

#[cfg(windows)]
fn prepare_launch_image(source_path: &Path, source: File) -> std::io::Result<(PathBuf, File)> {
    Ok((source_path.to_path_buf(), source))
}

fn hash_executable(file: &File) -> Result<[u8; 32], ()> {
    let mut file = file.try_clone().map_err(|_| ())?;
    file.rewind().map_err(|_| ())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| ())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

fn image_ticket_binding(path: &Path, digest: &[u8; 32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(digest);
    hex::encode(hasher.finalize())
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

/// Result returned when an authenticated relaunch query is evaluated.
pub type AdoptionResult = Result<CompleteExitReceipt, ProtocolError>;

/// Client-side adoption verifier for a persisted rendezvous locator.
#[derive(Debug, Clone)]
pub struct RendezvousClient {
    prepared: PreparedGeneration,
    authenticated_response: Option<AuthenticatedRendezvousReply>,
}

impl RendezvousClient {
    /// Creates a client with no live response; adoption remains unresolved.
    #[must_use]
    pub fn new(prepared: PreparedGeneration) -> Self {
        Self {
            prepared,
            authenticated_response: None,
        }
    }

    /// Creates a client from a response obtained through live authenticated IPC.
    #[must_use]
    pub fn from_authenticated_response(
        prepared: PreparedGeneration,
        response: AuthenticatedRendezvousReply,
    ) -> Self {
        Self {
            prepared,
            authenticated_response: Some(response),
        }
    }

    /// Adopts only a complete-exit receipt authenticated for the supplied record.
    pub fn adopt(&self, record: &RendezvousRecord) -> AdoptionResult {
        if record.generation != self.prepared.record.generation {
            return Err(ProtocolError::WrongGeneration {
                expected: self.prepared.record.generation,
                actual: record.generation,
            });
        }
        if record.nonce != self.prepared.record.nonce {
            return Err(ProtocolError::StaleRendezvous);
        }
        let Some(response) = self.authenticated_response.as_ref() else {
            return Err(ProtocolError::AdoptionUnresolved(
                "live rendezvous response is missing".to_owned(),
            ));
        };
        let request = AdoptionRequest {
            generation: self.prepared.record.generation,
            rendezvous_nonce: self.prepared.record.nonce.clone(),
            challenge: response.challenge.clone(),
        };
        self.prepared
            .verify_adoption_response(&request, Some(response))
    }
}

/// Common process-launch seam implemented by later native platform adapters.
pub trait PlatformAdapter {
    /// Makes the supervisor's IPC endpoint non-inheritable before any launch.
    fn secure_control_endpoint(&mut self, _endpoint: &TcpListener) -> Result<(), SpawnError> {
        Ok(())
    }

    /// Creates a process root whose target behavior is blocked on a release gate.
    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError>;

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
    fn release_exec(&mut self, paused_root: &mut PausedRoot) -> Result<(), SpawnError>;

    /// Closes the release gate and independently confirms root exit within a bound.
    fn terminate_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        timeout: Duration,
    ) -> Result<(), SpawnError> {
        process_handle.terminate_bounded(timeout)
    }

    /// Observes direct-root birth identity and liveness through retained OS state.
    fn observe_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError>;

    /// Observes only retained-handle exit when native birth identity was unavailable.
    fn observe_unidentified_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
    ) -> Result<ExitState, SpawnError> {
        process_handle.observe_exit()
    }
}

/// Safe portable foundation for the common admission sequence.
///
/// This adapter retains a process handle and uses an exec gate. Its
/// `Confirmed` membership is deliberately not a native containment proof;
/// Tasks 3 and 4 supply the platform-specific adapters and descendant census.
#[derive(Debug, Default)]
pub struct HostPlatformAdapter;

impl PlatformAdapter for HostPlatformAdapter {
    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        let mut launch = executable.command()?;
        let mut child = launch
            .command
            .args(&spec.args)
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
        Ok(())
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
    role: Role,
    process_handle: OwnedProcessHandle,
    containment_membership: ContainmentMembership,
    unresolved_until_exit: bool,
}

#[derive(Debug)]
struct UnidentifiedRoot {
    process_handle: OwnedProcessHandle,
}

/// Live launch and observation authority for one prepared generation.
#[derive(Debug)]
pub struct Supervisor<A: PlatformAdapter = HostPlatformAdapter> {
    generation: GenerationId,
    protocol: SupervisorProtocol,
    adapter: A,
    executable: ExecutableImage,
    control_endpoint: Option<TcpListener>,
    admission_open: bool,
    registered_identities: HashSet<NativeIdentity>,
    roots: Vec<RegisteredRoot>,
    unidentified_roots: Vec<UnidentifiedRoot>,
    fail_registration_once: bool,
    cleanup_started: bool,
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

    /// Prepares only when `candidate` matches the supervisor-discovered fixture image.
    ///
    /// This fixture-only hook provides negative coverage for executable substitution;
    /// callers still cannot select a launch executable.
    pub fn prepare_with_executable(
        generation: GenerationId,
        candidate: PathBuf,
    ) -> Result<(Self, PreparedGeneration), ProtocolError> {
        let executable = ExecutableImage::from_trusted_candidate(candidate)?;
        Self::prepare_with_authority(generation, HostPlatformAdapter, executable)
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
        let executable = ExecutableImage::discover()?;
        Self::prepare_with_authority(generation, adapter, executable)
    }

    fn prepare_with_authority(
        generation: GenerationId,
        adapter: A,
        executable: ExecutableImage,
    ) -> Result<(Self, PreparedGeneration), ProtocolError> {
        let (protocol, prepared) = SupervisorProtocol::prepare(generation)?;
        let control_endpoint =
            TcpListener::bind(("127.0.0.1", 0)).map_err(|_| ProtocolError::InvalidValue {
                field: "control endpoint",
            })?;
        let mut adapter = adapter;
        adapter
            .secure_control_endpoint(&control_endpoint)
            .map_err(|_| ProtocolError::InvalidValue {
                field: "control endpoint",
            })?;
        Ok((
            Self {
                generation,
                protocol,
                adapter,
                executable,
                control_endpoint: Some(control_endpoint),
                admission_open: true,
                registered_identities: HashSet::new(),
                roots: Vec::new(),
                unidentified_roots: Vec::new(),
                fail_registration_once: false,
                cleanup_started: false,
            },
            prepared,
        ))
    }

    /// Issues one Task 1 ticket while launch admission remains open.
    ///
    /// # Errors
    ///
    /// The executable identity and physical-image binding are derived from the
    /// supervisor-owned image; the caller supplies only its request nonce.
    ///
    /// Returns [`SpawnError::AdmissionClosed`] after owner loss, otherwise
    /// [`SpawnError::InvalidTicket`] for an invalid closed binding.
    pub fn issue_launch_ticket(
        &mut self,
        role: Role,
        request_nonce: impl Into<String>,
    ) -> Result<LaunchTicket, SpawnError> {
        if !self.admission_open
            || self.cleanup_started
            || (role == Role::Inference && inference_generation_is_blocked(self.generation))
        {
            return Err(SpawnError::AdmissionClosed);
        }
        let request_nonce = request_nonce.into();
        self.protocol
            .issue_launch_ticket(
                role,
                self.executable.executable_identity(role),
                self.executable.bind_request_nonce(&request_nonce),
            )
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
        if !self.admission_open || self.cleanup_started {
            return Err(SpawnError::AdmissionClosed);
        }
        if !self.executable.ticket_matches(&ticket, launch_spec.role) {
            return Err(SpawnError::InvalidTicket);
        }
        self.protocol
            .accept_ticket(
                ticket.clone(),
                launch_spec.role,
                self.executable.executable_identity(launch_spec.role),
                &ticket.request_nonce,
            )
            .map_err(|_| SpawnError::InvalidTicket)?;

        let mut process_handle = self
            .adapter
            .create_paused_root(&self.executable, &launch_spec)?;
        if !self.admission_open || self.cleanup_started {
            let _ = self
                .adapter
                .terminate_root(&mut process_handle, Duration::from_secs(2));
            return Err(SpawnError::AdmissionClosed);
        }
        let identity = self.adapter.capture_native_identity(&process_handle);
        let identity = match identity {
            Ok(identity) => identity,
            Err(error) => {
                if self
                    .adapter
                    .terminate_root(&mut process_handle, Duration::from_secs(2))
                    .is_err()
                {
                    self.unidentified_roots
                        .push(UnidentifiedRoot { process_handle });
                }
                return Err(error);
            }
        };
        let containment_membership = self
            .adapter
            .attach_containment(self.generation, &process_handle);
        let containment_membership = match containment_membership {
            Ok(ContainmentMembership::Confirmed) => ContainmentMembership::Confirmed,
            Ok(ContainmentMembership::Unresolved) => {
                self.cleanup_failed_admission(
                    identity,
                    launch_spec.role,
                    process_handle,
                    ContainmentMembership::Unresolved,
                    false,
                );
                return Err(SpawnError::ContainmentFailed);
            }
            Err(error) => {
                self.cleanup_failed_admission(
                    identity,
                    launch_spec.role,
                    process_handle,
                    ContainmentMembership::Unresolved,
                    false,
                );
                return Err(error);
            }
        };
        if !self.admission_open || self.cleanup_started {
            self.cleanup_failed_admission(
                identity,
                launch_spec.role,
                process_handle,
                containment_membership,
                false,
            );
            return Err(SpawnError::AdmissionClosed);
        }
        if self.take_registration_failure() {
            self.cleanup_failed_admission(
                identity,
                launch_spec.role,
                process_handle,
                containment_membership,
                false,
            );
            return Err(SpawnError::ObserverUnavailable);
        }
        if !self.registered_identities.insert(identity.clone()) {
            self.cleanup_failed_admission(
                identity,
                launch_spec.role,
                process_handle,
                containment_membership,
                false,
            );
            return Err(SpawnError::ObserverUnavailable);
        }

        let mut paused_root = PausedRoot {
            identity: identity.clone(),
            process_handle,
        };
        if let Err(error) = self.adapter.release_exec(&mut paused_root) {
            self.cleanup_failed_admission(
                identity,
                launch_spec.role,
                paused_root.process_handle,
                containment_membership,
                true,
            );
            return Err(error);
        }
        self.roots.push(RegisteredRoot {
            identity: identity.clone(),
            role: launch_spec.role,
            process_handle: paused_root.process_handle,
            containment_membership,
            unresolved_until_exit: false,
        });
        Ok(identity)
    }

    /// Closes admission permanently for this supervisor generation.
    pub fn owner_lost(&mut self) {
        self.admission_open = false;
        self.control_endpoint.take();
        if self
            .roots
            .iter_mut()
            .any(|root| root.role == Role::Inference && root_is_live(root))
        {
            mark_inference_generation_live(self.generation);
        }
    }

    /// Returns an authenticated adoption response from the live supervisor.
    pub fn answer_adoption(
        &mut self,
        request: &AdoptionRequest,
    ) -> Result<AuthenticatedRendezvousReply, ProtocolError> {
        self.protocol.answer_adoption(request)
    }

    /// Performs bounded, supervisor-owned cleanup after freezing admission.
    ///
    /// A complete-exit acknowledgement is recorded only after a fresh
    /// independent observation proves that every registered process exited.
    /// The graceful and owned-termination phases each have a fixed five-second
    /// monotonic deadline. Observer failures remain unresolved and retain every
    /// surviving identity that can be named safely.
    #[must_use]
    pub fn cleanup(&mut self) -> CleanupResult {
        self.admission_open = false;
        self.control_endpoint.take();
        self.cleanup_started = true;

        let graceful_deadline = Instant::now() + GRACEFUL_CLEANUP_TIMEOUT;
        let mut snapshot = self.observe();
        while !is_complete(&snapshot) && Instant::now() < graceful_deadline {
            thread::sleep(CLEANUP_POLL_INTERVAL);
            snapshot = self.observe();
        }
        if is_complete(&snapshot) {
            return self.acknowledge_cleanup();
        }

        let termination_deadline = Instant::now() + OWNED_TERMINATION_TIMEOUT;
        let mut termination_failures = Vec::new();
        for root in &mut self.roots {
            if root
                .process_handle
                .child
                .try_wait()
                .ok()
                .flatten()
                .is_some()
            {
                continue;
            }
            let remaining = termination_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                termination_failures.push(root.identity.clone());
                continue;
            }
            if self
                .adapter
                .terminate_root(&mut root.process_handle, remaining)
                .is_err()
            {
                termination_failures.push(root.identity.clone());
            }
        }
        for root in &mut self.unidentified_roots {
            let remaining = termination_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero()
                || self
                    .adapter
                    .terminate_root(&mut root.process_handle, remaining)
                    .is_err()
            {
                // An unidentified root cannot safely contribute a NativeIdentity;
                // its diagnostic PID is included in the unresolved reason below.
                termination_failures.push(NativeIdentity {
                    birth_identity: format!(
                        "unidentified:pid:{}",
                        root.process_handle.diagnostic_pid()
                    ),
                    diagnostic_pid: root.process_handle.diagnostic_pid(),
                });
            }
        }

        snapshot = self.observe();
        while !is_complete(&snapshot) && Instant::now() < termination_deadline {
            thread::sleep(CLEANUP_POLL_INTERVAL);
            snapshot = self.observe();
        }
        if is_complete(&snapshot) {
            return self.acknowledge_cleanup();
        }
        let survivors = cleanup_survivors(&snapshot, termination_failures);
        let reason = cleanup_reason(&snapshot);
        let result = CleanupResult::Unresolved { survivors, reason };
        let _ = self.protocol.record_unresolved(match &result {
            CleanupResult::Unresolved { reason, .. } => reason.clone(),
            CleanupResult::Acknowledged(_) => unreachable!(),
        });
        result
    }

    fn acknowledge_cleanup(&mut self) -> CleanupResult {
        let receipt = CompleteExitReceipt {
            generation: self.generation,
            owned_processes: self.registered_identities.iter().cloned().collect(),
            observed_at_ms: unix_epoch_millis(),
        };
        if self.protocol.record_complete_exit(receipt.clone()).is_err() {
            return CleanupResult::Unresolved {
                survivors: self.registered_identities.iter().cloned().collect(),
                reason: "complete-exit receipt could not be recorded".to_owned(),
            };
        }
        unmark_inference_generation(self.generation);
        CleanupResult::Acknowledged(receipt)
    }

    /// Returns the live supervisor control endpoint used to prove handle isolation.
    #[must_use]
    pub fn control_endpoint_addr(&self) -> Option<SocketAddr> {
        self.control_endpoint
            .as_ref()
            .and_then(|listener| listener.local_addr().ok())
    }

    /// Independently observes every registered direct root.
    ///
    /// Observer failure and birth-identity mismatch are represented as
    /// `Unresolved`; neither can collapse to an empty process set.
    #[must_use]
    pub fn observe(&mut self) -> ObservationSnapshot {
        let mut unresolved_survivors = Vec::new();
        let mut roots = Vec::with_capacity(self.roots.len());
        let mut unidentified_roots = Vec::with_capacity(self.unidentified_roots.len());

        for root in &mut self.roots {
            let observed = self
                .adapter
                .observe_root(&mut root.process_handle, &root.identity)
                .unwrap_or(ExitState::Unresolved);
            let exit_state = if root.unresolved_until_exit && observed != ExitState::Exited {
                ExitState::Unresolved
            } else {
                if observed == ExitState::Exited {
                    root.unresolved_until_exit = false;
                }
                observed
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

        let mut retained_unidentified = Vec::with_capacity(self.unidentified_roots.len());
        for mut root in self.unidentified_roots.drain(..) {
            let observed = self
                .adapter
                .observe_unidentified_root(&mut root.process_handle)
                .unwrap_or(ExitState::Unresolved);
            if observed == ExitState::Exited {
                continue;
            }
            unidentified_roots.push(UnidentifiedProcessObservation {
                diagnostic_pid: root.process_handle.diagnostic_pid(),
                exit_state: ExitState::Unresolved,
            });
            retained_unidentified.push(root);
        }
        self.unidentified_roots = retained_unidentified;

        ObservationSnapshot {
            generation: self.generation,
            roots,
            descendants: Vec::new(),
            unidentified_roots,
            unresolved_survivors,
        }
    }

    /// Causes the next registration attempt to fail before release.
    pub fn inject_registration_failure_once(&mut self) {
        self.fail_registration_once = true;
    }

    fn take_registration_failure(&mut self) -> bool {
        std::mem::take(&mut self.fail_registration_once)
    }

    fn cleanup_failed_admission(
        &mut self,
        identity: NativeIdentity,
        role: Role,
        mut process_handle: OwnedProcessHandle,
        containment_membership: ContainmentMembership,
        identity_was_registered: bool,
    ) {
        if self
            .adapter
            .terminate_root(&mut process_handle, Duration::from_secs(2))
            .is_err()
        {
            self.registered_identities.insert(identity.clone());
            self.roots.push(RegisteredRoot {
                identity,
                role,
                process_handle,
                containment_membership,
                unresolved_until_exit: true,
            });
        } else if identity_was_registered {
            self.registered_identities.remove(&identity);
        }
    }
}

impl<A: PlatformAdapter> Drop for Supervisor<A> {
    fn drop(&mut self) {
        unmark_inference_generation(self.generation);
    }
}

fn inference_generations() -> &'static Mutex<HashSet<GenerationId>> {
    LIVE_INFERENCE_GENERATIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn inference_generation_is_blocked(generation: GenerationId) -> bool {
    inference_generations().lock().map_or(true, |generations| {
        generations.iter().any(|other| *other != generation)
    })
}

fn mark_inference_generation_live(generation: GenerationId) {
    if let Ok(mut generations) = inference_generations().lock() {
        generations.insert(generation);
    }
}

fn unmark_inference_generation(generation: GenerationId) {
    if let Ok(mut generations) = inference_generations().lock() {
        generations.remove(&generation);
    }
}

fn root_is_live(root: &mut RegisteredRoot) -> bool {
    root.process_handle
        .child
        .try_wait()
        .map_or(true, |status| status.is_none())
}

fn cleanup_survivors(
    snapshot: &ObservationSnapshot,
    termination_failures: Vec<NativeIdentity>,
) -> Vec<NativeIdentity> {
    let mut survivors = snapshot.unresolved_survivors.clone();
    survivors.extend(
        snapshot
            .roots
            .iter()
            .filter(|root| root.exit_state != ExitState::Exited)
            .map(|root| root.identity.clone()),
    );
    survivors.extend(termination_failures);
    survivors.sort_by(|left, right| left.birth_identity.cmp(&right.birth_identity));
    survivors.dedup();
    survivors
}

fn cleanup_reason(snapshot: &ObservationSnapshot) -> String {
    let unresolved = snapshot
        .unresolved_survivors
        .iter()
        .map(|identity| identity.birth_identity.as_str())
        .collect::<Vec<_>>();
    let unidentified = snapshot
        .unidentified_roots
        .iter()
        .map(|root| root.diagnostic_pid.to_string())
        .collect::<Vec<_>>();
    format!(
        "independent cleanup observation unresolved; surviving identities=[{}]; unidentified diagnostic pids=[{}]",
        unresolved.join(","),
        unidentified.join(",")
    )
}

fn unix_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |duration| duration.as_millis().max(1))
}

fn is_fixture_executable(executable: &Path) -> bool {
    executable
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name == "process-ownership-fixture" || name == "process-ownership-fixture.exe"
        })
}

#[cfg(target_os = "linux")]
fn query_birth_identity(pid: u32) -> Result<String, ()> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).map_err(|_| ())?;
    let after_name = stat.rsplit_once(')').ok_or(())?.1;
    let start_ticks = after_name.split_whitespace().nth(19).ok_or(())?;
    start_ticks.parse::<u64>().map_err(|_| ())?;
    Ok(format!("linux:{pid}:{start_ticks}"))
}

#[cfg(target_os = "macos")]
fn query_birth_identity(pid: u32) -> Result<String, ()> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    // SAFETY: `info` points to writable storage of exactly the size passed to
    // proc_pidinfo, and the value is assumed initialized only on a full result.
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
        return Err(());
    }
    // SAFETY: proc_pidinfo returned the full structure size above.
    let info = unsafe { info.assume_init() };
    Ok(format!(
        "macos:{pid}:{}:{}",
        info.pbi_start_tvsec, info.pbi_start_tvusec
    ))
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn partial_staging_failure_removes_created_directory() {
        let before = process_staging_directories();
        let source = File::open(std::env::temp_dir())
            .expect("temporary directory should open as an invalid image source");

        let result = prepare_launch_image(Path::new("invalid-source"), source);
        let after = process_staging_directories();
        let leaked = after.difference(&before).cloned().collect::<Vec<_>>();
        for directory in &leaked {
            let image = directory.join("process-ownership-fixture");
            let _ = std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700));
            let _ = std::fs::remove_file(image);
            let _ = std::fs::remove_dir(directory);
        }

        assert!(result.is_err());
        assert!(leaked.is_empty(), "partial staging leaked: {leaked:?}");
    }

    fn process_staging_directories() -> HashSet<PathBuf> {
        let prefix = format!("orkworks-process-ownership-image-{}-", std::process::id());
        std::fs::read_dir(std::env::temp_dir())
            .expect("temporary directory should be readable")
            .filter_map(Result::ok)
            .filter_map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with(&prefix))
                    .then(|| entry.path())
            })
            .collect()
    }
}
