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
sources='grguDepth.cpp grguMisc.cpp grgu3df.cpp grguDraw.cpp grguSstGlide.cpp grguFog.cpp grguTex.cpp grguLfb.cpp GLRender.cpp OGLFogTables.cpp OGLTextureTables.cpp OGLColorAlphaTables.cpp TexDB.cpp PGUTexture.cpp Glide.cpp GLExtensions.cpp PGTexture.cpp FormatConversion.cpp grguBuffer.cpp grguColorAlpha.cpp GLutil.cpp gsplash.cpp g3wrap.cpp platform/linux/clock.cpp platform/linux/error.cpp platform/sdl/openglext.cpp platform/linux/library.cpp platform/sdl/window.cpp'
g++ -std=gnu++11 -O2 -fPIC -shared -DC_USE_SDL -DRG_SDL_WINDOW_POINTER -DSIZEOF_INT_P=8 -I. -Iplatform/linux $(pkg-config --cflags sdl2) $sources $(pkg-config --libs sdl2) -lGL -lGLU -ldl -o /work/libredguard-glide.so
