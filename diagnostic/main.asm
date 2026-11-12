section .text
global main
extern rg_probe_main
main:
    jmp rg_probe_main
