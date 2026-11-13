//! Native folder selection is kept at the platform boundary.
use std::ffi::{c_char, c_int};
/// # Safety
/// `path` points to `capacity` writable bytes for a NUL-terminated host path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_choose_game_directory(path: *mut c_char, capacity: usize) -> c_int {
    #[cfg(windows)]
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
        let directory = match crate::storage::path_from_c(path) {
            Ok(p) => p,
            Err(_) => return 0,
        };
        if crate::locate::resolve(&directory).is_none() {
            MessageBoxA(
                std::ptr::null_mut(),
                c"Choose your Redguard folder: the one that contains RGFX.EXE, FXART and MAPS, or the GOG folder around it."
                    .as_ptr()
                    .cast(),
                c"Redguard game files not found".as_ptr().cast(),
                MB_OK | MB_ICONERROR,
            );
            return 0;
        }
        1
    }
    #[cfg(unix)]
    {
        // No native dialog: ask in the terminal the game was started from.
        use std::io::{BufRead, IsTerminal, Write};
        if path.is_null() || capacity == 0 || !std::io::stdin().is_terminal() {
            eprintln!("Select a game folder with --game-dir GAME_DIRECTORY");
            return 0;
        }
        loop {
            eprint!("Your Redguard folder (the one with RGFX.EXE; empty to quit): ");
            let _ = std::io::stderr().flush();
            let mut line = String::new();
            if std::io::stdin().lock().read_line(&mut line).unwrap_or(0) == 0 {
                return 0;
            }
            let line = line.trim().trim_matches(['"', '\'']);
            if line.is_empty() {
                return 0;
            }
            let directory = std::path::Path::new(line);
            if crate::locate::resolve(directory).is_none() {
                eprintln!(
                    "The Redguard files (RGFX.EXE, FXART, MAPS) are not in or around that folder."
                );
                continue;
            }
            if unsafe { crate::storage::copy_path(directory, path, capacity) }.is_ok() {
                return 1;
            }
        }
    }
}

/// Tell the user why the game did not start. Used only for launches without `--game-dir`, which
/// are double-clicks that have no console to read; scripted and terminal launches get stderr.
pub fn show_error(text: &str) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        if let (Ok(text), Ok(title)) = (
            std::ffi::CString::new(text),
            std::ffi::CString::new("Redguard could not start"),
        ) {
            MessageBoxA(
                std::ptr::null_mut(),
                text.as_ptr().cast(),
                title.as_ptr().cast(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    #[cfg(not(windows))]
    let _ = text;
}
