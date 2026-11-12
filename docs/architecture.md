# How Maiko works

Maiko has two halves: a translation of the original program, and a native runtime that stands in for the DOS system it expected.

## Translation

[M-HT/SR](https://github.com/M-HT/SR) statically translates the 32-bit DOS engine (`RGFX.EXE`) into x86-64 NASM source. `tools/pipeline.py` drives it with a pinned SR revision, a small patch (`patches/sr-x64.patch`) and the metadata in `metadata/` and `native-metadata/`: label aliases, ranges that are data rather than code, the external procedures the engine calls, and byte-level replacements for the few places where the original cannot run natively.

The generated assembly is local and never committed. The build records the SHA-256 of the executable it translated and checks the original bytes at every replacement site.

### Places the original is changed

- **DOS and BIOS services** (keyboard interrupts and LEDs, VGA mode switch, timers, the Miles sound driver, CD audio) are replaced by calls into the runtime.
- **Reads through null pointers.** DOS lets a program read low memory at address 0 and above, so the original sometimes dereferences a null pointer harmlessly. A native process cannot map that page and faults. A few sites are guarded to return what DOS did.
- **Two known `RGFX.EXE` builds** differ by three bytes; both run, selected by hash.
- **Startup behavior.** A fresh start shows the book menu with Continue disabled, and the fixed-rate loading clock is paced on the host. These are the deliberate, visible differences from the original.
- **Unsupported instructions** (far jumps into 16-bit code, descriptor-table and privileged instructions) become traps that stop the game with a `BLOCKED` message.

Each of these is documented at the site that implements it; see "What changes the game's bytes" in [building.md](building.md).

## Runtime

A Cargo workspace holds the runtime:

| Crate | Path | Role |
|---|---|---|
| `maiko-runtime` | `runtime/rust/` | Installation validation, installed-movie discovery, DOS-style files and searches, isolated save and settings storage, clocks, SDL window, input and audio, guest memory, crash reporting. Shared by Windows and Linux. |
| `maiko-glide` | `runtime/glide/` | Converts the original Glide calls and their pointer-bearing structures to the native OpenGLide-ng renderer. Unimplemented imports fail explicitly. |
| `maiko-host` | `runtime/host/` | Startup, the service dispatcher called by the translated code, and keyboard callbacks into the engine. |

Small assembly files (`runtime/main.asm`, `service.asm`, `native_entry.asm`) and the C headers in `runtime/` define the call interface between translated code and Rust. Independent C and C++ test programs in `tests/` check that interface's layout and calling convention; they are not part of the game.

Rust's standard library provides files, paths, threads and clocks. `cap-std` confines save and settings writes to an independently opened user directory, so the original installation is never modified. SDL provides one window, keyboard, mouse and audio implementation for both platforms. Operating-system bindings remain for memory below 4 GiB, memory-protection queries, folder selection and fatal error reporting: Windows uses structured exceptions and Linux uses signals.

### Graphics

The original Glide calls go to [OpenGLide-ng](https://github.com/eugenenuke/openglide-ng), built from a pinned revision with two small patches and linked as a separate library. `tools/build_glide.py` builds it, `glide/imports.py` reads the original's Glide import table and generates the call wrappers, and `tests/backend_smoke.cpp` and `tests/bridge_test.cpp` check the renderer and the argument decoding.

### Movies and storage

Movies are root-level DOS `D:` Smacker files. The runtime looks first at an explicitly selected CD image, then at the GOG `game.gog` bundle in the game folder or its parent, then at loose `.SMK` files in the game folder, `MOVIES`, `ANIMS` and `VIDEO`. Media and game assets are read-only. Original save and settings writes go to the user folder only.

## Where to look

- `tools/`: translation, build, verification and packaging commands (`python3 -m tools.<name>`).
- `tests/`: unit and ABI tests, plus the game-driving probes described in [building.md](building.md).
- `linux/`, `glide/`, `audio/`: build rules and small headers for the Linux target and the upstream dependencies.
- `patches/`: changes to pinned upstream sources.
