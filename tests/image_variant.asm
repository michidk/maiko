bits 64
default rel
%include "image_variant.inc"
global test_variant_flags, test_variant_condition, test_skip_call
; The translated code defines CALL as its guest-stack call; a native call suffices here.
%macro CALL 1
    call %1
%endmacro
section .text
test_variant_flags:
    cmp ecx,0
    pushfq
    pop r8
    RG_SKIP_JUMP rg_exe_variant,.alternate
    mov eax,0
    jmp .done
.alternate:
    mov eax,1
.done:
    pushfq
    pop r9
    cmp r8,r9
    jne .failed
    or eax,0x100
    ret
.failed:
    mov eax,-1
    ret
test_variant_condition:
    cmp ecx,0
    RG_SKIP_JUMP rg_skip_outro,.taken
    jz .taken
    mov eax,0
    ret
.taken:
    mov eax,1
    ret
test_skip_call:
    xor eax,eax
    cmp ecx,0
    pushfq
    pop r8
    RG_SKIP_CALL rg_skip_intro,.callee
    pushfq
    pop r9
    cmp r8,r9
    jne .failed
    ret
.callee:
    mov eax,1
    ret
.failed:
    mov eax,-1
    ret
