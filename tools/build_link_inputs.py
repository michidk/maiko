#!/usr/bin/env python3
"""Build the game-free Windows or Linux link inputs for a runtime kit.

Writes OUTPUT/link (start files, system libraries, the Rust host and the SR heap and audio
objects that the patcher's LLD links with the user's engine objects) and OUTPUT/runtime (the
freshly built renderer). Nothing here reads RGFX.EXE or any other game file.
"""
import argparse
from pathlib import Path
import subprocess
import tempfile
import uuid

from .toolchain import ROOT, IMAGE, run, ensure_tool_image
from .sr_heap import stage_sr_heap
from .sr_audio import stage_sr_audio
from .windows_sdl import stage_windows_sdl
from .rust_runtime import build as build_rust_runtime, stage as stage_rust_runtime

LINUX_IMAGE = 'redguard-recomp-linux:bookworm'
OPENGLIDE = '9f8a2cb73af3b6d26e9ec67bb9e1873d3e211726'

# Mirrors patcher/src/link.rs. LLD's MinGW driver picks the first match per directory in this order.
WINDOWS_LIBRARIES = sorted(set(
    'SDL2_mixer SDL2 redguard-glide stdc++ ws2_32 userenv ntdll advapi32 dbghelp gcc_eh winpthread '
    'user32 shell32 ole32 mingw32 gcc moldname mingwex msvcrt kernel32 pthread'.split()))
WINDOWS = f'''set -e
cd /work
x86_64-w64-mingw32-g++ -Wall -Wextra -Werror -O2 -I/work -x c++ -c sr_Game_memory.c -o guest_memory.obj
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work -I/audio -I/sdl/include/SDL2 -DRG_USE_SDL_AUDIO -DSDL_MAIN_HANDLED \\
  -Wno-unused-parameter -c /audio/Albion-sound.c -o sr_audio.obj
mkdir -p /out/link /out/runtime
cp libmaiko_host.a guest_memory.obj sr_audio.obj /out/link/
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
cp redguard-glide.dll /out/runtime/
'''
LINUX = '''set -e
cd /work
sh /linux/build_backend.sh
g++ -Wall -Wextra -Werror -O2 -I/work -x c++ -c sr_Game_memory.c -o guest_memory.o
gcc -Wall -Wextra -Werror -O2 -I/work -I/linux -I/audio -DRG_USE_SDL -DRG_USE_SDL_AUDIO $(pkg-config --cflags SDL2_mixer) \\
  -Wno-unused-parameter -c /audio/Albion-sound.c -o sr_audio.o
mkdir -p /out/link /out/runtime
cp libmaiko_host.a guest_memory.o sr_audio.o /out/link/
for file in /usr/lib/x86_64-linux-gnu/crt1.o /usr/lib/x86_64-linux-gnu/crti.o /usr/lib/x86_64-linux-gnu/crtn.o \\
    /usr/lib/gcc/x86_64-linux-gnu/12/crtbegin.o /usr/lib/gcc/x86_64-linux-gnu/12/crtend.o /usr/lib/gcc/x86_64-linux-gnu/12/libgcc.a \\
    /lib/x86_64-linux-gnu/libgcc_s.so.1 /lib/x86_64-linux-gnu/libc.so.6 /usr/lib/x86_64-linux-gnu/libc_nonshared.a \\
    /lib64/ld-linux-x86-64.so.2 /lib/x86_64-linux-gnu/libm.so.6 /lib/x86_64-linux-gnu/libstdc++.so.6; do
  cp -L $file /out/link/
done
cp libredguard-glide.so /out/runtime/
'''


def openglide(patches):
    """The pinned OpenGLide tree with Maiko's patches, as a tar stream."""
    source = ROOT / '.cache/openglide-ng'
    if not source.exists():
        run(['git', 'clone', 'https://github.com/eugenenuke/openglide-ng', str(source)])
    archive = subprocess.check_output(['git', 'archive', OPENGLIDE], cwd=source)
    with tempfile.TemporaryDirectory(dir=ROOT / '.cache') as directory:
        subprocess.run(['tar', '-x', '-C', directory], input=archive, check=True)
        for patch in patches:
            subprocess.run(['patch', '-s', '-p1', '-d', directory], input=(ROOT / 'patches' / patch).read_bytes(), check=True)
        return subprocess.check_output(['tar', '-c', '-C', directory, '.'])


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
        # The renderer DLL and its import library come from the tested Glide build.
        run(['python3', '-m', 'tools.build_glide'], cwd=ROOT)
    else:
        image = LINUX_IMAGE
        run(['docker', 'build', '-t', image, '-f', str(ROOT / 'platforms/linux/Dockerfile'), str(ROOT / 'platforms/linux')])
    host = build_rust_runtime('x86_64-pc-windows-gnu' if windows else 'x86_64-unknown-linux-gnu', component='host')
    name = 'rg-link-' + uuid.uuid4().hex[:10]
    run(['docker', 'run', '-d', '--network', 'none', '--memory', '4g', '--cpus', '2', '--name', name, image, 'sleep', '1800'])
    try:
        for directory, destination in [('runtime', '/work'), ('glide', '/work'), ('platforms/linux', '/linux')]:
            run(['docker', 'cp', str(ROOT / directory) + '/.', name + ':' + destination])
        stage_rust_runtime(name, host)
        stage_sr_heap(name)
        stage_sr_audio(name)
        if windows:
            stage_windows_sdl(name)
            for file in ['redguard-glide.dll', 'libredguard-glide.a']:
                run(['docker', 'cp', str(ROOT / '.cache/glide' / file), name + ':/work/' + file])
        else:
            run(['docker', 'exec', name, 'mkdir', '/src'])
            subprocess.run(['docker', 'exec', '-i', name, 'tar', '-x', '-C', '/src'],
                           input=openglide(['openglide-x64.patch', 'openglide-sdl-window.patch']), check=True)
        run(['docker', 'exec', name, 'timeout', '1200', 'sh', '-c', WINDOWS if windows else LINUX])
        args.output.mkdir(parents=True)
        run(['docker', 'cp', name + ':/out/.', str(args.output)])
    finally:
        subprocess.run(['docker', 'rm', '-f', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(args.output)


if __name__ == '__main__':
    main()
