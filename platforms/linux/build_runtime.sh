#!/bin/sh
set -eu
ulimit -c 0
cd /work
nasm -O1 -f elf64 -I/support/ rgfx.asm -o rgfx.o
nasm -f elf64 -I/support/ native_entry.asm -o entry.o
cc='gcc -Wall -Wextra -Werror -O2 -I/work -I/linux'
# System libraries the Rust runtime's crates link: ALSA (cpal) and udev (gilrs).
native_libs='-lasound -ludev'
nasm -f elf64 main.asm -o main.o
nasm -f elf64 service.asm -o service.o
# Every runtime consumer gets its native dependencies after the Rust archive.
link() {
    $cc "$@" -Wl,--gc-sections \
      ${rust_library:-/work/libmaiko_runtime.a} \
      $native_libs -ldl -pthread -lm
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
sh /linux/check_functions.sh
nm -n redguard-native > linux-symbols.txt
ldd redguard-native > linux-dependencies.txt
link -no-pie /tests/installed_movies.c -o installed-movies
python3 - <<'PYTEST'
import subprocess,tempfile
with tempfile.TemporaryDirectory() as directory:
    subprocess.run(['/work/installed-movies'],cwd=directory,check=True)
PYTEST
