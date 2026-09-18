use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use process_ownership_fixture::observation::{is_complete, ContainmentMembership, ExitState};
use process_ownership_fixture::observation::{ObservationSnapshot, ProcessObservation};
use process_ownership_fixture::protocol::{GenerationId, NativeIdentity, ProtocolError, Role};
use process_ownership_fixture::supervisor::{
    ExecutableImage, HostPlatformAdapter, LaunchSpec, OwnedProcessHandle, PausedRoot,
    PlatformAdapter, SpawnError, Supervisor, RELEASE_EXEC_BYTE,
};
use process_ownership_fixture::targets::TargetBehavior;

const GENERATION: u64 = 11;
const TARGET_LIFETIME: Duration = Duration::from_millis(500);
#[cfg(any(target_os = "linux", windows))]
const CONTROL_ENDPOINT_TARGET_LIFETIME: Duration = Duration::from_secs(10);

static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

#[cfg(unix)]
struct RestorePermissions {
    path: PathBuf,
    permissions: fs::Permissions,
}

#[cfg(unix)]
impl Drop for RestorePermissions {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, self.permissions.clone());
    }
}

impl TestDirectory {
    fn new(name: &str) -> Self {
        let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "orkworks-process-ownership-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("test directory should be created");
        Self { path }
    }

    fn marker(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_process-ownership-fixture"))
}

fn launch_spec(role: Role, behavior: TargetBehavior, marker: &Path) -> LaunchSpec {
    launch_spec_with_lifetime(role, behavior, marker, TARGET_LIFETIME)
}

fn launch_spec_with_lifetime(
    role: Role,
    behavior: TargetBehavior,
    marker: &Path,
    lifetime: Duration,
) -> LaunchSpec {
    LaunchSpec::fixture(role, behavior, marker.to_path_buf(), lifetime)
        .expect("fixture launch specification should be valid")
}

fn prepared_supervisor() -> Supervisor {
    let (supervisor, prepared) =
        Supervisor::prepare(GENERATION).expect("fixture generation should prepare");
    assert_eq!(prepared.record.generation, GENERATION);
    supervisor
}

fn wait_for_marker(marker: &Path) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !marker.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(marker.exists(), "target marker was not written: {marker:?}");
}

