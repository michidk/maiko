//! Unix host implementation.
mod crash;
#[cfg(not(target_os = "macos"))]
mod dialog;
mod filesystem;
mod host;

pub(crate) use crash::*;
#[cfg(not(target_os = "macos"))]
pub(crate) use dialog::*;
pub(crate) use filesystem::*;
pub(crate) use host::*;

mod logging;
pub(crate) use logging::{capture_output, write_console};
