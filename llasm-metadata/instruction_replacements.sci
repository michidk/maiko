
# == Watcom overlapping-instruction idiom
# Original: the bytes 66 A9 89 CF decode as TEST AX,0xCF89, but the compiler jumps to the second byte pair, where 89 CF decodes as MOV EDI,ECX. Both readings are executed by the original program.
# Replacement: the first instruction as TEST AX,0xCF89 and an explicit JMP to the address two bytes later, so the translator sees two separate instructions instead of overlapping ones.
# Effect: identical to the original at both entry points; no behavior change.
# Why: the translator cannot represent one byte range that is two instructions. pipeline.py also checks all four original bytes before applying this expansion.
# Evidence: tools/pipeline.py metadata().
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_F70B6,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~0x8c5|or tmp3, eflags, ZF|cmovz tmp1, eflags, tmp3, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovz tmp2, eflags, eflags, tmp3|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|mov tmp3, 0x9669|lshr tmp3, tmp3, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_F70BA|endp

# == Watcom overlapping-instruction idiom
# Original: the bytes 66 A9 89 CF decode as TEST AX,0xCF89, but the compiler jumps to the second byte pair, where 89 CF decodes as MOV EDI,ECX. Both readings are executed by the original program.
# Replacement: the first instruction as TEST AX,0xCF89 and an explicit JMP to the address two bytes later, so the translator sees two separate instructions instead of overlapping ones.
# Effect: identical to the original at both entry points; no behavior change.
# Why: the translator cannot represent one byte range that is two instructions. pipeline.py also checks all four original bytes before applying this expansion.
# Evidence: tools/pipeline.py metadata().
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_F70EB,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~0x8c5|or tmp3, eflags, ZF|cmovz tmp1, eflags, tmp3, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovz tmp2, eflags, eflags, tmp3|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|mov tmp3, 0x9669|lshr tmp3, tmp3, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_F70EF|endp

# == Watcom overlapping-instruction idiom
# Original: the bytes 66 A9 89 CF decode as TEST AX,0xCF89, but the compiler jumps to the second byte pair, where 89 CF decodes as MOV EDI,ECX. Both readings are executed by the original program.
# Replacement: the first instruction as TEST AX,0xCF89 and an explicit JMP to the address two bytes later, so the translator sees two separate instructions instead of overlapping ones.
# Effect: identical to the original at both entry points; no behavior change.
# Why: the translator cannot represent one byte range that is two instructions. pipeline.py also checks all four original bytes before applying this expansion.
# Evidence: tools/pipeline.py metadata().
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_119CDE,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~0x8c5|or tmp3, eflags, ZF|cmovz tmp1, eflags, tmp3, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovz tmp2, eflags, eflags, tmp3|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|mov tmp3, 0x9669|lshr tmp3, tmp3, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_119CE2|endp

# == Watcom overlapping-instruction idiom
# Original: the bytes 66 A9 89 CF decode as TEST AX,0xCF89, but the compiler jumps to the second byte pair, where 89 CF decodes as MOV EDI,ECX. Both readings are executed by the original program.
# Replacement: the first instruction as TEST AX,0xCF89 and an explicit JMP to the address two bytes later, so the translator sees two separate instructions instead of overlapping ones.
# Effect: identical to the original at both entry points; no behavior change.
# Why: the translator cannot represent one byte range that is two instructions. pipeline.py also checks all four original bytes before applying this expansion.
# Evidence: tools/pipeline.py metadata().
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_13B3D7,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~0x8c5|or tmp3, eflags, ZF|cmovz tmp1, eflags, tmp3, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovz tmp2, eflags, eflags, tmp3|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|mov tmp3, 0x9669|lshr tmp3, tmp3, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_13B3DB|endp

# == Watcom overlapping-instruction idiom
# Original: the bytes 66 A9 89 CF decode as TEST AX,0xCF89, but the compiler jumps to the second byte pair, where 89 CF decodes as MOV EDI,ECX. Both readings are executed by the original program.
# Replacement: the first instruction as TEST AX,0xCF89 and an explicit JMP to the address two bytes later, so the translator sees two separate instructions instead of overlapping ones.
# Effect: identical to the original at both entry points; no behavior change.
# Why: the translator cannot represent one byte range that is two instructions. pipeline.py also checks all four original bytes before applying this expansion.
# Evidence: tools/pipeline.py metadata().
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_13B461,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~0x8c5|or tmp3, eflags, ZF|cmovz tmp1, eflags, tmp3, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovz tmp2, eflags, eflags, tmp3|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|mov tmp3, 0x9669|lshr tmp3, tmp3, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_13B465|endp

# == Keyboard LED state: scroll lock and num lock
# Original: clears bit 4 (scroll lock) or bit 5 (num lock) in the BIOS keyboard flags byte at 0x417, raises INT 9 so the BIOS refreshes the LEDs, then calls the helper at 0xD7AD4 with 100.
# Replacement: calls host service 9 or 10, which clears the same bit in the host's own lock state.
# Effect: the game keeps its original lock-state logic; a native process cannot write BIOS memory or raise INT 9.
# Why: the DOS BIOS data area and software interrupts do not exist natively.
# Evidence: runtime/host/lib.rs services 9 and 10; history:aad2c0c:docs/research/RESEARCH.md keyboard section.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 6b503dd400ddcfbea3b5262026602b372f091e2252d7dcedcbf295435982bb5b
loc_20DCD,20,PUSH loc_20000|call rg_llasm_service $, 9|add esp, esp, 4|tcall loc_20DE1|endp

# == Keyboard LED state: scroll lock and num lock
# Original: clears bit 4 (scroll lock) or bit 5 (num lock) in the BIOS keyboard flags byte at 0x417, raises INT 9 so the BIOS refreshes the LEDs, then calls the helper at 0xD7AD4 with 100.
# Replacement: calls host service 9 or 10, which clears the same bit in the host's own lock state.
# Effect: the game keeps its original lock-state logic; a native process cannot write BIOS memory or raise INT 9.
# Why: the DOS BIOS data area and software interrupts do not exist natively.
# Evidence: runtime/host/lib.rs services 9 and 10; history:aad2c0c:docs/research/RESEARCH.md keyboard section.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 6c649aedb7444e65b41d586347d927c05fc616e66ceb9b348df75d59077e2708
loc_20DE1,20,PUSH loc_20000|call rg_llasm_service $, 10|add esp, esp, 4|tcall loc_20DF5|endp

