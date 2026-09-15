use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(target_os = "macos"))]
use std::thread;
#[cfg(not(target_os = "macos"))]
use std::time::Instant;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(not(target_os = "macos"))]
use process_ownership_fixture::foreign_owner::{ForeignOwner, ForeignSnapshot};
#[cfg(not(target_os = "macos"))]
use process_ownership_fixture::observation::{is_complete, ExitState};
use process_ownership_fixture::protocol::{
    CleanupResult, CompleteExitReceipt, ProtocolError, Role,
};
use process_ownership_fixture::supervisor::{LaunchSpec, RendezvousClient, SpawnError, Supervisor};
use process_ownership_fixture::targets::TargetBehavior;

const TARGET_LIFETIME: Duration = Duration::from_millis(150);
#[cfg(not(target_os = "macos"))]
const SURVIVOR_LIFETIME: Duration = Duration::from_secs(30);
static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(name: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "orkworks-process-ownership-cleanup-{name}-{}-{nanos}-{sequence}",
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

fn launch_spec(
    role: Role,
    behavior: TargetBehavior,
    marker: &Path,
    lifetime: Duration,
) -> LaunchSpec {
    LaunchSpec::fixture(role, behavior, marker.to_path_buf(), lifetime)
        .expect("fixture launch specification should be valid")
}

fn cleanup_root_behavior() -> TargetBehavior {
    // The existing macOS host adapter intentionally rejects setsid-backed
    // launch, so the shared cleanup races use a silent root on that host.
    if cfg!(target_os = "macos") {
        TargetBehavior::Silent
    } else {
        TargetBehavior::Pty
    }
}

fn prepared_supervisor(
    generation: u64,
) -> (
    Supervisor,
    process_ownership_fixture::protocol::PreparedGeneration,
) {
    Supervisor::prepare(generation).expect("fixture generation should prepare")
}

#[cfg(not(target_os = "macos"))]
fn wait_for_marker(marker: &Path) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !marker.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(marker.exists(), "target marker was not written: {marker:?}");
}

fn assert_acknowledged(result: CleanupResult) -> CompleteExitReceipt {
    let CleanupResult::Acknowledged(receipt) = result else {
        panic!("cleanup must acknowledge only after independent exit observation: {result:?}");
    };
    receipt
}

