use super::*;
use crate::process_job::{peek_exit, ProcessJob};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use windows_sys::Win32::Foundation::{ERROR_INVALID_PARAMETER, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};

const CHILD_SCRIPT: &str = r#"
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $env:ComSpec
$psi.Arguments = '/c ping -t 127.0.0.1 >nul'
$psi.UseShellExecute = $false
if ($env:ORK_PROBE_FIXTURE_MODE -eq 'exit') {
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $child = [System.Diagnostics.Process]::Start($psi)
    $child.StandardInput.Close()
} else {
    $nativeLauncher = @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
public static class ProbeFixtureNativeLauncher {
    [StructLayout(LayoutKind.Sequential)]
    private struct StartupInfo {
        public uint cb;
        public IntPtr reserved;
        public IntPtr desktop;
        public IntPtr title;
        public uint x, y, xSize, ySize, xCountChars, yCountChars, fillAttribute, flags;
        public ushort showWindow, reserved2Length;
        public IntPtr reserved2, stdInput, stdOutput, stdError;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct ProcessInformation {
        public IntPtr process, thread;
        public uint processId, threadId;
    }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CreateProcess(string application, StringBuilder commandLine,
        IntPtr processAttributes, IntPtr threadAttributes, bool inheritHandles, uint flags,
        IntPtr environment, string currentDirectory, ref StartupInfo startup,
        out ProcessInformation processInformation);
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);
    public static uint Start() {
        var startup = new StartupInfo();
        startup.cb = (uint)Marshal.SizeOf(typeof(StartupInfo));
        startup.flags = 0x00000100;
        startup.stdInput = GetStdHandle(-10);
        startup.stdOutput = GetStdHandle(-11);
        startup.stdError = GetStdHandle(-12);
        var executable = Environment.GetEnvironmentVariable("ComSpec");
        var commandLine = new StringBuilder("\"" + executable + "\" /c ping -t 127.0.0.1 >nul");
        ProcessInformation process;
        if (!CreateProcess(executable, commandLine, IntPtr.Zero, IntPtr.Zero, true,
            0x08000000, IntPtr.Zero, null, ref startup, out process)) {
            throw new Win32Exception(Marshal.GetLastWin32Error());
        }
        CloseHandle(process.thread);
        CloseHandle(process.process);
        return process.processId;
    }
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GetStdHandle(int standardHandle);
}
'@
    Add-Type -TypeDefinition $nativeLauncher -Language CSharp
    $nativePid = [ProbeFixtureNativeLauncher]::Start()
}
$childPid = if ($env:ORK_PROBE_FIXTURE_MODE -eq 'exit') { $child.Id } else { $nativePid }
[System.IO.File]::WriteAllText($env:ORK_PROBE_FIXTURE_PID, [string]$childPid)
if ($env:ORK_PROBE_FIXTURE_MODE -eq 'exit' -or $env:ORK_PROBE_FIXTURE_MODE -eq 'cap') {
    while (!(Test-Path $env:ORK_PROBE_FIXTURE_RELEASE)) {
        Start-Sleep -Milliseconds 10
    }
}
if ($env:ORK_PROBE_FIXTURE_MODE -eq 'exit') {
    [Console]::Out.WriteLine('fixture-marker')
    exit 0
}
if ($env:ORK_PROBE_FIXTURE_MODE -eq 'cap') {
    [Console]::Out.WriteLine(('x' * 8192))
}
Start-Sleep -Seconds 30
"#;

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
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", CHILD_SCRIPT])
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
        let handle = unsafe {
            OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, 0, pid)
        };
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
                    if let Some(process) = FixtureProcess::open(pid).expect("open fixture process") {
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
                    max_bytes: 64,
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
        Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 30"])
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
    assert!(matches!(sentinel.0.try_wait(), Ok(None)), "unrelated sentinel stays alive");
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
