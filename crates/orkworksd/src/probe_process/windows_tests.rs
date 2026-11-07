use super::*;
use crate::process_job::{peek_exit, ProcessJob};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use windows_sys::Win32::Foundation::{ERROR_INVALID_PARAMETER, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};

const NATIVE_FIXTURE: &str = "probe_process::windows_tests::native_probe_fixture";

fn native_fixture_command(role: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", NATIVE_FIXTURE, "--nocapture", "--test-threads=1"])
        .env("ORK_PROBE_FIXTURE_ROLE", role);
    command
}

#[test]
fn native_probe_fixture() {
    let Some(role) = std::env::var_os("ORK_PROBE_FIXTURE_ROLE") else {
        return;
    };
    match role.to_str().unwrap() {
        "success" => std::process::exit(0),
        "marker" => {
            std::fs::write(std::env::var_os("ORK_PROBE_MARKER").unwrap(), b"ran").unwrap();
            std::process::exit(0);
        }
        "child" => {
            std::fs::write(
                std::env::var_os("ORK_PROBE_FIXTURE_PID").unwrap(),
                std::process::id().to_string(),
            )
            .unwrap();
        }
        "wrapper" => {
            use std::io::Write;
            let mode = std::env::var("ORK_PROBE_FIXTURE_MODE").unwrap();
            let mut child = native_fixture_command("child");
            child.stdin(Stdio::null());
            if mode == "exit" {
                // Null child stdio alone does not clear the wrapper's other
                // inheritable handle-table entries. Keep its capture writers
                // private so this fixture truly closes capture on root exit.
                for handle in [
                    std::io::stdout().as_raw_handle(),
                    std::io::stderr().as_raw_handle(),
                ] {
                    // SAFETY: both live standard handles belong to this fixture.
                    assert_ne!(
                        unsafe {
                            windows_sys::Win32::Foundation::SetHandleInformation(
                                handle,
                                windows_sys::Win32::Foundation::HANDLE_FLAG_INHERIT,
                                0,
                            )
                        },
                        0
                    );
                }
                child.stdout(Stdio::null()).stderr(Stdio::null());
            } else {
                // Real inherited capture writers, independently held by the
                // non-exec descendant until native Job cleanup terminates it.
                child.stdout(Stdio::inherit()).stderr(Stdio::inherit());
            }
            let _child = child.spawn().unwrap();
            if mode == "exit" || mode == "cap" {
                let release = std::path::PathBuf::from(
                    std::env::var_os("ORK_PROBE_FIXTURE_RELEASE").unwrap(),
                );
                while !release.exists() {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
            if mode == "exit" {
                println!("fixture-marker");
                std::io::stdout().flush().unwrap();
                std::process::exit(0);
            }
            if mode == "cap" {
                std::io::stdout().write_all(&[b'x'; 8192]).unwrap();
                std::io::stdout().flush().unwrap();
            }
        }
        "idle" => {}
        _ => panic!("unknown synthetic fixture role"),
    }
    std::thread::sleep(Duration::from_secs(30));
    std::process::exit(0);
}

struct FixtureFiles {
    pid: std::path::PathBuf,
    release: std::path::PathBuf,
}

impl FixtureFiles {
    fn new() -> Self {
        let prefix = format!("orkworks-probe-{}", uuid::Uuid::new_v4());
        Self {
            pid: std::env::temp_dir().join(format!("{prefix}.pid")),
            release: std::env::temp_dir().join(format!("{prefix}.release")),
        }
    }

    fn release_wrapper(&self) {
        std::fs::write(&self.release, b"release").expect("release fixture wrapper");
    }
}

impl Drop for FixtureFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.pid);
        let _ = std::fs::remove_file(&self.release);
    }
}

