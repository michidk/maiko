//! Per-user runtime state, separate from the owned game installation.
use std::io;
use std::path::PathBuf;
fn home() -> io::Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}
pub(crate) fn state_directory() -> io::Result<PathBuf> {
    home().map(|path| path.join("Library/Application Support/Maiko"))
}
pub(crate) fn log_directory() -> io::Result<PathBuf> {
    home().map(|path| path.join("Library/Logs/Maiko"))
}

pub(crate) fn default_game() -> Option<PathBuf> {
    let bytes = std::fs::read(state_directory().ok()?.join("game-directory")).ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    let path = std::ffi::CString::from_vec_with_nul(bytes).ok()?;
    let path = unsafe { crate::os::path_from_c(path.as_ptr()) }.ok()?;
    path.join("RGFX.EXE").is_file().then_some(path)
}
pub(crate) fn default_user_directory() -> Option<PathBuf> {
    state_directory().ok().map(|path| path.join("user-data"))
}
pub(crate) fn remember_game(game: &std::path::Path) -> io::Result<()> {
    let directory = state_directory()?;
    std::fs::create_dir_all(&directory)?;
    let temporary = directory.join(format!("game-directory-{}.tmp", std::process::id()));
    std::fs::write(
        &temporary,
        crate::os::path_cstring(game)?.as_bytes_with_nul(),
    )?;
    std::fs::rename(temporary, directory.join("game-directory"))
}
