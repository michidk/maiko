loc_C85FF,0x8,0 ; JNZ carries TEST ZF to C8615
loc_C8615,0,0x8 ; JZ consumes the incoming ZF
loc_C872A,0x8,0 ; JZ also supplies following inlined JZ
loc_C8730,0,0x8 ; consumes unchanged TEST EBP ZF
loc_CA2A5,0x38,0 ; JNE carries CMP ZF/SF/OF
loc_CA2B8,0,0x38 ; shared JLE consumes CMP flags
loc_CA2A9,0x38,0 ; JL carries the second CMP flags
loc_CA2AB,0,0x38 ; fallthrough JLE consumes unchanged flags
loc_D6774,0x1,0 ; JE carries CMPSB carry into the fallthrough SBB
loc_D6776,0,0x1 ; first SBB consumes incoming carry
loc_E3E05,0x9,0 ; JAE carries CMP carry/zero to shared JBE
loc_E3E0E,0,0x9 ; shared JBE consumes the incoming flags
loc_E5FD6,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E625C,0,0x9 ; consume unchanged CMP flags
loc_E626D,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E6284,0,0x9 ; consume unchanged CMP flags
loc_E6297,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E629D,0,0x9 ; consume unchanged CMP flags
loc_E5FDF,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E62C2,0,0x9 ; consume unchanged CMP flags
loc_E62D3,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E62D9,0,0x9 ; consume unchanged CMP flags
loc_E6013,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E62FC,0,0x9 ; consume unchanged CMP flags
loc_E6325,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E6331,0,0x9 ; consume unchanged CMP flags
loc_E6338,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E633E,0,0x9 ; consume unchanged CMP flags
loc_E601C,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E634C,0,0x9 ; consume unchanged CMP flags
loc_E6364,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E636A,0,0x9 ; consume unchanged CMP flags
loc_E63A7,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E63C3,0,0x9 ; consume unchanged CMP flags
loc_E63EC,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E63F8,0,0x9 ; consume unchanged CMP flags
loc_E63FF,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E6405,0,0x9 ; consume unchanged CMP flags
loc_E63AC,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E6413,0,0x9 ; consume unchanged CMP flags
loc_E642B,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E6431,0,0x9 ; consume unchanged CMP flags
loc_E6078,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E6476,0,0x9 ; consume unchanged CMP flags
loc_E6482,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E64A4,0,0x9 ; consume unchanged CMP flags
loc_E648A,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E64FA,0,0x9 ; consume unchanged CMP flags
loc_E64B0,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E64CA,0,0x9 ; consume unchanged CMP flags
loc_E64D6,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E64DC,0,0x9 ; consume unchanged CMP flags
loc_E6506,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E650C,0,0x9 ; consume unchanged CMP flags
loc_E6081,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E652A,0,0x9 ; consume unchanged CMP flags
loc_E652F,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E6546,0,0x9 ; consume unchanged CMP flags
loc_E654B,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E6551,0,0x9 ; consume unchanged CMP flags
loc_E608A,0x9,0 ; carry and zero remain live across texture-size dispatch
loc_E656B,0,0x9 ; consume unchanged CMP flags
loc_E6648,0x9,0 ; JAE carries the original BSI FourCC comparison
loc_E6703,0,0x9 ; JBE consumes unchanged FourCC flags
loc_E670B,0x9,0 ; JAE carries LCIH FourCC comparison
loc_E6729,0,0x9 ; JA consumes unchanged FourCC flags
loc_E6747,0x9,0 ; FourCC CMP flags survive taken JAE
loc_E6767,0,0x9 ; shared JBE consumes comparison flags
loc_E6654,0x9,0 ; FourCC CMP flags survive taken JAE
loc_E6787,0,0x9 ; shared JBE consumes comparison flags
loc_E71A6,0x8,0 ; JMP carries CMP EDX,4 zero into shared size test
loc_E6D70,0,0x8 ; JNE consumes flags from either CMP
loc_E6D07,0x9,0 ; size dispatch retains CMP carry/zero
loc_E6EEA,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6D10,0x9,0 ; size dispatch retains CMP carry/zero
loc_E6F3A,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6D55,0x9,0 ; size dispatch retains CMP carry/zero
loc_E7058,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6D5E,0x9,0 ; size dispatch retains CMP carry/zero
loc_E7140,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6D67,0x9,0 ; size dispatch retains CMP carry/zero
loc_E719D,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6DEE,0x9,0 ; size dispatch retains CMP carry/zero
loc_E71B7,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6DF7,0x9,0 ; size dispatch retains CMP carry/zero
loc_E721F,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6F13,0x9,0 ; size dispatch retains CMP carry/zero
loc_E6F1F,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6F26,0x9,0 ; size dispatch retains CMP carry/zero
loc_E6F2C,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6F52,0x9,0 ; size dispatch retains CMP carry/zero
loc_E6F58,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6F93,0x9,0 ; size dispatch retains CMP carry/zero
loc_E6FAF,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6F98,0x9,0 ; size dispatch retains CMP carry/zero
loc_E6FFF,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6FD8,0x9,0 ; size dispatch retains CMP carry/zero
loc_E6FE4,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E6FEB,0x9,0 ; size dispatch retains CMP carry/zero
loc_E6FF1,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E7017,0x9,0 ; size dispatch retains CMP carry/zero
loc_E701D,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E706C,0x9,0 ; size dispatch retains CMP carry/zero
loc_E7092,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E7074,0x9,0 ; size dispatch retains CMP carry/zero
loc_E7100,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E70A6,0x9,0 ; size dispatch retains CMP carry/zero
loc_E70C0,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E70D4,0x9,0 ; size dispatch retains CMP carry/zero
loc_E70DA,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E7114,0x9,0 ; size dispatch retains CMP carry/zero
loc_E711A,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E7149,0x9,0 ; size dispatch retains CMP carry/zero
loc_E7160,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E7171,0x9,0 ; size dispatch retains CMP carry/zero
loc_E7177,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E71C8,0x9,0 ; size dispatch retains CMP carry/zero
loc_E71DF,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E71F3,0x9,0 ; size dispatch retains CMP carry/zero
loc_E71F9,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E7230,0x9,0 ; size dispatch retains CMP carry/zero
loc_E7236,0,0x9 ; second unsigned branch consumes unchanged CMP flags
loc_E9774,0x8,0 ; JMP retains CMP EDI,4 zero for shared JNE
loc_E95B7,0,0x8 ; shared JNE consumes comparison zero
loc_E959C,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E9629,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_E95A5,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E970E,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_E95AE,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E976B,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_E963A,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E9660,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_E9642,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E96CE,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_E9674,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E968E,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_E96A2,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E96A8,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_E96E2,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E96E8,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_E9717,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E972E,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_E973F,0x9,0 ; texture-size CMP carry/zero survives branch
loc_E9745,0,0x9 ; next unsigned branch consumes unchanged CMP flags
loc_EC99B,0x1,0 ; byte comparison carry survives mismatch branch
loc_EC9A3,0x1,0 ; byte comparison carry survives mismatch branch
loc_EC9B2,0x1,0 ; byte comparison carry survives mismatch branch
loc_EC9BA,0,0x1 ; shared SBB uses carry from the selected byte comparison
loc_ED961,0,0x1 ; cosine consumes carry returned by the reduction helper
loc_ED96B,0,0x1 ; sine consumes carry returned by the reduction helper
loc_ED97D,0x1,0 ; JNP carries SAHF carry to the shared epilogue
loc_ED996,0,0x1 ; shared epilogue accepts carry from SAHF or CLC
loc_ED998,0x1,0 ; return preserves helper carry for sine/cosine caller
loc_EDD4A,0x8,0 ; nonnegative read result still carries TEST zero
loc_EDD53,0,0x8 ; end-of-file branch consumes the same zero
loc_EFE74,0x8,0 ; JMP carries text-mode CMP into shared JNE
loc_EFE44,0,0x8 ; shared mode branch consumes incoming zero
loc_F010B,0x1,0 ; shared DOS access result keeps carry from INT or TEST
loc_F0110,0x1,0 ; shared DOS access result keeps carry from INT or TEST
loc_F0115,0x1,0 ; shared DOS access result keeps carry from INT or TEST
loc_F011C,0,0x1 ; SBB consumes the selected access-result carry
loc_F1CA4,0x1,0 ; REP CMPSB carry survives equality branch
loc_F1CA6,0,0x1 ; SBB consumes the last compared byte carry
loc_F3374,0x9,0 ; overlap ordering CMP also supplies equality branch
loc_F339F,0,0x9 ; shared JBE consumes incoming comparison
loc_F3FDC,0x9,0 ; unsigned tag CMP survives lower-bound branch
loc_F3FDE,0,0x9 ; LEA preserves comparison for following JBE
loc_F517C,0x8,0 ; AND result zero survives JMP
loc_F517E,0x8,0 ; absent Smacker audio track XOR supplies zero to shared JNE
loc_F5180,0,0x8 ; MOV preserves zero from AND or XOR for JNE
loc_F7105,0x1,0 ; REP CMPSB carry survives equal branch
loc_F7107,0,0x1 ; shared SBB converts mismatch carry to sign
loc_F9BF3,0x9,0 ; range ordering retains CMP carry/zero
loc_F9BF7,0x9,0 ; MOV/JMP preserve incoming range comparison
loc_F9BF5,0,0x9 ; lower-range path accepts comparison flags
loc_F9BFB,0,0x9 ; shared upper-range branch consumes comparison
loc_FB072,0x9,0 ; case-folded comparison retains carry/zero to shared result
loc_FB076,0x9,0 ; case-folded comparison retains carry/zero to shared result
loc_FB079,0x9,0 ; case-folded comparison retains carry/zero to shared result
loc_FB078,0,0x1 ; DEC preserves preceding CMP carry
loc_FB07F,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FB0C0,0x9,0 ; next folded comparison retains carry/zero
loc_FB0C4,0x9,0 ; next folded comparison retains carry/zero
loc_FB0C7,0x9,0 ; next folded comparison retains carry/zero
loc_FB0C6,0,0x1 ; DEC preserves preceding CMP carry
loc_FB0CD,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FB4C2,0x9,0 ; identical folded byte-compare result pattern
loc_FB4C6,0x9,0 ; identical folded byte-compare result pattern
loc_FB4C9,0x9,0 ; identical folded byte-compare result pattern
loc_FB4C8,0,0x1 ; DEC preserves preceding CMP carry
loc_FB4CF,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FB51D,0x9,0 ; identical folded byte-compare result pattern
loc_FB521,0x9,0 ; identical folded byte-compare result pattern
loc_FB524,0x9,0 ; identical folded byte-compare result pattern
loc_FB523,0,0x1 ; DEC preserves preceding CMP carry
loc_FB52A,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FB573,0x9,0 ; identical folded byte-compare result pattern
loc_FB577,0x9,0 ; identical folded byte-compare result pattern
loc_FB57A,0x9,0 ; identical folded byte-compare result pattern
loc_FB579,0,0x1 ; DEC preserves preceding CMP carry
loc_FB580,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FB5CE,0x9,0 ; identical folded byte-compare result pattern
loc_FB5D2,0x9,0 ; identical folded byte-compare result pattern
loc_FB5D5,0x9,0 ; identical folded byte-compare result pattern
loc_FB5D4,0,0x1 ; DEC preserves preceding CMP carry
loc_FB5DB,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FB629,0x9,0 ; identical folded byte-compare result pattern
loc_FB62D,0x9,0 ; identical folded byte-compare result pattern
loc_FB630,0x9,0 ; identical folded byte-compare result pattern
loc_FB62F,0,0x1 ; DEC preserves preceding CMP carry
loc_FB636,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FB684,0x9,0 ; identical folded byte-compare result pattern
loc_FB688,0x9,0 ; identical folded byte-compare result pattern
loc_FB68B,0x9,0 ; identical folded byte-compare result pattern
loc_FB68A,0,0x1 ; DEC preserves preceding CMP carry
loc_FB691,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FE4DD,0x9,0 ; identical folded byte-compare result pattern
loc_FE4E1,0x9,0 ; identical folded byte-compare result pattern
loc_FE4E4,0x9,0 ; identical folded byte-compare result pattern
loc_FE4E3,0,0x1 ; DEC preserves preceding CMP carry
loc_FE4EA,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FE645,0x9,0 ; identical folded byte-compare result pattern
loc_FE649,0x9,0 ; identical folded byte-compare result pattern
loc_FE64C,0x9,0 ; identical folded byte-compare result pattern
loc_FE64B,0,0x1 ; DEC preserves preceding CMP carry
loc_FE652,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FE8A2,0x9,0 ; identical folded byte-compare result pattern
loc_FE8A6,0x9,0 ; identical folded byte-compare result pattern
loc_FE8A9,0x9,0 ; identical folded byte-compare result pattern
loc_FE8A8,0,0x1 ; DEC preserves preceding CMP carry
loc_FE8AF,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FE8FE,0x9,0 ; identical folded byte-compare result pattern
loc_FE902,0x9,0 ; identical folded byte-compare result pattern
loc_FE905,0x9,0 ; identical folded byte-compare result pattern
loc_FE904,0,0x1 ; DEC preserves preceding CMP carry
loc_FE90B,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FE98A,0x9,0 ; identical folded byte-compare result pattern
loc_FE98E,0x9,0 ; identical folded byte-compare result pattern
loc_FE991,0x9,0 ; identical folded byte-compare result pattern
loc_FE990,0,0x1 ; DEC preserves preceding CMP carry
loc_FE997,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_FEA08,0x9,0 ; identical folded byte-compare result pattern
loc_FEA0C,0x9,0 ; identical folded byte-compare result pattern
loc_FEA0F,0x9,0 ; identical folded byte-compare result pattern
loc_FEA0E,0,0x1 ; DEC preserves preceding CMP carry
loc_FEA15,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_1064F3,0x9,0 ; identical folded byte-compare result pattern
loc_1064F7,0x9,0 ; identical folded byte-compare result pattern
loc_1064FA,0x9,0 ; identical folded byte-compare result pattern
loc_1064F9,0,0x1 ; DEC preserves preceding CMP carry
loc_106500,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_108717,0x9,0 ; identical folded byte-compare result pattern
loc_10871B,0x9,0 ; identical folded byte-compare result pattern
loc_10871E,0x9,0 ; identical folded byte-compare result pattern
loc_10871D,0,0x1 ; DEC preserves preceding CMP carry
loc_108724,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_108881,0x9,0 ; identical folded byte-compare result pattern
loc_108885,0x9,0 ; identical folded byte-compare result pattern
loc_108888,0x9,0 ; identical folded byte-compare result pattern
loc_108887,0,0x1 ; DEC preserves preceding CMP carry
loc_10888E,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_109A2D,0x9,0 ; identical folded byte-compare result pattern
loc_109A31,0x9,0 ; identical folded byte-compare result pattern
loc_109A34,0x9,0 ; identical folded byte-compare result pattern
loc_109A33,0,0x1 ; DEC preserves preceding CMP carry
loc_109A3A,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_109D89,0x9,0 ; identical folded byte-compare result pattern
loc_109D8D,0x9,0 ; identical folded byte-compare result pattern
loc_109D90,0x9,0 ; identical folded byte-compare result pattern
loc_109D8F,0,0x1 ; DEC preserves preceding CMP carry
loc_109D96,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_109E94,0x9,0 ; identical folded byte-compare result pattern
loc_109E98,0x9,0 ; identical folded byte-compare result pattern
loc_109E9B,0x9,0 ; identical folded byte-compare result pattern
loc_109E9A,0,0x1 ; DEC preserves preceding CMP carry
loc_109EA1,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10D644,0x9,0 ; identical folded byte-compare result pattern
loc_10D648,0x9,0 ; identical folded byte-compare result pattern
loc_10D64B,0x9,0 ; identical folded byte-compare result pattern
loc_10D64A,0,0x1 ; DEC preserves preceding CMP carry
loc_10D651,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10D687,0x9,0 ; identical folded byte-compare result pattern
loc_10D68B,0x9,0 ; identical folded byte-compare result pattern
loc_10D68E,0x9,0 ; identical folded byte-compare result pattern
loc_10D68D,0,0x1 ; DEC preserves preceding CMP carry
loc_10D694,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10D6E7,0x9,0 ; identical folded byte-compare result pattern
loc_10D6EB,0x9,0 ; identical folded byte-compare result pattern
loc_10D6EE,0x9,0 ; identical folded byte-compare result pattern
loc_10D6ED,0,0x1 ; DEC preserves preceding CMP carry
loc_10D6F4,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10DF7A,0x9,0 ; identical folded byte-compare result pattern
loc_10DF7E,0x9,0 ; identical folded byte-compare result pattern
loc_10DF81,0x9,0 ; identical folded byte-compare result pattern
loc_10DF80,0,0x1 ; DEC preserves preceding CMP carry
loc_10DF87,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10DFC0,0x9,0 ; identical folded byte-compare result pattern
loc_10DFC4,0x9,0 ; identical folded byte-compare result pattern
loc_10DFC7,0x9,0 ; identical folded byte-compare result pattern
loc_10DFC6,0,0x1 ; DEC preserves preceding CMP carry
loc_10DFCD,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10E9AC,0x9,0 ; identical folded byte-compare result pattern
loc_10E9B0,0x9,0 ; identical folded byte-compare result pattern
loc_10E9B3,0x9,0 ; identical folded byte-compare result pattern
loc_10E9B2,0,0x1 ; DEC preserves preceding CMP carry
loc_10E9B9,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10E9EE,0x9,0 ; identical folded byte-compare result pattern
loc_10E9F2,0x9,0 ; identical folded byte-compare result pattern
loc_10E9F5,0x9,0 ; identical folded byte-compare result pattern
loc_10E9F4,0,0x1 ; DEC preserves preceding CMP carry
loc_10E9FB,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10EA89,0x9,0 ; identical folded byte-compare result pattern
loc_10EA8D,0x9,0 ; identical folded byte-compare result pattern
loc_10EA90,0x9,0 ; identical folded byte-compare result pattern
loc_10EA8F,0,0x1 ; DEC preserves preceding CMP carry
loc_10EA96,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10F789,0x9,0 ; identical folded byte-compare result pattern
loc_10F78D,0x9,0 ; identical folded byte-compare result pattern
loc_10F790,0x9,0 ; identical folded byte-compare result pattern
loc_10F78F,0,0x1 ; DEC preserves preceding CMP carry
loc_10F796,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10F7D2,0x9,0 ; identical folded byte-compare result pattern
loc_10F7D6,0x9,0 ; identical folded byte-compare result pattern
loc_10F7D9,0x9,0 ; identical folded byte-compare result pattern
loc_10F7D8,0,0x1 ; DEC preserves preceding CMP carry
loc_10F7DF,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_10F8A5,0x9,0 ; identical folded byte-compare result pattern
loc_10F8A9,0x9,0 ; identical folded byte-compare result pattern
loc_10F8AC,0x9,0 ; identical folded byte-compare result pattern
loc_10F8AB,0,0x1 ; DEC preserves preceding CMP carry
loc_10F8B2,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_11039B,0x9,0 ; identical folded byte-compare result pattern
loc_11039F,0x9,0 ; identical folded byte-compare result pattern
loc_1103A2,0x9,0 ; identical folded byte-compare result pattern
loc_1103A1,0,0x1 ; DEC preserves preceding CMP carry
loc_1103A8,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_1103E9,0x9,0 ; identical folded byte-compare result pattern
loc_1103ED,0x9,0 ; identical folded byte-compare result pattern
loc_1103F0,0x9,0 ; identical folded byte-compare result pattern
loc_1103EF,0,0x1 ; DEC preserves preceding CMP carry
loc_1103F6,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_11042F,0x9,0 ; identical folded byte-compare result pattern
loc_110433,0x9,0 ; identical folded byte-compare result pattern
loc_110436,0x9,0 ; identical folded byte-compare result pattern
loc_110435,0,0x1 ; DEC preserves preceding CMP carry
loc_11043C,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_110507,0x9,0 ; identical folded byte-compare result pattern
loc_11050B,0x9,0 ; identical folded byte-compare result pattern
loc_11050E,0x9,0 ; identical folded byte-compare result pattern
loc_11050D,0,0x1 ; DEC preserves preceding CMP carry
loc_110514,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_11057A,0x9,0 ; identical folded byte-compare result pattern
loc_11057E,0x9,0 ; identical folded byte-compare result pattern
loc_110581,0x9,0 ; identical folded byte-compare result pattern
loc_110580,0,0x1 ; DEC preserves preceding CMP carry
loc_110587,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_1105C0,0x9,0 ; identical folded byte-compare result pattern
loc_1105C4,0x9,0 ; identical folded byte-compare result pattern
loc_1105C7,0x9,0 ; identical folded byte-compare result pattern
loc_1105C6,0,0x1 ; DEC preserves preceding CMP carry
loc_1105CD,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_110671,0x9,0 ; identical folded byte-compare result pattern
loc_110675,0x9,0 ; identical folded byte-compare result pattern
loc_110678,0x9,0 ; identical folded byte-compare result pattern
loc_110677,0,0x1 ; DEC preserves preceding CMP carry
loc_11067E,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_12119A,0x9,0 ; identical folded byte-compare result pattern
loc_12119E,0x9,0 ; identical folded byte-compare result pattern
loc_1211A1,0x9,0 ; identical folded byte-compare result pattern
loc_1211A0,0,0x1 ; DEC preserves preceding CMP carry
loc_1211A7,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_12120E,0x9,0 ; identical folded byte-compare result pattern
loc_121212,0x9,0 ; identical folded byte-compare result pattern
loc_121215,0x9,0 ; identical folded byte-compare result pattern
loc_121214,0,0x1 ; DEC preserves preceding CMP carry
loc_12121B,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_121309,0x9,0 ; identical folded byte-compare result pattern
loc_12130D,0x9,0 ; identical folded byte-compare result pattern
loc_121310,0x9,0 ; identical folded byte-compare result pattern
loc_12130F,0,0x1 ; DEC preserves preceding CMP carry
loc_121316,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_122B67,0x9,0 ; identical folded byte-compare result pattern
loc_122B6B,0x9,0 ; identical folded byte-compare result pattern
loc_122B6E,0x9,0 ; identical folded byte-compare result pattern
loc_122B6D,0,0x1 ; DEC preserves preceding CMP carry
loc_122B74,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_122BF4,0x9,0 ; identical folded byte-compare result pattern
loc_122BF8,0x9,0 ; identical folded byte-compare result pattern
loc_122BFB,0x9,0 ; identical folded byte-compare result pattern
loc_122BFA,0,0x1 ; DEC preserves preceding CMP carry
loc_122C01,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_122D3E,0x9,0 ; identical folded byte-compare result pattern
loc_122D42,0x9,0 ; identical folded byte-compare result pattern
loc_122D45,0x9,0 ; identical folded byte-compare result pattern
loc_122D44,0,0x1 ; DEC preserves preceding CMP carry
loc_122D4B,0,0x9 ; SETNE/JBE consume selected comparison flags
loc_108A77,0x8,0 ; XOR zero survives branch around DEC
loc_108A7A,0,0x8 ; loop tests zero from XOR or DEC
loc_108F8D,0x8,0 ; fatal-error TEST zero survives shared epilogue JMP
loc_108FA4,0x8,0 ; fatal-error TEST zero survives shared epilogue JMP
loc_108FBB,0x8,0 ; fatal-error TEST zero survives shared epilogue JMP
loc_108FEE,0,0x8 ; shared JNE consumes preceding TEST
loc_108FEC,0x8,0 ; fallthrough TEST must materialize zero before shared consumer
loc_10AB7F,0x9,0 ; command dispatch shares unsigned comparison
loc_10AB85,0,0x9 ; next branch consumes unchanged comparison
loc_10AB91,0x9,0 ; command dispatch shares unsigned comparison
loc_10AB93,0,0x9 ; next branch consumes unchanged comparison
loc_10AB9C,0x9,0 ; command dispatch shares unsigned comparison
loc_10AB9E,0,0x9 ; next branch consumes unchanged comparison
loc_10B713,0x9,0 ; identical byte-compare result pattern
loc_10B717,0x9,0 ; identical byte-compare result pattern
loc_10B71D,0,0x9 ; SETNE/JBE consume selected comparison
loc_1201B3,0x9,0 ; identical byte-compare result pattern
loc_1201B7,0x9,0 ; identical byte-compare result pattern
loc_1201BD,0,0x9 ; SETNE/JBE consume selected comparison
loc_121746,0x9,0 ; identical byte-compare result pattern
loc_12174A,0x9,0 ; identical byte-compare result pattern
loc_121750,0,0x9 ; SETNE/JBE consume selected comparison
loc_121D93,0x9,0 ; identical byte-compare result pattern
loc_121D97,0x9,0 ; identical byte-compare result pattern
loc_121D9D,0,0x9 ; SETNE/JBE consume selected comparison
loc_1220CF,0x9,0 ; identical byte-compare result pattern
loc_1220D3,0x9,0 ; identical byte-compare result pattern
loc_1220D9,0,0x9 ; SETNE/JBE consume selected comparison
loc_10BA02,0x8,0 ; XOR zero survives branch around DEC
loc_10BA06,0,0x8 ; variable-width value loop tests XOR or DEC zero
loc_110C3F,0x8,0 ; source-choice CMP zero survives MOV selection
loc_110C43,0x8,0 ; JMP retains source-choice zero
loc_110C41,0,0x8 ; alternate path accepts original zero
loc_110C48,0,0x8 ; PUSH preserves zero for destination choice
loc_11141D,0x1,0 ; TEST clears carry before following ADC
loc_11141F,0,0x1 ; ADC consumes the cleared carry
loc_111642,0x9,0 ; shared numeric dispatch preserves CMP flags
loc_111644,0,0x9 ; downstream branch consumes comparison
loc_11164C,0x8,0 ; shared numeric dispatch preserves CMP flags
loc_111654,0,0x8 ; downstream branch consumes comparison
loc_11164E,0x8,0 ; alternate fallthrough CMP materializes zero
loc_113E19,0x8,0 ; alternate escape CMP supplies shared JNE
loc_113E03,0,0x8 ; shared quote/escape JNE consumes zero
loc_113E00,0x8,0 ; fallthrough quote CMP materializes zero
loc_1196DE,0,0x1 ; CRT error conversion consumes carry returned by DOS INT21
loc_F0349,0x1,0x1 ; forward carry from DOS INT21 to error converter callee
loc_F035C,0x1,0x1 ; forward carry from DOS INT21 to error converter callee
loc_1111DE,0x1,0x1 ; forward carry from DOS INT21 to error converter callee
loc_1111EE,0x1,0x1 ; forward carry from DOS INT21 to error converter callee
loc_1111FE,0x1,0x1 ; forward carry from DOS INT21 to error converter callee
loc_11121B,0x1,0x1 ; forward carry from DOS INT21 to error converter callee
loc_111231,0x1,0x1 ; forward carry from DOS INT21 to error converter callee
loc_113361,0x1,0x1 ; forward carry from DOS INT21 to error converter callee
loc_11337F,0x1,0x1 ; forward carry from DOS INT21 to error converter callee
loc_F0344,0x1,0 ; branch retains DOS result carry for error converter
loc_1111FA,0x1,0 ; branch retains DOS result carry for error converter
loc_111213,0x1,0 ; branch retains DOS result carry for error converter
loc_11B5F1,0x8,0 ; formatting dispatch JMP preserves CMP zero
loc_11B5E4,0x8,0 ; fallthrough CMP materializes shared zero
loc_11B5E7,0,0x8 ; shared JE consumes either formatting comparison
loc_11B5D5,0x9,0 ; formatting type branch preserves comparison
loc_11B5D7,0,0x9 ; shared JBE consumes formatting comparison
loc_11B5E0,0x9,0 ; formatting type branch preserves comparison
loc_11B5E2,0,0x9 ; shared JBE consumes formatting comparison
loc_11B5F6,0x9,0 ; formatting type branch preserves comparison
loc_11B5F8,0,0x9 ; shared JBE consumes formatting comparison
loc_11B707,0x8,0 ; formatting dispatch JMP preserves CMP zero
loc_11B6FA,0x8,0 ; fallthrough CMP materializes shared zero
loc_11B6FD,0,0x8 ; shared JE consumes either formatting comparison
loc_11B6A5,0x9,0 ; formatting type branch preserves comparison
loc_11B6A7,0,0x9 ; shared JBE consumes formatting comparison
loc_11B6BE,0x9,0 ; formatting type branch preserves comparison
loc_11B6C0,0,0x9 ; shared JBE consumes formatting comparison
loc_11B6C9,0x9,0 ; formatting type branch preserves comparison
loc_11B6CB,0,0x9 ; shared JBE consumes formatting comparison
loc_11B6D4,0x9,0 ; formatting type branch preserves comparison
loc_11B6DA,0,0x9 ; shared JBE consumes formatting comparison
loc_11B6F6,0x9,0 ; formatting type branch preserves comparison
loc_11B6F8,0,0x9 ; shared JBE consumes formatting comparison
loc_11C451,0x8,0 ; JLE retains CMP zero for taken-path JNE
loc_11C45D,0,0x8 ; JNE consumes original signed comparison zero
loc_11C52B,0x8,0 ; JG retains CMP zero for following JNE
loc_11C52D,0,0x8 ; MOV path accepts preceding comparison zero
loc_11C57E,0x8,0 ; mirrored JLE retains comparison zero
loc_11C587,0,0x8 ; JNE consumes preceding comparison zero
loc_1204C3,0x1,0 ; JNE retains FTST-derived carry through FPU operations
loc_1204DF,0,0x1 ; pow range reduction consumes preserved initial sign carry
loc_1205E6,0x9,0 ; exponent loop JBE preserves SHR carry and zero
loc_1205EC,0x8,0x9 ; JAE consumes SHR carry and forwards zero
loc_1205F0,0,0x8 ; exponent loop tests zero from preceding SHR
loc_1205F7,0x8,0 ; SHR carry branch must retain SHR zero
loc_1205FB,0x8,0x8 ; back edge retains zero across FMUL
loc_126AFC,0x8,0 ; byte classification JMP preserves AND zero
loc_126AFE,0x8,0 ; alternate XOR materializes zero
loc_126B00,0,0x8 ; shared classification JE consumes selected zero
loc_126BE3,0x8,0 ; JE retains TEST zero for fallthrough duplicate JE
loc_126BE5,0,0x8 ; MOV path consumes preceding TEST zero
loc_126C00,0x8,0 ; byte classification JMP preserves AND zero
loc_126C02,0x8,0 ; alternate XOR materializes zero
loc_126C04,0,0x8 ; shared classification JE consumes selected zero
loc_126C27,0x8,0 ; classification loop JMP preserves AND zero
loc_126C29,0x8,0 ; alternate XOR materializes zero
loc_126C2B,0,0x8 ; shared classification loop consumes selected zero
loc_1282E6,0x9,0 ; OR initializes carry and zero when REPE count is zero
loc_1282EA,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_1282EC,0,0x1 ; first SBB consumes incoming CMPSB carry
loc_1295D0,0x8,0 ; copy loop JMP preserves TEST zero
loc_1295C2,0x8,0 ; initial AND materializes loop zero
loc_1295C5,0,0x8 ; byte loop consumes AND or TEST zero
loc_129612,0x8,0 ; fill loop JMP preserves TEST zero
loc_129604,0x8,0 ; initial AND materializes loop zero
loc_129607,0,0x8 ; byte loop consumes AND or TEST zero
loc_129DBF,0x8,0 ; second copy loop JMP preserves TEST zero
loc_129DB1,0x8,0 ; initial AND materializes loop zero
loc_129DB4,0,0x8 ; byte loop consumes AND or TEST zero
loc_12A1E3,0x9,0 ; DOS version dispatch JMP preserves CMP flags
loc_12A1D1,0x9,0 ; fallthrough CMP materializes shared flags
loc_12A1D5,0,0x9 ; shared JBE consumes selected version comparison
loc_12A1BB,0x9,0 ; version dispatch JB retains comparison
loc_12A1BD,0,0x9 ; JBE consumes preceding version comparison
loc_12A1E9,0x9,0 ; version dispatch JB retains comparison
loc_12A1EB,0,0x9 ; JBE consumes preceding version comparison
loc_12A279,0x9,0 ; second DOS dispatch JMP preserves CMP flags
loc_12A267,0x9,0 ; fallthrough CMP materializes shared flags
loc_12A26B,0,0x9 ; shared JBE consumes selected version comparison
loc_12A251,0x9,0 ; second version dispatch JB retains comparison
loc_12A253,0,0x9 ; JBE consumes preceding version comparison
loc_12A27F,0x9,0 ; second version dispatch JB retains comparison
loc_12A281,0,0x9 ; JBE consumes preceding version comparison
loc_12E093,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_12E095,0,0x1 ; SBB consumes incoming CMPSB carry
loc_12E202,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_12E204,0,0x1 ; SBB consumes incoming CMPSB carry
loc_D6698,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_D669A,0,0x1 ; SBB consumes incoming CMPSB carry
loc_12D641,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_12D643,0,0x1 ; SBB consumes incoming CMPSB carry
loc_12E691,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_12E693,0,0x1 ; SBB consumes incoming CMPSB carry
loc_12E7FE,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_12E800,0,0x1 ; SBB consumes incoming CMPSB carry
loc_1337FC,0x9,0 ; JAE preserves SAHF carry/zero for shared JBE
loc_133808,0,0x9 ; JBE consumes incoming floating comparison

