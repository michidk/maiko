use crate::rg_log;
use std::ffi::{c_char, c_int};
pub(crate) unsafe fn choose_game_directory(path: *mut c_char, capacity: usize) -> c_int {
    // No native dialog: ask in the terminal the game was started from.
    use std::io::{BufRead, IsTerminal, Write};
    if path.is_null() || capacity == 0 || !std::io::stdin().is_terminal() {
        rg_log!("Select a game folder with --game-dir GAME_DIRECTORY");
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
            rg_log!("The Redguard files (RGFX.EXE, FXART, MAPS) are not in or around that folder.");
            continue;
        }
        if unsafe { crate::storage::copy_path(directory, path, capacity) }.is_ok() {
            return 1;
        }
    }
}

pub(crate) fn show_error(_text: &str) {}
