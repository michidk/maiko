//! Package only the staged Mac runtime; owned installations are never inputs.
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use zip::{ZipWriter, write::SimpleFileOptions};

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn inputs(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|_| invalid("Input outside runtime directory"))?;
            let first = relative.components().next().unwrap().as_os_str();
            if ![
                "redguard-native",
                "libredguard-glide.dylib",
                "Frameworks",
                "README.md",
                "LICENSES.txt",
            ]
            .iter()
            .any(|name| first == *name)
            {
                return Err(invalid(
                    "Runtime directory contains an unexpected file, game installation or launcher",
                ));
            }
            if first == "Frameworks" && relative.components().count() > 1 {
                let framework = relative.components().nth(1).unwrap().as_os_str();
                if framework != "SDL2.framework" && framework != "SDL2_mixer.framework" {
                    return Err(invalid("Unexpected runtime framework"));
                }
            }
            let kind = fs::symlink_metadata(&path)?.file_type();
            if kind.is_symlink() {
                let target = fs::read_link(&path)?;
                if target.is_absolute() || !path.canonicalize()?.starts_with(root) {
                    return Err(invalid("Framework symlink escapes the runtime directory"));
                }
                files.push(path);
            } else if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                files.push(path);
            } else {
                return Err(invalid("Unexpected runtime file type"));
            }
        }
    }
    for name in [
        "redguard-native",
        "libredguard-glide.dylib",
        "README.md",
        "LICENSES.txt",
        "Frameworks/SDL2.framework/SDL2",
        "Frameworks/SDL2_mixer.framework/SDL2_mixer",
    ] {
        if !root.join(name).is_file() {
            return Err(invalid(
                "Incomplete runtime: executable, renderer, frameworks and notices are required",
            ));
        }
    }
    files.sort();
    Ok(files)
}

pub fn package(source: &Path, output: &Path) -> io::Result<()> {
    let source = source.canonicalize()?;
    let files = inputs(&source)?;
    // Refuse to overwrite an existing download if validation or writing fails.
    let output_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let result = (|| {
        let mut archive = ZipWriter::new(output_file);
        for path in files {
            let relative = path.strip_prefix(&source).unwrap();
            let name = format!(
                "Maiko Mac Runtime/{}",
                relative.to_string_lossy().replace('\\', "/")
            );
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            if path.is_symlink() {
                archive.add_symlink(
                    name,
                    fs::read_link(&path)?.to_string_lossy(),
                    options.unix_permissions(0o777),
                )?;
            } else {
                #[cfg(unix)]
                let original_mode = {
                    use std::os::unix::fs::PermissionsExt;
                    path.metadata()?.permissions().mode() & 0o777
                };
                #[cfg(not(unix))]
                let original_mode = 0o644;
                let mode = if relative == Path::new("redguard-native") {
                    0o755
                } else {
                    original_mode
                };
                archive.start_file(name, options.unix_permissions(mode))?;
                io::copy(&mut File::open(path)?, &mut archive)?;
            }
        }
        archive.finish()?.flush()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn runtime(root: &Path) {
        for name in [
            "redguard-native",
            "libredguard-glide.dylib",
            "README.md",
            "LICENSES.txt",
            "Frameworks/SDL2.framework/SDL2",
            "Frameworks/SDL2_mixer.framework/SDL2_mixer",
        ] {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"fixture").unwrap();
        }
    }
    #[test]
    fn rejects_game_files_and_command_launchers_before_creating_archive() {
        for unexpected in ["game/RGFX.EXE", "RGFX.EXE", "game.gog", "RUN.command"] {
            let source = tempfile::tempdir().unwrap();
            runtime(source.path());
            let file = source.path().join(unexpected);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"not a runtime input").unwrap();
            let output = source.path().with_extension("zip");
            assert!(package(source.path(), &output).is_err());
            assert!(!output.exists());
        }
    }
    #[test]
    fn runtime_zip_preserves_executable_permission_and_contains_no_assets() {
        let source = tempfile::tempdir().unwrap();
        runtime(source.path());
        let output = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        fs::remove_file(&output).unwrap();
        package(source.path(), &output).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
        assert_eq!(archive.len(), 6);
        assert_eq!(
            archive
                .by_name("Maiko Mac Runtime/redguard-native")
                .unwrap()
                .unix_mode()
                .unwrap()
                & 0o777,
            0o755
        );
        assert!(archive.by_name("Maiko Mac Runtime/game/RGFX.EXE").is_err());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape_and_preserves_internal_framework_links() {
        let source = tempfile::tempdir().unwrap();
        runtime(source.path());
        let link = source.path().join("Frameworks/SDL2.framework/Current");
        std::os::unix::fs::symlink("SDL2", &link).unwrap();
        let output = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        fs::remove_file(&output).unwrap();
        package(source.path(), &output).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
        assert_eq!(
            archive
                .by_name("Maiko Mac Runtime/Frameworks/SDL2.framework/Current")
                .unwrap()
                .unix_mode()
                .unwrap()
                & 0o170000,
            0o120000
        );
        fs::remove_file(output).unwrap();
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", &link).unwrap();
        assert!(package(source.path(), &source.path().with_extension("zip")).is_err());
    }
}