loc_134A67,0x38,0 ; JGE carries CMP ZF/SF/OF into shared JLE
loc_134AAF,0,0x38 ; JLE consumes the same signed comparison

loc_1352E8,0x8,0 ; JMP carries byte CMP ZF to shared SETNE
loc_1352D2,0,0x8 ; SETNE consumes incoming ZF
loc_1355BB,0x1,0 ; JE retains CMPSB carry for ECX sign conversion
loc_1355BD,0,0x1 ; SBB consumes incoming CMPSB carry
loc_1355F6,0x1,0 ; JE retains CMPSB carry for ECX sign conversion
loc_1355F8,0,0x1 ; SBB consumes incoming CMPSB carry
loc_135622,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_135624,0,0x1 ; SBB consumes incoming CMPSB carry
loc_13566F,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_135671,0,0x1 ; SBB consumes incoming CMPSB carry
loc_135715,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_135717,0,0x1 ; SBB consumes incoming CMPSB carry
loc_13574E,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_135750,0,0x1 ; SBB consumes incoming CMPSB carry
loc_13B321,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_13B323,0,0x1 ; SBB consumes incoming CMPSB carry
loc_13B35F,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_13B361,0,0x1 ; SBB consumes incoming CMPSB carry
loc_13B393,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_13B395,0,0x1 ; SBB consumes incoming CMPSB carry
loc_13B401,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_13B403,0,0x1 ; SBB consumes incoming CMPSB carry
loc_13B48B,0x1,0 ; JE retains CMPSB carry for sign conversion
loc_13B48D,0,0x1 ; SBB consumes incoming CMPSB carry

