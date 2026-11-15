//! The system's folder picker. `None` when the user cancels or the system has none, in which case the
//! launcher asks for the path in its own window.
use std::path::PathBuf;

pub fn choose_folder() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Select your Redguard folder (the one containing RGFX.EXE)")
        .pick_folder()
}
