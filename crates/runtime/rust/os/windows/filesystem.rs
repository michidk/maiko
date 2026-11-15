//! Host path encoding and operations on already opened capability directories.
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use std::ffi::{CStr, CString, OsStr, OsString, c_char};
use std::io;
use std::path::{Path, PathBuf};
pub(crate) unsafe fn path_from_c(pointer: *const c_char) -> io::Result<PathBuf> {
    if pointer.is_null() {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let path = unsafe { CStr::from_ptr(pointer) };

    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Globalization::{CP_ACP, MultiByteToWideChar};
    let count = unsafe {
        MultiByteToWideChar(CP_ACP, 0, path.as_ptr().cast(), -1, std::ptr::null_mut(), 0)
    };
    if count <= 0 {
        return Err(io::Error::last_os_error());
    }
    let mut wide = vec![0; count as usize];
    if unsafe {
        MultiByteToWideChar(
            CP_ACP,
            0,
            path.as_ptr().cast(),
            -1,
            wide.as_mut_ptr(),
            count,
        )
    } != count
    {
        return Err(io::Error::last_os_error());
    }
    Ok(PathBuf::from(OsString::from_wide(&wide[..wide.len() - 1])))
}
pub(crate) fn path_cstring(path: &Path) -> io::Result<CString> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Globalization::{CP_ACP, WideCharToMultiByte};
    let wide: Vec<_> = path.as_os_str().encode_wide().collect();
    let count = unsafe {
        WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    if count <= 0 {
        return Err(io::Error::last_os_error());
    }
    let mut bytes = vec![0; count as usize];
    if unsafe {
        WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            bytes.as_mut_ptr(),
            count,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    } != count
    {
        return Err(io::Error::last_os_error());
    }
    CString::new(bytes).map_err(|_| io::Error::from(io::ErrorKind::PermissionDenied))
}
pub(crate) fn inside(path: &Path, base: &Path) -> Option<PathBuf> {
    let path = path.to_string_lossy().replace('\\', "/");
    let base = base.to_string_lossy().replace('\\', "/");
    let base = base.trim_end_matches('/');
    if path.len() >= base.len()
        && path
            .get(..base.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(base))
        && (path.len() == base.len() || path.as_bytes()[base.len()] == b'/')
    {
        Some(PathBuf::from(path[base.len()..].trim_start_matches('/')))
    } else {
        None
    }
}
pub(crate) fn sync_directory(dir: &Dir) -> io::Result<()> {
    let _ = dir;
    Ok(())
}
pub(crate) fn canonical(path: &Path) -> io::Result<PathBuf> {
    let path = std::fs::canonicalize(path)?;

    let text = path.to_string_lossy();
    if let Some(tail) = text.strip_prefix(r"\\?\UNC\") {
        return Ok(PathBuf::from(format!(r"\\{tail}")));
    }
    if let Some(tail) = text.strip_prefix(r"\\?\") {
        return Ok(PathBuf::from(tail));
    }
    Ok(path)
}
/// Atomically move between open directories without replacing an existing name.
/// Native rename also supports filesystems without hard links, such as FAT/exFAT.
pub(crate) fn rename_noreplace(
    source: &Dir,
    old: &OsStr,
    destination: &Dir,
    new: &OsStr,
) -> io::Result<()> {
    use cap_std::fs::OpenOptionsExt;
    use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
    use windows_sys::Wdk::Storage::FileSystem::{
        FILE_RENAME_INFORMATION, FileRenameInformation, NtSetInformationFile,
    };
    use windows_sys::Win32::{
        Foundation::RtlNtStatusToDosError, Storage::FileSystem::DELETE, System::IO::IO_STATUS_BLOCK,
    };
    let mut options = OpenOptions::new();
    options.access_mode(DELETE).follow(FollowSymlinks::No);
    let file = source.open_with(old, &options)?;
    let name = new.encode_wide().collect::<Vec<_>>();
    let bytes = std::mem::offset_of!(FILE_RENAME_INFORMATION, FileName) + name.len() * 2;
    let mut buffer = vec![
        0u64;
        bytes
            .max(std::mem::size_of::<FILE_RENAME_INFORMATION>())
            .div_ceil(8)
    ];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    unsafe {
        (*info).RootDirectory = destination.as_raw_handle();
        (*info).FileNameLength = (name.len() * 2) as u32;
        (*info)
            .FileName
            .as_mut_ptr()
            .copy_from_nonoverlapping(name.as_ptr(), name.len());
        let mut result: IO_STATUS_BLOCK = std::mem::zeroed();
        let status = NtSetInformationFile(
            file.as_raw_handle(),
            &mut result,
            info.cast(),
            (buffer.len() * 8) as u32,
            FileRenameInformation,
        );
        if status < 0 {
            return Err(io::Error::from_raw_os_error(
                RtlNtStatusToDosError(status) as i32
            ));
        }
    }
    Ok(())
}

pub(crate) fn logical_drives() -> u32 {
    (unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() }) & 0x01ff_fffc
}

pub(crate) fn dos_timestamp(seconds: u64) -> (u16, u16) {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::Storage::FileSystem::FileTimeToLocalFileTime;
    use windows_sys::Win32::System::WindowsProgramming::FileTimeToDosDateTime;
    let value = (seconds + 11644473600) * 10000000;
    let time = FILETIME {
        dwLowDateTime: value as u32,
        dwHighDateTime: (value >> 32) as u32,
    };
    let mut local = std::mem::MaybeUninit::<FILETIME>::zeroed();
    let (mut date, mut clock) = (0, 0);
    unsafe {
        if FileTimeToLocalFileTime(&time, local.as_mut_ptr()) != 0 {
            FileTimeToDosDateTime(local.as_ptr(), &mut date, &mut clock);
        }
    }
    (clock, date)
}

pub(crate) fn file_attributes(meta: &std::fs::Metadata, _name: &str) -> u8 {
    use std::os::windows::fs::MetadataExt;
    (meta.file_attributes() & 0x3f) as u8
}

pub(crate) fn too_many_open_files(error: &io::Error) -> bool {
    error.raw_os_error() == Some(4)
}

pub(crate) fn read_at(file: &std::fs::File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buffer, offset)
}

pub(crate) fn current_drive(isolated: bool) -> u64 {
    let mut drive = 2;
    if !isolated && let Ok(cwd) = std::env::current_dir() {
        let text = cwd.to_string_lossy();
        if text.as_bytes().get(1) == Some(&b':') {
            drive = u64::from(text.as_bytes()[0].to_ascii_uppercase() - b'A');
        }
    }
    drive
}
pub(crate) fn select_drive(drive: u8, isolated: bool) -> u64 {
    if isolated {
        return 4;
    }
    if drive < 26 {
        let _ = std::env::set_current_dir(format!("{}:", char::from(b'A' + drive)));
    }
    let drives = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
    u64::from(32 - drives.leading_zeros())
}
