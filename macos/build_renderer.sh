#!/bin/sh
set -eu
cd /src
python3 - <<'PY'
from pathlib import Path
text=Path('sdk2_unix.h.in').read_text()
for key,size in [('UNSIGNED_CHAR',1),('UNSIGNED_SHORT',2),('UNSIGNED_INT',4),('UNSIGNED_LONG',8),('UNSIGNED_LONG_LONG',8),('INT_P',8)]:
    text=text.replace('@SIZEOF_'+key+'@',str(size))
Path('sdk2_unix.h').write_text(text)
PY
sources='grguDepth.cpp grguMisc.cpp grgu3df.cpp grguDraw.cpp grguSstGlide.cpp grguFog.cpp grguTex.cpp grguLfb.cpp GLRender.cpp OGLFogTables.cpp OGLTextureTables.cpp OGLColorAlphaTables.cpp TexDB.cpp PGUTexture.cpp Glide.cpp GLExtensions.cpp PGTexture.cpp FormatConversion.cpp grguBuffer.cpp grguColorAlpha.cpp GLutil.cpp gsplash.cpp g3wrap.cpp platform/linux/error.cpp platform/sdl/openglext.cpp platform/linux/library.cpp platform/sdl/window.cpp /work/macos/glide_clock.cpp'
for arch in x86_64 arm64; do
    clang++ -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
        -stdlib=libc++ -std=gnu++11 -O2 -fPIC -dynamiclib -DC_USE_SDL \
        -D__darwin__ -DRG_SDL_WINDOW_POINTER -DSIZEOF_INT_P=8 \
        -I. -Iplatform/linux -I/frameworks/SDL2.framework/Headers -F/frameworks \
        --ld-path=/usr/lib/llvm-14/bin/ld64.lld \
        -Wl,-platform_version,macos,11.0,11.3 \
        -Wl,-install_name,@rpath/libredguard-glide.dylib \
        $sources -framework SDL2 -framework OpenGL \
        -o "/results/libredguard-glide-$arch.dylib"
    llvm-objdump --macho --private-headers --dylibs-used "/results/libredguard-glide-$arch.dylib" \
        > "/results/renderer-$arch.txt"
    clang++ -target "$arch-apple-macos11" -isysroot /sdk/MacOSX11.3.sdk \
        -stdlib=libc++ -std=c++11 -O2 -DSDL_MAIN_HANDLED \
        -I. -I/frameworks/SDL2.framework/Headers -F/frameworks \
        --ld-path=/usr/lib/llvm-14/bin/ld64.lld \
        -Wl,-platform_version,macos,11.0,11.3 \
        -Wl,-rpath,@executable_path/Frameworks -Wl,-rpath,@executable_path \
        /work/tests/backend_smoke.cpp "/results/libredguard-glide-$arch.dylib" \
        -framework SDL2 -o "/results/glide-smoke-$arch"
done