#[test]
fn owner_loss_during_launch_freezes_admission_before_target_execution() {
    let directory = TestDirectory::new("owner-loss-during-launch");
    let marker = directory.marker("late");
    let (mut supervisor, _) = prepared_supervisor(101);
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "launch-race")
        .expect("ticket should be issued before owner loss");

    supervisor.owner_lost();
    let result = supervisor.spawn(
        ticket,
        launch_spec(Role::Pty, cleanup_root_behavior(), &marker, TARGET_LIFETIME),
    );

    assert_eq!(result, Err(SpawnError::AdmissionClosed));
    assert!(!marker.exists(), "late launch must not execute target code");
    let receipt = assert_acknowledged(supervisor.cleanup());
    assert!(receipt.owned_processes.is_empty());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn graceful_ignore_escalates_to_bounded_owned_termination() {
    let directory = TestDirectory::new("graceful-ignore");
    let marker = directory.marker("survivor");
    let (mut supervisor, _) = prepared_supervisor(102);
    let ticket = supervisor
        .issue_launch_ticket(Role::Sidecar, "graceful-ignore")
        .expect("ticket should be issued");
    let identity = supervisor
        .spawn(
            ticket,
            launch_spec(
                Role::Sidecar,
                TargetBehavior::Silent,
                &marker,
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("survivor should be admitted");
    wait_for_marker(&marker);

    supervisor.owner_lost();
    let started = Instant::now();
    let receipt = assert_acknowledged(supervisor.cleanup());

    assert!(started.elapsed() >= Duration::from_secs(4));
    assert!(receipt.owned_processes.contains(&identity));
    assert!(is_complete(&supervisor.observe()));
}

#[test]
fn supervisor_death_before_receipt_keeps_adoption_unresolved() {
    let (supervisor, prepared) = prepared_supervisor(103);
    let request = prepared
        .adoption_request()
        .expect("adoption challenge should be generated");
    drop(supervisor);

    assert!(matches!(
        prepared.verify_adoption_response(&request, None),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn foreign_owner_lease_metadata_and_heartbeat_survive_owned_cleanup() {
    let directory = TestDirectory::new("foreign-owner");
    let (foreign_owner, before) = ForeignOwner::start(&directory.path, 41)
        .expect("foreign owner should acquire workspace lease");
    let (mut supervisor, _) = prepared_supervisor(104);
    let marker = directory.marker("owned");
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "foreign-cleanup")
        .expect("ticket should be issued");
    supervisor
        .spawn(
            ticket,
            launch_spec(Role::Pty, cleanup_root_behavior(), &marker, TARGET_LIFETIME),
        )
        .expect("owned root should be admitted");
    wait_for_marker(&marker);

    supervisor.owner_lost();
    let _ = assert_acknowledged(supervisor.cleanup());
    let after = foreign_owner
        .snapshot()
        .expect("foreign snapshot should read");

    assert_foreign_identity_unchanged(&before, &after);
    assert!(after.heartbeat >= before.heartbeat);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn two_open_generations_are_cleaned_without_cross_generation_effects() {
    let directory = TestDirectory::new("two-generations");
    let (mut generation_a, _) = prepared_supervisor(105);
    let (mut generation_b, _) = prepared_supervisor(106);
    let marker_a = directory.marker("a");
    let marker_b = directory.marker("b");
    let ticket_a = generation_a
        .issue_launch_ticket(Role::Pty, "generation-a")
        .expect("generation A ticket should be issued");
    let ticket_b = generation_b
        .issue_launch_ticket(Role::Inference, "generation-b")
        .expect("generation B ticket should be issued");
    let identity_a = generation_a
        .spawn(
            ticket_a,
            launch_spec(
                Role::Pty,
                cleanup_root_behavior(),
                &marker_a,
                TARGET_LIFETIME,
            ),
        )
        .expect("generation A root should be admitted");
    let identity_b = generation_b
        .spawn(
            ticket_b,
            launch_spec(
                Role::Inference,
                cleanup_root_behavior(),
                &marker_b,
                TARGET_LIFETIME,
            ),
        )
        .expect("generation B root should be admitted");
    wait_for_marker(&marker_a);
    wait_for_marker(&marker_b);

    let receipt_a = assert_acknowledged(generation_a.cleanup());
    assert!(receipt_a.owned_processes.contains(&identity_a));
    assert!(!receipt_a.owned_processes.contains(&identity_b));
    assert_eq!(generation_b.observe().generation, 106);
    let receipt_b = assert_acknowledged(generation_b.cleanup());
    assert!(receipt_b.owned_processes.contains(&identity_b));
}

#[test]
fn late_obsolete_generation_events_are_rejected() {
    let directory = TestDirectory::new("obsolete-events");
    let (mut generation_a, prepared_a) = prepared_supervisor(107);
    let (mut generation_b, _) = prepared_supervisor(108);
    let ticket_a = generation_a
        .issue_launch_ticket(Role::Pty, "obsolete-ticket")
        .expect("generation A ticket should be issued");
    let marker_b = directory.marker("replacement");
    let stale_spawn = generation_b.spawn(
        ticket_a,
        launch_spec(
            Role::Pty,
            cleanup_root_behavior(),
            &marker_b,
            TARGET_LIFETIME,
        ),
    );

    assert_eq!(stale_spawn, Err(SpawnError::InvalidTicket));
    assert!(!marker_b.exists());
    let mut stale_record = prepared_a.record.clone();
    stale_record.generation = 108;
    let request = prepared_a
        .adoption_request()
        .expect("adoption challenge should be generated");
    assert!(matches!(
        prepared_a.verify_adoption_response(&request, None),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));
    assert!(matches!(
        RendezvousClient::new(prepared_a.clone()).adopt(&stale_record),
        Err(ProtocolError::WrongGeneration { .. })
    ));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn immediate_relaunch_requires_authenticated_complete_exit_receipt() {
    let directory = TestDirectory::new("immediate-relaunch");
    let marker = directory.marker("owned");
    let (mut supervisor, prepared) = prepared_supervisor(109);
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "immediate-relaunch")
        .expect("ticket should be issued");
    supervisor
        .spawn(
            ticket,
            launch_spec(Role::Pty, cleanup_root_behavior(), &marker, TARGET_LIFETIME),
        )
        .expect("root should be admitted");
    wait_for_marker(&marker);
    let receipt = assert_acknowledged(supervisor.cleanup());
    let request = prepared
        .adoption_request()
        .expect("adoption challenge should be generated");
    let response = supervisor
        .answer_adoption(&request)
        .expect("live supervisor should answer adoption challenge");
    let adopted = RendezvousClient::from_authenticated_response(prepared.clone(), response)
        .adopt(&prepared.record)
        .expect("relaunch should adopt only complete authenticated exit");

    assert_eq!(adopted, receipt);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn inference_admission_is_blocked_while_an_older_root_survives() {
    let directory = TestDirectory::new("inference-barrier");
    let marker = directory.marker("older-inference");
    let (mut older, _) = prepared_supervisor(110);
    let ticket = older
        .issue_launch_ticket(Role::Inference, "older-inference")
        .expect("older inference ticket should be issued");
    older
        .spawn(
            ticket,
            launch_spec(
                Role::Inference,
                TargetBehavior::Silent,
                &marker,
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("older inference root should be admitted");
    wait_for_marker(&marker);
    older.owner_lost();
    let (mut replacement, _) = prepared_supervisor(111);

    let result = replacement.issue_launch_ticket(Role::Inference, "replacement-inference");

    assert_eq!(result, Err(SpawnError::AdmissionClosed));
    let snapshot = older.observe();
    assert!(snapshot
        .roots
        .iter()
        .any(|root| root.exit_state == ExitState::Running));
    let _ = assert_acknowledged(older.cleanup());
}

#[cfg(not(target_os = "macos"))]
fn assert_foreign_identity_unchanged(before: &ForeignSnapshot, after: &ForeignSnapshot) {
    assert_eq!(before.lease_owner, after.lease_owner);
    assert_eq!(before.metadata_bytes, after.metadata_bytes);
    assert_eq!(before.metadata_revision, after.metadata_revision);
}