fn wait_for_exit(child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if child
            .try_wait()
            .expect("fixture child status should be observable")
            .is_some()
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "fixture child did not exit before test deadline"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(any(target_os = "linux", windows))]
fn wait_for_complete<A: PlatformAdapter>(supervisor: &mut Supervisor<A>) -> ObservationSnapshot {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let snapshot = supervisor.observe();
        if is_complete(&snapshot) {
            return snapshot;
        }
        assert!(
            Instant::now() < deadline,
            "supervisor observation did not complete before test deadline: {snapshot:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdapterFault {
    UnresolvedContainment,
    #[cfg(any(target_os = "linux", windows))]
    ObserverUnavailable,
    BirthIdentityMismatch,
    ReleaseAndTermination,
    IdentityAndTermination,
    #[cfg(unix)]
    SubstituteLaunchPath,
}

#[derive(Debug)]
struct FaultAdapter {
    host: HostPlatformAdapter,
    fault: AdapterFault,
    fail_next_observation: bool,
}

impl FaultAdapter {
    fn new(fault: AdapterFault) -> Self {
        let fail_next_observation = match fault {
            AdapterFault::ReleaseAndTermination | AdapterFault::IdentityAndTermination => true,
            #[cfg(any(target_os = "linux", windows))]
            AdapterFault::ObserverUnavailable => true,
            _ => false,
        };
        Self {
            host: HostPlatformAdapter,
            fault,
            fail_next_observation,
        }
    }
}

impl PlatformAdapter for FaultAdapter {
    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        #[cfg(unix)]
        if self.fault == AdapterFault::SubstituteLaunchPath {
            use std::os::unix::fs::PermissionsExt;

            let launch_path = executable.path();
            let directory = launch_path.parent().ok_or(SpawnError::ContainmentFailed)?;
            let trusted_backup = directory.join("trusted-image-backup");
            let restore_permissions = RestorePermissions {
                path: directory.to_path_buf(),
                permissions: fs::metadata(directory)
                    .map_err(|_| SpawnError::ContainmentFailed)?
                    .permissions(),
            };
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .map_err(|_| SpawnError::ContainmentFailed)?;
            fs::rename(launch_path, &trusted_backup).map_err(|_| SpawnError::ContainmentFailed)?;
            fs::copy(
                std::env::current_exe().map_err(|_| SpawnError::ContainmentFailed)?,
                launch_path,
            )
            .map_err(|_| SpawnError::ContainmentFailed)?;
            let result = self.host.create_paused_root(executable, spec);
            let _ = fs::remove_file(launch_path);
            let _ = fs::rename(&trusted_backup, launch_path);
            drop(restore_permissions);
            return result;
        }
        self.host.create_paused_root(executable, spec)
    }

    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError> {
        let mut identity = self.host.capture_native_identity(process_handle)?;
        if self.fault == AdapterFault::IdentityAndTermination {
            return Err(SpawnError::IdentityUnavailable);
        }
        if self.fault == AdapterFault::BirthIdentityMismatch {
            identity.birth_identity.push_str(":stale");
        }
        Ok(identity)
    }

    fn attach_containment(
        &mut self,
        generation: GenerationId,
        process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError> {
        if self.fault == AdapterFault::UnresolvedContainment {
            return Ok(ContainmentMembership::Unresolved);
        }
        self.host.attach_containment(generation, process_handle)
    }

    fn release_exec(&mut self, paused_root: &mut PausedRoot) -> Result<(), SpawnError> {
        if self.fault == AdapterFault::ReleaseAndTermination {
            return Err(SpawnError::ContainmentFailed);
        }
        self.host.release_exec(paused_root)
    }

    fn terminate_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        timeout: Duration,
    ) -> Result<(), SpawnError> {
        if matches!(
            self.fault,
            AdapterFault::ReleaseAndTermination | AdapterFault::IdentityAndTermination
        ) {
            process_handle.close_release_gate();
            return Err(SpawnError::ObserverUnavailable);
        }
        self.host.terminate_root(process_handle, timeout)
    }

    fn observe_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError> {
        if std::mem::take(&mut self.fail_next_observation) {
            return Err(SpawnError::ObserverUnavailable);
        }
        self.host.observe_root(process_handle, expected_identity)
    }

    fn observe_unidentified_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
    ) -> Result<ExitState, SpawnError> {
        if std::mem::take(&mut self.fail_next_observation) {
            return Err(SpawnError::ObserverUnavailable);
        }
        self.host.observe_unidentified_root(process_handle)
    }
}

fn prepared_with_fault(fault: AdapterFault) -> Supervisor<FaultAdapter> {
    Supervisor::prepare_with_adapter(GENERATION, FaultAdapter::new(fault))
        .expect("fixture generation should prepare")
        .0
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn registration_failure_discards_ticket_without_executing_target() {
    let directory = TestDirectory::new("registration-failure");
    let marker = directory.marker("pty-executed");
    let mut supervisor = prepared_supervisor();
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "req-pty")
        .expect("ticket should be issued");
    supervisor.inject_registration_failure_once();

    let result = supervisor.spawn(
        ticket.clone(),
        launch_spec(Role::Pty, TargetBehavior::Pty, &marker),
    );
    let replay = supervisor.spawn(ticket, launch_spec(Role::Pty, TargetBehavior::Pty, &marker));

    assert_eq!(result, Err(SpawnError::ObserverUnavailable));
    assert_eq!(replay, Err(SpawnError::InvalidTicket));
    assert!(!marker.exists());
    assert!(supervisor.observe().roots.is_empty());
}

#[test]
fn forged_ticket_is_rejected_before_target_execution() {
    let directory = TestDirectory::new("forged-ticket");
    let marker = directory.marker("inference-executed");
    let mut supervisor = prepared_supervisor();
    let mut ticket = supervisor
        .issue_launch_ticket(Role::Inference, "req-inference")
        .expect("ticket should be issued");
    ticket.generation += 1;

    let result = supervisor.spawn(
        ticket,
        launch_spec(Role::Inference, TargetBehavior::Inference, &marker),
    );

    assert_eq!(result, Err(SpawnError::InvalidTicket));
    assert!(!marker.exists());
    assert!(supervisor.observe().roots.is_empty());
}

