//! Export fixed runtime/support inputs, never the accepted translated engine.
use super::{Result, run};
use maiko_patcher::{
    image::hash,
    kit::{Kit, Manifest, Target, safe_path, zip},
    macos,
    native::read_limited,
    prepare::Files,
};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, io::Write, path::Path, process::Command};

#[derive(Deserialize)]
struct Report {
    status: String,
    macos_game_built: bool,
    openglide_commit: String,
    source_sha256: BTreeMap<String, String>,
    artifact_sha256: BTreeMap<String, String>,
}

fn checked(build: &Path, report: &Report, name: &str) -> Result<Vec<u8>> {
    let bytes = read_limited(&build.join(name), 256 * 1024 * 1024)?;
    if report.artifact_sha256.get(name) != Some(&hash(&bytes)) {
        return Err(format!("Build artifact changed: {name}").into());
    }
    Ok(bytes)
}

fn renderer_source(report: &Report) -> Result<Vec<u8>> {
    if report.openglide_commit != "9f8a2cb73af3b6d26e9ec67bb9e1873d3e211726" {
        return Err("Unexpected renderer revision".into());
    }
    let mut files = Files::new();
    files.insert(
        "openglide-ng.tar".into(),
        run(Command::new("git").args([
            "-C",
            ".cache/openglide-ng",
            "archive",
            &report.openglide_commit,
        ]))?,
    );
    for path in [
        "patches/openglide-x64.patch",
        "patches/openglide-sdl-window.patch",
        "patches/openglide-macos.patch",
        "platforms/macos/build_renderer.sh",
        "platforms/macos/Dockerfile",
        "tests/backend_smoke.cpp",
    ] {
        files.insert(path.into(), fs::read(path)?);
    }
    files.insert("README.txt".into(), b"OpenGLide-ng source: https://github.com/eugenenuke/openglide-ng\nExtract openglide-ng.tar to /src and apply the three patches in their listed order: x64, sdl-window, macos, using patch -p1.\nUse the supplied platforms/macos/Dockerfile toolchain. Copy platforms/ and tests/ to /work. Supply the macOS 11.3 SDK at /sdk/MacOSX11.3.sdk and SDL2.framework 2.30.11 at /frameworks. Create /results and run sh /work/platforms/macos/build_renderer.sh.\nCombine the two renderer builds with llvm-lipo-14 -create or replace libredguard-glide.dylib with the matching architecture build. The Apple SDK is not redistributed.\n".to_vec());
    Ok(zip(&files, None)?)
}

pub(super) fn export(
    build: &Path,
    runtime: &Path,
    accepted: &Path,
    revision: &str,
    output: &Path,
) -> Result<()> {
    if output.exists() {
        return Err("Output already exists".into());
    }
    let report: Report =
        serde_json::from_slice(&read_limited(&build.join("report.json"), 1024 * 1024)?)?;
    if !report.macos_game_built
        || report.status != "synthetic_execution_and_macos_game_cross_link_passed"
    {
        return Err("Mac kit export requires a completed native backend check".into());
    }
    for (path, digest) in &report.source_sha256 {
        if !safe_path(path) || hash(&fs::read(path)?) != *digest {
            return Err(format!("Native source changed: {path}").into());
        }
    }
    let mut files = Files::new();
    let cache = Path::new(".cache/client-macos-toolchain");
    for name in [
        "sr-llasm.wasm",
        "dmd.wasm",
        "opt.wasm",
        "llc.wasm",
        "ld64.wasm",
        "llasm.d",
        "dlang.js",
        "LICENSES.txt",
    ] {
        files.insert(
            format!("tools/{name}"),
            read_limited(&cache.join(name), 256 * 1024 * 1024)?,
        );
    }
    for entry in fs::read_dir(cache.join("llasm-support"))? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid LLasm support name")?;
        if !safe_path(name) || !name.ends_with(".llinc") {
            return Err("Unexpected LLasm support input".into());
        }
        files.insert(
            format!("support/llasm/{name}"),
            read_limited(&path, 1024 * 1024)?,
        );
    }
    for arch in macos::ARCHITECTURES {
        let executable = checked(build, &report, &format!("redguard-native-{arch}"))?;
        files.extend(macos::system_stubs(&executable, arch)?);
        for name in macos::linker_inputs(arch)?
            .into_iter()
            .filter(|name| name.starts_with("support/") && !name.ends_with(".tbd"))
        {
            let basename = name.rsplit('/').next().ok_or("Invalid linker input")?;
            let source = if basename == "host.a" {
                format!("libmaiko-host-{arch}.a")
            } else {
                format!("game-{arch}/{basename}")
            };
            files.insert(name, checked(build, &report, &source)?);
        }
    }
    let runtime = runtime.canonicalize()?;
    let mut pending = vec![runtime.clone()];
    let mut package_files = Vec::new();
    let mut package_links = BTreeMap::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            let name = path
                .strip_prefix(&runtime)?
                .to_str()
                .ok_or("Invalid runtime path")?
                .replace('\\', "/");
            if name == "redguard-native" {
                continue;
            }
            if !safe_path(&name)
                || !(name == "libredguard-glide.dylib"
                    || name == "README.md"
                    || name == "LICENSES.txt"
                    || name == "Frameworks"
                    || (name == "Frameworks/SDL2.framework"
                        || name.starts_with("Frameworks/SDL2.framework/"))
                    || (name == "Frameworks/SDL2_mixer.framework"
                        || name.starts_with("Frameworks/SDL2_mixer.framework/")))
            {
                return Err(format!("Unexpected runtime file or game data: {name}").into());
            }
            let kind = fs::symlink_metadata(&path)?.file_type();
            if kind.is_symlink() {
                let target = fs::read_link(&path)?
                    .to_str()
                    .ok_or("Invalid framework target")?
                    .to_owned();
                if !name.starts_with("Frameworks/")
                    || !safe_path(&target)
                    || !path.canonicalize()?.starts_with(&runtime)
                {
                    return Err("Framework link escapes the runtime".into());
                }
                package_links.insert(name, target);
            } else if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                let bytes = read_limited(&path, 128 * 1024 * 1024)?;
                if name != "LICENSES.txt"
                    && report.artifact_sha256.get(&format!("runtime/{name}")) != Some(&hash(&bytes))
                {
                    return Err(
                        format!("Runtime differs from the checked native build: {name}").into(),
                    );
                }
                files.insert(format!("runtime/{name}"), bytes);
                package_files.push(name);
            } else {
                return Err("Unexpected runtime file type".into());
            }
        }
    }
    if !package_files.iter().any(|name| name == "LICENSES.txt") {
        return Err("Runtime needs complete licence notices".into());
    }
    let source_name = "redguard-macos-renderer-source.zip";
    files.insert(format!("runtime/{source_name}"), renderer_source(&report)?);
    package_files.push(source_name.into());
    package_files.sort();
    let executable = read_limited(accepted, 512 * 1024 * 1024)?;
    maiko_patcher::macho::check_universal(&executable)?;
    let manifest = Manifest {
        format: 1,
        target: Target::Macos,
        revision: revision.into(),
        executable_sha256: hash(&executable),
        files: files
            .iter()
            .map(|(name, bytes)| (name.clone(), hash(bytes)))
            .collect(),
        package_files,
        package_links,
    };
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    );
    let bytes = zip(&files, None)?;
    let kit = Kit::load(&bytes, Target::Macos)?;
    for arch in macos::ARCHITECTURES {
        macos::linker_files(&kit, arch)?;
    }
    kit.package(&executable)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&bytes)?;
    println!("{}  {}", hash(&bytes), output.display());
    Ok(())
}
