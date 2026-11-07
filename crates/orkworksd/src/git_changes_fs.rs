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

/// Advance and test the per-traversal candidate and time budget.
fn traversal_exceeded(count: &mut usize, start: std::time::Instant) -> bool {
    *count += 1;
    *count > 10_000 || start.elapsed() > std::time::Duration::from_secs(5)
}

/// Budget check as a result, for caller-local `?` propagation.
fn traversal_stop(count: &mut usize, start: std::time::Instant) -> io::Result<()> {
    if traversal_exceeded(count, start) {
        return Err(io::Error::other(
            "Worktree traversal budget exceeded; comparison unavailable.",
        ));
    }
    Ok(())
}

#[cfg(unix)]
mod unix_walk {
    use super::super::git_changes_eol::Eol;
    use super::{bounded, safe_path};
    use super::{traversal_stop, Entry};
    use std::{
        ffi::{CStr, CString},
        fs::File,
        io,
        os::fd::{AsRawFd, FromRawFd, IntoRawFd},
        os::unix::ffi::OsStrExt,
        path::{Path, PathBuf},
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

    /// Opened directory stream, closed exactly once on drop.
    struct Dir(*mut libc::DIR);
    impl Drop for Dir {
        fn drop(&mut self) {
            unsafe {
                libc::closedir(self.0);
            }
        }
    }

    /// Open a new description so fdopendir's stream offset does not modify the anchored handle.
    fn open_stream(dir: &File) -> io::Result<Dir> {
        let duplicate = open_dir(dir.as_raw_fd(), c".")?;
        let raw = duplicate.into_raw_fd();
        // SAFETY: fdopendir consumes raw on success. Dir's Drop closes the stream exactly once.
        let stream = unsafe { libc::fdopendir(raw) };
        if stream.is_null() {
            unsafe {
                libc::close(raw);
            }
            return Err(io::Error::last_os_error());
        }
        Ok(Dir(stream))
    }

    /// Bounded read of one top-level directory control file.
    fn read_control(
        dir: &File,
        name: &CStr,
        hash_bytes: &mut usize,
        start: std::time::Instant,
        control_bytes: &mut usize,
    ) -> io::Result<Option<Vec<u8>>> {
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
    }

    /// fstatat the entry without following its leaf link; None when it vanished.
    fn entry_stat(dir: &File, name: &CStr) -> Option<libc::stat> {
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
            None
        } else {
            Some(unsafe { stat.assume_init() })
        }
    }

    /// Link text of a worktree symlink, read from the anchored directory.
    fn link_content(dir: &File, name: &CStr) -> Result<Option<Vec<u8>>, String> {
        let mut buf = vec![0u8; 65537];
        let n = unsafe {
            libc::readlinkat(
                dir.as_raw_fd(),
                name.as_ptr(),
                buf.as_mut_ptr().cast(),
                buf.len(),
            )
        };
        if n < 0 {
            Err("Unreadable link.".into())
        } else if n as usize >= buf.len() {
            Err("Link too large.".into())
        } else {
            buf.truncate(n as usize);
            Ok(Some(buf))
        }
    }

    /// Capture content of one regular worktree leaf, honoring its expected blob.
    fn regular_content(
        dir: &File,
        name: &CStr,
        path: &Path,
        mode: i32,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        hash_bytes: &mut usize,
        start: std::time::Instant,
    ) -> Result<Option<Vec<u8>>, String> {
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err("Unreadable file; refresh.".into());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        match file.metadata() {
            Ok(m) if m.is_file() => {
                expected(path, mode).and_then(|(id, eol)| bounded(file, id, eol, hash_bytes, start))
            }
            _ => Err("Special file; content unavailable.".into()),
        }
    }

    /// Visit an entry the traversal could not read, keeping the walk-alive rules.
    fn visit_unreadable(
        visit: &mut impl FnMut(Entry) -> bool,
        path: PathBuf,
        content: &str,
    ) -> io::Result<bool> {
        Ok(visit(Entry {
            path,
            mode: 0,
            content: Err(content.into()),
        }))
    }

    /// One readdir outcome: stream end, a skipped name, or a validated entry.
    enum Listing {
        End,
        Skip,
        Entry(CString),
    }

