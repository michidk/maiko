# Platform build inputs

| Directory | Build and launch files |
|---|---|
| `linux/` | Native ELF renderer and runtime builds, function checks, POSIX ABI headers and `RUN.sh` |
| `windows/` | MinGW runtime build and Wine checks, Windows storage ABI header, `RUN.cmd` and base compiler container |
| `macos/` | LLasm backend checks, renderer cross-build, Darwin probe header, `RUN.command` and [launch instructions](macos/LAUNCH.md) |

Shared build orchestration remains in `tools/`. Its commands run from the
repository root, for example `python3 -m tools.build_linux --help` and
`python3 -m tools.check_macos_backend --help`. They stage the relevant platform
files into isolated compiler containers. The Linux container extends the base
compiler image defined by `windows/Dockerfile`; it includes the common native
assembler and compiler tools as well as MinGW and Wine.

Native Rust implementations live under `runtime/rust/os/`. Shared DOS services,
CPU backend glue, SDL services and renderer bindings remain in `runtime/`.
See [the runtime architecture](../docs/architecture.md) and
[source checks](../docs/building.md).
