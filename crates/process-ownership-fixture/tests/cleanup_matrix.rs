use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(target_os = "macos"))]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(not(target_os = "macos"))]
use std::thread;
#[cfg(not(target_os = "macos"))]
use std::time::Instant;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(not(target_os = "macos"))]
use process_ownership_fixture::foreign_owner::{ForeignOwner, ForeignSnapshot};
#[cfg(not(target_os = "macos"))]
use process_ownership_fixture::observation::{is_complete, ContainmentMembership, ExitState};
#[cfg(not(target_os = "macos"))]
use process_ownership_fixture::protocol::NativeIdentity;
use process_ownership_fixture::protocol::{
    encode_reply_line, CleanupResult, CompleteExitReceipt, FixtureReply, ProtocolError, Role,
};
use process_ownership_fixture::supervisor::{
    bound_cleanup_result, LaunchSpec, RendezvousClient, SpawnError, Supervisor,
};
#[cfg(not(target_os = "macos"))]
use process_ownership_fixture::supervisor::{
    CleanupPhase, ExecutableImage, HostPlatformAdapter, LaunchInterleaveLatch, OwnedProcessHandle,
    PausedRoot, PlatformAdapter,
};
use process_ownership_fixture::targets::TargetBehavior;

const TARGET_LIFETIME: Duration = Duration::from_millis(150);
#[cfg(not(target_os = "macos"))]
const SURVIVOR_LIFETIME: Duration = Duration::from_secs(30);
#[cfg(not(target_os = "macos"))]
const DEATH_PROOF_LIFETIME: Duration = Duration::from_secs(2);
static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(not(target_os = "macos"))]
static INFERENCE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

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
fn many_long_survivors_are_bounded_before_cleanup_reply_encoding() {
    let survivors = (0..256)
        .map(
            |index| process_ownership_fixture::protocol::NativeIdentity {
                birth_identity: format!("survivor-{index}-{}", "x".repeat(2_000)),
                diagnostic_pid: index as u32,
            },
        )
        .collect();
    let result = bound_cleanup_result(CleanupResult::Unresolved {
        survivors,
        reason: "observer details".repeat(10_000),
    });
    let encoded = encode_reply_line(&FixtureReply::Cleanup { result })
        .expect("bounded unresolved reply should fit the protocol frame");
    assert!(encoded.len() <= process_ownership_fixture::protocol::MAX_MESSAGE_BYTES);
}

