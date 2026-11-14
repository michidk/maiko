# Optional "Version 5 fixes" for the CPU-state LLasm backend.
#
# One entry per entry of native-metadata/v5_fixes.sci, at the same address and length; the
# patcher checks that pairing. The comment blocks there describe the original code, the change
# and its evidence; each block here describes the LLasm form of the same replacement.
# Reference: https://michidk.github.io/redguard-preservation/demo.html

# == Rope attachment while dying
# Original: MOV EAX,[EBP-0x14] at 0x57D34 on the rope/hook attachment path.
# Replacement: loads the player-dead flag 0x1A38B1 and continues at 0x57D28 when it is set; otherwise loads [EBP-0x14] and continues at 0x57D37.
# Effect: as native-metadata/v5_fixes.sci.
# Why: build 5.0 note "Fixed rope death-reattach bug."
# Evidence: native-metadata/v5_fixes.sci.
# sha256: c59cd05e48921d247dfe9dccf64635e8cb529aa4ccf122fa638279a4230846cf
loc_57D34,3,load tmp1, loc_1A38B1, 1|ctcallnz tmp1, loc_57D28|tcall rg_v5_57D34_part1|endp|proc rg_v5_57D34_part1|add tmpadr, ebp, -20|load eax, tmpadr, 1|tcall loc_57D37|endp

# == Ledge-fall push
# Original: MOV EAX,[EBP-0x28] at 0x6281D where CollideUnder starts a fall.
# Replacement: saves EBX and EDX, calls 0x6B155(actor, byte [actor+0x1CB], 0x1000), restores them, loads [EBP-0x28] and continues at 0x62820.
# Effect: as native-metadata/v5_fixes.sci.
# Why: build 5.0 note "Fixed run of edge fall 'stutter' bug."
# Evidence: native-metadata/v5_fixes.sci.
# sha256: 14361ab95caed99327d49adf52552e84c4651ae50e0ad0844319db4d6a2a87f7
loc_6281D,3,PUSH ebx|PUSH edx|add tmpadr, ebp, -40|load eax, tmpadr, 1|add tmpadr, eax, 459|load8z edx, tmpadr, 1|mov ebx, 4096|PUSH rg_v5_6281D_return|tcall loc_6B155|endp|proc rg_v5_6281D_return|POP edx|POP ebx|add tmpadr, ebp, -40|load eax, tmpadr, 1|tcall loc_62820|endp

