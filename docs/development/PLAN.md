# Native Redguard menu implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan sequentially. Existing experimental-spike authorization applies; no additional design approval is required.

**Goal:** Run the original interactive book menu in a Windows executable without a DOS emulator or launcher.

**Architecture:** Use pinned M-HT/SR to translate the original engine and replace required platform boundaries with a small native runtime. Reuse a Glide implementation through an explicit adapter between original 32-bit layouts and native calls. Preserve the diagnostic as a separate target.

**Tech stack:** Existing Python pipeline, SR C translator, generated x64 assembly, MinGW C/assembly runtime, a separately evaluated Glide library. Cyrus remains separate; do not rewrite SR or the graphics wrapper in Rust for this spike.

**Spec:** User-authorized spike and README.md acceptance criteria; RESEARCH.md records current evidence and limitations.

## Global constraints

- Keep work under experiments/recomp, outside published documentation and Cyrus source.
- Keep the feature branch; do not publish generated game code, binaries or game assets in Git.
- Default to the native executable's directory for game assets; optionally accept --game-dir. Never overwrite original executables or saves.
- Keep strict, diagnostic and native-menu reports distinct. Fatal diagnostic substitutions cannot silently become native support.
- Continue the existing bounded resource policy: one build at a time, container memory/CPU limits, timeouts and ignored generated output.
- Log actual start/end times by phase, build durations, failed approaches and unresolved boundaries. Retrospective estimates must be labelled.
- Unsupported services must name the original address and operation and stop. Implement intentional behavior, not blanket success responses.
- Wine checks are development evidence; native Windows acceptance is a separate field.

## Review focus

- Wrong game build or missing assets: explain the mismatch before executing original code (tasks 1 and 3).
- Incorrect calling convention, register preservation or pointer width: synthetic boundary tests before game integration (tasks 2 and 4).
- DOS case-insensitive paths and Windows launch working directory: asset lookup must be independent of the invoking directory (task 3).
- No valid autosave: original startup may select a movie/world instead of the menu; prepare an isolated documented menu scenario (task 5).
- Framebuffer pointers, focus loss and process exit: no truncated pointers, stuck input or original-save mutation (tasks 4–6).

## Task 1: Establish the exact baseline and first dependency inventory

Files: update RESEARCH.md and decisions.tsv; add metadata/runtime_boundaries.json and artifacts/baseline/ (ignored).

- [ ] Record original image hashes, GOG launch configuration and hashes/versions of each bundled Glide DLL. The inspected installation contains nGlide v1.00 in Redguard/, nGlide v1.03 in __support/dosbox/, and OpenGLide in DOSBOX/. GOG's primary play task starts DOSBOX/dosbox.exe directly; its autoexec starts RGFX.EXE.
- [ ] Observe the actually loaded graphics module during a reference run. File presence or working directory alone is not conclusive evidence of DLL selection. Reference emulation is permitted for comparison, not as the native solution.
- [ ] Capture the original book menu and its entry conditions in an isolated writable scenario. Determine necessary configuration, assets, CD/image dependencies and autosave state without modifying the source installation.
- [ ] Inspect the CRT initialization dispatcher and the candidate main edge 0x113B1A -> 0x20000. Inventory original addresses, arguments, return values, register preservation and global initialization for boundaries reached before asset loading.
- [ ] Validate candidate entry and call sites against original bytes. Do not treat generated labels alone as proof of their semantic identity.

Gate: reproducible reference scenario and evidence-backed initial runtime map. No native main jump before its preconditions are established.

## Task 2: Introduce a valid native entry adapter

Files: add runtime/native_entry.asm, runtime/native_main.c, runtime/guest_abi.h, tests/abi_harness.asm and tests/abi_harness.c; modify tools/pipeline.py and tools/build_windows.py for an explicitly named native target.

Interfaces: guest pointers are uint32_t; native pointers are never silently truncated. Define guest argument storage and guest-stack allocation in guest_abi.h after confirming the original convention. Native entry accepts argc/argv through the verified original registers, not the Windows calling convention.

