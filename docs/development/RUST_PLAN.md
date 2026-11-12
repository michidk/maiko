# Rust runtime migration

The user requests Rust for Maiko-owned runtime code and shared Windows/Linux implementations. This continues the authorized recompilation experiment. SR translation and generated assembly remain intact; upstream SDL, OpenGLide and SR sources remain external dependencies. Migrating those upstream projects is a different task.

Use one Rust runtime crate with explicit C ABI exports for the translated assembly and temporarily retained adapters. Keep guest addresses as checked 32-bit values. Rust standard-library files, paths, threads and monotonic time replace duplicate host logic. SDL Rust bindings will own the shared input, window and audio integration. Restrict OS-specific code to guest mappings, memory queries and crash dispatch. Avoid a second window/event framework beside SDL/OpenGLide.

## Current checkpoint

The Rust crate currently owns movie request policy, installed GOG movie discovery and BIOS/elapsed timing. The three replaced C files have been removed. Remaining C/C++ adapters and tests are still present; this is an incremental migration, not a completed Rust port. Rust unit and native Linux ABI contracts pass. Windows std requires Wine10 in the development toolchain because Wine8 lacks bcryptprimitives.dll. Full-host results are recorded separately in the decision trail.

## Deliverables and gates

1. Replace movie request policy and both clock implementations with Rust. Link the Rust library into both real hosts and run the existing file-service, timer and original movie-clock contracts. Remove replaced C sources after callers migrate.
2. Remove CD_IMAGE from the normal CMD. Keep explicit --cd-image as an optional media source; packaging must accept installed media evidence and must not require image-backed movie evidence for a normal no-media test. Test rejection of falsely claimed movie support.
3. Separate the early diagnostic entry program from the actual runtime. Retain shared fatal hardware stubs behind a neutral filename until dependency checks prove they are unused.
4. Port file/save/config/install selection to Rust std with shared path policy. Preserve isolated writes, original filenames, DOS errors and user files; run both OS contracts and save/reload.
5. Port input and audio to SDL Rust bindings, maintaining the existing renderer/window connection. Run scancode/focus/mouse/close and PCM tests on both targets.
6. Port host dispatch/startup, memory wrapper, platform services and crash handling. Keep reviewed assembly and external allocator/audio dependencies until equivalent Rust replacements pass lifetime and original-function tests.
7. Port remaining owned C/C++ bridges, diagnostic program and tests; remove obsolete sources/build rules. Audit remaining C inventory and clearly distinguish upstream dependencies from owned code.

Each completed unit must keep Linux and Windows builds usable. Full migration remains incomplete until the owned-code inventory is clear and both original-game regressions pass. A Rust library build or Wine run alone does not establish real-Windows acceptance or fix jump/startup/fog/combat issues. Future work remains on feature branches; current main publication was explicitly requested by the user.
