# Building and testing Maiko

Maiko translates your own copy of the game's `RGFX.EXE` into native code, links it with the Rust runtime and packages the result. The translated engine is derived from the game, so builds are made locally from your installation and are not committed to this repository. Everything downloaded or generated goes into the ignored `.cache/` and `artifacts/` folders.

For a package from an accepted runtime snapshot, use the [client-side browser builder or Rust CLI](client-builder.md). The steps below build and test the runtime itself.

## Requirements

- Linux x86-64 (the Windows runtime is cross-built and checked under Wine in a container).
- Python 3.11 with `venv`, Git, GCC/G++ and Docker.
- Rust through [rustup](https://rustup.rs/). `rust-toolchain.toml` pins the compiler and the Windows GNU standard library.
- For the Rust tests only: SDL2 and SDL2_mixer development libraries and `pkg-config`.
- A Redguard installation containing `RGFX.EXE` (GOG tested). The translator accepts exactly the build with SHA-256 `844a316b5f71e3c7fadf4b012ab63c5c13aa3606a4f1504f852ac1607ee4dfdf`; the runtime also recognizes a second build listed in [`metadata/image_variants.json`](../metadata/image_variants.json).

The Docker steps bind-mount your game folder and any CD image read-only. Mounted paths must resolve below the repository's parent folder, or below the folder named by `MAIKO_MOUNT_ROOT`. Containers run without network access and are removed after each run.

## Source checks (no game needed)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
sh tests/check_portable.sh
python3 tests/check_patch_tables.py
python3 -m tools.audit_runtime --check
```

Cargo uses its configured shared target directory; do not override it.

## Build the Windows runtime from your own copy

Run from the repository root. `REDGUARD_DIR` is the folder containing `RGFX.EXE`.

```sh
export REDGUARD_DIR=/path/to/Redguard

python3 -m tools.pipeline build                      # fetch and build the pinned SR translator
python3 -m tools.build_glide --install "$REDGUARD_DIR"   # build the OpenGLide-ng renderer
python3 -m tools.pipeline translate --install "$REDGUARD_DIR"
```

`translate` prints an artifact folder such as `artifacts/20260101T000000000000Z-native`. Then build and test it:

```sh
python3 -m tools.build_native artifacts/<timestamp>-native \
  --install "$REDGUARD_DIR" \
  --menu-autosave /path/to/SAVEGAME.SAV \
  --gameplay both --modal-ui --exercise --save-roundtrip --sound --writable-gameplay
```

`--menu-autosave` is a copy of one of your own save games, made while standing on the opening ship; the tests use it to reach gameplay. Add `--installed-gog --movie-skip` to also test the intro movie from a GOG layout. The build runs synthetic adapter, file and platform tests first, then drives the original game under Wine and records what it observed in `report.json`. Nothing is packaged unless the observed checks passed.

```sh
python3 -m tools.package_native artifacts/<timestamp>-native --output dist
python3 -m tools.verify_native_package dist \
  --build artifacts/<timestamp>-native \
  --install "$REDGUARD_DIR" \
  --menu-autosave /path/to/SAVEGAME.SAV \
  --test-image "$REDGUARD_DIR/RGFX.EXE"
```

`dist/` then contains `redguard-native-startup-test.zip` (the runtime that users extract; see the [README](../README.md)), a separate optional ZIP with the renderer source and build files, and checksums. `verify_native_package` extracts the real ZIP and runs the unmodified launcher under Wine. With `--installed-gog` it also needs `--pcm-reference`, the report written by `tests/check_movie_pcm.py` during a Linux build.

## Linux build

```sh
python3 -m tools.build_linux artifacts/<timestamp>-native \
  --install "$REDGUARD_DIR" --menu-autosave /path/to/SAVEGAME.SAV
```

This builds the ELF executable and a replaceable Glide library from the same translation, runs the adapter and original-function tests and exercises the book menu through X11 key events under Xvfb and Mesa. Add `--gameplay`, `--exercise`, `--sound` and the other flags (see `--help`) for the longer checks, and `--keep-container` to keep the container for follow-up work. This is a development target, not a Linux release.

## Game-driving probes

`tests/probe_*.py` and `tests/check_*.py` drive the original game through keyboard input and read its state from memory without modifying it. `build_linux` and `build_native` run the ones they need; each script's docstring says what it checks and what it does not establish. A passing Linux or Wine run is not real-Windows acceptance, and dummy audio and software rendering say nothing about hardware playback.

## What changes the game's bytes

Every place the build changes or replaces original game code is documented next to the code that does it:

- `native-metadata/instruction_replacements.sci` and `metadata/instruction_replacements.sci`: byte-level replacements. Each entry has a comment block (what the original does, what replaces it, effect, reason, evidence) and the SHA-256 of the original bytes. The build refuses an undocumented or unhashed entry and stops if the game's bytes differ. `tests/check_patch_tables.py` checks the tables.
- `tools/pipeline.py`: the hooks that edit the generated assembly, each documented in its docstring.
- `patches/*.patch`: changes to the pinned upstream SR and OpenGLide-ng sources, each with a header at the top.

Update that documentation in the same commit as any patch change.
