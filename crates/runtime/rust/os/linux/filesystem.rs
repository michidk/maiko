//! Atomic rename without replacement, relative to open capability directories.
pub(crate) unsafe fn rename(
    source: i32,
    old: *const std::ffi::c_char,
    destination: i32,
    new: *const std::ffi::c_char,
) -> i32 {
    unsafe { libc::renameat2(source, old, destination, new, libc::RENAME_NOREPLACE) }
}
