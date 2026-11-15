//! Host path encoding and operations on already opened capability directories.
use cap_std::fs::Dir;
use std::ffi::{CStr, CString, OsStr, c_char};
use std::io;
use std::path::{Path, PathBuf};
pub(crate) unsafe fn path_from_c(pointer: *const c_char) -> io::Result<PathBuf> {
    if pointer.is_null() {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let path = unsafe { CStr::from_ptr(pointer) };

    use std::os::unix::ffi::OsStrExt;
    Ok(PathBuf::from(OsStr::from_bytes(path.to_bytes())))
}
pub(crate) fn path_cstring(path: &Path) -> io::Result<CString> {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::PermissionDenied))
}
pub(crate) fn inside(path: &Path, base: &Path) -> Option<PathBuf> {
    path.strip_prefix(base).ok().map(Path::to_path_buf)
}
pub(crate) fn sync_directory(dir: &Dir) -> io::Result<()> {
    dir.open(".")?.sync_all()
}
pub(crate) fn canonical(path: &Path) -> io::Result<PathBuf> {
    std::fs::canonicalize(path)
}
/// Atomically move between open directories without replacing an existing name.
/// Native rename also supports filesystems without hard links, such as FAT/exFAT.
pub(crate) fn rename_noreplace(
    source: &Dir,
    old: &OsStr,
    destination: &Dir,
    new: &OsStr,
) -> io::Result<()> {
    use std::os::{fd::AsRawFd, unix::ffi::OsStrExt};
    let old = CString::new(old.as_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::PermissionDenied))?;
    let new = CString::new(new.as_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::PermissionDenied))?;
    let result = unsafe {
        crate::os::rename(
            source.as_raw_fd(),
            old.as_ptr(),
            destination.as_raw_fd(),
            new.as_ptr(),
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(crate) fn logical_drives() -> u32 {
    1 << 2
}

pub(crate) fn dos_timestamp(seconds: u64) -> (u16, u16) {
    let seconds = seconds as libc::time_t;
    let mut tm = std::mem::MaybeUninit::<libc::tm>::zeroed();
    if unsafe { libc::localtime_r(&seconds, tm.as_mut_ptr()) }.is_null() {
        return (0, 0);
    }
    let tm = unsafe { tm.assume_init() };
    if !(80..208).contains(&tm.tm_year) {
        return (0, 0);
    }
    (
        ((tm.tm_hour << 11) | (tm.tm_min << 5) | (tm.tm_sec / 2)) as u16,
        (((tm.tm_year - 80) << 9) | ((tm.tm_mon + 1) << 5) | tm.tm_mday) as u16,
    )
}

pub(crate) fn file_attributes(meta: &std::fs::Metadata, _name: &str) -> u8 {
    u8::from(meta.is_dir()) * 16
        + u8::from(_name.starts_with('.') && _name != "." && _name != "..") * 2
}

pub(crate) fn too_many_open_files(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(23 | 24))
}

pub(crate) fn read_at(file: &std::fs::File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buffer, offset)
}

pub(crate) fn current_drive(_isolated: bool) -> u64 {
    2
}
pub(crate) fn select_drive(_drive: u8, _isolated: bool) -> u64 {
    3
}
