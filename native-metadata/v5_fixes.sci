# Optional "Version 5 fixes": behavior of the build 5.0 RGFX.EXE on the reference build 1.0 image.
#
# The patcher appends this table to instruction_replacements.sci only when the version 5 fixes
# option is selected (patcher/src/prepare.rs, Options::v5_fixes). The format and the documentation
# rules are those of instruction_replacements.sci; the build refuses an entry without them.
#
# The build 5.0 executable ships with the 1998 demo (SHA-256
# 900cbfe2d586a7d80796763b1449ba050f8517b7fc7cc4742710c11e46455a31, version stamp 1.0 build 5.0).
# Every entry reproduces one change of that build at the matching site of the reference image.
# Addresses in "Evidence" are build 5.0 addresses. The demo-only island lock, the [RTX_PATCH]
# table, the F11 RTX overlay and the extra diagnostics are not included.
# Reference: https://michidk.github.io/redguard-preservation/demo.html

# == Rope attachment while dying
# Original: after the player and state checks, the rope/hook attachment test at 0x57CF7 continues at 0x57D34 and may attach the player.
# Replacement: returns "no attachment" through 0x57D28 while the player-dead flag 0x1A38B1 is set (it is set when health reaches zero and the death animation starts), otherwise runs the original MOV EAX,[EBP-0x14].
# Effect: a dying player no longer attaches to a rope or hook.
# Why: build 5.0 note "Fixed rope death-reattach bug."
# Evidence: build 5.0 0x57D93-0x57D9F (CMP EAX,1; JE; CMP [player-dead flag],0; JE).
# sha256: c59cd05e48921d247dfe9dccf64635e8cb529aa4ccf122fa638279a4230846cf
loc_57D34,3,cmp dword [loc_1A38B1],0|jne loc_57D28|mov eax,[ebp-0x14]

# == Ledge-fall push
# Original: when CollideUnder (0x6224A) starts a fall off a ledge, 0x6281D marks the actor as falling.
# Replacement: first calls 0x6B155(actor, actor->last_direction [+0x1CB], 0x1000), which moves the actor up to 4096 position units in its last movement direction, then runs the original MOV EAX,[EBP-0x28]. EBX and EDX are preserved.
# Effect: the fall starts clear of the ledge.
# Why: build 5.0 note "Fixed run of edge fall 'stutter' bug."
# Evidence: build 5.0 0x62A46-0x62A59 (MOV EBX,0x1000; MOV DL,[EAX+0x1CB]; CALL).
# sha256: 14361ab95caed99327d49adf52552e84c4651ae50e0ad0844319db4d6a2a87f7
loc_6281D,3,PUSH32 ebx|PUSH32 edx|mov eax,[ebp-0x28]|movzx edx,byte [eax+0x1cb]|mov ebx,0x1000|CALL loc_6B155|POP32 edx|POP32 ebx|mov eax,[ebp-0x28]

# == Object-flat texture binding
# Original: the two texture-record setters at 0x85151 and 0x851C4 store the raw texture ID in record +8 and leave the resolved index at +0xA unset. The flat renderer stops with "Bad Texture Index" (fx.c) when that index is 0 or above 30000.
# Replacement: rg_v5_bind_texture(handle, texture ID): unless the record already holds that ID with a nonzero index, finds the texture (0xBE4A0 with file ID>>7 & 0x1FF and image ID & 0x7F), loads its group when absent (0xBE36F), and stores the index at +0xA, the ID at +8, the frame count minus one (at least 0) at +0xE and zero at +0x10 and +0xC. An unresolvable ID is a fatal error (fx.c, code 0x24). The renderer's Bad Texture Index check is skipped.
# Effect: flats are bound to a loaded texture when created and no longer stop the game with Bad Texture Index.
# Why: build 5.0 note "Fixed texture binding crash bug on certain object flats."
# Evidence: build 5.0 setters 0x85392/0x85406 call the new resolver 0xBC2B4; the check at build 1.0 0xBC991-0xBC9E9 is absent from build 5.0.
# sha256: 093039f22ae5f7388803a23acd65876f381d3be5bc6346f177aed91c3482207b
loc_8517C,10,mov edx,[ebp-0xc]|mov eax,[ebp-0x10]|CALL rg_v5_bind_texture|jmp rg_v5_8517C_next|rg_v5_bind_texture:|PUSH32 ebx|PUSH32 ecx|PUSH32 esi|PUSH32 edi|mov edi,edx|CALL loc_85339|mov esi,eax|movsx eax,word [esi+8]|cmp eax,edi|jne rg_v5_bind_texture_find|cmp word [esi+0xa],0|jne rg_v5_bind_texture_done|rg_v5_bind_texture_find:|mov eax,edi|sar eax,7|and eax,0x1ff|mov edx,edi|and edx,0x7f|CALL loc_BE4A0|mov ecx,eax|test eax,eax|jnz rg_v5_bind_texture_found|mov eax,edi|CALL loc_BE36F|mov ecx,eax|cmp eax,-1|jne rg_v5_bind_texture_found|mov dword [loc_1A3161],0xacf|mov edx,loc_194785|mov eax,[loc_1A315D]|CALL loc_CB529|mov eax,0x24|CALL loc_22B66|rg_v5_bind_texture_found:|mov [esi+0xa],cx|mov [esi+8],di|imul eax,ecx,0x7c|mov ax,[eax+loc_2CA2F0]|dec eax|test ax,ax|jns rg_v5_bind_texture_frames|xor eax,eax|rg_v5_bind_texture_frames:|mov [esi+0xe],ax|mov word [esi+0x10],0|mov word [esi+0xc],0|rg_v5_bind_texture_done:|POP32 edi|POP32 esi|POP32 ecx|POP32 ebx|RET|rg_v5_8517C_next:
# sha256: 093039f22ae5f7388803a23acd65876f381d3be5bc6346f177aed91c3482207b
loc_851EF,10,mov edx,[ebp-0xc]|mov eax,[ebp-0x10]|CALL rg_v5_bind_texture
# sha256: 6038b609eb5d00ae4f1c3677bda9b66bc03fe57435971a4db38b1ed0590adbcc
loc_BC991,3,jmp loc_BC9EA

