//! Read-only qualification boundary; no graph recovery or deletion authority.
#![cfg_attr(not(test), allow(dead_code))]
use super::*;
use crate::workspace_runtime::WorkspaceLease;
use std::sync::Arc;

pub(crate) struct CleanupIntent {
    store: RecommendationStore,
}
#[derive(Clone)]
pub(crate) struct BoundCleanup(Arc<Binding>);
struct Binding {
    store: RecommendationStore,
    _lease: Arc<WorkspaceLease>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    root: fs::File,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    records: Option<fs::File>,
}
#[must_use]
pub(crate) struct CleanupInventory {
    records: Vec<Recommendation>,
    _operation: operation::Operation,
    _binding: BoundCleanup,
}
impl CleanupInventory {
    // Copies remain snapshots, never a deletion or source-freshness capability.
    pub(crate) fn records(&self) -> &[Recommendation] {
        &self.records
    }
}
impl RecommendationStore {
    // Memory only: capture while the caller owns its workspace identity.
    pub(crate) fn capture_cleanup(&self) -> CleanupIntent {
        CleanupIntent {
            store: self.clone(),
        }
    }
}
impl CleanupIntent {
    pub(crate) fn bind(self, lease: Arc<WorkspaceLease>) -> Result<BoundCleanup, StoreError> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            unix::bind(self.store, lease).map_err(StoreError::Io)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = (self, lease);
            Err(StoreError::Io(io::Error::new(
                io::ErrorKind::Unsupported,
                "recommendation cleanup preflight is not qualified on this platform",
            )))
        }
    }
}
impl BoundCleanup {
    // Blocking-only. Caller releases application guards before inspection.
    pub(crate) fn inspect(&self) -> Result<CleanupInventory, StoreError> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let operation = self.0.store.try_operation()?;
            let records = unix::inspect(&self.0)?;
            Ok(CleanupInventory {
                records,
                _binding: self.clone(),
                _operation: operation,
            })
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        Err(StoreError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "recommendation cleanup preflight is not qualified on this platform",
        )))
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod unix {
    use super::*;
    use std::{
        ffi::{CStr, CString},
        io::Read,
        os::{
            fd::{AsRawFd, FromRawFd, IntoRawFd},
            unix::fs::{MetadataExt, OpenOptionsExt},
        },
    };
    const ENTRIES: usize = 1024;
    const RECORD_BYTES: usize = 1024 * 1024;
    const TOTAL_BYTES: usize = 32 * 1024 * 1024;
    fn invalid(message: &str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }
    fn open(parent: &fs::File, name: &CStr, directory: bool) -> io::Result<fs::File> {
        // SAFETY: live parent FD and a fixed or enumerated single C basename.
        // No-follow rejects aliases; nonblocking avoids waiting on a FIFO.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC
                    | if directory { libc::O_DIRECTORY } else { 0 },
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful openat returned one new owned descriptor.
        Ok(unsafe { fs::File::from_raw_fd(fd) })
    }
    fn identity(file: &fs::File) -> io::Result<(u64, u64)> {
        let m = file.metadata()?;
        Ok((m.dev(), m.ino()))
    }
    fn entry(parent: &fs::File, name: &CStr) -> io::Result<Option<(u64, u64)>> {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: live FD, valid basename and writable stat output; never follows aliases.
        if unsafe {
            libc::fstatat(
                parent.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } < 0
        {
            let e = io::Error::last_os_error();
            return if e.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(e)
            };
        }
        // SAFETY: fstatat succeeded and initialized stat.
        let stat = unsafe { stat.assume_init() };
        Ok(Some((stat.st_dev as u64, stat.st_ino as u64)))
    }
    pub(super) fn bind(
        store: RecommendationStore,
        lease: Arc<WorkspaceLease>,
    ) -> io::Result<BoundCleanup> {
        let root = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(
                store
                    .dir
                    .parent()
                    .ok_or_else(|| invalid("recommendation root unavailable"))?,
            )?;
        lease.validate_directory(&root)?;
        let records = match open(&root, c"recommendations", true) {
            Ok(file) => Some(file),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        Ok(BoundCleanup(Arc::new(Binding {
            store,
            _lease: lease,
            root,
            records,
        })))
    }
    fn current(binding: &Binding) -> io::Result<()> {
        binding._lease.validate_directory(&binding.root)?;
        let expected = binding.records.as_ref().map(identity).transpose()?;
        if entry(&binding.root, c"recommendations")? != expected {
            return Err(invalid("recommendation directory changed after binding"));
        }
        Ok(())
    }
    pub(super) fn inspect(binding: &Binding) -> Result<Vec<Recommendation>, StoreError> {
        current(binding).map_err(StoreError::Io)?;
        let Some(directory) = &binding.records else {
            return Ok(Vec::new());
        };
        let names = names(directory).map_err(StoreError::Io)?;
        let mut records = Vec::new();
        let mut total = 0usize;
        for name in names {
            if !name.to_bytes().ends_with(b".json") {
                return Err(StoreError::Recovery(
                    "recommendation preflight requires recovered canonical record entries".into(),
                ));
            }
            let file = open(directory, &name, false).map_err(StoreError::Io)?;
            let metadata = file.metadata().map_err(StoreError::Io)?;
            if !metadata.is_file() || metadata.nlink() != 1 {
                return Err(StoreError::Io(invalid(
                    "recommendation is not an unaliased regular file",
                )));
            }
            if metadata.len() > RECORD_BYTES as u64 {
                return Err(StoreError::Io(invalid(
                    "recommendation exceeds record byte bound",
                )));
            }
            let mut bytes = Vec::new();
            (&file)
                .take((RECORD_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(StoreError::Io)?;
            if bytes.len() > RECORD_BYTES {
                return Err(StoreError::Io(invalid(
                    "recommendation exceeds record byte bound",
                )));
            }
            total = total
                .checked_add(bytes.len())
                .filter(|n| *n <= TOTAL_BYTES)
                .ok_or_else(|| {
                    StoreError::Io(invalid("recommendations exceed aggregate byte bound"))
                })?;
            if entry(directory, &name).map_err(StoreError::Io)?
                != Some((metadata.dev(), metadata.ino()))
            {
                return Err(StoreError::Io(invalid(
                    "recommendation entry changed during preflight",
                )));
            }
            let record: Recommendation =
                crate::harness::definition::parse_strict_json(&bytes, RECORD_BYTES).map_err(
                    |_| StoreError::Io(invalid("invalid or ambiguous recommendation JSON")),
                )?;
            validate_recommendation(&record)?;
            if name.to_bytes() != recommendation_filename(&record.id).as_bytes() {
                return Err(StoreError::Io(invalid(
                    "recommendation filename does not match its identity",
                )));
            }
            records.push(record);
        }
        validate_graph_records(&records)?;
        current(binding).map_err(StoreError::Io)?;
        Ok(records)
    }
    fn names(directory: &fs::File) -> io::Result<Vec<CString>> {
        // A fresh open description avoids sharing directory offsets across inspections.
        let file = open(directory, c".", true)?;
        let fd = file.into_raw_fd();
        // SAFETY: newly owned directory FD; fdopendir takes ownership on success.
        let raw = unsafe { libc::fdopendir(fd) };
        if raw.is_null() {
            let e = io::Error::last_os_error();
            // SAFETY: fdopendir failed and did not take ownership of fd.
            unsafe { libc::close(fd) };
            return Err(e);
        }
        struct Stream(*mut libc::DIR);
        impl Drop for Stream {
            fn drop(&mut self) {
                // SAFETY: this stream uniquely owns a successful fdopendir result.
                unsafe { libc::closedir(self.0) };
            }
        }
        let stream = Stream(raw);
        let mut names = Vec::new();
        loop {
            // SAFETY: errno is thread-local; reset to distinguish EOF from error.
            #[cfg(target_os = "macos")]
            unsafe {
                *libc::__error() = 0
            };
            #[cfg(target_os = "linux")]
            unsafe {
                *libc::__errno_location() = 0
            };
            // SAFETY: stream remains live, and each name is copied before the next call.
            let raw = unsafe { libc::readdir(stream.0) };
            if raw.is_null() {
                let e = io::Error::last_os_error();
                if e.raw_os_error() != Some(0) {
                    return Err(e);
                }
                break;
            }
            // SAFETY: successful readdir provides a NUL-terminated name in this dirent.
            let name = unsafe { CStr::from_ptr((*raw).d_name.as_ptr()) };
            if name == c"." || name == c".." {
                continue;
            }
            if names.len() == ENTRIES {
                return Err(invalid("recommendations exceed entry bound"));
            }
            names.push(name.to_owned());
        }
        Ok(names)
    }
}
