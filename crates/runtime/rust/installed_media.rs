//! Where the movies and CD music come from. The original reads both from the game disc: movies as
//! files on the drive named by `animation_drive` in SYSTEM.INI, music as its audio tracks. In order:
//! `--cd-image` (a disc image, a cue sheet, or a folder holding the disc's files), the GOG
//! `game.gog` with its `game.ins` cue sheet, then for the original CD release the Redguard disc in a
//! drive. Loose movie files in the game folder are found by `storage.rs` in every case.
use crate::rg_log;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A folder with the disc's files (a mounted disc or a copy of it), searched for loose movies.
static FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);
/// The cue sheet that gives the disc's audio tracks.
static AUDIO: Mutex<Option<PathBuf>> = Mutex::new(None);

pub(crate) fn audio() -> Option<PathBuf> {
    AUDIO.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The movie `leaf` on the selected disc folder, if there is one.
pub(crate) fn disc_file(leaf: &str) -> Option<PathBuf> {
    let folder = FOLDER.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
    named(&folder, &[leaf])
}

/// The file in `directory` with one of `names`, ignoring case.
pub(crate) fn named(directory: &Path, names: &[&str]) -> Option<PathBuf> {
    fs::read_dir(directory)
        .ok()?
        .flatten()
        .find(|entry| {
            let name = entry.file_name();
            names
                .iter()
                .any(|n| name.to_str().is_some_and(|s| s.eq_ignore_ascii_case(n)))
                && entry.path().is_file()
        })
        .map(|entry| entry.path())
}

/// The cue sheet files in `directory` (`.cue`, or the GOG `.ins`) that parse, in name order, with
/// the image files each names.
fn cue_sheets(directory: &Path) -> Vec<(PathBuf, Vec<String>)> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut sheets: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("cue") || e.eq_ignore_ascii_case("ins"))
                // A cue sheet is a few kilobytes; anything large is not one.
                && path.metadata().is_ok_and(|m| m.is_file() && m.len() < 1 << 20)
        })
        .filter_map(|path| {
            let files = crate::redbook::parse_cue(&fs::read_to_string(&path).ok()?)?;
            Some((path, files.into_iter().map(|(name, _)| name).collect()))
        })
        .collect();
    sheets.sort();
    sheets
}

/// A cue sheet in `directory` whose image files are all there and, when given, include `image`.
fn cue_for(directory: &Path, image: Option<&str>) -> Option<PathBuf> {
    cue_sheets(directory)
        .into_iter()
        .find(|(_, files)| {
            files.iter().all(|name| named(directory, &[name]).is_some())
                && image.is_none_or(|image| files.iter().any(|n| n.eq_ignore_ascii_case(image)))
        })
        .map(|(cue, _)| cue)
}

#[derive(Debug, Default, PartialEq)]
struct Choice {
    image: Option<PathBuf>,
    folder: Option<PathBuf>,
    audio: Option<PathBuf>,
}

/// What `--cd-image PATH` names: a folder with the disc's files (and perhaps a rip of it), a cue
/// sheet and the data track it names first, or an image with a cue sheet beside it that names it.
fn explicit(path: &Path) -> io::Result<Choice> {
    if path.is_dir() {
        return Ok(Choice {
            folder: Some(path.to_path_buf()),
            audio: cue_for(path, None),
            ..Choice::default()
        });
    }
    let directory = path.parent().unwrap_or(Path::new("."));
    let is_cue = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cue") || e.eq_ignore_ascii_case("ins"));
    if is_cue {
        let text = fs::read_to_string(path)?;
        let files = crate::redbook::parse_cue(&text).unwrap_or_default();
        let missing = files
            .iter()
            .find(|(name, _)| named(directory, &[name]).is_none());
        if files.is_empty() || missing.is_some() {
            rg_log!(
                "CD cue sheet {} is unusable: {}",
                path.display(),
                missing.map_or("no tracks".into(), |(name, _)| format!(
                    "{name} is not beside it"
                ))
            );
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        return Ok(Choice {
            image: named(directory, &[&files[0].0]),
            audio: Some(path.to_path_buf()),
            ..Choice::default()
        });
    }
    let name = path.file_name().and_then(|n| n.to_str());
    Ok(Choice {
        image: Some(path.to_path_buf()),
        audio: name.and_then(|name| cue_for(directory, Some(name))),
        ..Choice::default()
    })
}

/// The GOG `game.gog` and its cue sheet in the game folder or the one above it.
fn gog(game: &Path) -> io::Result<Option<Choice>> {
    for directory in [game.to_path_buf(), game.join("..")] {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        let images: Vec<_> = entries
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case("game.gog"))
                    && e.path().is_file()
            })
            .map(|e| e.path())
            .collect();
        match images.as_slice() {
            [] => continue,
            [image] => {
                rg_log!(
                    "GOG movies: using bundled installed data {}",
                    image.display()
                );
                let name = image.file_name().and_then(|n| n.to_str());
                return Ok(Some(Choice {
                    image: Some(image.clone()),
                    audio: cue_for(&directory, name),
                    ..Choice::default()
                }));
            }
            _ => {
                rg_log!("Ambiguous GOG disc images in {}", directory.display());
                return Err(io::Error::from(io::ErrorKind::InvalidData));
            }
        }
    }
    Ok(None)
}

