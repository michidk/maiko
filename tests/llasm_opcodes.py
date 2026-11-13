"""Translate synthetic LE instructions through the actual pinned SR backend."""
import argparse
from pathlib import Path
import struct
import subprocess

def le_fixture(code, relocations=()):
    data=bytearray(512+4096)
    data[:2]=b'MZ';struct.pack_into('<H',data,0x18,0x40);struct.pack_into('<I',data,0x3c,0x40)
    data[0x40:0x44]=b'LE\0\0'
    for offset,value in {20:1,24:1,32:2,36:4096,40:4096,44:4096,
                         64:192,68:2,72:240,104:244,108:252,128:512}.items():
        struct.pack_into('<I',data,0x40+offset,value)
    struct.pack_into('<6I',data,0x40+192,4096,0x20000,0x2005,1,1,0)
    struct.pack_into('<6I',data,0x40+216,4096,0x30000,0x2003,2,0,0)
    data[0x40+240:0x40+244]=b'\0\0\1\0'
    records=b''.join(struct.pack('<BBHBI',7,0x10,offset,2,0) for offset in relocations)
    struct.pack_into('<II',data,0x40+244,0,len(records))
    data[0x40+252:0x40+252+len(records)]=records
    data[512:512+len(code)]=code
    return data

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('translator',type=Path)
    parser.add_argument('output',type=Path)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    # BSWAP does not change flags. Retain a distinctive caller flag word.
    code=bytearray(b'\x90'*1088)
    cases=[(0,'rg_test','0fc8c3'),(16,'rg_test_edi','0fcfc3'),
           (32,'rg_test_memory','8b030fc88903c3'),
           (48,'rg_test_float','d903d803d91bc3'),
           # Both paths reach a shared JZ that consumes the first TEST's flags.
           (80,'rg_flags','85c07505b8111111117406b822222222c3b833333333c3'),
           (112,'rg_signed_divide','99f7ffc3'),
           (128,'rg_unsigned_divide','31d2f7f7c3'),
           (160,'rg_compare','39f87506b800000000c37e06b801000000c3b8ffffffffc3')]
    # DC E0+i and DC F0+i reverse the operands and write ST(i), without popping.
    for register in range(1, 8):
        cases.append((224 + (register - 1) * 16, f'rg_fsubr_to_{register}', f'dce{register}c3'))
        cases.append((336 + (register - 1) * 16, f'rg_fdivr_to_{register}', f'dcf{register}c3'))
    cases.append((464, 'rg_round', 'd9fcc3'))
    cases.append((480, 'rg_load_extended', 'db2bc3'))
    cases.append((496, 'rg_remainder', 'd9f8c3'))
    cases.append((512, 'rg_atan', 'd9f3c3'))
    cases.append((528, 'rg_ftst', 'd9e4c3'))
    cases.append((544, 'rg_jecxz', 'e306b801000000c3b802000000c3'))
    cases.append((560, 'rg_jo', '7006b801000000c3b802000000c3'))
    cases.append((576, 'rg_jno', '7106b801000000c3b802000000c3'))
    cases.append((592, 'rg_store_extended', 'db3bc3'))
    cases.append((608, 'rg_scale', 'd9fdc3'))
    cases.append((624, 'rg_exp2minus1', 'd9f0c3'))
    cases.append((640, 'rg_log2e', 'd9eac3'))
    cases.append((656, 'rg_log2ten', 'd9e9c3'))
    cases.append((704, 'rg_pack_dwords', '0f6f030f6f4b080f6bc10f7f43100f77c3'))
    cases.append((736, 'rg_byte_rotate', 'd2c8c3'))
    cases.append((752, 'rg_variable_shift', 'd3eac3'))
    cases.append((768, 'rg_high_not', 'f6d5c3'))
    cases.append((784, 'rg_high_exchange', '8637c3'))
    cases.append((800, 'rg_byte_carry', '10e0c3'))
    cases.append((832, 'rg_bit_scan', '0fbcd0c3'))
    cases.append((848, 'rg_cpu_features', '0fa2c3'))
    cases.append((864, 'rg_scan_not_equal', 'f2afc3'))
    cases.append((880, 'rg_scan_equal', 'f3afc3'))
    cases.append((896, 'rg_high_logic', '082fc3'))
    cases.append((912, 'rg_high_pair', '86fcc3'))
    cases.append((928, 'rg_overlap_test', '66a989cfc3'))
    cases.append((930, 'rg_overlap_move', '89cfc3'))
    # Relocated, scaled destinations use tmp0 during address calculation.
    # The x87 result must survive that calculation, including at index zero.
    indexed=[('rg_indexed_fst','d9148d'),('rg_indexed_fstp','d91c8d'),
             ('rg_indexed_fist','db148d'),('rg_indexed_fistp','db1c8d'),
             ('rg_indexed_fistp16','df1c4d'),('rg_indexed_cw','d93c4d'),
             ('rg_indexed_sw','dd3c4d')]
    relocations=[]
    for i,(name,opcode) in enumerate(indexed):
        offset=960+i*16
        cases.append((offset,name,opcode+'00000300c3'))
        relocations.append(offset+3)
    # Exercise the actual overlap expansion at both legal entry points.
    replacement = next(line for line in
        (Path(__file__).resolve().parents[1] / 'llasm-metadata/instruction_replacements.sci').read_text().splitlines()
        if line.startswith('loc_F70B6,'))
    replacement = replacement.replace('loc_F70B6,', 'loc_203A0,').replace('loc_F70BA', 'loc_203A4')
    (args.output / 'instruction_replacements.sci').write_text(replacement + '\n')

    for offset,name,machine_code in cases:
        machine_code=bytes.fromhex(machine_code)
        code[offset:offset+len(machine_code)]=machine_code
    (args.output/'test.exe').write_bytes(le_fixture(code,relocations))
    (args.output/'global_aliases.sci').write_text(''.join(
        f'loc_{0x20000+offset:X},{name}\n' for offset,name,_ in cases) +
        'loc_203A4,rg_overlap_return\nloc_30000,rg_indexed_output\n')
    # SR's local compare/branch optimization must leave ZF available to the
    # second branch. These explicit producer/consumer contracts are upstream's
    # instruction_flags.sci mechanism, not flags-error suppression.
    (args.output/'instruction_flags.sci').write_text(
        'loc_20052,0x8,0 ; first JNZ also carries ZF to the shared block\n'
        'loc_20059,0,0x8 ; shared JZ consumes the incoming ZF\n'
        'loc_200A2,0x38,0 ; JNE carries ZF/SF/OF from CMP to the shared JLE\n'
        'loc_200AA,0,0x38 ; shared JLE consumes the incoming flags\n'
        'loc_20230,0,0x20 ; JO consumes incoming overflow\n'
        'loc_20240,0,0x20 ; JNO consumes incoming overflow\n'
        'loc_20320,0x3f,0x1 ; ADC consumes carry and materializes result flags\n'
        'loc_20340,0x8,0 ; BSF materializes ZF\n'
        'loc_20360,0x3f,0 ; SCASD materializes comparison flags\n'
        'loc_20370,0x3f,0 ; SCASD materializes comparison flags\n')
    result=subprocess.run([str(args.translator.resolve()),'test.exe','test.llasm'],
                          cwd=args.output,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (args.output/'translation.log').write_text(result.stdout)
    if result.returncode:raise AssertionError(result.stdout[-1600:])
    print('PASS synthetic BSWAP, memory, x87, shared-flag branches, signed/unsigned division and reversed x87 destinations translated to LLasm')

if __name__=='__main__':main()
