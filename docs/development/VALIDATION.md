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

This change does not establish movie playback from the user's installation or fix the reported real-Windows jump, startup speed, combat or fog issues. Windows input and timers still use Win32; Linux uses SDL. The [platform boundary](PLATFORM.md) records the next consolidation work.
