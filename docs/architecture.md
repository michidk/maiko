# Runtime structure

Maiko provides native Windows and Linux services for an owned Redguard installation. The runtime reads game assets from that installation and stores saves and settings in a separate user directory.

## Rust workspace

| Crate | Path | Responsibility |
|---|---|---|
| `maiko-runtime` | `crates/runtime/rust/` | Installation checks, files, storage, timing, input, audio, movies, memory, crash reports and the in-game overlay (egui, drawn before each buffer swap) |
| `maiko-glide` | `crates/runtime/glide/` | The game's Glide calls, drawn by the rustyglide renderer linked into the program |
| `maiko-host` | `crates/runtime/host/` | Startup and runtime service dispatch |
| `maiko-patcher` | `crates/patcher/` | Local package builder shared by the CLI and browser |
| `maiko-launcher` | `crates/launcher/` | The launcher window and the game-folder search shared with the runtime |

SDL supplies windowing, input, timing and audio. [rustyglide](https://github.com/michidk/rustyglide) supplies the renderer, drawing through wgpu with Vulkan, Direct3D 12, Metal or OpenGL. Platform bindings handle memory, folder selection and fatal error reporting. Dependency sources and licences are listed in [THIRD_PARTY.md](../THIRD_PARTY.md).

## Files and storage

The game installation is read-only. `cap-std` confines save and settings writes to the selected user directory. Installed movies can be read from a selected CD image, a GOG installation or supported loose movie files. Music is read from the audio tracks of the GOG disc image (`game.gog` and its `game.ins` cue sheet) by `crates/runtime/rust/redbook.rs`, which implements the `AIL_redbook_*` calls the game uses and mixes the tracks into the audio output. Maiko does not distribute those files.

## Source layout

- `crates/`: the Rust crates. `crates/runtime/` also holds the interface headers and the CPU backend startup glue (`*.h`, `*.inc`, `*.asm`); `crates/runtime/rust/os/` has the compile-time host OS implementations for Linux, Windows and macOS, with common POSIX operations under `unix/`.
- `metadata/`: the translation tables. `image/` describes the original executable, `native/` and `llasm/` hold the patches for the native (Windows, Linux) and LLasm (macOS) backends, and `build5/` maps build 1.0 addresses to build 5.0 for that executable (`crates/patcher/src/image.rs`).
- `platforms/linux/`, `platforms/windows/` and `platforms/macos/`: platform build scripts, launchers, native headers and development container definitions.
- `web/`: the static website and browser builder.
- `tools/` and `audio/`: shared build and packaging support.
- `tests/`: independent fixtures, interface checks and runtime checks.
- `patches/`: documented changes to licensed upstream dependencies.

See [building.md](building.md) for source checks and [client-builder.md](client-builder.md) for package creation.

## Host OS boundary

Shared services implement DOS behavior and call the private `os` module for native
memory queries, locking, disk and clock access, path encoding, atomic rename,
file metadata, drive enumeration, folder selection, window handles and
fatal error handling. `os/mod.rs` selects the implementation at compile time.
Linux and macOS share POSIX helpers in `os/unix/`; Linux mapping queries and
macOS Mach queries remain in their own directories. Windows uses Win32 APIs.
The public C service symbols and their argument validation stay in the shared
runtime. Legacy storage entry points retain their OS-specific ABI signatures.

Storage policy stays in `storage.rs`: it resolves DOS names and confines writes
to already opened capability directories. OS rename helpers receive those
opened directories and preserve the no-overwrite contract. Crash reports use a
shared fixed-size buffer; native handlers and context decoding live under `os/`
and retain their allocation-free, lock-free fatal path.

CPU translation is independent of host OS. Shared native assembly and ABI files
stay in `runtime/`; `crates/runtime/host/llasm.rs` and `crates/runtime/rust/llasm.rs` implement
the CPU-state backend selected by Cargo's `llasm` feature. Linux can exercise that
backend as well as the native assembly backend. SDL continues to provide
windowing, input, audio and timing.

Without an explicit game-directory argument, startup checks `game/RGFX.EXE`
beside the executable, then `RGFX.EXE` directly beside it, before requesting a
folder. Explicit `--game-dir` and `--choose-game-dir` keep precedence. Image hash
validation still applies to every selected installation.

## Session diagnostics

The native entry point initializes `crates/runtime/rust/logging.rs` before installation
selection. The OS layer redirects native stdout and stderr to an append-only
session file, so C diagnostics and the allocation-free fatal handler also reach
the file without a background logging thread. Rust records additionally mirror
to the original console. The original game's own console writes (DOS handles 1
and 2, such as the version banner and exit quote) also go to both, with each line
prefixed `game: `. Seven session logs are retained. Startup records include
the revision, OS, architecture, CPU backend and arguments; exit boundaries record
the reason and status. Panic hooks include a backtrace. Log files are outside the
owned installation: `~/Library/Logs/Maiko` on macOS,
`$XDG_STATE_HOME/maiko/logs` or `~/.local/state/maiko/logs` on Linux, and
`%LOCALAPPDATA%/Maiko/logs` on Windows. `MAIKO_LOG_DIR` overrides that location.
