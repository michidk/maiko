bits 64
default rel
%include "asm_call.inc"
extern abi_host_roundtrip,abi_host_stdcall
global abi_guest,abi_guest_stdcall
section .text
abi_guest:
    fld1
    CALL abi_host_roundtrip
    fld1
    fcomip st0,st1
    fstp st0
    jz .ok
    ud2
.ok:
    RET
abi_guest_stdcall:
    CALL abi_host_stdcall, 0x12345678, 0xabcdef01
    RET
section .note.GNU-stack noalloc noexec nowrite progbits
