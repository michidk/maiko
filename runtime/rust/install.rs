//! Installation selection and exact original-executable validation on native hosts.
use crate::rg_log;
use crate::storage;
use sha2::{Digest, Sha256};
use std::ffi::{OsString, c_char, c_int};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Picker = unsafe extern "C" fn(*mut c_char, usize) -> c_int;
static PICKER: Mutex<Option<Picker>> = Mutex::new(None);
#[unsafe(no_mangle)]
pub static mut rg_exe_variant: u32 = 0;
// Read by the translated code: RG_SKIP_JUMP and the startup book (tools/pipeline.py).
#[unsafe(no_mangle)]
pub static mut rg_skip_intro: u32 = 0;
#[unsafe(no_mangle)]
pub static mut rg_skip_outro: u32 = 0;
/// Save slot to load instead of showing the startup book, or -1.
#[unsafe(no_mangle)]
pub static mut rg_load_save: i32 = -1;
const PRIMARY: &str = "844a316b5f71e3c7fadf4b012ab63c5c13aa3606a4f1504f852ac1607ee4dfdf";
const ALTERNATE: &str = "06d7a399cf01eec6bca3be39aaab1a39c42dc88116b6b74ae6f5f0547f7cdaf3";

#[derive(Default)]
struct Selection {
    game: Option<PathBuf>,
    user: Option<PathBuf>,
    media: Option<PathBuf>,
    choose: bool,
    skip_intro: bool,
    skip_outro: bool,
    load_save: Option<u16>,
}
impl Selection {
    fn parse(args: impl IntoIterator<Item = OsString>) -> io::Result<Self> {
        let mut args = args.into_iter();
        let mut result = Self::default();
        while let Some(arg) = args.next() {
            let slot = match arg.to_str().unwrap_or("") {
                "--game-dir" => &mut result.game,
                "--user-dir" => &mut result.user,
                "--cd-image" => &mut result.media,
                "--choose-game-dir" if !result.choose => {
                    result.choose = true;
                    continue;
                }
                "--fullscreen" => {
                    crate::input::set_display(true, None);
                    continue;
                }
                "--window-size" => {
                    let size = args
                        .next()
                        .as_deref()
                        .and_then(|a| a.to_str())
                        .and_then(crate::input::parse_size)
                        .ok_or_else(storage::denied)?;
                    crate::input::set_display(false, Some(size));
                    continue;
                }
                "--skip-intro" if !result.skip_intro => {
                    result.skip_intro = true;
                    continue;
                }
                "--skip-outro" if !result.skip_outro => {
                    result.skip_outro = true;
                    continue;
                }
                "--load-save" if result.load_save.is_none() => {
                    // The original clamps slots to 0..=999 (SAVEGAME.000 to SAVEGAME.999).
                    let slot = args.next().and_then(|slot| slot.to_str()?.parse().ok());
                    result.load_save = Some(
                        slot.filter(|&slot| slot <= 999)
                            .ok_or_else(storage::denied)?,
                    );
                    continue;
                }
                _ => return Err(storage::denied()),
            };
            if slot.is_some() {
                return Err(storage::denied());
            }
            *slot = Some(PathBuf::from(args.next().ok_or_else(storage::denied)?));
        }
        if result.choose && result.game.is_some() {
            return Err(storage::denied());
        }
        Ok(result)
    }
}
/// Where saves and settings go when the game is started without `--user-dir`: beside the program, or
/// in the user's data folder when the program sits inside the game folder, where the game's own files
/// must stay untouched.
fn default_user_dir(game: &Path) -> io::Result<PathBuf> {
    let beside = std::env::current_exe()?
        .parent()
        .ok_or_else(storage::absent)?
        .join("user-data");
    if !beside.starts_with(game) {
        return Ok(beside);
    }
    let data = directories::ProjectDirs::from("", "", "maiko")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .ok_or_else(storage::absent)?;
    // The storage layer creates the saves folder but not the folders above it.
    std::fs::create_dir_all(&data)?;
    Ok(data.join("user-data"))
}

