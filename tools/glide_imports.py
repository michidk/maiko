"""Validate Redguard's five-byte Glide stubs and generate native ABI metadata."""
import re
import struct


def inventory(image):
    result=[]
    for index in range(124):
        address=0x13d988+index*5
        code=image.at(address,5)
        if code[0]!=0xe8 or address+5+struct.unpack('<i',code[1:])[0]!=0x128c60:
            raise ValueError(f'Glide stub changed at {address:#x}')
        pointer=struct.unpack('<I',image.at(0x13dc14+4*index,4))[0]+0x20000
        decorated=image.at(pointer,128).split(b'\0',1)[0].decode('ascii')
        match=re.fullmatch(r'_([A-Z0-9]+)@(\d+)',decorated)
        if not match or int(match[2])%4 or int(match[2])>64:
            raise ValueError(f'Invalid Glide ABI name: {decorated}')
        result.append(dict(address=address,name=match[1],bytes=int(match[2]),index=index))
    return result


def emit(image,work,*,write_mappings=True):
    imports=inventory(image)
    if write_mappings:
        with (work/'external_procedures.sci').open('a') as out:
            out.writelines(f'loc_{i["address"]:X},host_glide_{i["index"]}\n' for i in imports)
    with (work/'extern.inc').open('a') as out:
        out.writelines(f'extern host_glide_{i["index"]}\n' for i in imports)
    (work/'glide_services.inc').write_text(''.join(
        f'HOST_SERVICE host_glide_{i["index"]}, {0x2000+i["index"]}, {i["bytes"]}\n' for i in imports))
    (work/'glide_imports.h').write_text(''.join(
        f'#define RG_{i["name"]} {i["index"]}\n' for i in imports))
    return imports
