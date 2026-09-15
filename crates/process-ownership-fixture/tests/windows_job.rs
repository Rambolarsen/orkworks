#![cfg(windows)]

use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use process_ownership_fixture::observation::is_complete;
use process_ownership_fixture::platform::{BreakawayResult, OwnerDomain};
use process_ownership_fixture::protocol::{NativeIdentity, Role};
use process_ownership_fixture::supervisor::{LaunchSpec, Supervisor};
use process_ownership_fixture::targets::{self, TargetBehavior, RELEASE_EXEC_BYTE};
use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
use windows_sys::Win32::System::Threading::{
    OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
};

const GENERATION: u64 = 23;
const TARGET_LIFETIME: Duration = Duration::from_secs(30);
const WAIT_TIMEOUT: Duration = Duration::from_secs(5);
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
    let deadline = Instant::now() + WAIT_TIMEOUT;
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

fn launch_electron_like_parent(marker: &Path) -> Child {
    let mut child = Command::new(fixture_executable())
        .args(targets::arguments(
            Role::Sidecar,
            TargetBehavior::Silent,
            marker,
            TARGET_LIFETIME,
        ))
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Electron-like parent fixture should start");
    child
        .stdin
        .take()
        .expect("parent release gate should exist")
        .write_all(&[RELEASE_EXEC_BYTE])
        .expect("parent release gate should open");
    child
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
fn forced_electron_parent_termination_does_not_kill_the_live_supervisor_domain() {
    let directory = TestDirectory::new("forced-parent");
    let parent_marker = directory.marker("electron-parent");
    let owned_marker = directory.marker("owned-inference");
    let (mut supervisor, domain) = prepared_supervisor();
    spawn(
        &mut supervisor,
        Role::Inference,
        TargetBehavior::Inference,
        &owned_marker,
    );
    wait_for_marker(&owned_marker);
    let mut parent = launch_electron_like_parent(&parent_marker);
    wait_for_marker(&parent_marker);

    // SAFETY: `parent` retains a valid process handle for the live child.
    let terminated = unsafe { TerminateProcess(parent.as_raw_handle(), 137) };
    assert_ne!(
        terminated, 0,
        "TerminateProcess should kill the parent fixture"
    );
    wait_until("Electron-like parent exit", || {
        parent
            .try_wait()
            .expect("parent exit should remain observable")
            .is_some()
    });

    assert!(
        !domain
            .active_process_ids()
            .expect("live supervisor should still observe its job")
            .is_empty(),
        "the supervisor domain must remain live after forced parent termination"
    );
    domain
        .terminate_owned()
        .expect("surviving supervisor should still clean its owned job");
    wait_for_complete(&mut supervisor);
}