loc_1393B6,0x30,0 ; JNE carries word TEST SF/OF to JGE
loc_1393BC,0,0x30 ; JGE consumes incoming sign comparison

loc_13BFAA,0x1,0 ; JBE retains unsigned CMP carry for JAE
loc_13BFB3,0,0x1 ; JAE consumes incoming carry

loc_13C525,0x1,0 ; JAE retains ADD carry for normalization RCR
loc_13C527,0,0x1 ; RCR consumes incoming ADD carry

loc_13C5F2,0x1,0 ; JNE retains ADD carry for mantissa ADC
loc_13C5F9,0,0x1 ; ADC after PUSH consumes incoming carry
loc_13C5FE,0x1,0 ; JNE retains ADD carry for exponent ADC
loc_13C602,0,0x1 ; ADC consumes incoming carry

loc_13CE4D,0x8,0 ; JGE retains CMP equality for shared JNE
loc_13CEF1,0,0x8 ; JNE consumes incoming CMP equality
loc_144E58,0x8,0x0 ; JE retains first opcode CMP equality
loc_144E5A,0x8,0x0 ; second opcode CMP materializes return equality
loc_144E5D,0x8,0x8 ; RET forwards selected opcode equality
loc_144E6A,0x8,0x0 ; JNE retains selector CMP equality
loc_144E72,0x8,0x0 ; JNE retains address CMP equality
loc_144E7D,0x0,0x8 ; epilogue consumes selected CMP equality
loc_144E7F,0x8,0x0 ; RET forwards selector/address equality
loc_144EA5,0x0,0x8 ; caller consumes equality returned by descriptor helper
loc_144EC3,0x0,0x8 ; caller consumes equality returned by descriptor helper
loc_144ED3,0x0,0x8 ; caller consumes equality returned by descriptor helper
loc_144EE3,0x0,0x8 ; caller consumes equality returned by descriptor helper
loc_144EF3,0x0,0x8 ; caller consumes equality returned by descriptor helper
loc_144EFA,0x0,0x8 ; caller consumes equality returned by descriptor helper
loc_144F28,0x0,0x8 ; caller consumes equality returned by descriptor helper
loc_144F38,0x0,0x8 ; caller consumes equality returned by descriptor helper
loc_144F48,0x0,0x8 ; caller consumes equality returned by descriptor helper
loc_D5966,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_ABC12,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_AC608,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_AC966,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_B22DA,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_B25D2,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_B25DC,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_B25AE,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_B2642,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_C7C90,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_C7CA3,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_24195,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_2419E,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_B583B,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_117C5E,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_46685,0,0x3f ; successor consumes flags from byte-checked replacement comparison
loc_5543A,0x8,0 ; variant-aware replacement consumes original CMP equality