# == Object-flat texture binding
# Original: the texture-record setters store the raw texture ID at 0x8517C and 0x851EF; the flat renderer checks the resolved index at 0xBC991.
# Replacement: both setters call rg_v5_bind_texture(handle, ID) and return to their next instruction; the procedures below implement the resolver of native-metadata/v5_fixes.sci, preserving EBX, ECX, ESI and EDI. 0xBC991 continues at 0xBC9EA.
# Effect: as native-metadata/v5_fixes.sci.
# Why: build 5.0 note "Fixed texture binding crash bug on certain object flats."
# Evidence: native-metadata/v5_fixes.sci.
# sha256: 093039f22ae5f7388803a23acd65876f381d3be5bc6346f177aed91c3482207b
loc_8517C,10,add tmpadr, ebp, -12|load edx, tmpadr, 1|add tmpadr, ebp, -16|load eax, tmpadr, 1|PUSH loc_85186|tcall rg_v5_bind_texture|endp|proc rg_v5_bind_texture|PUSH ebx|PUSH ecx|PUSH esi|PUSH edi|mov edi, edx|PUSH rg_v5_bind_texture_record|tcall loc_85339|endp|proc rg_v5_bind_texture_record|mov esi, eax|add tmpadr, esi, 8|load16s tmp1, tmpadr, 1|cmoveq tmp1, edi, tmp2, 1, 0|ctcallz tmp2, rg_v5_bind_texture_find|tcall rg_v5_bind_texture_check|endp|proc rg_v5_bind_texture_check|add tmpadr, esi, 10|load16z tmp1, tmpadr, 1|ctcallnz tmp1, rg_v5_bind_texture_done|tcall rg_v5_bind_texture_find|endp|proc rg_v5_bind_texture_find|ashr eax, edi, 7|and eax, eax, 0x1ff|and edx, edi, 0x7f|PUSH rg_v5_bind_texture_loaded|tcall loc_BE4A0|endp|proc rg_v5_bind_texture_loaded|mov ecx, eax|ctcallnz eax, rg_v5_bind_texture_found|tcall rg_v5_bind_texture_load|endp|proc rg_v5_bind_texture_load|mov eax, edi|PUSH rg_v5_bind_texture_group|tcall loc_BE36F|endp|proc rg_v5_bind_texture_group|mov ecx, eax|cmoveq eax, 0xffffffff, tmp1, 1, 0|ctcallz tmp1, rg_v5_bind_texture_found|tcall rg_v5_bind_texture_fatal|endp|proc rg_v5_bind_texture_fatal|store 0xacf, loc_1A3161, 1|mov edx, loc_194785|load eax, loc_1A315D, 1|PUSH rg_v5_bind_texture_fatal_code|tcall loc_CB529|endp|proc rg_v5_bind_texture_fatal_code|mov eax, 0x24|PUSH rg_v5_bind_texture_found|tcall loc_22B66|endp|proc rg_v5_bind_texture_found|add tmpadr, esi, 10|store16 ecx, tmpadr, 1|add tmpadr, esi, 8|store16 edi, tmpadr, 1|mul tmp1, ecx, 0x7c|add tmpadr, tmp1, loc_2CA2F0|load16z tmp1, tmpadr, 1|sub tmp1, tmp1, 1|ext16s tmp1, tmp1|cmovslt tmp1, 0, tmp2, 0, tmp1|add tmpadr, esi, 14|store16 tmp2, tmpadr, 1|add tmpadr, esi, 16|store16 0, tmpadr, 1|add tmpadr, esi, 12|store16 0, tmpadr, 1|tcall rg_v5_bind_texture_done|endp|proc rg_v5_bind_texture_done|POP edi|POP esi|POP ecx|POP ebx|POP tmp1|tcall tmp1|endp
# sha256: 093039f22ae5f7388803a23acd65876f381d3be5bc6346f177aed91c3482207b
loc_851EF,10,add tmpadr, ebp, -12|load edx, tmpadr, 1|add tmpadr, ebp, -16|load eax, tmpadr, 1|PUSH loc_851F9|tcall rg_v5_bind_texture|endp
# sha256: 6038b609eb5d00ae4f1c3677bda9b66bc03fe57435971a4db38b1ed0590adbcc
loc_BC991,3,tcall loc_BC9EA|endp

# == Animation slot selection
# Original: MOV AL,[EAX+8] at 0xA0FCD and MOV EAX,[EAX] at 0xA10C6 in the animation selection scans.
# Replacement: continues at 0xA0FF3 when the slot's group byte is 0, otherwise inserts it into AL and continues at 0xA0FD0; loads the slot entry and continues at 0xA1124 when it is null, otherwise at 0xA10C8.
# Effect: as native-metadata/v5_fixes.sci.
# Why: build 5.0 behavior.
# Evidence: native-metadata/v5_fixes.sci.
# sha256: 01a81e276892953e79d27637b4e08da56e09087504a12ec3f46f2189ad1a7a40
loc_A0FCD,3,add tmpadr, eax, 8|load8z tmp1, tmpadr, 1|ctcallz tmp1, loc_A0FF3|tcall rg_v5_A0FCD_part1|endp|proc rg_v5_A0FCD_part1|add tmpadr, eax, 8|load8z tmp1, tmpadr, 1|ins8ll eax, eax, tmp1|tcall loc_A0FD0|endp
# sha256: 736aaffccc35e5ac4a9c0d7b2f64ae9ed7ead848f3eb93c8ad4b50b9b76b5e39
loc_A10C6,2,load eax, eax, 1|ctcallz eax, loc_A1124|tcall loc_A10C8|endp

