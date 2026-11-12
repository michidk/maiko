# Native byte-level replacements.
#
# Each entry replaces `length` bytes of the original image at the address in its label with
# the instruction sequence after the second comma ('|' separates instructions). The comment
# block above an entry explains what the original does, what replaces it, why and where the
# evidence is. `# sha256:` is the SHA-256 of the original bytes; the build refuses to run if
# the image at that address differs. Comment lines and blank lines are removed before the
# file is given to the translator (tools/pipeline.py, read_replacements).
#
# Keep this documentation current: a new entry needs Original, Replacement, Effect, Why and
# Evidence lines and a sha256 line, or the build stops. Entries that share one comment block
# are separated from the next block by a blank line.

# == Keyboard LED state: scroll lock and num lock
# Original: clears bit 4 (scroll lock) or bit 5 (num lock) in the BIOS keyboard flags byte at 0x417, raises INT 9 so the BIOS refreshes the LEDs, then calls the helper at 0xD7AD4 with 100.
# Replacement: calls host service 9 or 10, which clears the same bit in the host's own lock state.
# Effect: the game keeps its original lock-state logic; a native process cannot write BIOS memory or raise INT 9.
# Why: the DOS BIOS data area and software interrupts do not exist natively.
# Evidence: runtime/host/lib.rs services 9 and 10; docs/research/RESEARCH.md keyboard section.
# sha256: 6b503dd400ddcfbea3b5262026602b372f091e2252d7dcedcbf295435982bb5b
loc_20DCD,20,CALL host_clear_scroll_lock
# sha256: 6c649aedb7444e65b41d586347d927c05fc616e66ceb9b348df75d59077e2708
loc_20DE1,20,CALL host_clear_num_lock

# == Keyboard handler install and lock-state save
# Original: loc_D72E5 hooks the DOS keyboard interrupt through DPMI (get/set vector, saved in the original's own words) and records extended-keyboard support. loc_D734F reads the BIOS lock flags at 0x417 into the original's caps/scroll/num variables.
# Replacement: host services 16 and 17. Service 16 allocates the keyboard stack, sets the extended-handler flag and starts native event delivery; the original key translation tables and handler logic are retained. Service 17 zeroes the three lock variables.
# Effect: key events reach the original translation code without a DOS interrupt vector.
# Why: DPMI vector hooks and BIOS memory are unavailable natively.
# Evidence: runtime/host/lib.rs services 16 and 17 ('original key translation retained').
# sha256: 23a74eed5606a4f0e78c1e5c6ab1c0ed392b2a34838003b42d95e0d0f0144700
loc_D72E5,79,CALL host_keyboard_install
# sha256: 7da01710ea56ac54cc3192e10ececb749f783885520e43d70d655b134fab879d
loc_D734F,94,CALL host_keyboard_locks

# == Interrupt-handler tail: EOI and IRET
# Original: end of a hardware interrupt handler: sends end-of-interrupt (OUT 0x20,0x20), restores GS/FS/ES/DS and the general registers (POPA) and executes IRET.
# Replacement: RET.
# Effect: the handler is entered as an ordinary function call by the host and returns like one; the registers it would restore were not saved by an interrupt entry.
# Why: the host calls the handler directly. There is no PIC and no interrupt frame, so OUT and IRET would fault or corrupt the stack.
# Evidence: original bytes at these two sites are identical 16-byte tails (see sha256).
# sha256: 0264c2fa6cb654ab416fc87960181d5d7ab098d8c50306a170c1c19d1163025c
loc_D7524,16,RET
# sha256: 0264c2fa6cb654ab416fc87960181d5d7ab098d8c50306a170c1c19d1163025c
loc_D7589,16,RET

# == VGA mode 13h request
# Original: PUSHA; INT 10h with AX=0x13 (320x200 VGA mode); POPA.
# Replacement: host service 20, which creates the native 640x480 window that the original Glide initialization then draws into.
# Effect: the legacy VGA mode switch becomes window preparation; Glide setup is untouched.
# Why: no BIOS video service natively; the Glide build never displays the VGA mode.
# Evidence: runtime/host/lib.rs service 20.
# sha256: 93f493d541780ad24ccd3a0fbca1252a1d912cec52a1c995dfa2b31f723fe641
loc_B9D2F,9,CALL host_prepare_graphics

