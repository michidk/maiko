//! Finding the Redguard game folder without asking, and remembering the answer.
//!
//! The order: the folder remembered from an earlier launch, `REDGUARD_DIR`, the folder the program
//! sits in, the folders around it (a `Redguard` or GOG `app/Redguard` beside it or above it), and the
//! standard GOG locations. Only when none of those holds the game does the caller ask the user, and
//! then it stores what was chosen. The remembered folder is Maiko's own; no other program's
//! preference is read.
//!
//! A folder counts as the game when it holds `fxart`, `maps` and `RGFX.EXE`. A chosen folder may
//! also be a subfolder of the game, or the GOG folder around it ([`resolve`]).
use std::path::{Path, PathBuf};

/// Names that make a folder the game's data folder.
const REQUIRED: [&str; 3] = ["fxart", "maps", "RGFX.EXE"];
/// Where the game folder can sit inside a folder the user picked or the program sits beside.
const LAYOUTS: [&str; 5] = [
    "Redguard",
    "redguard",
    "app/Redguard",
    "Redguard/Redguard",
    "Redguard/app/Redguard",
];
/// Environment variable that names the game folder.
pub const ENV_VAR: &str = "REDGUARD_DIR";

pub fn is_game_folder(folder: &Path) -> bool {
    if REQUIRED.iter().all(|name| folder.join(name).exists()) {
        return true;
    }
    let Ok(entries) = std::fs::read_dir(folder) else {
        return false;
    };
    let names: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    REQUIRED.iter().all(|required| {
        names.iter().any(|name| {
            name.to_str()
                .is_some_and(|name| name.eq_ignore_ascii_case(required))
        })
    })
}

/// The game's data folder for something the user picked: the folder itself, a folder above it or a
/// neighbouring `Redguard` layout. The folders above are tried before the layouts beside them, so the
/// enclosing install wins over a neighbouring package.
pub fn resolve(selected: &Path) -> Option<PathBuf> {
    let selected = std::path::absolute(selected).ok()?;
    if !selected.exists() {
        return None;
    }
    if let Some(found) = selected.ancestors().find(|p| is_game_folder(p)) {
        return Some(found.to_path_buf());
    }
    selected.ancestors().find_map(|parent| {
        LAYOUTS
            .iter()
            .map(|layout| parent.join(layout))
            .find(|path| is_game_folder(path))
    })
}

/// What the environment says about the machine; a value so that the search can be tested.
#[derive(Default, Clone)]
pub struct Machine {
    pub home: Option<PathBuf>,
    pub system_drive: Option<String>,
    pub program_files: Vec<PathBuf>,
    /// The Wine prefix (`WINEPREFIX`, else `~/.wine`) where a Windows GOG install may live.
    pub wine_prefix: Option<PathBuf>,
}

impl Machine {
    pub fn current() -> Self {
        let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
        let home = var("HOME").or_else(|| var("USERPROFILE"));
        Self {
            wine_prefix: var("WINEPREFIX").or_else(|| home.as_ref().map(|h| h.join(".wine"))),
            home,
            system_drive: std::env::var("SystemDrive").ok(),
            program_files: ["ProgramFiles(x86)", "ProgramFiles", "ProgramW6432"]
                .iter()
                .filter_map(|name| var(name))
                .collect(),
        }
    }
}

/// Conventional install folders: next to the working directory, the GOG Games folders, the Windows
/// GOG Galaxy folders, a Wine prefix, and the default Heroic and Lutris folders.
pub fn standard_folders(machine: &Machine) -> Vec<PathBuf> {
    let mut folders = vec![PathBuf::from("redguard"), PathBuf::from("Redguard")];
    if let Some(home) = &machine.home {
        folders.push(home.join("GOG Games/Redguard"));
        folders.push(home.join("GOG Games/Redguard/app/Redguard"));
    }
    if let Some(drive) = &machine.system_drive {
        folders.push(PathBuf::from(format!("{drive}/GOG Games/Redguard")));
    }
    for files in &machine.program_files {
        folders.push(files.join("GOG Galaxy/Games/Redguard"));
        folders.push(files.join("GOG Galaxy/Games/Redguard/app/Redguard"));
    }
    // A Windows GOG install inside a Wine prefix.
    if let Some(prefix) = &machine.wine_prefix {
        let drive = prefix.join("drive_c");
        for path in [
            "GOG Games/Redguard",
            "GOG Games/Redguard/app/Redguard",
            "Program Files (x86)/GOG Galaxy/Games/Redguard",
            "Program Files (x86)/GOG Galaxy/Games/Redguard/app/Redguard",
            "Program Files/GOG Galaxy/Games/Redguard",
        ] {
            folders.push(drive.join(path));
        }
    }
    // Default install folders of Heroic and Lutris (`~/Games/...`) on Linux.
    if let Some(home) = &machine.home {
        for path in [
            "Games/Heroic/Redguard",
            "Games/gog/redguard",
            "Games/redguard",
        ] {
            folders.push(home.join(path));
        }
    }
    folders
}

/// Where Maiko stores the folder it found (`install.json` in its own config directory).
pub fn preference_files() -> Vec<PathBuf> {
    directories::ProjectDirs::from("", "", "maiko")
        .map(|dirs| dirs.config_dir().join("install.json"))
        .into_iter()
        .collect()
}

fn read_preference(file: &Path) -> Option<PathBuf> {
    serde_json::from_slice::<PathBuf>(&std::fs::read(file).ok()?).ok()
}

