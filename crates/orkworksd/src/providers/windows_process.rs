//! Own a suspended provider's process tree before allowing provider code to run.
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::Child;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::{
    Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    },
    JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    },
    Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
};

pub(super) struct ProcessJob(OwnedHandle);

impl ProcessJob {
    pub(super) fn new() -> io::Result<Self> {
        // SAFETY: null attributes/name request a private, non-inheritable job.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful creation transfers one valid, owned handle.
        let job = Self(unsafe { OwnedHandle::from_raw_handle(handle) });
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: the live job and correctly sized initialized limits remain valid throughout.
        if unsafe {
            SetInformationJobObject(
                job.0.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }

    pub(super) fn attach_and_resume(&self, child: &Child) -> io::Result<()> {
        // SAFETY: both handles are live and owned, and child was spawned suspended.
        if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), child.as_raw_handle()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // Rust Child exposes the process handle, but not its initial thread.
        // Enumerate that suspended process's thread; no provider code has run yet.
        // SAFETY: TH32CS_SNAPTHREAD takes no pointers and ignores the process ID.
        let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful snapshot creation returns one owned kernel handle.
        let threads = unsafe { OwnedHandle::from_raw_handle(handle) };
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        // SAFETY: entry is writable with the required dwSize, and the snapshot is live.
        let mut found = unsafe { Thread32First(threads.as_raw_handle(), &mut entry) };
        while found != 0 {
            if entry.th32OwnerProcessID == child.id() {
                // SAFETY: use only the thread belonging to our still-owned suspended child.
                let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
                if thread.is_null() {
                    return Err(io::Error::last_os_error());
                }
                // SAFETY: OpenThread returned a fresh owned handle.
                let thread = unsafe { OwnedHandle::from_raw_handle(thread) };
                // SAFETY: this handle has THREAD_SUSPEND_RESUME access.
                return match unsafe { ResumeThread(thread.as_raw_handle()) } {
                    1 => Ok(()),
                    u32::MAX => Err(io::Error::last_os_error()),
                    _ => Err(io::Error::other(
                        "unexpected provider thread suspension state",
                    )),
                };
            }
            entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            // SAFETY: entry and snapshot remain valid for enumeration.
            found = unsafe { Thread32Next(threads.as_raw_handle(), &mut entry) };
        }
        Err(io::Error::other("suspended provider thread unavailable"))
    }

    pub(super) fn terminate(&self) -> io::Result<()> {
        // SAFETY: the owned job includes only this invocation's process tree.
        if unsafe { TerminateJobObject(self.0.as_raw_handle(), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}
