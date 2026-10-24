//! Source-ordered transport core only; no production launch creates this queue.
use super::{CodexHookReportRelay, MailboxDirectory};
use crate::runtime::codex_completion::{Event, Hook};
use serde::{Deserialize, Serialize};
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::time::{Duration, Instant};

const MAX_BYTES: usize = 1024 * 1024;
const MAX_RECORDS: usize = 256;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QueueState {
    version: u64,
    stream_id: String,
    generation: u64,
    hook_fingerprint: String,
    next_sequence: u64,
    acknowledged_sequence: u64,
    records: Vec<Record>,
}

struct Snapshot {
    state: QueueState,
    bytes: Vec<u8>,
    identity: (u64, u64),
}

fn required_option<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::deserialize(d)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    version: u64,
    sequence: u64,
    observed_micros: i64,
    hook_fingerprint: String,
    root_id: String,
    #[serde(deserialize_with = "required_option")]
    turn_id: Option<String>,
    event: Kind,
    #[serde(deserialize_with = "required_option")]
    tool_name: Option<String>,
    #[serde(deserialize_with = "required_option")]
    tool_use_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
enum Kind {
    UserPromptSubmit,
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    Stop,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Poll {
    Busy,
    Consumed(usize),
}

pub(crate) struct CompletionQueue {
    mailbox: MailboxDirectory,
    lock: File,
    stream_id: String,
    generation: u64,
    fingerprint: String,
    accepted: u64,
    failed: bool,
    directory: tempfile::TempDir,
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid completion stream")
}

fn private_file(file: &File) -> io::Result<()> {
    let m = file.metadata()?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o077 != 0
        || m.nlink() != 1
    {
        return Err(invalid());
    }
    Ok(())
}

impl CompletionQueue {
    pub(crate) fn create(
        relay: &CodexHookReportRelay,
        generation: u64,
        fingerprint: &str,
    ) -> io::Result<Self> {
        if fingerprint.len() != 64
            || !fingerprint
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid());
        }
        let directory = tempfile::Builder::new()
            .prefix("completion-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir_in(relay.mailbox_path())?;
        let mailbox = MailboxDirectory::open(directory.path())?;
        let lock = Self::create_file(&mailbox, "queue.lock")?;
        lock.sync_all()?;
        mailbox.handle.sync_all()?;
        let queue = Self {
            mailbox,
            lock,
            stream_id: uuid::Uuid::new_v4().simple().to_string(),
            generation,
            fingerprint: fingerprint.into(),
            accepted: 0,
            failed: false,
            directory,
        };
        queue.replace(&QueueState {
            version: 1,
            stream_id: queue.stream_id.clone(),
            generation,
            hook_fingerprint: fingerprint.into(),
            next_sequence: 1,
            acknowledged_sequence: 0,
            records: vec![],
        })?;
        Ok(queue)
    }

    pub(crate) fn path(&self) -> &Path {
        self.directory.path()
    }

