use std::ffi::{c_char, c_int};
pub(crate) unsafe fn choose_game_directory(path: *mut c_char, capacity: usize) -> c_int {
    unsafe {
        use windows_sys::Win32::{
            System::Com::*,
            UI::{Shell::*, WindowsAndMessaging::*},
        };
        if path.is_null()
            || capacity == 0
            || CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) < 0
        {
            return 0;
        }
        let mut options: BROWSEINFOA = std::mem::zeroed();
        options.lpszTitle = c"Select your Redguard game-data folder containing RGFX.EXE"
            .as_ptr()
            .cast();
        options.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;
        let selected = SHBrowseForFolderA(&options);
        let mut chosen = [0u8; 260];
        let valid = !selected.is_null() && SHGetPathFromIDListA(selected, chosen.as_mut_ptr()) != 0;
        CoTaskMemFree(selected.cast());
        CoUninitialize();
        if !valid {
            return 0;
        }
        let length = chosen.iter().position(|&b| b == 0).unwrap_or(chosen.len());
        if length >= capacity {
            return 0;
        }
        path.cast::<u8>()
            .copy_from_nonoverlapping(chosen.as_ptr(), length + 1);
        let directory = match crate::os::path_from_c(path) {
            Ok(p) => p,
            Err(_) => return 0,
        };
        if !directory.join("RGFX.EXE").is_file() {
            MessageBoxA(
                std::ptr::null_mut(),
                c"Choose the inner Redguard folder containing RGFX.EXE, SYSTEM.INI and FXART."
                    .as_ptr()
                    .cast(),
                c"Redguard game files not found".as_ptr().cast(),
                MB_OK | MB_ICONERROR,
            );
            return 0;
        }
        1
    }
}