# == Keyboard handler install and lock-state save
# Original: loc_D72E5 hooks the DOS keyboard interrupt through DPMI (get/set vector, saved in the original's own words) and records extended-keyboard support. loc_D734F reads the BIOS lock flags at 0x417 into the original's caps/scroll/num variables.
# Replacement: host services 16 and 17. Service 16 allocates the keyboard stack, sets the extended-handler flag and starts native event delivery; the original key translation tables and handler logic are retained. Service 17 zeroes the three lock variables.
# Effect: key events reach the original translation code without a DOS interrupt vector.
# Why: DPMI vector hooks and BIOS memory are unavailable natively.
# Evidence: runtime/host/lib.rs services 16 and 17 ('original key translation retained').
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 23a74eed5606a4f0e78c1e5c6ab1c0ed392b2a34838003b42d95e0d0f0144700
loc_D72E5,79,PUSH loc_20000|call rg_llasm_service $, 16|add esp, esp, 4|tcall loc_D7334|endp

# == Keyboard handler install and lock-state save
# Original: loc_D72E5 hooks the DOS keyboard interrupt through DPMI (get/set vector, saved in the original's own words) and records extended-keyboard support. loc_D734F reads the BIOS lock flags at 0x417 into the original's caps/scroll/num variables.
# Replacement: host services 16 and 17. Service 16 allocates the keyboard stack, sets the extended-handler flag and starts native event delivery; the original key translation tables and handler logic are retained. Service 17 zeroes the three lock variables.
# Effect: key events reach the original translation code without a DOS interrupt vector.
# Why: DPMI vector hooks and BIOS memory are unavailable natively.
# Evidence: runtime/host/lib.rs services 16 and 17 ('original key translation retained').
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 7da01710ea56ac54cc3192e10ececb749f783885520e43d70d655b134fab879d
loc_D734F,94,PUSH loc_20000|call rg_llasm_service $, 17|add esp, esp, 4|tcall loc_D73AD|endp

# == Interrupt-handler tail: EOI and IRET
# Original: end of a hardware interrupt handler: sends end-of-interrupt (OUT 0x20,0x20), restores GS/FS/ES/DS and the general registers (POPA) and executes IRET.
# Replacement: RET.
# Effect: the handler is entered as an ordinary function call by the host and returns like one; the registers it would restore were not saved by an interrupt entry.
# Why: the host calls the handler directly. There is no PIC and no interrupt frame, so OUT and IRET would fault or corrupt the stack.
# Evidence: original bytes at these two sites are identical 16-byte tails (see sha256).
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 0264c2fa6cb654ab416fc87960181d5d7ab098d8c50306a170c1c19d1163025c
loc_D7524,16,POP tmp1|tcall tmp1|endp

# == Interrupt-handler tail: EOI and IRET
# Original: end of a hardware interrupt handler: sends end-of-interrupt (OUT 0x20,0x20), restores GS/FS/ES/DS and the general registers (POPA) and executes IRET.
# Replacement: RET.
# Effect: the handler is entered as an ordinary function call by the host and returns like one; the registers it would restore were not saved by an interrupt entry.
# Why: the host calls the handler directly. There is no PIC and no interrupt frame, so OUT and IRET would fault or corrupt the stack.
# Evidence: original bytes at these two sites are identical 16-byte tails (see sha256).
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 0264c2fa6cb654ab416fc87960181d5d7ab098d8c50306a170c1c19d1163025c
loc_D7589,16,POP tmp1|tcall tmp1|endp

# == VGA mode 13h request
# Original: PUSHA; INT 10h with AX=0x13 (320x200 VGA mode); POPA.
# Replacement: host service 20, which creates the native 640x480 window that the original Glide initialization then draws into.
# Effect: the legacy VGA mode switch becomes window preparation; Glide setup is untouched.
# Why: no BIOS video service natively; the Glide build never displays the VGA mode.
# Evidence: runtime/host/lib.rs service 20.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 93f493d541780ad24ccd3a0fbca1252a1d912cec52a1c995dfa2b31f723fe641
loc_B9D2F,9,PUSH loc_20000|call rg_llasm_service $, 20|add esp, esp, 4|tcall loc_B9D38|endp

# == Null numeric input in the original string-to-number conversion
# Original: MOV DL,[EAX] reads the first character of the input string.
# Replacement: returns zero from the conversion when EAX is null (TEST EAX,EAX; JZ to the function's zero-result path), otherwise executes the original MOV.
# Effect: null input converts to 0. Every non-null conversion path is unchanged.
# Why: DOS4GW lets a null read return low memory; a native process faults. The game reaches this with a null configuration string while parsing the attack/defence/taunt tables.
# Evidence: history:aad2c0c:docs/research/RESEARCH.md ('native-only byte-checked guard at original 0xEB55D'); history:aad2c0c:docs/research/decisions.tsv 2026-11-11T16:15:37Z.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: f07aeb88b7e724ef0713d76dbd681acc2c11e4b1fe9d7500770ded65b29bd9a5
loc_EB55D,2,ctcallz eax, loc_EB5C3|tcall rg_patch_EB55D_part1|endp|proc rg_patch_EB55D_part1|load8z tmp0, eax, 1|ins8ll edx, edx, tmp0|tcall loc_EB55F|endp

# == Image variants: two RGFX.EXE builds
# Original: the executable at the GOG install (variant 0) has JZ 0x5544F at 0x55441 and MOV ECX,2 at 0xAD96B. The owner's RGFX.EXE (variant 1) differs by exactly three bytes: JZ becomes an unconditional JMP to 0x5544F, and the first two bytes of MOV ECX,2 become a JMP to 0xAD9E0.
# Replacement: RG_VARIANT_JUMP takes the variant-1 jump only when the executable hash selected variant 1 at startup; otherwise the original instruction runs.
# Effect: both known executables behave as they do natively in their own builds.
# Why: both must run. Which one is the unmodified release is not established.
# Evidence: metadata/image_variants.json; history:aad2c0c:docs/research/RESEARCH.md ('exactly three changed bytes').
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 57a97cadea0452df11971965cf92e9e279b06b4b5b2806b94dc1174a4fa06871
loc_55441,2,call rg_llasm_variant|ctcallnz tmp0, loc_5544F|tcall rg_patch_55441_part1|endp|proc rg_patch_55441_part1|and tmp0, eflags, ZF|ctcallnz tmp0, loc_5544F|tcall loc_55443|endp

