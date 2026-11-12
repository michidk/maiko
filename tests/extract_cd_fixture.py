"""Extract one owned ISO root file for disposable original-movie tests.

This development helper follows src/import/cdrom.rs. It is not a runtime
replacement for CD access and extracted game data must never be distributed.
"""
import argparse
from pathlib import Path
import struct

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('image',type=Path)
p.add_argument('name')
p.add_argument('output',type=Path)
a=p.parse_args()
if not a.name or any(c in a.name for c in '/\\:') or a.name in ['.','..']:
    p.error('Expected a root filename')
with a.image.open('rb') as image:
    for sector,offset in [(2048,0),(2352,16)]:
        image.seek(16*sector+offset);pvd=image.read(2048)
        if pvd[:7]==b'\x01CD001\x01':break
    else:raise ValueError('Expected ISO9660 MODE1/2352 or ordinary ISO')
    u32=lambda b,n:struct.unpack_from('<I',b,n)[0]
    if pvd[156]<34 or struct.unpack_from('<H',pvd,128)[0]!=2048:raise ValueError('Invalid primary descriptor')
    root,size=u32(pvd,158),u32(pvd,166)
    if size>16*1024*1024:raise ValueError('Oversized root directory')
    found=[]
    for index in range((size+2047)//2048):
        image.seek((root+index)*sector+offset);block=image.read(2048);position=0
        while position<min(len(block),size-index*2048) and block[position]:
            length=block[position]
            if length<34 or position+length>2048:raise ValueError('Invalid root record')
            record=block[position:position+length];position+=length
            if 33+record[32]>length:raise ValueError('Invalid root name')
            name=record[33:33+record[32]].split(b';')[0].decode('ascii')
            if name.upper()==a.name.upper():
                if record[25]&0x82:raise ValueError('Directory/multiple extents unsupported')
                found.append((u32(record,2),u32(record,10)))
    if len(found)!=1:raise ValueError('Expected exactly one matching root file')
    extent,length=found[0]
    image.seek(0,2)
    if (extent+(length+2047)//2048)*sector>image.tell():raise ValueError('Extent exceeds image')
    a.output.parent.mkdir(parents=True,exist_ok=True)
    with a.output.open('wb') as output:
        # Read bounded groups of sectors to avoid thousands of small host reads.
        for start in range(0,length,2048*128):
            count=min(length-start,2048*128)
            image.seek((extent+start//2048)*sector+offset)
            raw=image.read(((count-1)//2048)*sector+(count-1)%2048+1)
            if len(raw)!=((count-1)//2048)*sector+(count-1)%2048+1:raise ValueError('Truncated payload')
            for i in range(0,count,2048):output.write(raw[(i//2048)*sector:(i//2048)*sector+min(2048,count-i)])
    print(f'{a.name}: {length} bytes from sector {extent}, layout {sector}/{offset}')
