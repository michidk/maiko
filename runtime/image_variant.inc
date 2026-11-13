extern rg_exe_variant
extern rg_skip_intro
extern rg_skip_outro
; Take the jump to %2 when the host flag %1 is nonzero; otherwise run the
; original instruction that follows. Flags are preserved for that instruction.
%macro RG_SKIP_JUMP 2
    pushfq
    cmp dword [rel %1], 0
    je %%reference
    popfq
    jmp %2
%%reference:
    popfq
%endmacro
; Call %2 unless the host flag %1 is nonzero. Only for calls whose result and
; flags the original code does not use afterwards.
%macro RG_SKIP_CALL 2
    pushfq
    cmp dword [rel %1], 0
    jne %%skip
    popfq
    CALL %2
    jmp %%done
%%skip:
    popfq
%%done:
%endmacro
