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

## Completed Windows/Linux Rust migration

The owned Windows/Linux runtime implementations are Rust. All 22 replaced production C/C++ files have been removed. The inventory gate reports zero production C/C++ files and 44 independent C/C++ ABI or renderer fixtures. The original translated engine, assembly ABI, pinned SR allocator/sample decoder, SDL and OpenGLide remain. Python build and test tools remain Python. The Mac branch and PR were not changed.

Final source checks pass with Rust 1.90.0: workspace formatting, Linux and Windows-target Clippy with warnings denied, all 17 Rust unit tests, portable contracts, Python compilation and the production-source inventory. Logs and exact commands are recorded in `.cache/rust-port/accepted-source-checks.json`.

| Target | Local evidence | Result |
|--------|----------------|--------|
| Windows x64 under Wine 10 | `artifacts/20261112T151409391653Z-native/report.json` | 63 host/original-function PASS lines; original inventory open/close, separate-process autosave/reload and installed-GOG intro/skips/ship/Quit pass. |
| Native Linux x64 | `artifacts/20261112T155347607499Z-linux/report.json` | 50 host/original-function PASS lines; book/Options/return/Quit, inventory, separate-process save/reload and installed-GOG intro/skips/ship/Quit pass. |
| Separate diagnostic | `artifacts/20261112T152239732707Z-probe/report.json` | Rust probe reaches the original INT 21/AH30 boundary and exits with its expected status 2 under Wine. It is separate from the game. |
| Independent Glide backend and bridge | `.cache/glide/report.json` | Read-only-directory smoke and guest/native pointer, vertex, texture and framebuffer contracts pass. Its DLL hash matches the game build. |
| Actual seven-file Windows ZIP | `.cache/rust-port/release/package-verification.json` | Unchanged shipped CMD, physical D: install layout, uploaded alternate RGFX, book/Options/return/Quit, installed-GOG movie/skips/ship/Quit and independent PCM prefix pass. |

The Windows executable SHA-256 is `81adf7dfd4f12a71f6575c09461a31a7200d334c396f7eae1f2dfb8d8c9c67d5`. Linux is `9fea82be001cf8fdfa1c161cc443b37ca8e4651330d260e050105cc4fd0e8768`. The Linux build/test phase took 54.518 seconds, menu navigation 34.683 seconds and the complete command 246.074 seconds. Its movie probe exits 0 after 35.049 seconds. FFmpeg matches the first 32768 original decoded PCM bytes, SHA-256 `7245292991dd99810b6a91807988c48248533587a4e76a17573b602fabf336fa`.

Earlier final-source Linux attempts are retained as failures. Fixed three-second test delays sent input while the original book was still animating. The menu driver now waits for successive complete matching captures and requires distinct Options/return captures. Direct gameplay probes read the original book input gate and wait for it to remain ready before Continue or Quit. The same binary first passed those corrected tests in isolation, then passed the full command. The controls/save test still requires original movement, a changed autosave, exact position reload in a new process, clean Quit and an unchanged source save. Its failure collector preserves the original error even when a screenshot was never produced. These test changes do not alter original timing or gameplay.

After the game runs, the historical probe was isolated from the full game crate because its first link retained unused SDL and allocator imports. It compiles the unchanged shared crash-reporting module directly. Only `Cargo.lock`, `runtime/probe/Cargo.toml` and `runtime/probe/lib.rs` differ from the game reports' broad workspace manifests; no dependency versions changed. Rebuilding both game runtime and host archives for both targets produces byte-identical archives. `.cache/rust-port/archive-equivalence.json` records all four comparisons and current source hashes. The independent probe was then built and executed separately.

The actual release packager includes the locked Rust dependency notices, Rust standard-library notices and existing native dependency notices. The runtime ZIP SHA-256 is `92b7b35766819b6b8b6644d2a4d15e584ac905a022b45124ca7ced544481a5ed`. It contains only the executable, four required DLLs, `RUN.cmd` and `LICENSES.txt`. Renderer source/build evidence remains in a separate optional development ZIP. Its actual extracted-ZIP verification took 72.223 seconds and keeps `GAME_DIR=D:\Games\GOG Galaxy\Redguard\Redguard` byte-for-byte. No runtime media argument is supplied; it discovers installed GOG data automatically. The missing-game-path case saves exit 1. There are no game assets in the runtime ZIP.

Normal Windows/Linux exit now runs CRT cleanup callbacks. SDL input state intentionally lasts for the process so it cannot destroy video before audio cleanup or under the Windows loader lock. Native no-replace rename preserves the confined save directory without requiring hard links. Both hosts retain their independent ABI, shutdown and storage contracts.

These are bounded development checks. Windows testing here uses Wine, rendering uses software GL and audio uses SDL's dummy device. Real Windows acceptance, hardware playback, full-movie synchronization, sustained gameplay and completion remain unverified. The separate 15-second Windows startup observation still times out 124; it is not counted as a menu pass. The reported jump, combat, startup acceleration, normal-startup book behavior and distant fog issues remain gameplay work. The migration does not establish a fix for them.

The migration is prepared on `refactor/rust-runtime`. Local manifests and ignored artifacts preserve the evidence. Older idle generated Windows/Linux files were losslessly compressed; `.cache/rust-port/compressed-idle-artifacts.json` records their original hashes and sizes. Source and research history were preserved.

## Ship jump investigation after migration

The gameplay probe can observe up to five separate jumps with `--jump-cycle --jump-count N`. It records the original jump-key state as well as position, queued/falling bytes, fall origin, timer and timing globals. Each trial requires ascent, descent, a key released before and after the press, and a supported return to its starting height. Queued and falling phase observations remain diagnostic because 40 ms polling can miss a brief state. Five synthetic trace checks reject endless rise, frozen airborne state, a height crossing without support, held input, changed actor, an airborne starting state and an undelivered jump. These checks validate the test's acceptance rules; they do not establish that game physics is fixed.

Two new Linux ship runs use the unchanged accepted executable. Both fail because the 150 ms Space press is never observed in the original keyboard state and the actor stays at Y=-70144. The software-rendered ship runs at about 2–3 FPS; reducing Mesa worker threads does not resolve the missed input. Neither run reproduces endless rising or proves a successful jump. Evidence is retained in `.cache/gameplay-jump/linux-ship-first.{json,log}` and `.cache/gameplay-jump/linux-ship-two-threads.{json,log}`. Both game processes and their disposable container were stopped. The repeated-jump game gate and the user's real-Windows failure remain unverified.

Those retained reports predate the final trace-assessment refinement and therefore show `landed: true` despite no ascent. Their overall results are failures. Reassessing their unchanged samples with the final gate rejects landing too. `.cache/gameplay-jump/post-review-reassessment.json` records this without rerunning the game. The requested `LP_NUM_THREADS=2` environment for the second run is recorded in `.cache/gameplay-jump/invocations.json`, reconstructed from this continuation's tool invocation rather than captured from the game. A press and release handled in the same roughly 350 ms frame is a plausible explanation for the missed input; it remains a hypothesis.
