#![cfg(windows)]

use std::fs;
use std::net::TcpListener;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use process_ownership_fixture::observation::{is_complete, ExitState};
use process_ownership_fixture::platform::{
    electron_parent_arguments, BreakawayResult, ForcedParentCompletion, ForcedParentReady,
    OwnerDomain,
};
use process_ownership_fixture::protocol::{NativeIdentity, Role};
use process_ownership_fixture::supervisor::{LaunchSpec, SpawnError, Supervisor};
use process_ownership_fixture::targets::TargetBehavior;
use serde::de::DeserializeOwned;
use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{
    OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE,
};

const GENERATION: u64 = 23;
const TARGET_LIFETIME: Duration = Duration::from_secs(30);
const TEST_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;

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
            "orkworks-windows-job-{name}-{}-{nanos}-{sequence}",
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
    LaunchSpec::fixture(role, behavior, marker.to_path_buf(), TARGET_LIFETIME)
        .expect("fixture launch specification should be valid")
}

fn prepared_supervisor() -> (Supervisor<OwnerDomain>, OwnerDomain) {
    let domain = OwnerDomain::create(GENERATION).expect("private job should be created");
    let (supervisor, prepared) = Supervisor::prepare_with_adapter(GENERATION, domain.clone())
        .expect("fixture generation should prepare");
    assert_eq!(prepared.record.generation, GENERATION);
    (supervisor, domain)
}

fn spawn(
    supervisor: &mut Supervisor<OwnerDomain>,
    role: Role,
    behavior: TargetBehavior,
    marker: &Path,
) -> NativeIdentity {
    let ticket = supervisor
        .issue_launch_ticket(role, format!("windows-{}", behavior.as_str()))
        .expect("ticket should be issued");
    supervisor
        .spawn(ticket, launch_spec(role, behavior, marker))
        .expect("root should be admitted to the private job")
}

fn descendant_marker(root: &Path, suffix: &str) -> PathBuf {
    let mut marker = root.as_os_str().to_owned();
    marker.push(format!(".{suffix}"));
    marker.into()
}

fn wait_until(description: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + TEST_WAIT_TIMEOUT;
    while !condition() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(condition(), "timed out waiting for {description}");
}

fn wait_for_marker(marker: &Path) {
    wait_until(&format!("marker {marker:?}"), || marker.exists());
}

fn wait_for_complete(supervisor: &mut Supervisor<OwnerDomain>) {
    wait_until("independent supervisor completion", || {
        is_complete(&supervisor.observe())
    });
}

fn launch_electron_like_parent(ready: &Path, completion: &Path, target_marker: &Path) -> Child {
    // The helper keeps an otherwise empty environment while allowing its fresh
    // process to initialize the Windows socket used as the control endpoint.
    let system_root = std::env::var_os("SystemRoot")
        .expect("SystemRoot should be available for Windows socket initialization");
    Command::new(fixture_executable())
        .args(electron_parent_arguments(ready, completion, target_marker))
        .env_clear()
        .env("SystemRoot", system_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Electron-like parent fixture should start")
}

fn read_json<T: DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&fs::read(path).expect("evidence file should be readable"))
        .expect("evidence file should contain valid JSON")
}

struct ForcedTopology {
    parent: Child,
    supervisor: Option<OwnedHandle>,
}

impl Drop for ForcedTopology {
    fn drop(&mut self) {
        if self.parent.try_wait().ok().flatten().is_none() {
            // SAFETY: Child retains the exact Electron-like process handle.
            unsafe {
                TerminateProcess(self.parent.as_raw_handle(), 137);
                WaitForSingleObject(self.parent.as_raw_handle(), 5_000);
            }
        }
        if let Some(supervisor) = &self.supervisor {
            // SAFETY: OpenProcess returned this exact supervisor process handle
            // with terminate and synchronize rights for test cleanup.
            unsafe {
                if WaitForSingleObject(supervisor.as_raw_handle(), 0) == WAIT_TIMEOUT {
                    TerminateProcess(supervisor.as_raw_handle(), 137);
                    WaitForSingleObject(supervisor.as_raw_handle(), 5_000);
                }
            }
        }
    }
}

