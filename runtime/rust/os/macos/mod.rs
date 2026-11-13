//! macOS host implementation.
mod crash;
mod filesystem;
mod memory;

pub(crate) use crash::report_context;
pub(crate) use filesystem::rename;
pub(crate) use memory::*;

pub(crate) const NAME: &str = "macOS native";

mod paths;
pub(crate) use paths::{default_game, default_user_directory, log_directory, remember_game};

mod dialog;
pub(crate) use dialog::choose_game_directory;