/// The drive letter of `animation_drive=` in SYSTEM.INI text, `D` when it is absent.
fn animation_drive(system_ini: &str) -> u8 {
    system_ini
        .lines()
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case("animation_drive"))
        .and_then(|(_, value)| value.trim().bytes().next())
        .filter(u8::is_ascii_alphabetic)
        .map_or(b'D', |letter| letter.to_ascii_uppercase())
}

/// Places the Redguard disc may be: the drive letter inside a Wine prefix that holds the game, a
/// copy of the disc in a folder inside the game folder (DOSBox setups mount one as the CD drive),
/// then the host's own drive or mount points.
fn disc_candidates(letter: u8, game: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(prefix) = game
        .ancestors()
        .find(|a| {
            a.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("drive_c"))
        })
        .and_then(Path::parent)
    {
        let letter = letter.to_ascii_lowercase() as char;
        candidates.push(prefix.join(format!("dosdevices/{letter}:")));
    }
    let mut inside: Vec<_> = fs::read_dir(game)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default();
    inside.sort();
    candidates.extend(inside);
    candidates.extend(crate::os::disc_roots(letter));
    candidates
}

/// The first candidate that holds the disc's intro movie.
fn find_disc(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|folder| named(folder, &["INTRO.SMK"]).is_some())
        .cloned()
}