#[test]
fn terminate_owned_waits_until_normal_cleanup_is_independently_empty() {
    let directory = TestDirectory::new("normal-cleanup");
    let marker = directory.marker("pty");
    let mut domain = OwnerDomain::create(GENERATION).expect("private job should be created");
    let root = domain
        .launch_paused(launch_spec(Role::Pty, TargetBehavior::Pty, &marker))
        .expect("root should be registered before release");
    wait_for_marker(&marker);
    assert!(domain
        .active_process_ids()
        .expect("job membership should be observable")
        .contains(&root.identity.diagnostic_pid));

    domain
        .terminate_owned()
        .expect("job termination should be observed complete");

    assert!(domain
        .active_process_ids()
        .expect("completed job should remain observable")
        .is_empty());
}

#[test]
fn closing_the_last_private_job_handle_kills_an_owned_root() {
    let directory = TestDirectory::new("kill-on-close");
    let marker = directory.marker("inference");
    let mut domain = OwnerDomain::create(GENERATION).expect("private job should be created");
    let root = domain
        .launch_paused(launch_spec(
            Role::Inference,
            TargetBehavior::Inference,
            &marker,
        ))
        .expect("root should be registered before release");
    wait_for_marker(&marker);
    // SAFETY: the live PID came from the retained birth identity, and the
    // resulting handle is used only to observe that exact process exit.
    let raw_process = unsafe {
        OpenProcess(
            SYNCHRONIZE_ACCESS | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            root.identity.diagnostic_pid,
        )
    };
    assert!(
        !raw_process.is_null(),
        "owned root should remain observable"
    );
    // SAFETY: OpenProcess returned one fresh owned handle.
    let process = unsafe { OwnedHandle::from_raw_handle(raw_process) };

    drop(domain);

    // SAFETY: `process` retains the exact root handle for this bounded wait.
    let wait = unsafe { WaitForSingleObject(process.as_raw_handle(), 5_000) };
    assert_eq!(
        wait, WAIT_OBJECT_0,
        "kill-on-close should terminate the owned root"
    );
}

#[test]
fn sidecar_first_crash_leaves_pty_and_inference_descendants_owned() {
    let directory = TestDirectory::new("sidecar-first");
    let marker = directory.marker("sidecar");
    let mut supervisor_and_domain = prepared_supervisor();
    let sidecar = spawn(
        &mut supervisor_and_domain.0,
        Role::Sidecar,
        TargetBehavior::Forked,
        &marker,
    );
    for suffix in ["pty", "inference", "nested-inference"] {
        wait_for_marker(&descendant_marker(&marker, suffix));
    }
    let before = supervisor_and_domain
        .1
        .active_process_ids()
        .expect("job membership should be observable");
    assert!(
        before.len() >= 4,
        "expected root plus three descendants: {before:?}"
    );

    supervisor_and_domain
        .1
        .terminate_registered(&sidecar)
        .expect("sidecar root should be force-terminated");
    wait_until("sidecar root exit", || {
        !supervisor_and_domain
            .1
            .registered_identity_is_alive(&sidecar)
            .expect("registered identity should be observable")
    });

    let survivors = supervisor_and_domain
        .1
        .active_process_ids()
        .expect("descendant membership should remain observable");
    assert!(
        survivors.len() >= 3,
        "sidecar exit must not release its descendants from the job: {survivors:?}"
    );
    supervisor_and_domain
        .1
        .terminate_owned()
        .expect("remaining descendants should terminate with the job");
    wait_for_complete(&mut supervisor_and_domain.0);
}

#[test]
fn pty_like_new_process_group_and_nested_inference_remain_in_the_job() {
    let directory = TestDirectory::new("nested-descendants");
    let marker = directory.marker("sidecar");
    let (mut supervisor, domain) = prepared_supervisor();
    spawn(
        &mut supervisor,
        Role::Sidecar,
        TargetBehavior::Forked,
        &marker,
    );
    for suffix in ["pty", "inference", "nested-inference"] {
        wait_for_marker(&descendant_marker(&marker, suffix));
    }

    let members = domain
        .active_process_ids()
        .expect("job process census should succeed");

    assert!(
        members.len() >= 4,
        "the private job should contain sidecar, PTY-like, inference, and nested inference processes: {members:?}"
    );
    domain
        .terminate_owned()
        .expect("all nested job members should terminate");
    wait_for_complete(&mut supervisor);
}

