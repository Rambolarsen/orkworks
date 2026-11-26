//! NTFS operations anchored to retained, non-inheritable handles.
use std::{
    ffi::{c_void, OsStr, OsString},
    fs::{File, OpenOptions},
    io,
    mem::{offset_of, size_of, size_of_val},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Component, Path, Prefix},
    ptr::{null, null_mut, NonNull},
    slice,
};
use windows_sys::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FileRenameInformation, NtCreateFile, NtSetInformationFile, FILE_CREATE,
            FILE_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT, FILE_RENAME_INFORMATION,
            FILE_SYNCHRONOUS_IO_NONALERT, FILE_WRITE_THROUGH,
        },
    },
    Win32::{
        Foundation::{
            LocalFree, RtlNtStatusToDosError, ERROR_NO_MORE_FILES, HANDLE, OBJ_CASE_INSENSITIVE,
            UNICODE_STRING, WIN32_ERROR,
        },
        Security::{
            AclSizeInformation, EqualSid, GetAce, GetAclInformation, GetLengthSid,
            GetSecurityDescriptorControl, GetSecurityDescriptorLength, GetSecurityDescriptorOwner,
            GetTokenInformation, TokenUser, ACCESS_ALLOWED_ACE, ACL, ACL_SIZE_INFORMATION,
            DACL_SECURITY_INFORMATION, OBJECT_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR, PSID, SE_DACL_PROTECTED, TOKEN_QUERY, TOKEN_USER,
        },
        Storage::FileSystem::{
            FileDispositionInfoEx, FileIdBothDirectoryInfo, FileIdBothDirectoryRestartInfo,
            GetDriveTypeW, GetFileInformationByHandle, GetFileInformationByHandleEx,
            GetFinalPathNameByHandleW, GetVolumeInformationByHandleW, SetFileInformationByHandle,
            BY_HANDLE_FILE_INFORMATION, FILE_ALL_ACCESS, FILE_ATTRIBUTE_DEVICE,
            FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT,
            FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
            FILE_DISPOSITION_INFO_EX, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_ID_BOTH_DIR_INFO, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
            FILE_SHARE_WRITE, VOLUME_NAME_GUID,
        },
        System::{
            SystemServices::{ACCESS_ALLOWED_ACE_TYPE, FILE_PERSISTENT_ACLS},
            Threading::{GetCurrentProcess, OpenProcessToken},
            IO::IO_STATUS_BLOCK,
        },
    },
};

// These three declaration-only APIs are in the SDK's Authorization module.
// The existing locked dependency enables the rest of the native bindings.
const SDDL_REVISION_1: u32 = 1;
const SE_FILE_OBJECT: i32 = 1;

// https://learn.microsoft.com/windows/win32/api/sddl/
// https://learn.microsoft.com/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo
#[link(name = "advapi32")]
extern "system" {
    #[link_name = "ConvertSidToStringSidW"]
    fn convert_sid_to_string(sid: PSID, string_sid: *mut *mut u16) -> i32;
    #[link_name = "ConvertStringSecurityDescriptorToSecurityDescriptorW"]
    fn convert_string_to_security_descriptor(
        sddl: *const u16,
        revision: u32,
        descriptor: *mut PSECURITY_DESCRIPTOR,
        descriptor_size: *mut u32,
    ) -> i32;
    #[link_name = "GetSecurityInfo"]
    fn get_security_info(
        object: HANDLE,
        object_type: i32,
        security_info: OBJECT_SECURITY_INFORMATION,
        owner: *mut PSID,
        group: *mut PSID,
        dacl: *mut *mut ACL,
        sacl: *mut *mut ACL,
        descriptor: *mut PSECURITY_DESCRIPTOR,
    ) -> WIN32_ERROR;
}

const SHARING: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn check(success: i32) -> io::Result<()> {
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn nt_result(status: i32) -> io::Result<()> {
    if status < 0 {
        // SAFETY: status is returned by an NT file operation.
        return Err(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        ));
    }
    Ok(())
}

struct LocalMemory(*mut c_void);
impl Drop for LocalMemory {
    fn drop(&mut self) {
        // SAFETY: this handle owns an allocation returned by a LocalAlloc-based API.
        unsafe { LocalFree(self.0) };
    }
}

