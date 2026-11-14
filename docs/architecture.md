# Runtime structure

Maiko provides native Windows and Linux services for an owned Redguard installation. The runtime reads game assets from that installation and stores saves and settings in a separate user directory.

## Rust workspace

| Crate | Path | Responsibility |
|---|---|---|
| `maiko-runtime` | `runtime/rust/` | Installation checks, files, storage, timing, input, audio, movies, memory, crash reports and the in-game overlay (egui, drawn before each buffer swap) |
| `maiko-glide` | `runtime/glide/` | Interface to the separately linked graphics renderer |
| `maiko-host` | `runtime/host/` | Startup and runtime service dispatch |
| `maiko-patcher` | `patcher/` | Local package builder shared by the CLI and browser |

SDL supplies windowing, input, timing and audio. OpenGLide-ng supplies the renderer. Platform bindings handle memory, folder selection and fatal error reporting. Dependency sources and licences are listed in [THIRD_PARTY.md](../THIRD_PARTY.md).

## Files and storage

The game installation is read-only. `cap-std` confines save and settings writes to the selected user directory. Installed movies can be read from a selected CD image, a GOG installation or supported loose movie files. Music is read from the audio tracks of the GOG disc image (`game.gog` and its `game.ins` cue sheet) by `runtime/rust/redbook.rs`, which implements the `AIL_redbook_*` calls the game uses and mixes the tracks into the audio output. Maiko does not distribute those files.

## Source layout

- `runtime/`: shared Rust services, interface headers and CPU backend startup glue.
- `runtime/rust/os/`: compile-time host OS implementations for Linux, Windows and macOS, with common POSIX operations under `unix/`.
- `platforms/linux/`, `platforms/windows/` and `platforms/macos/`: platform build scripts, launchers, native headers and development container definitions.
- `patcher/` and `web/`: local package creation and the static website.
- `tools/`, `glide/` and `audio/`: shared build, renderer and packaging support.
- `tests/`: independent fixtures, interface checks and runtime checks.
- `patches/`: documented changes to licensed upstream dependencies.
- `build5-metadata/`: build 5.0 addresses and site hashes. The metadata names build 1.0 addresses; for a build 5.0 input the patcher rewrites them through this table (`patcher/src/image.rs`).

See [building.md](building.md) for source checks and [client-builder.md](client-builder.md) for package creation.

## Host OS boundary

Shared services implement DOS behavior and call the private `os` module for native
memory queries, locking, disk and clock access, path encoding, atomic rename,
file metadata, drive enumeration, folder selection, renderer window handles and
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
stay in `runtime/`; `runtime/host/llasm.rs` and `runtime/rust/llasm.rs` implement
the CPU-state backend selected by Cargo's `llasm` feature. Linux can exercise that
backend as well as the native assembly backend. SDL continues to provide
windowing, input, audio and timing.

Without an explicit game-directory argument, startup checks `game/RGFX.EXE`
beside the executable, then `RGFX.EXE` directly beside it, before requesting a
folder. Explicit `--game-dir` and `--choose-game-dir` keep precedence. Image hash
validation still applies to every selected installation.

## Session diagnostics

The native entry point initializes `runtime/rust/logging.rs` before installation
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
