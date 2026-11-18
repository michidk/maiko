# macOS runtime port

User authorization: bring macOS to the existing experimental game feature set. Existing Windows physics/rendering defects remain open. No further design approval required.

## Delivery requirements

User instruction, 2026-11-12: future preview download pages must also offer a separate Mac ZIP once a Mac game build is available. That ZIP must include the original game files, rather than requiring a Windows GOG installation or a separate manual asset copy. This instruction authorizes bundling the owned game copy for the user's preview test download. Game data stays out of Git and source archives.

Package the Mac executable, required runtime libraries/frameworks, notices, a launchable `RUN.command` and an adjacent `game/` directory containing the required original game files, including the recognized RGFX.EXE used for version validation. Resolve the default game path relative to the launcher so extraction location and spaces in paths work. Support Intel and Apple Silicon through native builds, with the launcher selecting the matching executable if the ZIP contains both architectures. Keep runtime packaging separate from SDKs, compiler output and research evidence. The Windows CMD retains the exact user-requested Windows GAME_DIR preset.

Before publishing the Mac link, verify the extracted ZIP's layout, launcher, architecture selection and bundled-asset manifest. Run the game from that extracted package on a Mac before claiming Mac runtime acceptance. State any remaining verification limits on the preview page. Until an actual Mac game binary exists, keep this as a delivery requirement rather than linking diagnostic executables as a game download.

Reuse the game/runtime service contracts, SDL input/timers/audio and POSIX files/saves. The first direct NASM Mach-O attempt failed on 32-bit absolute relocations in the actual game. Upstream SR uses llasm/LLVM for both Intel and Apple Silicon Macs. Continue with that backend for both targets. An Intel binary through Rosetta remains a possible test option, but is not the native ARM target.

1. Validate toolchain and object format with actual translated assembly and an ABI fixture. Never change the existing Windows/ELF output for Mach-O quirks.
2. Build Darwin-specific host query/crash adapters, share portable file and save services and SDL. Exercise portable behavior on Linux and compile Darwin code with an SDK. No success stubs for memory queries.
3. Cross-build original game, renderer and runnable boundary test executables. Inspect all Mach-O segments, symbols, relocations, dylib dependencies and low-address layout. Package runtime dependencies and a path-selectable RUN.command. Keep game assets out of source control; the requested personal runtime ZIP includes the owned game files.
4. Run the boundary/game gates on macOS if a machine is available. Otherwise retain macOS runtime verification false and hand over exact executable/tests, with the missing runtime gate explicit. Do not claim parity from cross-compilation.
5. Review source/evidence across models once; preserve decision log and resource limits. No Git publishing or PR creation.

Verification goals: ABI registers/x87/callback/stack, low-address heap, POSIX case/path/save behavior, renderer and input, original menu, ship jump, movies when installed, save/reload and clean exit. Hardware/macOS acceptance is distinct from Linux tests and static Mach-O inspection.

## Checkpoint, 2026-11-12 09:26 UTC

`python3 check_macos_backend.py` recreates the separate bounded Docker toolchain, checks SDK/SDL download hashes, archives pinned SR/OpenGLide sources, applies recorded patches and emits diagnostic Mach-O executables for x86-64 and ARM64. No game assets are inputs. It removes its container by default. `--keep-container` retains a bounded container for at most 30 minutes for follow-up compilation.

Latest clean result: `artifacts/20261112T092457027776Z-macos-backend/report.json`. The generated LLVM fixture runs on Linux and checks BSWAP, all combinations of the six arithmetic condition flags, offset memory access, x87 arithmetic, shared-flag branches and balanced guest returns. Both Mac architectures link that fixture, Darwin memory-query diagnostics, the Glide renderer and a textured-triangle test. None ran on macOS. The Darwin query diagnostic does not implement or prove a guest allocator.

The full Redguard LLVM attempt remains incomplete. The NASM native replacements must be rewritten for LLasm with explicit flag contracts. Without those replacements, verified annotations move translation past two shared-ZF regions to the next failure at 0xCA2B8. `artifacts/macos-llasm-flags/` is an investigation, not runnable game output. The existing NASM host-register ABI and direct pointer casts must be adapted to SR's LLVM CPU state. ARM additionally needs a coherent pointer-offset arena for code, guest heap and callback state. No Mac game, package, native runtime, menu, ship or parity claim is accepted yet.