- [ ] Test synthetic calls through the adapter for register preservation, argument passing, return values, stack balance, floating-point state and rejection of pointers beyond the guest address range.
- [ ] Allocate a bounded guest stack and argument storage in the addressable range. Check every allocation and conversion.
- [ ] Supply verified CRT globals/initializers or replace their library boundaries. Preserve required constructors; do not skip the initializer table wholesale.
- [ ] Replace the debug text-display boundary that touches 0xB0000 with deliberate native logging; map all related accesses before removing the original initialization.
- [ ] Execute original initialization with fail-fast tracing at the next unresolved boundary. Preserve the initial DOS-entry diagnostic separately.
- [ ] Audit translator additions actually exercised on this path. Check flags, x87/MMX state and sign-extended immediates; eliminate or justify relevant assembler warnings rather than suppressing them globally.

Gate: execution advances into original game initialization with verified ABI preconditions, without guessed DOS replies or invalid hardware-memory accesses.

## Task 3: Read original assets through native runtime services

Files: add runtime/game_files.c, runtime/game_files.h, runtime/guest_heap.c; extend metadata/runtime_boundaries.json and metadata/external_procedures.sci; add tests/runtime_io.c.

Interfaces: game_files_init(const wchar_t *root) selects the root; original file/heap signatures are defined individually from task 1 evidence and adapted at the assembly boundary. Use explicit handle mapping where original handles are integers.

- [ ] Resolve the default game root from the executable path, with --game-dir override; verify launch from another working directory and a path containing spaces.
- [ ] Implement only the identified file and heap operations required by startup, preserving error, EOF, seek and ownership behavior.
- [ ] Test missing files, case variations, binary byte identity, seek/EOF, allocation alignment and allocation failure. Reject unsupported build hashes before running game code.
- [ ] Map writes to the isolated test scenario; preserve the user's original saves and configuration.
- [ ] Add timers or other services only as the real execution path requires them, with original units and behavior established first.

Gate: original engine successfully opens and parses its startup configuration/assets; logs distinguish actual parsing from merely opening filenames.

## Task 4: Prove and connect a reusable Glide backend

Files: add glide/README.md, glide/backend_smoke.c, glide/guest_glide.h and glide/bridge.c; extend tools/build_windows.py and metadata mappings.

- [ ] Evaluate OpenGLide as a source-build candidate, including relevant SDL2/64-bit forks. Record pinned revision, license and build changes. Treat nGlide/dgVoodoo as possible Windows comparison backends only after checking binary architecture and exposed API; do not assume a bundled 32-bit DLL loads into the x64 host.
- [ ] Build a standalone native smoke test that creates a window, draws a textured triangle, swaps buffers, locks/unlocks the framebuffer and exits. Verify pixels or captures as well as return codes.
- [ ] Compare the full game import-name inventory with the selected backend. Existing OpenGLide comparison matches 124 of 130 names; determine whether the six missing names are mandatory at load time or used at runtime. No successful empty stubs.
- [ ] Define fixed-width guest layouts and explicit conversion for texture descriptors, framebuffer descriptors and returned pointers. Use DOSBox-X forwarding code as a reference with its license accounted for.
- [ ] Test layout sizes/offsets, texture transfer, returned framebuffer addressability, float argument passing and stack cleanup independently of Redguard.
- [ ] Replace the original DOS overlay-loading/dispatch boundary with native function mappings, retaining original rendering calls and recording the actually exercised API subset.

Gate: the original engine issues real Glide calls through the adapter and produces a correct startup frame. A standalone triangle does not satisfy the game milestone.

## Task 5: Reach the original interactive book menu

Files: add runtime/input.c and runtime/timing.c as required; extend native entry and scenario setup; update RESEARCH.md.

- [ ] Use the baseline scenario from task 1 to follow the original menu-selection path. If a direct menu entry is needed for diagnosis, record its verified initialization contract and keep it separate from normal startup acceptance.
- [ ] Trace and implement remaining services actually reached. If audio/video blocks menu startup, establish a legitimate optional/disabled path or implement the required service; do not pretend playback succeeded.
- [ ] Connect keyboard and mouse events to the original input representation. Handle focus loss and release captured input on exit.
- [ ] Compare the original rendered menu with the reference at the original resolution. Exercise page navigation, selection and closing through original code.
- [ ] Verify the menu remains responsive across focus changes and repeated navigation; capture blockers and crash addresses.

