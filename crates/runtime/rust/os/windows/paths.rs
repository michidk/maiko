//! Per-user runtime state, separate from the owned game installation.
use std::io;
use std::path::PathBuf;

pub(crate) fn default_game() -> Option<PathBuf> {
    None
}
pub(crate) fn remember_game(_game: &std::path::Path) -> io::Result<()> {
    Ok(())
}
/// The drive the CD release was installed from, as SYSTEM.INI names it.
pub(crate) fn disc_roots(letter: u8) -> Vec<PathBuf> {
    vec![PathBuf::from(format!("{}:\\", letter as char))]
}
