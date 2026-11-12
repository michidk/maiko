//! Installation selection and exact original-executable validation on both hosts.
use crate::{media::rg_cd_mount, storage};
use sha2::{Digest, Sha256};
use std::ffi::{OsString, c_char, c_int};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Picker = unsafe extern "C" fn(*mut c_char, usize) -> c_int;
static PICKER: Mutex<Option<Picker>> = Mutex::new(None);
#[unsafe(no_mangle)]
pub static mut rg_exe_variant: u32 = 0;
const PRIMARY: &str = "844a316b5f71e3c7fadf4b012ab63c5c13aa3606a4f1504f852ac1607ee4dfdf";
const ALTERNATE: &str = "06d7a399cf01eec6bca3be39aaab1a39c42dc88116b6b74ae6f5f0547f7cdaf3";

#[derive(Default)]
struct Selection {
    game: Option<PathBuf>,
    user: Option<PathBuf>,
    media: Option<PathBuf>,
    choose: bool,
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
fn choose() -> io::Result<PathBuf> {
    let picker = PICKER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or(crate::dialog::rg_choose_game_directory);
    let mut buffer = [0; 4096];
    if unsafe { picker(buffer.as_mut_ptr(), buffer.len()) } == 0 {
        return Err(storage::denied());
    }
    unsafe { storage::path_from_c(buffer.as_ptr()) }
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
            eprintln!("Unsupported RGFX.EXE build: {hex}; supported {PRIMARY} or {ALTERNATE}");
            return Err(storage::denied());
        }
    };
    unsafe {
        rg_exe_variant = variant;
    }
    eprintln!("Recognized RGFX.EXE variant={variant} sha256={hex}");
    Ok(())
}
fn select(selection: Selection) -> io::Result<()> {
    let game = if let Some(game) = selection.game {
        game
    } else if selection.choose {
        choose()?
    } else {
        let executable = std::env::current_exe()?;
        let directory = executable.parent().ok_or_else(storage::absent)?;
        if directory.join("RGFX.EXE").exists() {
            directory.to_path_buf()
        } else {
            choose()?
        }
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
    if let Some(user) = selection.user {
        storage::STORAGE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
            .ok_or_else(storage::absent)?
            .initialize_user(&user)?;
    }
    if let Some(media) = &selection.media {
        let mut path = [0; 4096];
        unsafe {
            storage::copy_path(media, path.as_mut_ptr(), path.len())?;
        }
        if unsafe { rg_cd_mount(path.as_ptr()) } == 0 {
            return Err(storage::denied());
        }
    }
    std::env::set_current_dir(&game)?;
    if selection.media.is_none()
        && unsafe { crate::installed_media::rg_gog_movies_mount(rg_cd_mount) } < 0
    {
        return Err(storage::denied());
    }
    eprintln!("Verified original game directory: {}", game.display());
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
        let selection=Selection::parse(args).inspect_err(|_| eprintln!("Usage: redguard-native [--game-dir GAME_DIRECTORY | --choose-game-dir] [--user-dir SAVE_DIRECTORY] [--cd-image GAME_GOG_OR_ISO]"))?;
        select(selection)
    })();
    if let Err(e) = &result {
        eprintln!(
            "Game image validation or installation selection failed before original code execution: {e}"
        );
    }
    result.is_ok() as c_int
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_reject_duplicates_conflicts_and_missing_values() {
        for args in [
            vec!["--game-dir"],
            vec!["--bad"],
            vec!["--game-dir", "a", "--game-dir", "b"],
            vec!["--game-dir", "a", "--choose-game-dir"],
        ] {
            assert!(Selection::parse(args.into_iter().map(OsString::from)).is_err());
        }
        let selected =
            Selection::parse(["--game-dir", "a b", "--user-dir", "saves"].map(OsString::from))
                .unwrap();
        assert_eq!(selected.game, Some(PathBuf::from("a b")));
    }
}