#[test]
fn breakaway_is_rejected_when_the_private_job_allows_no_breakaway() {
    let domain = OwnerDomain::create(GENERATION).expect("private job should be created");

    let result = domain.attempt_breakaway();

    assert_eq!(result, BreakawayResult::Rejected);
    domain
        .terminate_owned()
        .expect("breakaway probe root should terminate with the job");
}

#[test]
fn job_process_thread_and_supervisor_endpoint_handles_are_non_inheritable() {
    let directory = TestDirectory::new("handle-inheritance");
    let marker = directory.marker("pty");
    let (mut supervisor, domain) = prepared_supervisor();
    let endpoint = supervisor
        .control_endpoint_addr()
        .expect("supervisor endpoint should be live");
    spawn(&mut supervisor, Role::Pty, TargetBehavior::Pty, &marker);
    wait_for_marker(&marker);

    assert!(domain
        .handles_are_non_inheritable()
        .expect("supervisor handle flags should be observable"));
    supervisor.owner_lost();
    let rebound = TcpListener::bind(endpoint)
        .expect("owned targets must not retain the supervisor endpoint after the owner-side close");

    drop(rebound);
    domain
        .terminate_owned()
        .expect("owned target should terminate with the job");
    wait_for_complete(&mut supervisor);
}

#[test]
fn invalid_or_replayed_ticket_cannot_release_target_or_change_admission_observation() {
    let directory = TestDirectory::new("ticket-replay");
    let invalid_marker = directory.marker("invalid-target");
    let admitted_marker = directory.marker("admitted-target");
    let replay_marker = directory.marker("replayed-target");
    let (mut supervisor, domain) = prepared_supervisor();
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "windows-ticket-replay")
        .expect("ticket should be issued");

    let mut invalid_ticket = ticket.clone();
    invalid_ticket.ticket_id.push_str("-forged");
    assert_eq!(
        supervisor.spawn(
            invalid_ticket,
            launch_spec(Role::Inference, TargetBehavior::Inference, &invalid_marker),
        ),
        Err(SpawnError::InvalidTicket)
    );
    assert!(
        !invalid_marker.exists(),
        "an invalid ticket must not release a target"
    );
    assert!(
        domain
            .active_process_ids()
            .expect("invalid-ticket census should be observable")
            .is_empty(),
        "an invalid ticket must not create an owned target"
    );

    let identity = supervisor
        .spawn(
            ticket.clone(),
            launch_spec(Role::Inference, TargetBehavior::Inference, &admitted_marker),
        )
        .expect("the original ticket should admit one target");
    wait_for_marker(&admitted_marker);
    assert!(domain
        .admission_observation(&identity)
        .expect("admission observation should be available"));
    let members_before_replay = domain
        .active_process_ids()
        .expect("pre-replay census should be observable");

    assert_eq!(
        supervisor.spawn(
            ticket,
            launch_spec(Role::Inference, TargetBehavior::Inference, &replay_marker),
        ),
        Err(SpawnError::InvalidTicket)
    );
    thread::sleep(Duration::from_millis(250));
    assert!(
        !replay_marker.exists(),
        "a replayed ticket must not release a target"
    );
    assert_eq!(
        domain
            .active_process_ids()
            .expect("post-replay census should be observable"),
        members_before_replay,
        "a replayed ticket must not change the owned process census"
    );
    assert!(domain
        .admission_observation(&identity)
        .expect("admission observation should remain available"));

    domain
        .terminate_owned()
        .expect("ticket replay coverage should clean up the admitted target");
}