#[test]
fn ticket_executable_identity_and_image_binding_are_supervisor_derived() {
    let directory = TestDirectory::new("derived-ticket-identity");
    let marker = directory.marker("forged-image-binding-executed");
    let mut supervisor = prepared_supervisor();
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "caller-nonce")
        .expect("ticket should be issued");

    assert_eq!(ticket.executable_identity, "fixture-pty");
    assert_ne!(ticket.request_nonce, "caller-nonce");
    assert!(ticket.request_nonce.ends_with(":caller-nonce"));

    let mut forged = ticket;
    forged.request_nonce = "caller-nonce".to_owned();
    let result = supervisor.spawn(forged, launch_spec(Role::Pty, TargetBehavior::Pty, &marker));

    assert_eq!(result, Err(SpawnError::InvalidTicket));
    assert!(!marker.exists());
}

#[test]
fn renamed_foreign_executable_is_rejected_during_preparation() {
    let directory = TestDirectory::new("renamed-foreign-executable");
    let renamed = directory.marker(if cfg!(windows) {
        "process-ownership-fixture.exe"
    } else {
        "process-ownership-fixture"
    });
    fs::copy(
        std::env::current_exe().expect("test executable path should be available"),
        &renamed,
    )
    .expect("foreign executable should be copied under the expected basename");

    let result = Supervisor::prepare_with_executable(GENERATION, renamed);

    assert!(matches!(
        result,
        Err(ProtocolError::InvalidValue {
            field: "fixture executable"
        })
    ));
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn launch_uses_prepared_executable_image_after_path_substitution() {
    let directory = TestDirectory::new("launch-time-substitution");
    let candidate = directory.marker(if cfg!(windows) {
        "process-ownership-fixture.exe"
    } else {
        "process-ownership-fixture"
    });
    let prepared_image = directory.marker("prepared-image");
    let marker = directory.marker("prepared-target-executed");
    fs::copy(fixture_executable(), &candidate)
        .expect("trusted fixture image should be copied for preparation");
    let (mut supervisor, _) = Supervisor::prepare_with_executable(GENERATION, candidate.clone())
        .expect("matching fixture image should prepare");

    let substitution = fs::rename(&candidate, &prepared_image);
    if cfg!(windows) {
        assert!(
            substitution.is_err(),
            "the retained image handle must prevent path replacement on Windows"
        );
    } else {
        substitution.expect("Unix path replacement should exercise descriptor-based launch");
        fs::copy(
            std::env::current_exe().expect("test executable path should be available"),
            &candidate,
        )
        .expect("foreign image should replace the prepared pathname");
    }

    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "req-substitution")
        .expect("ticket should be derived from the prepared image");
    supervisor
        .spawn(ticket, launch_spec(Role::Pty, TargetBehavior::Pty, &marker))
        .expect("spawn should use the prepared image rather than reopening its path");

    wait_for_marker(&marker);
}

#[cfg(target_os = "linux")]
#[test]
fn linux_launch_uses_retained_descriptor_after_actual_staged_path_substitution() {
    let directory = TestDirectory::new("actual-staged-path-substitution");
    let marker = directory.marker("descriptor-target-executed");
    let mut supervisor = prepared_with_fault(AdapterFault::SubstituteLaunchPath);
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "req-actual-path-substitution")
        .expect("ticket should be issued");

    supervisor
        .spawn(ticket, launch_spec(Role::Pty, TargetBehavior::Pty, &marker))
        .expect("Linux must execute the retained descriptor, not the substituted pathname");

    wait_for_marker(&marker);
}

#[cfg(target_os = "macos")]
#[test]
fn macos_actual_staged_path_substitution_fails_closed_without_descriptor_exec() {
    let directory = TestDirectory::new("macos-descriptor-unavailable");
    let marker = directory.marker("macos-target-executed");
    let mut supervisor = prepared_with_fault(AdapterFault::SubstituteLaunchPath);
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "req-macos-fail-closed")
        .expect("ticket should be issued");

    let result = supervisor.spawn(ticket, launch_spec(Role::Pty, TargetBehavior::Pty, &marker));

    assert_eq!(result, Err(SpawnError::ContainmentFailed));
    assert!(!marker.exists());
}

