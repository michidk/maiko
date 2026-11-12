# Byte-level replacements for the probe-only diagnostic build.
#
# The probe build runs the translated program far enough to observe the original's start-up
# without a native runtime. Besides the five Watcom idiom expansions it turns instructions
# that cannot be executed natively (16-bit transitions, descriptor-table loads, HLT, SIDT)
# into UD2, so reaching one stops the program instead of continuing wrongly. They are
# diagnostics, never game support. Their original bytes are checked in tools/pipeline.py
# (metadata(), the `expected` table); sha256 lines are therefore not repeated here.


# == Watcom overlapping-instruction idiom
# Original: the bytes 66 A9 89 CF decode as TEST AX,0xCF89, but the compiler jumps to the second byte pair, where 89 CF decodes as MOV EDI,ECX. Both readings are executed by the original program.
# Replacement: the first instruction as TEST AX,0xCF89 and an explicit JMP to the address two bytes later, so the translator sees two separate instructions instead of overlapping ones.
# Effect: identical to the original at both entry points; no behavior change.
# Why: the translator cannot represent one byte range that is two instructions. pipeline.py also checks all four original bytes before applying this expansion.
# Evidence: tools/pipeline.py metadata(); the same five sites exist in the probe table.
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_F70B6,2,test ax, 0xcf89|jmp loc_F70BA
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_F70EB,2,test ax, 0xcf89|jmp loc_F70EF
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_119CDE,2,test ax, 0xcf89|jmp loc_119CE2
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_13B3D7,2,test ax, 0xcf89|jmp loc_13B3DB
# sha256: 9d15c509a33550f45d3012567ef607e1cf7fcf65747932f0ea3bc532bcbd19b0
loc_13B461,2,test ax, 0xcf89|jmp loc_13B465

# == Unsupported 16-bit transitions
# Original: far jump (66 EA offset:selector) into 16-bit code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault, which the probe runtime reports.
# Why: the translator and probe runtime have no 16-bit execution model.
# Evidence: tools/pipeline.py expected table; docs/development/PLATFORM.md (twelve diagnostic traps).
loc_135EEC,6,ud2 ; PROBE ONLY: unsupported transition to 16-bit code; must fail if executed
loc_136137,6,ud2 ; PROBE ONLY: unsupported 16-bit transition
loc_13619A,6,ud2 ; PROBE ONLY: unsupported 16-bit transition
loc_1361F2,6,ud2 ; PROBE ONLY: unsupported 16-bit transition

# == Descriptor-table and privileged instructions
# Original: LSL, LAR, SIDT or HLT in the original's system-level code.
# Replacement: UD2.
# Effect: reaching the site raises an invalid-instruction fault.
# Why: LSL and LAR need a guest descriptor model, SIDT needs a guest IDT and HLT has no meaning natively.
# Evidence: tools/pipeline.py expected table.
loc_127C9A,3,ud2 ; PROBE ONLY: LSL needs a guest descriptor model
loc_127EB1,3,ud2 ; PROBE ONLY: LSL needs a guest descriptor model
loc_128CEB,1,ud2 ; PROBE ONLY: unsupported hlt
loc_135543,3,ud2 ; PROBE ONLY: unsupported lsl
loc_1356B5,3,ud2 ; PROBE ONLY: unsupported lsl
loc_135999,3,ud2 ; PROBE ONLY: unsupported lar
loc_144CA7,3,ud2 ; PROBE ONLY: unsupported lsl
loc_13BD6B,4,ud2 ; PROBE ONLY: unsupported sidt
