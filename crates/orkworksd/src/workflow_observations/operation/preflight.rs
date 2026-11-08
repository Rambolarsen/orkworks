//! Bounded non-following SQL-tag refusal, before tolerant legacy repair.
use super::*;
use std::io::{BufRead, Read};
#[cfg(test)]
thread_local! {pub(super) static AFTER_ENTRY:std::cell::RefCell<Option<Box<dyn FnOnce()>>>=std::cell::RefCell::new(None);}

fn refused() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "SQL observation evidence requires checked opening",
    )
}
// Inspect lexical top-level keys even if an earlier damaged value cannot be
// deserialized. A reserved tag must never reach tolerant tail repair.
fn string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut escaped = false;
    for (offset, &byte) in bytes.get(start + 1..)?.iter().enumerate() {
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            return Some(start + offset + 2);
        }
    }
    None
}
fn whitespace(bytes: &[u8], mut at: usize) -> usize {
    while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
        at += 1;
    }
    at
}
fn sql_tag(bytes: &[u8]) -> bool {
    if bytes.get(whitespace(bytes, 0)) != Some(&b'{') {
        return false;
    }
    let mut at = 0;
    let mut depth = 0usize;
    while at < bytes.len() {
        match bytes[at] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth = depth.saturating_sub(1),
            b'"' => {
                let Some(end) = string_end(bytes, at) else {
                    return false;
                };
                if depth == 1
                    && serde_json::from_slice::<String>(&bytes[at..end])
                        .ok()
                        .as_deref()
                        == Some("record")
                {
                    let colon = whitespace(bytes, end);
                    if bytes.get(colon) == Some(&b':') {
                        let value = whitespace(bytes, colon + 1);
                        if bytes.get(value) == Some(&b'"') {
                            if let Some(last) = string_end(bytes, value) {
                                if matches!(
                                    serde_json::from_slice::<String>(&bytes[value..last])
                                        .ok()
                                        .as_deref(),
                                    Some("sql_observation_v1" | "sql_tombstone_v1")
                                ) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                at = end;
                continue;
            }
            _ => {}
        }
        at += 1;
    }
    false
}
fn linked(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
        {
            return true;
        }
    }
    false
}
fn check(deadline: std::time::Instant) -> io::Result<()> {
    if std::time::Instant::now() >= deadline {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "observation preflight expired",
        ))
    } else {
        Ok(())
    }
}
pub(in crate::workflow_observations) fn legacy_preflight(root: &Path) -> io::Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    check(deadline)?;
    for name in ["format.json", "session-store"] {
        let result = fs::symlink_metadata(root.join(name));
        check(deadline)?;
        match result {
            Ok(_) => return Err(refused()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    let dir = root.join("workflow-observations");
    let metadata = fs::symlink_metadata(&dir);
    check(deadline)?;
    match metadata {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return check(deadline),
        Err(error) => return Err(error),
        Ok(metadata) if !metadata.is_dir() || linked(&metadata) => return Err(refused()),
        Ok(_) => {}
    }
    let entries = fs::read_dir(&dir);
    check(deadline)?;
    let mut inspected = 0usize;
    let mut total = 0usize;
    let mut lines = 0usize;
    for entry in entries? {
        check(deadline)?;
        inspected += 1;
        if inspected > 65537 {
            return Err(refused());
        }
        let entry = entry?;
        #[cfg(test)]
        AFTER_ENTRY.with(|slot| {
            let callback = slot.borrow_mut().take();
            if let Some(callback) = callback {
                callback();
            }
        });
        check(deadline)?;
        if entry.path().extension() != Some(std::ffi::OsStr::new("ndjson")) {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path());
        check(deadline)?;
        let metadata = metadata?;
        if !metadata.is_file() || linked(&metadata) {
            return Err(refused());
        }
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(
                windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT,
            );
        }
        let file = options.open(entry.path());
        check(deadline)?;
        let file = file?;
        let opened = file.metadata();
        check(deadline)?;
        let opened = opened?;
        if !opened.is_file() || linked(&opened) {
            return Err(refused());
        }
        let mut reader = io::BufReader::with_capacity(8192, file);
        let mut line = Vec::with_capacity(64 * 1024);
        loop {
            check(deadline)?;
            line.clear();
            let count = reader
                .by_ref()
                .take(64 * 1024 + 2)
                .read_until(b'\n', &mut line);
            check(deadline)?;
            let count = count?;
            if count == 0 {
                break;
            }
            total += count;
            lines += 1;
            if total > 64 * 1024 * 1024
                || lines > 65536
                || count > 64 * 1024 + usize::from(line.last() == Some(&b'\n'))
            {
                return Err(refused());
            }
            if sql_tag(&line) {
                return Err(refused());
            }
        }
    }
    check(deadline)
}