#[cfg(not(target_os = "macos"))]
fn inference_test_guard() -> std::sync::MutexGuard<'static, ()> {
    INFERENCE_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("inference fixture tests should serialize")
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
fn preissued_inference_ticket_rechecks_barrier_before_launch() {
    let _inference_guard = inference_test_guard();
    let directory = TestDirectory::new("preissued-inference-ticket");
    let marker = directory.marker("must-not-execute");
    let (mut replacement, _) = prepared_supervisor(116);
    let ticket = replacement
        .issue_launch_ticket(Role::Inference, "preissued")
        .expect("ticket should be issued before older barrier is live");
    let (mut older, _) = prepared_supervisor(117);
    let older_ticket = older
        .issue_launch_ticket(Role::Inference, "older")
        .expect("older inference ticket should be issued");
    older
        .spawn(
            older_ticket,
            launch_spec(
                Role::Inference,
                TargetBehavior::Silent,
                &directory.marker("older"),
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("older root should be admitted");
    older.owner_lost();

    assert_eq!(
        replacement.spawn(
            ticket,
            launch_spec(
                Role::Inference,
                TargetBehavior::Silent,
                &marker,
                TARGET_LIFETIME,
            ),
        ),
        Err(SpawnError::AdmissionClosed)
    );
    assert!(!marker.exists());
    let _ = assert_acknowledged(older.cleanup());
    let _ = assert_acknowledged(replacement.cleanup());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn fresh_same_generation_supervisor_cannot_clear_dead_authority_barrier() {
    let _inference_guard = inference_test_guard();
    let directory = TestDirectory::new("same-generation-authority");
    let (mut original, _) = prepared_supervisor(119);
    let ticket = original
        .issue_launch_ticket(Role::Inference, "original")
        .expect("original inference ticket should be issued");
    original
        .spawn(
            ticket,
            launch_spec(
                Role::Inference,
                TargetBehavior::Silent,
                &directory.marker("original"),
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("original root should be admitted");
    original.owner_lost();

    let (mut replacement, _) = prepared_supervisor(119);
    assert_eq!(
        replacement.issue_launch_ticket(Role::Inference, "same-generation"),
        Err(SpawnError::AdmissionClosed)
    );
    let _ = assert_acknowledged(original.cleanup());
    assert!(replacement
        .issue_launch_ticket(Role::Inference, "after-original-receipt")
        .is_ok());
    let _ = assert_acknowledged(replacement.cleanup());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn same_generation_authorities_keep_each_others_barrier_live() {
    let _inference_guard = inference_test_guard();
    let directory = TestDirectory::new("same-generation-two-authorities");
    let (mut first, _) = prepared_supervisor(121);
    let (mut second, _) = prepared_supervisor(121);
    let first_ticket = first
        .issue_launch_ticket(Role::Inference, "first")
        .expect("first authority should launch before owner loss");
    let second_ticket = second
        .issue_launch_ticket(Role::Inference, "second")
        .expect("second authority should launch before owner loss");
    first
        .spawn(
            first_ticket,
            launch_spec(
                Role::Inference,
                TargetBehavior::Silent,
                &directory.marker("first"),
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("first root should be admitted");
    second
        .spawn(
            second_ticket,
            launch_spec(
                Role::Inference,
                TargetBehavior::Silent,
                &directory.marker("second"),
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("second root should be admitted");
    first.owner_lost();
    second.owner_lost();
    let _ = assert_acknowledged(first.cleanup());

    let (mut replacement, _) = prepared_supervisor(121);
    assert_eq!(
        replacement.issue_launch_ticket(Role::Inference, "replacement"),
        Err(SpawnError::AdmissionClosed)
    );
    let _ = assert_acknowledged(second.cleanup());
    assert!(replacement
        .issue_launch_ticket(Role::Inference, "after-both-receipts")
        .is_ok());
    let _ = assert_acknowledged(replacement.cleanup());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn graceful_resistant_child_escalates_to_acknowledged_receipt() {
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
    assert!(started.elapsed() < Duration::from_secs(12));
    assert_eq!(receipt.generation, 102);
    let phases = supervisor.cleanup_phase_timings();
    let graceful = phases
        .iter()
        .find(|timing| timing.phase == CleanupPhase::Graceful)
        .expect("cleanup should record graceful phase");
    let escalation = phases
        .iter()
        .find(|timing| timing.phase == CleanupPhase::Escalation)
        .expect("cleanup should record escalation phase");
    assert!(graceful.elapsed_ms >= 4_900);
    assert!(graceful.elapsed_ms <= 5_500);
    assert!(escalation.elapsed_ms <= 5_000);
    assert!(receipt.owned_processes.contains(&identity));
    assert!(is_complete(&supervisor.observe()));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn injected_termination_failure_keeps_survivor_unresolved() {
    let directory = TestDirectory::new("injected-termination-failure");
    let marker = directory.marker("survivor");
    let (mut supervisor, _) = prepared_supervisor(123);
    let ticket = supervisor
        .issue_launch_ticket(Role::Sidecar, "injected-termination-failure")
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
    supervisor.inject_termination_failure_once();
    let started = Instant::now();
    let CleanupResult::Unresolved { survivors, reason } = supervisor.cleanup() else {
        panic!("an injected termination failure must remain unresolved");
    };

    assert!(started.elapsed() >= Duration::from_secs(9));
    assert!(started.elapsed() < Duration::from_secs(12));
    assert!(survivors.contains(&identity));
    assert!(reason.contains("injected termination failure"));
    assert!(!reason.contains("observer unavailable"));
}

#[test]
fn supervisor_death_before_receipt_keeps_adoption_unresolved() {
    let (supervisor, prepared) = prepared_supervisor(103);
    let (_, successor_prepared) = prepared_supervisor(104);
    let request = prepared
        .adoption_request(
            successor_prepared.record.generation,
            successor_prepared.record.nonce.clone(),
        )
        .expect("adoption challenge should be generated");
    drop(supervisor);

    assert!(matches!(
        prepared.verify_adoption_response(&request, None),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));
}

#[test]
fn adoption_retry_after_lost_response_returns_the_same_receipt() {
    let (mut supervisor, prepared) = prepared_supervisor(122);
    let (_, successor_prepared) = prepared_supervisor(123);
    supervisor.owner_lost();
    let receipt = assert_acknowledged(supervisor.cleanup());
    let request = prepared
        .adoption_request(
            successor_prepared.record.generation,
            successor_prepared.record.nonce.clone(),
        )
        .expect("adoption challenge should be generated");

    let first_response = supervisor
        .answer_adoption(&request)
        .expect("complete exit should be authenticated");
    let expected_response = first_response.clone();
    drop(first_response);

    let retry_response = supervisor
        .answer_adoption(&request)
        .expect("retrying a lost response should be idempotent");

    assert_eq!(retry_response, expected_response);
    assert_eq!(
        prepared
            .verify_adoption_response(&request, Some(&retry_response))
            .expect("retry response should remain verifiable"),
        receipt
    );
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
fn competing_foreign_owner_cannot_acquire_advisory_lease() {
    let directory = TestDirectory::new("foreign-contention");
    let (owner, before) =
        ForeignOwner::start(&directory.path, 42).expect("first foreign owner should acquire lease");
    assert!(ForeignOwner::start(&directory.path, 43).is_err());
    let during = owner
        .snapshot()
        .expect("first owner should remain readable");
    assert_foreign_identity_unchanged(&before, &during);
    drop(owner);
    let (replacement, _) = ForeignOwner::start(&directory.path, 43)
        .expect("lease should become available after owner release");
    let replacement_snapshot = replacement
        .snapshot()
        .expect("replacement owner state should survive prior Drop");
    assert_eq!(replacement_snapshot.metadata_revision, 43);
    drop(replacement);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn two_open_generations_are_cleaned_without_cross_generation_effects() {
    let _inference_guard = inference_test_guard();
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
                SURVIVOR_LIFETIME,
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
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("generation B root should be admitted");
    wait_for_marker(&marker_a);
    wait_for_marker(&marker_b);
    assert!(generation_a
        .observe()
        .roots
        .iter()
        .any(|root| root.exit_state == ExitState::Running));
    assert!(generation_b
        .observe()
        .roots
        .iter()
        .any(|root| root.exit_state == ExitState::Running));

    let receipt_a = assert_acknowledged(generation_a.cleanup());
    assert!(receipt_a.owned_processes.contains(&identity_a));
    assert!(!receipt_a.owned_processes.contains(&identity_b));
    assert_eq!(generation_b.observe().generation, 106);
    assert!(generation_b
        .observe()
        .roots
        .iter()
        .any(|root| root.exit_state == ExitState::Running));
    let receipt_b = assert_acknowledged(generation_b.cleanup());
    assert!(receipt_b.owned_processes.contains(&identity_b));
}

#[test]
fn late_obsolete_generation_events_are_rejected() {
    let directory = TestDirectory::new("obsolete-events");
    let (mut generation_a, prepared_a) = prepared_supervisor(107);
    let (mut generation_b, prepared_b) = prepared_supervisor(108);
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
        .adoption_request(108, prepared_b.record.nonce.clone())
        .expect("adoption challenge should be generated");
    assert!(matches!(
        prepared_a.verify_adoption_response(&request, None),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));
    assert!(matches!(
        RendezvousClient::new(prepared_a.clone(), 108, prepared_b.record.nonce.clone())
            .adopt(&stale_record),
        Err(ProtocolError::WrongGeneration { .. })
    ));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn immediate_relaunch_requires_authenticated_complete_exit_receipt() {
    let _inference_guard = inference_test_guard();
    let directory = TestDirectory::new("immediate-relaunch");
    let marker = directory.marker("owned");
    let replacement_marker = directory.marker("replacement");
    let (mut supervisor, prepared) = prepared_supervisor(109);
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "immediate-relaunch")
        .expect("ticket should be issued");
    supervisor
        .spawn(
            ticket,
            launch_spec(
                Role::Inference,
                TargetBehavior::Silent,
                &marker,
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("root should be admitted");
    wait_for_marker(&marker);
    supervisor.owner_lost();
    let (mut replacement, replacement_prepared) = prepared_supervisor(118);
    assert_eq!(
        replacement.issue_launch_ticket(Role::Inference, "before-complete-exit"),
        Err(SpawnError::AdmissionClosed)
    );
    let receipt = assert_acknowledged(supervisor.cleanup());
    let request = prepared
        .adoption_request(
            replacement_prepared.record.generation,
            replacement_prepared.record.nonce.clone(),
        )
        .expect("adoption challenge should be generated");
    let response = supervisor
        .answer_adoption(&request)
        .expect("live supervisor should answer adoption challenge");
    let adopted = RendezvousClient::from_authenticated_response(
        prepared.clone(),
        replacement_prepared.record.generation,
        replacement_prepared.record.nonce.clone(),
        response,
    )
    .adopt(&prepared.record)
    .expect("relaunch should adopt only complete authenticated exit");

    assert_eq!(adopted, receipt);
    let replacement_ticket = replacement
        .issue_launch_ticket(Role::Inference, "after-complete-exit")
        .expect("replacement should be admitted after complete-exit adoption");
    replacement
        .spawn(
            replacement_ticket,
            launch_spec(
                Role::Inference,
                TargetBehavior::Silent,
                &replacement_marker,
                TARGET_LIFETIME,
            ),
        )
        .expect("replacement target should execute");
    wait_for_marker(&replacement_marker);
    let _ = assert_acknowledged(replacement.cleanup());
    assert_eq!(replacement_prepared.record.generation, 118);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn inference_admission_is_blocked_while_an_older_root_survives() {
    let _inference_guard = inference_test_guard();
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
#[test]
fn supervisor_death_with_live_inference_root_retains_relaunch_barrier() {
    let _inference_guard = inference_test_guard();
    let directory = TestDirectory::new("death-live-inference");
    let marker = directory.marker("older-inference");
    let (mut older, _) = prepared_supervisor(112);
    let ticket = older
        .issue_launch_ticket(Role::Inference, "death-live-inference")
        .expect("older inference ticket should be issued");
    older
        .spawn(
            ticket,
            launch_spec(
                Role::Inference,
                TargetBehavior::Silent,
                &marker,
                DEATH_PROOF_LIFETIME,
            ),
        )
        .expect("older inference root should be admitted");
    wait_for_marker(&marker);
    older.owner_lost();
    assert!(older
        .observe()
        .roots
        .iter()
        .any(|root| root.exit_state == ExitState::Running));
    // The root is live at supervisor death; its retained handle is then
    // dropped by the fixture, but the generation barrier must remain.
    drop(older);

    let (mut replacement, _) = prepared_supervisor(113);

    assert_eq!(
        replacement.issue_launch_ticket(Role::Inference, "after-death"),
        Err(SpawnError::AdmissionClosed)
    );
    // A later authenticated complete-exit receipt is the only fixture path
    // that releases the retained barrier after the dead supervisor case.
    let (mut barrier_reaper, _) = prepared_supervisor(112);
    let _ = assert_acknowledged(barrier_reaper.cleanup());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn owner_loss_checkpoint_during_paused_launch_rejects_before_target_execution() {
    let directory = TestDirectory::new("owner-loss-checkpoint");
    let marker = directory.marker("must-not-execute");
    let (mut supervisor, _) = prepared_supervisor(114);
    let ticket = supervisor
        .issue_launch_ticket(Role::Pty, "owner-loss-checkpoint")
        .expect("ticket should be issued");
    let latch = Arc::new(LaunchInterleaveLatch::new());
    supervisor.arm_owner_loss_interleave(Arc::clone(&latch));
    let spec = launch_spec(Role::Pty, TargetBehavior::Silent, &marker, TARGET_LIFETIME);
    let supervisor = Arc::new(Mutex::new(supervisor));
    let (launch_sender, launch_receiver) = std::sync::mpsc::channel();
    let launch_supervisor = Arc::clone(&supervisor);
    let launch_thread = thread::spawn(move || {
        let result = launch_supervisor
            .lock()
            .expect("launch lock should not be poisoned")
            .spawn(ticket, spec);
        launch_sender
            .send(result)
            .expect("launch result receiver lives");
    });

    let owner_loss_supervisor = Arc::clone(&supervisor);
    let owner_loss_latch = Arc::clone(&latch);
    let owner_loss_done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let owner_loss_done_for_thread = Arc::clone(&owner_loss_done);
    let owner_loss_thread = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !owner_loss_latch.root_created() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(owner_loss_latch.root_created());
        owner_loss_latch.request_owner_loss();
        owner_loss_latch.release();
        owner_loss_supervisor
            .lock()
            .expect("owner-loss lock should not be poisoned")
            .owner_lost();
        owner_loss_done_for_thread.store(true, Ordering::Release);
    });
    let cleanup_supervisor = Arc::clone(&supervisor);
    let cleanup_done = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(7);
        while !owner_loss_done.load(Ordering::Acquire) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(owner_loss_done.load(Ordering::Acquire));
        cleanup_supervisor
            .lock()
            .expect("cleanup lock should not be poisoned")
            .cleanup()
    });

    let result = launch_receiver
        .recv_timeout(Duration::from_secs(7))
        .expect("in-flight launch should complete within its bound");
    launch_thread.join().expect("launch thread should complete");
    owner_loss_thread
        .join()
        .expect("owner-loss thread should complete");
    let cleanup_result = cleanup_done.join().expect("cleanup thread should complete");

    assert_eq!(result, Err(SpawnError::AdmissionClosed));
    assert!(!marker.exists());
    let _ = assert_acknowledged(cleanup_result);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn unresolved_cleanup_retains_survivor_and_specific_observer_errors() {
    let directory = TestDirectory::new("unresolved-cleanup");
    let marker = directory.marker("survivor");
    let (mut supervisor, _) = Supervisor::prepare_with_adapter(
        115,
        FailingCleanupAdapter {
            host: HostPlatformAdapter,
            fail_observation: true,
            fail_termination: true,
        },
    )
    .expect("fixture generation should prepare");
    let ticket = supervisor
        .issue_launch_ticket(Role::Sidecar, "unresolved-cleanup")
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
        .expect("root should be admitted");
    wait_for_marker(&marker);
    supervisor.owner_lost();

    let started = Instant::now();
    let result = supervisor.cleanup();

    let CleanupResult::Unresolved { survivors, reason } = result else {
        panic!("faulted cleanup must remain unresolved");
    };
    assert!(started.elapsed() >= Duration::from_secs(9));
    assert!(started.elapsed() < Duration::from_secs(12));
    assert!(survivors.contains(&identity));
    assert!(reason.contains("observe root"));
    assert!(reason.contains("observer unavailable"));
    assert!(reason.contains("terminate root"));
    assert!(reason.contains("containment failed"));
    assert_eq!(reason.matches("observe root").count(), 1);
    assert!(reason.len() < 48 * 1024);
    let encoded = encode_reply_line(&FixtureReply::Cleanup {
        result: CleanupResult::Unresolved { survivors, reason },
    })
    .expect("unresolved cleanup reply must fit the protocol frame bound");
    assert!(encoded.len() <= process_ownership_fixture::protocol::MAX_MESSAGE_BYTES);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn final_observation_filters_roots_that_exited_after_termination_error() {
    let directory = TestDirectory::new("final-survivor-filter");
    let (mut supervisor, _) = Supervisor::prepare_with_adapter(
        120,
        TerminateThenFailAdapter {
            host: HostPlatformAdapter,
            fail_after_terminating_once: true,
        },
    )
    .expect("fixture generation should prepare");
    let first_ticket = supervisor
        .issue_launch_ticket(Role::Sidecar, "first")
        .expect("first ticket should issue");
    let first = supervisor
        .spawn(
            first_ticket,
            launch_spec(
                Role::Sidecar,
                TargetBehavior::Silent,
                &directory.marker("first"),
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("first root should be admitted");
    let second_ticket = supervisor
        .issue_launch_ticket(Role::Sidecar, "second")
        .expect("second ticket should issue");
    let second = supervisor
        .spawn(
            second_ticket,
            launch_spec(
                Role::Sidecar,
                TargetBehavior::Silent,
                &directory.marker("second"),
                SURVIVOR_LIFETIME,
            ),
        )
        .expect("second root should be admitted");
    supervisor.owner_lost();

    let CleanupResult::Unresolved { survivors, .. } = supervisor.cleanup() else {
        panic!("the second root should keep cleanup unresolved");
    };
    assert!(!survivors.contains(&first));
    assert!(survivors.contains(&second));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn blocking_adapter_must_use_deadline_aware_cleanup_operations() {
    let directory = TestDirectory::new("blocking-adapter");
    let marker = directory.marker("blocking");
    let (mut supervisor, _) = Supervisor::prepare_with_adapter(
        122,
        BlockingCleanupAdapter {
            host: HostPlatformAdapter,
        },
    )
    .expect("fixture generation should prepare");
    let ticket = supervisor
        .issue_launch_ticket(Role::Sidecar, "blocking-adapter")
        .expect("ticket should issue");
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
        .expect("root should be admitted");
    wait_for_marker(&marker);
    supervisor.owner_lost();

    let started = Instant::now();
    let CleanupResult::Unresolved { survivors, .. } = supervisor.cleanup() else {
        panic!("deadline-aware blocking adapter should remain unresolved");
    };
    assert!(started.elapsed() < Duration::from_secs(11));
    assert!(survivors.contains(&identity));
}

#[cfg(not(target_os = "macos"))]
fn assert_foreign_identity_unchanged(before: &ForeignSnapshot, after: &ForeignSnapshot) {
    assert_eq!(before.lease_owner, after.lease_owner);
    assert_eq!(before.metadata_bytes, after.metadata_bytes);
    assert_eq!(before.metadata_revision, after.metadata_revision);
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug)]
struct FailingCleanupAdapter {
    host: HostPlatformAdapter,
    fail_observation: bool,
    fail_termination: bool,
}

#[cfg(not(target_os = "macos"))]
impl PlatformAdapter for FailingCleanupAdapter {
    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        self.host.create_paused_root(executable, spec)
    }

    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError> {
        self.host.capture_native_identity(process_handle)
    }

    fn attach_containment(
        &mut self,
        generation: u64,
        process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError> {
        self.host.attach_containment(generation, process_handle)
    }

    fn release_exec(&mut self, paused_root: &mut PausedRoot) -> Result<(), SpawnError> {
        self.host.release_exec(paused_root)
    }

    fn terminate_root(
        &mut self,
        _process_handle: &mut OwnedProcessHandle,
        _timeout: Duration,
    ) -> Result<(), SpawnError> {
        if self.fail_termination {
            return Err(SpawnError::ContainmentFailed);
        }
        unreachable!("test adapter only models termination failure")
    }

    fn observe_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError> {
        if self.fail_observation {
            return Err(SpawnError::ObserverUnavailable);
        }
        self.host.observe_root(process_handle, expected_identity)
    }
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug)]
struct TerminateThenFailAdapter {
    host: HostPlatformAdapter,
    fail_after_terminating_once: bool,
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug)]
struct BlockingCleanupAdapter {
    host: HostPlatformAdapter,
}

#[cfg(not(target_os = "macos"))]
impl PlatformAdapter for BlockingCleanupAdapter {
    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        self.host.create_paused_root(executable, spec)
    }

    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError> {
        self.host.capture_native_identity(process_handle)
    }

    fn attach_containment(
        &mut self,
        generation: u64,
        process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError> {
        self.host.attach_containment(generation, process_handle)
    }

    fn release_exec(&mut self, paused_root: &mut PausedRoot) -> Result<(), SpawnError> {
        self.host.release_exec(paused_root)
    }

    fn terminate_root(
        &mut self,
        _process_handle: &mut OwnedProcessHandle,
        _timeout: Duration,
    ) -> Result<(), SpawnError> {
        thread::sleep(Duration::from_secs(60));
        Err(SpawnError::ContainmentFailed)
    }

    fn terminate_root_bounded(
        &mut self,
        _process_handle: &mut OwnedProcessHandle,
        _deadline: std::time::Instant,
    ) -> Result<(), SpawnError> {
        Err(SpawnError::ContainmentFailed)
    }

    fn observe_root(
        &mut self,
        _process_handle: &mut OwnedProcessHandle,
        _expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError> {
        thread::sleep(Duration::from_secs(60));
        Err(SpawnError::ObserverUnavailable)
    }

    fn observe_root_bounded(
        &mut self,
        _process_handle: &mut OwnedProcessHandle,
        _expected_identity: &NativeIdentity,
        _deadline: std::time::Instant,
    ) -> Result<ExitState, SpawnError> {
        Err(SpawnError::ObserverUnavailable)
    }
}

#[cfg(not(target_os = "macos"))]
impl PlatformAdapter for TerminateThenFailAdapter {
    fn create_paused_root(
        &mut self,
        executable: &ExecutableImage,
        spec: &LaunchSpec,
    ) -> Result<OwnedProcessHandle, SpawnError> {
        self.host.create_paused_root(executable, spec)
    }

    fn capture_native_identity(
        &mut self,
        process_handle: &OwnedProcessHandle,
    ) -> Result<NativeIdentity, SpawnError> {
        self.host.capture_native_identity(process_handle)
    }

    fn attach_containment(
        &mut self,
        generation: u64,
        process_handle: &OwnedProcessHandle,
    ) -> Result<ContainmentMembership, SpawnError> {
        self.host.attach_containment(generation, process_handle)
    }

    fn release_exec(&mut self, paused_root: &mut PausedRoot) -> Result<(), SpawnError> {
        self.host.release_exec(paused_root)
    }

    fn terminate_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        timeout: Duration,
    ) -> Result<(), SpawnError> {
        if self.fail_after_terminating_once {
            self.fail_after_terminating_once = false;
            let _ = self.host.terminate_root(process_handle, timeout);
            return Err(SpawnError::ContainmentFailed);
        }
        Err(SpawnError::ContainmentFailed)
    }

    fn observe_root(
        &mut self,
        process_handle: &mut OwnedProcessHandle,
        expected_identity: &NativeIdentity,
    ) -> Result<ExitState, SpawnError> {
        self.host.observe_root(process_handle, expected_identity)
    }
}