#[test]
fn independently_launched_child_is_never_admitted() {
    let directory = TestDirectory::new("unregistered-child");
    let marker = directory.marker("unregistered-executed");
    let mut supervisor = prepared_supervisor();
    let args = process_ownership_fixture::targets::arguments(
        Role::Pty,
        TargetBehavior::Silent,
        &marker,
        TARGET_LIFETIME,
    );
    let mut child = Command::new(fixture_executable())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("unregistered fixture child should start");
    child
        .stdin
        .take()
        .expect("release gate should be piped")
        .write_all(&[RELEASE_EXEC_BYTE])
        .expect("test should release unregistered child");

    wait_for_marker(&marker);
    let snapshot = supervisor.observe();
    wait_for_exit(&mut child);

    assert!(snapshot.roots.is_empty());
    assert!(snapshot.descendants.is_empty());
    assert!(is_complete(&snapshot));
}

#[test]
fn owner_loss_closes_admission_before_late_ticket_or_spawn() {
    let directory = TestDirectory::new("owner-loss");
    let marker = directory.marker("late-executed");
    let mut supervisor = prepared_supervisor();
    let early_ticket = supervisor
        .issue_launch_ticket(Role::Inference, "req-early")
        .expect("ticket should be issued before owner loss");

    supervisor.owner_lost();

    let late_ticket = supervisor.issue_launch_ticket(Role::Inference, "req-late");
    let late_spawn = supervisor.spawn(
        early_ticket,
        launch_spec(Role::Inference, TargetBehavior::Inference, &marker),
    );

    assert_eq!(late_ticket, Err(SpawnError::AdmissionClosed));
    assert_eq!(late_spawn, Err(SpawnError::AdmissionClosed));
    assert!(!marker.exists());
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn supervisor_control_endpoint_is_not_inherited_by_target() {
    let directory = TestDirectory::new("control-endpoint-inheritance");
    let marker = directory.marker("endpoint-target-executed");
    let mut supervisor = prepared_supervisor();
    let endpoint = supervisor
        .control_endpoint_addr()
        .expect("prepared supervisor should own a live control endpoint");
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "req-endpoint")
        .expect("ticket should be issued");
    let identity = supervisor
        .spawn(
            ticket,
            launch_spec_with_lifetime(
                Role::Pty,
                TargetBehavior::Silent,
                &marker,
                CONTROL_ENDPOINT_TARGET_LIFETIME,
            ),
        )
        .expect("registered target should launch");
    wait_for_marker(&marker);
    let before_rebind = supervisor.observe();
    assert_eq!(before_rebind.roots[0].identity, identity);
    assert_eq!(before_rebind.roots[0].exit_state, ExitState::Running);

    supervisor.owner_lost();
    let rebound = std::net::TcpListener::bind(endpoint)
        .expect("child must not inherit the supervisor control endpoint");
    let after_rebind = supervisor.observe();
    assert_eq!(after_rebind.roots[0].identity, identity);
    assert_eq!(after_rebind.roots[0].exit_state, ExitState::Running);
    drop(rebound);
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn unresolved_containment_is_rejected_before_release() {
    let directory = TestDirectory::new("unresolved-containment");
    let marker = directory.marker("unresolved-target-executed");
    let mut supervisor = prepared_with_fault(AdapterFault::UnresolvedContainment);
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "req-containment")
        .expect("ticket should be issued");

    let result = supervisor.spawn(ticket, launch_spec(Role::Pty, TargetBehavior::Pty, &marker));

    assert_eq!(result, Err(SpawnError::ContainmentFailed));
    assert!(!marker.exists());
    assert!(supervisor.observe().roots.is_empty());
}

