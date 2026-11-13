//! Pinned browser LLasm/LLVM toolchain. This build never reads game input.
use super::{Result, download, patch, run, tools};
use maiko_patcher::image::hash;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const D_REVISION: &str = "6a9b3a8e83c6c79248a7f7957c07280009f250fe";
const SDK_HASH: &str = "87d1d1a2879d139cdc624b968efad3d4a97b8078cdff95e63ac88ecafd1a0171";

fn extract(cache: &Path, archive: &Path, directory: &str, members: &[&str]) -> Result<PathBuf> {
    let destination = cache.join(directory);
    let missing: Vec<_> = members
        .iter()
        .filter(|member| !destination.join(member).exists())
        .collect();
    if !destination.exists() || !missing.is_empty() {
        let mut command = Command::new("tar");
        command.arg("-xf").arg(archive).arg("-C").arg(cache);
        for member in missing {
            command.arg(format!("{directory}/{member}"));
        }
        run(&mut command)?;
    }
    Ok(fs::canonicalize(destination)?)
}

pub(super) fn build() -> Result<()> {
    let cache = Path::new(".cache/client-macos-toolchain");
    fs::create_dir_all(cache)?;
    let sdk_archive = cache.join("wasi-sdk.tar.gz");
    download(
        "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-29/wasi-sdk-29.0-x86_64-linux.tar.gz",
        &sdk_archive,
        SDK_HASH,
    )?;
    let sdk = extract(cache, &sdk_archive, "wasi-sdk-29.0-x86_64-linux", &[])?;
    tools::build_sr(cache, &sdk.join("bin"), true)?;

    let d_archive = cache.join("browser-d.tar.gz");
    download(
        &format!("https://codeload.github.com/live-codes/browser-d/tar.gz/{D_REVISION}"),
        &d_archive,
        "c492974eb40c725d7411271c31b4c49d1cc0af464163a9b49bc5678c4560136c",
    )?;
    let d = extract(
        cache,
        &d_archive,
        &format!("browser-d-{D_REVISION}"),
        &["packages/dlang-wasm"],
    )?
    .join("packages/dlang-wasm");
    let compressed = fs::read(d.join("assets/dmd.wasm.gz"))?;
    if hash(&compressed) != "cbc35572a3069bb49df70513f8ed72db2f3fa8173741a68e9bf53203b0685613" {
        return Err("DMD asset checksum mismatch".into());
    }
    fs::write(
        cache.join("dmd.wasm"),
        run(Command::new("gzip")
            .arg("-dc")
            .arg(d.join("assets/dmd.wasm.gz")))?,
    )?;
    if hash(&fs::read(cache.join("dmd.wasm"))?)
        != "65039fcc95a116e2007a0f74279b29868a882676b105b26005840a74965ebc45"
    {
        return Err("Unpacked DMD checksum mismatch".into());
    }
    let driver = cache.join("dlang-entry.js");
    let api = serde_json::to_string(
        d.join("src/api.js")
            .to_str()
            .ok_or("Invalid D compiler path")?,
    )?;
    fs::write(
        &driver,
        format!(
            r#"import {{createApi}} from {api};
export async function compile(source, input, wasm) {{
    const api = createApi({{packaged: {{wasmPath: 'kit-dmd.wasm', readBytes: async () => wasm}}}});
    const compiler = await api.createCompiler('d');
    try {{ return await compiler.run(source, input); }} finally {{ compiler.dispose(); }}
}}
"#
        ),
    )?;
    run(Command::new("bun")
        .arg("build")
        .arg(&driver)
        .args(["--target=browser", "--minify", "--outfile"])
        .arg(cache.join("dlang.js")))?;

    let llvm_archive = cache.join("llvm-project-19.1.0.src.tar.xz");
    download(
        "https://github.com/llvm/llvm-project/releases/download/llvmorg-19.1.0/llvm-project-19.1.0.src.tar.xz",
        &llvm_archive,
        "5042522b49945bc560ff9206f25fb87980a9b89b914193ca00d961511ff0673c",
    )?;
    let llvm = extract(
        cache,
        &llvm_archive,
        "llvm-project-19.1.0.src",
        &["llvm", "lld", "libunwind", "cmake", "third-party"],
    )?;
    let patch_hash = hash(&fs::read("patches/llvm-wasi.patch")?);
    let stamp = llvm.join(".maiko-wasi-patch");
    if stamp.exists() {
        if fs::read_to_string(&stamp)? != patch_hash {
            return Err("LLVM patch changed; remove the idle cached LLVM source/build directories and rebuild".into());
        }
    } else {
        patch(&llvm, "patches/llvm-wasi.patch")?;
        fs::write(stamp, patch_hash)?;
    }
    fs::create_dir_all(cache.join("native"))?;
    let native = fs::canonicalize(cache.join("native"))?;
    run(Command::new("cmake")
        .args(["-G", "Ninja", "-S"])
        .arg(llvm.join("llvm"))
        .arg("-B")
        .arg(&native)
        .args([
            "-DCMAKE_BUILD_TYPE=Release",
            "-DLLVM_TARGETS_TO_BUILD=X86;AArch64",
            "-DLLVM_INCLUDE_TESTS=OFF",
            "-DLLVM_INCLUDE_EXAMPLES=OFF",
            "-DLLVM_INCLUDE_BENCHMARKS=OFF",
            "-DLLVM_ENABLE_ZLIB=OFF",
            "-DLLVM_ENABLE_ZSTD=OFF",
        ]))?;
    run(Command::new("ninja")
        .arg("-C")
        .arg(&native)
        .args(["-j2", "llvm-tblgen"]))?;
    fs::create_dir_all(cache.join("wasi"))?;
    let wasi = fs::canonicalize(cache.join("wasi"))?;
    run(Command::new("cmake").args(["-G", "Ninja", "-S"]).arg(llvm.join("llvm")).arg("-B").arg(&wasi)
        .arg(format!("-DCMAKE_TOOLCHAIN_FILE={}", sdk.join("share/cmake/wasi-sdk-p1.cmake").display()))
        .arg(format!("-DLLVM_TABLEGEN={}", native.join("bin/llvm-tblgen").display()))
        .args(["-DCMAKE_BUILD_TYPE=MinSizeRel", "-DLLVM_HOST_TRIPLE=wasm32-wasip1", "-DLLVM_TARGETS_TO_BUILD=X86;AArch64", "-DLLVM_ENABLE_PROJECTS=lld", "-DLLVM_ENABLE_THREADS=OFF", "-DLLVM_ENABLE_BACKTRACES=OFF", "-DLLVM_ENABLE_CRASH_OVERRIDES=OFF", "-DLLVM_INCLUDE_TESTS=OFF", "-DLLVM_INCLUDE_EXAMPLES=OFF", "-DLLVM_INCLUDE_BENCHMARKS=OFF", "-DLLVM_ENABLE_ZLIB=OFF", "-DLLVM_ENABLE_ZSTD=OFF", "-DLLVM_ENABLE_LIBXML2=OFF", "-DLLVM_ENABLE_LIBEDIT=OFF", "-DLLVM_BUILD_STATIC=ON", "-DLLVM_DEFAULT_TARGET_TRIPLE=arm64-apple-macos11",
            "-DCMAKE_C_FLAGS=-D_WASI_EMULATED_MMAN -D_WASI_EMULATED_PROCESS_CLOCKS -D_WASI_EMULATED_GETPID",
            "-DCMAKE_CXX_FLAGS=-D_WASI_EMULATED_MMAN -D_WASI_EMULATED_PROCESS_CLOCKS -D_WASI_EMULATED_GETPID",
            "-DCMAKE_EXE_LINKER_FLAGS=-Wl,--stack-first -Wl,-z,stack-size=16777216 -Wl,--initial-memory=67108864 -Wl,--max-memory=4294967296 -lwasi-emulated-mman -lwasi-emulated-process-clocks -lwasi-emulated-getpid"]))?;
    run(Command::new("ninja")
        .arg("-C")
        .arg(&wasi)
        .args(["-j2", "opt", "llc", "lld"]))?;
    for (source, name) in [("opt", "opt"), ("llc", "llc"), ("lld", "ld64")] {
        fs::copy(
            wasi.join("bin").join(source),
            cache.join(format!("{name}.wasm")),
        )?;
    }
    let mut notices = format!(
        "Browser D compiler {D_REVISION}\n{}\n{}\nLLVM/LLD 19.1.0\n{}\n",
        fs::read_to_string(d.join("LICENSE"))?,
        fs::read_to_string(d.join("THIRD-PARTY-NOTICES.md"))?,
        fs::read_to_string(llvm.join("llvm/LICENSE.TXT"))?
    );
    notices.push_str("\nLLVM WASI compatibility patch: breeze833/clang-wasi b284fc9fb092f7ffec71a17163b8c585a4763548, Apache-2.0.\n");
    for (name, url, digest) in [
        (
            "wasi-sdk",
            "https://raw.githubusercontent.com/WebAssembly/wasi-sdk/wasi-sdk-29/LICENSE",
            "268872b9816f90fd8e85db5a28d33f8150ebb8dd016653fb39ef1f94f2686bc5",
        ),
        (
            "wasi-libc",
            "https://raw.githubusercontent.com/WebAssembly/wasi-libc/ac020b86fd44bafe60aa4fa12f407d16e3731329/LICENSE",
            "2711a8b5a5cdfef0e639f96c1aca12ae23d7d64a02d0507f1bdf14d2b27bbc3a",
        ),
        (
            "wasi-libc-mit",
            "https://raw.githubusercontent.com/WebAssembly/wasi-libc/ac020b86fd44bafe60aa4fa12f407d16e3731329/LICENSE-MIT",
            "23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3",
        ),
        (
            "musl",
            "https://raw.githubusercontent.com/WebAssembly/wasi-libc/ac020b86fd44bafe60aa4fa12f407d16e3731329/libc-top-half/musl/COPYRIGHT",
            "f9bc4423732350eb0b3f7ed7e91d530298476f8fec0c6c427a1c04ade22655af",
        ),
        (
            "cloudlibc",
            "https://raw.githubusercontent.com/WebAssembly/wasi-libc/ac020b86fd44bafe60aa4fa12f407d16e3731329/libc-bottom-half/cloudlibc/LICENSE",
            "c8b789cf5a746611e6300a0cc7750dbf92b61912a709d04e639245f7290656d0",
        ),
        (
            "musl-fts",
            "https://raw.githubusercontent.com/WebAssembly/wasi-libc/ac020b86fd44bafe60aa4fa12f407d16e3731329/fts/musl-fts/COPYING",
            "55af87e4017668f54467a3380e7ebbac5e672d8c763bfe95e6fc882a6fdc4046",
        ),
    ] {
        let path = cache.join(format!("{name}-LICENSE.txt"));
        download(url, &path, digest)?;
        notices.push_str(&format!("\n{name}\n{}\n", fs::read_to_string(path)?));
    }
    for path in ["SR/SR_main.c", "SR/udis86-1.6/LICENSE"] {
        let source = String::from_utf8(run(Command::new("git").args([
            "-C",
            ".cache/sr",
            "show",
            &format!("{}:{path}", super::SR),
        ]))?)?;
        notices.push_str(
            source
                .split("#include")
                .next()
                .ok_or("Missing SR licence")?,
        );
    }
    fs::write(cache.join("LICENSES.txt"), notices)?;
    Ok(())
}
