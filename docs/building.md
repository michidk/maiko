# Source checks

For a runnable package from your own installation, use the [client-side browser builder or Rust CLI](client-builder.md). You need to own Redguard; game files and generated packages do not belong in this repository.

## Requirements

- Rust through [rustup](https://rustup.rs/). `rust-toolchain.toml` pins the compiler.
- SDL2 and SDL2_mixer development libraries and `pkg-config` for the runtime tests.
- Python 3.11 and a C/C++ compiler for the existing independent interface tests.
- Bun for the static website.

## Checks without game files

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
sh tests/check_portable.sh
python3 tests/check_patch_tables.py
python3 -m tools.audit_runtime --check
bun install --frozen-lockfile
bun test web
```

Cargo uses its configured shared target directory; do not override it.

The runtime and package builders have command-line help for their supported options. Downloaded dependencies and generated output stay in ignored `.cache/` and `artifacts/` directories. Original installations and media are read-only inputs. Generated packages are for the owner's local use and must not be uploaded as release assets.

A Linux or Wine check does not establish real-Windows acceptance. Dummy audio and software rendering checks do not establish hardware playback or rendering.
