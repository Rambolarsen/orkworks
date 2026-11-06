//! Component observations only: no production pair, source, schema or admission.
//! The sole ignored entry is run only by the approved manual workflow.
#![cfg(windows)]

use rusqlite::{ffi, Connection, OpenFlags};
use std::{
    ffi::{c_void, CStr},
    fs::{self, File},
    io::{self, Write},
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        fs::MetadataExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::Path,
    ptr,
};
use tempfile::TempDir;
use windows_sys::Win32::{
    Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{
        CreateFileW, FileIdInfo, GetDriveTypeW, GetFileInformationByHandleEx,
        GetVolumeInformationW, GetVolumePathNameW, CREATE_NEW, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_SHARE_READ, FILE_SHARE_WRITE,
        OPEN_EXISTING,
    },
};

type Result<T> = std::result::Result<T, Failure>;
#[derive(Debug)]
struct Failure {
    phase: &'static str,
    code: i64,
}

// Neither native identity nor errors containing paths implement Debug/Display here.
#[derive(PartialEq, Eq)]
struct Identity {
    volume: u64,
    id: [u8; 16],
}

fn os_error(phase: &'static str, error: io::Error) -> Failure {
    Failure {
        phase,
        code: error.raw_os_error().unwrap_or(-1) as i64,
    }
}
fn sql_error(phase: &'static str, error: rusqlite::Error) -> Failure {
    Failure {
        phase,
        code: error.sqlite_error().map_or(-1, |e| e.extended_code) as i64,
    }
}
fn record(value: serde_json::Value) -> Result<()> {
    let mut bytes = serde_json::to_vec(&value).map_err(|_| Failure {
        phase: "report",
        code: -1,
    })?;
    // libtest may have written a progress prefix without a trailing newline.
    bytes.insert(0, b'\n');
    bytes.push(b'\n');
    if bytes.len() > 2048 {
        return Err(Failure {
            phase: "report_bound",
            code: -1,
        });
    }
    // Direct Write bypasses libtest's println capture; only fixed tags/codes/booleans.
    io::stdout()
        .lock()
        .write_all(&bytes)
        .map_err(|e| os_error("report", e))
}
fn check(case: &'static str, observation: &'static str, observed: bool, code: i64) -> Result<()> {
    record(
        serde_json::json!({"case":case,"observation":observation,"expected":true,"observed":observed,"code":code}),
    )?;
    if observed {
        Ok(())
    } else {
        Err(Failure { phase: case, code })
    }
}
fn complete(case: &'static str) -> Result<()> {
    record(serde_json::json!({"case":case,"complete":true}))
}
fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
fn root() -> Result<TempDir> {
    let root = tempfile::Builder::new()
        .prefix("ork-pair-component-")
        .tempdir()
        .map_err(|e| os_error("temp_root", e))?;
    require_ntfs(root.path())?;
    Ok(root)
}
fn require_ntfs(path: &Path) -> Result<()> {
    let path = wide(path);
    let mut volume = [0u16; 32768];
    let mut filesystem = [0u16; 32];
    // SAFETY: NUL-terminated input and writable arrays of the specified lengths.
    if unsafe { GetVolumePathNameW(path.as_ptr(), volume.as_mut_ptr(), volume.len() as u32) } == 0 {
        return Err(os_error("volume_path", io::Error::last_os_error()));
    }
    // SAFETY: volume is a NUL-terminated root returned by the successful API call.
    let local = unsafe { GetDriveTypeW(volume.as_ptr()) } == 3; // DRIVE_FIXED
                                                                // SAFETY: optional outputs are null; filesystem is a valid writable array.
    if unsafe {
        GetVolumeInformationW(
            volume.as_ptr(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    } == 0
    {
        return Err(os_error("volume_info", io::Error::last_os_error()));
    }
    let end = filesystem
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(filesystem.len());
    if !local || filesystem[..end] != [78, 84, 70, 83] {
        // NTFS
        return Err(Failure {
            phase: "local_ntfs_required",
            code: -1,
        });
    }
    Ok(())
}
fn open_guard(path: &Path, directory: bool) -> Result<File> {
    let name = wide(path);
    let flags = FILE_FLAG_OPEN_REPARSE_POINT
        | if directory {
            FILE_FLAG_BACKUP_SEMANTICS
        } else {
            0
        };
    // SAFETY: valid NUL-terminated name; null SECURITY_ATTRIBUTES makes the owned
    // handle non-inheritable. Read/write sharing deliberately omits delete sharing.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | if directory { 0 } else { GENERIC_WRITE },
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            ptr::null(),
            if directory { OPEN_EXISTING } else { CREATE_NEW },
            flags,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(os_error("create_guard", io::Error::last_os_error()));
    }
    // SAFETY: successful CreateFileW transferred one owned handle, closed by File.
    let file = unsafe { File::from_raw_handle(handle) };
    let metadata = file.metadata().map_err(|e| os_error("guard_metadata", e))?;
    if metadata.file_attributes() & 0x400 != 0
        || metadata.is_dir() != directory
        || (!directory && !metadata.is_file())
    {
        return Err(Failure {
            phase: "regular_nonreparse_required",
            code: -1,
        });
    }
    Ok(file)
}
fn identity(handle: *mut c_void) -> Result<Identity> {
    let mut info = FILE_ID_INFO::default();
    // SAFETY: the caller retains the File or exclusive SQLite connection/schema;
    // the borrowed handle is used only for this query and is never closed here.
    if unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            &mut info as *mut _ as *mut c_void,
            size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
    {
        return Err(os_error("file_identity", io::Error::last_os_error()));
    }
    Ok(Identity {
        volume: info.VolumeSerialNumber,
        id: info.FileId.Identifier,
    })
}
fn actual_identity(connection: &Connection, schema: &CStr) -> Result<Identity> {
    let mut handle = INVALID_HANDLE_VALUE;
    // SAFETY: exclusive synchronous use of a live connection, valid schema CStr,
    // documented opcode and initialized HANDLE output. Never close its result.
    let code = unsafe {
        ffi::sqlite3_file_control(
            connection.handle(),
            schema.as_ptr(),
            ffi::SQLITE_FCNTL_WIN32_GET_HANDLE,
            &mut handle as *mut _ as *mut c_void,
        )
    };
    if code != ffi::SQLITE_OK || handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(Failure {
            phase: "schema_handle",
            code: code as i64,
        });
    }
    identity(handle)
}
fn require_vfs(connection: &Connection, schema: &CStr) -> Result<()> {
    let mut name: *mut std::ffi::c_char = ptr::null_mut();
    // SAFETY: documented VFSNAME takes char**; successful result is SQLite allocated.
    let code = unsafe {
        ffi::sqlite3_file_control(
            connection.handle(),
            schema.as_ptr(),
            ffi::SQLITE_FCNTL_VFSNAME,
            &mut name as *mut _ as *mut c_void,
        )
    };
    if code != ffi::SQLITE_OK || name.is_null() {
        return Err(Failure {
            phase: "vfs_query",
            code: code as i64,
        });
    }
    // SAFETY: successful VFSNAME provides a NUL-terminated SQLite allocation.
    let expected = unsafe { CStr::from_ptr(name) }.to_bytes() == b"win32";
    // SAFETY: release only the allocation returned by VFSNAME, never a file handle.
    unsafe { ffi::sqlite3_free(name.cast()) };
    if !expected {
        return Err(Failure {
            phase: "win32_required",
            code: -1,
        });
    }
    Ok(())
}
fn existing_uri(path: &Path) -> Result<String> {
    let text = path
        .to_str()
        .ok_or(Failure {
            phase: "synthetic_path_encoding",
            code: -1,
        })?
        .replace('\\', "/");
    // TempDir supplies a local absolute drive path; no caller-controlled URI input.
    if text.as_bytes().get(1) != Some(&b':') {
        return Err(Failure {
            phase: "local_drive_path",
            code: -1,
        });
    }
    let mut uri = String::from("file:///");
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"/:.-_~".contains(&byte) {
            uri.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    uri.push_str("?mode=rw&vfs=win32");
    Ok(uri)
}
fn connection(path: &Path) -> Result<Connection> {
    Connection::open_with_flags_and_vfs(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        "win32",
    )
    .map_err(|e| sql_error("existing_open", e))
}
fn original_and_sharing() -> Result<()> {
    let root = root()?;
    let main_path = root.path().join("main.sqlite");
    let hierarchy_path = root.path().join("hierarchy.sqlite");
    let main = open_guard(&main_path, false)?;
    let hierarchy = open_guard(&hierarchy_path, false)?;
    let main_id = identity(main.as_raw_handle())?;
    let hierarchy_id = identity(hierarchy.as_raw_handle())?;
    let connection = connection(&main_path)?;
    connection
        .execute(
            "ATTACH DATABASE ?1 AS hierarchy",
            [existing_uri(&hierarchy_path)?],
        )
        .map_err(|e| sql_error("existing_attach", e))?;
    let main_schema = c"main";
    let hierarchy_schema = c"hierarchy";
    require_vfs(&connection, main_schema)?;
    require_vfs(&connection, hierarchy_schema)?;
    record(serde_json::json!({"filesystem":"NTFS","vfs":"win32"}))?;
    let actual_main = actual_identity(&connection, main_schema)?;
    let actual_hierarchy = actual_identity(&connection, hierarchy_schema)?;
    check("original_roles", "main_original", actual_main == main_id, 0)?;
    check(
        "original_roles",
        "hierarchy_original",
        actual_hierarchy == hierarchy_id,
        0,
    )?;
    check(
        "original_roles",
        "distinct",
        actual_main != actual_hierarchy,
        0,
    )?;
    connection
        .execute_batch(
            "PRAGMA temp_store=MEMORY; PRAGMA main.page_size=4096;
        PRAGMA hierarchy.page_size=4096; PRAGMA main.max_page_count=16;
        PRAGMA hierarchy.max_page_count=16; BEGIN;
        CREATE TABLE main.probe(value INTEGER); CREATE TABLE hierarchy.probe(value INTEGER);
        INSERT INTO main.probe VALUES (7); INSERT INTO hierarchy.probe VALUES (11); COMMIT;",
        )
        .map_err(|e| sql_error("toy_commit", e))?;
    let rows: (i64, i64) = connection
        .query_row(
            "SELECT (SELECT value FROM main.probe), (SELECT value FROM hierarchy.probe)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| sql_error("toy_rows", e))?;
    check("original_roles", "toy_rows", rows == (7, 11), 0)?;
    check(
        "original_roles",
        "main_after_commit",
        actual_identity(&connection, main_schema)? == main_id,
        0,
    )?;
    check(
        "original_roles",
        "hierarchy_after_commit",
        actual_identity(&connection, hierarchy_schema)? == hierarchy_id,
        0,
    )?;
    complete("original_roles")?;

    let foreign = open_guard(&root.path().join("foreign.sqlite"), false)?;
    let foreign_id = identity(foreign.as_raw_handle())?;
    check(
        "swapped_foreign",
        "swapped_main_denied",
        actual_main != hierarchy_id,
        0,
    )?;
    check(
        "swapped_foreign",
        "swapped_hierarchy_denied",
        actual_hierarchy != main_id,
        0,
    )?;
    check(
        "swapped_foreign",
        "foreign_main_denied",
        actual_main != foreign_id,
        0,
    )?;
    check(
        "swapped_foreign",
        "foreign_hierarchy_denied",
        actual_hierarchy != foreign_id,
        0,
    )?;
    complete("swapped_foreign")?;

    let renamed = root.path().join("renamed.sqlite");
    let rename = fs::rename(&main_path, &renamed);
    check(
        "file_sharing",
        "held_rename_denied",
        rename.is_err(),
        rename.err().and_then(|e| e.raw_os_error()).unwrap_or(0) as i64,
    )?;
    let delete = fs::remove_file(&main_path);
    check(
        "file_sharing",
        "held_delete_denied",
        delete.is_err(),
        delete.err().and_then(|e| e.raw_os_error()).unwrap_or(0) as i64,
    )?;
    check(
        "file_sharing",
        "identity_retained",
        identity(main.as_raw_handle())? == main_id
            && actual_identity(&connection, main_schema)? == main_id,
        0,
    )?;
    connection
        .close()
        .map_err(|(_, e)| sql_error("close_original", e))?;
    drop((main, hierarchy, foreign));
    fs::rename(&main_path, &renamed).map_err(|e| os_error("released_file_rename", e))?;
    check("file_sharing", "released_rename_succeeded", true, 0)?;
    complete("file_sharing")?;
    root.close().map_err(|e| os_error("fixture_cleanup", e))
}
fn directory_sharing() -> Result<()> {
    let root = root()?;
    let leaf = root.path().join("leaf");
    fs::create_dir(&leaf).map_err(|e| os_error("leaf_create", e))?;
    let guard = open_guard(&leaf, true)?;
    let renamed = root.path().join("renamed-leaf");
    let rename = fs::rename(&leaf, &renamed);
    check(
        "directory_sharing",
        "held_rename_denied",
        rename.is_err(),
        rename.err().and_then(|e| e.raw_os_error()).unwrap_or(0) as i64,
    )?;
    drop(guard);
    fs::rename(&leaf, &renamed).map_err(|e| os_error("released_directory_rename", e))?;
    check("directory_sharing", "released_rename_succeeded", true, 0)?;
    complete("directory_sharing")?;
    root.close().map_err(|e| os_error("fixture_cleanup", e))
}
fn query_failure() -> Result<()> {
    let root = root()?;
    let path = root.path().join("main.sqlite");
    let guard = open_guard(&path, false)?;
    let connection = connection(&path)?;
    let mut output = INVALID_HANDLE_VALUE;
    // SAFETY: live exclusive connection and initialized output. A non-OK result
    // is inspected first; its output is never consumed, queried or closed.
    let code = unsafe {
        ffi::sqlite3_file_control(
            connection.handle(),
            c"unattached".as_ptr(),
            ffi::SQLITE_FCNTL_WIN32_GET_HANDLE,
            &mut output as *mut _ as *mut c_void,
        )
    };
    check(
        "query_failure",
        "unknown_schema_denied",
        code != ffi::SQLITE_OK,
        code as i64,
    )?;
    connection
        .close()
        .map_err(|(_, e)| sql_error("close_query", e))?;
    drop(guard);
    complete("query_failure")?;
    root.close().map_err(|e| os_error("fixture_cleanup", e))
}
fn partial_creation() -> Result<()> {
    let root = root()?;
    let first = open_guard(&root.path().join("main.sqlite"), false)?;
    let original = identity(first.as_raw_handle())?;
    let second_path = root.path().join("hierarchy.sqlite");
    let collision = open_guard(&second_path, false)?;
    let second = open_guard(&second_path, false);
    let code = second.as_ref().err().map_or(0, |e| e.code);
    check(
        "partial_creation",
        "second_create_denied",
        second.is_err(),
        code,
    )?;
    check(
        "partial_creation",
        "first_retained",
        identity(first.as_raw_handle())? == original,
        0,
    )?;
    drop((second, first, collision));
    complete("partial_creation")?;
    root.close().map_err(|e| os_error("fixture_cleanup", e))
}

#[test]
#[ignore = "one approved manual Windows component attempt only"]
fn qualify_windows_pair_handles() -> Result<()> {
    let result = (|| {
        // Version mismatch is unavailable, before creating any SQL fixtures.
        if rusqlite::version() != "3.46.0" {
            return Err(Failure {
                phase: "sqlite_version",
                code: ffi::SQLITE_MISMATCH as i64,
            });
        }
        record(serde_json::json!({"runtime":"sqlite","version":"3.46.0"}))?;
        original_and_sharing()?;
        directory_sharing()?;
        query_failure()?;
        partial_creation()
    })();
    if let Err(ref failure) = result {
        let _ = record(serde_json::json!({"failure":failure.phase,"code":failure.code}));
    }
    result
}