// Use aligned, owned self-relative descriptor bytes so the bundle can move
// from the storage blocking task back to session admission.
pub(super) struct PrivateAcl(Vec<usize>);

impl PrivateAcl {
    pub(super) fn current_user() -> io::Result<Self> {
        let mut token = null_mut();
        // SAFETY: token is a writable HANDLE slot; the process pseudohandle is borrowed.
        check(unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) })?;
        // SAFETY: OpenProcessToken returned a uniquely owned handle.
        let token = unsafe { OwnedHandle::from_raw_handle(token) };
        // An aligned stack buffer covers TOKEN_USER plus the maximum SID (68 bytes).
        let mut user = [0usize; 16];
        let mut length = 0;
        // SAFETY: user is aligned for TOKEN_USER and its full capacity is supplied.
        check(unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenUser,
                user.as_mut_ptr().cast(),
                size_of_val(&user) as u32,
                &mut length,
            )
        })?;
        let mut sid_text = null_mut();
        // SAFETY: GetTokenInformation populated a TOKEN_USER with its SID in user.
        check(unsafe {
            convert_sid_to_string(
                (*user.as_ptr().cast::<TOKEN_USER>()).User.Sid,
                &mut sid_text,
            )
        })?;
        let _sid_text = LocalMemory(sid_text.cast());
        // SAFETY: the conversion API returns a terminated SID string of at most
        // the conversion API. Read through the terminator.
        let sid_text: Vec<u16> = (0..)
            .map(|index| unsafe { *sid_text.add(index) })
            .take_while(|unit| *unit != 0)
            .collect();
        let sid = String::from_utf16(&sid_text).map_err(|_| invalid("Invalid image owner SID"))?;
        let sddl: Vec<u16> = format!("O:{sid}D:P(A;;FA;;;{sid})")
            .encode_utf16()
            .chain([0])
            .collect();
        let mut descriptor = null_mut();
        // SAFETY: sddl is NUL-terminated; the API returns a descriptor owned by LocalFree.
        check(unsafe {
            convert_string_to_security_descriptor(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        })?;
        let descriptor = LocalMemory(descriptor);
        // SAFETY: the conversion returned a valid self-relative descriptor.
        let length = unsafe { GetSecurityDescriptorLength(descriptor.0) } as usize;
        let mut bytes = vec![0usize; length.div_ceil(size_of::<usize>())];
        // SAFETY: the API reported the valid descriptor byte count, and the
        // aligned destination allocation covers it.
        unsafe {
            std::ptr::copy_nonoverlapping(
                descriptor.0.cast::<u8>(),
                bytes.as_mut_ptr().cast(),
                length,
            )
        };
        Ok(Self(bytes))
    }

    pub(super) fn verify(&self, file: &File) -> io::Result<()> {
        let mut owner = null_mut();
        let mut acl = null_mut();
        let mut descriptor = null_mut();
        // SAFETY: valid handle and output slots. Owner and ACL live inside descriptor.
        let status = unsafe {
            get_security_info(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                null_mut(),
                &mut acl,
                null_mut(),
                &mut descriptor,
            )
        };
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        let descriptor = LocalMemory(descriptor);
        let mut expected = null_mut();
        let mut control = 0;
        let mut revision = 0;
        let mut owner_defaulted = 0;
        // SAFETY: both security descriptors remain allocated while their fields are used.
        check(unsafe {
            GetSecurityDescriptorOwner(
                self.0.as_ptr().cast_mut().cast(),
                &mut expected,
                &mut owner_defaulted,
            )
        })?;
        check(unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) })?;
        // A successful query can still return a null DACL.
        let acl = NonNull::new(acl).ok_or_else(|| invalid("Image object has no DACL"))?;
        // SAFETY: the successful queries supplied live SIDs in the retained descriptors.
        if owner.is_null()
            || expected.is_null()
            || control & SE_DACL_PROTECTED == 0
            || unsafe { EqualSid(owner, expected) == 0 }
        {
            return Err(invalid(
                "Image object does not have a protected owner-only ACL",
            ));
        }

        let mut acl_info = ACL_SIZE_INFORMATION::default();
        // SAFETY: the DACL resides in the retained descriptor and the output
        // buffer has the exact size for AclSizeInformation.
        check(unsafe {
            GetAclInformation(
                acl.as_ptr(),
                std::ptr::from_mut(&mut acl_info).cast(),
                size_of_val(&acl_info) as u32,
                AclSizeInformation,
            )
        })?;
        if acl_info.AceCount != 1 {
            return Err(invalid(
                "Image object has an unexpected number of ACL entries",
            ));
        }
        let mut ace = null_mut();
        // SAFETY: the DACL contains exactly one entry, and ace is a writable slot.
        check(unsafe { GetAce(acl.as_ptr(), 0, &mut ace) })?;

        // SAFETY: this SID came from the retained descriptor we created.
        let expected_len = unsafe { GetLengthSid(expected) } as usize;
        let sid_offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
        let ace_len = sid_offset + expected_len;
        let acl_size = acl_info.AclBytesInUse as usize;
        let Some(ace_offset) = (ace as usize).checked_sub(acl.as_ptr() as usize) else {
            return Err(invalid("Invalid image ACL entry position"));
        };
        if !(size_of::<ACL>()..=acl_size).contains(&ace_offset) || ace_len > acl_size - ace_offset {
            return Err(invalid("Invalid image ACL entry length"));
        }
        // SAFETY: both ranges are within the retained, validated descriptors.
        // The entry range above covers the fixed header, mask and exact SID.
        let (entry, expected_sid) = unsafe {
            (
                slice::from_raw_parts(acl.as_ptr().cast::<u8>().add(ace_offset), ace_len),
                slice::from_raw_parts(expected.cast::<u8>(), expected_len),
            )
        };
        let entry_len = usize::from(u16::from_le_bytes([entry[2], entry[3]]));
        let access = u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]);
        if u32::from(entry[0]) != ACCESS_ALLOWED_ACE_TYPE
            || entry[1] != 0
            || entry_len != ace_len
            || access != FILE_ALL_ACCESS
            || &entry[sid_offset..] != expected_sid
        {
            return Err(invalid("Image object grants access outside the owner ACL"));
        }
        Ok(())
    }
}