    /// Consume one directory entry, checking budgets, dot names and safe paths.
    fn next_entry(
        stream: *mut libc::DIR,
        prefix: &Path,
        count: &mut usize,
        start: std::time::Instant,
    ) -> io::Result<Listing> {
        let item = unsafe { libc::readdir(stream) };
        if item.is_null() {
            return Ok(Listing::End);
        }
        traversal_stop(count, start)?;
        let bytes = unsafe { CStr::from_ptr((*item).d_name.as_ptr()) }.to_bytes();
        if bytes == b"." || bytes == b".." || bytes.eq_ignore_ascii_case(b".git") {
            return Ok(Listing::Skip);
        }
        let path = prefix.join(std::ffi::OsStr::from_bytes(bytes));
        if safe_path(&path).is_err() {
            return Err(io::Error::other(
                "Unsupported Git path; comparison unavailable.",
            ));
        }
        Ok(Listing::Entry(
            CString::new(bytes).map_err(io::Error::other)?,
        ))
    }

    /// Open the child directory after descent approval; the recursion carries
    /// the shared budget/callback set forward.
    #[allow(clippy::too_many_arguments)]
    fn walk_directory(
        dir: &File,
        name: &CStr,
        path: &Path,
        visit: &mut impl FnMut(Entry) -> bool,
        descend: &mut impl FnMut(&Path, bool) -> bool,
        count: &mut usize,
        start: std::time::Instant,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        hash_bytes: &mut usize,
        prepare: &mut impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
        control_bytes: &mut usize,
    ) -> io::Result<bool> {
        if descend(path, true) {
            let child = open_dir(dir.as_raw_fd(), name)?;
            return recurse(
                &child,
                path,
                visit,
                descend,
                count,
                start,
                expected,
                hash_bytes,
                prepare,
                control_bytes,
            );
        }
        Ok(true)
    }

    /// Read and visit one non-directory leaf.
    #[allow(clippy::too_many_arguments)]
    fn visit_leaf(
        dir: &File,
        name: &CStr,
        path: &Path,
        stat: libc::stat,
        visit: &mut impl FnMut(Entry) -> bool,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        hash_bytes: &mut usize,
        start: std::time::Instant,
    ) -> io::Result<bool> {
        let (mode, content) = match stat.st_mode & libc::S_IFMT {
            libc::S_IFLNK => (0o120000, link_content(dir, name)),
            libc::S_IFREG => {
                let mode = if stat.st_mode & 0o111 != 0 {
                    0o100755
                } else {
                    0o100644
                };
                (
                    mode,
                    regular_content(dir, name, path, mode, expected, hash_bytes, start),
                )
            }
            _ => (0, Err("Special file; content unavailable.".into())),
        };
        Ok(visit(Entry {
            path: path.to_owned(),
            mode,
            content,
        }))
    }

