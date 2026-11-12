#!/bin/sh
set -eu
ulimit -c 0
cd /work
nasm -O1 -f elf64 -I/support/ rgfx.asm -o rgfx.o
nasm -f elf64 -I/support/ native_entry.asm -o entry.o
cc='gcc -Wall -Wextra -Werror -O2 -I/work -I/linux -DRG_USE_SDL -DRG_USE_SDL_AUDIO'
nasm -f elf64 main.asm -o main.o
nasm -f elf64 service.asm -o service.o
$cc -Wno-unused-parameter -I/audio $(pkg-config --cflags SDL2_mixer) -c /audio/Albion-sound.c -o sr_audio.o
g++ -Wall -Wextra -Werror -O2 -I/work -x c++ -c sr_Game_memory.c -o guest_memory.o
# Every runtime consumer gets its native dependencies after the Rust archive.
# Group SR and Rust because their allocator/sample ABI references are mutual.
link() {
    $cc -I/audio $(pkg-config --cflags SDL2_mixer) "$@" -Wl,--gc-sections \
      -Wl,--start-group ${rust_library:-/work/libmaiko_runtime.a} guest_memory.o sr_audio.o -Wl,--end-group \
      -L/work -Wl,-rpath,'$ORIGIN' -lredguard-glide -lstdc++ $(pkg-config --libs SDL2_mixer) -ldl -pthread -lm
}
rust_library=/work/libmaiko_host.a link -no-pie main.o service.o entry.o rgfx.o -L/work -Wl,-rpath,'$ORIGIN' -lredguard-glide -Wl,-Map,linux.map -o redguard-native
nasm -f elf64 -DABI_TEST -I/support/ native_entry.asm -o abi-entry.o
nasm -f elf64 -I/support/ /tests/linux_abi.asm -o abi-test.o
link -no-pie /tests/linux_abi.c abi-entry.o abi-test.o -o linux-abi
./linux-abi
link -no-pie /tests/linux_platform.c -o linux-platform
./linux-platform
link -no-pie /tests/user_storage.c -o user-storage
./user-storage
link -no-pie /tests/cd_image.c -o cd-image-test
python3 /tests/check_cd_image.py /work/cd-image-test
link /tests/import_interruption.c -o import-interruption
./import-interruption
link /tests/audio_samples.c -o audio-samples
for name in pan loop volume cache; do
    env SDL_AUDIODRIVER=dummy ./audio-samples "$name"
done
link /tests/audio_stream.c -o audio-stream
env SDL_AUDIODRIVER=dummy ./audio-stream
link /tests/stream_pcm.c -o stream-pcm
./stream-pcm
link /tests/linux_alloc.c -o linux-alloc
./linux-alloc
link /tests/heap_lifetime.c -o heap-lifetime
./heap-lifetime
g++ -std=c++11 -Wall -Wextra -Werror -O2 -I/src $(pkg-config --cflags sdl2) -no-pie /tests/backend_smoke.cpp -L/work -Wl,-rpath,'$ORIGIN' -lredguard-glide $(pkg-config --libs sdl2) -o linux-glide-smoke
env LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./linux-glide-smoke
env LIBGL_ALWAYS_SOFTWARE=1 GLIBC_TUNABLES=glibc.malloc.mmap_threshold=0:glibc.malloc.trim_threshold=0:glibc.malloc.top_pad=0 xvfb-run -a ./linux-glide-smoke high
link /tests/sdl_input.c -o sdl-input-test
env LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./sdl-input-test
sh /linux/check_functions.sh
nm -n redguard-native > linux-symbols.txt
ldd redguard-native > linux-dependencies.txt
link -no-pie /tests/installed_movies.c -o installed-movies
python3 - <<'PYTEST'
import subprocess,tempfile
with tempfile.TemporaryDirectory() as directory:
    subprocess.run(['/work/installed-movies'],cwd=directory,check=True)
PYTEST
