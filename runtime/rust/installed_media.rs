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
/// A disc image and the cue sheet that gives its audio tracks.
static AUDIO: Mutex<Option<(PathBuf, PathBuf)>> = Mutex::new(None);

pub(crate) fn audio() -> Option<(PathBuf, PathBuf)> {
    AUDIO.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The movie `leaf` on the selected disc folder, if there is one.
pub(crate) fn disc_file(leaf: &str) -> Option<PathBuf> {
    let folder = FOLDER.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
    named(&folder, &[leaf])
}

/// The file in `directory` with one of `names`, ignoring case.
fn named(directory: &Path, names: &[&str]) -> Option<PathBuf> {
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

#[derive(Debug, Default, PartialEq)]
struct Choice {
    image: Option<PathBuf>,
    folder: Option<PathBuf>,
    audio: Option<(PathBuf, PathBuf)>,
}

/// What `--cd-image PATH` names: a folder with the disc's files, a cue sheet and the image it names,
/// or an image with an optional cue sheet beside it (`NAME.cue`, or `game.ins` for `game.gog`).
fn explicit(path: &Path) -> io::Result<Choice> {
    if path.is_dir() {
        return Ok(Choice {
            folder: Some(path.to_path_buf()),
            ..Choice::default()
        });
    }
    let directory = path.parent().unwrap_or(Path::new("."));
    let is_cue = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cue") || e.eq_ignore_ascii_case("ins"));
    if is_cue {
        let text = fs::read_to_string(path)?;
        let image = crate::redbook::cue_file(&text)
            .and_then(|name| named(directory, &[name]))
            .ok_or_else(|| {
                rg_log!(
                    "CD cue sheet {} must name one image file beside it (rips with a file per track are not supported)",
                    path.display()
                );
                io::Error::from(io::ErrorKind::InvalidData)
            })?;
        let audio = Some((image.clone(), path.to_path_buf()));
        return Ok(Choice {
            image: Some(image),
            audio,
            ..Choice::default()
        });
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let cue = named(directory, &[&format!("{stem}.cue"), &format!("{stem}.ins")]);
    Ok(Choice {
        image: Some(path.to_path_buf()),
        audio: cue.map(|cue| (path.to_path_buf(), cue)),
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
                let cue = named(&directory, &["game.ins", "game.cue"]);
                return Ok(Some(Choice {
                    image: Some(image.clone()),
                    audio: cue.map(|cue| (image.clone(), cue)),
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

/// Places the Redguard disc may be: the drive letter inside a Wine prefix that holds the game, then
/// the host's own drive or mount points.
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
        Some((image, cue)) => rg_log!(
            "CD music: disc image {} with cue sheet {}",
            image.display(),
            cue.display()
        ),
        None if choice.image.is_some() || choice.folder.is_some() => rg_log!(
            "CD music unavailable: the music is on the disc's audio tracks, which need a disc image with its cue sheet (--cd-image FILE.cue); a mounted disc, a folder or an ISO holds only its files"
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

    #[test]
    fn finds_bundled_gog_data_beside_the_installed_game() {
        let install = tempfile::tempdir().unwrap();
        let root = install.path().join("Redguard");
        fs::create_dir(&root).unwrap();
        fs::write(install.path().join("game.gog"), b"image").unwrap();
        fs::write(install.path().join("game.ins"), b"cue").unwrap();
        let choice = choose(None, &root, "").unwrap();
        let image = root.join("../game.gog");
        assert_eq!(choice.image, Some(image.clone()));
        assert_eq!(choice.audio, Some((image, root.join("../game.ins"))));
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
    fn an_explicit_cue_sheet_mounts_the_image_it_names_and_gives_music() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("REDGUARD.BIN"), b"image").unwrap();
        fs::write(
            dir.path().join("redguard.cue"),
            "FILE \"Redguard.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n",
        )
        .unwrap();
        let cue = dir.path().join("redguard.cue");
        let choice = explicit(&cue).unwrap();
        let image = dir.path().join("REDGUARD.BIN");
        assert_eq!(choice.image, Some(image.clone()));
        assert_eq!(choice.audio, Some((image.clone(), cue.clone())));
        // The image itself finds its cue sheet beside it.
        assert_eq!(explicit(&image).unwrap().audio, Some((image, cue)));
        // A cue sheet naming a missing file, or one file per track, is refused.
        fs::write(
            dir.path().join("other.cue"),
            "FILE \"missing.bin\" BINARY\n",
        )
        .unwrap();
        assert!(explicit(&dir.path().join("other.cue")).is_err());
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
