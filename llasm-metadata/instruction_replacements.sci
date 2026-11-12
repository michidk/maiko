loc_F70B6,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~(CF|OF|ZF|SF|PF)|cmovz eflags, tmp1, ZF, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovnz eflags, tmp2, tmp3, eflags|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|lshr tmp3, 0x9669, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_F70BA|endp
loc_F70EB,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~(CF|OF|ZF|SF|PF)|cmovz eflags, tmp1, ZF, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovnz eflags, tmp2, tmp3, eflags|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|lshr tmp3, 0x9669, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_F70EF|endp
loc_119CDE,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~(CF|OF|ZF|SF|PF)|cmovz eflags, tmp1, ZF, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovnz eflags, tmp2, tmp3, eflags|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|lshr tmp3, 0x9669, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_119CE2|endp
loc_13B3D7,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~(CF|OF|ZF|SF|PF)|cmovz eflags, tmp1, ZF, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovnz eflags, tmp2, tmp3, eflags|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|lshr tmp3, 0x9669, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_13B3DB|endp
loc_13B461,2,and tmp1, eax, 0xcf89|and eflags, eflags, ~(CF|OF|ZF|SF|PF)|cmovz eflags, tmp1, ZF, eflags|and tmp2, tmp1, 0x8000|or tmp3, eflags, SF|cmovnz eflags, tmp2, tmp3, eflags|lshr tmp2, tmp1, 4|xor tmp2, tmp2, tmp1|and tmp2, tmp2, 15|lshr tmp3, 0x9669, tmp2|and tmp3, tmp3, 1|shl tmp3, tmp3, PF_SHIFT|or eflags, eflags, tmp3|tcall loc_13B465|endp
loc_135EEC,6,call rg_translation_trap 0x135eec
loc_136137,6,call rg_translation_trap 0x136137
loc_13619A,6,call rg_translation_trap 0x13619a
loc_1361F2,6,call rg_translation_trap 0x1361f2
loc_127C9A,3,call rg_translation_trap 0x127c9a
loc_127EB1,3,call rg_translation_trap 0x127eb1
loc_128CEB,1,call rg_translation_trap 0x128ceb
loc_135543,3,call rg_translation_trap 0x135543
loc_1356B5,3,call rg_translation_trap 0x1356b5
loc_135999,3,call rg_translation_trap 0x135999
loc_144CA7,3,call rg_translation_trap 0x144ca7
loc_13BD6B,4,call rg_translation_trap 0x13bd6b
loc_D7524,16,POP tmp1|tcall tmp1|endp
loc_D7589,16,POP tmp1|tcall tmp1|endp
