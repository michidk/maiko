use std::ffi::{c_char, c_int};
pub(crate) unsafe fn choose_game_directory(path: *mut c_char, capacity: usize) -> c_int {
    let _ = (path, capacity);
    eprintln!("Select a game folder with --game-dir GAME_DIRECTORY");
    0
}
