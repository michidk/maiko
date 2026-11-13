//! macOS's built-in folder picker, launched before SDL initializes.
use crate::rg_log;
use std::ffi::{c_char, c_int};
pub(crate) unsafe fn choose_game_directory(path: *mut c_char, capacity: usize) -> c_int {
    if path.is_null() || capacity == 0 {
        return 0;
    }
    let output=std::process::Command::new("/usr/bin/osascript")
        .args(["-e", "POSIX path of (choose folder with prompt \"Select your Redguard folder containing RGFX.EXE\")"])
        .output();
    let selected = output
        .ok()
        .and_then(|output| crate::dialog::picker_path(output, capacity));
    let Some(selected) = selected else {
        rg_log!("event=installation-selection result=cancelled-or-picker-failed");
        return 0;
    };
    unsafe {
        path.copy_from_nonoverlapping(selected.as_ptr(), selected.as_bytes_with_nul().len());
    }
    1
}
