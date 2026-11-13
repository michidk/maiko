//! Redirect native diagnostics synchronously so fatal reports survive _exit.
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
pub(crate) fn capture_output(file: &File) -> io::Result<Option<File>> {
    let original = unsafe { libc::dup(2) };
    let console = (original >= 0).then(|| unsafe { File::from_raw_fd(original) });
    for destination in [1, 2] {
        if unsafe { libc::dup2(file.as_raw_fd(), destination) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(console)
}
pub(crate) fn write_console(file: &File, bytes: &[u8]) {
    unsafe {
        libc::write(file.as_raw_fd(), bytes.as_ptr().cast(), bytes.len());
    }
}
