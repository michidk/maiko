# Full-game continuation

The user authorized continuation beyond the menu on 2026-11-11. Preserve original translated game logic, keep Cyrus separate, support both recognized RGFX images, and retain a Windows executable with an external game-directory argument and editable CMD. Linux is the fast development target; Windows remains a required delivery target.

## Current user priority

The Windows/Linux Rust migration is complete. Keep Mac development stopped.

Then reproduce a single-tap jump on the opening ship, trace original vertical movement, collision and landing, and verify rise, fall, landing and repeated jumps. Next trace attack input, weapon state, hits, damage and enemy behavior through defeating the ship enemies and reaching the island. Provide the requested normal book startup, New Game intro playback and the initial speed burst. Compare distant sea and fog with the original scene before choosing a translation or renderer fix. Expand island coverage through dialogue, doors, scene changes, inventory, saves and actual quest progression.

The checkpoints below describe earlier work. They do not supersede this order or establish that the reported gameplay defects are fixed.

## Acceptance and sequence

1. Reproduce Continue and New Game from the real menu. Record first unsupported service or translation fault with registers, original/host address and asset context. Add a reusable bounded gameplay runner with immutable evidence and hashes.
2. Load an original world, draw it and deliver original movement/camera/action input. Add targeted original-function or boundary regressions for every discovered ABI/translation defect, then rerun menu and gameplay scenarios. Loading assets alone does not prove gameplay.
3. Implement isolated writable save/config storage. Keep the install read-only by default. Validate file creation/write/seek/rename/delete, failure behavior, path traversal rejection and save/reload of actual original state. Reuse SR virtualfs where its contract fits; do not guess DOS replies.
4. Inventory original sound, speech/music and movie boundaries from execution and existing Cyrus magic/decomp evidence. Reuse appropriate open libraries behind explicit adapters. Validate actual playback, timing and skip/exit behavior. Keep disabled-audio scenarios distinct from enabled acceptance.
5. Exercise original scripts, dialogue, combat, inventory, world transitions and representative saves. Track what actually ran, repeated-session memory and unresolved services. No full-game claim without sustained gameplay and completion-path coverage.
6. Run Windows regression and extracted runtime ZIP tests, keep only required files, publish through the existing unlisted preview when a useful update exists, and obtain real Windows user evidence. Linux/Wine success does not set native_windows_verified.

## Implementation rules

Use the Python/SR translation pipeline, shared Rust runtime and assembly ABI interfaces. Build/run one at a time in bounded containers, use timeout with kill escalation, keep generated code/assets ignored, and preserve source saves. Every newly reached failure is investigated before changing behavior. Record phases and actual build/run durations in decisions.tsv and RESEARCH.md. This is the user-authorized experimental continuation; no repeated design approval or between-task permission prompts.

## Review focus

Guest/host pointer width and calling convention; address-taken code left as raw x86; x87 semantics and overflow warnings; unsorted/8.3 directory behavior; save writes escaping the overlay; optional sound disabled accidentally becoming the default; translated frames and native Windows exception reporting; responsiveness, focus and close/termination; unbounded allocations and frame logging.

Checkpoint, 2026-11-12 00:12 UTC: direct read-only CD image access now supplies original movie files without extraction. Original INTRO.SMK decoding, configured skips, opening-ship transition and save/Quit pass on Linux and the Windows target under Wine, including the extracted seven-file ZIP. Movie sound and CD music remain missing. Next implement the original Miles streaming contract at the verified callback boundary, then exercise dialogue, successful combat, map and world transitions. The earlier isolated Quit timeout is retained; a same-binary repeat and Windows save/reload pass do not establish its cause. Actual Windows acceptance and full-game completion remain false.

Checkpoint, 2026-11-12 00:43 UTC: original Smacker movie PCM now mixes on Linux through a real Miles double-buffer adapter. Original decoded track0, all skip points, ship entry and Quit0 pass. New adapter tests cover exact PCM buffer order, source ownership, consumed positions, EOF, release/reuse and stereo gain. The first audio-enabled integration exposed a previously dormant DPMI memory-pin dispatcher; native wrappers now return real host lock/unlock results. Windows audio integration is running. Next cover Windows streamed output and original gameplay/save regression, then real dialogue/combat/transitions and CD music. Dummy-device nonzero PCM is distinct from listening and synchronization acceptance.

Review continuation, 2026-11-12 01:24 UTC: fixed negotiated movie output rate, removed allocation/conversion from the audio callback mutex, and strengthened callback translation guards across all return paths. Independent FFmpeg comparison matches the first32768 original decoded PCM bytes. The Linux skip sequence consumes522240 frames with zero measured starvation; lower nonzero counts include source silence. Hardware/listening, full-movie synchronization and other movie coverage remain open. Windows regression and actual extracted-ZIP/reference comparison precede replacement of the download. CD music remains the next implementation boundary, using the existing cue/muting and Miles wrapper research; no CD music implementation is claimed.

## Latest Linux gameplay checkpoint

The diagnostic-budget fix allows three separate ship jumps to rise, descend and land in the Linux check. Keyboard attacks defeat both original pirates and the original scripts load the island. The native startup adaptation now opens a visible book without an autosave, using the original menu entry-disable path for Continue and the original New Game path for the intro. These are bounded Linux checks; the reported hardware-Windows jump remains unresolved. See [VALIDATION.md](VALIDATION.md) for exact builds and acceptance limits.

The shared Rust clock now paces original fixed 12-Hz warm-up steps. Complete ship and island countdowns, three repeated ship jumps, intro/audio/skips and the ship encounter pass on the same Linux build. Next verify the updated build on Windows, compare the distant sea/fog scene against the original, and follow island progression. Arrival on the island does not establish conversations, doors, quest completion or sustained gameplay.
