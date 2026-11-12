#!/bin/sh
set -eu
cd /work
cc='gcc -Wall -Wextra -Werror -O2 -I/work -I/linux -DRG_USE_SDL -DRG_USE_SDL_AUDIO'
$cc -Dmain=rg_unused_main -c native_main.c -o test_host.o
objects='test_host.o guest_memory.o host_os.o paths.o user_storage.o game_files.o movie_files.o cd_image.o platform_services.o timing_sdl.o game_install.o crash.o input_sdl.o audio_sdl.o stream_pcm.o sr_audio.o entry.o rgfx.o bridge.o'
for name in clip numeric heap directory input rob_vertex file_read animation_flags attribute animation_advance critical movie_clock audio; do
    $cc $(pkg-config --cflags SDL2_mixer) /tests/original_$name.c $objects -no-pie -L/work -Wl,-rpath,'$ORIGIN' -lredguard-glide $(pkg-config --libs SDL2_mixer) -lcrypto -lstdc++ -pthread -lm -o original-$name
    if [ "$name" = input ]; then
        env LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./original-$name
    else
        env SDL_AUDIODRIVER=dummy ./original-$name
    fi
done
$cc $(pkg-config --cflags sdl2) /tests/native_timing.c timing_sdl.c $(pkg-config --libs sdl2) -pthread -o linux-timing
./linux-timing
$cc /tests/linux_fault.c $objects -no-pie -L/work -Wl,-rpath,'$ORIGIN' -lredguard-glide $(pkg-config --libs SDL2_mixer) -lcrypto -lstdc++ -pthread -o linux-fault
python3 - <<'PY'
import subprocess
for args in [[],['--worker']]:
    result=subprocess.run(['./linux-fault']+args,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=10)
    print(result.stdout)
    assert result.returncode==3
    assert result.stdout.count('BLOCKED Linux signal=0000000000000004')==1
print('PASS Linux original UD2 diagnostic and exit3 on main and worker threads')
PY