# == Dialogue lines without voice
# Original: CMP [EAX+0xE],0; JE at 0x87760, the subtitle-draw text check CMP WORD [0x1F663A],1 at 0x9BE86 and CMP EAX,-1; JE at 0x9C109.
# Replacement: stores 0xFFFF at [EAX+0x12] when +0xE is zero and continues at 0x87766; continues at 0x9BE94 to draw when dialog_print_text is 1 or the line's handle is -1, otherwise returns through the function epilogue of 0x9BF99; continues at 0x9C182 when the handle is -1, otherwise at 0x9C10E.
# Effect: as native-metadata/v5_fixes.sci.
# Why: build 5.0 behavior.
# Evidence: native-metadata/v5_fixes.sci.
# sha256: 1bddab9f36dfb3aeff11618b9c9e4b78de7215929c65e0f9cd91a8a163610e46
loc_87760,6,add tmpadr, eax, 14|load tmp1, tmpadr, 1|ctcallnz tmp1, loc_87766|tcall rg_v5_87760_part1|endp|proc rg_v5_87760_part1|add tmpadr, eax, 18|store16 0xffff, tmpadr, 1|tcall loc_87766|endp
# sha256: 0c7a6599cd4f11f1845cf832a3cc45b830b1f83db4ceb90ac6238083aa59e632
loc_9BE86,14,load16z tmp1, loc_1F663A, 1|cmoveq tmp1, 1, tmp2, 1, 0|ctcallnz tmp2, loc_9BE94|tcall rg_v5_9BE86_part1|endp|proc rg_v5_9BE86_part1|add tmpadr, ebp, -24|load tmp1, tmpadr, 1|mul eax, tmp1, 300|load tmp2, loc_1F6634, 1|add eax, eax, tmp2|add tmpadr, eax, 296|load eax, tmpadr, 1|ashr eax, eax, 16|cmoveq eax, 0xffffffff, tmp2, 1, 0|ctcallnz tmp2, loc_9BE94|tcall rg_v5_9BE86_return|endp|proc rg_v5_9BE86_return|mov esp, ebp|POP ebp|POP edi|POP esi|POP edx|POP ecx|POP ebx|POP tmp1|tcall tmp1|endp
# sha256: b1ae738692049a8256da9d56dcf022d19316f5b80a8775bd7f08a3ad4d1735cd
loc_9C109,5,cmoveq eax, 0xffffffff, tmp1, 1, 0|ctcallnz tmp1, loc_9C182|tcall loc_9C10E|endp

# == Missing dialogue entries
# Original: MOV EAX,5 at 0x8755D, the start of GetRtx's fatal path.
# Replacement: writes "cbu1" to the name buffer at [EBP-0x24], calls 0x8713F([EBP-0x24], [EBP-8]) and continues at 0x875D2 when it finds the entry; otherwise sets EAX to 5 and continues at 0x87562.
# Effect: as native-metadata/v5_fixes.sci.
# Why: build 5.0 behavior.
# Evidence: native-metadata/v5_fixes.sci.
# sha256: fef9c338eeb24e0549bd20b4413ab17c6d37994c737d83f514e26979ef1132e2
loc_8755D,5,add tmpadr, ebp, -36|store8 0x63, tmpadr, 1|add tmpadr, ebp, -35|store8 0x62, tmpadr, 1|add tmpadr, ebp, -34|store8 0x75, tmpadr, 1|add tmpadr, ebp, -33|store8 0x31, tmpadr, 1|add tmpadr, ebp, -8|load edx, tmpadr, 1|add eax, ebp, -36|PUSH rg_v5_8755D_return|tcall loc_8713F|endp|proc rg_v5_8755D_return|ctcallnz eax, loc_875D2|tcall rg_v5_8755D_part1|endp|proc rg_v5_8755D_part1|mov eax, 5|tcall loc_87562|endp

# == Inventory model textures at full resolution
# Original: MOV EAX,[EBP-8]; CALL 0xE0C64 at 0x5A1C1.
# Replacement: saves EDX, calls 0xE5F24(1, 1), 0xE0C64([EBP-8]) and 0xE5F24([0x200704], [0x200700]), restores EDX and continues at 0x5A1C9.
# Effect: as native-metadata/v5_fixes.sci.
# Why: build 5.0 behavior.
# Evidence: native-metadata/v5_fixes.sci.
# sha256: 6ecde6cbfcec29f29d42d6735a54a0394519136fb642e478a14b02fa59de71f4
loc_5A1C1,8,PUSH edx|mov edx, 1|mov eax, 1|PUSH rg_v5_5A1C1_scaled|tcall loc_E5F24|endp|proc rg_v5_5A1C1_scaled|add tmpadr, ebp, -8|load eax, tmpadr, 1|PUSH rg_v5_5A1C1_loaded|tcall loc_E0C64|endp|proc rg_v5_5A1C1_loaded|load edx, loc_200700, 1|load eax, loc_200704, 1|PUSH rg_v5_5A1C1_restored|tcall loc_E5F24|endp|proc rg_v5_5A1C1_restored|POP edx|tcall loc_5A1C9|endp
