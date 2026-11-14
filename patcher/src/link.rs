//! Windows and Linux link commands for the kit's WebAssembly LLD.
//!
//! The kit ships only game-free link inputs under `support/link/`: C runtime
//! start files, system import libraries, the Rust host and the SR heap/audio
//! objects. The four engine objects are assembled from the user's own
//! executable, then LLD links them with those inputs into the native program.
use crate::{
    image::Result,
    kit::{Kit, Target},
    prepare::Files,
};

const WINDOWS_OBJECTS: [&str; 3] = ["crt2.o", "crtbegin.o", "crtend.o"];
const WINDOWS_RUNTIME: [&str; 3] = ["libmaiko_host.a", "guest_memory.obj", "sr_audio.obj"];
/// The library order GCC passes for the native build, including its implicit MinGW libraries.
pub const WINDOWS_LIBRARIES: [&str; 32] = [
    "SDL2_mixer",
    "SDL2",
    "redguard-glide",
    "stdc++",
    "ws2_32",
    "userenv",
    "ntdll",
    "advapi32",
    "dbghelp",
    "gcc_eh",
    "winpthread",
    "user32",
    "shell32",
    "ole32",
    "mingw32",
    "gcc",
    "gcc_eh",
    "moldname",
    "mingwex",
    "msvcrt",
    "kernel32",
    "pthread",
    "advapi32",
    "shell32",
    "user32",
    "kernel32",
    "mingw32",
    "gcc",
    "gcc_eh",
    "moldname",
    "mingwex",
    "msvcrt",
];
const LINUX_START: [&str; 3] = ["crt1.o", "crti.o", "crtbegin.o"];
const LINUX_RUNTIME: [&str; 3] = ["libmaiko_host.a", "guest_memory.o", "sr_audio.o"];
const LINUX_SYSTEM: [&str; 10] = [
    "libstdc++.so.6",
    "libm.so.6",
    "libgcc.a",
    "libgcc_s.so.1",
    "libc.so.6",
    "libc_nonshared.a",
    "ld-linux-x86-64.so.2",
    "libgcc.a",
    "crtend.o",
    "crtn.o",
];
/// Libraries the Linux program links against from the package itself.
const LINUX_PACKAGED: [&str; 3] = [
    "libredguard-glide.so",
    "libSDL2_mixer-2.0.so.0",
    "libSDL2-2.0.so.0",
];

fn support(name: &str) -> String {
    format!("support/link/{name}")
}

/// Engine sources assembled from the user's executable, and their objects.
pub fn engine(target: Target) -> Vec<(&'static str, String)> {
    let extension = if target == Target::Linux { "o" } else { "obj" };
    ["rgfx", "native_entry", "main", "service"]
        .into_iter()
        .map(|name| (name, format!("{name}.{extension}")))
        .collect()
}

/// Every fixed file a Windows or Linux kit must carry for the link.
pub fn required(target: Target) -> Vec<String> {
    match target {
        Target::Windows => WINDOWS_OBJECTS
            .iter()
            .chain(&WINDOWS_RUNTIME)
            .map(|name| support(name))
            .collect(),
        Target::Linux => LINUX_START
            .iter()
            .chain(&LINUX_RUNTIME)
            .chain(&LINUX_SYSTEM)
            .map(|name| support(name))
            .chain(LINUX_PACKAGED.iter().map(|name| format!("runtime/{name}")))
            .collect(),
        Target::Macos => Vec::new(),
    }
}

/// The files LLD reads: all of `support/link/`, plus the packaged Linux libraries.
pub fn files(kit: &Kit) -> Files {
    let target = kit.manifest.target;
    kit.files
        .iter()
        .filter(|(name, _)| {
            name.starts_with("support/link/")
                || (target == Target::Linux
                    && LINUX_PACKAGED
                        .iter()
                        .any(|library| name.as_str() == format!("runtime/{library}")))
        })
        .map(|(name, bytes)| (name.clone(), bytes.clone()))
        .collect()
}

/// LLD arguments (argument 0 selects its GNU driver) and the executable they write.
pub fn command(target: Target) -> Result<(Vec<String>, String)> {
    let output = target.executable().to_owned();
    let words = |values: &[&str]| {
        values
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>()
    };
    let objects = engine(target).into_iter().map(|(_, object)| object);
    let mut args = Vec::new();
    match target {
        Target::Windows => {
            args.extend(words(&[
                "ld.lld",
                "-m",
                "i386pep",
                "-Bdynamic",
                "-o",
                &output,
            ]));
            args.extend([
                support("crt2.o"),
                support("crtbegin.o"),
                "-Lsupport/link".into(),
            ]);
            args.extend(objects);
            args.extend(words(&["--gc-sections", "--start-group"]));
            args.extend(WINDOWS_RUNTIME.iter().map(|name| support(name)));
            args.push("--end-group".into());
            args.extend(WINDOWS_LIBRARIES.iter().map(|name| format!("-l{name}")));
            args.extend(words(&[
                "--image-base",
                "0x10000000",
                "--disable-dynamicbase",
                "--no-insert-timestamp",
            ]));
            args.push(support("crtend.o"));
        }
        Target::Linux => {
            args.extend(words(&[
                "ld.lld",
                "--build-id",
                "--eh-frame-hdr",
                "-m",
                "elf_x86_64",
                "--hash-style=gnu",
                "--as-needed",
                "-dynamic-linker",
                "/lib64/ld-linux-x86-64.so.2",
                "-o",
                &output,
            ]));
            args.extend(LINUX_START.iter().map(|name| support(name)));
            args.extend(objects);
            args.extend(words(&[
                "-rpath",
                "$ORIGIN",
                "--gc-sections",
                "--start-group",
            ]));
            args.extend(LINUX_RUNTIME.iter().map(|name| support(name)));
            // A search path keeps the library's file name as its DT_NEEDED entry.
            args.extend(words(&["--end-group", "-Lruntime", "-lredguard-glide"]));
            args.extend(
                LINUX_PACKAGED[1..]
                    .iter()
                    .map(|name| format!("runtime/{name}")),
            );
            args.extend(LINUX_SYSTEM.iter().map(|name| support(name)));
        }
        Target::Macos => {
            return Err("Mac packages link per architecture; see macos::compilation".into());
        }
    }
    Ok((args, output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_reference_only_required_inputs_and_engine_objects() {
        for target in [Target::Windows, Target::Linux] {
            let (args, output) = command(target).unwrap();
            assert_eq!(output, target.executable());
            let required = required(target);
            for arg in &args {
                if arg.starts_with("support/") || (arg.starts_with("runtime/") && arg.contains('.'))
                {
                    assert!(required.contains(arg), "{arg} is not a required kit input");
                }
            }
            for (_, object) in engine(target) {
                assert!(args.contains(&object));
            }
        }
    }
}
