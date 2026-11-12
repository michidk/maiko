bits 64
%include "asm_unwind.inc"
global rg_unresolved_far
section .text
translated_section:
SECTION_PROLOG
rg_unresolved_far:
ud2
translated_end:
section .pdata rdata align=4
P_UNWIND_INFO translated_section, translated_end, translated_unwind
section .xdata rdata align=4
translated_unwind:
X_UNWIND_INFO translated_section