# == Animation slot selection
# Original: actor animation selection 0xA0E77 scans the actor's group table. The ordered scan compares each slot's group byte with the requested group; the random pick at 0xA10C6 reads the slot entry without checking it.
# Replacement: the ordered scan skips slots whose group byte is 0 (to 0xA0FF3); the random pick skips a null slot entry (to 0xA1124). Otherwise the original instruction runs.
# Effect: group-0 slots are never selected by the ordered scan, and null slots no longer fault in the random pick.
# Why: build 5.0 behavior.
# Evidence: build 5.0 0xA140C (CMP BYTE [EAX+8],0; JE) and 0xA1529 (CMP DWORD [EAX],0; JE).
# sha256: 01a81e276892953e79d27637b4e08da56e09087504a12ec3f46f2189ad1a7a40
loc_A0FCD,3,cmp byte [eax+8],0|je loc_A0FF3|mov al,[eax+8]
# sha256: 736aaffccc35e5ac4a9c0d7b2f64ae9ed7ead848f3eb93c8ad4b50b9b76b5e39
loc_A10C6,2,cmp dword [eax],0|je loc_A1124|mov eax,[eax]

# == Dialogue lines without voice
# Original: PlayRtx (0x87727) returns a string-only entry (no sample at +0xE) without setting its sound handle. With speech on, the subtitle update (0x9BFA2) skips a line whose handle is -1, so it is never drawn or freed, and the subtitle draw (0x9BE56) draws only when [dialog] dialog_print_text is on.
# Replacement: PlayRtx sets the handle of a string-only entry to -1 and continues into its start path (the null sample returns -1). The update loop sends a line without a handle to the timed-subtitle branch at 0x9C182 that speech-off lines use. The draw (0x9BE56) also proceeds for a line without a handle; otherwise it returns through the function's own epilogue, as at 0x9BF99.
# Effect: string-only lines are shown for characters x 15 units with speech on, also when subtitles are off. Voiced lines are unchanged.
# Why: build 5.0 behavior; ENGLISH.RTX has 933 string-only entries.
# Evidence: build 5.0 0x87B7B-0x87B84, 0x9C2AA-0x9C2CE and 0x9C545-0x9C54A.
# sha256: 1bddab9f36dfb3aeff11618b9c9e4b78de7215929c65e0f9cd91a8a163610e46
loc_87760,6,cmp dword [eax+0xe],0|jne rg_v5_87760_next|mov word [eax+0x12],0xffff|rg_v5_87760_next:
# sha256: 0c7a6599cd4f11f1845cf832a3cc45b830b1f83db4ceb90ac6238083aa59e632
loc_9BE86,14,cmp word [loc_1F663A],1|je rg_v5_9BE86_draw|imul eax,[ebp-0x18],0x12c|add eax,[loc_1F6634]|mov eax,[eax+0x128]|sar eax,0x10|cmp eax,-1|je rg_v5_9BE86_draw|mov r11d,ebp|POP32 ebx, ecx, edx, esi, edi, ebp|RET|rg_v5_9BE86_draw:
# sha256: b1ae738692049a8256da9d56dcf022d19316f5b80a8775bd7f08a3ad4d1735cd
loc_9C109,5,cmp eax,-1|je loc_9C182

# == Missing dialogue entries
# Original: when GetRtx (0x87368) cannot find the requested entry, 0x8755D starts the fatal-error path.
# Replacement: first retries the lookup (0x8713F) with entry name "cbu1" in the same name buffer and continues on the found path (0x875D2) when it exists; otherwise runs the original MOV EAX,5.
# Effect: a missing line plays "DON'T SUPPOSE YOU KNOW WHAT'S GOING ON." instead of stopping the game. A missing cbu1 is still fatal.
# Why: build 5.0 behavior. Build 5.0 also writes an error line to GENERAL.LOG, which is not reproduced.
# Evidence: build 5.0 0x87903-0x87976.
# sha256: fef9c338eeb24e0549bd20b4413ab17c6d37994c737d83f514e26979ef1132e2
loc_8755D,5,mov byte [ebp-0x24],0x63|mov byte [ebp-0x23],0x62|mov byte [ebp-0x22],0x75|mov byte [ebp-0x21],0x31|mov edx,[ebp-8]|lea eax,[ebp-0x24]|CALL loc_8713F|test eax,eax|jne loc_875D2|mov eax,5

# == Inventory model textures at full resolution
# Original: OpenItemSystemModels loads each item model's textures through 0xE0C64 with the [3dfx] text_scale reduction.
# Replacement: sets the reduction to 1,1 (0xE5F24), loads, then restores the configured factors from 0x200704/0x200700. EDX is preserved.
# Effect: inventory models use full-resolution textures, as the sky textures already do.
# Why: build 5.0 behavior.
# Evidence: build 5.0 0x5A239-0x5A25B; build 1.0 0xBE36F uses the same override.
# sha256: 6ecde6cbfcec29f29d42d6735a54a0394519136fb642e478a14b02fa59de71f4
loc_5A1C1,8,PUSH32 edx|mov edx,1|mov eax,1|CALL loc_E5F24|mov eax,[ebp-8]|CALL loc_E0C64|mov edx,[loc_200700]|mov eax,[loc_200704]|CALL loc_E5F24|POP32 edx
