; Keep the guest ABI symbol separate so ABI fixtures can supply their own host.
section .text
global rg_service
extern rg_host_service
rg_service:
    jmp rg_host_service
%ifidn __OUTPUT_FORMAT__,elf64
section .note.GNU-stack noalloc noexec nowrite progbits
%endif