/// Ask the user, then accept the game folder, a folder inside it or the GOG folder around it.
fn choose() -> io::Result<PathBuf> {
    let picker = PICKER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or(crate::dialog::rg_choose_game_directory);
    let mut buffer = [0; 4096];
    if unsafe { picker(buffer.as_mut_ptr(), buffer.len()) } == 0 {
        return Err(storage::denied());
    }
    let picked = unsafe { storage::path_from_c(buffer.as_ptr()) }?;
    crate::locate::resolve(&picked).ok_or_else(storage::denied)
}
fn validate(path: &Path) -> io::Result<()> {
    unsafe {
        rg_exe_variant = 0;
    }
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 16384];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    let hex = digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let variant = match hex.as_str() {
        PRIMARY => 0,
        ALTERNATE => 1,
        _ => {
            rg_log!("Unsupported RGFX.EXE build: {hex}; supported {PRIMARY} or {ALTERNATE}");
            return Err(storage::denied());
        }
    };
    unsafe {
        rg_exe_variant = variant;
    }
    rg_log!("Recognized RGFX.EXE variant={variant} sha256={hex}");
    Ok(())
}
fn adjacent_game(directory: &Path) -> Option<PathBuf> {
    [directory.join("game"), directory.to_path_buf()]
        .into_iter()
        .find(|candidate| candidate.join("RGFX.EXE").is_file())
}

