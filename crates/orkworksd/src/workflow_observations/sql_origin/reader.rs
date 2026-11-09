//! Descriptor-relative copied data only; a future checked owner must serialize access.
use super::{decode_line, hex_digest, valid_id, RecordData, Result, StoreError, MAX_LINE_BYTES};
use crate::session_application::sql_writes::{reserve_observation_read, ObservationReadCapacity};
use std::{
    fs::File,
    time::{Duration, Instant},
};
const MAX_ENTRIES: usize = 4097;
const MAX_SEGMENTS: usize = 4096;
pub(super) const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
pub(super) const MAX_LINES: usize = 65536;
const MAX_SELECTED_BYTES: usize = 768 * 1024;

#[cfg(test)]
thread_local! {pub(super) static INVENTORY_SCANS:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}

pub(super) type Stamp = (u64, u64, u64, i64, i64, i64, i64);
pub(super) struct Inventory {
    pub(super) sequence: Option<u64>,
    pub(super) bytes: usize,
    pub(super) segments: usize,
    pub(super) lines: usize,
    _capacity: std::sync::Arc<ObservationReadCapacity>,
}
struct Scanned {
    records: Vec<RecordData>,
    sequence: Option<u64>,
    bytes: usize,
    segments: usize,
    lines: usize,
}
pub(crate) struct ScanRequest {
    deadline: Instant,
    capacity: std::sync::Arc<ObservationReadCapacity>,
}
pub(crate) struct Snapshot {
    records: Vec<RecordData>,
    sequence: Option<u64>,
    _capacity: std::sync::Arc<ObservationReadCapacity>,
}
impl Snapshot {
    pub(crate) fn records(&self) -> &[RecordData] {
        &self.records
    }
    pub(crate) fn sequence(&self) -> Option<u64> {
        self.sequence
    }
}
impl ScanRequest {
    pub(crate) fn hold_capacity(&self) -> std::sync::Arc<ObservationReadCapacity> {
        self.capacity.clone()
    }
    /// Numeric inventory fixtures exercise independent count/byte limits without
    /// conflating them with scheduling speed. Production capture still caps at2s.
    #[cfg(test)]
    pub(super) fn for_numeric_test(mut self) -> Self {
        self.deadline = Instant::now() + Duration::from_secs(120);
        self
    }
    pub(in crate::workflow_observations) fn limit_deadline(mut self, original: Instant) -> Self {
        self.deadline = self.deadline.min(original);
        self
    }
    pub(super) fn deadline(&self) -> Instant {
        self.deadline
    }
    pub(super) fn inspect_at(
        self,
        directory: &File,
        epoch: &str,
        workspace: &str,
        visit: impl FnMut(&str, &RecordData, u64, usize, Stamp) -> Result<()>,
    ) -> Result<Inventory> {
        validate_scope(self.deadline, epoch, workspace)?;
        #[cfg(test)]
        INVENTORY_SCANS.with(|n| n.set(n.get() + 1));
        let scanned = native::scan(directory, self.deadline, epoch, workspace, false, visit)?;
        check(self.deadline)?;
        Ok(Inventory {
            sequence: scanned.sequence,
            bytes: scanned.bytes,
            segments: scanned.segments,
            lines: scanned.lines,
            _capacity: self.capacity,
        })
    }
    /// Capture before any queue/permit wait. This is accounting, not file authority.
    pub(crate) fn capture(original_deadline: Instant) -> Result<Self> {
        let deadline = original_deadline.min(Instant::now() + Duration::from_secs(2));
        check(deadline)?;
        let capacity = std::sync::Arc::new(reserve_observation_read()?);
        check(deadline)?;
        Ok(Self { deadline, capacity })
    }
    /// Expected scope is data from the future original owner, never a current-ID lookup.
    pub(crate) fn read_at(
        self,
        directory: &File,
        epoch: &str,
        workspace_hash: &str,
    ) -> Result<Snapshot> {
        validate_scope(self.deadline, epoch, workspace_hash)?;
        let scanned = native::scan(
            directory,
            self.deadline,
            epoch,
            workspace_hash,
            true,
            |_, _, _, _, _| Ok(()),
        )?;
        check(self.deadline)?;
        Ok(Snapshot {
            records: scanned.records,
            sequence: scanned.sequence,
            _capacity: self.capacity,
        })
    }
}
fn validate_scope(deadline: Instant, epoch: &str, workspace: &str) -> Result<()> {
    check(deadline)?;
    if uuid::Uuid::parse_str(epoch)
        .map(|v| v.to_string() != epoch)
        .unwrap_or(true)
        || !hex_digest(workspace)
    {
        return Err(StoreError::Corrupt);
    }
    Ok(())
}
fn check(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(StoreError::Expired)
    } else {
        Ok(())
    }
}
fn add(count: &mut usize, n: usize, max: usize) -> Result<()> {
    *count = count
        .checked_add(n)
        .filter(|v| *v <= max)
        .ok_or(StoreError::TooLarge)?;
    Ok(())
}
#[derive(Default)]
struct Counts {
    entries: usize,
    segments: usize,
    bytes: usize,
    lines: usize,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod native {
    use super::*;
    use std::{
        collections::{BTreeMap, HashSet},
        ffi::{CStr, CString},
        io::Read,
        os::{
            fd::{AsRawFd, FromRawFd, IntoRawFd},
            unix::fs::MetadataExt,
        },
    };
    #[cfg(test)]
    thread_local! { pub(super) static AFTER_METADATA: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = std::cell::RefCell::new(None); }
    fn open(directory: &File, name: &CStr, is_directory: bool) -> Result<File> {
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK
            | if is_directory { libc::O_DIRECTORY } else { 0 };
        // SAFETY: retained directory descriptor and one NUL-terminated basename.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(StoreError::Io(std::io::Error::last_os_error()));
        }
        // SAFETY: openat returned a fresh owned descriptor; File closes it exactly once.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    fn stamp(file: &File) -> Result<(u64, u64, u64, i64, i64, i64, i64)> {
        Ok(metadata_stamp(&file.metadata()?))
    }
    fn metadata_stamp(m: &std::fs::Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    }
    struct Names(*mut libc::DIR);
    impl Drop for Names {
        fn drop(&mut self) {
            // SAFETY: this object uniquely owns a successful fdopendir result.
            unsafe {
                libc::closedir(self.0);
            }
        }
    }
    impl Names {
        fn new(directory: &File) -> Result<Self> {
            let fd = open(directory, c".", true)?.into_raw_fd();
            // SAFETY: fresh directory descriptor; ownership transfers on success.
            let ptr = unsafe { libc::fdopendir(fd) };
            if ptr.is_null() {
                let error = std::io::Error::last_os_error();
                // SAFETY: failed fdopendir did not consume this fresh descriptor.
                unsafe {
                    libc::close(fd);
                }
                return Err(StoreError::Io(error));
            }
            Ok(Self(ptr))
        }
        fn next(&mut self) -> Result<Option<CString>> {
            loop {
                // SAFETY: errno is thread-local and is reset to distinguish EOF.
                #[cfg(target_os = "linux")]
                unsafe {
                    *libc::__errno_location() = 0;
                }
                #[cfg(target_os = "macos")]
                unsafe {
                    *libc::__error() = 0;
                }
                // SAFETY: live uniquely owned directory stream; copy names before reuse.
                let item = unsafe { libc::readdir(self.0) };
                if item.is_null() {
                    let error = std::io::Error::last_os_error();
                    return if error.raw_os_error() == Some(0) {
                        Ok(None)
                    } else {
                        Err(StoreError::Io(error))
                    };
                }
                // SAFETY: readdir supplies a live NUL-terminated entry name.
                let name = unsafe { CStr::from_ptr((*item).d_name.as_ptr()) };
                if name == c"." || name == c".." {
                    continue;
                }
                return Ok(Some(name.to_owned()));
            }
        }
    }
    struct Input<'a> {
        file: &'a mut File,
        deadline: Instant,
        buffer: [u8; 8192],
        start: usize,
        end: usize,
    }
    impl<'a> Input<'a> {
        fn new(file: &'a mut File, deadline: Instant) -> Self {
            Self {
                file,
                deadline,
                buffer: [0; 8192],
                start: 0,
                end: 0,
            }
        }
        fn fill(&mut self, counts: &mut Counts) -> Result<bool> {
            check(self.deadline)?;
            if self.start < self.end {
                return Ok(true);
            }
            let limit = self
                .buffer
                .len()
                .min(MAX_TOTAL_BYTES.saturating_sub(counts.bytes) + 1);
            self.end = self.file.read(&mut self.buffer[..limit])?;
            self.start = 0;
            add(&mut counts.bytes, self.end, MAX_TOTAL_BYTES)?;
            check(self.deadline)?;
            Ok(self.end > 0)
        }
        fn line(&mut self, counts: &mut Counts, line: &mut Vec<u8>) -> Result<bool> {
            line.clear();
            loop {
                if !self.fill(counts)? {
                    if line.is_empty() {
                        return Ok(false);
                    }
                    return Err(StoreError::Corrupt); // Incomplete NDJSON is never repaired.
                }
                let remaining = &self.buffer[self.start..self.end];
                let end = remaining.iter().position(|b| *b == b'\n');
                let n = end.unwrap_or(remaining.len());
                if line
                    .len()
                    .checked_add(n)
                    .filter(|n| *n <= MAX_LINE_BYTES)
                    .is_none()
                {
                    return Err(StoreError::TooLarge);
                }
                line.extend_from_slice(&remaining[..n]);
                self.start += n + usize::from(end.is_some());
                if end.is_some() {
                    return Ok(true);
                }
            }
        }
        fn sequence(&mut self, counts: &mut Counts) -> Result<u64> {
            let (mut value, mut seen, mut trailing) = (0u64, false, false);
            while self.fill(counts)? {
                for b in &self.buffer[self.start..self.end] {
                    if b.is_ascii_whitespace() {
                        if seen {
                            trailing = true;
                        }
                    } else if b.is_ascii_digit() && !trailing {
                        seen = true;
                        value = value
                            .checked_mul(10)
                            .and_then(|n| n.checked_add(u64::from(*b - b'0')))
                            .ok_or(StoreError::Corrupt)?;
                    } else {
                        return Err(StoreError::Corrupt);
                    }
                }
                self.start = self.end;
            }
            if seen {
                Ok(value)
            } else {
                Err(StoreError::Corrupt)
            }
        }
    }
    pub(super) fn scan(
        directory: &File,
        deadline: Instant,
        epoch: &str,
        workspace: &str,
        retain: bool,
        mut visit: impl FnMut(&str, &RecordData, u64, usize, Stamp) -> Result<()>,
    ) -> Result<Scanned> {
        if !directory.metadata()?.is_dir() {
            return Err(StoreError::Corrupt);
        }
        let original = stamp(directory)?;
        let mut names = Names::new(directory)?;
        let mut counts = Counts::default();
        let (mut counter, mut highest) = (None, 0);
        let mut selected: BTreeMap<u64, (String, usize)> = BTreeMap::new();
        let mut data: BTreeMap<u64, RecordData> = BTreeMap::new();
        let mut selected_bytes = 2usize; // Compact JSON array, including separators.
        let mut line = Vec::new();
        while let Some(name) = names.next()? {
            check(deadline)?;
            add(&mut counts.entries, 1, MAX_ENTRIES)?;
            let text = name.to_str().map_err(|_| StoreError::Corrupt)?;
            let session = if text == "sequence" {
                None
            } else {
                let session = text
                    .strip_suffix(".ndjson")
                    .filter(|id| valid_id(id))
                    .ok_or(StoreError::Corrupt)?;
                add(&mut counts.segments, 1, MAX_SEGMENTS)?;
                Some(session)
            };
            let mut file = open(directory, &name, false)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.nlink() != 1 {
                return Err(StoreError::Corrupt);
            }
            if session.is_some() && metadata.len() > super::super::super::MAX_SEGMENT_BYTES {
                return Err(StoreError::TooLarge);
            }
            #[cfg(test)]
            if session.is_some() {
                AFTER_METADATA.with(|slot| {
                    let callback = slot.borrow_mut().take();
                    if let Some(callback) = callback {
                        callback();
                    }
                });
            }
            // Use the same metadata that passed kind/link/size checks: a second
            // stat here could adopt a change that occurred after validation.
            let before = metadata_stamp(&metadata);
            {
                let mut input = Input::new(&mut file, deadline);
                if let Some(session) = session {
                    let (mut observations, mut tombstones) = (0usize, 0usize);
                    let (mut ids, mut sequences) = (HashSet::new(), HashSet::new());
                    let mut offset = 0u64;
                    while input.line(&mut counts, &mut line)? {
                        let start = offset;
                        offset += line.len() as u64 + 1;
                        if line.iter().all(u8::is_ascii_whitespace) {
                            continue;
                        }
                        add(&mut counts.lines, 1, MAX_LINES)?;
                        let record = decode_line(&line, session)?;
                        if let Some(origin) = record.origin() {
                            if origin.database_epoch() != epoch
                                || origin.workspace_sha256() != workspace
                            {
                                return Err(StoreError::Conflict);
                            }
                        }
                        let (id, sequence) = record.identity();
                        if !ids.insert(id.to_owned()) || !sequences.insert(sequence) {
                            return Err(StoreError::Corrupt);
                        }
                        highest = highest.max(sequence);
                        visit(session, &record, start, line.len(), before)?;
                        if record.observation().is_some() {
                            add(
                                &mut observations,
                                1,
                                super::super::super::MAX_SEGMENT_OBSERVATIONS,
                            )?;
                            if selected.contains_key(&sequence)
                                || selected.values().any(|(retained, _)| retained == id)
                            {
                                return Err(StoreError::Corrupt);
                            }
                            let bytes = if retain { record.encode()?.len() } else { 0 };
                            selected_bytes = selected_bytes
                                .checked_add(bytes + usize::from(!selected.is_empty()))
                                .ok_or(StoreError::TooLarge)?;
                            selected.insert(sequence, (id.to_owned(), bytes));
                            if retain {
                                data.insert(sequence, record);
                            }
                            if selected.len() > super::super::super::MAX_WORKSPACE_OBSERVATIONS {
                                if let Some((removed_sequence, (_, removed_bytes))) =
                                    selected.pop_first()
                                {
                                    selected_bytes -= removed_bytes + 1;
                                    data.remove(&removed_sequence);
                                }
                            }
                            // Inventory returns no payload snapshot. It retains at most
                            // 10,000 UUID/sequence pairs; the result-byte cap stays on reads.
                            if retain && selected_bytes > MAX_SELECTED_BYTES {
                                return Err(StoreError::TooLarge);
                            }
                        } else {
                            add(&mut tombstones, 1, super::super::super::MAX_TOMBSTONES)?;
                        }
                        check(deadline)?;
                    }
                } else {
                    counter = Some(input.sequence(&mut counts)?);
                }
            }
            if before != stamp(&file)? || before != stamp(&open(directory, &name, false)?)? {
                return Err(StoreError::Conflict);
            }
        }
        if stamp(directory)? != original {
            return Err(StoreError::Conflict);
        }
        if counts.entries > 0 && counter.is_none() || counter.is_some_and(|n| n < highest) {
            return Err(StoreError::Corrupt);
        }
        check(deadline)?;
        Ok(Scanned {
            records: data.into_values().collect(),
            sequence: counter,
            bytes: counts.bytes,
            segments: counts.segments,
            lines: counts.lines,
        })
    }
}
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod native {
    use super::*;
    pub(super) fn scan(
        _: &File,
        _: Instant,
        _: &str,
        _: &str,
        _: bool,
        _: impl FnMut(&str, &RecordData, u64, usize, Stamp) -> Result<()>,
    ) -> Result<Scanned> {
        Err(StoreError::Unsupported)
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
