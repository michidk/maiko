extern rg_exe_variant
%macro RG_VARIANT_JUMP 1
    pushfq
    cmp dword [rel rg_exe_variant], 1
    jne %%reference
    popfq
    jmp %1
%%reference:
    popfq
%endmacro
