#!/usr/bin/env python3
"""Assemble the existing System V SR code as an x86-64 Mach-O object.

Expand the already tested ELF/System V macros first, then change only section
directives. NASM adds Darwin's C symbol prefix to globals and externs together.
Generated game code remains in the caller's ignored build directory.
"""
import argparse
from pathlib import Path
import re
import subprocess
import tempfile

def macho_sections(text):
    output=[]
    for line in text.splitlines(True):
        match=re.match(r'\s*\[?section\s+([^\s\]]+)([^\n]*)',line,re.I)
        if not match:
            output.append(line);continue
        name,attributes=match.groups()
        if name=='.note.GNU-stack':continue
        if name=='.text':target='__TEXT,__text'
        elif name.startswith('cseg'):target='__TEXT,'+name
        elif name.startswith(('dseg','useg')):target='__DATA,'+name
        elif name in ['.data','.bss']:target='__DATA,__'+name[1:]
        else:raise ValueError('Unmapped Mach-O section: '+name)
        align=re.search(r'\balign=(\d+)',attributes)
        flags=' code' if target.startswith('__TEXT,') else ' data'
        if 'nobits' in attributes or name=='.bss':flags=' bss'
        output.append('section '+target+flags+' align='+(align[1] if align else '16')+'\n')
    return ''.join(output)

def assemble(source,output,includes,defines):
    command=['nasm','-E','-f','elf64']
    for directory in [source.parent,*includes]:command+=['-I',str(directory)+'/']
    for define in defines:command+=['-D',define]
    expanded=subprocess.check_output([*command,str(source)],text=True,cwd=source.parent)
    with tempfile.NamedTemporaryFile(mode='w',suffix='.asm',dir=output.parent) as temporary:
        temporary.write(macho_sections(expanded));temporary.flush()
        subprocess.run(['nasm','-O1','-f','macho64','--prefix','_',temporary.name,'-o',str(output)],check=True)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path)
    parser.add_argument('output',type=Path)
    parser.add_argument('--include',type=Path,action='append',default=[])
    parser.add_argument('--define',action='append',default=[])
    args=parser.parse_args()
    assemble(args.source.resolve(),args.output.resolve(),[p.resolve() for p in args.include],args.define)

if __name__=='__main__':main()
