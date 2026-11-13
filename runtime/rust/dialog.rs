//! Folder selection ABI. Unix hosts use the explicit game-directory option.
use std::ffi::{c_char, c_int};
/// # Safety
/// `path` points to `capacity` writable bytes for a NUL-terminated host path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_choose_game_directory(path: *mut c_char, capacity: usize) -> c_int {
    unsafe { crate::os::choose_game_directory(path, capacity) }
}
