//! Export fixed runtime/support inputs, never the accepted translated engine.
use super::Result;
use maiko_patcher::{
    image::hash,
    kit::{Target, safe_path},
    macos,
    native::read_limited,
    prepare::Files,
};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Deserialize)]
struct Report {
    status: String,
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

pub(super) fn export(build: &Path, runtime: &Path, revision: &str, output: &Path) -> Result<()> {
    if output.exists() {
        return Err("Output already exists".into());
    }
    let report: Report =
        serde_json::from_slice(&read_limited(&build.join("report.json"), 1024 * 1024)?)?;
    if !matches!(
        report.status.as_str(),
        "synthetic_execution_and_macos_kit_inputs_built"
            | "synthetic_execution_and_macos_game_cross_link_passed"
    ) {
        return Err("Mac kit export requires a completed native backend check (--kit)".into());
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
        "llasm.d",
        "dlang.js",
        "LICENSES.txt",
    ] {
        files.insert(
            format!("tools/{name}"),
            read_limited(&cache.join(name), 256 * 1024 * 1024)?,
        );
    }
    files.insert(
        "tools/lld.wasm".into(),
        read_limited(&cache.join("ld64.wasm"), 256 * 1024 * 1024)?,
    );
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
    let mut package_files = Vec::new();
    for entry in fs::read_dir(runtime)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid runtime path")?
            .to_owned();
        if name == "redguard-native" {
            continue;
        }
        if !["maiko", "README.md", "LICENSES.txt"].contains(&name.as_str())
            || !fs::symlink_metadata(&path)?.is_file()
        {
            return Err(format!("Unexpected runtime file or game data: {name}").into());
        }
        let bytes = read_limited(&path, 128 * 1024 * 1024)?;
        if name != "LICENSES.txt"
            && report.artifact_sha256.get(&format!("runtime/{name}")) != Some(&hash(&bytes))
        {
            return Err(format!("Runtime differs from the checked native build: {name}").into());
        }
        files.insert(format!("runtime/{name}"), bytes);
        package_files.push(name);
    }
    super::write_kit(Target::Macos, revision, files, package_files, output)
}
