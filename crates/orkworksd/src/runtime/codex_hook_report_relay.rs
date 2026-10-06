use super::codex_approval_application::{self, HookReportOutcome, NativeHookReport};
use crate::http::session_handlers::{
    report_harness_session_from_local_relay, HarnessSessionReportRequest,
};
use crate::runtime::terminal_runtime::verify_workflow_report_token;
use crate::AppState;
use axum::http::StatusCode;
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
#[cfg(not(unix))]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const MAX_REPORT_BYTES: u64 = 4 * 1024;
const MAX_REPORTS_PER_PASS: usize = 32;
const MAX_MAILBOX_ENTRIES_PER_PASS: usize = 1024;
const STALE_PENDING_REPORT_AGE: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityEnvelope {
    report: HarnessSessionReportRequest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalEnvelope {
    approval: NativeHookReport,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CodexHookReportEnvelope {
    Identity(IdentityEnvelope),
    Approval(ApprovalEnvelope),
}

struct PendingReceipt {
    bytes: Vec<u8>,
    first_receipt: Instant,
}

#[derive(Default)]
struct ReceiptBook {
    pending: HashMap<OsString, PendingReceipt>,
    retry_order: VecDeque<OsString>,
    // Once exhausted, conservatively disable assistance for this relay's
    // remaining lifetime. This floor never advances on drain or reconnect.
    overflow_floor: Option<Instant>,
}
const MAX_PENDING_RECEIPTS: usize = 64;
const MAX_RETRIES_PER_PASS: usize = MAX_REPORTS_PER_PASS / 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RelayOutcome {
    pub(crate) reports_consumed: usize,
    pub(crate) reports_accepted: usize,
}

/// A private temporary directory whose path is passed only to one Codex
/// runtime. Its handle and lifetime are tied to the PTY driver's lifetime.
pub(crate) struct CodexHookReportRelay {
    mailbox: Option<Arc<MailboxDirectory>>,
    directory: Option<tempfile::TempDir>,
    receipts: Mutex<ReceiptBook>,
}

impl CodexHookReportRelay {
    pub(crate) fn new() -> io::Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("orkworks-codex-hook-reports-")
            .tempdir()?;
        let mailbox = MailboxDirectory::open(directory.path())?;
        Ok(Self {
            mailbox: Some(Arc::new(mailbox)),
            directory: Some(directory),
            receipts: Mutex::new(ReceiptBook::default()),
        })
    }

    pub(crate) fn mailbox_path(&self) -> &Path {
        self.directory
            .as_ref()
            .expect("Codex mailbox directory remains live")
            .path()
    }

    pub(crate) async fn consume_ready(
        &self,
        state: Arc<AppState>,
        session_id: &str,
        report_token: &str,
        runtime_generation: u64,
    ) -> RelayOutcome {
        if !verify_workflow_report_token(session_id, report_token) {
            return RelayOutcome::default();
        }
        let owns_live_codex_runtime =
            state
                .sessions
                .lock()
                .unwrap()
                .get(session_id)
                .is_some_and(|handle| {
                    handle.runtime.run_generation() == runtime_generation
                        && handle.info.lifecycle == "alive"
                        && handle.info.lifecycle_phase == "active"
                        && (handle.info.harness.as_deref() == Some("codex")
                            || handle.info.harness_id.as_deref() == Some("codex"))
                });
        if !owns_live_codex_runtime {
            return RelayOutcome::default();
        }

        let mut outcome = RelayOutcome::default();
        let mailbox = Arc::clone(
            self.mailbox
                .as_ref()
                .expect("Codex mailbox handle remains live"),
        );

        // Cached retries have their own bounded quota, leaving at least half
        // the pass for unvisited files. Unused retry slots remain available
        // for identity binding and later hooks.
        let (mut reports, excluded) = {
            let mut book = self.receipts.lock().unwrap();
            let mut retries = Vec::new();
            for _ in 0..book.retry_order.len().min(MAX_RETRIES_PER_PASS) {
                if let Some(name) = book.retry_order.pop_front() {
                    if book.pending.contains_key(&name) {
                        retries.push(name.clone());
                        book.retry_order.push_back(name);
                    }
                }
            }
            (
                retries,
                book.pending.keys().cloned().collect::<HashSet<_>>(),
            )
        };
        let unvisited_budget = MAX_REPORTS_PER_PASS - reports.len();
        let scan_mailbox = Arc::clone(&mailbox);
        let new_reports = tokio::task::spawn_blocking(move || {
            select_unvisited_reports(&scan_mailbox, unvisited_budget, &excluded)
        })
        .await;
        if let Ok(Ok(new_reports)) = new_reports {
            reports.extend(new_reports);
        }
        for name in reports {
            let read_mailbox = Arc::clone(&mailbox);
            let read_name = name.clone();
            let receipt = Instant::now();
            let report =
                tokio::task::spawn_blocking(move || read_report(&read_mailbox, &read_name))
                    .await
                    .ok()
                    .flatten();
            let mut keep = false;
            if let Some((report, bytes)) = report {
                let (first_receipt, changed, full, overflowed) = {
                    let mut book = self.receipts.lock().unwrap();
                    let full = !book.pending.contains_key(&name)
                        && book.pending.len() >= MAX_PENDING_RECEIPTS;
                    if full && matches!(report, CodexHookReportEnvelope::Approval(_)) {
                        book.overflow_floor.get_or_insert(receipt);
                    }
                    let floor = book.overflow_floor.unwrap_or(receipt);
                    let (first, changed) = book.pending.get(&name).map_or((floor, false), |old| {
                        (old.first_receipt, old.bytes != bytes)
                    });
                    (first, changed, full, book.overflow_floor.is_some())
                };
                if changed || overflowed {
                    codex_approval_application::revoke_correlation(
                        &state,
                        session_id,
                        report_token,
                        runtime_generation,
                    );
                }
                if !changed {
                    match report {
                        CodexHookReportEnvelope::Identity(envelope) => {
                            let status = report_harness_session_from_local_relay(
                                state.clone(),
                                session_id.to_string(),
                                envelope.report,
                                report_token,
                            )
                            .await;
                            if status == StatusCode::OK {
                                outcome.reports_accepted += 1;
                            }
                        }
                        CodexHookReportEnvelope::Approval(envelope) => {
                            if full {
                                // Leave the original file unmodified. The fixed
                                // overflow floor prevents renewed grace later.
                                keep = true;
                            } else {
                                let callback_state = state.clone();
                                let callback_id = session_id.to_string();
                                let callback_token = report_token.to_string();
                                // Application validation/persistence does blocking
                                // I/O too; keep it off the async PTY driver.
                                let callback = tokio::task::spawn_blocking(move || {
                                    codex_approval_application::report_hook(
                                        &callback_state,
                                        &callback_id,
                                        &callback_token,
                                        runtime_generation,
                                        first_receipt,
                                        &envelope.approval,
                                    )
                                })
                                .await;
                                match callback.unwrap_or(HookReportOutcome::Retry) {
                                    HookReportOutcome::Accepted => outcome.reports_accepted += 1,
                                    HookReportOutcome::Rejected => {}
                                    HookReportOutcome::Retry => {
                                        keep = true;
                                        let mut book = self.receipts.lock().unwrap();
                                        if !book.pending.contains_key(&name) {
                                            book.pending.insert(
                                                name.clone(),
                                                PendingReceipt {
                                                    bytes,
                                                    first_receipt,
                                                },
                                            );
                                            book.retry_order.push_back(name.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Unidentifiable/corrupted bytes revoke only; they cannot
                // fabricate a permission candidate or an attention transition.
                codex_approval_application::revoke_correlation(
                    &state,
                    session_id,
                    report_token,
                    runtime_generation,
                );
            }
            if !keep {
                outcome.reports_consumed += 1;
                {
                    let mut book = self.receipts.lock().unwrap();
                    book.pending.remove(&name);
                    book.retry_order.retain(|pending| pending != &name);
                }
                let remove_mailbox = Arc::clone(&mailbox);
                let _ = tokio::task::spawn_blocking(move || remove_mailbox.remove(&name)).await;
            }
        }

        outcome
    }
}

impl Drop for CodexHookReportRelay {
    fn drop(&mut self) {
        let Some(directory) = self.directory.take() else {
            return;
        };
        drop(self.mailbox.take());

        // On Unix the mailbox path is writable by the Codex child for its
        // whole lifetime, so an `is_same_directory` check immediately before
        // removal is still a TOCTOU: the child can rename the original away
        // and create a fresh directory at that pathname between the check
        // and TempDir's recursive path-based delete, causing this drop to
        // destroy unrelated data. There is no handle-safe way to remove a
        // directory entry by path, so leak it instead of risking that.
        // Windows is not affected: its directory handle keeps an exclusive
        // share mode for the relay's whole lifetime, so nothing can occupy
        // or replace this pathname while it's held.
        if cfg!(unix) {
            let _ = directory.keep();
        } else {
            drop(directory);
        }
    }
}

fn is_report_filename(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".json") else {
        return false;
    };
    uuid::Uuid::parse_str(stem).is_ok_and(|id| id.simple().to_string() == stem)
}

#[cfg(test)]
fn select_reports(
    mailbox: &MailboxDirectory,
    report_limit: usize,
) -> io::Result<Vec<std::ffi::OsString>> {
    select_unvisited_reports(mailbox, report_limit, &HashSet::new())
}

fn select_unvisited_reports(
    mailbox: &MailboxDirectory,
    report_limit: usize,
    excluded: &HashSet<OsString>,
) -> io::Result<Vec<OsString>> {
    let mut reports = Vec::with_capacity(report_limit);
    let mut inspected = 0;
    while reports.len() < report_limit && inspected < MAX_MAILBOX_ENTRIES_PER_PASS {
        // Never consume more directory entries than the remaining selected
        // quota: unconsumed overflow files must not repeatedly hide later
        // identity/Permission entries in the same physical scan page.
        let page_limit =
            (report_limit - reports.len()).min(MAX_MAILBOX_ENTRIES_PER_PASS - inspected);
        let entries = mailbox.entries(page_limit)?;
        if entries.is_empty() {
            break;
        }
        inspected += entries.len();
        let at_end = entries.len() < page_limit;
        for name in entries {
            if is_report_filename(&name) && !excluded.contains(&name) {
                reports.push(name);
            } else {
                discard_stale_pending_report(mailbox, &name);
            }
        }
        if at_end {
            break;
        }
    }
    reports.sort();
    Ok(reports)
}

fn discard_stale_pending_report(mailbox: &MailboxDirectory, name: &OsStr) {
    if !name
        .to_str()
        .is_some_and(|name| name.starts_with(".pending-"))
    {
        return;
    }
    let Ok(file) = mailbox.open_report(name) else {
        return;
    };
    let stale = file
        .metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| std::time::SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age >= STALE_PENDING_REPORT_AGE);
    drop(file);
    if stale {
        let _ = mailbox.remove(name);
    }
}

fn read_report(
    mailbox: &MailboxDirectory,
    name: &OsStr,
) -> Option<(CodexHookReportEnvelope, Vec<u8>)> {
    let mut file = mailbox.open_report(name).ok()?;
    let mut bytes = Vec::with_capacity(MAX_REPORT_BYTES as usize);
    file.by_ref()
        .take(MAX_REPORT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_REPORT_BYTES {
        return None;
    }
    let envelope = serde_json::from_slice::<CodexHookReportEnvelope>(&bytes).ok()?;
    if let CodexHookReportEnvelope::Identity(envelope) = &envelope {
        if envelope.report.source != "codex_hook"
            || !envelope
                .report
                .hook_fingerprint
                .as_deref()
                .is_some_and(crate::metadata::valid_hook_fingerprint)
        {
            return None;
        }
    }
    Some((envelope, bytes))
}

#[cfg(unix)]
struct MailboxDirectory {
    handle: File,
    scan: Mutex<Option<DirectoryScan>>,
}

#[cfg(unix)]
struct DirectoryScan(*mut libc::DIR);
// SAFETY: a stream has no thread affinity and is accessed only through its
// owning MailboxDirectory mutex. No DIR pointer or dirent reference escapes.
#[cfg(unix)]
unsafe impl Send for DirectoryScan {}
#[cfg(unix)]
impl Drop for DirectoryScan {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
    }
}

#[cfg(unix)]
impl MailboxDirectory {
    fn open(path: &Path) -> io::Result<Self> {
        use std::os::unix::fs::OpenOptionsExt;

        let handle = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        if !handle.metadata()?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Codex report mailbox is not a directory",
            ));
        }
        Ok(Self {
            handle,
            scan: Mutex::new(None),
        })
    }

    fn entries(&self, limit: usize) -> io::Result<Vec<std::ffi::OsString>> {
        use std::ffi::CStr;
        use std::os::fd::AsRawFd;
        use std::os::unix::ffi::OsStringExt;

        let mut scan = self.scan.lock().unwrap();
        if scan.is_none() {
            let descriptor = unsafe {
                libc::openat(
                    self.handle.as_raw_fd(),
                    b".\0".as_ptr().cast(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                )
            };
            if descriptor < 0 {
                return Err(io::Error::last_os_error());
            }
            let stream = unsafe { libc::fdopendir(descriptor) };
            if stream.is_null() {
                let error = io::Error::last_os_error();
                unsafe { libc::close(descriptor) };
                return Err(error);
            }
            *scan = Some(DirectoryScan(stream));
        }
        let mut names = Vec::with_capacity(limit);
        while names.len() < limit {
            // SAFETY: the mutex exclusively owns this retained directory
            // stream; each name is copied before the next readdir call.
            let entry = unsafe { libc::readdir(scan.as_ref().expect("initialized scan").0) };
            if entry.is_null() {
                *scan = None;
                break;
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name != b"." && name != b".." {
                names.push(std::ffi::OsString::from_vec(name.to_vec()));
            }
        }
        Ok(names)
    }

    fn open_report(&self, name: &OsStr) -> io::Result<File> {
        use std::ffi::CString;
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::ffi::OsStrExt;

        let name = CString::new(name.as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid report name"))?;
        let descriptor = unsafe {
            libc::openat(
                self.handle.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openat returned a new descriptor owned by this call.
        let file = unsafe { File::from_raw_fd(descriptor) };
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Codex hook report is not a regular file",
            ));
        }
        Ok(file)
    }

    fn remove(&self, name: &OsStr) -> io::Result<()> {
        use std::ffi::CString;
        use std::os::fd::AsRawFd;
        use std::os::unix::ffi::OsStrExt;

        let name = CString::new(name.as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid report name"))?;
        if unsafe { libc::unlinkat(self.handle.as_raw_fd(), name.as_ptr(), 0) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(windows)]
struct MailboxDirectory {
    path: PathBuf,
    _handle: File,
    scan: Mutex<Option<std::fs::ReadDir>>,
}

#[cfg(windows)]
impl MailboxDirectory {
    fn open(path: &Path) -> io::Result<Self> {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_SHARE_READ, FILE_SHARE_WRITE,
        };

        let handle = std::fs::OpenOptions::new()
            .read(true)
            // Omitting FILE_SHARE_DELETE prevents mailbox rename/replacement while open.
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = handle.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Codex report mailbox is not a plain directory",
            ));
        }
        Ok(Self {
            path: path.to_path_buf(),
            _handle: handle,
            scan: Mutex::new(None),
        })
    }

    fn entries(&self, limit: usize) -> io::Result<Vec<std::ffi::OsString>> {
        let mut scan = self.scan.lock().unwrap();
        if scan.is_none() {
            *scan = Some(std::fs::read_dir(&self.path)?);
        }
        let mut names = Vec::with_capacity(limit);
        while names.len() < limit {
            match scan.as_mut().expect("initialized scan").next() {
                Some(entry) => names.push(entry?.file_name()),
                None => {
                    *scan = None;
                    break;
                }
            }
        }
        Ok(names)
    }

    fn open_report(&self, name: &OsStr) -> io::Result<File> {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };

        let file = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(self.path.join(name))?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Codex hook report is not a plain file",
            ));
        }
        Ok(file)
    }

    fn remove(&self, name: &OsStr) -> io::Result<()> {
        std::fs::remove_file(self.path.join(name))
    }
}

#[cfg(not(any(unix, windows)))]
struct MailboxDirectory {
    _path: PathBuf,
}

#[cfg(not(any(unix, windows)))]
impl MailboxDirectory {
    fn open(_: &Path) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "safe Codex report mailboxes are unsupported on this platform",
        ))
    }

    fn entries(&self, _: usize) -> io::Result<Vec<std::ffi::OsString>> {
        unreachable!()
    }

    fn open_report(&self, _: &OsStr) -> io::Result<File> {
        unreachable!()
    }

    fn remove(&self, _: &OsStr) -> io::Result<()> {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::is_report_filename;
    use crate::test_support::{
        test_app_state_with_workspace, test_session_info, test_session_metadata,
    };
    use std::ffi::OsStr;

    fn identity_fixture() -> (
        tempfile::TempDir,
        std::sync::Arc<crate::AppState>,
        String,
        u64,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        let id = format!("relay-budget-{}", uuid::Uuid::new_v4());
        let mut meta = test_session_metadata(&id, "relay", "/tmp", "running", "now", "now");
        meta.harness = "codex".into();
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let mut info = test_session_info(&id, "relay", "/tmp", "running", "now");
        info.harness = Some("codex".into());
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut runtime = super::super::session_runtime::SessionRuntime::detached_test();
        let generation = runtime.run_generation();
        runtime.native_approval =
            Some(super::super::codex_approval_application::NativeApprovalState::new(generation));
        state.sessions.lock().unwrap().insert(
            id.clone(),
            crate::SessionHandle {
                info,
                kill_tx,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                runtime,
                terminal_attached: false,
                resume_in_progress: false,
                capacity: crate::capacity_state::CapacityState::default(),
                active_work_hook: false,
            },
        );
        super::super::terminal_runtime::set_workflow_report_token(&id, "relay-budget-token".into());
        (dir, state, id, generation)
    }

    fn queue_identity(relay: &super::CodexHookReportRelay) -> std::path::PathBuf {
        let path = relay
            .mailbox_path()
            .join(format!("{}.json", uuid::Uuid::new_v4().simple()));
        std::fs::write(
            &path,
            serde_json::json!({"report": {
                "harnessSessionId":"root-id", "source":"codex_hook", "confidence":0.98,
                "hookFingerprint":"a".repeat(64)
            }})
            .to_string(),
        )
        .unwrap();
        path
    }

    #[tokio::test]
    async fn consume_ready_uses_full_identity_budget_without_retries() {
        let (_dir, state, id, generation) = identity_fixture();
        let relay = super::CodexHookReportRelay::new().unwrap();
        let paths: Vec<_> = (0..24).map(|_| queue_identity(&relay)).collect();

        let outcome = relay
            .consume_ready(state.clone(), &id, "relay-budget-token", generation)
            .await;

        assert_eq!(
            outcome.reports_consumed, 24,
            "unused retry slots must remain available to ordinary identity reports"
        );
        assert_eq!(outcome.reports_accepted, 24);
        assert!(paths.iter().all(|path| !path.exists()));
        assert!(relay.receipts.lock().unwrap().pending.is_empty());
        super::super::terminal_runtime::clear_workflow_report_token(&id);
    }

    #[tokio::test]
    async fn consume_ready_reserves_new_identity_capacity_under_retry_pressure() {
        let (_dir, state, id, generation) = identity_fixture();
        let relay = super::CodexHookReportRelay::new().unwrap();
        let metadata_path = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .sessions_dir()
            .join(format!("{id}.json"));
        let held_path = metadata_path.with_extension("held");
        std::fs::rename(&metadata_path, &held_path).unwrap();
        // Missing durable metadata makes actual approval application return Retry.
        for _ in 0..32 {
            let path = relay
                .mailbox_path()
                .join(format!("{}.json", uuid::Uuid::new_v4().simple()));
            std::fs::write(
                path,
                serde_json::json!({"approval": {
                    "rootId":"root-id", "turnId":null, "toolUseId":"tool",
                    "event":"PreToolUse", "observedAt":chrono::Utc::now().to_rfc3339(),
                    "hookFingerprint":"a".repeat(64)
                }})
                .to_string(),
            )
            .unwrap();
        }
        for _ in 0..2 {
            let outcome = relay
                .consume_ready(state.clone(), &id, "relay-budget-token", generation)
                .await;
            assert_eq!(outcome.reports_consumed, 0);
        }
        let (retry_order, original_receipts) = {
            let book = relay.receipts.lock().unwrap();
            assert_eq!(book.pending.len(), 32);
            (
                book.retry_order.clone(),
                book.pending
                    .iter()
                    .map(|(name, receipt)| {
                        (name.clone(), (receipt.bytes.clone(), receipt.first_receipt))
                    })
                    .collect::<std::collections::HashMap<_, _>>(),
            )
        };
        std::fs::rename(held_path, metadata_path).unwrap();
        let identities: Vec<_> = (0..24).map(|_| queue_identity(&relay)).collect();
        // Start the pressure pass at a complete scan page, after fixture setup.
        *relay.mailbox.as_ref().unwrap().scan.lock().unwrap() = None;

        let outcome = relay
            .consume_ready(state.clone(), &id, "relay-budget-token", generation)
            .await;

        assert_eq!(
            outcome.reports_consumed, 32,
            "retry and new reports share the unchanged total cap"
        );
        assert_eq!(outcome.reports_accepted, 16);
        assert_eq!(
            identities.iter().filter(|path| !path.exists()).count(),
            16,
            "full retry pressure must leave sixteen new identity slots"
        );
        let book = relay.receipts.lock().unwrap();
        assert_eq!(
            book.retry_order,
            retry_order
                .into_iter()
                .skip(16)
                .collect::<std::collections::VecDeque<_>>()
        );
        assert_eq!(book.pending.len(), 16);
        for (name, receipt) in &book.pending {
            assert_eq!(receipt.bytes, original_receipts[name].0);
            assert_eq!(
                receipt.first_receipt, original_receipts[name].1,
                "retry rotation must retain the original receipt time"
            );
        }
        drop(book);
        super::super::terminal_runtime::clear_workflow_report_token(&id);
    }

    #[test]
    fn only_uuid_json_names_are_consumable() {
        assert!(is_report_filename(OsStr::new(
            "00112233445566778899aabbccddeeff.json"
        )));
        assert!(!is_report_filename(OsStr::new(".pending-001122.json")));
        assert!(!is_report_filename(OsStr::new("not-a-uuid.json")));
        assert!(!is_report_filename(OsStr::new(
            "00112233-4455-6677-8899-aabbccddeeff.json"
        )));
    }

    #[test]
    fn reads_closed_approval_envelope_without_weakening_identity() {
        let relay = super::CodexHookReportRelay::new().unwrap();
        let name = std::ffi::OsString::from("00112233445566778899aabbccddeeff.json");
        let valid = serde_json::json!({"approval": {
            "rootId":"root", "turnId":null, "toolUseId":null,
            "event":"PermissionRequest", "observedAt":"2026-10-05T01:02:03.123456Z",
            "hookFingerprint":"a".repeat(64)
        }});
        std::fs::write(relay.mailbox_path().join(&name), valid.to_string()).unwrap();
        assert!(super::read_report(relay.mailbox.as_ref().unwrap(), &name).is_some());
        let mut token = valid.clone();
        token["approval"]["reportToken"] = serde_json::json!("secret");
        let mut free_text = valid.clone();
        free_text["approval"]["toolInput"] = serde_json::json!({"command":"secret"});
        let mut mixed = valid.clone();
        mixed["report"] = serde_json::json!({"harnessSessionId":"root","source":"codex_hook","confidence":0.98,"hookFingerprint":"a".repeat(64)});
        for invalid in [token, free_text, mixed] {
            std::fs::write(relay.mailbox_path().join(&name), invalid.to_string()).unwrap();
            assert!(super::read_report(relay.mailbox.as_ref().unwrap(), &name).is_none());
        }
        std::fs::write(relay.mailbox_path().join(&name), vec![b' '; 4097]).unwrap();
        assert!(super::read_report(relay.mailbox.as_ref().unwrap(), &name).is_none());
    }

    #[test]
    fn retained_overflow_reports_cannot_starve_unvisited_reports() {
        let relay = super::CodexHookReportRelay::new().unwrap();
        let mut expected = std::collections::HashSet::new();
        for _ in 0..48 {
            let name = std::ffi::OsString::from(format!("{}.json", uuid::Uuid::new_v4().simple()));
            std::fs::write(relay.mailbox_path().join(&name), b"{}").unwrap();
            expected.insert(name);
        }
        let mut seen = std::collections::HashSet::new();
        for _ in 0..4 {
            let page = super::select_unvisited_reports(
                relay.mailbox.as_ref().unwrap(),
                16,
                &std::collections::HashSet::new(),
            )
            .unwrap();
            assert!(page.len() <= 16);
            // Keep every selected file, as full-book overflow must do.
            seen.extend(page);
        }
        assert_eq!(
            seen, expected,
            "retained overflow files must not starve later identity/Permission files"
        );
    }

    #[test]
    fn bounded_directory_pages_eventually_visit_reports_after_noise() {
        let relay = super::CodexHookReportRelay::new().unwrap();
        for sequence in 0..(super::MAX_MAILBOX_ENTRIES_PER_PASS * 2) {
            std::fs::write(
                relay.mailbox_path().join(format!("noise-{sequence}")),
                b"ignored",
            )
            .unwrap();
        }
        let name = format!("{}.json", uuid::Uuid::new_v4().simple());
        std::fs::write(relay.mailbox_path().join(&name), b"{}").unwrap();
        let mut seen = false;
        for _ in 0..4 {
            let entries = relay
                .mailbox
                .as_ref()
                .unwrap()
                .entries(super::MAX_MAILBOX_ENTRIES_PER_PASS)
                .unwrap();
            assert!(entries.len() <= super::MAX_MAILBOX_ENTRIES_PER_PASS);
            seen |= entries.contains(&std::ffi::OsString::from(&name));
        }
        assert!(
            seen,
            "bounded scans must advance, not rescan the same noise page forever"
        );
    }

    #[cfg(unix)]
    #[test]
    fn temp_cleanup_does_not_remove_a_replacement_directory() {
        let relay = super::CodexHookReportRelay::new().unwrap();
        let mailbox_path = relay.mailbox_path().to_path_buf();
        let moved_path = mailbox_path.with_file_name(format!(
            "{}-moved",
            mailbox_path.file_name().unwrap().to_string_lossy()
        ));
        std::fs::rename(&mailbox_path, &moved_path).unwrap();
        std::fs::create_dir(&mailbox_path).unwrap();
        let sentinel = mailbox_path.join("keep-me");
        std::fs::write(&sentinel, b"not the mailbox").unwrap();

        drop(relay);

        let replacement_survived = sentinel.exists();
        let original_survived = moved_path.exists();
        let _ = std::fs::remove_dir_all(&mailbox_path);
        let _ = std::fs::remove_dir_all(&moved_path);
        assert!(
            replacement_survived,
            "TempDir cleanup must not remove a replacement at the mailbox path"
        );
        assert!(
            original_survived,
            "a moved mailbox is left for manual cleanup"
        );
    }

    #[test]
    fn pending_files_do_not_consume_the_report_limit() {
        let relay = super::CodexHookReportRelay::new().unwrap();
        for sequence in 0..(super::MAX_REPORTS_PER_PASS * 2) {
            std::fs::write(
                relay.mailbox_path().join(format!(".pending-{sequence}")),
                b"partial",
            )
            .unwrap();
        }
        let mut report_name = None;
        for _ in 0..128 {
            let candidate = format!("{}.json", uuid::Uuid::new_v4().simple());
            std::fs::write(
                relay.mailbox_path().join(&candidate),
                br#"{"report":{"harnessSessionId":"native-id","source":"codex_hook","confidence":0.98,"hookFingerprint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#,
            )
            .unwrap();
            let first_entries = relay
                .mailbox
                .as_ref()
                .unwrap()
                .entries(super::MAX_REPORTS_PER_PASS)
                .unwrap();
            if !first_entries.contains(&std::ffi::OsString::from(&candidate)) {
                report_name = Some(candidate);
                break;
            }
            std::fs::remove_file(relay.mailbox_path().join(candidate)).unwrap();
        }
        let report_name = report_name.expect("place a completed report beyond the first scan page");
        // Placement inspection is separate from the production scan under test.
        *relay.mailbox.as_ref().unwrap().scan.lock().unwrap() = None;

        let report_names =
            super::select_reports(relay.mailbox.as_ref().unwrap(), super::MAX_REPORTS_PER_PASS)
                .unwrap();
        assert_eq!(
            report_names.len(),
            1,
            "abandoned staging files must not occupy the completed-report limit"
        );
        assert_eq!(report_names[0], std::ffi::OsString::from(report_name));
    }

    #[cfg(windows)]
    #[test]
    fn mailbox_handle_allows_reports_but_prevents_directory_replacement() {
        let relay = super::CodexHookReportRelay::new().unwrap();
        let report_path = relay
            .mailbox_path()
            .join(format!("{}.json", uuid::Uuid::new_v4().simple()));
        std::fs::write(&report_path, b"report").unwrap();

        let moved_path = relay.mailbox_path().with_file_name(format!(
            "{}-moved",
            relay.mailbox_path().file_name().unwrap().to_string_lossy()
        ));
        assert!(
            std::fs::rename(relay.mailbox_path(), moved_path).is_err(),
            "the held directory handle must prevent mailbox replacement"
        );
    }
}
