# macOS backend work

M-HT/SR's [Albion build configuration](https://github.com/M-HT/SR/blob/ac690ddf3010bc3d2c875cb1b5e8da4f53a8d5b3/games/Albion/SR-Main/SConstruct#L259) selects llasm/LLVM for both Intel and ARM64 Macs. Apple Silicon can use native ARM code through that route. Our Redguard macOS game port is incomplete.

From the repository root:

```sh
python3 -m tools.check_macos_backend
```

Requires Python 3.11 or newer, Docker, Rust 1.90 and the pinned SR/OpenGLide checkouts under `.cache/`. Install Rust's `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin` and `x86_64-apple-darwin` targets with `rustup target add`. The script downloads the SDK and SDL frameworks if absent. The SDK comes from the third-party joseluisq/macosx-sdks mirror; its hash matches that mirror's published checksum, not an Apple-signed checksum. SDL downloads use official release URLs with locally pinned content hashes. These checks detect changed bytes; they do not independently authenticate the initial downloads. The SDK stays in local build tooling and is excluded from runtime packages.

The script builds pinned upstream source archives without modifying the existing Windows translator. The Debian image tag and package versions are recorded, but apt package resolution is not fully pinned. The source is committed on `feat/macos-backend` and reviewed in draft PR #1. Every build records content hashes of its local source inputs in `source-manifest.json` and the report, and rejects a source change during the build. Upstream archives, patches and output artifacts are hashed separately.

The command executes a synthetic translated-code test on native x86-64 Linux and under ARM64 Linux QEMU user emulation. The ARM test exercises Linux ARM64 code generation, not the Darwin ABI or hardware. The pipeline uses upstream's `-m64 -inline-idiv -pic -ptrofs` LLasm flags and `opt -O3` followed by target-specific `llc --relocation-model=pic`. Tests include signed/unsigned division, x87 arithmetic and shared comparison flags with signed overflow cases.

It then cross-links that test, Darwin memory-query tests, the Glide renderer and a triangle smoke test for both Mac architectures. The instruction fixture links the production Rust CPU helpers. Darwin memory tests link the shared Rust runtime, including its Mach memory queries, instead of a separate C implementation. These links provide compile coverage, not runtime acceptance. The memory diagnostic uses the normal runtime feature set and does not initialize the offset arena. Physical-page accounting under Rosetta remains unverified. Reports and tool versions are under `artifacts/`. The default command builds developer diagnostics without translated Redguard code and keeps all game verification fields false.

To also cross-link the complete original engine, first emit a translation using `tools.translate_llasm` as described in [the metadata instructions](../llasm-metadata/README.md), then run:

```sh
python3 -m tools.check_macos_backend \
  --translation artifacts/<timestamp>-llasm-analysis
```

This builds `redguard-native-arm64` and `redguard-native-x86_64` with the Rust CPU-state host, upstream offset heap, audio support and renderer. It requires at least 60 GB of free disk space and gives the offline compiler container 6 GB of memory. The full engine uses `opt -O2` and `llc -O=2`; the smaller diagnostics use O3. A successful link sets `macos_game_built`, while `macos_runtime_verified` and `full_game_verified` remain false.

Both complete engine binaries have cross-linked. The same CPU-state/offset backend starts the original engine on Linux, and original clipping, numeric parsing, packed vertex, animation flags and attribute tests pass. The strict Linux book-menu check verifies fresh-start selection, disabled Continue, Options navigation, different rendered pages and clean original Quit. A bounded New Game probe reaches the ship, opens and closes inventory, delivers sword and attack inputs and observes vertical jump motion. Dummy audio and software GL were used, so this does not establish hardware audio or rendering. An upstream emitter fix preserves indexed x87 store values while calculating their relocated destinations; without it, the camera translation became zero and the menu rendered gray. The instruction fixture covers those floating, integer, control-word and status-word stores. No game or diagnostic has run on macOS.

With the original disc image present, the Linux offset backend decodes movie PCM, skips the intro's configured frame points, detects CD music tracks and reaches initialized ship gameplay. A missing zero-flag producer for absent Smacker audio tracks previously sent a zero sample rate to the audio service; the flag metadata now preserves that producer at the shared branch. The longer movie probe still times out at its final Quit step, which also reproduces on the existing native Linux backend. It does not pass the complete movie acceptance check.

The triangle test needs the matching `libredguard-glide-ARCH.dylib` renamed to `libredguard-glide.dylib` beside it and the official SDL2 framework in a `Frameworks` directory beside it. It writes `backend-smoke.ppm` in the working directory. Full-game builds also stage a `runtime/` directory with both executables, a universal renderer, SDL frameworks and `RUN.command`. The launcher chooses the host architecture and resolves the adjacent `game/` directory relative to itself. The SDK and compiler remain outside that directory. This staging folder still needs the owned game files and licence notices before packaging.

`assemble.py` retains the initial isolated NASM ABI experiment. The original game's 32-bit absolute relocations do not assemble through that adapter, so it is not the selected game backend.

See [the implementation checkpoint](../docs/development/MACOS_PLAN.md) for the remaining game translation, CPU-state adapter, guest arena and real Mac checks. Existing Windows/Linux game builds continue using their established backend.

For a personal test ZIP, add the owned installation under `runtime/game/`, including `game.gog` and `game.ins` for disc media. Include Maiko, SR, OpenGLide, SDL, SDL_mixer and Rust dependency notices, and provide the renderer's corresponding source and build files alongside the ZIP. Preserve the framework symlinks and the launcher's executable permission in the archive. The resulting package can be extracted and launched without copying a separate Windows installation. Mark it as cross-built and unverified on macOS until a real Mac run passes. Game assets belong only in an authorized personal test download, not Git or source archives.
