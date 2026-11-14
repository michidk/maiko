//! Maiko's launcher: finds the game folder, shows the original Redguard launcher with artwork read
//! from the player's own `REDGUARD.EXE`, and starts the native game with the chosen options.
//!
//! The library half (finding the game, the launcher settings and the artwork reader) is shared with
//! the game runtime and has no UI; the window is the `maiko` binary (feature `app`).
#[cfg(feature = "app")]
pub mod app;
pub mod artwork;
pub mod font;
pub mod locate;
pub mod picker;
pub mod settings;
