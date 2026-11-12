bits 64
default rel
%include "image_variant.inc"
global test_variant_flags, test_variant_condition
section .text
test_variant_flags:
    cmp ecx,0
    pushfq
    pop r8
    RG_VARIANT_JUMP .alternate
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
    RG_VARIANT_JUMP .taken
    jz .taken
    mov eax,0
    ret
.taken:
    mov eax,1
    ret