# == Null numeric input in the original string-to-number conversion
# Original: MOV DL,[EAX] reads the first character of the input string.
# Replacement: returns zero from the conversion when EAX is null (TEST EAX,EAX; JZ to the function's zero-result path), otherwise executes the original MOV.
# Effect: null input converts to 0. Every non-null conversion path is unchanged.
# Why: DOS4GW lets a null read return low memory; a native process faults. The game reaches this with a null configuration string while parsing the attack/defence/taunt tables.
# Evidence: docs/research/RESEARCH.md ('native-only byte-checked guard at original 0xEB55D'); docs/research/decisions.tsv 2026-11-11T16:15:37Z.
# sha256: f07aeb88b7e724ef0713d76dbd681acc2c11e4b1fe9d7500770ded65b29bd9a5
loc_EB55D,2,test eax,eax|jz loc_EB5C3|mov dl,[eax]

# == Image variants: two RGFX.EXE builds
# Original: the executable at the GOG install (variant 0) has JZ 0x5544F at 0x55441 and MOV ECX,2 at 0xAD96B. The owner's RGFX.EXE (variant 1) differs by exactly three bytes: JZ becomes an unconditional JMP to 0x5544F, and the first two bytes of MOV ECX,2 become a JMP to 0xAD9E0.
# Replacement: RG_VARIANT_JUMP takes the variant-1 jump only when the executable hash selected variant 1 at startup; otherwise the original instruction runs.
# Effect: both known executables behave as they do natively in their own builds.
# Why: both must run. Which one is the unmodified release is not established.
# Evidence: metadata/image_variants.json; docs/research/RESEARCH.md ('exactly three changed bytes').
# sha256: 57a97cadea0452df11971965cf92e9e279b06b4b5b2806b94dc1174a4fa06871
loc_55441,2,RG_VARIANT_JUMP loc_5544F|jz loc_5544F
# sha256: 86c50ee89b1749fd3d78aafa56fe1abe55d39899809d5f2cf0908a56fc213149
loc_AD96B,5,RG_VARIANT_JUMP loc_AD9E0|mov ecx,2

# == Null pointer in directory-scan cleanup
# Original: CMP ECX,[EDX+4] with EDX = ECX = 0 after a directory scan that found nothing.
# Replacement: when EDX is null, jump to the function's normal exit (loc_D5726); otherwise the original CMP.
# Effect: an empty scan cleans up without reading memory at address 4.
# Why: DOS can read address 4 harmlessly; a native process faults.
# Evidence: docs/research/decisions.tsv 2026-11-11T17:34:55Z; docs/research/RESEARCH.md ('cleanup null dereference at original 0xD5963').
# sha256: a2e0ef9208b2e1d50c6859cecbc4b780c6cbb4f1edb1d9347d81abbcdae81f4c
loc_D5963,3,test edx,edx|jz loc_D5726|cmp ecx,[edx+4]

# == Font style 10 metric from a null source
# Original: MOV DX,[EAX+0x46] reads a font metric word.
# Replacement: RG_FONT10_METRIC (runtime/font_metric.inc): the same read for a non-null source; 0xF000 for null. Flags are saved and restored and the upper half of EDX is preserved.
# Effect: reproduces what the DOS reference showed for this one site.
# Why: the DOS reference read 0xF000 from the interrupt-table high word at DS:null+0x46; native Windows faults.
# Evidence: docs/research/decisions.tsv 2026-11-11T17:56:23Z (reference observation); docs/research/RESEARCH.md. This models the observed DOS reference, not every BIOS or every null read.
# sha256: 5007d7e7615549b9d4dd169698d7401f35a90756ce557248e6e40ae7b61cd559
loc_BDCAD,4,RG_FONT10_METRIC

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/development/PLATFORM.md.
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_ABC0A,8,CALL host_poll_input|cmp word [loc_2055C2],0
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_AC600,8,CALL host_poll_input|cmp word [loc_2055C2],0
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_AC95E,8,CALL host_poll_input|cmp word [loc_2055C2],0
# sha256: 1c0b1aa327c4487ece634eab6a0c9decffcf637888e08dc99a884ed0a7cda76a
loc_B22D2,8,CALL host_poll_input|cmp word [eax+loc_2055C0],0
# sha256: 1c0b1aa327c4487ece634eab6a0c9decffcf637888e08dc99a884ed0a7cda76a
loc_B25CA,8,CALL host_poll_input|cmp word [eax+loc_2055C0],0
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_B25D4,8,CALL host_poll_input|cmp word [loc_2055C2],0
# sha256: f11c186b89af21d5eb5bdba4fd3a66c29a9c410285c9f553563a6ede2bed0241
loc_B25A6,8,CALL host_poll_input|cmp word [loc_205612],0
# sha256: 59003b473ded936542904e1f9cc1b4b2c9e11c5cc326f32098840bae1d83b7dc
loc_B263A,8,CALL host_poll_input|cmp word [loc_2055F8],0
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_C7C88,8,CALL host_poll_input|cmp word [loc_2055C2],0
# sha256: 1c0b1aa327c4487ece634eab6a0c9decffcf637888e08dc99a884ed0a7cda76a
loc_C7C9B,8,CALL host_poll_input|cmp word [eax+loc_2055C0],0
# sha256: 1c0b1aa327c4487ece634eab6a0c9decffcf637888e08dc99a884ed0a7cda76a
loc_2418D,8,CALL host_poll_input|cmp word [eax+loc_2055C0],0
# sha256: 8359c8f2ab7af5c7569d43a68ce42c771d8e5c6e1349821ed39c1c0234f3785d
loc_2419C,2,CALL host_poll_input|xor eax,eax
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_B5833,8,CALL host_poll_input|cmp word [loc_2055C2],0

