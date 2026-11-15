extern rg_call_head, rg_call_ring
%unmacro ACTION_CALL 0
%macro ACTION_CALL 0
    pushfq
    mov r10d, [rel rg_call_head]
    and r10d, 63
    lea r8, [rel rg_call_ring]
    mov dword [r8+r10*4], %%site
    inc dword [rel rg_call_head]
    popfq
%%site:
%endmacro
