# Byte-level replacements applied to every build, native and probe.
#
# Format and rules are the same as native-metadata/instruction_replacements.sci: the comment
# block above an entry documents it, `# sha256:` pins the original bytes, and comments are
# removed before the translator reads the file (tools/pipeline.py, read_replacements).

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
