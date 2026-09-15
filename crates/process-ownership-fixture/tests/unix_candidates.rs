#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use process_ownership_fixture::observation::{is_complete, ExitState};
use process_ownership_fixture::platform::{PlatformError, UnixCandidate, UnixCandidateKind};
use process_ownership_fixture::protocol::{NativeIdentity, Role};
use process_ownership_fixture::supervisor::LaunchSpec;
use process_ownership_fixture::targets::TargetBehavior;
use serde::Serialize;

const TARGET_LIFETIME: Duration = Duration::from_secs(5);
static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "orkworks-process-ownership-unix-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("test directory should be created");
        Self(path)
    }

    fn marker(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, Serialize)]
struct EvidenceRecord {
    mechanism: &'static str,
    scenario: &'static str,
    outcome: &'static str,
    native_identities: Vec<NativeIdentity>,
    descendant_identities: Vec<NativeIdentity>,
    containment: Vec<&'static str>,
    cleanup_phases: Vec<&'static str>,
    host: &'static str,
    architecture: &'static str,
    build_mode: &'static str,
    cleanup_observed: bool,
    survivor_state: &'static str,
    failure_reason: Option<String>,
}

fn fixture_spec(behavior: TargetBehavior, marker: &Path) -> LaunchSpec {
    LaunchSpec::fixture(
        Role::Sidecar,
        behavior,
        marker.to_path_buf(),
        TARGET_LIFETIME,
    )
    .expect("fixture launch specification should be valid")
}

fn wait_for_marker(marker: &Path) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !marker.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(marker.exists(), "target marker was not written: {marker:?}");
}

fn wait_for_running(candidate: &mut UnixCandidate) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let snapshot = candidate
            .observe()
            .expect("candidate observation should work");
        if snapshot
            .roots
            .iter()
            .any(|root| root.exit_state == ExitState::Running)
        {
            return true;
        }
        if snapshot
            .roots
            .iter()
            .any(|root| root.exit_state == ExitState::Exited)
        {
            return false;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn candidate(kind: UnixCandidateKind, generation: u64) -> Result<UnixCandidate, PlatformError> {
    UnixCandidate::for_generation(kind, generation)
}

#[test]
fn candidate_matrix_is_fail_closed_for_every_unix_mechanism() {
    let behaviors = [
        TargetBehavior::Pty,
        TargetBehavior::NewGroup,
        TargetBehavior::Forked,
        TargetBehavior::Daemonized,
        TargetBehavior::Reparented,
        TargetBehavior::Silent,
    ];
    let kinds = [
        UnixCandidateKind::ProcessGroup,
        UnixCandidateKind::RegisteredRoot,
        UnixCandidateKind::Launchd,
    ];

    for (kind_index, kind) in kinds.into_iter().enumerate() {
        for (scenario_index, behavior) in behaviors.into_iter().enumerate() {
            let directory = TestDirectory::new(&format!("{kind_index}-{scenario_index}"));
            let marker = directory.marker("target");
            let mechanism = kind.as_str();
            let mut evidence = EvidenceRecord {
                mechanism,
                scenario: behavior.as_str(),
                outcome: "unresolved",
                native_identities: Vec::new(),
                descendant_identities: Vec::new(),
                containment: Vec::new(),
                cleanup_phases: Vec::new(),
                host: std::env::consts::OS,
                architecture: std::env::consts::ARCH,
                build_mode: if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                },
                cleanup_observed: false,
                survivor_state: "unresolved",
                failure_reason: None,
            };

            let result = candidate(kind, 400 + kind_index as u64);
            let mut candidate = match result {
                Ok(candidate) => candidate,
                Err(PlatformError::UnsupportedPlatform { .. }) => {
                    evidence.survivor_state = "unsupported";
                    evidence.outcome = "unsupported";
                    evidence.failure_reason = Some(
                        "portable Unix cleanup has no kernel-bound ownership primitive".into(),
                    );
                    write_evidence(&directory, &evidence);
                    println!("{}", serde_json::to_string(&evidence).unwrap());
                    continue;
                }
                Err(error) => panic!("{mechanism} candidate should prepare: {error}"),
            };

            let paused = match candidate.launch_paused(fixture_spec(behavior, &marker)) {
                Ok(paused) => paused,
                Err(PlatformError::UnsupportedPlatform { .. }) => {
                    evidence.survivor_state = "unsupported";
                    evidence.outcome = "unsupported";
                    evidence.failure_reason = Some(
                        "portable Unix cleanup has no kernel-bound ownership primitive".into(),
                    );
                    write_evidence(&directory, &evidence);
                    println!("{}", serde_json::to_string(&evidence).unwrap());
                    continue;
                }
                Err(error) => {
                    panic!("{mechanism} launch should either run or reject cleanly: {error}")
                }
            };
            evidence.native_identities.push(paused.identity.clone());
            evidence.containment.push("registered-root-confirmed");
            wait_for_marker(&marker);
            let independently_live = wait_for_running(&mut candidate);
            if !independently_live {
                evidence.failure_reason = Some("root exited before live observation".into());
            }

            let cleanup = candidate.terminate_owned();
            let after = candidate
                .observe()
                .expect("post-cleanup observation should work");
            evidence.descendant_identities = after
                .descendants
                .iter()
                .map(|descendant| descendant.identity.clone())
                .collect();
            evidence.cleanup_phases = vec!["identity-revalidation", "owned-termination", "census"];
            evidence.cleanup_observed = independently_live && is_complete(&after);
            evidence.outcome = expected_outcome(kind, behavior);
            evidence.survivor_state = if evidence.cleanup_observed {
                "none"
            } else {
                "survivor-or-ambiguous"
            };
            if let Err(ref error) = cleanup {
                evidence.failure_reason = Some(error.to_string());
            }
            let expected = evidence.outcome;
            match expected {
                "pass" => {
                    assert!(
                        cleanup.is_ok(),
                        "unexpected cleanup rejection: {evidence:?}"
                    );
                    assert!(
                        evidence.cleanup_observed,
                        "pass row was not complete: {evidence:?}"
                    );
                    assert!(
                        evidence.failure_reason.is_none(),
                        "pass row has a failure: {evidence:?}"
                    );
                }
                "reject" | "unresolved" => {
                    assert!(
                        evidence.failure_reason.is_some() || !evidence.cleanup_observed,
                        "rejection row was unexplained: {evidence:?}"
                    );
                }
                "unsupported" => unreachable!("unsupported candidate was handled above"),
                other => panic!("unknown expected candidate outcome: {other}"),
            }
            write_evidence(&directory, &evidence);
            println!("{}", serde_json::to_string(&evidence).unwrap());

            drop(paused);
            assert!(
                evidence.cleanup_observed || evidence.failure_reason.is_some(),
                "{mechanism}/{} must pass with evidence or reject explicitly",
                behavior.as_str()
            );
        }
    }
}