fn component(name: &OsStr) -> io::Result<Vec<u16>> {
    let name: Vec<u16> = name.encode_wide().collect();
    if name.is_empty()
        || name.len() > 255
        || matches!(name.last(), Some(32 | 46))
        || name.iter().any(|unit| matches!(unit, 0 | 47 | 58 | 92))
    {
        return Err(invalid("Invalid image path component"));
    }
    Ok(name)
}

fn file_info(file: &File) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the live handle and writable information buffer cover the native call.
    check(unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) })?;
    Ok(info)
}

fn verify_kind(file: &File, directory: bool) -> io::Result<()> {
    let info = file_info(file)?;
    let flags = info.dwFileAttributes;
    if flags & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DEVICE) != 0
        || (flags & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
        || (!directory && info.nNumberOfLinks != 1)
    {
        return Err(invalid(
            "Invalid type, reparse point or hard link in image storage",
        ));
    }
    Ok(())
}

pub(super) fn child(
    parent: &File,
    name: &OsStr,
    directory: bool,
    access: u32,
    create: Option<&PrivateAcl>,
) -> io::Result<File> {
    let name = component(name)?;
    let length = size_of_val(name.as_slice()) as u16;
    let name = UNICODE_STRING {
        Length: length,
        MaximumLength: length,
        Buffer: name.as_ptr().cast_mut(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle(),
        ObjectName: &name,
        Attributes: OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: create.map_or(null(), |acl| acl.0.as_ptr().cast()),
        SecurityQualityOfService: null(),
    };
    // Always suppress final-component reparsing, including exclusive creation
    // against a pre-existing dangling link. Check the returned kind on reopen.
    let options = FILE_OPEN_REPARSE_POINT
        | FILE_WRITE_THROUGH
        | if create.is_some() && directory {
            FILE_DIRECTORY_FILE
        } else {
            0
        };
    let mut handle = null_mut();
    let mut result = IO_STATUS_BLOCK::default();
    // SAFETY: one validated component relative to the held parent, all native
    // input/output storage stays live, and no inheritance flag is set.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            access,
            &attributes,
            &mut result,
            null(),
            if directory { 0 } else { FILE_ATTRIBUTE_NORMAL },
            SHARING,
            if create.is_some() {
                FILE_CREATE
            } else {
                FILE_OPEN
            },
            options | FILE_SYNCHRONOUS_IO_NONALERT,
            null(),
            0,
        )
    };
    nt_result(status)?;
    // SAFETY: a synchronous success transfers the uniquely owned file handle.
    let file = unsafe { File::from_raw_handle(handle) };
    verify_kind(&file, directory)?;
    Ok(file)
}

