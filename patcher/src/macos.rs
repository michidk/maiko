//! Browser compiler commands and source-free Mach-O linker inputs.
use crate::{image::Result, kit::Kit, prepare::Files};

pub const ARCHITECTURES: [&str; 2] = ["x86_64", "arm64"];
const OBJECTS: [&str; 12] = [
    "audio.o",
    "heap.o",
    "llasm_cmps.o",
    "llasm_divide.o",
    "llasm_float.o",
    "llasm_fs_mem.o",
    "llasm_inout.o",
    "llasm_movmem.o",
    "llasm_movs.o",
    "llasm_pushx.o",
    "llasm_scas.o",
    "llasm_stos.o",
];
const LIBRARIES: [(&str, &str); 6] = [
    ("System", "/usr/lib/libSystem.B.dylib"),
    (
        "Security",
        "/System/Library/Frameworks/Security.framework/Versions/A/Security",
    ),
    (
        "CoreFoundation",
        "/System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation",
    ),
    ("iconv", "/usr/lib/libiconv.2.dylib"),
    ("resolv", "/usr/lib/libresolv.9.dylib"),
    ("c++", "/usr/lib/libc++.1.dylib"),
];
const RUNTIME: [&str; 3] = [
    "libredguard-glide.dylib",
    "Frameworks/SDL2.framework/Versions/A/SDL2",
    "Frameworks/SDL2_mixer.framework/Versions/A/SDL2_mixer",
];

fn architecture(arch: &str) -> Result<()> {
    if !ARCHITECTURES.contains(&arch) {
        return Err("Unknown Mac architecture".into());
    }
    Ok(())
}

pub fn linker_inputs(arch: &str) -> Result<Vec<String>> {
    architecture(arch)?;
    let mut inputs: Vec<_> = OBJECTS
        .iter()
        .map(|name| format!("support/link/{arch}/{name}"))
        .collect();
    inputs.push(format!("support/link/{arch}/host.a"));
    inputs.extend(RUNTIME.iter().map(|name| format!("runtime/{name}")));
    inputs.extend(
        LIBRARIES
            .iter()
            .map(|(name, _)| format!("support/link/{arch}/{name}.tbd")),
    );
    Ok(inputs)
}

pub fn linker_files(kit: &Kit, arch: &str) -> Result<Files> {
    linker_inputs(arch)?
        .into_iter()
        .map(|name| {
            let bytes = kit
                .files
                .get(&name)
                .ok_or_else(|| format!("Missing Mac linker input {name}"))?;
            Ok((name, bytes.clone()))
        })
        .collect()
}

pub fn optimization() -> Vec<String> {
    ["opt", "-O2", "game.ll", "-o", "game.bc"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

pub fn compilation(arch: &str) -> Result<Vec<(&'static str, Vec<String>, String)>> {
    architecture(arch)?;
    let bitcode = "game.bc".to_owned();
    let object = format!("engine-{arch}.o");
    let executable = format!("redguard-native-{arch}");
    let triple = format!("{arch}-apple-darwin");
    let mut link: Vec<String> = [
        "ld64",
        "-arch",
        arch,
        "-platform_version",
        "macos",
        "11.0",
        "11.3",
        "-pie",
        "-dead_strip",
        "-adhoc_codesign",
        "-rpath",
        "@executable_path/Frameworks",
        "-rpath",
        "@executable_path",
        "-o",
        &executable,
        &object,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    link.extend(linker_inputs(arch)?);
    Ok(vec![
        (
            "llc",
            vec![
                "llc".into(),
                "-O=2".into(),
                "-filetype=obj".into(),
                format!("-mtriple={triple}"),
                "--relocation-model=pic".into(),
                bitcode,
                "-o".into(),
                object.clone(),
            ],
            object,
        ),
        ("ld64", link, executable),
    ])
}

/// Emit accepted library identities/imports and the Rust personality unwind ABI.
/// These are linker contracts, with no SDK implementation or game bytes.
pub fn system_stubs(executable: &[u8], arch: &str) -> Result<Files> {
    use object::Object;
    use std::collections::{BTreeMap, BTreeSet};
    architecture(arch)?;
    let file = object::File::parse(executable).map_err(|e| e.to_string())?;
    let expected = if arch == "arm64" {
        object::Architecture::Aarch64
    } else {
        object::Architecture::X86_64
    };
    if file.architecture() != expected || file.kind() != object::ObjectKind::Executable {
        return Err("Expected a thin accepted Mac executable for this architecture".into());
    }
    let mut symbols: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for import in file.imports().map_err(|e| e.to_string())? {
        let library = std::str::from_utf8(import.library()).map_err(|_| "Invalid dylib name")?;
        let symbol = std::str::from_utf8(import.name()).map_err(|_| "Invalid import name")?;
        if library.starts_with("/usr/lib/") || library.starts_with("/System/") {
            if !LIBRARIES.iter().any(|(_, path)| *path == library) {
                return Err(format!("Undocumented system library: {library}"));
            }
            symbols.entry(library).or_default().insert(symbol);
        }
    }
    let libraries = crate::macho::dylibs(executable)?;
    let mut files = Files::new();
    let version = |value: u32| format!("{}.{}.{}", value >> 16, (value >> 8) & 255, value & 255);
    for (name, library) in LIBRARIES {
        let (current, compatibility) = libraries
            .get(library)
            .ok_or_else(|| format!("Accepted build is missing {library}"))?;
        let mut names = symbols.get(library).cloned().unwrap_or_default();
        if name == "System" {
            // LLVM 19 retains Rust personality code that LLD 14 discarded.
            // These libSystem/libunwind exports are required by those objects,
            // even though they are absent from the accepted LLD 14 imports.
            names.extend([
                "__Unwind_SetIP",
                "__Unwind_SetGR",
                "__Unwind_GetRegionStart",
                "__Unwind_GetIPInfo",
                "__Unwind_GetLanguageSpecificData",
                "__Unwind_GetTextRelBase",
                "__Unwind_GetDataRelBase",
            ]);
        }
        let stub = format!(
            "--- !tapi-tbd\ntbd-version: 4\ntargets: [ {arch}-macos ]\ninstall-name: {}\ncurrent-version: {}\ncompatibility-version: {}\nexports:\n  - targets: [ {arch}-macos ]\n    symbols: {}\n...\n",
            serde_json::to_string(library).map_err(|e| e.to_string())?,
            version(*current),
            version(*compatibility),
            serde_json::to_string(&names).map_err(|e| e.to_string())?
        );
        files.insert(format!("support/link/{arch}/{name}.tbd"), stub.into_bytes());
    }
    Ok(files)
}
