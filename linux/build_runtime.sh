#!/bin/sh
set -eu
ulimit -c 0
cd /work
nasm -O1 -f elf64 -I/support/ rgfx.asm -o rgfx.o
nasm -f elf64 -I/support/ native_entry.asm -o entry.o
cc='gcc -Wall -Wextra -Werror -O2 -I/work -I/linux -DRG_USE_SDL -DRG_USE_SDL_AUDIO'
$cc $(pkg-config --cflags sdl2) -c native_main.c cd_image.c /linux/host_os.c /linux/paths.c /linux/user_storage.c /linux/game_files.c /linux/platform_services.c /linux/game_install.c /linux/crash.c input_sdl.c
$cc -I/audio $(pkg-config --cflags SDL2_mixer) -c /audio/audio_sdl.c -o audio_sdl.o
$cc $(pkg-config --cflags SDL2_mixer) -I/audio -c /audio/stream_pcm.c -o stream_pcm.o
$cc -Wno-unused-parameter -I/audio $(pkg-config --cflags SDL2_mixer) -c /audio/Albion-sound.c -o sr_audio.o
g++ -Wall -Wextra -Werror -O2 -I/work -c guest_memory.cpp -o guest_memory.o
g++ -std=c++11 -Wall -Wextra -Werror -O2 -I/work -I/src -c bridge.cpp
g++ -no-pie native_main.o guest_memory.o host_os.o paths.o user_storage.o game_files.o cd_image.o platform_services.o game_install.o crash.o input_sdl.o audio_sdl.o stream_pcm.o sr_audio.o entry.o rgfx.o bridge.o /work/libmaiko_runtime.a -L/work -Wl,-rpath,'$ORIGIN' -lredguard-glide $(pkg-config --libs SDL2_mixer) -lcrypto -pthread -Wl,-Map,linux.map -o redguard-native
nasm -f elf64 -DABI_TEST -I/support/ native_entry.asm -o abi-entry.o
nasm -f elf64 -I/support/ /tests/linux_abi.asm -o abi-test.o
$cc -no-pie /tests/linux_abi.c guest_memory.o abi-entry.o abi-test.o -o linux-abi
./linux-abi
$cc -no-pie /tests/linux_platform.c guest_memory.o /linux/paths.c /linux/user_storage.c /linux/game_files.c /linux/host_os.c cd_image.o /work/libmaiko_runtime.a -o linux-platform
./linux-platform
$cc -no-pie /tests/user_storage.c /linux/paths.c /linux/user_storage.c /linux/game_files.c cd_image.o /work/libmaiko_runtime.a -o user-storage
./user-storage
$cc -no-pie /tests/cd_image.c cd_image.o -o cd-image-test
python3 /tests/check_cd_image.py /work/cd-image-test
$cc /tests/import_interruption.c /linux/paths.c /linux/user_storage.c /work/libmaiko_runtime.a -o import-interruption
./import-interruption
$cc -I/audio $(pkg-config --cflags SDL2_mixer) /tests/audio_samples.c guest_memory.o sr_audio.o -lstdc++ $(pkg-config --libs SDL2_mixer) -o audio-samples
for name in pan loop volume cache; do
    env SDL_AUDIODRIVER=dummy ./audio-samples "$name"
done
$cc -I/audio $(pkg-config --cflags SDL2_mixer) /tests/audio_stream.c audio_sdl.o stream_pcm.o sr_audio.o guest_memory.o /linux/host_os.c /linux/paths.c /linux/user_storage.c -lstdc++ $(pkg-config --libs SDL2_mixer) /work/libmaiko_runtime.a -o audio-stream
env SDL_AUDIODRIVER=dummy ./audio-stream
$cc -I/audio $(pkg-config --cflags sdl2) /tests/stream_pcm.c stream_pcm.o $(pkg-config --libs sdl2) -o stream-pcm
./stream-pcm
$cc /tests/linux_alloc.c guest_memory.o -pthread -o linux-alloc
./linux-alloc
$cc /tests/heap_lifetime.c guest_memory.o -pthread -o heap-lifetime
./heap-lifetime
g++ -std=c++11 -Wall -Wextra -Werror -O2 -I/src $(pkg-config --cflags sdl2) -no-pie backend_smoke.cpp -L/work -Wl,-rpath,'$ORIGIN' -lredguard-glide $(pkg-config --libs sdl2) -o linux-glide-smoke
env LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./linux-glide-smoke
env LIBGL_ALWAYS_SOFTWARE=1 GLIBC_TUNABLES=glibc.malloc.mmap_threshold=0:glibc.malloc.trim_threshold=0:glibc.malloc.top_pad=0 xvfb-run -a ./linux-glide-smoke high
$cc $(pkg-config --cflags sdl2) /tests/sdl_input.c input_sdl.c $(pkg-config --libs sdl2) -o sdl-input-test
env LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./sdl-input-test
sh /linux/check_functions.sh
nm -n redguard-native > linux-symbols.txt
ldd redguard-native > linux-dependencies.txt

$cc -no-pie /tests/installed_movies.c /linux/paths.c /linux/user_storage.c /linux/game_files.c cd_image.o /work/libmaiko_runtime.a -o installed-movies
python3 - <<'PYTEST'
import subprocess,tempfile
with tempfile.TemporaryDirectory() as directory:
    subprocess.run(['/work/installed-movies'],cwd=directory,check=True)
PYTEST