# == Image variants: two RGFX.EXE builds
# Original: the executable at the GOG install (variant 0) has JZ 0x5544F at 0x55441 and MOV ECX,2 at 0xAD96B. The owner's RGFX.EXE (variant 1) differs by exactly three bytes: JZ becomes an unconditional JMP to 0x5544F, and the first two bytes of MOV ECX,2 become a JMP to 0xAD9E0.
# Replacement: RG_VARIANT_JUMP takes the variant-1 jump only when the executable hash selected variant 1 at startup; otherwise the original instruction runs.
# Effect: both known executables behave as they do natively in their own builds.
# Why: both must run. Which one is the unmodified release is not established.
# Evidence: metadata/image_variants.json; history:aad2c0c:docs/research/RESEARCH.md ('exactly three changed bytes').
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 86c50ee89b1749fd3d78aafa56fe1abe55d39899809d5f2cf0908a56fc213149
loc_AD96B,5,call rg_llasm_variant|ctcallnz tmp0, loc_AD9E0|tcall rg_patch_AD96B_part1|endp|proc rg_patch_AD96B_part1|mov ecx, 2|tcall loc_AD970|endp

# == Null pointer in directory-scan cleanup
# Original: CMP ECX,[EDX+4] with EDX = ECX = 0 after a directory scan that found nothing.
# Replacement: when EDX is null, jump to the function's normal exit (loc_D5726); otherwise the original CMP.
# Effect: an empty scan cleans up without reading memory at address 4.
# Why: DOS can read address 4 harmlessly; a native process faults.
# Evidence: history:aad2c0c:docs/research/decisions.tsv 2026-11-11T17:34:55Z; history:aad2c0c:docs/research/RESEARCH.md ('cleanup null dereference at original 0xD5963').
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: a2e0ef9208b2e1d50c6859cecbc4b780c6cbb4f1edb1d9347d81abbcdae81f4c
loc_D5963,3,ctcallz edx, loc_D5726|tcall rg_patch_D5963_part1|endp|proc rg_patch_D5963_part1|add tmpadr, edx, 4|load tmp0, tmpadr, 1|call x86_compare $, ecx, tmp0, 32|tcall loc_D5966|endp

# == Font style 10 metric from a null source
# Original: MOV DX,[EAX+0x46] reads a font metric word.
# Replacement: RG_FONT10_METRIC (runtime/font_metric.inc): the same read for a non-null source; 0xF000 for null. Flags are saved and restored and the upper half of EDX is preserved.
# Effect: reproduces what the DOS reference showed for this one site.
# Why: the DOS reference read 0xF000 from the interrupt-table high word at DS:null+0x46; native Windows faults.
# Evidence: history:aad2c0c:docs/research/decisions.tsv 2026-11-11T17:56:23Z (reference observation); history:aad2c0c:docs/research/RESEARCH.md. This models the observed DOS reference, not every BIOS or every null read.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 5007d7e7615549b9d4dd169698d7401f35a90756ce557248e6e40ae7b61cd559
loc_BDCAD,4,mov tmp0, 0xf000|ifnz eax|add tmpadr, eax, 70|load16z tmp0, tmpadr, 1|endif|ins16 edx, edx, tmp0|tcall loc_BDCB1|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_ABC0A,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|load16z tmp0, loc_2055C2, 1|call x86_compare $, tmp0, 0, 16|tcall loc_ABC12|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_AC600,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|load16z tmp0, loc_2055C2, 1|call x86_compare $, tmp0, 0, 16|tcall loc_AC608|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_AC95E,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|load16z tmp0, loc_2055C2, 1|call x86_compare $, tmp0, 0, 16|tcall loc_AC966|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 1c0b1aa327c4487ece634eab6a0c9decffcf637888e08dc99a884ed0a7cda76a
loc_B22D2,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|add tmpadr, eax, loc_2055C0|load16z tmp0, tmpadr, 1|call x86_compare $, tmp0, 0, 16|tcall loc_B22DA|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 1c0b1aa327c4487ece634eab6a0c9decffcf637888e08dc99a884ed0a7cda76a
loc_B25CA,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|add tmpadr, eax, loc_2055C0|load16z tmp0, tmpadr, 1|call x86_compare $, tmp0, 0, 16|tcall loc_B25D2|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_B25D4,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|load16z tmp0, loc_2055C2, 1|call x86_compare $, tmp0, 0, 16|tcall loc_B25DC|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: f11c186b89af21d5eb5bdba4fd3a66c29a9c410285c9f553563a6ede2bed0241
loc_B25A6,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|load16z tmp0, loc_205612, 1|call x86_compare $, tmp0, 0, 16|tcall loc_B25AE|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 59003b473ded936542904e1f9cc1b4b2c9e11c5cc326f32098840bae1d83b7dc
loc_B263A,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|load16z tmp0, loc_2055F8, 1|call x86_compare $, tmp0, 0, 16|tcall loc_B2642|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_C7C88,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|load16z tmp0, loc_2055C2, 1|call x86_compare $, tmp0, 0, 16|tcall loc_C7C90|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 1c0b1aa327c4487ece634eab6a0c9decffcf637888e08dc99a884ed0a7cda76a
loc_C7C9B,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|add tmpadr, eax, loc_2055C0|load16z tmp0, tmpadr, 1|call x86_compare $, tmp0, 0, 16|tcall loc_C7CA3|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 1c0b1aa327c4487ece634eab6a0c9decffcf637888e08dc99a884ed0a7cda76a
loc_2418D,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|add tmpadr, eax, loc_2055C0|load16z tmp0, tmpadr, 1|call x86_compare $, tmp0, 0, 16|tcall loc_24195|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 8359c8f2ab7af5c7569d43a68ce42c771d8e5c6e1349821ed39c1c0234f3785d
loc_2419C,2,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|mov eax, 0|call x86_compare $, 0, 0, 32|tcall loc_2419E|endp

# == Input polling in busy-wait loops
# Original: loops and key checks compare a flag word that a DOS keyboard interrupt updates asynchronously (CMP WORD [flag],0 at the 0x955C0 area).
# Replacement: CALL host_poll_input immediately before the original compare. The call is a service entry; the host pumps window and keyboard events on every service entry, which updates the flag, then the original compare runs.
# Effect: waits for a key or a skip now end when the player presses a key. Original timing and control flow are unchanged.
# Why: no interrupt can update the flag natively, so these loops would never exit.
# Evidence: runtime/host/lib.rs (service 23 returns after the pump); docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 2ae4f835c954a9eb89db264142786e0993493be52da561bc0137fd6d38e71c61
loc_B5833,8,PUSH loc_20000|call rg_llasm_service $, 23|add esp, esp, 4|load16z tmp0, loc_2055C2, 1|call x86_compare $, tmp0, 0, 16|tcall loc_B583B|endp