Gate: original menu renders and responds in a native Windows executable under Wine without DOSBox. Report remaining audio, video and window behavior limitations explicitly.

## Task 6: Package and accept on Windows

Files: add package_menu.py; reuse the existing managed preview and retain separate diagnostic packaging.

- [ ] Package the native executable, permitted runtime dependencies, notices, provenance and a short RUN.cmd that retains logs and exit code.
- [ ] Test the actual extracted ZIP from an isolated game scenario, including missing-assets failure and clean exit. Confirm no dependency on build-container paths.
- [ ] Serve the personal test through an unlisted preview path, verify downloaded bytes, and provide instructions to place files alongside the original assets or use --game-dir.
- [ ] Obtain Windows evidence of menu rendering, navigation, focus recovery and exit. Mark native_windows_verified only on actual Windows evidence.
- [ ] Review the complete diff and decision trail; preserve source and research on the feature branch. Publish only through managed signed commits if publishing is requested; no automatic PR.

Gate: original interactive book menu verified on Windows. Full-game playability remains a separate milestone.

## Effort checkpoints and continuation

Use a first four-hour active-work checkpoint for tasks 1–3: report whether initialization and asset loading are advancing, actual time by task and unresolved service count. Use a separate two-hour graphics feasibility checkpoint for task 4's backend build and smoke test. These are review budgets, not delivery estimates, and do not revoke existing authorization or require another design approval. When a budget is exceeded, document the cause and narrow the next experiment before spending further time.

After the menu, plan world loading/movement, combat/scripts, save/load, audio/video and long-session regression as a separate increment. macOS and broader architecture support follow a demonstrated Windows runtime; do not claim portability from SDL or wrapper availability alone.

## Linux feasibility follow-up

User asked whether native Linux would make development easier. Keep Windows delivery active. First reuse the exact SR output/metadata for a minimal ELF64 ABI test, with a System V entry and host-service adapter, non-PIE low image and bounded low32 allocation. That test must retain guest registers, stack and x87 state. Then port file/path services with explicit DOS case lookup, timer and crash reporting, followed by the existing OpenGLide Linux/SDL path and shared input representation. Test the same original functions and game scenario on both hosts.

Use an initial4–8hour feasibility checkpoint, not a delivery estimate. A useful outcome is the same real startup checkpoint on Linux, with recorded effort and graphics blockers. A Linux run does not prove Windows pointer widths, exception handling or driver compatibility. Avoid a sweeping portability refactor before the minimal ELF adapter demonstrates reuse. No Linux implementation has begun.


Linux follow-up checkpoint, 20:09 UTC: ELF entry/callback and original functions pass. The original book, Options, Escape and clean Quit run without Wine using SDL2, with both inspected images exercised. SDL input and timers now live in shared runtime modules. Windows retains the existing Win32 implementation while its shared startup/assembly regression is checked. Next portability step is an explicitly tested Windows SDL build; next game step is original world loading. Neither is implied complete by Linux menu acceptance. Keep DOS file/search/path behavior, low32 allocation and ABI glue behind narrow interfaces; evaluate SR's virtualfs implementation before expanding custom filesystem coverage.

Linux review checkpoint, 20:25 UTC: confirmed long-name enumeration and missing keypad/extended-key defects are fixed and pass the full native suite. High-pointer SDL rendering passes with a 64-bit Glide argument; the alleged truncation is not present on this build. The original menu sequence passes again with byte-identical reviewed frames and clean Quit. Shared Windows startup regression and extracted ZIP interaction passed under Wine before these Linux fixes; the later bridge width assertions compile for Windows. Keep the existing preview package unchanged. Linux feasibility is complete for this narrow scenario. Windows SDL adoption, actual Windows acceptance and original world loading remain the next separate gates.
