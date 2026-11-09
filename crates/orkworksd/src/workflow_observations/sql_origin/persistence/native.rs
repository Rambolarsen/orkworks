use super::*;
use std::{
    ffi::{CStr, CString},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
};
pub(super) fn stamp(file: &File) -> Result<reader::Stamp> {
    let m = file.metadata()?;
    if !m.is_file() || m.nlink() != 1 {
        return Err(StoreError::Corrupt);
    }
    Ok((
        m.dev(),
        m.ino(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    ))
}
fn open(directory: &File, name: &CStr, flags: i32) -> Result<File> {
    // SAFETY: retained directory and single NUL-terminated component; fresh owned FD.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        return Err(StoreError::Io(std::io::Error::last_os_error()));
    }
    // SAFETY: successful openat returned a descriptor owned exactly once.
    Ok(unsafe { File::from_raw_fd(fd) })
}
pub(super) fn component(directory: &File, id: &str) -> Result<CString> {
    if !valid_id(id) {
        return Err(StoreError::Invalid("invalid observation identity".into()));
    }
    let name = CString::new(format!("{id}.ndjson")).map_err(|_| StoreError::Corrupt)?;
    // SAFETY: retained live directory and fixed platform query selector.
    let limit = unsafe { libc::fpathconf(directory.as_raw_fd(), libc::_PC_NAME_MAX) };
    let limit = usize::try_from(limit)
        .ok()
        .filter(|n| *n > 0)
        .ok_or(StoreError::Unsupported)?;
    if name.as_bytes().len() > limit {
        return Err(StoreError::TooLarge);
    }
    Ok(name)
}
pub(super) struct Pin {
    name: CString,
    file: Option<File>,
    pub(super) original: Option<reader::Stamp>,
}
impl Pin {
    pub(super) fn capture(directory: &File, name: &CStr) -> Result<Self> {
        let file = match open(directory, name, libc::O_RDONLY) {
            Ok(file) => Some(file),
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        let original = file.as_ref().map(stamp).transpose()?;
        Ok(Self {
            name: name.to_owned(),
            file,
            original,
        })
    }
    pub(super) fn verify(&self, directory: &File) -> Result<()> {
        if self.file.as_ref().map(stamp).transpose()? != self.original {
            return Err(StoreError::Conflict);
        }
        let current = Self::capture(directory, &self.name)?;
        if current.original != self.original {
            return Err(StoreError::Conflict);
        }
        Ok(())
    }
    pub(super) fn read(
        &mut self,
        directory: &File,
        index: &Index,
        budget: &mut ReadBudget,
        deadline: Option<std::time::Instant>,
    ) -> Result<Vec<u8>> {
        if let Some(deadline) = deadline {
            check_deadline(deadline)?;
        }
        budget.charge(index.len as usize)?;
        self.verify(directory)?;
        let file = self.file.as_mut().ok_or(StoreError::Conflict)?;
        file.seek(SeekFrom::Start(index.offset))?;
        let mut bytes = vec![0; index.len as usize];
        file.read_exact(&mut bytes)?;
        self.verify(directory)?;
        if let Some(deadline) = deadline {
            check_deadline(deadline)?;
        }
        Ok(bytes)
    }
}
struct Temporary<'a> {
    directory: &'a File,
    name: CString,
    file: File,
    live: bool,
}
impl<'a> Temporary<'a> {
    fn create(directory: &'a File) -> Result<Self> {
        let name = CString::new(format!(".observation-{}.tmp", uuid::Uuid::new_v4()))
            .map_err(|_| StoreError::Corrupt)?;
        let file = open(
            directory,
            &name,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
        )?;
        stamp(&file)?;
        Ok(Self {
            directory,
            name,
            file,
            live: true,
        })
    }
    fn publish(mut self, target: &Pin) -> Result<Pin> {
        self.file.sync_all()?;
        target.verify(self.directory)?;
        // SAFETY: both names are single components relative to the retained directory.
        if unsafe {
            libc::renameat(
                self.directory.as_raw_fd(),
                self.name.as_ptr(),
                self.directory.as_raw_fd(),
                target.name.as_ptr(),
            )
        } < 0
        {
            return Err(StoreError::Io(std::io::Error::last_os_error()));
        }
        self.live = false;
        self.directory.sync_all()?;
        let pin = Pin {
            name: target.name.clone(),
            original: Some(stamp(&self.file)?),
            file: Some(self.file.try_clone()?),
        };
        pin.verify(self.directory)?;
        Ok(pin)
    }
}
impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        if self.live {
            // Remove only our retained inode, never a replacement at the temporary name.
            if let (Ok(current), Ok(own)) = (
                open(self.directory, &self.name, libc::O_RDONLY),
                stamp(&self.file),
            ) {
                if stamp(&current).is_ok_and(|s| s.0 == own.0 && s.1 == own.1) {
                    // SAFETY: own single basename and live original directory FD.
                    unsafe {
                        libc::unlinkat(self.directory.as_raw_fd(), self.name.as_ptr(), 0);
                    }
                }
            }
        }
    }
}
pub(super) fn counter(directory: &File, pin: &Pin, value: u64) -> Result<Pin> {
    let mut temp = Temporary::create(directory)?;
    temp.file.write_all(value.to_string().as_bytes())?;
    temp.publish(pin)
}
pub(super) fn append(directory: &File, pin: &Pin, line: &[u8]) -> Result<()> {
    pin.verify(directory)?;
    let flags = libc::O_WRONLY
        | libc::O_APPEND
        | if pin.original.is_none() {
            libc::O_CREAT | libc::O_EXCL
        } else {
            0
        };
    let mut file = open(directory, &pin.name, flags)?;
    let before = stamp(&file)?;
    if pin.original.is_some_and(|original| original != before) {
        return Err(StoreError::Conflict);
    }
    #[cfg(test)]
    if take_fault(Fault::PartialAppend) {
        file.write_all(&line[..line.len() / 2])?;
        return Err(std::io::Error::other("injected partial append").into());
    }
    file.write_all(line)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    let after = stamp(&file)?;
    if after.2 != before.2 + line.len() as u64 + 1 {
        return Err(StoreError::Conflict);
    }
    let written = Pin {
        name: pin.name.clone(),
        file: Some(file),
        original: Some(after),
    };
    written.verify(directory)?;
    directory.sync_all()?;
    Ok(())
}
pub(super) fn rewrite(
    directory: &File,
    pin: &mut Pin,
    indices: &[Index],
    line: &[u8],
    session: &str,
    now: DateTime<Utc>,
    budget: &mut ReadBudget,
) -> Result<()> {
    let mut temp = Temporary::create(directory)?;
    for tombstones in [true, false] {
        for index in indices {
            if index.mode == Mode::Drop || (index.mode == Mode::Tombstone) != tombstones {
                continue;
            }
            let mut bytes = pin.read(directory, index, budget, None)?;
            if index.evicted {
                let record = decode_line(&bytes, session)?;
                bytes = mutation::evicted_retry(session, &record, now)?
                    .ok_or(StoreError::Corrupt)?
                    .encode()?;
            }
            temp.file.write_all(&bytes)?;
            temp.file.write_all(b"\n")?;
        }
    }
    temp.file.write_all(line)?;
    temp.file.write_all(b"\n")?;
    #[cfg(test)]
    fail(Fault::BeforeRename)?;
    temp.publish(pin)?;
    Ok(())
}
#[cfg(test)]
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Fault {
    AfterCounter,
    PartialAppend,
    BeforeRename,
}
#[cfg(test)]
thread_local! {pub(super) static FAIL:std::cell::Cell<Option<Fault>>=const{std::cell::Cell::new(None)};}
#[cfg(test)]
fn take_fault(stage: Fault) -> bool {
    FAIL.with(|slot| {
        if slot.get() == Some(stage) {
            slot.set(None);
            true
        } else {
            false
        }
    })
}
#[cfg(test)]
pub(super) fn fail(stage: Fault) -> Result<()> {
    if take_fault(stage) {
        Err(std::io::Error::other("injected observation failure").into())
    } else {
        Ok(())
    }
}