# == Object-file buffer check
# Original: CMP BYTE [EAX],0 tests the first byte of a freshly allocated buffer.
# Replacement: TEST EAX,EAX, a pointer check.
# Effect: the allocation's existence is tested rather than its uninitialized content.
# Why: the original relied on DOS memory that is not zero-filled; native pages are zero-filled and that content would reject a valid buffer.
# Evidence: history:aad2c0c:docs/research/decisions.tsv 2026-11-11T21:05:47Z (root-cause row).
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: a47fe7f6ec040a966cb24a4799930b575501c9fc0592cc5f2bb577ee74f7cdda
loc_117C5B,3,call x86_compare $, eax, 0, 32|tcall loc_117C5E|endp

# == Static objects without an animation record
# Original: loads a pointer from [EAX+0x247] (the object's animation record) and compares animation types through it.
# Replacement: after the load, TEST EAX,EAX and jump to the generic path (loc_9AC81 or loc_394D0) when the pointer is null.
# Effect: objects with no animation skip the animation-type comparisons and take the generic flag clearing, without a fake record being allocated.
# Why: the DOS reference reads a null record as the word 0xCA60, which fails both comparisons, so the generic path is what DOS takes.
# Evidence: history:aad2c0c:docs/research/decisions.tsv 2026-11-11T21:19:54Z and 21:21:53Z.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: af292594c8f7811f1da7be096cbbb95241d21a79ad79d2090077259e020515dd
loc_9AC64,6,add tmpadr, eax, 583|load eax, tmpadr, 1|ctcallz eax, loc_9AC81|tcall loc_9AC6A|endp

# == Static objects without an animation record
# Original: loads a pointer from [EAX+0x247] (the object's animation record) and compares animation types through it.
# Replacement: after the load, TEST EAX,EAX and jump to the generic path (loc_9AC81 or loc_394D0) when the pointer is null.
# Effect: objects with no animation skip the animation-type comparisons and take the generic flag clearing, without a fake record being allocated.
# Why: the DOS reference reads a null record as the word 0xCA60, which fails both comparisons, so the generic path is what DOS takes.
# Evidence: history:aad2c0c:docs/research/decisions.tsv 2026-11-11T21:19:54Z and 21:21:53Z.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: af292594c8f7811f1da7be096cbbb95241d21a79ad79d2090077259e020515dd
loc_3924E,6,add tmpadr, eax, 583|load eax, tmpadr, 1|ctcallz eax, loc_394D0|tcall loc_39254|endp

# == Attribute setter with no selected player
# Original: CMP DWORD [EAX+0x272],0 on the player's attribute table at New Game startup.
# Replacement: when EAX is null, skip to the function's no-op exit (loc_46687); otherwise the original compare.
# Effect: with no selected player the setter returns 0 and writes nothing.
# Why: the DOS reference reads DS:null+0x272 as 0, so the original returns 0 without writing.
# Evidence: history:aad2c0c:docs/research/decisions.tsv 2026-11-11T21:26:06Z; tests/original_attribute.c.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: ec6a8365e3706b686e8fbeab4454c39169ae8c8b63b8e87e2e22fe94ae08a3e0
loc_4667E,7,ctcallz eax, loc_46687|tcall rg_patch_4667E_part1|endp|proc rg_patch_4667E_part1|add tmpadr, eax, 626|load tmp0, tmpadr, 1|call x86_compare $, tmp0, 0, 32|tcall loc_46685|endp

# == Null record pointer loaded from [EAX+0x25]
# Original: MOV EAX,[EAX+0x25] loads a record pointer that later code dereferences.
# Replacement: after the load, TEST EAX,EAX and jump to the function's skip path (loc_2FAD7 or loc_2FB23) when the pointer is null.
# Effect: the null case takes the skip path instead of faulting.
# Why: native null reads fault. No DOS reference measurement is recorded for these two sites.
# Evidence: none recorded beyond the fault that introduced them; treat the skip path as an unmeasured approximation.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 080db46dbdde092af0aed1399c49b4c5c196b32961c48284691b245840e33d55
loc_2FAAC,3,add tmpadr, eax, 37|load eax, tmpadr, 1|ctcallz eax, loc_2FAD7|tcall loc_2FAAF|endp

# == Null record pointer loaded from [EAX+0x25]
# Original: MOV EAX,[EAX+0x25] loads a record pointer that later code dereferences.
# Replacement: after the load, TEST EAX,EAX and jump to the function's skip path (loc_2FAD7 or loc_2FB23) when the pointer is null.
# Effect: the null case takes the skip path instead of faulting.
# Why: native null reads fault. No DOS reference measurement is recorded for these two sites.
# Evidence: none recorded beyond the fault that introduced them; treat the skip path as an unmeasured approximation.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 080db46dbdde092af0aed1399c49b4c5c196b32961c48284691b245840e33d55
loc_2FAF8,3,add tmpadr, eax, 37|load eax, tmpadr, 1|ctcallz eax, loc_2FB23|tcall loc_2FAFB|endp

# == Movie clock
# Original: 168 bytes that initialize the movie timer through a DPMI/Windows VxD query (INT 2Fh, AX=0x1684) and a real-mode callback.
# Replacement: host service 24, which registers the host clock reader and shutdown callbacks, then RET.
# Effect: movie playback reads the native millisecond clock.
# Why: the VxD timer and real-mode callback do not exist natively.
# Evidence: runtime/host/lib.rs service 24; runtime/rust/movies.rs.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 2b8b9a4b4841057356ece45fcb580732df1e946374777c66ee8b346b325aae99
loc_11A380,168,PUSH loc_20000|call rg_llasm_service $, 24|add esp, esp, 4|POP tmp1|tcall tmp1|endp

