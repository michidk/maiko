bits 64
default rel
%include "asm_call.inc"
%include "asm_unwind.inc"
%ifidn __OUTPUT_FORMAT__, elf64
%macro SECTION_PROLOG 0
    push rsi
    push rdi
    push rbp
    push rbx
    push r13
    sub rsp, 32
    mov r13, rsp
%endmacro
%macro SECTION_EPILOG 0
    add rsp, 32
    pop r13
    pop rbx
    pop rbp
    pop rdi
    pop rsi
    ret
%endmacro
%endif
extern rg_service
global rg_call, rg_guest_return
section .text
; Windows ABI -> SR register machine. r12-r15 are reserved to the host.
rg_call:
    push rbx
    .prolog_1:
    push rbp
    .prolog_2:
    push rsi
    .prolog_3:
    push rdi
    .prolog_4:
    push r12
    .prolog_5:
    push r13
    .prolog_6:
    push r14
    .prolog_7:
    push r15
    .prolog_8:
    sub rsp, 552
    .prolog_alloc:
    fxsave64 [rsp+32]
    .end_prolog:
    mov r15, rsp
%ifidn __OUTPUT_FORMAT__, win64
    mov r12d, ecx
    mov r14, rdx
%else
    mov r12d, edi
    mov r14, rsi
%endif
    call guest_enter
rg_guest_return:
    mov rsp, r15
    fxrstor64 [rsp+32]
    add rsp, 552
    pop r15
    pop r14
    pop r13
    pop r12
    pop rdi
    pop rsi
    pop rbp
    pop rbx
    ret
rg_call_end:

; SR's section unwind records require this exact frame, even when we
; enter a translated function directly rather than its section prolog.
guest_enter:
    SECTION_PROLOG
    mov eax, [r14]
    mov ebx, [r14+4]
    mov ecx, [r14+8]
    mov edx, [r14+12]
    mov esi, [r14+16]
    mov edi, [r14+20]
    mov ebp, [r14+24]
    mov r11d, [r14+28]
    cld
    CALL r12
    mov [r14], eax
    mov [r14+4], ebx
    mov [r14+8], ecx
    mov [r14+12], edx
    mov [r14+16], esi
    mov [r14+20], edi
    mov [r14+24], ebp
    mov [r14+28], r11d
    pushfq
    pop r8
    mov [r14+32], r8d
    SECTION_EPILOG
guest_enter_end:
%ifidn __OUTPUT_FORMAT__, win64
section .pdata rdata align=4
P_UNWIND_INFO rg_call, rg_call_end, rg_call_unwind
P_UNWIND_INFO guest_enter, guest_enter_end, guest_enter_unwind
section .xdata rdata align=8
guest_enter_unwind:
X_UNWIND_INFO guest_enter
rg_call_unwind:
    db 1, rg_call.end_prolog-rg_call, 30, 0
    ; FXSAVE64 stores nonvolatile XMM6..15 at RSP+32+160+16*n.
%assign xmm_reg 15
%rep 10
    db rg_call.end_prolog-rg_call, (xmm_reg << 4) | 8
    dw (32+160+16*xmm_reg)/16
%assign xmm_reg xmm_reg-1
%endrep
    db rg_call.prolog_alloc-rg_call, 1
    dw 552/8
    db rg_call.prolog_8-rg_call, 0xf0
    db rg_call.prolog_7-rg_call, 0xe0
    db rg_call.prolog_6-rg_call, 0xd0
    db rg_call.prolog_5-rg_call, 0xc0
    db rg_call.prolog_4-rg_call, 0x70
    db rg_call.prolog_3-rg_call, 0x60
    db rg_call.prolog_2-rg_call, 0x50
    db rg_call.prolog_1-rg_call, 0x30
%endif
section .text

%macro HOST_SERVICE 2-3 0
 global %1
 %1:
    pushfq
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r11
    sub rsp, 552
    fxsave64 [rsp+32]
    lea rdx, [rsp+552]
%ifidni %2, INTERRUPT
    mov ecx, r9d
    or ecx, 0x1000
%else
    mov ecx, %2
%endif
    cld
%ifidn __OUTPUT_FORMAT__, elf64
    mov edi, ecx
    mov rsi, rdx
%endif
    call rg_service
    fxrstor64 [rsp+32]
    add rsp, 552
    pop r11
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    popfq
    RET %3
%endmacro

%ifdef ABI_TEST
HOST_SERVICE abi_host_roundtrip, 42
HOST_SERVICE abi_host_unwind, 43
HOST_SERVICE abi_host_stdcall, 44, 8
%else
%include "native_services.inc"
%include "native_traces.inc"
%endif
%ifidn __OUTPUT_FORMAT__, elf64
section .note.GNU-stack noalloc noexec nowrite progbits
%endif