# == Object-file buffer check
# Original: CMP BYTE [EAX],0 tests the first byte of a freshly allocated buffer.
# Replacement: TEST EAX,EAX, a pointer check.
# Effect: the allocation's existence is tested rather than its uninitialized content.
# Why: the original relied on DOS memory that is not zero-filled; native pages are zero-filled and that content would reject a valid buffer.
# Evidence: docs/research/decisions.tsv 2026-11-11T21:05:47Z (root-cause row).
# sha256: a47fe7f6ec040a966cb24a4799930b575501c9fc0592cc5f2bb577ee74f7cdda
loc_117C5B,3,test eax,eax

# == Static objects without an animation record
# Original: loads a pointer from [EAX+0x247] (the object's animation record) and compares animation types through it.
# Replacement: after the load, TEST EAX,EAX and jump to the generic path (loc_9AC81 or loc_394D0) when the pointer is null.
# Effect: objects with no animation skip the animation-type comparisons and take the generic flag clearing, without a fake record being allocated.
# Why: the DOS reference reads a null record as the word 0xCA60, which fails both comparisons, so the generic path is what DOS takes.
# Evidence: docs/research/decisions.tsv 2026-11-11T21:19:54Z and 21:21:53Z.
# sha256: af292594c8f7811f1da7be096cbbb95241d21a79ad79d2090077259e020515dd
loc_9AC64,6,mov eax,[eax+0x247]|test eax,eax|jz loc_9AC81
# sha256: af292594c8f7811f1da7be096cbbb95241d21a79ad79d2090077259e020515dd
loc_3924E,6,mov eax,[eax+0x247]|test eax,eax|jz loc_394D0

# == Attribute setter with no selected player
# Original: CMP DWORD [EAX+0x272],0 on the player's attribute table at New Game startup.
# Replacement: when EAX is null, skip to the function's no-op exit (loc_46687); otherwise the original compare.
# Effect: with no selected player the setter returns 0 and writes nothing.
# Why: the DOS reference reads DS:null+0x272 as 0, so the original returns 0 without writing.
# Evidence: docs/research/decisions.tsv 2026-11-11T21:26:06Z; tests/original_attribute.c.
# sha256: ec6a8365e3706b686e8fbeab4454c39169ae8c8b63b8e87e2e22fe94ae08a3e0
loc_4667E,7,test eax,eax|jz loc_46687|cmp dword [eax+0x272],0

# == Null record pointer loaded from [EAX+0x25]
# Original: MOV EAX,[EAX+0x25] loads a record pointer that later code dereferences.
# Replacement: after the load, TEST EAX,EAX and jump to the function's skip path (loc_2FAD7 or loc_2FB23) when the pointer is null.
# Effect: the null case takes the skip path instead of faulting.
# Why: native null reads fault. No DOS reference measurement is recorded for these two sites.
# Evidence: none recorded beyond the fault that introduced them; treat the skip path as an unmeasured approximation.
# sha256: 080db46dbdde092af0aed1399c49b4c5c196b32961c48284691b245840e33d55
loc_2FAAC,3,mov eax,[eax+0x25]|test eax,eax|jz loc_2FAD7
# sha256: 080db46dbdde092af0aed1399c49b4c5c196b32961c48284691b245840e33d55
loc_2FAF8,3,mov eax,[eax+0x25]|test eax,eax|jz loc_2FB23

