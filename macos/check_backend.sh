#!/bin/sh
set -eu
cd /work
mkdir -p /sdk /frameworks /fixture /results
tar -xJf MacOSX11.3.sdk.tar.xz -C /sdk
7z x -y -o/tmp/sdl SDL2-2.30.11.dmg > /results/sdl-extract.log
7z x -y -o/tmp/mixer SDL2_mixer-2.8.1.dmg > /results/mixer-extract.log
cp -a /tmp/sdl/SDL2/SDL2.framework /frameworks/
cp -a /tmp/mixer/SDL2_mixer/SDL2_mixer.framework /frameworks/
rm -rf /tmp/sdl /tmp/mixer
tar -xf sr.tar
patch -p1 < sr-x64.patch
patch -p1 < sr-llasm.patch
sed -i -E 's/^#define OUTPUT_TYPE[[:space:]]+OUT_X86$/#define OUTPUT_TYPE OUT_LLASM/' SR/SR_defs.h
grep -q '^#define OUTPUT_TYPE OUT_LLASM$' SR/SR_defs.h
scons -C SR -j2 > /results/sr-build.log 2>&1
gdc -O2 -Wall llasm/llasm.d -o /usr/local/bin/llasm
sh /work/macos/compile_tests.sh
mkdir -p /src
tar -xf /work/openglide.tar -C /src
cd /src
patch -p1 < /work/openglide-x64.patch
patch -p1 < /work/openglide-sdl-window.patch
patch -p1 < /work/openglide-macos.patch
sh /work/macos/build_renderer.sh

if [ -f /game/rgfx.llasm ]; then
    # Real game input is emitted from an owned, hash-checked installation.
    # Keep the SDK and all generated engine code out of the source tree.
    cp /work/SR/llasm-support/*.llinc /game/
    cd /game
    # The original-game optimizer needs more memory than the synthetic test.
    # A separate timeout covers the consumer of the streaming IR as well.
    timeout 240 llasm -m64 -inline-idiv -pic -ptrofs -O rgfx.llasm > /results/game.ll
    timeout 600 opt -O2 /results/game.ll -o /results/game.bc
    rm /results/game.ll
    # Keep the same engine available for original-function and game probes on
    # Linux; those observations remain separate from Darwin execution.
    timeout 240 llc -O=2 -filetype=obj -mtriple=x86_64-unknown-linux-gnu \
        --relocation-model=pic /results/game.bc -o /results/engine-linux.o
    for arch in x86_64 arm64; do
        directory="/results/game-$arch"
        mkdir -p "$directory"
        timeout 240 llc -O=2 -filetype=obj -mtriple="$arch-apple-darwin" \
            --relocation-model=pic /results/game.bc -o "$directory/engine.o"
        for source in /work/SR/llasm-support/*.c; do
            # The shared Rust dispatcher supplies the CPU-state INT boundary.
            [ "$(basename "$source")" = llasm_int.c ] && continue
            clang++ -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
                -x c++ -stdlib=libc++ -std=c++11 -O2 -DPTROFS_64BIT \
                -I/work/SR/llasm-support -c "$source" \
                -o "$directory/$(basename "$source" .c).o"
        done
        clang++ -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
            -x c++ -stdlib=libc++ -std=c++11 -O2 -DPTROFS_64BIT \
            -I/work -c /work/sr_Game_memory.c -o "$directory/heap.o"
        clang -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
            -O2 -I/work/runtime -I/audio -F/frameworks \
            -I/frameworks/SDL2.framework/Headers -I/frameworks/SDL2_mixer.framework/Headers \
            -c /audio/Albion-sound.c -o "$directory/audio.o"
        clang++ -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
            -stdlib=libc++ --ld-path=/usr/lib/llvm-14/bin/ld64.lld \
            -Wl,-platform_version,macos,11.0,11.3 -Wl,-dead_strip \
            -Wl,-rpath,@executable_path/Frameworks -Wl,-rpath,@executable_path \
            "$directory"/*.o "/results/libmaiko-host-$arch.a" \
            "/results/libredguard-glide-$arch.dylib" -F/frameworks \
            -framework SDL2 -framework SDL2_mixer -framework Security \
            -framework CoreFoundation -liconv -lresolv \
            -o "/results/redguard-native-$arch"
        llvm-objdump --macho --private-headers --dylibs-used "/results/redguard-native-$arch" \
            > "/results/game-$arch.txt"
        rm -r "$directory"
    done
    mkdir -p /results/runtime/Frameworks
    cp /results/redguard-native-* /results/runtime/
    llvm-lipo-14 -create /results/libredguard-glide-x86_64.dylib \
        /results/libredguard-glide-arm64.dylib \
        -output /results/runtime/libredguard-glide.dylib
    cp -a /frameworks/SDL2.framework /frameworks/SDL2_mixer.framework \
        /results/runtime/Frameworks/
    cp /work/macos/RUN.command /results/runtime/
    chmod +x /results/runtime/RUN.command
    rm /results/libmaiko-host-*.a /results/game.bc
fi
