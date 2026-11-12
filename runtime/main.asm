; Process entry forwarding only. Game initialization lives in Rust.
section .text
global main
extern rg_native_main
main:
    jmp rg_native_main
%ifidn __OUTPUT_FORMAT__,elf64
section .note.GNU-stack noalloc noexec nowrite progbits
%endif
