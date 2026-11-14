//! Per-user runtime state, separate from the owned game installation.
use std::io;
use std::path::PathBuf;
pub(crate) fn state_directory() -> io::Result<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .map(|path| path.join("maiko"))
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}
pub(crate) fn log_directory() -> io::Result<PathBuf> {
    state_directory().map(|path| path.join("logs"))
}

pub(crate) fn default_game() -> Option<PathBuf> {
    None
}
pub(crate) fn default_user_directory() -> Option<PathBuf> {
    None
}
pub(crate) fn remember_game(_game: &std::path::Path) -> io::Result<()> {
    Ok(())
}
/// Folders where a desktop or the user mounts a disc: `/media/USER/LABEL`, `/run/media/USER/LABEL`,
/// `/media/LABEL` and `/mnt/LABEL`. The DOS drive letter means nothing here.
pub(crate) fn disc_roots(_letter: u8) -> Vec<PathBuf> {
    let user = std::env::var_os("USER").unwrap_or_default();
    [
        PathBuf::from("/media").join(&user),
        PathBuf::from("/run/media").join(&user),
        PathBuf::from("/media"),
        PathBuf::from("/mnt"),
    ]
    .iter()
    .filter_map(|parent| std::fs::read_dir(parent).ok())
    .flat_map(|entries| entries.flatten().map(|entry| entry.path()))
    .collect()
}