    // The recursion threads one shared budget/callback set; a context struct
    // would wrap eight borrows behind one alias without reducing coupling.
    #[allow(clippy::too_many_arguments)]
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
        let attrs = read_control(dir, c".gitattributes", hash_bytes, start, control_bytes)?;
        let ignore = read_control(dir, c".gitignore", hash_bytes, start, control_bytes)?;
        prepare(prefix, attrs, ignore).map_err(io::Error::other)?;
        let stream = open_stream(dir)?;
        loop {
            let name = match next_entry(stream.0, prefix, count, start)? {
                Listing::End => break,
                Listing::Skip => continue,
                Listing::Entry(name) => name,
            };
            let path = prefix.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
            let Some(stat) = entry_stat(dir, &name) else {
                if !visit_unreadable(visit, path, "File changed or is unreadable; refresh.")? {
                    return Ok(false);
                }
                continue;
            };
            let kind = stat.st_mode & libc::S_IFMT;
            if kind == libc::S_IFDIR {
                if !walk_directory(
                    dir,
                    &name,
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
                continue;
            }
            if !descend(&path, false) {
                continue;
            }
            if !visit_leaf(dir, &name, &path, stat, visit, expected, hash_bytes, start)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Start at the filesystem root and open every canonical component relative
    /// to the held parent. No ancestor symlink can redirect the root open.
    pub(super) fn traverse(
        root: &Path,
        visit: &mut impl FnMut(Entry) -> bool,
        descend: &mut impl FnMut(&Path, bool) -> bool,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        prepare: &mut impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
    ) -> io::Result<()> {
        use std::os::unix::fs::OpenOptionsExt;
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
        recurse(
            &anchored,
            Path::new(""),
            visit,
            descend,
            &mut 0,
            std::time::Instant::now(),
            expected,
            &mut 0,
            prepare,
            &mut 0,
        )?;
        Ok(())
    }
}

#[cfg(unix)]
pub fn walk(
    root: &Path,
    mut visit: impl FnMut(Entry) -> bool,
    mut descend: impl FnMut(&Path, bool) -> bool,
    mut expected: impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
    mut prepare: impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
) -> io::Result<()> {
    unix_walk::traverse(root, &mut visit, &mut descend, &mut expected, &mut prepare)
}

#[cfg(windows)]
mod windows_walk {
    use super::super::git_changes_eol::Eol;
    use super::{bounded, safe_path};
    use super::{traversal_stop, Entry};
    use std::{
        fs::File,
        io,
        path::{Path, PathBuf},
    };

    // Omit FILE_SHARE_DELETE, pinning each ancestor against rename/replacement.
    fn pin(path: &Path) -> io::Result<File> {
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .custom_flags(0x00200000 | 0x02000000)
            .open(path)
    }

    /// Bounded read of one directory control file through its pinned handle.
    fn read_control(
        root: &Path,
        relative: &Path,
        name: &str,
        hash_bytes: &mut usize,
        start: std::time::Instant,
        control_bytes: &mut usize,
    ) -> io::Result<Option<Vec<u8>>> {
        let file = match pin(&root.join(relative).join(name)) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("Unsupported Git control file."));
        }
        let data = bounded(file, None, Eol::Raw, hash_bytes, start)
            .map_err(io::Error::other)?
            .unwrap_or_default();
        *control_bytes += data.len();
        if *control_bytes > 4 * 1024 * 1024 {
            return Err(io::Error::other("Git control-file budget exceeded."));
        }
        Ok(Some(data))
    }

    /// Reparse-point target text as link content, without dereferencing it.
    fn reparse_content(root: &Path, path: &Path) -> Result<Option<Vec<u8>>, String> {
        // The reparse point itself is pinned; read_link does not dereference its target.
        std::fs::read_link(root.join(path))
            .ok()
            .and_then(|p| p.to_str().map(|s| s.as_bytes().to_vec()))
            .ok_or("Unsupported reparse point.".into())
            .map(Some)
    }

    /// Capture content of one pinned regular leaf, honoring its expected blob.
    fn regular_content(
        held: File,
        path: &Path,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        hash_bytes: &mut usize,
        start: std::time::Instant,
    ) -> Result<Option<Vec<u8>>, String> {
        expected(path, 0o100644).and_then(|(id, eol)| bounded(held, id, eol, hash_bytes, start))
    }

    /// Visit an entry the traversal could not read, keeping the walk-alive rules.
    fn visit_unreadable(
        visit: &mut impl FnMut(Entry) -> bool,
        path: PathBuf,
        content: &str,
    ) -> bool {
        visit(Entry {
            path,
            mode: 0,
            content: Err(content.into()),
        })
    }

    // The recursion threads one shared budget/callback set; a context struct
    /// One read_dir outcome: a skipped name, or a validated relative path.
    /// Budget behavior: caller advances it before resolving the item.
    #[allow(clippy::too_many_arguments)]
    fn listing_path(
        item: std::io::Result<std::fs::DirEntry>,
        relative: &Path,
    ) -> io::Result<Option<PathBuf>> {
        let item = item?;
        let path = relative.join(item.file_name());
        if item
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(".git")
        {
            return Ok(None);
        }
        if safe_path(&path).is_err() {
            return Err(io::Error::other(
                "Unsupported Git path; comparison unavailable.",
            ));
        }
        Ok(Some(path))
    }