fn fixture_command(mode: &str, files: &FixtureFiles) -> Command {
    let mut command = native_fixture_command("wrapper");
    command
        .env("ORK_PROBE_FIXTURE_MODE", mode)
        .env("ORK_PROBE_FIXTURE_PID", &files.pid)
        .env("ORK_PROBE_FIXTURE_RELEASE", &files.release)
        .stdin(Stdio::null());
    command
}

struct Sentinel(Child);

impl Drop for Sentinel {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct ProbeTask<T>(Option<tokio::task::JoinHandle<T>>);

impl<T> ProbeTask<T> {
    fn spawn(future: impl std::future::Future<Output = T> + Send + 'static) -> Self
    where
        T: Send + 'static,
    {
        Self(Some(tokio::spawn(future)))
    }

    fn abort(&self) {
        if let Some(task) = &self.0 {
            task.abort();
        }
    }

    async fn join(mut self) -> Result<T, tokio::task::JoinError> {
        self.0.take().expect("probe task handle exists").await
    }
}

impl<T> Drop for ProbeTask<T> {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
        }
    }
}

struct FixtureProcess(OwnedHandle);

impl FixtureProcess {
    fn open(pid: u32) -> std::io::Result<Option<Self>> {
        // SAFETY: request access only to the fixture PID read from its private marker file.
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, 0, pid) };
        if handle.is_null() {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
                return Ok(None);
            }
            return Err(error);
        }
        // SAFETY: OpenProcess returned one owned kernel handle.
        Ok(Some(Self(unsafe { OwnedHandle::from_raw_handle(handle) })))
    }

    fn wait_for_exit(&self) {
        // SAFETY: the exact fixture process handle remains live during this bounded wait.
        assert_eq!(
            unsafe { WaitForSingleObject(self.0.as_raw_handle(), 5_000) },
            WAIT_OBJECT_0,
            "owned descendant should exit within the bounded fixture wait"
        );
    }
}

impl Drop for FixtureProcess {
    fn drop(&mut self) {
        // SAFETY: this is the exact process handle opened from the fixture's private PID marker.
        let _ = unsafe { TerminateProcess(self.0.as_raw_handle(), 1) };
        // SAFETY: bounded wait on the same handle; cleanup never searches by name or PID.
        let _ = unsafe { WaitForSingleObject(self.0.as_raw_handle(), 5_000) };
    }
}

