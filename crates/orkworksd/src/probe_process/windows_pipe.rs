//! Overlapped capture readers: dropping a pending read never retains a blocking worker.
use std::{
    io,
    os::windows::{ffi::OsStrExt, io::FromRawHandle},
    process::Stdio,
};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::{
    Foundation::{GENERIC_WRITE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING},
};

pub(super) fn capture_pipe() -> io::Result<(NamedPipeServer, Stdio)> {
    let name = format!(r"\\.\pipe\orkworks-probe-{}", uuid::Uuid::new_v4());
    let server = ServerOptions::new()
        .access_inbound(true)
        .access_outbound(false)
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .create(&name)?;
    let wide: Vec<u16> = std::ffi::OsStr::new(&name)
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: name is NUL terminated; no security attributes makes the writer
    // non-inheritable. std Command duplicates only configured child stdio.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: CreateFileW returned one owned writer handle. The child writes
    // synchronously, while the server's reads use Tokio's overlapped I/O driver.
    let writer = unsafe { std::fs::File::from_raw_handle(handle) };
    Ok((server, Stdio::from(writer)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    #[test]
    fn pending_capture_with_external_writer_does_not_retain_runtime_worker() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let writer = runtime.block_on(async {
            let (mut reader, writer) = capture_pipe().unwrap();
            reader.connect().await.unwrap();
            let mut byte = [0];
            assert!(tokio::time::timeout(
                std::time::Duration::from_millis(20),
                reader.read(&mut byte)
            )
            .await
            .is_err());
            drop(reader);
            writer
        });
        // A synchronous ReadFile worker would prevent ordinary Runtime::drop
        // while this independently held writer keeps the pipe open.
        let started = std::time::Instant::now();
        drop(runtime);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        drop(writer);
    }
}
