#!/bin/sh
set -eu
cd /work
python3 tests/llasm_opcodes.py /work/SR/SR.exe /fixture
cp SR/llasm-support/*.llinc /fixture/
touch /fixture/extern.llinc
printf 'macro ACTION_CONDITIONAL_JUMP_FORWARD\nendm\n' > /fixture/macros.llinc
cd /fixture
llasm -m64 -inline-idiv -pic -ptrofs -O test.llasm > /results/fixture.ll
opt -O3 /results/fixture.ll -o /results/fixture.bc
llc -O=3 -filetype=obj -mtriple=x86_64-unknown-linux-gnu --relocation-model=pic \
    /results/fixture.bc -o /results/fixture-linux.o
cd /work
clang++ -x c++ -std=c++11 -O2 -DPTROFS_64BIT -I SR/llasm-support -c \
    SR/llasm-support/llasm_float.c -o /results/float-linux.o
clang -Wall -Wextra -Werror -O2 -DPTROFS_64BIT -I SR/llasm-support \
    tests/llasm_execution.c /results/float-linux.o /results/fixture-linux.o \
    /results/libmaiko-llasm-linux.a -lm -ldl -lpthread -o /results/llasm-execution-linux
/results/llasm-execution-linux > /results/linux-execution.log
llc -O=3 -filetype=obj -mtriple=aarch64-unknown-linux-gnu --relocation-model=pic \
    /results/fixture.bc -o /results/fixture-linux-arm64.o
aarch64-linux-gnu-g++ -x c++ -std=c++11 -O2 -DPTROFS_64BIT -I SR/llasm-support -c \
    SR/llasm-support/llasm_float.c -o /results/float-linux-arm64.o
aarch64-linux-gnu-gcc -Wall -Wextra -Werror -O2 -DPTROFS_64BIT -I SR/llasm-support \
    tests/llasm_execution.c /results/float-linux-arm64.o /results/fixture-linux-arm64.o \
    /results/libmaiko-llasm-linux-arm64.a -lm -ldl -lpthread -o /results/llasm-execution-linux-arm64
qemu-aarch64 -L /usr/aarch64-linux-gnu /results/llasm-execution-linux-arm64 \
    > /results/linux-arm64-emulated-execution.log
for arch in x86_64 arm64; do
    llc -O=3 -filetype=obj -mtriple="$arch-apple-darwin" --relocation-model=pic \
        /results/fixture.bc -o "/results/fixture-$arch.o"
    clang++ -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
        -x c++ -stdlib=libc++ -std=c++11 -O2 -DPTROFS_64BIT -I SR/llasm-support -c \
        SR/llasm-support/llasm_float.c -o "/results/float-$arch.o"
    clang -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
        --ld-path=/usr/lib/llvm-14/bin/ld64.lld \
        -Wl,-platform_version,macos,11.0,11.3 -O2 -DPTROFS_64BIT \
        -I SR/llasm-support tests/llasm_execution.c "/results/float-$arch.o" \
        "/results/fixture-$arch.o" "/results/libmaiko-llasm-$arch.a" -o "/results/llasm-execution-$arch"
    clang -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
        --ld-path=/usr/lib/llvm-14/bin/ld64.lld \
        -Wl,-platform_version,macos,11.0,11.3 -Wall -Wextra -Werror -O2 \
        -I runtime -I linux -I macos tests/macos_memory.c "/results/libmaiko-runtime-$arch.a" \
        -Wl,-dead_strip -F/frameworks -framework SDL2 -framework SDL2_mixer \
        -framework Security -framework CoreFoundation -liconv \
        -o "/results/macos-memory-$arch"
    llvm-objdump --macho --private-headers --dylibs-used "/results/llasm-execution-$arch" \
        > "/results/macho-$arch.txt"

done
dpkg-query -W clang lld llvm gdc gcc python3 scons qemu-user gcc-aarch64-linux-gnu \
    > /results/tool-versions.txt
cp /fixture/translation.log /results/translation.log

# Archives are staging inputs; retain their hashes in the report, not copies.
rm /results/libmaiko-llasm-*.a /results/libmaiko-runtime-*.a