fn choose(explicit_path: Option<&Path>, game: &Path, system_ini: &str) -> io::Result<Choice> {
    if let Some(path) = explicit_path {
        return explicit(path);
    }
    if let Some(choice) = gog(game)? {
        return Ok(choice);
    }
    let letter = animation_drive(system_ini);
    let candidates = disc_candidates(letter, game);
    let folder = find_disc(&candidates);
    if folder.is_none() {
        rg_log!(
            "No Redguard disc found for the CD drive {}: (looked in {}). Movies play only from loose .SMK files in the game folder; to use the disc, insert and mount it or pass --cd-image with a disc image, its cue sheet or a folder holding its files",
            letter as char,
            candidates
                .iter()
                .map(|c| c.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(Choice {
        audio: folder.as_deref().and_then(|folder| cue_for(folder, None)),
        folder,
        ..Choice::default()
    })
}

/// Choose and mount the movie and music source. Call after selecting the game and user folders,
/// before game threads start. An explicit `--cd-image` that cannot be used is an error.
pub(crate) fn select(
    explicit_path: Option<&Path>,
    game: &Path,
    system_ini: &str,
) -> io::Result<()> {
    let choice = choose(explicit_path, game, system_ini)?;
    if let Some(image) = &choice.image
        && !crate::media::mount(image)
    {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }
    if let Some(folder) = &choice.folder {
        rg_log!(
            "CD movies: reading the disc's files in {}",
            folder.display()
        );
    }
    match &choice.audio {
        Some(cue) => rg_log!("CD music: disc image from the cue sheet {}", cue.display()),
        None if choice.image.is_some() || choice.folder.is_some() => rg_log!(
            "CD music unavailable: the music is on the disc's audio tracks, which need a disc image with its cue sheet beside the disc's files or given with --cd-image FILE.cue; a mounted disc or an ISO holds only its files"
        ),
        None => {}
    }
    *FOLDER.lock().unwrap_or_else(|e| e.into_inner()) = choice.folder;
    *AUDIO.lock().unwrap_or_else(|e| e.into_inner()) = choice.audio;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_TRACK: &str = "  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n";

    #[test]
    fn finds_bundled_gog_data_beside_the_installed_game() {
        let install = tempfile::tempdir().unwrap();
        let root = install.path().join("Redguard");
        fs::create_dir(&root).unwrap();
        fs::write(install.path().join("game.gog"), b"image").unwrap();
        let cue = format!("FILE \"game.gog\" BINARY\n{ONE_TRACK}");
        fs::write(install.path().join("game.ins"), cue).unwrap();
        let choice = choose(None, &root, "").unwrap();
        assert_eq!(choice.image, Some(root.join("../game.gog")));
        assert_eq!(choice.audio, Some(root.join("../game.ins")));
    }

    #[test]
    fn flat_install_and_case_handling_prefer_local_data() {
        let install = tempfile::tempdir().unwrap();
        let root = install.path().join("Redguard");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("GAME.GOG"), b"local").unwrap();
        fs::write(install.path().join("game.gog"), b"parent").unwrap();
        let choice = choose(None, &root, "").unwrap();
        assert_eq!(choice.image, Some(root.join("GAME.GOG")));
        assert_eq!(choice.audio, None);
    }

    #[test]
    fn directories_are_not_disc_images() {
        let install = tempfile::tempdir().unwrap();
        let root = install.path().join("Redguard");
        fs::create_dir_all(root.join("game.gog")).unwrap();
        assert_eq!(gog(&root).unwrap(), None);
    }

    #[test]
    fn an_explicit_cue_sheet_mounts_its_data_track_and_gives_music() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("REDGUARD (TRACK 1).BIN"), b"data").unwrap();
        fs::write(dir.path().join("Redguard (Track 2).bin"), b"audio").unwrap();
        let cue = dir.path().join("Redguard.cue");
        fs::write(
            &cue,
            format!("FILE \"Redguard (Track 1).bin\" BINARY\n{ONE_TRACK}FILE \"Redguard (Track 2).bin\" BINARY\n  TRACK 02 AUDIO\n    INDEX 01 00:00:00\n"),
        )
        .unwrap();
        let choice = explicit(&cue).unwrap();
        let data = dir.path().join("REDGUARD (TRACK 1).BIN");
        assert_eq!(choice.image, Some(data.clone()));
        assert_eq!(choice.audio, Some(cue.clone()));
        // The data track itself finds the cue sheet that names it.
        assert_eq!(explicit(&data).unwrap().audio, Some(cue.clone()));
        // A cue sheet naming a missing file is refused, and not offered for music.
        let other = dir.path().join("other.cue");
        fs::write(&other, format!("FILE \"missing.bin\" BINARY\n{ONE_TRACK}")).unwrap();
        assert!(explicit(&other).is_err());
        assert_eq!(cue_for(dir.path(), None), Some(cue));
    }

    #[test]
    fn an_explicit_folder_holds_the_disc_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("intro.smk"), b"movie").unwrap();
        let choice = explicit(dir.path()).unwrap();
        assert_eq!(choice.folder.as_deref(), Some(dir.path()));
        assert_eq!(choice.image, None);
        assert_eq!(choice.audio, None);
        // An explicit choice wins over a GOG image beside the game.
        fs::write(dir.path().join("game.gog"), b"image").unwrap();
        assert_eq!(
            choose(Some(dir.path()), dir.path(), "").unwrap().image,
            None
        );
    }

    #[test]
    fn a_copy_of_the_disc_inside_the_game_folder_gives_movies_and_music() {
        // A DOSBox setup of the CD release: the disc copied to PlayCD and mounted as E:.
        let game = tempfile::tempdir().unwrap();
        let disc = game.path().join("PlayCD");
        fs::create_dir_all(game.path().join("fxart")).unwrap();
        fs::create_dir(&disc).unwrap();
        fs::write(disc.join("Intro.smk"), b"movie").unwrap();
        fs::write(disc.join("Track 1.bin"), b"data").unwrap();
        let cue = disc.join("Play Disc.cue");
        fs::write(&cue, format!("FILE \"Track 1.bin\" BINARY\n{ONE_TRACK}")).unwrap();
        let choice = choose(None, game.path(), "animation_drive=e:\\\n").unwrap();
        assert_eq!(choice.folder, Some(disc));
        assert_eq!(choice.audio, Some(cue));
        assert_eq!(choice.image, None);
    }

    #[test]
    fn the_cd_drive_letter_comes_from_system_ini() {
        assert_eq!(animation_drive(""), b'D');
        assert_eq!(animation_drive("[system]\nanimation_drive=E:\\\n"), b'E');
        assert_eq!(animation_drive("Animation_Drive = f:\\\r\n"), b'F');
        assert_eq!(animation_drive("animation_drive=\n"), b'D');
    }

    #[test]
    fn a_cd_install_in_a_wine_prefix_finds_the_disc_through_its_drive_letter() {
        let prefix = tempfile::tempdir().unwrap();
        let game = prefix.path().join("drive_c/REDGUARD");
        let disc = prefix.path().join("disc");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(prefix.path().join("dosdevices")).unwrap();
        fs::create_dir(&disc).unwrap();
        let candidates = disc_candidates(b'E', &game);
        assert_eq!(candidates[0], prefix.path().join("dosdevices/e:"));
        assert_eq!(find_disc(&candidates[..1]), None);
        fs::rename(&disc, &candidates[0]).unwrap();
        assert_eq!(find_disc(&candidates[..1]), None);
        fs::write(candidates[0].join("Intro.Smk"), b"movie").unwrap();
        assert_eq!(find_disc(&candidates[..1]), Some(candidates[0].clone()));
        let choice = choose(None, &game, "animation_drive=E:\\\n").unwrap();
        assert_eq!(choice.folder, Some(candidates[0].clone()));
        assert_eq!(choice.image, None);
    }
}
