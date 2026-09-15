//! Independent foreign workspace owner used by the cleanup isolation fixture.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(20);
static OWNER_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Filesystem state observed for an owner outside the cleanup attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignSnapshot {
    /// Monotonic heartbeat counter written by the foreign owner thread.
    pub heartbeat: u64,
    /// Stable identity retained in the workspace lease.
    pub lease_owner: String,
    /// Exact metadata bytes written by the foreign owner.
    pub metadata_bytes: Vec<u8>,
    /// Metadata revision encoded in the metadata snapshot.
    pub metadata_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtomicOwnerState {
    heartbeat: u64,
    lease_owner: String,
    metadata_bytes: Vec<u8>,
}

/// Errors produced while creating or observing the foreign owner fixture.
#[derive(Debug, Error)]
pub enum ForeignOwnerError {
    /// A filesystem operation failed.
    #[error("foreign owner filesystem operation failed: {0}")]
    Io(#[from] io::Error),
    /// A persisted foreign-owner value was malformed.
    #[error("foreign owner state is malformed: {0}")]
    Malformed(&'static str),
}

/// Foreign owner holding an OS advisory lease and atomically updating state.
pub struct ForeignOwner {
    workspace: PathBuf,
    lease_path: PathBuf,
    state_path: PathBuf,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    /// Retains the open, advisory-locked lease descriptor for the owner lifetime.
    lease: Option<File>,
    #[cfg(windows)]
    lease_lock: Option<Box<windows_sys::Win32::System::IO::OVERLAPPED>>,
}

impl ForeignOwner {
    /// Acquires the workspace lease, writes known metadata, and starts heartbeat updates.
    pub fn start(
        workspace: impl AsRef<Path>,
        metadata_revision: u64,
    ) -> Result<(Self, ForeignSnapshot), ForeignOwnerError> {
        let workspace = workspace.as_ref().to_path_buf();
        fs::create_dir_all(&workspace)?;
        let sequence = OWNER_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let owner = format!("foreign-owner-{}-{sequence}", std::process::id());
        let lease_path = workspace.join(".orkworks-foreign-lease");
        let state_path = workspace.join(".orkworks-foreign-state");
        let mut lease = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lease_path)?;
        lease.write_all(owner.as_bytes())?;
        lease.sync_all()?;
        let initial = AtomicOwnerState {
            heartbeat: 0,
            lease_owner: owner.clone(),
            metadata_bytes: format!("owner={owner}\nrevision={metadata_revision}\n").into_bytes(),
        };
        atomic_write_state(&state_path, &initial)?;
        #[cfg(unix)]
        lock_lease(&lease)?;
        #[cfg(windows)]
        let lease_lock = Some(lock_lease(&lease)?);

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_state_path = state_path.clone();
        let worker_owner = owner;
        let worker_metadata = initial.metadata_bytes.clone();
        let worker = thread::Builder::new()
            .name("foreign-owner-heartbeat".to_owned())
            .spawn(move || {
                let mut heartbeat = 0_u64;
                while !worker_stop.load(Ordering::Acquire) {
                    heartbeat = heartbeat.saturating_add(1);
                    let state = AtomicOwnerState {
                        heartbeat,
                        lease_owner: worker_owner.clone(),
                        metadata_bytes: worker_metadata.clone(),
                    };
                    let _ = atomic_write_state(&worker_state_path, &state);
                    thread::sleep(HEARTBEAT_INTERVAL);
                }
            })
            .map_err(ForeignOwnerError::Io)?;
        let foreign = Self {
            workspace,
            lease_path,
            state_path,
            stop,
            worker: Some(worker),
            lease: Some(lease),
            #[cfg(windows)]
            lease_lock,
        };
        let snapshot = foreign.snapshot()?;
        Ok((foreign, snapshot))
    }

    /// Reads one atomically replaced owner-state snapshot and the retained lease.
    pub fn snapshot(&self) -> Result<ForeignSnapshot, ForeignOwnerError> {
        let state: AtomicOwnerState = serde_json::from_slice(&fs::read(&self.state_path)?)
            .map_err(|_| ForeignOwnerError::Malformed("owner state"))?;
        let lease_owner = String::from_utf8(fs::read(&self.lease_path)?)
            .map_err(|_| ForeignOwnerError::Malformed("lease owner"))?;
        let metadata_revision = parse_metadata_revision(&state.metadata_bytes)?;
        if lease_owner != state.lease_owner {
            return Err(ForeignOwnerError::Malformed("lease/state owner mismatch"));
        }
        Ok(ForeignSnapshot {
            heartbeat: state.heartbeat,
            lease_owner,
            metadata_bytes: state.metadata_bytes,
            metadata_revision,
        })
    }

    /// Returns the workspace path used by this foreign owner.
    #[must_use]
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }
}

impl Drop for ForeignOwner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        #[cfg(windows)]
        if let (Some(lease), Some(mut lock)) = (self.lease.as_ref(), self.lease_lock.take()) {
            unlock_lease(lease, &mut lock);
        }
        self.lease.take();
        let _ = fs::remove_file(&self.lease_path);
        let _ = fs::remove_file(&self.state_path);
    }
}

fn atomic_write_state(path: &Path, state: &AtomicOwnerState) -> Result<(), ForeignOwnerError> {
    let sequence = OWNER_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("tmp-{sequence}"));
    let bytes =
        serde_json::to_vec(state).map_err(|_| ForeignOwnerError::Malformed("owner state"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    atomic_replace(&temporary, path)?;
    Ok(())
}

#[cfg(not(windows))]
fn atomic_replace(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
fn atomic_replace(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: both vectors are NUL-terminated and remain alive for the call.
    let ok = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn lock_lease(file: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: the descriptor belongs to the retained lease file.
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn lock_lease(file: &File) -> io::Result<Box<windows_sys::Win32::System::IO::OVERLAPPED>> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    };
    let mut overlapped = Box::<windows_sys::Win32::System::IO::OVERLAPPED>::default();
    // SAFETY: the handle and overlapped storage remain valid for the lease lifetime.
    let result = unsafe {
        LockFileEx(
            file.as_raw_handle() as _,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            u32::MAX,
            u32::MAX,
            &mut *overlapped,
        )
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(overlapped)
    }
}

#[cfg(windows)]
fn unlock_lease(file: &File, overlapped: &mut windows_sys::Win32::System::IO::OVERLAPPED) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::UnlockFileEx;
    // SAFETY: the handle and lock state were retained from lock_lease.
    unsafe {
        let _ = UnlockFileEx(file.as_raw_handle() as _, 0, u32::MAX, u32::MAX, overlapped);
    }
}

fn parse_metadata_revision(bytes: &[u8]) -> Result<u64, ForeignOwnerError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| ForeignOwnerError::Malformed("metadata bytes"))?;
    let revision = text
        .lines()
        .find_map(|line| line.strip_prefix("revision="))
        .ok_or(ForeignOwnerError::Malformed("metadata revision"))?;
    revision
        .parse()
        .map_err(|_| ForeignOwnerError::Malformed("metadata revision"))
}
