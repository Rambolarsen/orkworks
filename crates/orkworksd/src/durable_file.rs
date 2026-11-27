//! Native durable publication primitives shared by the archive and metadata writers.
use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

pub(crate) fn open_metadata_file(path: &Path) -> io::Result<File> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
        };

        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Metadata entry is not an ordinary file",
            ));
        }
        Ok(file)
    }
    #[cfg(not(windows))]
    {
        File::open(path)
    }
}

pub(crate) fn read_metadata_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    open_metadata_file(path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub(crate) fn read_metadata_string(path: &Path) -> io::Result<String> {
    let mut contents = String::new();
    open_metadata_file(path)?.read_to_string(&mut contents)?;
    Ok(contents)
}

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