## Review follow-up, 2026-11-12 09:52 UTC

Sonnet 5.5 confirmed the checkpoint's claim scope, upstream architecture targets, artifact hashes and the first flag contracts. Its source-provenance and coverage findings triggered the following fixes. No further delegate was used for this fix pass.

Latest clean backend check: `artifacts/20261112T094057335470Z-macos-backend/report.json`, 52.566 seconds with a warm toolchain. The report identifies 134 source inputs, 46 output files, upstream archive hashes and the actual LLVM driver. All source and output hashes were reread. The source-manifest gate rejects changed or added source files. Source remains uncommitted.

The check now uses upstream's inline-division LLasm flags and opt/llc path. Signed/unsigned division and shared signed-comparison flags, including overflow boundaries, execute on native x86-64 Linux and emulated ARM64 Linux. QEMU does not establish Darwin ABI or Mac hardware correctness. Both architectures cross-link Mac diagnostics and compile the shared POSIX host, paths, save storage, file services and SDL input/timers. The initial POSIX compile caught a missing framework search path, now corrected. Memory-query diagnostics also contain code/stack/library readability cases, but no Darwin execution or game allocator proof is claimed.

Original-game flag metadata is preserved under `llasm-metadata/`. `translate_llasm.py` rebuilds its inputs from the owned, hash-checked image and records metadata and translator hashes. It moves past the earlier signed-comparison and CMPSB carry boundaries and the inspected texture-size dispatch branches. The next failure is incoming carry/zero at 0xE6703. The analysis target still omits most native substitutions and does not compile/link a game. Its overlap expansions lack independent execution tests. A caller-supplied translator is identified by hash, with source verification false; a reference commit is not presented as verified executable provenance.

## Opcode checkpoint, 2026-11-12 10:08 UTC

The clean backend check `artifacts/20261112T100249885086Z-macos-backend/report.json` now includes reversed x87 arithmetic destinations and guest rounding-mode support. Native Linux x64 and QEMU ARM64 execution pass; both Darwin architectures cross-link. No Mac execution. Original-game analysis reaches the cross-call carry dependency at ED96B in `artifacts/20261112T100559022773Z-llasm-analysis/report.json`. Translation and the game host ABI remain incomplete. These opcode additions postdate the independent checkpoint review.

## Publication checkpoint, 2026-11-12

Latest clean backend check in the original workspace: artifacts/20261112T104343553824Z-macos-backend/report.json, 57.966 seconds with warm tools. All 46 artifact hashes were reread. Native Linux x64 and emulated ARM64 fixtures pass extended loads/stores, remainder quotient bits, atan quadrants, FTST, counter/overflow branches, reversed x87 arithmetic and guest rounding modes. Both Darwin architectures cross-link. The generated-code x87 stack uses upstream double precision and does not model all exceptions.

The original-game analysis progressed through inspected carry/zero contracts in CRT formatting, pow exponent loops, byte classification and memory copy/fill. Latest report artifacts/20261112T105137014566Z-llasm-analysis/report.json stops at 0x12E204, incoming CMPSB carry at SBB. Four weak-flag warnings remain at FA134, FA1F3, FA6B6 and117072. No unknown-flag error bypass was added. The many byte-identical string-pattern annotations were generated with one-off commands; a reusable generator and broader independent execution coverage are still needed.

No original-game LLasm output has been emitted. No Mac game is built, linked, run or packaged. The CPU-state services, offset arena and most native replacements remain pending. Runtime assets requested for the future personal Mac ZIP stay outside Git.

Effort reconstruction: approximately10:14-10:38UTC for the missing opcodes and repeated red/green cycles;10:40-10:52UTC for further original-game flag contracts and the clean57.966-second backend check. The08:42-10:52 Mac-work span is about2h10 elapsed including checks, context recovery and interruptions, not a precise active-hours count. Publication work follows separately. These are reconstructed intervals; synthetic build times are not an estimate for finishing the game.
