//! Per-user runtime state, separate from the owned game installation.
use std::io;
use std::path::PathBuf;
pub(crate) fn log_directory() -> io::Result<PathBuf> {
    maiko_launcher::settings::log_directory()
}

pub(crate) fn default_game() -> Option<PathBuf> {
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