# == Audio startup and shutdown
# Original: loc_C84C0 is the audio initialization routine (305 bytes replaced): after calling loc_C85F4 it calls the Miles startup function at 0xCB580 and further initialization helpers. loc_CB580 is the first instruction (three PUSHes) of that startup function. loc_D03A0 is the first instruction (two PUSHes) of AIL_redbook_open_drive, the CD-audio open.
# Replacement: loc_C84C0 keeps the original call to loc_C85F4, then calls host_audio_init (service 0x3000) and returns. loc_CB580 becomes host_audio_startup (0x3011) and loc_D03A0 becomes host_redbook_open_drive (0x3100), each followed by RET.
# Effect: sample audio goes through the native mixer; the CD-audio open succeeds when the GOG disc image and its cue sheet are found (runtime/rust/redbook.rs), and otherwise returns 0, the original no-CD path. The other AIL_redbook_* functions are replaced as external procedures (native-metadata/external_procedures.sci).
# Why: the Miles driver, DOS audio hardware and a CD-ROM device are not available natively.
# Evidence: runtime/host/lib.rs service 0x3000 and 0x3100; runtime/rust/audio.rs; runtime/rust/redbook.rs; docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls.
# sha256: 0235edcd76b8bb6e428c3fe36bc889a383a502520068e9f2cdaedbbe4643d37e
loc_C84C0,305,PUSH rg_after_audio_close|tcall loc_C85F4|endp|proc rg_after_audio_close|PUSH loc_20000|call rg_llasm_service $, 12288|add esp, esp, 4|POP tmp1|tcall tmp1|endp

# == Audio startup and shutdown
# Original: loc_C84C0 is the audio initialization routine (305 bytes replaced): after calling loc_C85F4 it calls the Miles startup function at 0xCB580 and further initialization helpers. loc_CB580 is the first instruction (three PUSHes) of that startup function. loc_D03A0 is the first instruction (two PUSHes) of AIL_redbook_open_drive, the CD-audio open.
# Replacement: loc_C84C0 keeps the original call to loc_C85F4, then calls host_audio_init (service 0x3000) and returns. loc_CB580 becomes host_audio_startup (0x3011) and loc_D03A0 becomes host_redbook_open_drive (0x3100), each followed by RET.
# Effect: sample audio goes through the native mixer; the CD-audio open succeeds when the GOG disc image and its cue sheet are found (runtime/rust/redbook.rs), and otherwise returns 0, the original no-CD path. The other AIL_redbook_* functions are replaced as external procedures (native-metadata/external_procedures.sci).
# Why: the Miles driver, DOS audio hardware and a CD-ROM device are not available natively.
# Evidence: runtime/host/lib.rs service 0x3000 and 0x3100; runtime/rust/audio.rs; runtime/rust/redbook.rs; docs/architecture.md.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls.
# sha256: 4b065669e47a4cb45505bace1c278a15421fba8813ffeb939cc9a175f2d7a2b0
loc_CB580,3,PUSH loc_20000|call rg_llasm_service $, 12305|add esp, esp, 4|POP tmp1|tcall tmp1|endp

# == Miles callback preparation
# Original: MOV EDX,[0x3A258] loads the first Miles callback pointer.
# Replacement: CALL host_audio_callbacks_prepare first (service 0x301b), then the original MOV.
# Effect: the native callback trampolines are ready before the original code reads the pointer.
# Why: the callbacks are called from the native audio thread, so their state must exist before the game uses them.
# Evidence: runtime/rust/audio.rs.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: c3958dcdb18f93308f34ec55d5eee53e9da8e26a2e5be66bc29869f51299728c
loc_F1EE3,6,PUSH loc_20000|call rg_llasm_service $, 12315|add esp, esp, 4|load edx, loc_1AA258, 1|tcall loc_F1EE9|endp

# == Point-within-radius test with no shape (catacombs)
# Original: MOV EAX,[EAX] loads the object's shape pointer; the routine then reads a float at shape+0xC (the radius) and compares (radius + range)^2 with the squared distance.
# Replacement: after the load, TEST EAX,EAX and jump to loc_BCB4F, the 'not within range' result, when the shape is null. The guard sits before the first x87 push so the floating-point stack stays balanced.
# Effect: an object with no shape is never within range, instead of faulting. Fixes the crash on entering CATACOMB.RGM.
# Why: DOS4GW reads address 0xC as the interrupt table. The result there is not measured. A DOS-reference read of that dword is a BIOS pointer, a huge float, which would make the test return 'within range'; 'not within range' is the conservative choice.
# Evidence: history:aad2c0c:docs/development/VALIDATION.md 'Catacomb null-shape guard'; reproduces with entry markers 0 and 2.
# Open: measure the DOS reference at address 0xC and replace this guess with the observed value, as RG_FONT10_METRIC does.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 736aaffccc35e5ac4a9c0d7b2f64ae9ed7ead848f3eb93c8ad4b50b9b76b5e39
loc_BCB2D,2,load eax, eax, 1|ctcallz eax, loc_BCB4F|tcall loc_BCB2F|endp

# == Script step with no script pointer (Vile)
# Original: the actor script interpreter (loc_1631A5, in the hand-written assembly object) reads the next opcode with MOV AL,[EDX], where EDX is the actor's script counter. Opcodes outside 0..30 go to loc_163386, which resets the counter to the script's start and returns.
# Replacement: when EDX is null, jump to loc_163386; otherwise the original MOV.
# Effect: an actor whose script counter is null restarts its script instead of faulting. Every non-null script runs exactly as before.
# Why: DOS lets the read at address 0 succeed, returning a byte of low memory; a native process faults. Which opcode that byte decodes to, and so what DOS did, has not been measured. Restarting is the interpreter's own response to an unusable opcode.
# Evidence: reproduces deterministically by loading world 14 (VILE.RGM) and playing for about 25 seconds with tests/probe_world_wander.py; faults at address 0 with EDX = 0.
# Open: find which actor has the null counter and whether DOS left it null too.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 7891d3d56ef3106aec0998ecabbc1d924863e15a03a4fd3ff312f341f9b58c68
loc_1631B1,2,ctcallz edx, loc_163386|tcall rg_patch_1631B1_part1|endp|proc rg_patch_1631B1_part1|load8z tmp0, edx, 1|ins8ll eax, eax, tmp0|tcall loc_1631B3|endp

# == Unsupported 16-bit transitions (fatal trap)
# Original: a far jump (66 EA offset:selector) from 32-bit code into 16-bit code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on, instead of continuing with wrong behavior.
# Why: neither the translator nor the runtime has a 16-bit execution model. These four sites are in unsupported mixed-mode system code and are not reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm; twelve such traps remain in the native build.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: ff1adf1d1bd5d6b819e6111ce361fd9b9b191a057b77871ae88827b3b6487a04
loc_135EEC,6,call rg_translation_trap 0x135eec|POP tmp1|tcall tmp1|endp

