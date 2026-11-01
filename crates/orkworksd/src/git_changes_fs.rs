//! Anchored no-follow worktree reads. Directory handles stay alive during traversal.
use super::git_changes::{safe_path, INPUT_LIMIT};
use super::git_changes_eol::{Eol, Stats};
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

pub struct Entry {
    pub path: PathBuf,
    pub mode: i32,
    pub content: Result<Option<Vec<u8>>, String>,
}

/// Read one regular worktree file without following any ancestor or leaf link.
pub(crate) fn read_regular(root: &Path, relative: &Path) -> io::Result<Vec<u8>> {
    crate::git_changes::safe_path(relative).map_err(io::Error::other)?;
    let root = root.canonicalize()?;
    #[cfg(unix)]
    let file = {
        use std::{
            ffi::CString,
            os::{
                fd::{AsRawFd, FromRawFd},
                unix::{ffi::OsStrExt, fs::OpenOptionsExt},
            },
        };
        let full = root.join(relative);
        let mut held = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/")?;
        let parts: Vec<_> = full
            .components()
            .filter_map(|c| {
                if let std::path::Component::Normal(n) = c {
                    Some(n)
                } else {
                    None
                }
            })
            .collect();
        for (i, part) in parts.iter().enumerate() {
            let name = CString::new(part.as_bytes()).map_err(io::Error::other)?;
            let flags = libc::O_RDONLY
                | libc::O_NOFOLLOW
                | libc::O_NONBLOCK
                | libc::O_CLOEXEC
                | if i + 1 < parts.len() {
                    libc::O_DIRECTORY
                } else {
                    0
                };
            // SAFETY: held owns the directory fd; name is NUL-terminated. The
            // returned fd is adopted exactly once after checking for failure.
            let fd = unsafe { libc::openat(held.as_raw_fd(), name.as_ptr(), flags) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            held = unsafe { File::from_raw_fd(fd) };
        }
        held
    };
    #[cfg(windows)]
    let (file, _ancestors) = {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        let mut path = PathBuf::new();
        let mut ancestors = Vec::new();
        let full = root.join(relative);
        let mut last = None;
        for component in full.components() {
            path.push(component.as_os_str());
            if !matches!(component, std::path::Component::Normal(_)) {
                continue;
            }
            let f = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(3)
                .custom_flags(0x00200000 | 0x02000000)
                .open(&path)?;
            if f.metadata()?.file_attributes() & 0x400 != 0 {
                return Err(io::Error::other("Document links are unavailable."));
            }
            if let Some(previous) = last.take() {
                ancestors.push(previous);
            }
            last = Some(f);
        }
        (
            last.ok_or_else(|| io::Error::other("Invalid document path."))?,
            ancestors,
        )
    };
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("Document is not a regular file."));
    }
    bounded(file, None, Eol::Raw, &mut 0, std::time::Instant::now())
        .map_err(io::Error::other)?
        .ok_or_else(|| io::Error::other("Document unavailable."))
}

