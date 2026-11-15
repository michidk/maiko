#!/usr/bin/env python3
"""Build the game-free Windows or Linux link inputs for a runtime kit.

Writes OUTPUT/link (start files, system libraries, the Rust host and the SR heap and audio
objects that the patcher's LLD links with the user's engine objects) and OUTPUT/runtime (the
freshly built launcher). Nothing here reads RGFX.EXE or any other game file.
"""
import argparse
import json
from pathlib import Path
import subprocess
import uuid

from .toolchain import ROOT, IMAGE, run, ensure_tool_image
from .sr_heap import stage_sr_heap
from .windows_sdl import stage_windows_sdl
from .rust_runtime import build as build_rust_runtime, stage as stage_rust_runtime, CRATE_IMPORT_LIBRARIES, crate_import_libraries

LINUX_IMAGE = 'redguard-recomp-linux:bookworm'

# Mirrors crates/patcher/src/link.rs. LLD's MinGW driver picks the first match per directory in this order.
WINDOWS_LIBRARIES = sorted(set(
    'SDL2 stdc++ ws2_32 userenv ntdll advapi32 dbghelp gcc_eh winpthread '
    'user32 shell32 ole32 mingw32 gcc moldname mingwex msvcrt kernel32 pthread'.split() + CRATE_IMPORT_LIBRARIES))
WINDOWS = f'''set -e
cd /work
x86_64-w64-mingw32-g++ -Wall -Wextra -Werror -O2 -I/work -x c++ -c sr_Game_memory.c -o guest_memory.obj
mkdir -p /out/link /out/runtime
cp libmaiko_host.a guest_memory.obj /out/link/
gcc=/usr/lib/gcc/x86_64-w64-mingw32/12-posix; mingw=/usr/x86_64-w64-mingw32/lib
cp $mingw/crt2.o $gcc/crtbegin.o $gcc/crtend.o /out/link/
resolve() {{
  for directory in /sdl/lib /work $gcc $mingw; do
    for file in lib$1.dll.a $1.dll.a lib$1.a $1.lib; do
      if [ -f $directory/$file ]; then cp -L $directory/$file /out/link/; return; fi
    done
  done
  echo "No library for -l$1" >&2; exit 1
}}
for library in {' '.join(WINDOWS_LIBRARIES)}; do resolve $library; done
echo '1 ICON "maiko.ico"' > maiko.rc
x86_64-w64-mingw32-windres maiko.rc -O coff -o maiko-icon.o
x86_64-w64-mingw32-gcc -O2 -s -mwindows -o /out/runtime/maiko.exe launcher_main.c maiko-icon.o libmaiko_launcher.a -L/work \\
  -lshell32 -lole32 -luser32 -lgdi32 -ladvapi32 -lopengl32 {' '.join('-l' + name for name in CRATE_IMPORT_LIBRARIES)} \\
  -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp
'''
LINUX = '''set -e
cd /work
g++ -Wall -Wextra -Werror -O2 -I/work -x c++ -c sr_Game_memory.c -o guest_memory.o
mkdir -p /out/link /out/runtime
cp libmaiko_host.a guest_memory.o /out/link/
for file in /usr/lib/x86_64-linux-gnu/crt1.o /usr/lib/x86_64-linux-gnu/crti.o /usr/lib/x86_64-linux-gnu/crtn.o \\
    /usr/lib/gcc/x86_64-linux-gnu/12/crtbegin.o /usr/lib/gcc/x86_64-linux-gnu/12/crtend.o /usr/lib/gcc/x86_64-linux-gnu/12/libgcc.a \\
    /lib/x86_64-linux-gnu/libgcc_s.so.1 /lib/x86_64-linux-gnu/libc.so.6 /usr/lib/x86_64-linux-gnu/libc_nonshared.a \\
    /lib64/ld-linux-x86-64.so.2 /lib/x86_64-linux-gnu/libm.so.6 /lib/x86_64-linux-gnu/libstdc++.so.6 \\
    /usr/lib/x86_64-linux-gnu/libasound.so.2; do
  cp -L $file /out/link/
done
gcc -O2 -s -o /out/runtime/maiko launcher_main.c libmaiko_launcher.a \\
  -Wl,--as-needed -lasound -lgcc_s -lutil -lrt -lpthread -lm -ldl
'''

# The launcher is a Rust static library (maiko-launcher); this is the executable's whole C side.
LAUNCHER_MAIN = b'int maiko_launcher_main(void);\nint main(void) { return maiko_launcher_main(); }\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', choices=['windows', 'linux'], required=True)
    parser.add_argument('--output', type=Path, required=True, help='New folder for link/ and runtime/')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('--output must not exist')
    windows = args.target == 'windows'
    ensure_tool_image()
    if windows:
        image = IMAGE
    else:
        image = LINUX_IMAGE
        run(['docker', 'build', '-t', image, '-f', str(ROOT / 'platforms/linux/Dockerfile'), str(ROOT / 'platforms/linux')])
    rust_target = 'x86_64-pc-windows-gnu' if windows else 'x86_64-unknown-linux-gnu'
    host = build_rust_runtime(rust_target, component='host')
    launcher, _ = build_rust_runtime(rust_target, component='launcher')
    name = 'rg-link-' + uuid.uuid4().hex[:10]
    run(['docker', 'run', '-d', '--network', 'none', '--memory', '4g', '--cpus', '2', '--name', name, image, 'sleep', '1800'])
    try:
        for directory, destination in [('crates/runtime', '/work'), ('platforms/linux', '/linux')]:
            run(['docker', 'cp', str(ROOT / directory) + '/.', name + ':' + destination])
        stage_rust_runtime(name, host)
        run(['docker', 'cp', str(launcher), name + ':/work/libmaiko_launcher.a'])
        subprocess.run(['docker', 'exec', '-i', name, 'sh', '-c', 'cat > /work/launcher_main.c'], check=True,
                       input=LAUNCHER_MAIN)
        stage_sr_heap(name)
        if windows:
            stage_windows_sdl(name)
            # The launcher's crates link import libraries that their -sys crates ship for MinGW.
            for library in crate_import_libraries():
                run(['docker', 'cp', str(library), name + ':/work/'])
            run(['docker', 'cp', str(ROOT / 'platforms/windows/maiko.ico'), name + ':/work/maiko.ico'])
        run(['docker', 'exec', name, 'timeout', '1200', 'sh', '-c', WINDOWS if windows else LINUX])
        args.output.mkdir(parents=True)
        run(['docker', 'cp', name + ':/out/.', str(args.output)])
    finally:
        subprocess.run(['docker', 'rm', '-f', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(args.output)


if __name__ == '__main__':
    main()