pub(super) struct Anchor {
    handles: Vec<File>,
}

impl Anchor {
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        let mut components = path.components();
        let drive = match components.next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => return Err(invalid("Image storage requires a local NTFS drive")),
            },
            _ => return Err(invalid("Image storage requires an absolute drive path")),
        };
        if components.next() != Some(Component::RootDir) {
            return Err(invalid("Image storage requires an absolute drive path"));
        }
        let drive_root = format!("{}:\\", char::from(drive));
        let drive_name: Vec<u16> = drive_root.encode_utf16().chain([0]).collect();
        // SAFETY: drive_name is a NUL-terminated drive root. DRIVE_FIXED is 3.
        if unsafe { GetDriveTypeW(drive_name.as_ptr()) } != 3 {
            return Err(invalid("Image storage requires a fixed local drive"));
        }
        let volume = OpenOptions::new()
            .read(true)
            .share_mode(SHARING)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&drive_root)?;
        verify_kind(&volume, true)?;
        let mut file_system = [0; 16];
        let mut flags = 0;
        // SAFETY: all supplied outputs are valid, and the optional outputs are null.
        check(unsafe {
            GetVolumeInformationByHandleW(
                volume.as_raw_handle(),
                null_mut(),
                0,
                null_mut(),
                null_mut(),
                &mut flags,
                file_system.as_mut_ptr(),
                file_system.len() as u32,
            )
        })?;
        if flags & FILE_PERSISTENT_ACLS == 0
            || !file_system
                .iter()
                .copied()
                .take_while(|unit| *unit != 0)
                .eq("NTFS".encode_utf16())
        {
            return Err(invalid(
                "Image storage requires local NTFS with persistent ACLs",
            ));
        }
        let mut volume_name = [0; 64];
        // SAFETY: the output capacity is supplied and the handle remains open.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                volume.as_raw_handle(),
                volume_name.as_mut_ptr(),
                volume_name.len() as u32,
                VOLUME_NAME_GUID,
            )
        } as usize;
        if length == 0 || length >= volume_name.len() {
            return Err(invalid("Image storage volume identity is unavailable"));
        }
        let volume_name = String::from_utf16_lossy(&volume_name[..length]);
        let volume_id = volume_name
            .strip_prefix(r"\\?\Volume{")
            .and_then(|name| name.strip_suffix("}\\"))
            .and_then(|id| uuid::Uuid::parse_str(id).ok());
        if volume_id.is_none() {
            return Err(invalid(
                "Image storage drive does not resolve to a volume root",
            ));
        }

        let mut handles = vec![volume];
        for part in components {
            let Component::Normal(name) = part else {
                return Err(invalid("Invalid image storage ancestor"));
            };
            // Keep every ancestor alive to pin the complete path through publication.
            let parent = handles.last().expect("volume handle is retained");
            let opened = child(
                parent,
                name,
                true,
                FILE_LIST_DIRECTORY
                    | FILE_READ_ATTRIBUTES
                    | windows_sys::Win32::Storage::FileSystem::SYNCHRONIZE,
                None,
            )?;
            handles.push(opened);
        }
        if handles.len() == 1 {
            return Err(invalid("Image storage root must be below the volume root"));
        }
        Ok(Self { handles })
    }

    pub(super) fn directory(&self) -> &File {
        self.handles.last().expect("volume handle is retained")
    }
}

