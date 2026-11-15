#!/bin/sh
set -eu
ulimit -c 0
cd /work
nasm -O1 -f elf64 -I/support/ rgfx.asm -o rgfx.o
nasm -f elf64 -I/support/ native_entry.asm -o entry.o
cc='gcc -Wall -Wextra -Werror -O2 -I/work -I/linux -DRG_USE_SDL'
nasm -f elf64 main.asm -o main.o
nasm -f elf64 service.asm -o service.o
g++ -Wall -Wextra -Werror -O2 -I/work -x c++ -c sr_Game_memory.c -o guest_memory.o
# Every runtime consumer gets its native dependencies after the Rust archive.
# Group SR's allocator and Rust because their references are mutual.
link() {
    $cc $(pkg-config --cflags sdl2) "$@" -Wl,--gc-sections \
      -Wl,--start-group ${rust_library:-/work/libmaiko_runtime.a} guest_memory.o -Wl,--end-group \
      -lstdc++ $(pkg-config --libs sdl2) -lasound -ldl -pthread -lm
}
rust_library=/work/libmaiko_host.a link -no-pie main.o service.o entry.o rgfx.o -Wl,-Map,linux.map -o redguard-native
nasm -f elf64 -DABI_TEST -I/support/ native_entry.asm -o abi-entry.o
nasm -f elf64 -I/support/ /tests/linux_abi.asm -o abi-test.o
link -no-pie /tests/linux_abi.c abi-entry.o abi-test.o -o linux-abi
./linux-abi
link -no-pie /tests/linux_platform.c -o linux-platform
./linux-platform
link -no-pie /tests/cd_image.c -o cd-image-test
python3 /tests/check_cd_image.py /work/cd-image-test
# Without a sound device the output mixes silently in real time, so the stream still drains.
link /tests/audio_stream.c -o audio-stream
./audio-stream
link /tests/linux_alloc.c -o linux-alloc
./linux-alloc
link /tests/heap_lifetime.c -o heap-lifetime
./heap-lifetime
link /tests/sdl_input.c -o sdl-input-test
xvfb-run -a ./sdl-input-test
sh /linux/check_functions.sh
nm -n redguard-native > linux-symbols.txt
ldd redguard-native > linux-dependencies.txt
link -no-pie /tests/installed_movies.c -o installed-movies
python3 - <<'PYTEST'
import subprocess,tempfile
with tempfile.TemporaryDirectory() as directory:
    subprocess.run(['/work/installed-movies'],cwd=directory,check=True)
PYTEST
