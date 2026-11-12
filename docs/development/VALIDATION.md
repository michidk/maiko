# Publication checkpoint

The source was extracted from the Redguard preservation experiment on 2026-11-12. This repository contains source, patches, metadata, synthetic tests and research notes. It contains no original game files, generated translations, binaries, SDKs, dependency checkouts or private agent state.

## Checks performed for the extraction

- Python source compilation passes.
- `sh tests/check_portable.sh` passes without game data. It compiles the actual CD-image and POSIX save/file adapters, tests ISO2048 and raw2352 reads and malformed images, and exercises isolated saves, immutable game assets, path/symlink rejection and interruption-safe import.
- `python3 -m tools.pipeline build` builds the pinned SR source from a fresh dependency checkout.
- `python3 -m tools.pipeline translate --native --install PATH` emits native assembly from the canonical owned RGFX.EXE. `rgfx.asm`, `seg02.inc`, `seg05.inc` and `seg07.inc` match the translated source used for the previously delivered Windows executable byte-for-byte.

The previous Windows executable was SHA-256 `f703639037267bf6ec741e21582b69dd2551d979f5896b73d10bb2d23b6ae926`. No executable is committed. This extraction does not include a fresh complete Windows/Linux game rebuild or a new gameplay acceptance run. The native runtime sources are preserved, with Mac-specific conditional additions excluded from this baseline.

## Limits

The Windows user's endless ship jump, startup behavior, missing movies, fog artifacts and combat/progression remain unresolved. Linux/Wine development gates cover bounded scenarios, not completing the game. Hardware audio, movie synchronization, distribution packaging and wider gameplay coverage remain open. Historical evidence paths in RESEARCH.md and decisions.tsv refer to ignored local development artifacts. They are not available in a clean clone.

## Shared Windows/Linux movie-file lookup

The installed-movie resolver is shared by both host adapters. The same synthetic file-service fixture passes on native Linux, under ASan/UBSan, and in the Windows x64 build under Wine 8. It covers case handling, folder precedence, file reads and attributes, missing movies and rejected mutations. No game files or CD image are needed for this fixture.

Both full hosts were rebuilt with this resolver. The Windows original-function and adapter regressions pass. Its separate 15-second startup observation reaches the configured timeout with no reported fault; that observation does not verify menu navigation or clean exit. The native Linux build passes its original-function regressions and book-menu, Options, Escape and Quit sequence with exit 0. Captured back-buffer frames were inspected and show the expected pages. They do not establish the physical window contents or hardware rendering.

Local reports are `artifacts/20261112T111648496611Z-native/report.json` and `artifacts/20261112T114317044877Z-linux/report.json`. The Windows executable SHA-256 is `5ffb610b0089d234f2e3866e1170f654791ed5dcf7c97cb64e222875ed7fa153`; the Linux executable SHA-256 is `8a31fad8463815d4be4d2a5cba3f30fcdc357712f9af4f2d6fef2992dd1a1c96`.

This change does not establish movie playback from the user's installation or fix the reported real-Windows jump, startup speed, combat or fog issues. At that checkpoint, Windows input and timers used Win32 and Linux used SDL. Timing has since moved to shared Rust as described below. The [platform boundary](PLATFORM.md) records the next consolidation work.

## Shared Rust runtime checkpoint

Rust 1.90.0 now builds the same movie-policy and BIOS/elapsed-clock implementation for Windows and Linux. Three replaced C files were removed. The remaining owned C/C++ runtime and test code has not yet migrated; [RUST_PLAN.md](RUST_PLAN.md) tracks it. The C ABI declares the shared tick as atomic, while translated x64 code continues its aligned 32-bit loads.

Rustfmt, clippy with warnings denied, all three Rust unit tests, four movie-evidence packaging tests and the portable host contracts pass. Both full hosts rebuild and pass their adapter/original-function regressions. Linux passes book, Options, Escape and Quit with exit 0. Its inspected back-buffer captures show those pages, without establishing the presented desktop window or hardware rendering.

The Windows original-function suite runs under Wine 10. Wine 8 cannot load Rust 1.90 Windows std because it lacks bcryptprimitives.dll/ProcessPrng. The tool image now installs checksum-pinned WineHQ 10 packages. This is a development-tool requirement, not a game patch or real-Windows acceptance. The separate 15-second startup observation times out 124 without a reported fault.

An internal seven-file ZIP fixture then passes the original book/Options/Escape/Quit sequence under Wine 10 with the shipped D: preset, the user-supplied RGFX variant and no image mounted. RUN.cmd remains byte-identical to its ZIP member during that run; a missing game path saves exit 1. This fixture contains a placeholder licence file and is not a distributable package. It does not exercise the changed release packager, its Rust licence/media output or its save/inventory/audio gates for this build. The normal launcher no longer has a CD_IMAGE setting. The optional advanced image CLI and synthetic image-reader tests remain.

