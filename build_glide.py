#!/usr/bin/env python3
"""Build and exercise the pinned native x64 Glide backend without game code."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time
import tempfile
import uuid
from build_windows import ROOT, IMAGE, run, ensure_tool_image
from le_image import Image
from sr_heap import stage_sr_heap
from glide.imports import emit

COMMIT='9f8a2cb73af3b6d26e9ec67bb9e1873d3e211726'

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--install',type=Path,required=True)
    args=parser.parse_args()
    cache=ROOT/'.cache'; source=cache/'openglide-ng'; output=cache/'glide'
    output.mkdir(exist_ok=True)
    if not source.exists():
        run(['git','clone','https://github.com/eugenenuke/openglide-ng',str(source)])
        run(['git','checkout','--detach',COMMIT],cwd=source)
    if subprocess.check_output(['git','rev-parse','HEAD'],cwd=source,text=True).strip()!=COMMIT:
        raise ValueError('Unexpected cached OpenGLide revision')
    # Export the pinned tree rather than trusting edits in the cached checkout.
    with tempfile.TemporaryDirectory(dir=cache) as directory:
        archive=subprocess.check_output(['git','archive',COMMIT],cwd=source)
        subprocess.run(['tar','-x','-C',directory],input=archive,check=True)
        subprocess.run(['patch','-p1','-d',directory],input=(ROOT/'patches/openglide-x64.patch').read_bytes(),check=True)
        archive=subprocess.check_output(['tar','-c','-C',directory,'.'])
    emit(Image(args.install.resolve()/'RGFX.EXE'),output)
    name='rg-glide-'+uuid.uuid4().hex[:10]
    began=time.monotonic()
    report=dict(commit=COMMIT,status='started',native_windows_verified=False,menu_verified=False)
    try:
        report['toolchain']=ensure_tool_image()
        run(['docker','run','-d','--network','none','--memory','4g','--memory-swap','4g','--cpus','2','--tmpfs','/readonly:ro','--name',name,IMAGE,'sleep','360'])
        run(['docker','exec',name,'mkdir','/src'])
        subprocess.run(['docker','exec','-i',name,'tar','-x','-C','/src'],input=archive,check=True)
        for folder in ['runtime','glide']:
            run(['docker','cp',str(ROOT/folder)+'/.',name+':/work/'])
        run(['docker','cp',str(output/'glide_imports.h'),name+':/work/'])
        report['sr_heap']=stage_sr_heap(name)
        commands='''set -e
x86_64-w64-mingw32-g++ -Wall -Wextra -Werror -O2 -I/work -c guest_memory.cpp -o guest_memory.o
sh /work/build_backend.sh
cd /work
WINEDEBUG=-all WINEPREFIX=/tmp/glide-wine LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a /usr/lib/wine/wine64 backend-smoke.exe readonly
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -c input.c
x86_64-w64-mingw32-g++ -std=c++11 -Wall -Wextra -Werror -O2 -I/src -I/work bridge_test.cpp bridge.cpp guest_memory.o input.o -L/work -lredguard-glide -luser32 -static-libgcc -static-libstdc++ -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -o bridge-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/glide-wine LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a /usr/lib/wine/wine64 bridge-test.exe
'''
        with (output/'build.log').open('w') as log:
            run(['docker','exec',name,'timeout','240','sh','-c',commands],stdout=log,stderr=subprocess.STDOUT)
        for file in ['redguard-glide.dll','libredguard-glide.a','libwinpthread-1.dll','backend-smoke.ppm','backend-smoke.exe','bridge-test.exe','OpenGLid.log']:
            run(['docker','cp',name+':/work/'+file,str(output/file)])
        headers=output/'include';headers.mkdir(exist_ok=True)
        for file in source.glob('sdk2_*.h'):
            run(['docker','cp',name+':/src/'+file.name,str(headers/file.name)])
        run(['docker','cp',name+':/src/sdk2_unix.h',str(headers/'sdk2_unix.h')])
        report.update(status='smoke_and_guest_bridge_passed',dll_sha256=hashlib.sha256((output/'redguard-glide.dll').read_bytes()).hexdigest())
    finally:
        report['build_run_seconds']=round(time.monotonic()-began,3)
        (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
        subprocess.run(['docker','rm','-f',name],stdout=subprocess.DEVNULL)
    print(output)

if __name__=='__main__':main()
