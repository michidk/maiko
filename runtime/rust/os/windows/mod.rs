//! Windows host implementation.
mod crash;
mod dialog;
mod filesystem;
mod host;
mod memory;
mod window;

pub(crate) use crash::*;
pub(crate) use dialog::*;
pub(crate) use filesystem::*;
pub(crate) use host::*;
pub(crate) use memory::*;
pub(crate) use window::renderer_window;

pub(crate) const NAME: &str = "Windows native x64";

mod logging;
pub(crate) use logging::{capture_output, write_console};

mod paths;
pub(crate) use paths::{
    default_game, default_user_directory, disc_roots, log_directory, remember_game,
};
