bits 64
default rel
%include "font_metric.inc"
global rg_test_font10_metric
section .text
rg_test_font10_metric:
    mov eax,ecx
    mov r9,rdx
    mov edx,0x12345678
    xor r8d,r8d
    stc
    pushfq
    pop r10
    RG_FONT10_METRIC
    pushfq
    pop r8
    xor r8,r10
    mov [r9],r8d
    mov eax,edx
    ret