#[cfg(unix)]
pub fn walk(
    root: &Path,
    mut visit: impl FnMut(Entry) -> bool,
    mut descend: impl FnMut(&Path, bool) -> bool,
    mut expected: impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
    mut prepare: impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
) -> io::Result<()> {
    use std::{
        ffi::{CStr, CString},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::OpenOptionsExt},
        },
    };
    fn open_dir(fd: i32, name: &CStr) -> io::Result<File> {
        // SAFETY: name is NUL-terminated; fd is owned by a live File. Returned fd is adopted once.
        let child = unsafe {
            libc::openat(
                fd,
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if child < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(unsafe { File::from_raw_fd(child) })
        }
    }
    fn recurse(
        dir: &File,
        prefix: &Path,
        visit: &mut impl FnMut(Entry) -> bool,
        descend: &mut impl FnMut(&Path, bool) -> bool,
        count: &mut usize,
        start: std::time::Instant,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        hash_bytes: &mut usize,
        prepare: &mut impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
        control_bytes: &mut usize,
    ) -> io::Result<bool> {
        let mut control = |name: &CStr| -> io::Result<Option<Vec<u8>>> {
            let fd = unsafe {
                libc::openat(
                    dir.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                let e = io::Error::last_os_error();
                return if e.kind() == io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(e)
                };
            }
            let file = unsafe { File::from_raw_fd(fd) };
            if !file.metadata()?.is_file() {
                return Err(io::Error::other(
                    "Unsupported Git control file; comparison unavailable.",
                ));
            }
            let data = bounded(file, None, Eol::Raw, hash_bytes, start)
                .map_err(io::Error::other)?
                .unwrap_or_default();
            *control_bytes += data.len();
            if *control_bytes > 4 * 1024 * 1024 {
                return Err(io::Error::other("Git control-file budget exceeded."));
            }
            Ok(Some(data))
        };
        let attrs = control(c".gitattributes")?;
        let ignore = control(c".gitignore")?;
        prepare(prefix, attrs, ignore).map_err(io::Error::other)?;
        // Open a new description so fdopendir's stream offset does not modify the anchored handle.
        let duplicate = open_dir(dir.as_raw_fd(), c".")?;
        use std::os::fd::IntoRawFd;
        let raw = duplicate.into_raw_fd();
        // SAFETY: fdopendir consumes raw on success. Dir's Drop closes the stream exactly once.
        let stream = unsafe { libc::fdopendir(raw) };
        if stream.is_null() {
            unsafe {
                libc::close(raw);
            }
            return Err(io::Error::last_os_error());
        }
        struct Dir(*mut libc::DIR);
        impl Drop for Dir {
            fn drop(&mut self) {
                unsafe {
                    libc::closedir(self.0);
                }
            }
        }
        let stream = Dir(stream);
        loop {
            let item = unsafe { libc::readdir(stream.0) };
            if item.is_null() {
                break;
            }
            *count += 1;
            if *count > 10_000 || start.elapsed() > std::time::Duration::from_secs(5) {
                return Err(io::Error::other(
                    "Worktree traversal budget exceeded; comparison unavailable.",
                ));
            }
            let bytes = unsafe { CStr::from_ptr((*item).d_name.as_ptr()) }.to_bytes();
            if bytes == b"." || bytes == b".." || bytes.eq_ignore_ascii_case(b".git") {
                continue;
            }
            let name = CString::new(bytes).map_err(io::Error::other)?;
            let path = prefix.join(std::ffi::OsStr::from_bytes(bytes));
            if safe_path(&path).is_err() {
                return Err(io::Error::other(
                    "Unsupported Git path; comparison unavailable.",
                ));
            }
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            if unsafe {
                libc::fstatat(
                    dir.as_raw_fd(),
                    name.as_ptr(),
                    stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } != 0
            {
                if !visit(Entry {
                    path,
                    mode: 0,
                    content: Err("File changed or is unreadable; refresh.".into()),
                }) {
                    return Ok(false);
                }
                continue;
            }
            let stat = unsafe { stat.assume_init() };
            let kind = stat.st_mode & libc::S_IFMT;
            if kind == libc::S_IFDIR {
                if descend(&path, true) {
                    let child = open_dir(dir.as_raw_fd(), &name)?;
                    if !recurse(
                        &child,
                        &path,
                        visit,
                        descend,
                        count,
                        start,
                        expected,
                        hash_bytes,
                        prepare,
                        control_bytes,
                    )? {
                        return Ok(false);
                    }
                }
                continue;
            }
            if !descend(&path, false) {
                continue;
            }
            let (mode, content) = if kind == libc::S_IFLNK {
                let mut buf = vec![0u8; 65537];
                let n = unsafe {
                    libc::readlinkat(
                        dir.as_raw_fd(),
                        name.as_ptr(),
                        buf.as_mut_ptr().cast(),
                        buf.len(),
                    )
                };
                (
                    0o120000,
                    if n < 0 {
                        Err("Unreadable link.".into())
                    } else if n as usize >= buf.len() {
                        Err("Link too large.".into())
                    } else {
                        buf.truncate(n as usize);
                        Ok(Some(buf))
                    },
                )
            } else if kind == libc::S_IFREG {
                let mode = if stat.st_mode & 0o111 != 0 {
                    0o100755
                } else {
                    0o100644
                };
                let fd = unsafe {
                    libc::openat(
                        dir.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                    )
                };
                let result = if fd < 0 {
                    Err("Unreadable file; refresh.".into())
                } else {
                    let f = unsafe { File::from_raw_fd(fd) };
                    match f.metadata() {
                        Ok(m) if m.is_file() => expected(&path, mode)
                            .and_then(|(id, eol)| bounded(f, id, eol, hash_bytes, start)),
                        _ => Err("Special file; content unavailable.".into()),
                    }
                };
                (mode, result)
            } else {
                (0, Err("Special file; content unavailable.".into()))
            };
            if !visit(Entry {
                path,
                mode,
                content,
            }) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    // Start at the filesystem root and open every canonical component relative
    // to the held parent. No ancestor symlink can redirect the root open.
    let mut anchored = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    for part in root.components() {
        if let std::path::Component::Normal(name) = part {
            let name = CString::new(name.as_bytes()).map_err(io::Error::other)?;
            anchored = open_dir(anchored.as_raw_fd(), &name)?;
        }
    }
    let root = anchored;
    recurse(
        &root,
        Path::new(""),
        &mut visit,
        &mut descend,
        &mut 0,
        std::time::Instant::now(),
        &mut expected,
        &mut 0,
        &mut prepare,
        &mut 0,
    )?;
    Ok(())
}

#[cfg(windows)]
pub fn walk(
    root: &Path,
    mut visit: impl FnMut(Entry) -> bool,
    mut descend: impl FnMut(&Path, bool) -> bool,
    mut expected: impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
    mut prepare: impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
) -> io::Result<()> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    // Omit FILE_SHARE_DELETE, pinning each ancestor against rename/replacement.
    fn pin(path: &Path) -> io::Result<File> {
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .custom_flags(0x00200000 | 0x02000000)
            .open(path)
    }
    fn recurse(
        root: &Path,
        relative: &Path,
        visit: &mut impl FnMut(Entry) -> bool,
        descend: &mut impl FnMut(&Path, bool) -> bool,
        count: &mut usize,
        start: std::time::Instant,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        hash_bytes: &mut usize,
        prepare: &mut impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
        control_bytes: &mut usize,
    ) -> io::Result<bool> {
        let dir = pin(&root.join(relative))?;
        if dir.metadata()?.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("Reparse directory unavailable."));
        }
        let mut control = |name: &str| -> io::Result<Option<Vec<u8>>> {
            let f = match pin(&root.join(relative).join(name)) {
                Ok(f) => f,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e),
            };
            let metadata = f.metadata()?;
            if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
                return Err(io::Error::other("Unsupported Git control file."));
            }
            let data = bounded(f, None, Eol::Raw, hash_bytes, start)
                .map_err(io::Error::other)?
                .unwrap_or_default();
            *control_bytes += data.len();
            if *control_bytes > 4 * 1024 * 1024 {
                return Err(io::Error::other("Git control-file budget exceeded."));
            }
            Ok(Some(data))
        };
        let attrs = control(".gitattributes")?;
        let ignore = control(".gitignore")?;
        prepare(relative, attrs, ignore).map_err(io::Error::other)?;
        for item in std::fs::read_dir(root.join(relative))? {
            *count += 1;
            if *count > 10_000 || start.elapsed() > std::time::Duration::from_secs(5) {
                return Err(io::Error::other(
                    "Worktree traversal budget exceeded; comparison unavailable.",
                ));
            }
            let item = item?;
            let path = relative.join(item.file_name());
            if item
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(".git")
            {
                continue;
            }
            if safe_path(&path).is_err() {
                return Err(io::Error::other(
                    "Unsupported Git path; comparison unavailable.",
                ));
            }
            let held = match pin(&root.join(&path)) {
                Ok(f) => f,
                Err(_) => {
                    if !visit(Entry {
                        path,
                        mode: 0,
                        content: Err("Unreadable file.".into()),
                    }) {
                        return Ok(false);
                    }
                    continue;
                }
            };
            let meta = held.metadata()?;
            if !descend(&path, meta.is_dir() && meta.file_attributes() & 0x400 == 0) {
                continue;
            }
            if meta.file_attributes() & 0x400 != 0 {
                // The reparse point itself is pinned; read_link does not dereference its target.
                let content = std::fs::read_link(root.join(&path))
                    .ok()
                    .and_then(|p| p.to_str().map(|s| s.as_bytes().to_vec()))
                    .ok_or("Unsupported reparse point.".into())
                    .map(Some);
                if !visit(Entry {
                    path,
                    mode: 0o120000,
                    content,
                }) {
                    return Ok(false);
                }
            } else if meta.is_dir() {
                if descend(&path, true)
                    && !recurse(
                        root,
                        &path,
                        visit,
                        descend,
                        count,
                        start,
                        expected,
                        hash_bytes,
                        prepare,
                        control_bytes,
                    )?
                {
                    return Ok(false);
                }
            } else if !visit(Entry {
                path: path.clone(),
                mode: 0o100644,
                content: if meta.is_file() {
                    expected(&path, 0o100644)
                        .and_then(|(id, eol)| bounded(held, id, eol, hash_bytes, start))
                } else {
                    Err("Special file; content unavailable.".into())
                },
            }) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    // Keep every normal ancestor open without FILE_SHARE_DELETE, and reject
    // reparse points before proceeding to the next component.
    let mut pinned = Vec::new();
    let mut path = PathBuf::new();
    for component in root.components() {
        path.push(component.as_os_str());
        if matches!(component, std::path::Component::Normal(_)) {
            let handle = pin(&path)?;
            let metadata = handle.metadata()?;
            if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
                return Err(io::Error::other("Reparse ancestor unavailable."));
            }
            pinned.push(handle);
        }
    }
    recurse(
        root,
        Path::new(""),
        &mut visit,
        &mut descend,
        &mut 0,
        std::time::Instant::now(),
        &mut expected,
        &mut 0,
        &mut prepare,
        &mut 0,
    )?;
    Ok(())
}
fn bounded(
    mut f: File,
    expected: Option<git2::Oid>,
    eol: Eol,
    hash_bytes: &mut usize,
    start: std::time::Instant,
) -> Result<Option<Vec<u8>>, String> {
    let size = f.metadata().map_err(|_| "Unreadable file.")?.len();
    if size > INPUT_LIMIT as u64 {
        let expected = expected.ok_or("File exceeds 1 MiB; patch unavailable.")?;
        // Compare large tracked files without capturing their text. The separate
        // streaming budget bounds I/O; it does not enlarge the render/input budget.
        if size > 64 * 1024 * 1024 || (*hash_bytes as u64) + size > 64 * 1024 * 1024 {
            return Err("Large-file equality budget exceeded; content unavailable.".into());
        }
        use sha1::{Digest, Sha1};
        let mut hash = Sha1::new();
        hash.update(format!("blob {size}\0").as_bytes());
        let mut buf = [0u8; 65536];
        let mut stats = Stats::default();
        let mut read = 0u64;
        loop {
            if start.elapsed() > std::time::Duration::from_secs(5) {
                return Err("Capture budget exceeded; content unavailable.".into());
            }
            let n = f.read(&mut buf).map_err(|_| "Unreadable file.")?;
            if n == 0 {
                break;
            }
            read += n as u64;
            *hash_bytes += n;
            if read > size || *hash_bytes > 64 * 1024 * 1024 {
                return Err("File changed or equality budget exceeded; refresh.".into());
            }
            stats.add(&buf[..n]);
            hash.update(&buf[..n]);
        }
        if read == size
            && hash.finalize().as_slice() == expected.as_bytes()
            && (!eol.converts(&stats) || stats.crlf == 0 || eol == Eol::AutoIndexSame)
        {
            return Ok(None);
        }
        if eol.converts(&stats) && stats.crlf > 0 {
            use std::io::{Seek, SeekFrom};
            if (*hash_bytes as u64) + size > 64 * 1024 * 1024 {
                return Err("Large-file equality budget exceeded; content unavailable.".into());
            }
            f.seek(SeekFrom::Start(0))
                .map_err(|_| "File changed; refresh.")?;
            let normalized_size = size - stats.crlf;
            let mut hash = Sha1::new();
            hash.update(format!("blob {normalized_size}\0").as_bytes());
            let mut pending_cr = false;
            let mut output = 0u64;
            let mut raw = 0u64;
            loop {
                if start.elapsed() > std::time::Duration::from_secs(5) {
                    return Err("Capture budget exceeded; content unavailable.".into());
                }
                let n = f.read(&mut buf).map_err(|_| "Unreadable file.")?;
                if n == 0 {
                    break;
                }
                raw += n as u64;
                *hash_bytes += n;
                if raw > size || *hash_bytes > 64 * 1024 * 1024 {
                    return Err("File changed or equality budget exceeded; refresh.".into());
                }
                let mut clean = Vec::with_capacity(n + 1);
                for &b in &buf[..n] {
                    if pending_cr {
                        if b != b'\n' {
                            clean.push(b'\r')
                        }
                        pending_cr = false;
                    }
                    if b == b'\r' {
                        pending_cr = true
                    } else {
                        clean.push(b)
                    }
                }
                output += clean.len() as u64;
                hash.update(&clean);
            }
            if pending_cr {
                hash.update(b"\r");
                output += 1;
            }
            if raw == size
                && output == normalized_size
                && hash.finalize().as_slice() == expected.as_bytes()
            {
                return Ok(None);
            }
        }
        return Err("File exceeds 1 MiB; patch unavailable.".into());
    }
    let mut buf = Vec::new();
    f.take((INPUT_LIMIT + 1) as u64)
        .read_to_end(&mut buf)
        .map_err(|_| "Unreadable file.")?;
    if buf.len() > INPUT_LIMIT {
        Err("File exceeds 1 MiB; patch unavailable.".into())
    } else {
        eol.clean(&mut buf);
        Ok(Some(buf))
    }
}

/// Reads only caller-resolved Git administrative/global control metadata; source
/// controls use the anchored per-directory reader above. Never follows a leaf
/// link or blocks on a FIFO, and never accepts arbitrary renderer paths.
pub fn metadata_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(3).custom_flags(0x00200000 | 0x02000000);
    }
    let file = match options.open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let metadata = file.metadata()?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("Git metadata link unavailable."));
        }
    }
    if !metadata.is_file() {
        return Err(io::Error::other("Unsupported Git control metadata."));
    }
    bounded(file, None, Eol::Raw, &mut 0, std::time::Instant::now()).map_err(io::Error::other)
}
