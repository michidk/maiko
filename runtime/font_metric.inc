; Only original 0xBDCAD: DOS reference style10 copies 0xF000 from DS:null+0x46.
; No other null field reads acquire this policy.
%macro RG_FONT10_METRIC 0
    pushfq
    test eax,eax
    jnz %%source
    mov dx,0xf000
    jmp %%done
%%source:
    mov dx,[eax+0x46]
%%done:
    popfq
%endmacro
