# Rust runtime migration

Maiko's owned Windows/Linux runtime is implemented in Rust. The translated original engine and handwritten ABI assembly remain intact. SR's allocator and sample decoder, SDL and OpenGLide remain external C/C++ dependencies. No Mac work is included in this migration.

## Structure

The Cargo workspace separates reusable services from libraries that require translated game globals or the renderer.

- `maiko-runtime`, in `runtime/rust/`, owns installation validation, installed GOG media discovery, DOS files and searches, isolated save/config storage, clocks, SDL input/audio, guest allocations, operating-system services and crash reports.
- `maiko-glide`, in `runtime/glide/`, converts the original DOS Glide arguments and pointer-bearing structures to the native renderer ABI.
- `maiko-host`, in `runtime/host/`, owns game startup, service dispatch and keyboard callbacks into the translated engine.
- `maiko-probe`, in `runtime/probe/`, retains the separate historical diagnostic entry program.

`main.asm` and `service.asm` forward to Rust exports. Shared services use Rust std, SDL bindings and capability-based filesystem operations. Platform-specific code is limited to native handles, folder selection, guest-addressable memory and OS diagnostics.

Keeping Glide separate prevents file-only binaries from importing the renderer DLL. Keeping the game host separate prevents standalone contracts from referencing translated-game globals. The diagnostic compiles the shared crash-reporting module with only its OS dependencies, so it does not link SDL or the game allocator.

## Source policy

`python3 -m tools.audit_runtime --check` rejects owned production C/C++ in the runtime, Linux, audio, Glide and diagnostic directories. Replaced production implementations have been deleted.

Independent C/C++ test programs remain in `tests/`. They check Rust's exported C ABI against C headers, the assembly calling convention and OpenGLide's native structures. They are not linked into the shipped game. Existing Python build and game-driving tools remain Python. Upstream dependency sources are staged from pinned revisions and are not counted as owned runtime code.

## Acceptance gates

- Rust formatting, workspace Clippy and unit tests.
- Windows and Linux file, storage, media, input, audio, memory and crash-report contracts.
- Calls into original translated functions, including normal-exit callbacks and intentional fatal exceptions.
- Original book/Options/return/Quit sequence and installed-GOG intro/skips/ship sequence.
- Inventory open/close and separate-process save/reload.
- Actual release packager and extracted seven-file Windows ZIP, including the shipped game-path preset and both accepted original EXE variants.

Detailed run results belong in [VALIDATION.md](VALIDATION.md) and the [decision trail](../research/decisions.tsv). A successful Wine run does not establish real-Windows acceptance. Movie checks use the SDL dummy audio device and do not establish hardware playback or audio/video synchronization.

Gameplay fixes follow this migration. The reported endless jump, combat, startup burst and distant fog remain separate work in [GAMEPLAY_PLAN.md](GAMEPLAY_PLAN.md). The existing Mac branch and PR remain untouched.