/// Find the game folder with everything known about the machine, in the documented order.
pub fn find(
    preferences: &[PathBuf],
    env: Option<PathBuf>,
    program_folder: &Path,
    machine: &Machine,
) -> Option<PathBuf> {
    let remembered = preferences.iter().filter_map(|file| read_preference(file));
    let from_env = env.into_iter();
    // The exact folders first, then the folders around the program, then the standard places.
    remembered
        .chain(from_env)
        .find_map(|path| resolve(&path))
        .or_else(|| is_game_folder(program_folder).then(|| program_folder.to_path_buf()))
        .or_else(|| resolve(program_folder))
        .or_else(|| {
            standard_folders(machine)
                .iter()
                .find_map(|path| resolve(path))
        })
}

/// Store the chosen folder for the next launch.
pub fn remember(game: &Path) -> std::io::Result<()> {
    let file = preference_files()
        .into_iter()
        .next()
        .ok_or_else(|| std::io::Error::other("no configuration folder on this system"))?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, serde_json::to_vec_pretty(game)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn game(at: &Path) -> PathBuf {
        for dir in ["fxart", "maps"] {
            fs::create_dir_all(at.join(dir)).unwrap();
        }
        fs::write(at.join("RGFX.EXE"), b"x").unwrap();
        at.to_path_buf()
    }

    #[test]
    fn original_cd_asset_names_are_found_on_case_sensitive_filesystems() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("FXART")).unwrap();
        std::fs::create_dir(root.path().join("MAPS")).unwrap();
        std::fs::write(root.path().join("rgfx.exe"), b"x").unwrap();
        assert!(is_game_folder(root.path()));
        assert_eq!(resolve(root.path()), Some(root.path().to_path_buf()));
    }

    #[test]
    fn a_picked_folder_resolves_to_the_enclosing_game_or_a_neighbouring_layout() {
        let root = tempfile::tempdir().unwrap();
        let outer = root.path().join("GOG Games/Redguard");
        let inner = game(&outer.join("app/Redguard"));
        assert_eq!(resolve(&inner), Some(inner.clone()));
        assert_eq!(
            resolve(&inner.join("fxart")),
            Some(inner.clone()),
            "a subfolder"
        );
        assert_eq!(
            resolve(&outer),
            Some(inner.clone()),
            "the GOG folder around it"
        );
        assert_eq!(resolve(&root.path().join("GOG Games")), Some(inner.clone()));
        assert_eq!(resolve(&root.path().join("missing")), None);
        let empty = root.path().join("empty");
        fs::create_dir(&empty).unwrap();
        assert_eq!(resolve(&empty), None);
    }

    #[test]
    fn the_enclosing_install_wins_over_a_neighbouring_package() {
        let root = tempfile::tempdir().unwrap();
        let outer = game(&root.path().join("Redguard"));
        game(&root.path().join("Redguard/Redguard"));
        assert_eq!(resolve(&outer.join("maps")), Some(outer));
    }

    #[test]
    fn standard_folders_include_the_gog_and_galaxy_folders() {
        let machine = Machine {
            home: Some("/home/p".into()),
            system_drive: Some("C:".into()),
            program_files: vec!["C:/Program Files (x86)".into()],
            wine_prefix: Some("/home/p/.wine".into()),
        };
        let folders = standard_folders(&machine);
        for expected in [
            "redguard",
            "Redguard",
            "/home/p/GOG Games/Redguard",
            "/home/p/GOG Games/Redguard/app/Redguard",
            "C:/GOG Games/Redguard",
            "C:/Program Files (x86)/GOG Galaxy/Games/Redguard",
            "/home/p/.wine/drive_c/GOG Games/Redguard",
            "/home/p/.wine/drive_c/Program Files (x86)/GOG Galaxy/Games/Redguard",
            "/home/p/Games/Heroic/Redguard",
            "/home/p/Games/gog/redguard",
        ] {
            assert!(folders.contains(&PathBuf::from(expected)), "{expected}");
        }
    }

    #[test]
    fn search_order_is_remembered_then_environment_then_program_then_standard() {
        let root = tempfile::tempdir().unwrap();
        let remembered_game = game(&root.path().join("remembered"));
        let env_game = game(&root.path().join("env"));
        let program = root.path().join("maiko");
        fs::create_dir(&program).unwrap();
        let beside = game(&root.path().join("Redguard"));
        let standard_home = root.path().join("home");
        let standard = game(&standard_home.join("GOG Games/Redguard"));
        let machine = Machine {
            home: Some(standard_home),
            ..Machine::default()
        };
        let preference = root.path().join("install.json");
        fs::write(&preference, serde_json::to_vec(&remembered_game).unwrap()).unwrap();
        let find = |preferences: &[PathBuf], env: Option<PathBuf>| {
            find(preferences, env, &program, &machine)
        };
        assert_eq!(
            find(std::slice::from_ref(&preference), Some(env_game.clone())),
            Some(remembered_game)
        );
        assert_eq!(find(&[], Some(env_game.clone())), Some(env_game));
        // A remembered folder that has gone away is skipped, not an error.
        fs::write(
            &preference,
            serde_json::to_vec(&root.path().join("gone")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            find(&[preference], None),
            Some(beside),
            "the folder beside the program"
        );
        fs::remove_dir_all(root.path().join("Redguard")).unwrap();
        assert_eq!(find(&[], None), Some(standard), "a standard location");
    }

    #[test]
    fn the_program_folder_itself_wins_over_the_folders_around_it() {
        let root = tempfile::tempdir().unwrap();
        let program = game(&root.path().join("Redguard"));
        game(&root.path().join("Redguard/Redguard"));
        assert_eq!(
            find(&[], None, &program, &Machine::default()),
            Some(program)
        );
    }
}
