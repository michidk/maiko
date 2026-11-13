bits 64
default rel
%include "asm_call.inc"
%include "asm_unwind.inc"
extern abi_host_stdcall
global abi_guest_stdcall
extern abi_host_roundtrip
extern abi_host_unwind
global abi_guest_unwind
global abi_guest
section .text
abi_guest_stdcall:
    CALL abi_host_stdcall, 0x12345678, 0xabcdef01
    RET
abi_guest:
    CALL abi_host_roundtrip
    RET

section .text
unwind_prolog:
SECTION_PROLOG
abi_guest_unwind:
    mov rax, r13
    mov rdx, rsp
    CALL abi_host_unwind
    RET
unwind_end:
section .pdata rdata align=4
P_UNWIND_INFO unwind_prolog, unwind_end, unwind_info
section .xdata rdata align=8
unwind_info:
X_UNWIND_INFO unwind_prolog
