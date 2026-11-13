//! Folder selection ABI. Unix hosts use the explicit game-directory option.
use std::ffi::{c_char, c_int};
/// # Safety
/// `path` points to `capacity` writable bytes for a NUL-terminated host path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_choose_game_directory(path: *mut c_char, capacity: usize) -> c_int {
    unsafe { crate::os::choose_game_directory(path, capacity) }
}

#[cfg(any(target_os = "macos", all(test, unix)))]
pub(crate) fn picker_path(
    output: std::process::Output,
    capacity: usize,
) -> Option<std::ffi::CString> {
    if !output.status.success() {
        return None;
    }
    let path = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    if path.is_empty() || path.len() >= capacity {
        return None;
    }
    std::ffi::CString::new(path).ok()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    #[test]
    fn folder_picker_preserves_spaces_and_rejects_cancel_overflow_and_nul() {
        let output = |status, bytes: &[u8]| std::process::Output {
            status: std::process::ExitStatus::from_raw(status),
            stdout: bytes.to_vec(),
            stderr: Vec::new(),
        };
        assert_eq!(
            picker_path(output(0, b"/Users/test/Game Files/\n"), 64)
                .unwrap()
                .to_bytes(),
            b"/Users/test/Game Files/"
        );
        assert!(picker_path(output(256, b"cancelled"), 64).is_none());
        assert!(picker_path(output(0, b"abcd\n"), 4).is_none());
        assert!(picker_path(output(0, b"a\0b\n"), 64).is_none());
    }
}