#[test]
fn completion_rejects_unresolved_containment_membership() {
    let identity = NativeIdentity {
        birth_identity: "fixture-birth".to_owned(),
        diagnostic_pid: 7,
    };
    let snapshot = ObservationSnapshot {
        generation: GENERATION,
        roots: vec![ProcessObservation {
            identity,
            containment_membership: ContainmentMembership::Unresolved,
            exit_state: ExitState::Exited,
        }],
        descendants: Vec::new(),
        unidentified_roots: Vec::new(),
        unresolved_survivors: Vec::new(),
        observation_unresolved: false,
    };

    assert!(!is_complete(&snapshot));
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn failed_release_retains_uncertain_identity_for_observation() {
    let directory = TestDirectory::new("release-failure");
    let marker = directory.marker("release-failure-target-executed");
    let mut supervisor = prepared_with_fault(AdapterFault::ReleaseAndTermination);
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "req-release")
        .expect("ticket should be issued");

    let result = supervisor.spawn(
        ticket,
        launch_spec(Role::Inference, TargetBehavior::Inference, &marker),
    );
    let unresolved = supervisor.observe();

    assert_eq!(result, Err(SpawnError::ContainmentFailed));
    assert!(!marker.exists());
    assert_eq!(unresolved.roots.len(), 1);
    assert_eq!(unresolved.roots[0].exit_state, ExitState::Unresolved);
    assert_eq!(unresolved.unresolved_survivors.len(), 1);
    assert!(!is_complete(&unresolved));

    let exited = wait_for_complete(&mut supervisor);
    assert_eq!(exited.roots[0].exit_state, ExitState::Exited);
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn identity_capture_and_termination_failure_retains_unidentified_root() {
    let directory = TestDirectory::new("identity-capture-failure");
    let marker = directory.marker("identity-failure-target-executed");
    let mut supervisor = prepared_with_fault(AdapterFault::IdentityAndTermination);
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "req-identity-failure")
        .expect("ticket should be issued");

    let result = supervisor.spawn(
        ticket,
        launch_spec(Role::Inference, TargetBehavior::Inference, &marker),
    );
    let unresolved = supervisor.observe();

    assert_eq!(result, Err(SpawnError::IdentityUnavailable));
    assert!(!marker.exists());
    assert!(unresolved.roots.is_empty());
    assert_eq!(unresolved.unidentified_roots.len(), 1);
    assert_eq!(
        unresolved.unidentified_roots[0].exit_state,
        ExitState::Unresolved
    );
    assert!(!is_complete(&unresolved));

    let exited = wait_for_complete(&mut supervisor);
    assert!(exited.unidentified_roots.is_empty());
    assert!(is_complete(&exited));
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn silent_target_is_observed_from_native_identity_and_liveness() {
    let directory = TestDirectory::new("silent-observation");
    let marker = directory.marker("silent-executed");
    let mut supervisor = prepared_supervisor();
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "req-silent")
        .expect("ticket should be issued");

    let identity = supervisor
        .spawn(
            ticket,
            launch_spec(Role::Inference, TargetBehavior::Silent, &marker),
        )
        .expect("registered target should launch");
    wait_for_marker(&marker);
    fs::remove_file(&marker).expect("diagnostic marker should be removable");
    let running = supervisor.observe();

    assert_eq!(running.roots.len(), 1);
    assert_eq!(running.roots[0].identity, identity);
    assert_eq!(running.roots[0].exit_state, ExitState::Running);
    assert_eq!(
        running.roots[0].containment_membership,
        ContainmentMembership::Confirmed
    );
    assert!(!is_complete(&running));

    let exited = wait_for_complete(&mut supervisor);

    assert_eq!(exited.roots[0].exit_state, ExitState::Exited);
    assert!(is_complete(&exited));
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn observer_failure_is_unresolved() {
    let directory = TestDirectory::new("observer-failure");
    let marker = directory.marker("observer-failure-executed");
    let mut supervisor = prepared_with_fault(AdapterFault::ObserverUnavailable);
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "req-observed")
        .expect("ticket should be issued");
    let identity = supervisor
        .spawn(ticket, launch_spec(Role::Pty, TargetBehavior::Pty, &marker))
        .expect("registered target should launch");
    wait_for_marker(&marker);

    let unavailable = supervisor.observe();

    assert_eq!(unavailable.roots[0].exit_state, ExitState::Unresolved);
    assert_eq!(unavailable.unresolved_survivors, vec![identity]);
    assert!(!is_complete(&unavailable));
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn birth_identity_mismatch_from_platform_adapter_is_unresolved() {
    let directory = TestDirectory::new("birth-identity-mismatch");
    let marker = directory.marker("birth-mismatch-executed");
    let mut supervisor = prepared_with_fault(AdapterFault::BirthIdentityMismatch);
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "req-birth-mismatch")
        .expect("ticket should be issued");
    let identity = supervisor
        .spawn(ticket, launch_spec(Role::Pty, TargetBehavior::Pty, &marker))
        .expect("registered target should launch");
    wait_for_marker(&marker);

    let mismatched = supervisor.observe();

    assert_eq!(mismatched.roots[0].exit_state, ExitState::Unresolved);
    assert_eq!(mismatched.unresolved_survivors, vec![identity]);
    assert!(!is_complete(&mismatched));
}