The relocated diagnostic program independently links and reaches its expected INT21/AH30 boundary with exit 2. It remains separate from the actual game runtime.

Local reports are `artifacts/20261112T121245724369Z-native/report.json`, `artifacts/20261112T121541309334Z-linux/report.json`, `artifacts/20261112T122139973087Z-probe/report.json` and `.cache/rust-migration/package-contract/package-verification.json`. The Windows executable SHA-256 is `d41e171c4ec0923e85060d522b7aaef748bdd85a2ce194db65497fb0a256026b`; Linux is `061eaddec3b9afdcc9542f41fe4b50ff2ef5929b3cf5e116c1424bd6da91629e`.

These changes do not resolve the real-Windows jump, startup speed, combat or fog reports, establish user-install movie playback, or complete the Rust migration. The Mac branch and draft PR remain separate and unchanged.

## Installed GOG movies

Selecting the game directory now discovers GOG movie data automatically in that directory or its parent. Discovery is shared Rust code; the existing read-only sector reader supplies the original Smacker decoder. Loose SMK lookup remains available. An explicit media argument takes priority. The runtime never extracts or changes installed movie files.

Our installed layout is `app/Redguard/RGFX.EXE` with bundled movie data in `app/game.gog`. GOG's `dosbox_redguard_single.conf` mounts `app/game.ins`, whose first FILE record names that data. The native runtime discovers the data directly from the selected game folder, with no DOSBox or GOG launcher process and no media setting in RUN.cmd.

The Linux run `artifacts/20261112T124308262383Z-linux/report.json` passes the original book sequence and intro playback with mixed audio, the configured skip points, ship entry and clean original Quit. Its command passes only game and isolated user directories. FFmpeg independently matches 32768 bytes of decoded original PCM. These are bounded movie checks, not full playback or hardware audio synchronization acceptance.

Two earlier installed-GOG runs reached the ship but failed the final Quit test. Their reports remain failed. The test now holds Escape through the slower world update, waits for book mode and its transition, then taps Up briefly to avoid the original menu repeat delay. It retains the clean-exit requirement. The successful Linux build/test phase took 50.444 seconds; menu navigation took 19.970 seconds and the whole build/verification command took 109.694 seconds.

The Windows build `artifacts/20261112T124032407802Z-native/report.json` also passes installed-GOG intro playback, mixed audio, configured skips, ship entry and original Quit under Wine 10. It additionally passes the original-function suite, inventory open/close and separate-process save/reload. The standalone 15-second startup observation remains a timeout and is distinct from those completed probes. The whole build/verification command took 306.392 seconds. Its executable SHA-256 is `c137a3b8ff4d5cf769284d4d3bfed9976104fbfbebae798817757fc2600e60c3`.

The actual release packager now passes for that build, including its save/inventory/audio gates, installed-GOG movie evidence and Rust licence notice. It collects SR/GCC/MinGW/LGPL notices from the current build tool image rather than an unrelated diagnostic directory. Its output is a seven-file runtime ZIP and a separate renderer-source/development ZIP. The earlier placeholder-notice fixture is not this release package. Real Windows hardware, complete cinematic playback and audio/video synchronization remain unverified.

The real release ZIP also passes `.cache/rust-migration/gog-release/package-verification.json` with the exact unmodified shipped D: preset, the uploaded alternative RGFX image and automatic installed-GOG discovery. It exercises book/Options/Escape/Quit, original movie/skips/ship/Quit, the independent PCM-prefix hash and a saved missing-path exit. The seven-file runtime ZIP SHA-256 is `f3821cb619c6315f94e8eb1ec8bca6eeab9b2b75a4ea4a2ae3a440b6e8eeac4e`. No runtime media argument is supplied. The invocation records the owned game install used; no game assets are included in the ZIP.

The current installed-GOG checkpoint has six Rust unit tests and five movie-evidence tests. The later Python CLI edits added rejection of invalid option combinations (`--installed-gog` without `--movie-skip`, conflicting explicit media, and fresh-start probes with movie media). They did not change the commands used by the completed full-host runs or the compiled Rust/C sources. The final ZIP verifier uses those updated tools. Current syntax, portable, unit, fmt and clippy checks are rerun after the review edits.

The strengthened release-ZIP test places the game directory physically under the mapped D: tree, with bundled GOG data in its parent. It confirms the canonical game path remains `D:\Games\GOG Galaxy\Redguard\Redguard`, automatic discovery opens `..\game.gog`, and the unchanged launcher completes both menu and movie probes with exit 0. The report sets `physical_d_layout_verified: true`. This supersedes the earlier symlink-based fixture; the ZIP and executable hashes are unchanged.