# == Unsupported 16-bit transitions (fatal trap)
# Original: a far jump (66 EA offset:selector) from 32-bit code into 16-bit code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on, instead of continuing with wrong behavior.
# Why: neither the translator nor the runtime has a 16-bit execution model. These four sites are in unsupported mixed-mode system code and are not reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm; twelve such traps remain in the native build.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: ff1adf1d1bd5d6b819e6111ce361fd9b9b191a057b77871ae88827b3b6487a04
loc_136137,6,call rg_translation_trap 0x136137|POP tmp1|tcall tmp1|endp

# == Unsupported 16-bit transitions (fatal trap)
# Original: a far jump (66 EA offset:selector) from 32-bit code into 16-bit code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on, instead of continuing with wrong behavior.
# Why: neither the translator nor the runtime has a 16-bit execution model. These four sites are in unsupported mixed-mode system code and are not reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm; twelve such traps remain in the native build.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: ff1adf1d1bd5d6b819e6111ce361fd9b9b191a057b77871ae88827b3b6487a04
loc_13619A,6,call rg_translation_trap 0x13619a|POP tmp1|tcall tmp1|endp

# == Unsupported 16-bit transitions (fatal trap)
# Original: a far jump (66 EA offset:selector) from 32-bit code into 16-bit code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on, instead of continuing with wrong behavior.
# Why: neither the translator nor the runtime has a 16-bit execution model. These four sites are in unsupported mixed-mode system code and are not reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm; twelve such traps remain in the native build.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: ff1adf1d1bd5d6b819e6111ce361fd9b9b191a057b77871ae88827b3b6487a04
loc_1361F2,6,call rg_translation_trap 0x1361f2|POP tmp1|tcall tmp1|endp

# == Descriptor-table and privileged instructions (fatal trap)
# Original: LSL, LAR, SIDT or HLT in the original's system-level code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on.
# Why: LSL and LAR need a guest descriptor model, SIDT needs a guest interrupt descriptor table, and HLT has no meaning in a native process. None is reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 60cbe1165c0f7184a47c68b9418ad25066b5dfa06cacfa4bbd86e3ec1dcf13ca
loc_127C9A,3,call rg_translation_trap 0x127c9a|POP tmp1|tcall tmp1|endp

# == Descriptor-table and privileged instructions (fatal trap)
# Original: LSL, LAR, SIDT or HLT in the original's system-level code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on.
# Why: LSL and LAR need a guest descriptor model, SIDT needs a guest interrupt descriptor table, and HLT has no meaning in a native process. None is reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 60cbe1165c0f7184a47c68b9418ad25066b5dfa06cacfa4bbd86e3ec1dcf13ca
loc_127EB1,3,call rg_translation_trap 0x127eb1|POP tmp1|tcall tmp1|endp

# == Descriptor-table and privileged instructions (fatal trap)
# Original: LSL, LAR, SIDT or HLT in the original's system-level code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on.
# Why: LSL and LAR need a guest descriptor model, SIDT needs a guest interrupt descriptor table, and HLT has no meaning in a native process. None is reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 2017ff3461395672aa0aa4f64894fd2f95a4b120e2690e8951656d79adc2eed2
loc_128CEB,1,call rg_translation_trap 0x128ceb|POP tmp1|tcall tmp1|endp

# == Descriptor-table and privileged instructions (fatal trap)
# Original: LSL, LAR, SIDT or HLT in the original's system-level code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on.
# Why: LSL and LAR need a guest descriptor model, SIDT needs a guest interrupt descriptor table, and HLT has no meaning in a native process. None is reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 60cbe1165c0f7184a47c68b9418ad25066b5dfa06cacfa4bbd86e3ec1dcf13ca
loc_135543,3,call rg_translation_trap 0x135543|POP tmp1|tcall tmp1|endp

# == Descriptor-table and privileged instructions (fatal trap)
# Original: LSL, LAR, SIDT or HLT in the original's system-level code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on.
# Why: LSL and LAR need a guest descriptor model, SIDT needs a guest interrupt descriptor table, and HLT has no meaning in a native process. None is reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 60cbe1165c0f7184a47c68b9418ad25066b5dfa06cacfa4bbd86e3ec1dcf13ca
loc_1356B5,3,call rg_translation_trap 0x1356b5|POP tmp1|tcall tmp1|endp

# == Descriptor-table and privileged instructions (fatal trap)
# Original: LSL, LAR, SIDT or HLT in the original's system-level code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on.
# Why: LSL and LAR need a guest descriptor model, SIDT needs a guest interrupt descriptor table, and HLT has no meaning in a native process. None is reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 820240448fc9d0e855c61584ed04a8a069253d608b84bab4083cd4887b6b6423
loc_135999,3,call rg_translation_trap 0x135999|POP tmp1|tcall tmp1|endp

# == Descriptor-table and privileged instructions (fatal trap)
# Original: LSL, LAR, SIDT or HLT in the original's system-level code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on.
# Why: LSL and LAR need a guest descriptor model, SIDT needs a guest interrupt descriptor table, and HLT has no meaning in a native process. None is reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm.
# Replacement: LLasm traps the complete LSL/JE/MOV decision, including the unreachable ZF consumer.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: e9f61fbeb43abeefd82d4399122796d994a534f8d16e1cfe6d95f2fe7c035d16
loc_144CA7,10,call rg_translation_trap 0x144ca7|POP tmp1|tcall tmp1|endp

# == Descriptor-table and privileged instructions (fatal trap)
# Original: LSL, LAR, SIDT or HLT in the original's system-level code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the native fault reporter prints and stops on.
# Why: LSL and LAR need a guest descriptor model, SIDT needs a guest interrupt descriptor table, and HLT has no meaning in a native process. None is reached during normal play.
# Evidence: tests/check_original_fault.py; tests/entry_fault.asm.
# Replacement: LLasm uses explicit CPU-state host calls and tail calls; the behavior above is retained.
# sha256: 17204d99c7f0f84dccfc4b09f720a13bacc4af033aa234d7a39210584f35662a
loc_13BD6B,4,call rg_translation_trap 0x13bd6b|POP tmp1|tcall tmp1|endp

