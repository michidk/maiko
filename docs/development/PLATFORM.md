# Windows and Linux host boundary

Maiko runs the same translated engine and Rust runtime on Windows and Linux. Game behavior belongs in the original translation metadata or shared Rust services. The ABI headers in `runtime/` define the contract used by the assembly and independent C test programs.

Rust std provides files, paths, threads, installation hashing and monotonic clocks. `cap-std` and `cap-fs-ext` confine save/config operations to an independently opened user directory. SDL Rust bindings provide one window, keyboard, mouse and audio implementation for both platforms. The Rust Glide bridge preserves native host pointers while translating the original game's 32-bit argument layouts.

Operating-system bindings remain necessary for below-4-GiB memory, memory protection queries, disk information, folder selection and fatal reporting. Windows receives an HWND from SDL; Linux passes SDL's window pointer to the patched OpenGLide backend. Windows uses SEH and Linux uses signals. These operations share Rust interfaces but have explicit platform implementations.

## Installed movies and storage

`runtime/rust/movies.rs` owns root-level DOS D: movie request validation and folder precedence. Loose SMK files are searched in the selected game directory, then `MOVIES`, `ANIMS` and `VIDEO`. Installed GOG data named `game.gog` is discovered in the selected game folder or its parent. An explicitly selected compatible image takes priority. The shared Rust media reader serves installed bundled data before loose-file lookup.

Media and game assets are read-only. Original save/config writes are available only with an isolated `--user-dir`. Capability-based file operations, path checks and retained storage contracts protect the original installation.

Discovery uses the canonical game directory. A junction or symlink therefore searches the target's parent. Invalid or ambiguous bundled movie data currently stops startup with a diagnostic. An absent bundle leaves loose-file lookup enabled.

## Verification and remaining limits

Both builds exercise shared services through the same C ABI fixtures. Original-function tests additionally call translated engine functions through the assembly entry interface. Real game probes cover menus, installed-GOG movie skips, inventory and save/reload. Exact results and platform limits are recorded in [VALIDATION.md](VALIDATION.md).

Independent C/C++ test programs remain to detect ABI layout and calling-convention mistakes. They are not runtime implementations and do not ship in the game executable. [RUST_PLAN.md](RUST_PLAN.md) describes the workspace and source policy.

The reported real-Windows jump, combat, startup speed and fog issues remain unresolved. A passing Linux or Wine check does not establish a real-Windows fix. Gameplay work resumes from [GAMEPLAY_PLAN.md](GAMEPLAY_PLAN.md) after the migration acceptance checks. Mac development is excluded.
