"""Translate synthetic LE instructions through the actual pinned SR backend."""
import argparse
from pathlib import Path
import struct
import subprocess

def le_fixture(code):
    data=bytearray(512+4096)
    data[:2]=b'MZ';struct.pack_into('<H',data,0x18,0x40);struct.pack_into('<I',data,0x3c,0x40)
    data[0x40:0x44]=b'LE\0\0'
    for offset,value in {20:1,24:1,32:2,36:4096,40:4096,44:4096,
                         64:192,68:2,72:240,104:244,108:252,128:512}.items():
        struct.pack_into('<I',data,0x40+offset,value)
    struct.pack_into('<6I',data,0x40+192,4096,0x20000,0x2005,1,1,0)
    struct.pack_into('<6I',data,0x40+216,4096,0x30000,0x2003,2,0,0)
    data[0x40+240:0x40+244]=b'\0\0\1\0'
    data[512:512+len(code)]=code
    return data

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('translator',type=Path)
    parser.add_argument('output',type=Path)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    # BSWAP does not change flags. Retain a distinctive caller flag word.
    code=bytearray(b'\x90'*640)
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
    for offset,name,machine_code in cases:
        machine_code=bytes.fromhex(machine_code)
        code[offset:offset+len(machine_code)]=machine_code
    (args.output/'test.exe').write_bytes(le_fixture(code))
    (args.output/'global_aliases.sci').write_text(''.join(
        f'loc_{0x20000+offset:X},{name}\n' for offset,name,_ in cases))
    # SR's local compare/branch optimization must leave ZF available to the
    # second branch. These explicit producer/consumer contracts are upstream's
    # instruction_flags.sci mechanism, not flags-error suppression.
    (args.output/'instruction_flags.sci').write_text(
        'loc_20052,0x8,0 ; first JNZ also carries ZF to the shared block\n'
        'loc_20059,0,0x8 ; shared JZ consumes the incoming ZF\n'
        'loc_200A2,0x38,0 ; JNE carries ZF/SF/OF from CMP to the shared JLE\n'
        'loc_200AA,0,0x38 ; shared JLE consumes the incoming flags\n'
        'loc_20230,0,0x20 ; JO consumes incoming overflow\n'
        'loc_20240,0,0x20 ; JNO consumes incoming overflow\n')
    result=subprocess.run([str(args.translator.resolve()),'test.exe','test.llasm'],
                          cwd=args.output,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (args.output/'translation.log').write_text(result.stdout)
    if result.returncode:raise AssertionError(result.stdout[-1600:])
    print('PASS synthetic BSWAP, memory, x87, shared-flag branches, signed/unsigned division and reversed x87 destinations translated to LLasm')

if __name__=='__main__':main()