    /// Pin the entry, or visit it as unreadable and tell the walk to continue.
    fn pinned_entry(
        root: &Path,
        path: &Path,
        visit: &mut impl FnMut(Entry) -> bool,
    ) -> io::Result<Option<File>> {
        match pin(&root.join(path)) {
            Ok(f) => Ok(Some(f)),
            Err(_) => {
                Ok(visit_unreadable(visit, path.to_owned(), "Unreadable file.").then(|_| None))
            }
        }
    }

    /// Visit a reparse leaf as link content without dereferencing it.
    fn visit_reparse(root: &Path, path: &Path, visit: &mut impl FnMut(Entry) -> bool) -> bool {
        visit(Entry {
            path: path.to_owned(),
            mode: 0o120000,
            content: reparse_content(root, path),
        })
    }

    /// Recurse into an approved directory descent.
    #[allow(clippy::too_many_arguments)]
    fn walk_directory(
        root: &Path,
        path: &Path,
        visit: &mut impl FnMut(Entry) -> bool,
        descend: &mut impl FnMut(&Path, bool) -> bool,
        count: &mut usize,
        start: std::time::Instant,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        hash_bytes: &mut usize,
        prepare: &mut impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
        control_bytes: &mut usize,
    ) -> io::Result<bool> {
        if descend(path, true) {
            return recurse(
                root,
                path,
                visit,
                descend,
                count,
                start,
                expected,
                hash_bytes,
                prepare,
                control_bytes,
            );
        }
        Ok(true)
    }

    /// Read and visit one pinned non-directory leaf.
    #[allow(clippy::too_many_arguments)]
    fn visit_leaf(
        held: File,
        path: &Path,
        meta: &std::fs::Metadata,
        visit: &mut impl FnMut(Entry) -> bool,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        hash_bytes: &mut usize,
        start: std::time::Instant,
    ) -> io::Result<bool> {
        let content = if meta.is_file() {
            regular_content(held, path, expected, hash_bytes, start)
        } else {
            Err("Special file; content unavailable.".into())
        };
        Ok(visit(Entry {
            path: path.to_owned(),
            mode: 0o100644,
            content,
        }))
    }

