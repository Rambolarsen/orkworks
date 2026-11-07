use super::*;
use crate::process_job::{peek_exit, ProcessJob};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::{Command, Stdio};
use std::time::Duration;
use windows_sys::Win32::System::{
    Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
};
use windows_sys::Win32::Foundation::WAIT_OBJECT_0;

const CHILD_SCRIPT: &str = r#"
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $env:ComSpec
$psi.Arguments = '/c ping -t 127.0.0.1 >nul'
$psi.UseShellExecute = $false
$child = [System.Diagnostics.Process]::Start($psi)
[System.IO.File]::WriteAllText($env:ORK_PROBE_FIXTURE_PID, [string]$child.Id)
if ($env:ORK_PROBE_FIXTURE_MODE -eq 'exit') {
    [Console]::Out.WriteLine('fixture-marker')
    exit 0
}
if ($env:ORK_PROBE_FIXTURE_MODE -eq 'cap') {
    [Console]::Out.WriteLine(('x' * 8192))
}
Start-Sleep -Seconds 30
"#;

fn fixture_command(mode: &str, marker: &std::path::Path) -> Command {
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", CHILD_SCRIPT])
        .env("ORK_PROBE_FIXTURE_MODE", mode)
        .env("ORK_PROBE_FIXTURE_PID", marker)
        .stdin(Stdio::null());
    command
}

fn fixture_marker() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("orkworks-probe-{}.pid", uuid::Uuid::new_v4()))
}

struct Sentinel(std::process::Child);

impl Drop for Sentinel {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn wait_for_fixture_pid(marker: &std::path::Path) -> u32 {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(pid) = std::fs::read_to_string(marker) {
                if let Ok(pid) = pid.trim().parse() {
                    return pid;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("fixture child should publish its PID")
}

fn wait_for_fixture_exit(pid: u32) {
    // SAFETY: opens only the exact fixture PID read from its private marker file.
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    if handle.is_null() {
        // OpenProcess reports an invalid PID when the fixture has already exited.
        return;
    }
    // SAFETY: OpenProcess returned one owned kernel handle.
    let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
    // SAFETY: the handle remains live for the bounded wait.
    assert_eq!(
        unsafe { WaitForSingleObject(handle.as_raw_handle(), 5_000) },
        WAIT_OBJECT_0,
        "owned descendant should exit within the bounded fixture wait"
    );
}

#[tokio::test]
async fn timeout_terminates_non_exec_descendant_holding_capture_pipe() {
    let marker = fixture_marker();
    let mut command = fixture_command("timeout", &marker);
    let service = ProbeService::new().expect("create probe service");
    let result = service
        .run(
            &mut command,
            ProbePolicy {
                max_bytes: 64 * 1024,
                reject_overflow: false,
                require_success: false,
            },
            Duration::from_secs(2),
        )
        .await;

    let child_pid = wait_for_fixture_pid(&marker).await;
    assert!(matches!(result, Err(ProbeError::Timeout)));
    wait_for_fixture_exit(child_pid);
    let _ = std::fs::remove_file(marker);
}

#[tokio::test]
async fn output_cap_terminates_non_exec_descendant_holding_capture_pipe() {
    let marker = fixture_marker();
    let mut command = fixture_command("cap", &marker);
    let service = ProbeService::new().expect("create probe service");
    let result = service
        .run(
            &mut command,
            ProbePolicy {
                max_bytes: 64,
                reject_overflow: true,
                require_success: false,
            },
            Duration::from_secs(5),
        )
        .await;

    let child_pid = wait_for_fixture_pid(&marker).await;
    assert!(matches!(result, Err(ProbeError::Overflow)));
    wait_for_fixture_exit(child_pid);
    let _ = std::fs::remove_file(marker);
}

#[tokio::test]
async fn cancellation_terminates_only_owned_descendant() {
    let marker = fixture_marker();
    let mut command = fixture_command("timeout", &marker);
    let service = ProbeService::new().expect("create probe service");
    let run = tokio::spawn(async move {
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
    let child_pid = wait_for_fixture_pid(&marker).await;
    run.abort();
    let _ = run.await;
    wait_for_fixture_exit(child_pid);
    let _ = std::fs::remove_file(marker);
}

#[tokio::test]
async fn successful_wrapper_exit_terminates_descendant_and_leaves_sentinel_alive() {
    let marker = fixture_marker();
    let mut sentinel = Sentinel(Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 30"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start unrelated sentinel"));

    let mut command = fixture_command("exit", &marker);
    let service = ProbeService::new().expect("create probe service");
    let result = service
        .run(
            &mut command,
            ProbePolicy {
                max_bytes: 64 * 1024,
                reject_overflow: false,
                require_success: true,
            },
            Duration::from_secs(5),
        )
        .await;
    let child_pid = wait_for_fixture_pid(&marker).await;
    assert!(result.is_ok(), "successful wrapper keeps successful output policy");
    wait_for_fixture_exit(child_pid);
    assert!(matches!(sentinel.0.try_wait(), Ok(None)), "unrelated sentinel stays alive");
    let _ = std::fs::remove_file(marker);
}

#[tokio::test]
async fn peek_exit_observes_success_without_reaping_owned_child() {
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
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

    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 30"])
        .creation_flags(CREATE_SUSPENDED)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start suspended fixture");
    let job = ProcessJob::new().expect("create owned job");
    job.attach_and_resume(&child).expect("assign and resume fixture");
    let error = match job.attach_and_resume(&child) {
        Ok(()) => panic!("resuming an already running thread should fail"),
        Err(error) => error,
    };
    assert_eq!(error.to_string(), "unexpected provider thread suspension state");
    job.terminate().expect("job still owns fixture after resume failure");
    let _ = child.wait();
}
