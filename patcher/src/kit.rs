//! Source-free compiler/runtime kits, shared by the CLI and browser.
use crate::{
    image::{Result, hash},
    prepare::Files,
};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};

const MAX_KIT: u64 = 384 * 1024 * 1024;
const MAX_ENTRY: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Linux,
    Windows,
    Macos,
}
impl Target {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "linux" => Ok(Self::Linux),
            "windows" => Ok(Self::Windows),
            "macos" => Ok(Self::Macos),
            _ => Err("Target must be linux, windows or macos".into()),
        }
    }
    pub fn executable(self) -> &'static str {
        match self {
            Self::Linux | Self::Macos => "redguard-native",
            Self::Windows => "redguard-native.exe",
        }
    }
    pub fn assembly_format(self) -> &'static str {
        match self {
            Self::Linux => "elf64",
            Self::Windows => "win64",
            Self::Macos => "macho64",
        }
    }
    pub fn tools(self) -> &'static [&'static str] {
        match self {
            Self::Linux | Self::Windows => &["sr", "nasm"],
            Self::Macos => &["sr-llasm", "dmd", "opt", "llc", "ld64"],
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    pub target: Target,
    pub revision: String,
    pub executable_sha256: String,
    pub files: std::collections::BTreeMap<String, String>,
    pub package_files: Vec<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub package_links: std::collections::BTreeMap<String, String>,
}

pub struct Kit {
    pub manifest: Manifest,
    pub files: Files,
}

pub fn safe_path(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 240
        && name.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_-+.".contains(&byte))
        })
}

pub fn unzip(bytes: &[u8]) -> Result<Files> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    if zip.len() > 4096 {
        return Err("Too many files in runtime kit".into());
    }
    let mut files = Files::new();
    let mut size = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| error.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_owned();
        if !safe_path(&name) || files.contains_key(&name) {
            return Err(format!("Unsafe or duplicate kit path: {name}"));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("Symlinks are not allowed in runtime kits".into());
        }
        size = size.checked_add(entry.size()).ok_or("Kit size overflow")?;
        if size > MAX_KIT || entry.size() > MAX_ENTRY {
            return Err("Runtime kit exceeds the size limit".into());
        }
        let mut output = Vec::new();
        entry
            .by_ref()
            .take(MAX_ENTRY + 1)
            .read_to_end(&mut output)
            .map_err(|error| error.to_string())?;
        if output.len() as u64 != entry.size() {
            return Err("Runtime kit entry size mismatch".into());
        }
        files.insert(name, output);
    }
    Ok(files)
}

pub fn zip(files: &Files, executable: Option<&str>) -> Result<Vec<u8>> {
    zip_with_links(files, executable, &Default::default())
}