# == Fresh-start book
# Original: TEST EAX,EAX and JNZ preserve the original ignore_autosave direct-world path; otherwise the original starts the intro.
# Replacement: Keep the nonzero direct-world branch; clear autosave admission and initial selection, then enter the original book setup after its admission store.
# Effect: matches the startup behavior of the established native backend.
# Why: macOS uses LLasm and cannot use the NASM text hooks.
# Evidence: tools/pipeline.py native_startup_book and native_startup_clock document the same guarded original sites; Mac execution remains unverified.
# sha256: c6e5be83949a6c61a883302319cc7b32987553441a38773c841986bf79063160
loc_2063A,4,ctcallnz eax, loc_20697|tcall rg_patch_2063A_part1|endp|proc rg_patch_2063A_part1|store 0, loc_1A1DA3, 1|store 0, loc_1A3F74, 4|tcall loc_2060A|endp

# == Fresh-start book fade and selection
# Original: Copy autosave admission into the book fade request.
# Replacement: Request the fade independently and select the first enabled New Game item on a fresh start, with the same bounded list walk as native_startup_book.
# Effect: matches the startup behavior of the established native backend.
# Why: macOS uses LLasm and cannot use the NASM text hooks.
# Evidence: tools/pipeline.py native_startup_book and native_startup_clock document the same guarded original sites; Mac execution remains unverified.
# sha256: b62d4e090b5d93c639833515cef018c0255dd93c121550c80f7cc7eb0b9382de
loc_ABB3F,10,store8 1, loc_1FA372, 2|load tmp1, loc_1A1DA3, 1|ctcallnz tmp1, loc_ABB49|tcall rg_patch_ABB3F_part1|endp|proc rg_patch_ABB3F_part1|load eax, loc_1A667B, 1|mov ecx, 64|tcall rg_startup_select_new|endp|proc rg_startup_select_new|ctcallz eax, loc_ABB49|tcall rg_patch_ABB3F_part2|endp|proc rg_patch_ABB3F_part2|add tmpadr, eax, 4|load tmp1, tmpadr, 1|cmoveq tmp1, 10, tmp1, 0, 1|ctcallnz tmp1, rg_startup_select_next|tcall rg_patch_ABB3F_part3|endp|proc rg_patch_ABB3F_part3|add tmpadr, eax, 12|load8z tmp1, tmpadr, 1|and tmp2, tmp1, 8|ctcallnz tmp2, rg_startup_select_next|tcall rg_patch_ABB3F_part4|endp|proc rg_patch_ABB3F_part4|add tmpadr, eax, 12|load8z tmp1, tmpadr, 1|and tmp2, tmp1, 1|ctcallz tmp2, rg_startup_select_next|tcall rg_patch_ABB3F_part5|endp|proc rg_patch_ABB3F_part5|store eax, loc_1A6677, 1|tcall loc_ABB49|endp|proc rg_startup_select_next|add tmpadr, eax, 12|load8z tmp1, tmpadr, 1|and tmp1, tmp1, 64|ctcallnz tmp1, loc_ABB49|tcall rg_patch_ABB3F_part6|endp|proc rg_patch_ABB3F_part6|add tmpadr, eax, 49|load eax, tmpadr, 1|sub ecx, ecx, 1|ctcallnz ecx, rg_startup_select_new|tcall loc_ABB49|endp

# == Startup book fade admission
# Original: Enter the fade only when an autosave was admitted.
# Replacement: Admit the fade for either an autosave or startup book context 1.
# Effect: matches the startup behavior of the established native backend.
# Why: macOS uses LLasm and cannot use the NASM text hooks.
# Evidence: tools/pipeline.py native_startup_book and native_startup_clock document the same guarded original sites; Mac execution remains unverified.
# sha256: 47771aa863c2f6820ab77e74ee498a581ffc303285e197d1301b7ea15d3995c4
loc_AB81A,13,load tmp1, loc_1A1DA3, 1|cmoveq tmp1, 1, tmp1, 0, 1|ctcallz tmp1, loc_AB827|tcall rg_patch_AB81A_part1|endp|proc rg_patch_AB81A_part1|load tmp1, loc_1F9BD0, 4|cmoveq tmp1, 1, tmp1, 0, 1|ctcallnz tmp1, loc_AB91B|tcall loc_AB827|endp

# == Disable Continue without an autosave
# Original: Disable Continue only in book context 2.
# Replacement: Also disable Continue in startup context 1 when no autosave was admitted.
# Effect: matches the startup behavior of the established native backend.
# Why: macOS uses LLasm and cannot use the NASM text hooks.
# Evidence: tools/pipeline.py native_startup_book and native_startup_clock document the same guarded original sites; Mac execution remains unverified.
# sha256: f95a40dc56960cc1a78d502959895f9c7f67c109460bd6fc4317c3125d749b23
loc_AF874,9,load tmp1, loc_1F9BD0, 4|cmoveq tmp1, 2, tmp2, 0, 1|ctcallz tmp2, loc_AF87D|tcall rg_patch_AF874_part1|endp|proc rg_patch_AF874_part1|load tmp1, loc_1F9BD0, 4|cmoveq tmp1, 1, tmp2, 0, 1|ctcallnz tmp2, loc_AF889|tcall rg_patch_AF874_part2|endp|proc rg_patch_AF874_part2|load tmp1, loc_1A1DA3, 1|ctcallnz tmp1, loc_AF889|tcall loc_AF87D|endp

# == Ignore startup Escape without an autosave
# Original: Context 2 ignores Escape; other contexts resume save slot 1.
# Replacement: Also ignore Escape in startup context 1 when there is no admitted autosave.
# Effect: matches the startup behavior of the established native backend.
# Why: macOS uses LLasm and cannot use the NASM text hooks.
# Evidence: tools/pipeline.py native_startup_book and native_startup_clock document the same guarded original sites; Mac execution remains unverified.
# sha256: fa17eabea2efec8cb3de1418836b566e9ccbf25a5dc135bbaf41e4d017845f3a
loc_AC8A4,9,load tmp1, loc_1F9BD0, 4|cmoveq tmp1, 2, tmp2, 0, 1|ctcallz tmp2, loc_AC8AD|tcall rg_patch_AC8A4_part1|endp|proc rg_patch_AC8A4_part1|load tmp1, loc_1F9BD0, 4|cmoveq tmp1, 1, tmp2, 0, 1|ctcallnz tmp2, loc_AC8AF|tcall rg_patch_AC8A4_part2|endp|proc rg_patch_AC8A4_part2|load tmp1, loc_1A1DA3, 1|ctcallnz tmp1, loc_AC8AF|tcall loc_AC8AD|endp

