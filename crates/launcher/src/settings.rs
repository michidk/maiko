//! What the launcher remembers between starts, and the game command it builds from it.
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The game executable the launcher starts, beside the launcher itself.
pub const GAME: &str = if cfg!(windows) {
    "redguard-native.exe"
} else {
    "redguard-native"
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub fullscreen: bool,
    /// Window size as `WIDTHxHEIGHT`; empty for the game's default.
    pub window_size: String,
    pub skip_intro: bool,
    pub skip_outro: bool,
    /// `--texture-filter` value; empty for the game's own choice.
    pub texture_filter: String,
    /// Graphics backend for the renderer (`WGPU_BACKEND`); empty lets it choose.
    pub renderer: String,
    /// Windows: show the game's console window with its log while it runs. Off by default.
    pub console: bool,
    /// A disc image (cue sheet) for CD music when the game folder has none beside it.
    pub cd_image: Option<PathBuf>,
}

/// `name` in Maiko's configuration folder.
pub(crate) fn config_file(name: &str) -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "maiko").map(|dirs| dirs.config_dir().join(name))
}

/// Write `value` as JSON to `name` in Maiko's configuration folder.
pub(crate) fn save_config(name: &str, value: &impl Serialize) -> std::io::Result<()> {
    let file = config_file(name)
        .ok_or_else(|| std::io::Error::other("no configuration folder on this system"))?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, serde_json::to_vec_pretty(value)?)
}

impl Settings {
    pub fn load() -> Self {
        config_file("launcher.json")
            .and_then(|file| std::fs::read(file).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        save_config("launcher.json", self)
    }

    /// The `WGPU_BACKEND` value for the game, when a known backend is chosen.
    pub fn renderer_backend(&self) -> Option<&str> {
        matches!(self.renderer.as_str(), "vulkan" | "dx12" | "metal" | "gl")
            .then_some(self.renderer.as_str())
    }

    /// The game's arguments for `game`. The game reads and writes its own files (INI files,
    /// saves) in that folder, like the original.
    pub fn arguments(&self, game: &Path) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec!["--game-dir".into(), game.into()];
        if self.fullscreen {
            args.push("--fullscreen".into());
        } else if parse_size(&self.window_size).is_some() {
            args.extend(["--window-size".into(), self.window_size.clone().into()]);
        }
        if self.skip_intro {
            args.push("--skip-intro".into());
        }
        if self.skip_outro {
            args.push("--skip-outro".into());
        }
        if matches!(self.texture_filter.as_str(), "smooth" | "sharp") {
            args.extend([
                "--texture-filter".into(),
                self.texture_filter.clone().into(),
            ]);
        }
        if let Some(image) = &self.cd_image {
            args.extend(["--cd-image".into(), image.into()]);
        }
        args
    }
}

fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (width, height) = text.split_once(['x', 'X'])?;
    let size = (width.trim().parse().ok()?, height.trim().parse().ok()?);
    (size.0 >= 320 && size.1 >= 240 && size.0 <= 16384 && size.1 <= 16384).then_some(size)
}

/// Where the game writes a log of every start: `%LOCALAPPDATA%\Maiko\logs` on Windows,
/// `~/Library/Logs/Maiko` on macOS, and `$XDG_STATE_HOME/maiko/logs` (by default
/// `~/.local/state/maiko/logs`) elsewhere.
pub fn log_directory() -> std::io::Result<PathBuf> {
    let var = |name| std::env::var_os(name).map(PathBuf::from);
    let directory = if cfg!(windows) {
        var("LOCALAPPDATA").map(|path| path.join("Maiko/logs"))
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|home| home.join("Library/Logs/Maiko"))
    } else {
        var("XDG_STATE_HOME")
            .or_else(|| var("HOME").map(|home| home.join(".local/state")))
            .map(|path| path.join("maiko/logs"))
    };
    directory.ok_or_else(|| std::io::ErrorKind::NotFound.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_carry_the_folders_and_chosen_options() {
        let mut settings = Settings::default();
        let args = |s: &Settings| {
            s.arguments(Path::new("/g"))
                .into_iter()
                .map(|a| a.into_string().unwrap())
                .collect::<Vec<_>>()
                .join(" ")
        };
        assert_eq!(args(&settings), "--game-dir /g");
        settings.window_size = "1280x960".into();
        settings.skip_intro = true;
        assert_eq!(
            args(&settings),
            "--game-dir /g --window-size 1280x960 --skip-intro"
        );
        settings.fullscreen = true;
        assert_eq!(args(&settings), "--game-dir /g --fullscreen --skip-intro");
        settings.texture_filter = "sharp".into();
        assert!(args(&settings).ends_with("--skip-intro --texture-filter sharp"));
    }

    #[test]
    fn the_console_window_is_off_unless_chosen() {
        assert!(!Settings::default().console);
        let saved: Settings = serde_json::from_str(r#"{"fullscreen":true}"#).unwrap();
        assert!(
            !saved.console,
            "settings saved before the option keep it off"
        );
    }

    #[test]
    fn only_known_renderers_reach_the_game() {
        let renderer = |name: &str| Settings {
            renderer: name.into(),
            ..Settings::default()
        };
        assert_eq!(renderer("").renderer_backend(), None);
        assert_eq!(renderer("vulkan").renderer_backend(), Some("vulkan"));
        assert_eq!(renderer("dx12,gl").renderer_backend(), None);
    }

    #[test]
    fn window_sizes_must_be_sensible() {
        assert_eq!(parse_size("1024x768"), Some((1024, 768)));
        assert_eq!(parse_size("100x100"), None);
        assert_eq!(parse_size("big"), None);
    }
}
