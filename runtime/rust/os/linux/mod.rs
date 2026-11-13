//! Linux host implementation.
mod crash;
mod filesystem;
mod memory;

pub(crate) use crash::report_context;
pub(crate) use filesystem::rename;
pub(crate) use memory::*;

pub(crate) const NAME: &str = "Linux native x64";
