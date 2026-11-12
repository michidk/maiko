bits 64
default rel
extern rg_entry
extern stack_start
%include "asm_call.inc"
%include "asm_pushx.inc"
extern probe_interrupt
extern probe_boundary
global rg_run
global x86_int
section .text
rg_run:
    sub rsp, 40
    lea r11, [stack_start]
    xor eax, eax
    xor ebx, ebx
    xor ecx, ecx
    xor edx, edx
    xor esi, esi
    xor edi, edi
    xor ebp, ebp
    jmp rg_entry
x86_int:
    mov r8d, [r11d]
    mov edx, eax
    mov ecx, r9d
    call probe_interrupt
    ud2
%define host_unsupported_hardware probe_boundary
%include "hardware_boundaries.inc"
