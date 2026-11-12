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
