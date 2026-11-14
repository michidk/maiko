#!/bin/sh
set -eu
cd /work
cc='gcc -Wall -Wextra -Werror -O2 -I/work -I/linux -DRG_USE_SDL -DRG_USE_SDL_AUDIO'
objects='service.o entry.o rgfx.o -Wl,--start-group /work/libmaiko_host.a guest_memory.o sr_audio.o -Wl,--end-group'
for name in clip numeric heap directory input rob_vertex file_read animation_flags attribute animation_advance critical movie_clock audio close; do
    $cc $(pkg-config --cflags SDL2_mixer) /tests/original_$name.c $objects -Wl,--gc-sections -no-pie -L/work -Wl,-rpath,'$ORIGIN' -lredguard-glide $(pkg-config --libs SDL2_mixer) -lstdc++ -pthread -lm -o original-$name
    if [ "$name" = file_read ] && [ -n "${MAIKO_LINK_ONLY:-}" ]; then
        : # reads SYSTEM.INI from a real game folder; release kit builds only have RGFX.EXE
    elif [ "$name" = input ]; then
        env LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./original-$name
    elif [ "$name" = close ]; then
        env LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a python3 /tests/check_original_shutdown.py
    else
        env SDL_AUDIODRIVER=dummy ./original-$name
    fi
done
$cc $(pkg-config --cflags sdl2) /tests/native_timing.c /work/libmaiko_runtime.a -Wl,--gc-sections $(pkg-config --libs sdl2) -pthread -lm -o linux-timing
./linux-timing
$cc /tests/linux_fault.c $objects -Wl,--gc-sections -no-pie -L/work -Wl,-rpath,'$ORIGIN' -lredguard-glide $(pkg-config --libs SDL2_mixer) -lstdc++ -pthread -lm -o linux-fault
python3 - <<'PY'
import subprocess
for args in [[],['--worker']]:
    result=subprocess.run(['./linux-fault']+args,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=10)
    print(result.stdout)
    assert result.returncode==3
    assert result.stdout.count('BLOCKED Linux signal=0000000000000004')==1
print('PASS Linux original UD2 diagnostic and exit3 on main and worker threads')
PY
