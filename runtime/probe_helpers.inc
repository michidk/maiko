; Diagnostic boundaries terminate instead of pretending to implement a DOS service.
global x86_in_al_dx
x86_in_al_dx:
    jmp probe_boundary
global x86_in_al_imm
x86_in_al_imm:
    jmp probe_boundary
global x86_in_ax_dx
x86_in_ax_dx:
    jmp probe_boundary
global x86_in_ax_imm
x86_in_ax_imm:
    jmp probe_boundary
global x86_in_eax_dx
x86_in_eax_dx:
    jmp probe_boundary
global x86_in_eax_imm
x86_in_eax_imm:
    jmp probe_boundary
global x86_mov_mem_reg_16
x86_mov_mem_reg_16:
    jmp probe_boundary
global x86_mov_mem_reg_32
x86_mov_mem_reg_32:
    jmp probe_boundary
global x86_mov_mem_reg_8
x86_mov_mem_reg_8:
    jmp probe_boundary
global x86_mov_reg_mem_16
x86_mov_reg_mem_16:
    jmp probe_boundary
global x86_mov_reg_mem_32
x86_mov_reg_mem_32:
    jmp probe_boundary
global x86_mov_reg_mem_8
x86_mov_reg_mem_8:
    jmp probe_boundary
global x86_out_dx_al
x86_out_dx_al:
    jmp probe_boundary
global x86_out_dx_ax
x86_out_dx_ax:
    jmp probe_boundary
global x86_out_dx_eax
x86_out_dx_eax:
    jmp probe_boundary
global x86_out_imm_al
x86_out_imm_al:
    jmp probe_boundary
global x86_out_imm_ax
x86_out_imm_ax:
    jmp probe_boundary
global x86_out_imm_eax
x86_out_imm_eax:
    jmp probe_boundary
global x86_read_fs_dword
x86_read_fs_dword:
    jmp probe_boundary
global x86_write_fs_dword
x86_write_fs_dword:
    jmp probe_boundary