    // The recursion threads one shared budget/callback set; a context struct
    // would wrap eight borrows behind one alias without reducing coupling.
    #[allow(clippy::too_many_arguments)]
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
        let attrs = read_control(
            root,
            relative,
            ".gitattributes",
            hash_bytes,
            start,
            control_bytes,
        )?;
        let ignore = read_control(
            root,
            relative,
            ".gitignore",
            hash_bytes,
            start,
            control_bytes,
        )?;
        prepare(relative, attrs, ignore).map_err(io::Error::other)?;
        for item in std::fs::read_dir(root.join(relative))? {
            traversal_stop(count, start)?;
            let Some(path) = listing_path(item, relative)? else {
                continue;
            };
            let Some(held) = pinned_entry(root, &path, visit)? else {
                continue;
            };
            let meta = held.metadata()?;
            if let Ok(false) = route_pinned_entry(
                root,
                &path,
                held,
                &meta,
                visit,
                descend,
                count,
                start,
                expected,
                hash_bytes,
                prepare,
                control_bytes,
            ) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Route one pinned entry after descent: reparse read, directory descent
    /// or leaf capture. Ok(false) marks the walk stopped by the visitor.
    #[allow(clippy::too_many_arguments)]
    fn route_pinned_entry(
        root: &Path,
        path: &Path,
        held: File,
        meta: &std::fs::Metadata,
        visit: &mut impl FnMut(Entry) -> bool,
        descend: &mut impl FnMut(&Path, bool) -> bool,
        count: &mut usize,
        start: std::time::Instant,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        hash_bytes: &mut usize,
        prepare: &mut impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
        control_bytes: &mut usize,
    ) -> io::Result<bool> {
        let reparse = meta.file_attributes() & 0x400 != 0;
        if !descend(path, meta.is_dir() && !reparse) {
            return Ok(true);
        }
        if reparse {
            return Ok(visit_reparse(root, path, visit));
        }
        if meta.is_dir() {
            return walk_directory(
                root,
                path,
                visit,
                descend,
                count,
                start,
                expected,
                hash_bytes,
                prepare,
                control_bytes,
            );
        }
        visit_leaf(held, path, meta, visit, expected, hash_bytes, start)
    }

    /// Keep every normal ancestor open without FILE_SHARE_DELETE, and reject
    /// reparse points before proceeding to the next component.
    fn pin_ancestors(root: &Path) -> io::Result<Vec<File>> {
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
        Ok(pinned)
    }

    pub(super) fn traverse(
        root: &Path,
        visit: &mut impl FnMut(Entry) -> bool,
        descend: &mut impl FnMut(&Path, bool) -> bool,
        expected: &mut impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
        prepare: &mut impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
    ) -> io::Result<()> {
        let _pinned = pin_ancestors(root)?;
        recurse(
            root,
            Path::new(""),
            visit,
            descend,
            &mut 0,
            std::time::Instant::now(),
            expected,
            &mut 0,
            prepare,
            &mut 0,
        )?;
        Ok(())
    }
}

#[cfg(windows)]
pub fn walk(
    root: &Path,
    mut visit: impl FnMut(Entry) -> bool,
    mut descend: impl FnMut(&Path, bool) -> bool,
    mut expected: impl FnMut(&Path, i32) -> Result<(Option<git2::Oid>, Eol), String>,
    mut prepare: impl FnMut(&Path, Option<Vec<u8>>, Option<Vec<u8>>) -> Result<(), String>,
) -> io::Result<()> {
    windows_walk::traverse(root, &mut visit, &mut descend, &mut expected, &mut prepare)
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
        let (read, digest, stats) = raw_blob_digest(&mut f, size, hash_bytes, start)?;
        if read == size
            && digest.as_slice() == expected.as_bytes()
            && (!eol.converts(&stats) || stats.crlf == 0 || eol == Eol::AutoIndexSame)
        {
            return Ok(None);
        }
        if eol.converts(&stats)
            && stats.crlf > 0
            && normalized_blob_matches(&mut f, size, stats.crlf, expected, hash_bytes, start)?
        {
            return Ok(None);
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

/// Streaming raw SHA-1 over an oversized file, tracking line statistics.
fn raw_blob_digest(
    f: &mut File,
    size: u64,
    hash_bytes: &mut usize,
    start: std::time::Instant,
) -> Result<(u64, [u8; 20], Stats), String> {
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
    Ok((read, hash.finalize().into(), stats))
}

/// One chunk of CRLF-normalized re-hash: appends the cleaned bytes and
/// advances the carried carriage-return state.
fn crlf_normalized_chunk(buf: &[u8], pending_cr: &mut bool, clean: &mut Vec<u8>) {
    for &b in buf {
        if *pending_cr {
            if b != b'\n' {
                clean.push(b'\r')
            }
            *pending_cr = false;
        }
        if b == b'\r' {
            *pending_cr = true
        } else {
            clean.push(b)
        }
    }
}

/// Streaming normalized SHA-1 over an oversized file, deciding whether it still
/// equals the expected blob id once CRLF normalization applies.
fn normalized_blob_matches(
    f: &mut File,
    size: u64,
    crlf: u64,
    expected: git2::Oid,
    hash_bytes: &mut usize,
    start: std::time::Instant,
) -> Result<bool, String> {
    use sha1::{Digest, Sha1};
    use std::io::{Seek, SeekFrom};
    if (*hash_bytes as u64) + size > 64 * 1024 * 1024 {
        return Err("Large-file equality budget exceeded; content unavailable.".into());
    }
    f.seek(SeekFrom::Start(0))
        .map_err(|_| "File changed; refresh.")?;
    let normalized_size = size - crlf;
    let mut hash = Sha1::new();
    hash.update(format!("blob {normalized_size}\0").as_bytes());
    let mut pending_cr = false;
    let mut output = 0u64;
    let mut raw = 0u64;
    let mut buf = [0u8; 65536];
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
        crlf_normalized_chunk(&buf[..n], &mut pending_cr, &mut clean);
        output += clean.len() as u64;
        hash.update(&clean);
    }
    if pending_cr {
        hash.update(b"\r");
        output += 1;
    }
    Ok(raw == size
        && output == normalized_size
        && hash.finalize().as_slice() == expected.as_bytes())
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