    fn create_file(mailbox: &MailboxDirectory, name: &str) -> io::Result<File> {
        let name = CString::new(name).map_err(|_| invalid())?;
        // SAFETY: name is NUL-terminated; the retained directory descriptor is
        // live. A successful openat returns a new descriptor owned below.
        let fd = unsafe {
            libc::openat(
                mailbox.handle.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_EXCL
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the new descriptor has no other owner.
        let file = unsafe { File::from_raw_fd(fd) };
        private_file(&file)?;
        Ok(file)
    }

    fn replace(&self, state: &QueueState) -> io::Result<()> {
        let bytes = serde_json::to_vec(state).map_err(|_| invalid())?;
        if bytes.len() > MAX_BYTES {
            return Err(invalid());
        }
        let name = format!("pending-{}", uuid::Uuid::new_v4().simple());
        let mut file = Self::create_file(&self.mailbox, &name)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        let from = CString::new(name).map_err(|_| invalid())?;
        // SAFETY: both names are valid NUL-terminated strings relative to the
        // same retained directory; rename publishes a complete synced file.
        if unsafe {
            libc::renameat(
                self.mailbox.handle.as_raw_fd(),
                from.as_ptr(),
                self.mailbox.handle.as_raw_fd(),
                c"queue.json".as_ptr(),
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        self.mailbox.handle.sync_all()
    }

    fn identity(&self) -> io::Result<()> {
        let named = std::fs::symlink_metadata(self.path())?;
        let retained = self.mailbox.handle.metadata()?;
        if !named.is_dir()
            || named.uid() != unsafe { libc::geteuid() }
            || named.mode() & 0o077 != 0
            || (named.dev(), named.ino()) != (retained.dev(), retained.ino())
        {
            return Err(invalid());
        }
        private_file(&self.lock)?;
        let current = self.mailbox.open_report(OsStr::new("queue.lock"))?;
        private_file(&current)?;
        let a = current.metadata()?;
        let b = self.lock.metadata()?;
        if (a.dev(), a.ino()) != (b.dev(), b.ino()) {
            return Err(invalid());
        }
        Ok(())
    }

    fn unfinished(&self) -> io::Result<bool> {
        let names = self.mailbox.entries(513)?;
        if names.len() > 512 {
            return Err(invalid());
        }
        let mut busy = false;
        let mut inflights = 0;
        for name in names {
            let text = name.to_str().ok_or_else(invalid)?;
            let file = self.mailbox.open_report(&name)?;
            private_file(&file)?;
            match text {
                "poison" => return Err(invalid()),
                "queue.json" | "queue.lock" => {}
                _ if text
                    .strip_prefix("inflight-")
                    .is_some_and(|id| hex_id(id, 32)) =>
                {
                    busy = true;
                    inflights += 1;
                }
                // A retained temporary replacement is an incomplete publication.
                _ if text
                    .strip_prefix("pending-")
                    .is_some_and(|id| hex_id(id, 32)) =>
                {
                    return Err(invalid())
                }
                _ => return Err(invalid()),
            }
        }
        if inflights > 256 {
            return Err(invalid());
        }
        Ok(busy)
    }

    fn read(&self, root: &str) -> io::Result<Snapshot> {
        let file = self.mailbox.open_report(OsStr::new("queue.json"))?;
        private_file(&file)?;
        let metadata = file.metadata()?;
        let identity = (metadata.dev(), metadata.ino());
        let mut bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BYTES {
            return Err(invalid());
        }
        let state: QueueState = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        let count = state
            .next_sequence
            .checked_sub(state.acknowledged_sequence)
            .and_then(|n| n.checked_sub(1));
        if state.next_sequence == u64::MAX
            || state.version != 1
            || state.stream_id != self.stream_id
            || state.generation != self.generation
            || state.hook_fingerprint != self.fingerprint
            || state.acknowledged_sequence != self.accepted
            || count != Some(state.records.len() as u64)
            || state.records.len() > MAX_RECORDS
            || !id(root)
        {
            return Err(invalid());
        }
        for (offset, record) in state.records.iter().enumerate() {
            if record.version != 1
                || record.sequence
                    != self
                        .accepted
                        .checked_add(offset as u64 + 1)
                        .ok_or_else(invalid)?
                || record.observed_micros <= 0
                || record.hook_fingerprint != self.fingerprint
                || record.root_id != root
                || !id(&record.root_id)
                || record.turn_id.as_deref().is_some_and(|s| !id(s))
                || record.tool_use_id.as_deref().is_some_and(|s| !id(s))
                || record.tool_name.as_deref().is_some_and(|s| {
                    s.is_empty() || s.len() > 128 || !s.bytes().all(|b| (33..=126).contains(&b))
                })
                || serde_json::to_vec(record).map_err(|_| invalid())?.len() > 4096
            {
                return Err(invalid());
            }
        }
        Ok(Snapshot {
            state,
            bytes,
            identity,
        })
    }

    fn verify_snapshot(&self, snapshot: &Snapshot) -> io::Result<()> {
        let file = self.mailbox.open_report(OsStr::new("queue.json"))?;
        private_file(&file)?;
        let metadata = file.metadata()?;
        if (metadata.dev(), metadata.ino()) != snapshot.identity {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        if bytes != snapshot.bytes {
            return Err(invalid());
        }
        Ok(())
    }

    fn poison(&mut self) {
        self.failed = true;
        if let Ok(file) = Self::create_file(&self.mailbox, "poison") {
            let _ = file.sync_all();
            let _ = self.mailbox.handle.sync_all();
        }
    }

    /// Caller holds workspace -> sessions; this takes queue -> metadata store
    /// through the synchronous callback. No stream/fence reconstruction exists.
    pub(crate) fn consume(
        &mut self,
        root: &str,
        mut apply: impl FnMut(Event<'_>) -> io::Result<()>,
    ) -> io::Result<Poll> {
        if self.failed {
            return Err(invalid());
        }
        let result = (|| {
            self.identity()?;
            let lock = self.lock.try_clone()?;
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match fs2::FileExt::try_lock_exclusive(&lock) {
                    Ok(()) => break,
                    Err(e)
                        if e.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => return Err(e),
                }
            }
            let guard = Locked(lock);
            self.identity()?;
            if self.unfinished()? {
                return Ok(Poll::Busy);
            }
            let mut snapshot = self.read(root)?;
            let count = snapshot.state.records.len();
            if count == 0 {
                return Ok(Poll::Consumed(0));
            }
            // Latch before callbacks so even a caught panic cannot replay an
            // effect or reconstruct an accepted fence from unacked records.
            self.failed = true;
            for record in &snapshot.state.records {
                self.verify_snapshot(&snapshot)?;
                if self.unfinished()? {
                    return Err(invalid());
                }
                self.accepted = record.sequence;
                apply(record.as_event(self.generation))?;
            }
            self.identity()?;
            if self.unfinished()? {
                return Err(invalid());
            }
            self.verify_snapshot(&snapshot)?;
            snapshot.state.acknowledged_sequence = self.accepted;
            snapshot.state.records.clear();
            self.replace(&snapshot.state)?;
            self.identity()?;
            fs2::FileExt::unlock(&guard.0)?;
            self.failed = false;
            Ok(Poll::Consumed(count))
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }
}

fn hex_id(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
struct Locked(File);
impl Drop for Locked {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}
impl Record {
    fn as_event(&self, generation: u64) -> Event<'_> {
        Event {
            generation,
            root: &self.root_id,
            turn: self.turn_id.as_deref().unwrap_or(""),
            sequence: self.sequence,
            observed_micros: self.observed_micros,
            hook: match self.event {
                Kind::UserPromptSubmit => Hook::Submitted,
                Kind::PreToolUse => Hook::Pre,
                Kind::PermissionRequest => Hook::Permission,
                Kind::PostToolUse => Hook::Post,
                Kind::Stop => Hook::Stop,
            },
            tool: self.tool_name.as_deref(),
            invocation: self.tool_use_id.as_deref(),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