fn select(selection: Selection) -> io::Result<()> {
    // Without --game-dir the game is started by double-click, so nothing is asked unless it has to
    // be: adjacent installation, remembered folder, REDGUARD_DIR, neighbouring folders, standard
    // install folders, then a question (see locate.rs). The answer is stored at the end.
    let interactive = selection.game.is_none();
    let game = if let Some(game) = selection.game {
        game
    } else if selection.choose {
        choose()?
    } else {
        let executable = std::env::current_exe()?;
        let program = executable.parent().ok_or_else(storage::absent)?;
        let found = adjacent_game(program)
            .or_else(crate::os::default_game)
            .or_else(|| {
                crate::locate::find(
                    &crate::locate::preference_files(),
                    std::env::var_os(crate::locate::ENV_VAR).map(PathBuf::from),
                    program,
                    &crate::locate::Machine::current(),
                )
            });
        match found {
            Some(folder) => {
                rg_log!("Game folder found without asking: {}", folder.display());
                folder
            }
            None => choose()?,
        }
    };
    let user = match (&selection.user, interactive) {
        (Some(user), _) => Some(user.clone()),
        (None, true) => {
            crate::os::default_user_directory().or_else(|| default_user_dir(&game).ok())
        }
        (None, false) => crate::os::default_user_directory(),
    };
    // Resolve the game root and its case-insensitive asset names before making
    // it current. Writes remain confined to the independently opened user root.
    storage::select_game(&game)?;
    let (image, game) = {
        let guard = storage::STORAGE.lock().unwrap_or_else(|e| e.into_inner());
        let root = guard.as_ref().ok_or_else(storage::absent)?;
        (root.resolved("C:\\RGFX.EXE")?, root.game.path.clone())
    };
    validate(&image)?;
    if let Some(user) = &user {
        storage::STORAGE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
            .ok_or_else(storage::absent)?
            .initialize_user(user)?;
    }
    if interactive && let Err(error) = crate::locate::remember(&game) {
        rg_log!("Could not remember the game folder: {error}");
    }
    if let Err(error) = crate::os::remember_game(&game) {
        rg_log!("event=installation-cache-write-failed error={error}");
    }
    if let Some(slot) = selection.load_save {
        let save = storage::STORAGE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .ok_or_else(storage::absent)?
            .resolved(&format!("C:\\SAVEGAME\\SAVEGAME.{slot:03}\\SAVEGAME.SAV"));
        if !save.is_ok_and(|save| save.is_file()) {
            rg_log!("Save slot {slot} not found: SAVEGAME\\SAVEGAME.{slot:03}\\SAVEGAME.SAV");
            return Err(storage::absent());
        }
    }
    // Variant 1 already skips the outro in its bytes (metadata/image_variants.json).
    unsafe {
        rg_skip_intro = u32::from(selection.skip_intro);
        rg_skip_outro = u32::from(selection.skip_outro || rg_exe_variant == 1);
        rg_load_save = selection.load_save.map_or(-1, i32::from);
    }
    // The installer writes the CD drive letter into SYSTEM.INI; the saves folder holds the copy in use.
    let system_ini = storage::STORAGE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|root| root.resolved("C:\\SYSTEM.INI").ok())
        .and_then(|path| std::fs::read(path).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    crate::installed_media::select(selection.media.as_deref(), &game, &system_ini)?;
    std::env::set_current_dir(&game)?;
    rg_log!("Verified original game directory: {}", game.display());
    Ok(())
}
/// # Safety
/// `picker`, when supplied, writes a NUL-terminated path within its capacity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_install_set_picker(picker: Option<Picker>) {
    *PICKER.lock().unwrap_or_else(|e| e.into_inner()) = picker;
}
/// # Safety
/// `argv` contains `argc` pointers to NUL-terminated arguments.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_install_select(argc: c_int, argv: *const *const c_char) -> c_int {
    if argc < 1 || argv.is_null() {
        return 0;
    }
    let args = unsafe { std::slice::from_raw_parts(argv, argc as usize) };
    let result = (|| {
        let args = args[1..]
            .iter()
            .map(|&arg| {
                if arg.is_null() {
                    return Err(storage::denied());
                }
                unsafe { storage::path_from_c(arg) }.map(PathBuf::into_os_string)
            })
            .collect::<io::Result<Vec<_>>>()?;
        let selection=Selection::parse(args).inspect_err(|_| rg_log!("Usage: redguard-native [--game-dir GAME_DIRECTORY | --choose-game-dir] [--user-dir SAVE_DIRECTORY] [--cd-image DISC_IMAGE_CUE_OR_FOLDER] [--fullscreen | --window-size WIDTHxHEIGHT] [--skip-intro] [--skip-outro] [--load-save SLOT]"))?;
        select(selection)
    })();
    if let Err(e) = &result {
        rg_log!(
            "Game image validation or installation selection failed before original code execution: {e}"
        );
        let explicit = args[1..].iter().any(|&a| {
            !a.is_null() && unsafe { std::ffi::CStr::from_ptr(a) }.to_bytes() == b"--game-dir"
        });
        if !explicit {
            crate::dialog::show_error(
                "The game folder could not be used, so Redguard did not start.\n\nChoose the inner Redguard folder that contains RGFX.EXE (the GOG release or the original CD). Only the two RGFX.EXE builds Maiko knows are supported.\n\nSee the session log for the detailed messages.",
            );
        }
    }
    result.is_ok() as c_int
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_game_is_found_before_adjacent_image_or_picker() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path();
        assert_eq!(adjacent_game(directory), None);
        std::fs::write(directory.join("RGFX.EXE"), b"adjacent").unwrap();
        assert_eq!(adjacent_game(directory), Some(directory.to_path_buf()));
        std::fs::create_dir(directory.join("game")).unwrap();
        std::fs::write(directory.join("game/RGFX.EXE"), b"bundled").unwrap();
        assert_eq!(adjacent_game(directory), Some(directory.join("game")));
        std::fs::remove_file(directory.join("game/RGFX.EXE")).unwrap();
        std::fs::create_dir(directory.join("game/RGFX.EXE")).unwrap();
        assert_eq!(adjacent_game(directory), Some(directory.to_path_buf()));
    }

    #[test]
    fn arguments_reject_duplicates_conflicts_and_missing_values() {
        for args in [
            vec!["--game-dir"],
            vec!["--bad"],
            vec!["--game-dir", "a", "--game-dir", "b"],
            vec!["--game-dir", "a", "--choose-game-dir"],
            vec!["--skip-intro", "--skip-intro"],
            vec!["--load-save"],
            vec!["--load-save", "1000"],
            vec!["--load-save", "-1"],
            vec!["--load-save", "1", "--load-save", "2"],
        ] {
            assert!(Selection::parse(args.into_iter().map(OsString::from)).is_err());
        }
        let selected =
            Selection::parse(["--game-dir", "a b", "--user-dir", "saves"].map(OsString::from))
                .unwrap();
        assert_eq!(selected.game, Some(PathBuf::from("a b")));
        let selected = Selection::parse(
            ["--skip-outro", "--load-save", "7", "--skip-intro"].map(OsString::from),
        )
        .unwrap();
        assert!(selected.skip_intro && selected.skip_outro);
        assert_eq!(selected.load_save, Some(7));
    }

    #[test]
    fn saves_default_to_a_folder_beside_the_program() {
        let user = default_user_dir(Path::new("/somewhere/else")).unwrap();
        assert_eq!(user.file_name().unwrap(), "user-data");
        assert_eq!(user.parent(), std::env::current_exe().unwrap().parent());
        // Inside the game folder the saves go to the user's data folder instead.
        let program = std::env::current_exe().unwrap();
        let moved = default_user_dir(program.parent().unwrap()).unwrap();
        assert!(!moved.starts_with(program.parent().unwrap()));
    }
}