# == Movie clock
# Original: 168 bytes that initialize the movie timer through a DPMI/Windows VxD query (INT 2Fh, AX=0x1684) and a real-mode callback.
# Replacement: host service 24, which registers the host clock reader and shutdown callbacks, then RET.
# Effect: movie playback reads the native millisecond clock.
# Why: the VxD timer and real-mode callback do not exist natively.
# Evidence: runtime/host/lib.rs service 24; runtime/rust/movies.rs.
# sha256: 2b8b9a4b4841057356ece45fcb580732df1e946374777c66ee8b346b325aae99
loc_11A380,168,CALL host_movie_clock_init|RET

# == Audio startup and shutdown
# Original: loc_C84C0 is the audio initialization routine (305 bytes replaced): after calling loc_C85F4 it calls the Miles startup function at 0xCB580 and further initialization helpers. loc_CB580 is the first instruction (three PUSHes) of that startup function. loc_D03A0 is the first instruction (two PUSHes) of the Redbook CD-audio open.
# Replacement: loc_C84C0 keeps the original call to loc_C85F4, then calls host_audio_init (service 0x3000) and returns. loc_CB580 and loc_D03A0 become host_audio_startup (0x3011) and host_audio_cd_open (0x3012), each followed by RET.
# Effect: sample audio goes through the native mixer; the CD-audio open takes the original no-device path. CD music is not implemented.
# Why: the Miles driver, DOS audio hardware and a CD-ROM device are not available natively.
# Evidence: runtime/host/lib.rs service 0x3000; runtime/rust/audio.rs; docs/development/PLATFORM.md.
# sha256: 0235edcd76b8bb6e428c3fe36bc889a383a502520068e9f2cdaedbbe4643d37e
loc_C84C0,305,CALL loc_C85F4|CALL host_audio_init|RET
# sha256: 4b065669e47a4cb45505bace1c278a15421fba8813ffeb939cc9a175f2d7a2b0
loc_CB580,3,CALL host_audio_startup|RET
# sha256: 072f005fc3f2a49f2e29225d116c10a3904c326a1b810a6eb840d9a574526694
loc_D03A0,2,CALL host_audio_cd_open|RET

# == Miles callback preparation
# Original: MOV EDX,[0x3A258] loads the first Miles callback pointer.
# Replacement: CALL host_audio_callbacks_prepare first (service 0x301b), then the original MOV.
# Effect: the native callback trampolines are ready before the original code reads the pointer.
# Why: the callbacks are called from the native audio thread, so their state must exist before the game uses them.
# Evidence: runtime/rust/audio.rs.
# sha256: c3958dcdb18f93308f34ec55d5eee53e9da8e26a2e5be66bc29869f51299728c
loc_F1EE3,6,CALL host_audio_callbacks_prepare|mov edx,[loc_1AA258]

# == Point-within-radius test with no shape (catacombs)
# Original: MOV EAX,[EAX] loads the object's shape pointer; the routine then reads a float at shape+0xC (the radius) and compares (radius + range)^2 with the squared distance.
# Replacement: after the load, TEST EAX,EAX and jump to loc_BCB4F, the 'not within range' result, when the shape is null. The guard sits before the first x87 push so the floating-point stack stays balanced.
# Effect: an object with no shape is never within range, instead of faulting. Fixes the crash on entering CATACOMB.RGM.
# Why: DOS4GW reads address 0xC as the interrupt table. The result there is not measured. A DOS-reference read of that dword is a BIOS pointer, a huge float, which would make the test return 'within range'; 'not within range' is the conservative choice.
# Evidence: docs/development/VALIDATION.md 'Catacomb null-shape guard'; reproduces with entry markers 0 and 2.
# Open: measure the DOS reference at address 0xC and replace this guess with the observed value, as RG_FONT10_METRIC does.
# sha256: 736aaffccc35e5ac4a9c0d7b2f64ae9ed7ead848f3eb93c8ad4b50b9b76b5e39
loc_BCB2D,2,mov eax,[eax]|test eax,eax|jz loc_BCB4F
