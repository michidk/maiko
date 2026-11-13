//! Build only pinned upstream compiler code, with no game input.
use super::{Result, SR, archive, download, patch, run};
use std::{fs, path::Path, process::Command};

pub(super) fn build() -> Result<()> {
    let cache = Path::new(".cache/client-toolchain");
    fs::create_dir_all(cache)?;
    let archive_path = cache.join("wasi-sdk.tar.gz");
    download(
        "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-29/wasi-sdk-29.0-x86_64-linux.tar.gz",
        &archive_path,
        "87d1d1a2879d139cdc624b968efad3d4a97b8078cdff95e63ac88ecafd1a0171",
    )?;
    let sdk = cache.join("wasi-sdk-29.0-x86_64-linux");
    if !sdk.exists() {
        run(Command::new("tar")
            .arg("-xzf")
            .arg(&archive_path)
            .arg("-C")
            .arg(cache))?;
    }
    let sdk = fs::canonicalize(sdk)?.join("bin");
    let temporary = tempfile::tempdir_in(".cache")?;
    let source = temporary.path().join("sr");
    archive(Path::new(".cache/sr"), SR, &source)?;
    patch(&source, "patches/sr-x64.patch")?;
    patch(&source, "patches/sr-wasi.patch")?;
    let definitions = source.join("SR/SR_defs.h");
    let text = fs::read_to_string(&definitions)?;
    let original = "#define OUTPUT_TYPE  OUT_X86";
    if text.matches(original).count() != 1 {
        return Err("SR output configuration changed".into());
    }
    fs::write(
        definitions,
        text.replace(original, "#define OUTPUT_TYPE  OUT_X64"),
    )?;
    let sr = fs::canonicalize(source.join("SR"))?;
    let mut sources = Vec::new();
    for directory in [&sr, &sr.join("udis86-1.6/libudis86")] {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "c" || extension == "cpp")
            {
                sources.push(path);
            }
        }
    }
    sources.sort();
    let mut objects = Vec::new();
    for source in sources {
        let cpp = source
            .extension()
            .is_some_and(|extension| extension == "cpp");
        let object = source.with_extension("wasi.o");
        let mut command = Command::new(sdk.join(if cpp { "clang++" } else { "clang" }));
        command.args(["-O2", "-D_WASI_EMULATED_MMAN", "-D_WASI_EMULATED_SIGNAL"]);
        if cpp {
            command.arg("-fno-exceptions");
        }
        command
            .arg(format!(
                "-ffile-prefix-map={}=/src",
                fs::canonicalize(temporary.path())?.display()
            ))
            .arg(format!("-I{}", sr.display()))
            .arg(format!("-I{}/udis86-1.6", sr.display()))
            .arg("-c")
            .arg(source)
            .arg("-o")
            .arg(&object);
        run(&mut command)?;
        objects.push(object);
    }
    run(Command::new(sdk.join("clang++"))
        .args(objects)
        .args([
            "-lwasi-emulated-mman",
            "-lwasi-emulated-signal",
            "-Wl,--strip-all,-z,stack-size=8388608,--max-memory=2147483648",
            "-o",
        ])
        .arg(cache.join("sr.wasm")))?;
    let nasm_archive = cache.join("nasm-2.16.01.tar.xz");
    download(
        "https://www.nasm.us/pub/nasm/releasebuilds/2.16.01/nasm-2.16.01.tar.xz",
        &nasm_archive,
        "c77745f4802375efeee2ec5c0ad6b7f037ea9c87c92b149a9637ff099f162558",
    )?;
    let nasm_source = temporary.path().join("nasm");
    fs::create_dir(&nasm_source)?;
    run(Command::new("tar")
        .arg("-xf")
        .arg(&nasm_archive)
        .arg("--strip-components=1")
        .arg("-C")
        .arg(&nasm_source))?;
    run(Command::new("./configure")
        .current_dir(&nasm_source)
        .args([
            "--host=wasm32-wasi",
            "--build=x86_64-linux-gnu",
            "--without-debug",
            "--disable-pedantic",
            "--disable-werror",
        ])
        .env("CC", sdk.join("clang"))
        .env("AR", sdk.join("llvm-ar"))
        .env("RANLIB", sdk.join("llvm-ranlib"))
        .env(
            "CFLAGS",
            format!(
                "-O2 -ffile-prefix-map={}=/src",
                fs::canonicalize(temporary.path())?.display()
            ),
        )
        .env(
            "LDFLAGS",
            "-Wl,--strip-all,-z,stack-size=8388608,--max-memory=2147483648",
        ))?;
    run(Command::new("make")
        .current_dir(&nasm_source)
        .args(["-j2", "nasm"]))?;
    fs::copy(nasm_source.join("nasm"), cache.join("nasm.wasm"))?;
    let mut notice = fs::read_to_string(nasm_source.join("LICENSE"))?;
    let sr_notice = fs::read_to_string(sr.join("SR_main.c"))?;
    notice.push_str(
        sr_notice
            .split("#include")
            .next()
            .ok_or("Missing SR licence")?,
    );
    notice.push_str(&fs::read_to_string(sr.join("udis86-1.6/LICENSE"))?);
    notice.push_str("\nSR is MIT licensed. Upstream: https://github.com/M-HT/SR\n");
    fs::write(cache.join("LICENSES.txt"), notice)?;
    Ok(())
}
