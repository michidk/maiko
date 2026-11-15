//! Keep CRT and Win32 diagnostics on the same persistent log handle.
use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, BorrowedHandle, IntoRawHandle};
use windows_sys::Win32::System::Console::*;
pub(crate) fn capture_output(file: &File) -> io::Result<Option<File>> {
    let original = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
    let console =
        if original.is_null() || original == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            None
        } else {
            unsafe { BorrowedHandle::borrow_raw(original) }
                .try_clone_to_owned()
                .ok()
                .map(File::from)
        };
    let raw = file.try_clone()?.into_raw_handle();
    let descriptor = unsafe { libc::open_osfhandle(raw as isize, libc::O_APPEND | libc::O_BINARY) };
    if descriptor < 0 {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(raw);
        }
        return Err(io::Error::last_os_error());
    }
    let saved_output = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    let saved_descriptors = [unsafe { libc::dup(1) }, unsafe { libc::dup(2) }];
    let result = (|| {
        for destination in [1, 2] {
            if unsafe { libc::dup2(descriptor, destination) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        for destination in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            if unsafe { SetStdHandle(destination, file.as_raw_handle()) } == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(console)
    })();
    if result.is_err() {
        unsafe {
            SetStdHandle(STD_OUTPUT_HANDLE, saved_output);
            SetStdHandle(STD_ERROR_HANDLE, original);
        }
    }
    for (destination, saved) in [1, 2].into_iter().zip(saved_descriptors) {
        if saved >= 0 {
            unsafe {
                if result.is_err() {
                    libc::dup2(saved, destination);
                }
                libc::close(saved);
            }
        } else if result.is_err() {
            unsafe {
                libc::close(destination);
            }
        }
    }
    unsafe {
        libc::close(descriptor);
    }
    result
}
pub(crate) fn write_console(file: &File, bytes: &[u8]) {
    let mut written = 0;
    unsafe {
        windows_sys::Win32::Storage::FileSystem::WriteFile(
            file.as_raw_handle(),
            bytes.as_ptr(),
            bytes.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        );
    }
}
