# macOS backend work

M-HT/SR's [Albion build configuration](https://github.com/M-HT/SR/blob/ac690ddf3010bc3d2c875cb1b5e8da4f53a8d5b3/games/Albion/SR-Main/SConstruct#L259) selects llasm/LLVM for both Intel and ARM64 Macs. Apple Silicon can use native ARM code through that route. Our Redguard macOS game port is incomplete.

From the repository root:

```sh
python3 check_macos_backend.py
```

Requires Python 3.11 or newer, Docker and the pinned SR/OpenGLide checkouts already used by the experiment, under `.cache/`. The script downloads the SDK and SDL frameworks if absent. The SDK comes from the third-party joseluisq/macosx-sdks mirror; its hash matches that mirror's published checksum, not an Apple-signed checksum. SDL downloads use official release URLs with locally pinned content hashes. These checks detect changed bytes; they do not independently authenticate the initial downloads. The SDK stays in local build tooling and is excluded from runtime packages.

The script builds pinned upstream source archives without modifying the existing Windows translator. The Debian image tag and package versions are recorded, but apt package resolution is not fully pinned. This experiment is currently uncommitted. Every build records content hashes of its local source inputs in `source-manifest.json` and the report, and rejects a source change during the build. Upstream archives, patches and output artifacts are hashed separately.

The command executes a synthetic translated-code test on native x86-64 Linux and under ARM64 Linux QEMU user emulation. The ARM test exercises Linux ARM64 code generation, not the Darwin ABI or hardware. The pipeline uses upstream's `-m64 -inline-idiv -pic -ptrofs` LLasm flags and `opt -O3` followed by target-specific `llc --relocation-model=pic`. Tests include signed/unsigned division, x87 arithmetic and shared comparison flags with signed overflow cases.

It then cross-links that test, Darwin memory-query tests, the Glide renderer and a triangle smoke test for both Mac architectures. It also compiles shared POSIX file/storage/host modules and SDL input/timers to Darwin objects. This is compile coverage, not runtime acceptance. The region query can legitimately report zero low-address space under PIE; it does not prove a game arena can be allocated. Physical-page accounting under Rosetta remains unverified. Reports and tool versions are under `artifacts/`. These executables contain no translated Redguard code. They are developer tests, not a game download. The report keeps `macos_game_built`, `macos_runtime_verified` and `full_game_verified` false.

The triangle test needs the matching `libredguard-glide-ARCH.dylib` renamed to `libredguard-glide.dylib` beside it and the official SDL2 framework in a `Frameworks` directory beside it. It writes `backend-smoke.ppm` in the working directory. No assembled runtime package is supplied yet. Nothing here has run on macOS.

`assemble.py` retains the initial isolated NASM ABI experiment. The original game's 32-bit absolute relocations do not assemble through that adapter, so it is not the selected game backend.

See [the implementation checkpoint](../MACOS_PLAN.md) for the remaining game translation, CPU-state adapter, guest arena and real Mac checks. Existing Windows/Linux game builds continue using their established backend.

The requested game delivery is a separate Mac ZIP linked from the preview download page. It must include the original game files under `game/`, required Mac runtime dependencies and a `RUN.command` that resolves its game directory relative to itself. The user should be able to extract and launch without copying a Windows installation. This package does not exist yet; the current backend diagnostics do not satisfy that requirement. Game assets belong only in the requested test download, not Git or source archives.
