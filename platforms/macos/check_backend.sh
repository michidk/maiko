#!/bin/sh
set -eu
cd /work
mkdir -p /sdk /fixture /results
tar -xJf MacOSX11.3.sdk.tar.xz -C /sdk
tar -xf sr.tar
patch -p1 < sr-x64.patch
patch -p1 < sr-llasm.patch
sed -i -E 's/^#define OUTPUT_TYPE[[:space:]]+OUT_X86$/#define OUTPUT_TYPE OUT_LLASM/' SR/SR_defs.h
grep -q '^#define OUTPUT_TYPE OUT_LLASM$' SR/SR_defs.h
scons -C SR -j2 > /results/sr-build.log 2>&1
gdc -O2 -Wall llasm/llasm.d -o /usr/local/bin/llasm
sh /work/platforms/macos/compile_tests.sh

# Kit mode builds the game-free link inputs; a translation adds the real engine.
if [ -f /game/rgfx.llasm ] || [ -n "${MAIKO_KIT:-}" ]; then
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
fi
    for arch in x86_64 arm64; do
        directory="/results/game-$arch"
        mkdir -p "$directory"
        [ -f /results/game.bc ] && timeout 240 llc -O=2 -filetype=obj -mtriple="$arch-apple-darwin" \
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
        mac_link() {
            clang++ -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
                -stdlib=libc++ --ld-path=/usr/lib/llvm-14/bin/ld64.lld \
                -Wl,-platform_version,macos,11.0,11.3 \
                -Wl,-rpath,@executable_path \
                "$directory"/*.o "$@" \
                -framework Security -framework CoreAudio -framework AudioToolbox -framework AppKit -framework ApplicationServices -framework Carbon -framework CoreVideo -framework IOKit \
                -framework CoreFoundation -liconv -lresolv \
                -framework Metal -framework QuartzCore -framework CoreGraphics -framework Foundation -lobjc \
                -o "/results/redguard-native-$arch"
        }
        if [ -f "$directory/engine.o" ]; then
            mac_link -Wl,-dead_strip "/results/libmaiko-host-$arch.a"
        else
            # Without the engine, a probe keeps the whole host so its system imports cover
            # everything the browser's link can reference. The engine's own symbols are
            # replaced by empty definitions; the probe is never run or packaged.
            host="-Wl,-force_load,/results/libmaiko-host-$arch.a"
            if ! mac_link $host -Wl,--error-limit=0 2> "/results/probe-$arch.log"; then
                sed -n 's/.*undefined symbol: //p' "/results/probe-$arch.log" | sort -u |
                    awk '{ printf ".globl %s\n%s:\n    ret\n", $1, $1 }' > /tmp/engine-stub.s
                [ -s /tmp/engine-stub.s ] || { cat "/results/probe-$arch.log"; exit 1; }
                clang -target "$arch-apple-macos11" -c /tmp/engine-stub.s -o /tmp/engine-stub-$arch.o
                mac_link $host /tmp/engine-stub-$arch.o
            fi
        fi
        llvm-objdump --macho --private-headers --dylibs-used "/results/redguard-native-$arch" \
            > "/results/game-$arch.txt"
        # Retain only source-free linker inputs for the browser kit exporter.
        # The translated engine is rebuilt from the visitor's owned executable.
        rm -f "$directory/engine.o"
    done
    mkdir -p /results/runtime
    llvm-lipo-14 -create /results/redguard-native-x86_64 /results/redguard-native-arm64 \
        -output /results/runtime/redguard-native
    chmod +x /results/runtime/redguard-native
    # The launcher (maiko-launcher) is a Rust static library; this is its whole C side.
    printf 'int maiko_launcher_main(void);\nint main(void) { return maiko_launcher_main(); }\n' \
        > /tmp/launcher_main.c
    for arch in x86_64 arm64; do
        clang -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
            --ld-path=/usr/lib/llvm-14/bin/ld64.lld -Wl,-platform_version,macos,11.0,11.3 \
            /tmp/launcher_main.c \
            "/results/libmaiko-launcher-$arch.a" -framework AppKit -framework ApplicationServices -framework Carbon -framework IOKit \
            -framework QuartzCore -framework CoreGraphics -framework CoreVideo -framework Metal -framework Foundation \
            -framework AudioToolbox -framework CoreAudio -framework CoreFoundation -framework Security \
            -framework UniformTypeIdentifiers -lobjc \
            -liconv -lresolv -o "/results/maiko-$arch"
    done
    llvm-lipo-14 -create /results/maiko-x86_64 /results/maiko-arm64 -output /results/runtime/maiko
    chmod +x /results/runtime/maiko
    cp /work/platforms/macos/LAUNCH.md /results/runtime/README.md
    rm -f /results/game.bc
fi
