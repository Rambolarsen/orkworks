//! Native durable publication primitives shared by the archive and metadata writers.
#[cfg(not(windows))]
use std::fs;
use std::{io, path::Path};

pub(crate) fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(path)?.sync_all()
    }
    #[cfg(windows)]
    {
        // The write-through move in replace provides the Windows
        // publication barrier; directory handles do not use Unix fsync.
        let _ = path;
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Durable session state is unsupported on this platform",
        ))
    }
}

pub(crate) fn replace(source: &Path, target: &Path) -> io::Result<()> {
    #[cfg(not(windows))]
    {
        fs::rename(source, target)
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let source: Vec<u16> = source
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let target: Vec<u16> = target
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        if source[..source.len() - 1].contains(&0) || target[..target.len() - 1].contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid session state path",
            ));
        }
        // SAFETY: both paths are validated, nul-terminated UTF-16 buffers,
        // retained for the complete API call. Same-directory staging avoids a
        // cross-volume copy. WRITE_THROUGH waits for publication on disk.
        let result = unsafe {
            MoveFileExW(
                source.as_ptr(),
                target.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
