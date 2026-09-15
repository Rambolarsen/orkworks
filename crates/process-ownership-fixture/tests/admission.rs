use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use process_ownership_fixture::observation::{is_complete, ContainmentMembership, ExitState};
use process_ownership_fixture::protocol::Role;
use process_ownership_fixture::supervisor::{
    LaunchSpec, SpawnError, Supervisor, RELEASE_EXEC_BYTE,
};
use process_ownership_fixture::targets::TargetBehavior;

const GENERATION: u64 = 11;
const TARGET_LIFETIME: Duration = Duration::from_millis(500);

static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
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
    LaunchSpec::fixture(
        fixture_executable(),
        role,
        behavior,
        marker.to_path_buf(),
        TARGET_LIFETIME,
    )
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

#[test]
fn registration_failure_discards_ticket_without_executing_target() {
    let directory = TestDirectory::new("registration-failure");
    let marker = directory.marker("pty-executed");
    let mut supervisor = prepared_supervisor();
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "fixture-pty", "req-pty")
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
        .issue_launch_ticket(Role::Inference, "fixture-inference", "req-inference")
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
fn independently_launched_child_is_never_admitted() {
    let directory = TestDirectory::new("unregistered-child");
    let marker = directory.marker("unregistered-executed");
    let mut supervisor = prepared_supervisor();
    let spec = launch_spec(Role::Pty, TargetBehavior::Silent, &marker);
    let mut child = Command::new(spec.executable())
        .args(spec.args())
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
        .issue_launch_ticket(Role::Inference, "fixture-inference", "req-early")
        .expect("ticket should be issued before owner loss");

    supervisor.owner_lost();

    let late_ticket =
        supervisor.issue_launch_ticket(Role::Inference, "fixture-inference", "req-late");
    let late_spawn = supervisor.spawn(
        early_ticket,
        launch_spec(Role::Inference, TargetBehavior::Inference, &marker),
    );

    assert_eq!(late_ticket, Err(SpawnError::AdmissionClosed));
    assert_eq!(late_spawn, Err(SpawnError::AdmissionClosed));
    assert!(!marker.exists());
}

#[test]
fn silent_target_is_observed_from_native_identity_and_liveness() {
    let directory = TestDirectory::new("silent-observation");
    let marker = directory.marker("silent-executed");
    let mut supervisor = prepared_supervisor();
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "fixture-inference", "req-silent")
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
        ContainmentMembership::Registered
    );
    assert!(!is_complete(&running));

    thread::sleep(TARGET_LIFETIME + Duration::from_millis(100));
    let exited = supervisor.observe();

    assert_eq!(exited.roots[0].exit_state, ExitState::Exited);
    assert!(is_complete(&exited));
}

#[test]
fn observer_failure_and_birth_identity_mismatch_are_unresolved() {
    let directory = TestDirectory::new("unresolved-observation");
    let marker = directory.marker("observed-executed");
    let mut supervisor = prepared_supervisor();
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "fixture-pty", "req-observed")
        .expect("ticket should be issued");
    let identity = supervisor
        .spawn(ticket, launch_spec(Role::Pty, TargetBehavior::Pty, &marker))
        .expect("registered target should launch");
    wait_for_marker(&marker);

    supervisor.inject_observer_failure_once();
    let unavailable = supervisor.observe();
    supervisor.inject_birth_identity_mismatch_once();
    let mismatched = supervisor.observe();

    assert_eq!(unavailable.roots[0].exit_state, ExitState::Unresolved);
    assert_eq!(unavailable.unresolved_survivors, vec![identity.clone()]);
    assert!(!is_complete(&unavailable));
    assert_eq!(mismatched.roots[0].exit_state, ExitState::Unresolved);
    assert_eq!(mismatched.unresolved_survivors, vec![identity]);
    assert!(!is_complete(&mismatched));
}