fn zip_with_links(
    files: &Files,
    executable: Option<&str>,
    links: &std::collections::BTreeMap<String, String>,
) -> Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        if !safe_path(name) {
            return Err(format!("Unsafe package path: {name}"));
        }
        let mode = if Some(name.as_str()) == executable
            || name.ends_with(".dylib")
            || name.ends_with("/SDL2")
            || name.ends_with("/SDL2_mixer")
        {
            0o755
        } else {
            0o644
        };
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(mode);
        zip.start_file(name, options)
            .map_err(|error| error.to_string())?;
        zip.write_all(bytes).map_err(|error| error.to_string())?;
    }
    for (name, target) in links {
        if !safe_path(name) || !safe_path(target) || files.contains_key(name) {
            return Err("Unsafe or duplicate framework link".into());
        }
        zip.add_symlink(
            name,
            target,
            zip::write::SimpleFileOptions::default().unix_permissions(0o777),
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(zip
        .finish()
        .map_err(|error| error.to_string())?
        .into_inner())
}

impl Kit {
    pub fn load(bytes: &[u8], target: Target) -> Result<Self> {
        let files = unzip(bytes)?;
        let manifest_bytes = files.get("manifest.json").ok_or("Missing kit manifest")?;
        if manifest_bytes.len() > 1024 * 1024 {
            return Err("Runtime manifest exceeds the size limit".into());
        }
        let manifest: Manifest =
            serde_json::from_slice(manifest_bytes).map_err(|error| error.to_string())?;
        if manifest.format != 1 || manifest.target != target {
            return Err("Runtime kit format or target does not match".into());
        }
        if manifest.executable_sha256 == "0".repeat(64)
            || manifest.executable_sha256.len() != 64
            || !manifest
                .executable_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("Runtime kit needs an accepted executable SHA-256".into());
        }
        if files.len() != manifest.files.len() + 1 {
            return Err("Runtime kit contains unlisted files".into());
        }
        for (name, expected) in &manifest.files {
            if hash(
                files
                    .get(name)
                    .ok_or_else(|| format!("Missing kit file {name}"))?,
            ) != *expected
            {
                return Err(format!("Runtime kit checksum mismatch: {name}"));
            }
        }
        for tool in target.tools() {
            let name = format!("tools/{tool}.wasm");
            if !files
                .get(&name)
                .is_some_and(|data| data.starts_with(b"\0asm\x01\0\0\0"))
            {
                return Err(format!("Invalid WebAssembly tool: {name}"));
            }
        }
        if !manifest
            .package_files
            .iter()
            .any(|name| name == "LICENSES.txt")
        {
            return Err("Runtime kit lacks licence notices".into());
        }
        for name in &manifest.package_files {
            if !safe_path(name) || !files.contains_key(&format!("runtime/{name}")) {
                return Err(format!("Missing runtime package file {name}"));
            }
        }
        for (name, link) in &manifest.package_links {
            if target != Target::Macos
                || !(name.starts_with("Frameworks/SDL2.framework/")
                    || name.starts_with("Frameworks/SDL2_mixer.framework/"))
                || !safe_path(name)
                || !safe_path(link)
                || manifest.package_files.contains(name)
            {
                return Err("Unsafe or duplicate framework link".into());
            }
        }
        if target == Target::Macos {
            for name in ["tools/llasm.d", "tools/dlang.js"] {
                if files
                    .get(name)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .is_none_or(str::is_empty)
                {
                    return Err(format!("Missing or invalid Mac compiler asset {name}"));
                }
            }
            for arch in crate::macos::ARCHITECTURES {
                for name in crate::macos::linker_inputs(arch)? {
                    if !files.contains_key(&name) {
                        return Err(format!("Missing Mac linker input {name}"));
                    }
                }
            }
        }
        Ok(Self { manifest, files })
    }

    pub fn compiler_files(&self) -> Files {
        self.files
            .iter()
            .filter(|(name, _)| {
                name.starts_with(if self.manifest.target == Target::Macos {
                    "support/llasm/"
                } else {
                    "support/"
                })
            })
            .map(|(name, bytes)| {
                (
                    name.strip_prefix("support/llasm/")
                        .unwrap_or(name)
                        .to_owned(),
                    bytes.clone(),
                )
            })
            .collect()
    }

    pub fn relink(&self, assembled: &[u8]) -> Result<Vec<u8>> {
        let recipe: crate::relink::Recipe = serde_json::from_slice(
            self.files
                .get("recipe.json")
                .ok_or("Missing relocation recipe")?,
        )
        .map_err(|e| e.to_string())?;
        let executable = recipe.apply(
            self.files
                .get("template.bin")
                .ok_or("Missing runtime template")?,
            assembled,
        )?;
        if hash(&executable) != self.manifest.executable_sha256 {
            return Err("Reconstructed executable differs from the accepted build".into());
        }
        Ok(executable)
    }

    pub fn package(&self, executable: &[u8]) -> Result<Vec<u8>> {
        if hash(executable) != self.manifest.executable_sha256 {
            return Err(
                "Compiled executable differs from the accepted build. No package was produced."
                    .into(),
            );
        }
        let target = self.manifest.target;
        let mut files = Files::new();
        for name in &self.manifest.package_files {
            files.insert(name.clone(), self.files[&format!("runtime/{name}")].clone());
        }
        files.insert(target.executable().into(), executable.to_vec());
        files.insert("build.json".into(), serde_json::to_vec_pretty(&serde_json::json!({"revision": self.manifest.revision, "target": target, "executable_sha256": self.manifest.executable_sha256})).map_err(|error| error.to_string())?);
        zip_with_links(
            &files,
            Some(target.executable()),
            &self.manifest.package_links,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn macos_kit(link: &str) -> (Vec<u8>, Vec<u8>) {
        let thin = |cpu, subtype| {
            [0xfeedfacfu32, cpu, subtype, 2, 0, 0, 0, 0]
                .into_iter()
                .flat_map(u32::to_le_bytes)
                .collect::<Vec<_>>()
        };
        let executable =
            crate::macho::universal(&thin(0x01000007, 3), &thin(0x0100000c, 0)).unwrap();
        let mut files = Files::new();
        for tool in Target::Macos.tools() {
            files.insert(format!("tools/{tool}.wasm"), b"\0asm\x01\0\0\0".to_vec());
        }
        for name in ["llasm.d", "dlang.js"] {
            files.insert(format!("tools/{name}"), b"compiler source".to_vec());
        }
        for arch in crate::macos::ARCHITECTURES {
            for name in crate::macos::linker_inputs(arch).unwrap() {
                files.insert(name, b"runtime support".to_vec());
            }
        }
        files.insert("runtime/LICENSES.txt".into(), b"Licence notices".to_vec());
        files.insert("runtime/README.md".into(), b"Launch instructions".to_vec());
        let package_files = files
            .keys()
            .filter_map(|name| name.strip_prefix("runtime/").map(str::to_owned))
            .collect();
        let manifest = Manifest {
            format: 1,
            target: Target::Macos,
            revision: "test".into(),
            executable_sha256: hash(&executable),
            files: files
                .iter()
                .map(|(name, bytes)| (name.clone(), hash(bytes)))
                .collect(),
            package_files,
            package_links: std::collections::BTreeMap::from([
                ("Frameworks/SDL2.framework/SDL2".into(), link.into()),
                (
                    "Frameworks/SDL2.framework/Versions/Current".into(),
                    "A".into(),
                ),
            ]),
        };
        files.insert(
            "manifest.json".into(),
            serde_json::to_vec(&manifest).unwrap(),
        );
        (zip(&files, None).unwrap(), executable)
    }

    #[test]
    fn macos_package_contains_both_architectures_framework_links_and_no_compiler() {
        let (bytes, executable) = macos_kit("Versions/Current/SDL2");
        let kit = Kit::load(&bytes, Target::Macos).unwrap();
        assert!(kit.package(b"wrong executable").is_err());
        let package = kit.package(&executable).unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(package)).unwrap();
        let names: Vec<_> = archive.file_names().map(str::to_owned).collect();
        assert!(names.iter().all(|name| !name.starts_with("tools/")
            && !name.starts_with("support/")
            && !name.ends_with(".command")
            && name != "RGFX.EXE"));
        let mut binary = Vec::new();
        archive
            .by_name("redguard-native")
            .unwrap()
            .read_to_end(&mut binary)
            .unwrap();
        crate::macho::check_universal(&binary).unwrap();
        assert_eq!(binary, executable);
        assert_eq!(
            archive
                .by_name("redguard-native")
                .unwrap()
                .unix_mode()
                .unwrap()
                & 0o777,
            0o755
        );
        let mut link = archive.by_name("Frameworks/SDL2.framework/SDL2").unwrap();
        assert_eq!(link.unix_mode().unwrap() & 0o170000, 0o120000);
        let mut target = String::new();
        link.read_to_string(&mut target).unwrap();
        assert_eq!(target, "Versions/Current/SDL2");
    }

    #[test]
    fn macos_kit_refuses_framework_links_outside_the_archive() {
        let (bytes, _) = macos_kit("../../outside");
        assert!(
            Kit::load(&bytes, Target::Macos)
                .err()
                .unwrap()
                .contains("framework link")
        );
    }

    #[test]
    fn refuses_escaping_and_windows_paths() {
        for name in ["../x", "/x", "x/../y", "C:/x", "x\\y", "", "x//y", "x\0y"] {
            assert!(!safe_path(name), "{name}");
        }
        assert!(safe_path("link/libSDL2.so.0"));
    }
    #[test]
    fn zip_roundtrip_retains_files_and_executable_permissions() {
        let files = Files::from([
            ("redguard-native".into(), b"\x7fELF".to_vec()),
            ("LICENSES.txt".into(), b"Notice".to_vec()),
        ]);
        let bytes = zip(&files, Some("redguard-native")).unwrap();
        assert_eq!(unzip(&bytes).unwrap(), files);
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert_eq!(
            archive
                .by_name("redguard-native")
                .unwrap()
                .unix_mode()
                .unwrap()
                & 0o777,
            0o755
        );
    }
}