pub(super) fn names(directory: &File) -> io::Result<Vec<OsString>> {
    const HEADER: usize = offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
    let mut entries = Vec::new();
    let mut info_class = FileIdBothDirectoryRestartInfo;
    loop {
        let mut buffer = [0u64; 512];
        // SAFETY: the buffer is aligned for the directory info records and
        // supplies its full writable size.
        let read = unsafe {
            GetFileInformationByHandleEx(
                directory.as_raw_handle(),
                info_class,
                buffer.as_mut_ptr().cast(),
                size_of_val(&buffer) as u32,
            )
        };
        if read == 0 {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                Ok(entries)
            } else {
                Err(error)
            };
        }

        info_class = FileIdBothDirectoryInfo;
        let bytes = size_of_val(&buffer);
        let mut offset = 0;
        loop {
            if offset % size_of::<u64>() != 0 || offset + HEADER > bytes {
                return Err(invalid("Invalid image directory record"));
            }
            // SAFETY: header fields fit in the aligned buffer. Do not make a
            // full struct reference because the final file name is variable length.
            let (next, name_len) = unsafe {
                let record = buffer
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .cast::<FILE_ID_BOTH_DIR_INFO>();
                (
                    (*record).NextEntryOffset as usize,
                    (*record).FileNameLength as usize,
                )
            };
            if name_len == 0
                || name_len % 2 != 0
                || name_len > bytes - offset - HEADER
                || (next != 0 && (next < HEADER + name_len || next > bytes - offset))
            {
                return Err(invalid("Invalid image directory name length"));
            }
            // SAFETY: validated length and the native file-name offset are in bounds.
            let name = unsafe {
                let start = buffer
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset + HEADER)
                    .cast::<u16>();
                OsString::from_wide(slice::from_raw_parts(start, name_len / 2))
            };
            if name != "." && name != ".." {
                entries.push(name);
            }
            if next == 0 {
                break;
            }
            offset += next;
        }
    }
}

pub(super) fn rename(directory: &File, name: &OsStr) -> io::Result<()> {
    let name = component(name)?;
    let name_bytes = size_of_val(name.as_slice());
    let offset = offset_of!(FILE_RENAME_INFORMATION, FileName);
    let bytes = size_of::<FILE_RENAME_INFORMATION>() + name_bytes;
    let mut info = vec![0usize; bytes.div_ceil(size_of::<usize>())];
    let mut result = IO_STATUS_BLOCK::default();
    // SAFETY: buffer has the required alignment and capacity for the native
    // header and inline UTF-16 name. Null RootDirectory and a single name
    // rename the held directory within its current parent.
    let status = unsafe {
        let record = info.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
        (*record).Anonymous.ReplaceIfExists = false;
        (*record).RootDirectory = null_mut();
        (*record).FileNameLength = name_bytes as u32;
        let name_out = info.as_mut_ptr().cast::<u8>().add(offset).cast::<u16>();
        std::ptr::copy_nonoverlapping(name.as_ptr(), name_out, name.len());
        NtSetInformationFile(
            directory.as_raw_handle(),
            &mut result,
            info.as_ptr().cast(),
            bytes as u32,
            FileRenameInformation,
        )
    };
    nt_result(status)
}

pub(super) fn mark_remove(file: &File) -> io::Result<()> {
    let info = FILE_DISPOSITION_INFO_EX {
        Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
    };
    // SAFETY: file is a verified object opened with DELETE. Closing this
    // handle removes the name even if an existing share-delete reader remains.
    check(unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileDispositionInfoEx,
            std::ptr::from_ref(&info).cast(),
            size_of_val(&info) as u32,
        )
    })
}

pub(super) fn verify_dispatch_path(path: &Path, owned: &File) -> io::Result<File> {
    let launch_file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    verify_kind(&launch_file, false)?;
    let a = file_info(&launch_file)?;
    let b = file_info(owned)?;
    if (a.dwVolumeSerialNumber, a.nFileIndexHigh, a.nFileIndexLow)
        != (b.dwVolumeSerialNumber, b.nFileIndexHigh, b.nFileIndexLow)
    {
        return Err(invalid(
            "Image launch path no longer names the published file",
        ));
    }
    Ok(launch_file)
}
