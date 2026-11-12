# Maiko

Maiko is an experimental native port of *The Elder Scrolls Adventures: Redguard* using static recompilation. It translates the original DOS engine with [M-HT/SR](https://github.com/M-HT/SR) and replaces DOS platform services with native Windows and Linux code. It does not use DOSBox to run the translated game.

You need your own Redguard installation. Game executables, assets, CD images, generated translations and builds are excluded from this source repository. The runtime recognizes the two inspected RGFX.EXE variants in [metadata/image_variants.json](metadata/image_variants.json). Translation currently requires the canonical variant described below.

This project started as an experiment in [redguard-preservation](https://github.com/michidk/redguard-preservation) and is separate from [Cyrus](https://github.com/michidk/cyrus). This branch contains the Windows/Linux implementation.

## Current state

- Windows x64 runs original game code. User testing confirms movement, sound and music, but reports endless rising after one jump on the opening ship, startup bypassing the book, initial acceleration, missing movies and distant fog artifacts. Combat progression and full-game completion remain unverified.
- Linux x64 development tests cover fresh startup into a visible book, Continue disabled without an autosave, fixed-step startup pacing, three released ship jumps, defeat of both ship pirates and the original island transition. Inventory/logbook, isolated save/reload and sample audio also have bounded checks. Island quest progression, hardware audio and distribution support remain unverified.
- Original intro video, configured skips into the ship and a bounded decoded-audio prefix pass Linux and Wine checks using our installed GOG layout, without a runtime media argument. This does not establish movies in the user's installation, full playback or audio/video synchronization.

Build success, Wine execution and real-platform acceptance are recorded separately. Twelve diagnostic traps remain in unsupported mixed-mode system code and stop execution if reached.

See [the build and runtime notes](#reproduce), [the gameplay plan](docs/development/GAMEPLAY_PLAN.md), [research history](docs/research/RESEARCH.md) and [decision trail](docs/research/decisions.tsv). Historical research links into ignored artifacts refer to the original development workspace; those artifacts are not published here.

## Repository layout

- `tools/`: translation, build, verification and packaging commands. Run entry points as `python3 -m tools.<name>` from the repository root.
- `runtime/rust/`: shared Rust host services, with the core Cargo package at the repository root.
- `runtime/host/`, `runtime/glide/`, `runtime/probe/`: game dispatcher, Glide bridge and diagnostic Rust crates.
- `runtime/`, `linux/`, `glide/`, `audio/`: assembly ABI, C ABI headers and upstream dependency build rules.
- `diagnostic/`: the separate early translated-entry probe, not the game runtime.
- `metadata/`, `native-metadata/`, `probe-only/`, `patches/`: translator inputs and upstream changes.
- `tests/`: synthetic checks and game-backed development probes.
- `docs/development/`: implementation plans and validation notes.
- `docs/research/`: research history and the decision trail.
- `.cache/`, `artifacts/`: ignored dependencies, generated translations and local test output.

## Reproduce

Run the source-only checks without game data or Docker:

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features -- -D warnings
sh tests/check_portable.sh
```

Install Rust through rustup; `rust-toolchain.toml` pins the compiler and Windows GNU standard library. Cargo uses its configured shared target directory; the scripts do not override it. Source checks also require SDL2 and SDL2_mixer development libraries and pkg-config. The owned runtime implementations are Rust; migration acceptance gates and retained independent C ABI tests are described in [RUST_PLAN.md](docs/development/RUST_PLAN.md).

Use Linux, Python 3.11 with venv support, GCC/G++, Git and Docker. The Docker game builds and package verifier currently require bind-mounted inputs to resolve below `/home/vibepod/workspaces`. This applies to `tools/build_native.py`, `tools/build_linux.py` and `tools/verify_native_package.py`, including selected game, CD-image and verifier input paths. The scripts retain this development-environment restriction; a clone elsewhere can run the source-only checks, but must stage those inputs under that directory for Docker runs. The source build installs pinned SCons into a local virtual environment. Everything downloaded or generated is ignored under `.cache/` and `artifacts/`.

```sh
export REDGUARD_DIR=/path/to/Redguard
python3 -m tools.pipeline inspect
python3 -m tools.pipeline build

# Honest compatibility gate: currently fails at a mixed-mode far transfer.
python3 -m tools.pipeline translate

# Explicit diagnostic mode: unresolved CPU/system boundaries become fatal traps.
python3 -m tools.pipeline translate --probe-traps

docker build -t redguard-recomp-tools:bookworm .
# Substitute the artifact directory printed by the preceding translation.
python3 -m tools.build_windows artifacts/TIMESTAMP-probe --wine
```

The Docker build downloads tools; the build/run container has networking disabled and no game-install mount. Containers are removed after each run. `--wine` checks the expected diagnostic exit and message, not gameplay. The generated report keeps `menu_verified` and `native_windows_verified` false.

The translation input is pinned to SHA-256 `844a316b5f71e3c7fadf4b012ab63c5c13aa3606a4f1504f852ac1607ee4dfdf`. It reads the installation without changing it. Source code is pinned to SR `ac690ddf3010bc3d2c875cb1b5e8da4f53a8d5b3` and patched by `patches/sr-x64.patch`.

## Continue the native target

```sh
python3 -m tools.build_glide --install /path/to/Redguard
python3 -m tools.pipeline translate --native --install /path/to/Redguard
python3 -m tools.build_native artifacts/TIMESTAMP-native --install /path/to/Redguard
```

The native build runs synthetic adapter, file and platform tests before the original game. It mounts the install read-only, uses the shared Rust runtime and SDL services, and keeps unresolved CPU instructions as explicit fatal traps. Original CRT constructors execute before the original main function. A successful build is not a successful menu run; inspect `report.json` and `native-wine.log` for the reached boundary. Automated Windows-target checks run under Wine. The user has tested the game on real Windows; full acceptance remains open.

## Windows startup test

The Rust runtime requires Windows 10 or newer on x64.

Extract the runtime ZIP into its own writable folder. Edit the `GAME_DIR` setting near the top of `RUN.cmd` to the directory containing the original `RGFX.EXE`, then run it. Leaving the setting empty opens a folder selector. `RUN.cmd "C:\path\to\Redguard"` and `redguard-native.exe --game-dir "C:\path\to\Redguard"` also select a directory. Nothing needs to be copied into the game installation. `RUN.cmd` uses its own `user-data` folder for saves and settings. Edit `USER_DIR` to select another location outside the original installation. Existing saves/settings are imported once; later changes and deletions remain local to that folder.

The runtime ZIP contains seven files: the executable, four required DLLs, the editable CMD and consolidated licence notices. Renderer sources, build reports and reference logs are in a separate optional development ZIP. Return `native-test.log` and describe New Game, movement, inventory, sound and saving. Bounded original gameplay passes on Linux and under Wine; full-game completion and full Windows acceptance remain unverified.

The two exact RGFX hashes in `metadata/image_variants.json` are recognized. They differ at three bytes in two branch sites. The native profile preserves both behaviors with a hash-selected, flag-preserving branch adapter. Neither image is established as the unmodified release. All other image hashes remain rejected.

Without `--user-dir`, the game-file adapter denies writes. With that option, original save/config writes go to an isolated folder outside the installation. The graphics wrapper may create its configuration and log in the selected game directory. Wine development checks and actual Windows game rendering remain separate acceptance fields.

## Native Linux feasibility target

The [platform boundary and continuation plan](docs/development/PLATFORM.md) describes shared code and remaining host differences.

The shared Rust runtime provides clocks, files, isolated save/config storage, installation validation, SDL window/input/audio integration and DOS services on both hosts. OpenGLide provides the graphics context and Glide implementation. Native bindings handle low-address memory, folder selection and fatal exception or signal reporting.

The Linux x86-64 host uses the same native SR output as Windows. A guarded native startup adaptation opens the original book without an autosave, selects New Game and disables Continue until a save is admitted. This deliberately replaces the original fresh-start auto-intro behavior. The explicit MENU.INI ignore_autosave=1 direct-start override is preserved. Its OS adapters provide low-address memory, read-only DOS file/path/search behavior, executable hashing and signal diagnostics. These are compatibility services that SDL does not supply.

```sh
python3 -m tools.build_linux artifacts/TIMESTAMP-native \
  --install /path/to/Redguard \
  --menu-autosave /path/to/SAVEGAME.SAV
```

Add `--installed-gog --movie-skip --sound` to verify automatic movie discovery from the installed GOG layout as well as the original intro/skip-to-ship sequence. These are development-test options; the normal runtime needs only `--game-dir`.

The install must resolve below `/home/vibepod/workspaces` for the Docker mount. The command creates a separate Linux artifact directory and leaves the input translation and install unchanged. It builds an ELF64 executable and replaceable Glide library, runs original-function and adapter regressions, then exercises Options, Escape and original Quit through X11 keyboard events under Xvfb/Mesa. No Wine is involved. The scenario uses a copied autosave and disables sound/CD audio by default. Add `--sound --redbook` to test sample audio and the explicit absent-CD-device path. Use `--test-image /path/to/RGFX.EXE` to check either inspected image in that disposable scenario.

This is a Linux feasibility target, not a distributable Linux release. It currently depends on Debian 12 SDL2, SDL_mixer, OpenGL and C/C++ runtimes. Other distributions, Wayland, other graphics drivers, audible hardware output, sustained gameplay and ARM game execution remain unverified. The report distinguishes the tested Linux menu sequence from overall menu and real Windows acceptance.

The Linux test suite also exercises 9000 simultaneous guest allocations, long host filenames during DOS directory searches, keypad/extended-key mappings and a native SDL window pointer above 4 GiB. The Glide window argument preserves the full host pointer. Captures are OpenGLide back-buffer readbacks, not screenshots of the presented desktop window.

Remaining Linux limits include incomplete game-loading coverage, unsorted DOS searches without generated long-name aliases and signal alternate stacks installed only on the initializing thread. Isolated save/config writes are available with `--user-dir`; guest memory uses the shared SR pooled heap. SDL's SIGINT/SIGTERM behavior during a game loop that stops pumping events remains untested; use an external timeout with a kill escalation for manual runs. The current 618 NASM byte-immediate warnings were audited against matching generated source and are sign-extended negative byte immediates. This does not prove unexercised translated code correct.

Add `--gameplay both --gameplay-seconds 30` to record bounded Continue/New Game observations. `--world-controls` measures original player position after walking and attempts in-game Quit. With `--writable-gameplay`, that stricter probe now completes original autosave and Quit. Add `--save-roundtrip` to verify exact player-position persistence in a second process, and `--exercise` for original inventory open/close and bounded action input. `--modal-ui` on Continue seeds one synthetic logbook entry in disposable storage and checks the original logbook interface. Map opening is reported separately and remains unverified in the current seed. A timeout or observation-only report is never full-game acceptance. See [whole-game plan](docs/development/GAMEPLAY_PLAN.md).

The game now uses SR's pooled below2GiB heap through a shared native wrapper. The build exports and hashes the exact upstream sources from the pinned revision. DOS path and file policy is shared Rust code; native bindings handle operating-system details.

## Files

- `tools/sr_audio.py`, `audio/` and `tools/windows_sdl.py`: pinned SR sample playback through SDL_mixer and official Windows SDK staging.
- `runtime/rust/storage.rs` and `runtime/rust/files.rs`: shared paths, DOS files and isolated opt-in save/config storage.
- `tools/sr_heap.py` and `runtime/rust/memory.rs`: pinned pooled SR heap staging and checked native allocation API.
- `tools/le_image.py`: hash-checked LE inspection and page loading.
- `tools/pipeline.py`: pinned dependency build, metadata validation, translation and assembly compaction.
- `metadata/`: exact-build roots and overlapping-instruction adaptations.
- `probe-only/`: explicit fatal substitutions for unsupported boundaries.
- `runtime/`: Rust services, game/Glide/diagnostic crates and assembly ABI contracts. Every unimplemented service terminates.
- `linux/` and `tools/build_linux.py`: native ELF build rules and Linux game verification.
- `tools/build_glide.py` and `glide/`: pinned native renderer build, pixel smoke tests, guest layouts and original-stub mapping.
- `tools/build_native.py`: native runtime integration, ABI/input/file tests and original startup tracing.
- `tools/build_windows.py`: reproducible Windows linking and Wine boundary check.
- `tools/package_native.py`: package the observed Windows gameplay test and renderer sources/notices.
- `tools/verify_native_package.py`: extract and run the actual seven-file ZIP under Wine with a host D: install, the shipped GAME_DIR preset, no-image menu navigation and saved path-error exits. Optional media mode also checks an independent movie PCM reference.
- `tests/check_movie_pcm.py`: compare a bounded original Smacker source prefix byte-for-byte with FFmpeg in the Linux development image.
- `runtime/rust/media.rs` and `tests/check_cd_image.py`: shared read-only ISO/MODE1 CD root-file adapter and synthetic sector/handle tests.
- `tools/package_diagnostic.py`: package a hash-matched Wine-verified diagnostic and runtime notices.
- `Dockerfile`: isolated MinGW/NASM/Wine toolchain.

## Acceptance still required

The original translated book menu must draw, respond to input and exit in a normal Windows window. A linked executable, a recreated menu, or an emulator-assisted run does not satisfy that milestone. Wine evidence now includes the original book, Options navigation, Escape recovery and clean original Quit from the actual extracted ZIP. The user has exercised gameplay on real Windows; full menu, other menu paths and focus acceptance remain open. Do not ship the diagnostic substitutions as game support.

Launcher default: `RUN.cmd` presets `GAME_DIR=D:\Games\GOG Galaxy\Redguard\Redguard`. Keep the runtime outside the game directory. Assets load directly from that installation. Shared Rust code discovers GOG movie data named `game.gog` in the selected game directory or its parent automatically. Root-level DOS D: SMK reads also support matching loose files in GAME_DIR, MOVIES, ANIMS or VIDEO. No media setting is needed for the tested installed GOG layout. The runtime reads installed movie data without extracting or modifying it. A missing bundle preserves loose-file lookup; a present but invalid or ambiguous bundle stops startup with a diagnostic.

Optional movie media: Pass `--game-dir PATH --user-dir SAVE_PATH --cd-image PATH_TO_GAME_GOG` to use movies from the owned CD image. This advanced command-line option is separate from the normal launcher. The adapter accepts ordinary ISO2048 and MODE1/2352 images, supports root files required by the shipped movie layout, and denies writes. It does not implement CD music, general MSCDEX or nested ISO directories.

Report fields: `movie_video_skip_to_ship_verified` covers observed intro frames, skip points and world entry. `movie_playback_verified` requires broader movie acceptance including audio, and remains false. `cd_playback_verified` means CD music playback, not reading files from the image.

Windows exposes original game cwd and current-drive queries as virtual C:, while the selected CD image supplies D:. The selected host game/user directories stay separate. A real Windows drive-letter, UNC or alias installation still requires acceptance on that machine.

Movie-audio checks now convert to the negotiated device rate and record all consumed frames separately from nonzero and starved frames. An opt-in developer capture, `RG_MOVIE_PCM_CAPTURE`, records at most256KiB of pre-conversion source PCM for independent comparison. It is absent from normal launcher runs and packages contain no captured PCM. The Linux movie gate compares32768 bytes with FFmpeg after2s uninterrupted playback. Correct decoding of that prefix and zero starvation in the skip scenario do not establish full-movie quality, hardware playback or synchronization.

Windows gameplay feedback, 2026-11-12: the user reports movement and audio working, a single jump on the opening ship that never falls, immediate ship startup, initial acceleration, missing movies and distant fog artifacts. The same shipped executable rises, descends and lands in Wine on both ship and island. These checks do not resolve the Windows report. New bounded STATE lines record the original position, jump/fall state, animation vertical-control flag, jump key, fly flag and timing fields. The original gravity and simulation arithmetic is retained. Current development source deliberately opens the book on a default fresh start and paces the original fixed 12-Hz loading warm-up. Three released ship jumps pass on Linux and on the rebuilt Windows executable under Wine. These checks do not resolve the real-Windows report; the older shipped download predates these changes. Copied-autosave tests and fresh-start tests remain separate.
