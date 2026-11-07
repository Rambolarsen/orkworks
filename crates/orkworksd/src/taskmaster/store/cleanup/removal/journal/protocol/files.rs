use super::*;
pub(super) fn basename(text: &str) -> io::Result<CString> {
    if text.is_empty() || text == "." || text == ".." || text.contains('/') {
        return Err(invalid("invalid journal basename"));
    }
    CString::new(text).map_err(|_| invalid("invalid journal basename"))
}
pub(super) fn check(parent: &fs::File, name: &CStr, file: &fs::File) -> io::Result<()> {
    if entry(parent, name)? != Some(identity(file)?) {
        return Err(invalid("journal entry identity changed"));
    }
    Ok(())
}
pub(super) struct Leaf {
    pub(super) name: CString,
    pub(super) bytes: Vec<u8>,
    pub(super) file: fs::File,
}
impl Leaf {
    pub(super) fn verify(&self, parent: &fs::File) -> io::Result<()> {
        let current = read(parent, &self.name)?;
        if identity(&current.file)? != identity(&self.file)? || current.bytes != self.bytes {
            return Err(invalid("journal leaf changed"));
        }
        Ok(())
    }
}
pub(super) fn read(parent: &fs::File, name: &CStr) -> io::Result<Leaf> {
    let file = open(parent, name, false)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > LIMIT as u64 {
        return Err(invalid("journal leaf is aliased, nonregular or oversized"));
    }
    let mut bytes = Vec::new();
    (&file).take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        return Err(invalid("journal leaf exceeds byte bound"));
    }
    check(parent, name, &file)?;
    Ok(Leaf {
        name: name.to_owned(),
        bytes,
        file,
    })
}
pub(super) fn optional_read(parent: &fs::File, name: &CStr) -> io::Result<Option<Leaf>> {
    match read(parent, name) {
        Ok(leaf) => Ok(Some(leaf)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
pub(super) fn mkdir(parent: &fs::File, name: &CStr) -> io::Result<fs::File> {
    // SAFETY: live parent descriptor and a validated single basename; private mode.
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } < 0 {
        return Err(io::Error::last_os_error());
    }
    checkpoint()?;
    let file = open(parent, name, true)?;
    check(parent, name, &file)?;
    Ok(file)
}
pub(super) fn write(parent: &fs::File, name: &CStr, bytes: &[u8]) -> io::Result<Leaf> {
    // SAFETY: live parent and validated basename; O_EXCL prevents overwriting an entry.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY
                | libc::O_CREAT
                | libc::O_EXCL
                | libc::O_NOFOLLOW
                | libc::O_NONBLOCK
                | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful openat returned a new uniquely owned descriptor.
    let mut file = unsafe { fs::File::from_raw_fd(fd) };
    checkpoint()?;
    file.write_all(bytes)?;
    checkpoint()?;
    sync(&file)?;
    check(parent, name, &file)?;
    let leaf = read(parent, name)?;
    if identity(&leaf.file)? != identity(&file)? || leaf.bytes != bytes {
        return Err(invalid("journal write changed"));
    }
    Ok(leaf)
}
pub(super) fn remove(
    parent: &fs::File,
    name: &CStr,
    file: &fs::File,
    directory: bool,
) -> io::Result<()> {
    check(parent, name, file)?;
    // SAFETY: live descriptors, validated basename; directories must be empty.
    if unsafe {
        libc::unlinkat(
            parent.as_raw_fd(),
            name.as_ptr(),
            if directory { libc::AT_REMOVEDIR } else { 0 },
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    checkpoint()
}
pub(super) fn sync(file: &fs::File) -> io::Result<()> {
    file.sync_all()?;
    checkpoint()
}
pub(super) fn checkpoint() -> io::Result<()> {
    #[cfg(test)]
    crate::taskmaster::store::cleanup::JOURNAL_FAILURE.with(|failure| match failure.get() {
        Some(0) => {
            failure.set(None);
            Err(io::Error::other("injected journal boundary failure"))
        }
        Some(n) => {
            failure.set(Some(n - 1));
            Ok(())
        }
        None => Ok(()),
    })?;
    Ok(())
}