#[test]
fn registration_failure_never_releases_the_suspended_root() {
    let directory = TestDirectory::new("registration-failure");
    let marker = directory.marker("must-not-run");
    let (mut supervisor, domain) = prepared_supervisor();
    supervisor.inject_registration_failure_once();
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "windows-registration-failure")
        .expect("ticket should be issued");

    let error = supervisor
        .spawn(
            ticket,
            launch_spec(Role::Inference, TargetBehavior::Inference, &marker),
        )
        .expect_err("injected registration failure should reject the spawn");

    assert_eq!(error, SpawnError::ObserverUnavailable);
    thread::sleep(Duration::from_millis(250));
    assert!(!marker.exists(), "the suspended target must never execute");
    let active_processes = domain
        .active_process_ids()
        .expect("failed-admission job census should remain observable");
    let observation = supervisor.observe();
    let conservatively_unresolved = !observation.unresolved_survivors.is_empty()
        || !observation.unidentified_roots.is_empty()
        || observation
            .roots
            .iter()
            .any(|root| root.exit_state == ExitState::Unresolved);
    assert!(
        active_processes.is_empty() || conservatively_unresolved,
        "the suspended root must be independently exited or retained as unresolved: active={active_processes:?}, observation={observation:?}"
    );
    domain
        .terminate_owned()
        .expect("failed-admission cleanup should leave no active job processes");
}

#[test]
fn forced_electron_parent_termination_leaves_supervisor_to_empty_its_job() {
    let directory = TestDirectory::new("forced-parent");
    let ready_path = directory.marker("supervisor-ready.json");
    let completion_path = directory.marker("supervisor-complete.json");
    let owned_marker = directory.marker("owned-inference");
    let parent = launch_electron_like_parent(&ready_path, &completion_path, &owned_marker);
    let mut topology = ForcedTopology {
        parent,
        supervisor: None,
    };
    wait_for_marker(&ready_path);
    wait_for_marker(&owned_marker);
    let ready: ForcedParentReady = read_json(&ready_path);
    assert_ne!(ready.supervisor_pid, topology.parent.id());
    assert_ne!(ready.owned_target_pid, topology.parent.id());
    assert_ne!(ready.owned_target_pid, ready.supervisor_pid);
    assert!(
        ready.target_admitted_through_supervisor_ticket,
        "the forced-parent target must be admitted through the supervisor ticket"
    );
    assert!(
        ready.endpoint_non_inheritable_before_release,
        "the supervisor endpoint must reject inheritance before target release"
    );
    assert!(
        ready.handles_non_inheritable,
        "the out-of-process supervisor must retain only non-inheritable authority handles"
    );
    // SAFETY: the supervisor PID was published while that helper is blocked on
    // its parent-owned pipe. The returned exact handle is retained from here on.
    let raw_supervisor = unsafe {
        OpenProcess(
            SYNCHRONIZE_ACCESS | PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
            0,
            ready.supervisor_pid,
        )
    };
    assert!(!raw_supervisor.is_null(), "supervisor should be live");
    // SAFETY: OpenProcess returned one fresh owned handle.
    topology.supervisor = Some(unsafe { OwnedHandle::from_raw_handle(raw_supervisor) });
    // SAFETY: the retained supervisor handle is live for this zero-time wait.
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                topology
                    .supervisor
                    .as_ref()
                    .expect("supervisor handle should be retained")
                    .as_raw_handle(),
                0,
            )
        },
        WAIT_TIMEOUT,
        "supervisor should be alive before its parent is terminated"
    );

    // SAFETY: `parent` retains the exact live Electron-like process handle.
    let terminated = unsafe { TerminateProcess(topology.parent.as_raw_handle(), 137) };
    assert_ne!(
        terminated, 0,
        "TerminateProcess should kill the actual supervisor parent"
    );
    wait_until("Electron-like parent exit", || {
        topology
            .parent
            .try_wait()
            .expect("parent exit should remain observable")
            .is_some()
    });
    wait_for_marker(&completion_path);
    let completion: ForcedParentCompletion = read_json(&completion_path);

    assert_eq!(completion.supervisor_pid, ready.supervisor_pid);
    assert!(completion.owner_loss_observed);
    assert!(
        completion.endpoint_rebound_while_target_live,
        "the owned target must not inherit the supervisor endpoint"
    );
    assert_eq!(completion.active_processes_after_termination, 0);
    // SAFETY: completion is written immediately before the supervisor exits;
    // this retained exact handle observes that exit without PID lookup.
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                topology
                    .supervisor
                    .as_ref()
                    .expect("supervisor handle should be retained")
                    .as_raw_handle(),
                5_000,
            )
        },
        WAIT_OBJECT_0,
        "supervisor should exit only after publishing zero active processes"
    );
}