async fn wait_for_fixture_process(files: &FixtureFiles) -> FixtureProcess {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(pid) = std::fs::read_to_string(&files.pid) {
                if let Ok(pid) = pid.trim().parse() {
                    if let Some(process) = FixtureProcess::open(pid).expect("open fixture process")
                    {
                        return process;
                    }
                    panic!("fixture descendant exited before the owner outcome");
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("fixture child should publish its PID while still alive")
}

#[tokio::test]
async fn timeout_terminates_non_exec_descendant_holding_capture_pipe() {
    let files = FixtureFiles::new();
    let mut command = fixture_command("timeout", &files);
    let service = ProbeService::new().expect("create probe service");
    let run = ProbeTask::spawn(async move {
        service
            .run(
                &mut command,
                ProbePolicy {
                    max_bytes: 64 * 1024,
                    reject_overflow: false,
                    require_success: false,
                },
                Duration::from_secs(2),
            )
            .await
    });

    let fixture = wait_for_fixture_process(&files).await;
    let result = run.join().await.expect("probe task completes");
    assert!(matches!(result, Err(ProbeError::Timeout)));
    fixture.wait_for_exit();
}

#[tokio::test]
async fn output_cap_terminates_non_exec_descendant_holding_capture_pipe() {
    let files = FixtureFiles::new();
    let mut command = fixture_command("cap", &files);
    let service = ProbeService::new().expect("create probe service");
    let run = ProbeTask::spawn(async move {
        service
            .run(
                &mut command,
                ProbePolicy {
                    max_bytes: 1024,
                    reject_overflow: true,
                    require_success: false,
                },
                Duration::from_secs(5),
            )
            .await
    });

    let fixture = wait_for_fixture_process(&files).await;
    files.release_wrapper();
    let result = run.join().await.expect("probe task completes");
    assert!(matches!(result, Err(ProbeError::Overflow)));
    fixture.wait_for_exit();
}

#[tokio::test]
async fn cancellation_terminates_only_owned_descendant() {
    let files = FixtureFiles::new();
    let mut command = fixture_command("timeout", &files);
    let service = ProbeService::new().expect("create probe service");
    let run = ProbeTask::spawn(async move {
        service
            .run(
                &mut command,
                ProbePolicy {
                    max_bytes: 64 * 1024,
                    reject_overflow: false,
                    require_success: false,
                },
                Duration::from_secs(30),
            )
            .await
    });

    let fixture = wait_for_fixture_process(&files).await;
    run.abort();
    let _ = run.join().await;
    fixture.wait_for_exit();
}

#[tokio::test]
async fn successful_wrapper_exit_terminates_descendant_and_leaves_sentinel_alive() {
    let files = FixtureFiles::new();
    let mut sentinel = Sentinel(
        native_fixture_command("idle")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start unrelated sentinel"),
    );

    let mut command = fixture_command("exit", &files);
    let service = ProbeService::new().expect("create probe service");
    let run = ProbeTask::spawn(async move {
        service
            .run(
                &mut command,
                ProbePolicy {
                    max_bytes: 64 * 1024,
                    reject_overflow: false,
                    require_success: true,
                },
                Duration::from_secs(5),
            )
            .await
    });

    let fixture = wait_for_fixture_process(&files).await;
    files.release_wrapper();
    let result = run.join().await.expect("probe task completes");
    let output = result.expect("successful wrapper keeps successful output policy");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("fixture-marker"),
        "wrapper output is captured before successful completion"
    );
    fixture.wait_for_exit();
    assert!(
        matches!(sentinel.0.try_wait(), Ok(None)),
        "unrelated sentinel stays alive"
    );
}

#[tokio::test]
async fn peek_exit_observes_success_without_reaping_owned_child() {
    let mut child = native_fixture_command("success")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start success fixture");

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(success) = peek_exit(&child).expect("observe child exit") {
                assert!(success);
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("fixture should exit");
    assert!(child.wait().expect("reap after peek").success());
}

#[test]
fn owned_process_job_reports_resume_failure_without_losing_termination_authority() {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;

    let mut child = native_fixture_command("idle")
        .creation_flags(CREATE_SUSPENDED)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start suspended fixture");
    let job = ProcessJob::new().expect("create owned job");
    job.attach_and_resume(&child)
        .expect("assign and resume fixture");
    let error = match job.attach_and_resume(&child) {
        Ok(()) => panic!("resuming an already running thread should fail"),
        Err(error) => error,
    };
    assert_eq!(
        error.to_string(),
        "unexpected provider thread suspension state"
    );
    job.terminate()
        .expect("job still owns fixture after resume failure");
    let _ = child.wait();
}

#[tokio::test(flavor = "current_thread")]
async fn assignment_denial_never_runs_configured_code_and_reaps_suspended_child() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("unexpected");
    let service = ProbeService::new().unwrap();
    let mut command = native_fixture_command("marker");
    command.env("ORK_PROBE_MARKER", &marker);
    let failure = crate::process_job::fail_assignment_for_test();
    assert_eq!(
        service
            .run(
                &mut command,
                ProbePolicy {
                    max_bytes: 64,
                    reject_overflow: true,
                    require_success: true
                },
                Duration::from_secs(3)
            )
            .await
            .unwrap_err(),
        ProbeError::Setup
    );
    drop(failure);
    service.shutdown().await;
    assert!(
        !marker.exists(),
        "configured code ran before assignment succeeded"
    );
    assert_eq!(
        service.shared.permits.available_permits(),
        super::OWNER_LIMIT
    );
}