# == Pace startup fixed steps
# Original: Call the original fixed-step clock update at loc_5F761.
# Replacement: Call shared host service 27 first, then the original clock update with its original return address.
# Effect: matches the startup behavior of the established native backend.
# Why: macOS uses LLasm and cannot use the NASM text hooks.
# Evidence: tools/pipeline.py native_startup_book and native_startup_clock document the same guarded original sites; Mac execution remains unverified.
# sha256: aa7ec252b14d19640b7b3e0e45b875d7e1cd4d79965b5e93d873545118c14521
loc_5F6BC,5,PUSH loc_20000|call rg_llasm_service $, 27|add esp, esp, 4|PUSH loc_5F6C1|tcall loc_5F761|endp

# == Intro movie skip (--skip-intro)
# Original: CALL 0xB1464 with EAX=0, EDX=0, EBX=1 plays movie 0 (MENU.INI movie_name[0] = INTRO.SMK). 0x2078D is in main()'s New Game command (0x1771); 0x20651 is the MENU.INI ignore_autosave=1 direct start. The result is not used: both are followed by MOV EDX,0xFFFFFF00; XOR EAX,EAX.
# Replacement: RG_SKIP_CALL makes the call unless the host sets rg_skip_intro (runtime/rust/install.rs, the --skip-intro option).
# Effect: with --skip-intro, New Game goes straight to the ship. Without it the original call runs. Movies the scripts queue later (main() at 0x2095A) still play.
# Why: players asked to skip the intro without patching their files.
# Evidence: the Linux New Game probe logs 'MOVIE missing d:\INTRO.SMK' after the New Game command without the option and no INTRO.SMK request with it; the call targets were read from the original bytes (E8 rel32 to 0xB1464).
# Replacement: LLasm loads the shared host option and either returns to the original continuation or calls the original movie function.
# sha256: 2214e5ba23177230f3b4f80e5c61a524049f469b26a37cc83649321d88b4ffb3
loc_20651,5,load tmp1, rg_skip_intro, 4|ctcallnz tmp1, loc_20656|tcall rg_intro_20651|endp|proc rg_intro_20651|PUSH loc_20656|tcall loc_B1464|endp

# == Intro movie skip (--skip-intro)
# Original: CALL 0xB1464 with EAX=0, EDX=0, EBX=1 plays movie 0 (MENU.INI movie_name[0] = INTRO.SMK). 0x2078D is in main()'s New Game command (0x1771); 0x20651 is the MENU.INI ignore_autosave=1 direct start. The result is not used: both are followed by MOV EDX,0xFFFFFF00; XOR EAX,EAX.
# Replacement: RG_SKIP_CALL makes the call unless the host sets rg_skip_intro (runtime/rust/install.rs, the --skip-intro option).
# Effect: with --skip-intro, New Game goes straight to the ship. Without it the original call runs. Movies the scripts queue later (main() at 0x2095A) still play.
# Why: players asked to skip the intro without patching their files.
# Evidence: the Linux New Game probe logs 'MOVIE missing d:\INTRO.SMK' after the New Game command without the option and no INTRO.SMK request with it; the call targets were read from the original bytes (E8 rel32 to 0xB1464).
# Replacement: LLasm loads the shared host option and either returns to the original continuation or calls the original movie function.
# sha256: 650bd1075bb2ea5b5d060052e70f5349f163574a5888207d259b81ad9141bc51
loc_2078D,5,load tmp1, rg_skip_intro, 4|ctcallnz tmp1, loc_20792|tcall rg_intro_2078D|endp|proc rg_intro_2078D|PUSH loc_20792|tcall loc_B1464|endp

# == Load a selected save before entering the startup book
# Original: Set EAX to context 1, call the original book and store its command result.
# Replacement: If rg_load_save is nonnegative, copy the slot to the original load slot, clear the host request and return the original Continue command. Otherwise call the original book.
# Effect: --load-save follows the same original load-game path as the native backend.
# Why: the shared host exposes this option on both backends, and an admitted slot must not be silently ignored.
# Evidence: tools/pipeline.py native_startup_book checks these original bytes and uses the same slot, command and one-shot request.
# sha256: 477e14edebca80c9aab027ccbfcc373cfb5fddfe18953de36a9af565925c9832
loc_2060A,13,load tmp1, rg_load_save, 4|cmovslt tmp1, 0, tmp2, 1, 0|ctcallnz tmp2, rg_startup_book_menu|tcall rg_startup_load_save|endp|proc rg_startup_load_save|load tmp1, rg_load_save, 4|store tmp1, loc_1F9C0C, 4|store -1, rg_load_save, 4|mov eax, 0x1772|tcall rg_startup_book_result|endp|proc rg_startup_book_menu|mov eax, 1|PUSH rg_startup_book_result|tcall loc_AB924|endp|proc rg_startup_book_result|add tmpadr, ebp, -16|store eax, tmpadr, 1|tcall loc_20617|endp


# == Audio startup and shutdown
# Original: loc_C84C0 is the audio initialization routine (305 bytes replaced): after calling loc_C85F4 it calls the Miles startup function at 0xCB580 and further initialization helpers. loc_CB580 is the first instruction (three PUSHes) of that startup function. loc_D03A0 is the first instruction (two PUSHes) of AIL_redbook_open_drive, the CD-audio open.
# Replacement: loc_C84C0 keeps the original call to loc_C85F4, then calls host_audio_init (service 0x3000) and returns. loc_CB580 becomes host_audio_startup (0x3011) and loc_D03A0 becomes host_redbook_open_drive (0x3100), each followed by RET.
# Effect: sample audio goes through the native mixer; the CD-audio open succeeds when the GOG disc image and its cue sheet are found (runtime/rust/redbook.rs), and otherwise returns 0, the original no-CD path. The other AIL_redbook_* functions are replaced as external procedures (native-metadata/external_procedures.sci).
# Why: the Miles driver, DOS audio hardware and a CD-ROM device are not available natively.
# Evidence: runtime/host/lib.rs service 0x3000 and 0x3100; runtime/rust/audio.rs; runtime/rust/redbook.rs; docs/architecture.md.
# Replacement: LLasm calls the shared Redbook open service through the CPU-state adapter.
# sha256: 072f005fc3f2a49f2e29225d116c10a3904c326a1b810a6eb840d9a574526694
loc_D03A0,2,PUSH loc_20000|call rg_llasm_service $, 12544|add esp, esp, 4|POP tmp1|tcall tmp1|endp
