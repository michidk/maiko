//! Client-side build preparation shared by the native CLI and WebAssembly.
pub mod build;
pub mod finish;
pub mod image;
pub mod kit;
pub mod llasm;
pub mod macho;
pub mod macos;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
pub mod prepare;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub mod relink;

#[cfg(not(target_arch = "wasm32"))]
pub mod macos_package;
