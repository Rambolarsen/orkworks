use crate::http::session_handlers::{
    report_harness_session_from_local_relay, HarnessSessionReportRequest,
};
use crate::runtime::terminal_runtime::verify_workflow_report_token;
use crate::AppState;
use axum::http::StatusCode;
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
#[cfg(not(unix))]
use std::path::PathBuf;
use std::sync::Arc;

const MAX_REPORT_BYTES: u64 = 4 * 1024;
const MAX_REPORTS_PER_PASS: usize = 32;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexHookReportEnvelope {
    report: HarnessSessionReportRequest,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RelayOutcome {
    pub(crate) reports_consumed: usize,
    pub(crate) reports_accepted: usize,
}

/// A private temporary directory whose path is passed only to one Codex
/// runtime. Its handle and lifetime are tied to the PTY driver's lifetime.
pub(crate) struct CodexHookReportRelay {
    mailbox: Option<MailboxDirectory>,
    directory: Option<tempfile::TempDir>,
}

impl CodexHookReportRelay {
    pub(crate) fn new() -> io::Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("orkworks-codex-hook-reports-")
            .tempdir()?;
        let mailbox = MailboxDirectory::open(directory.path())?;
        Ok(Self {
            mailbox: Some(mailbox),
            directory: Some(directory),
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
        let mailbox = self
            .mailbox
            .as_ref()
            .expect("Codex mailbox handle remains live");
        let Ok(mut reports) = mailbox.entries(MAX_REPORTS_PER_PASS) else {
            return outcome;
        };
        reports.retain(|name| is_report_filename(name));
        reports.sort();

        for name in reports {
            outcome.reports_consumed += 1;
            if let Some(report) = read_report(mailbox, &name) {
                let status = report_harness_session_from_local_relay(
                    state.clone(),
                    session_id.to_string(),
                    report,
                    report_token,
                )
                .await;
                if status == StatusCode::OK {
                    outcome.reports_accepted += 1;
                }
            }
            let _ = mailbox.remove(&name);
        }

        outcome
    }
}

impl Drop for CodexHookReportRelay {
    fn drop(&mut self) {
        let Some(directory) = self.directory.take() else {
            return;
        };
        let path = directory.path().to_path_buf();
        let path_still_names_mailbox = self
            .mailbox
            .as_ref()
            .is_some_and(|mailbox| mailbox.is_same_directory(&path));
        drop(self.mailbox.take());

        if path_still_names_mailbox {
            drop(directory);
        } else {
            // A child replaced or moved the path. Avoid letting TempDir remove
            // an unrelated directory now occupying the original pathname.
            // The displaced mailbox may remain in the system temp directory.
            let _ = directory.keep();
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

fn read_report(mailbox: &MailboxDirectory, name: &OsStr) -> Option<HarnessSessionReportRequest> {
    let mut file = mailbox.open_report(name).ok()?;
    let mut bytes = Vec::with_capacity(MAX_REPORT_BYTES as usize);
    file.by_ref()
        .take(MAX_REPORT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_REPORT_BYTES {
        return None;
    }
    let report = serde_json::from_slice::<CodexHookReportEnvelope>(&bytes)
        .ok()?
        .report;
    if report.source != "codex_hook"
        || !report
            .hook_fingerprint
            .as_deref()
            .is_some_and(crate::metadata::valid_hook_fingerprint)
    {
        return None;
    }
    Some(report)
}

#[cfg(unix)]
struct MailboxDirectory {
    handle: File,
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
        Ok(Self { handle })
    }

    fn entries(&self, limit: usize) -> io::Result<Vec<std::ffi::OsString>> {
        use std::ffi::CStr;
        use std::os::fd::AsRawFd;
        use std::os::unix::ffi::OsStringExt;

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

        struct DirectoryStream(*mut libc::DIR);
        impl Drop for DirectoryStream {
            fn drop(&mut self) {
                unsafe { libc::closedir(self.0) };
            }
        }
        let stream = DirectoryStream(stream);
        let mut names = Vec::with_capacity(limit);
        while names.len() < limit {
            let entry = unsafe { libc::readdir(stream.0) };
            if entry.is_null() {
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

    fn is_same_directory(&self, path: &Path) -> bool {
        use std::os::unix::fs::MetadataExt;

        let Ok(opened) = self.handle.metadata() else {
            return false;
        };
        let Ok(current) = std::fs::symlink_metadata(path) else {
            return false;
        };
        current.is_dir() && opened.dev() == current.dev() && opened.ino() == current.ino()
    }
}

#[cfg(windows)]
struct MailboxDirectory {
    path: PathBuf,
    _handle: File,
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
        })
    }

    fn entries(&self, limit: usize) -> io::Result<Vec<std::ffi::OsString>> {
        let mut names = Vec::with_capacity(limit);
        for entry in std::fs::read_dir(&self.path)? {
            names.push(entry?.file_name());
            if names.len() >= limit {
                break;
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

    fn is_same_directory(&self, path: &Path) -> bool {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

        let Ok(opened) = self._handle.metadata() else {
            return false;
        };
        let Ok(current) = std::fs::symlink_metadata(path) else {
            return false;
        };
        current.is_dir()
            && current.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
            && opened.volume_serial_number() == current.volume_serial_number()
            && opened.file_index() == current.file_index()
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

    fn is_same_directory(&self, _: &Path) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::is_report_filename;
    use std::ffi::OsStr;

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