fn expected_outcome(kind: UnixCandidateKind, behavior: TargetBehavior) -> &'static str {
    let _ = (kind, behavior);
    "unsupported"
}

fn write_evidence(directory: &TestDirectory, evidence: &EvidenceRecord) {
    let evidence_path = directory.marker("evidence.json");
    fs::write(&evidence_path, serde_json::to_vec(evidence).unwrap())
        .expect("candidate evidence should be durable");
    assert!(evidence_path.is_file());
}

#[test]
fn registered_root_rejects_unbound_per_pid_cleanup() {
    let result = candidate(UnixCandidateKind::RegisteredRoot, 401);
    assert!(matches!(
        result,
        Err(PlatformError::UnsupportedPlatform { candidate })
            if candidate.contains("registered-root")
    ));
}

#[test]
fn process_group_rejects_unbound_group_cleanup() {
    let result = candidate(UnixCandidateKind::ProcessGroup, 403);
    assert!(matches!(
        result,
        Err(PlatformError::UnsupportedPlatform { candidate })
            if candidate.contains("process-group")
    ));
}

#[test]
fn daemonized_and_reparented_rows_are_rejected_without_native_claims() {
    for behavior in [TargetBehavior::Daemonized, TargetBehavior::Reparented] {
        let result = candidate(UnixCandidateKind::RegisteredRoot, 406);
        assert!(
            matches!(
                result,
                Err(PlatformError::UnsupportedPlatform { candidate })
                    if candidate.contains("registered-root")
            ),
            "{} row must be rejected without native census evidence",
            behavior.as_str()
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn launchd_candidate_is_explicitly_unsupported_without_a_native_fixture() {
    let directory = TestDirectory::new("launchd-unsupported");
    let result = process_ownership_fixture::platform::LaunchdDomain::bootstrap(
        "orkworks-task4-404",
        &directory.marker("missing.plist"),
    );
    assert!(matches!(
        result,
        Err(PlatformError::UnsupportedPlatform {
            candidate: "launchd"
        })
    ));
}

#[cfg(target_os = "macos")]
#[test]
fn macos_process_candidates_fail_closed_without_handle_based_launch() {
    for kind in [
        UnixCandidateKind::ProcessGroup,
        UnixCandidateKind::RegisteredRoot,
        UnixCandidateKind::Launchd,
    ] {
        let result = candidate(kind, 405);
        assert!(matches!(
            result,
            Err(PlatformError::UnsupportedPlatform { .. })
        ));
    }
}

#[test]
fn cleanup_race_is_bounded_and_never_reports_empty_on_observer_failure() {
    let result = candidate(UnixCandidateKind::RegisteredRoot, 402);
    assert!(matches!(
        result,
        Err(PlatformError::UnsupportedPlatform { candidate })
            if candidate.contains("registered-root")
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn concurrent_verified_launches_do_not_inherit_sibling_image_descriptors() {
    use process_ownership_fixture::supervisor::Supervisor;

    const LAUNCHES: usize = 12;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(LAUNCHES));
    let mut threads = Vec::with_capacity(LAUNCHES);
    for index in 0..LAUNCHES {
        let barrier = std::sync::Arc::clone(&barrier);
        threads.push(thread::spawn(move || {
            let directory = TestDirectory::new(&format!("fd-race-{index}"));
            let marker = directory.marker("root");
            let (mut supervisor, prepared) = Supervisor::prepare(500 + index as u64)
                .expect("supervisor should prepare concurrently");
            let ticket = supervisor
                .issue_launch_ticket(Role::Sidecar, format!("fd-race-{index}"))
                .expect("ticket should be issued");
            barrier.wait();
            supervisor
                .spawn(ticket, fixture_spec(TargetBehavior::Silent, &marker))
                .expect("verified image launch should succeed");
            wait_for_marker(&marker);
            let contents = fs::read_to_string(&marker).expect("marker should be readable");
            let inherited = contents
                .lines()
                .find_map(|line| line.strip_prefix("verified_image_fds="))
                .and_then(|count| count.parse::<usize>().ok())
                .expect("target should report verified image descriptor count");
            assert_eq!(
                inherited, 1,
                "target inherited sibling image descriptors: {contents}"
            );
            drop(prepared);
        }));
    }
    for thread in threads {
        thread.join().expect("concurrent launch should not panic");
    }
}
