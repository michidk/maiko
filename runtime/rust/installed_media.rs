use crate::rg_log;
use std::ffi::{CString, c_char, c_int};
use std::fs;
use std::path::Path;

fn mount_installed(root: &Path, mut mount: impl FnMut(&Path) -> bool) -> i32 {
    for directory in [root.to_path_buf(), root.join("..")] {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        let mut candidates = entries.filter_map(Result::ok).filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.eq_ignore_ascii_case("game.gog"))
                && entry.path().is_file()
        });
        if let Some(candidate) = candidates.next() {
            if candidates.next().is_some() {
                rg_log!(
                    "Ambiguous installed GOG movie data in {}",
                    directory.display()
                );
                return -1;
            }
            let path = candidate.path();
            if mount(&path) {
                rg_log!(
                    "GOG movies: using bundled installed data {}",
                    path.display()
                );
                return 1;
            }
            return -1;
        }
    }
    0
}

/// # Safety
/// Call after selecting the game working directory, before game threads start.
/// `mount` accepts a temporary NUL-terminated path and opens it read-only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_gog_movies_mount(
    mount: unsafe extern "C" fn(*const c_char) -> c_int,
) -> c_int {
    mount_installed(Path::new("."), |path| {
        // Only ASCII names from the game directory or its parent are passed to
        // the legacy fopen boundary, preserving native non-UTF8 root paths.
        let Some(path) = path.to_str().and_then(|p| CString::new(p).ok()) else {
            return false;
        };
        unsafe { mount(path.as_ptr()) != 0 }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_bundled_gog_data_beside_the_installed_game() {
        let install = tempfile::tempdir().unwrap();
        let root = install.path().join("Redguard");
        fs::create_dir(&root).unwrap();
        fs::write(install.path().join("game.gog"), b"installed movie fixture").unwrap();
        let mut opened = Vec::new();
        assert_eq!(
            mount_installed(&root, |path| {
                opened.push(path.to_path_buf());
                fs::read(path).unwrap() == b"installed movie fixture"
            }),
            1
        );
        assert_eq!(opened, [root.join("../game.gog")]);
    }

    #[test]
    fn flat_install_and_case_handling_prefer_local_data() {
        let install = tempfile::tempdir().unwrap();
        let root = install.path().join("Redguard");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("GAME.GOG"), b"local").unwrap();
        fs::write(install.path().join("game.gog"), b"parent").unwrap();
        let mut opened = Vec::new();
        assert_eq!(
            mount_installed(&root, |path| {
                opened.push(path.to_path_buf());
                true
            }),
            1
        );
        assert_eq!(opened, [root.join("GAME.GOG")]);
    }

    #[test]
    fn absent_data_preserves_loose_movies_and_invalid_data_is_reported() {
        let install = tempfile::tempdir().unwrap();
        let root = install.path().join("Redguard");
        fs::create_dir_all(root.join("game.gog")).unwrap();
        assert_eq!(
            mount_installed(&root, |_| panic!("directories are not movie data")),
            0
        );
        fs::write(install.path().join("game.gog"), b"invalid").unwrap();
        assert_eq!(mount_installed(&root, |_| false), -1);
    }
}
