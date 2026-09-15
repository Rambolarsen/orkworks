//! Independent foreign workspace owner used by the cleanup isolation fixture.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

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
    /// Metadata revision encoded in the metadata file.
    pub metadata_revision: u64,
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

/// Independent owner holding a workspace lease and updating its heartbeat.
#[derive(Debug)]
pub struct ForeignOwner {
    workspace: PathBuf,
    lease_path: PathBuf,
    heartbeat_path: PathBuf,
    metadata_path: PathBuf,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    /// Retains the open lease descriptor for the lifetime of the owner.
    lease: Option<File>,
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
        let heartbeat_path = workspace.join(".orkworks-foreign-heartbeat");
        let metadata_path = workspace.join(".orkworks-foreign-metadata");
        let lease = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lease_path)?;
        let mut lease = lease;
        lease.write_all(owner.as_bytes())?;
        lease.sync_all()?;
        let metadata_bytes = format!("owner={owner}\nrevision={metadata_revision}\n").into_bytes();
        fs::write(&metadata_path, &metadata_bytes)?;
        fs::write(&heartbeat_path, b"0")?;

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_heartbeat_path = heartbeat_path.clone();
        let worker = thread::Builder::new()
            .name("foreign-owner-heartbeat".to_owned())
            .spawn(move || {
                let mut heartbeat = 0_u64;
                while !worker_stop.load(Ordering::Acquire) {
                    heartbeat = heartbeat.saturating_add(1);
                    let _ = fs::write(&worker_heartbeat_path, heartbeat.to_string());
                    thread::sleep(HEARTBEAT_INTERVAL);
                }
            })
            .map_err(ForeignOwnerError::Io)?;
        let foreign = Self {
            workspace,
            lease_path,
            heartbeat_path,
            metadata_path,
            stop,
            worker: Some(worker),
            lease: Some(lease),
        };
        let snapshot = foreign.snapshot()?;
        Ok((foreign, snapshot))
    }

    /// Reads the foreign owner state without asking its worker thread to attest anything.
    pub fn snapshot(&self) -> Result<ForeignSnapshot, ForeignOwnerError> {
        let lease_owner = String::from_utf8(fs::read(&self.lease_path)?)
            .map_err(|_| ForeignOwnerError::Malformed("lease owner"))?;
        let heartbeat = String::from_utf8(fs::read(&self.heartbeat_path)?)
            .map_err(|_| ForeignOwnerError::Malformed("heartbeat"))?
            .trim()
            .parse()
            .map_err(|_| ForeignOwnerError::Malformed("heartbeat"))?;
        let metadata_bytes = fs::read(&self.metadata_path)?;
        let metadata_revision = parse_metadata_revision(&metadata_bytes)?;
        Ok(ForeignSnapshot {
            heartbeat,
            lease_owner,
            metadata_bytes,
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
        self.lease.take();
        let _ = fs::remove_file(&self.lease_path);
        let _ = fs::remove_file(&self.heartbeat_path);
        let _ = fs::remove_file(&self.metadata_path);
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
