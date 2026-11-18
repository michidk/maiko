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
