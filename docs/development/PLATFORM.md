# Windows and Linux host boundary

Maiko runs the same translated engine, native service dispatcher, guest-register contract, pooled heap, Glide bridge and SDL audio on Windows and Linux. Game behavior belongs in this shared code or the original translation metadata. Platform adapters implement operating-system operations behind the headers in `runtime/`.

The boundary is sufficient for continued gameplay work, but equal interfaces do not yet mean equal implementations. Timing is now shared Rust code using std::time::Instant. Windows input still uses Win32; Linux input uses SDL. File handles, path casing, save storage and crash reporting also have separate host implementations. The host tests must exercise the same observable contracts before either implementation changes.

## Installed movie files

`runtime/rust/movies.rs` owns the common request validation and folder precedence. A root-level DOS `D:` request for an SMK file searches the selected game directory, then `MOVIES`, `ANIMS` and `VIDEO`. The operating-system callback resolves the candidate and checks that it is a file. On Windows, this alias applies to requests outside the configured game root; paths already inside that root retain normal file resolution. On Linux, eligible DOS D: requests use the alias. The file adapters retain their read-only rules for game assets.

Both host builds compile `tests/installed_movies.c` against their actual file and storage adapters. It checks read/attributes, case handling, folder precedence, missing movies, immutable files and path boundaries without game data or an image. `sh tests/check_portable.sh` runs the Linux version. The Windows native-build checks run the same fixture under Wine. Neither establishes playback of movies in a particular installation. Mounted-image precedence and a Windows game root on drive D: are not yet covered together with this fixture. Linux case collisions and Windows path-length errors can also produce different errors between the adapters.

The shared Rust installer also finds bundled GOG movie data named `game.gog` in the selected game folder or its parent. Discovery runs after selecting the native game working directory, so it preserves the host path encoding. This installed source and an explicitly selected compatible image are handled before loose-file lookup. Removing that source is separate from locating installed movies, decoding SMK content or keeping the original animation clock.

## Next shared work

1. Bring Windows onto the shared SDL input path with explicit window-handle integration. Verify focus loss, release events, repeat, mouse movement, startup timing and clean exit before replacing the current backend.
2. Use the same jump/timing observation scenario on both targets. The reported real-Windows endless rise is still unresolved; Linux or Wine landing is not a Windows fix.
3. Extend shared file/save contracts as original gameplay reaches new operations. Keep native handles, memory mapping and exception or signal handling in host-specific adapters.

The Rust runtime already implements the clock and movie-policy C ABI. The remaining migration is tracked in [RUST_PLAN.md](RUST_PLAN.md). It must preserve these contracts, the low-address memory requirement and platform acceptance checks.


Installed media discovery uses the canonical game directory. If the selected game folder is a junction or symlink, its parent means the target's parent. A discovered but invalid or ambiguous bundled movie-data file stops startup with a diagnostic; an absent bundle leaves loose-file lookup enabled. This strict failure is intentional in the current experiment.
