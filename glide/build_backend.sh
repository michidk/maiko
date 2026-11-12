#!/bin/sh
# Run inside the pinned experiment tool container with a disposable /src checkout.
set -eu
cd /src
python3 - <<'PY'
from pathlib import Path
text=Path('sdk2_unix.h.in').read_text()
for key,size in [('UNSIGNED_CHAR',1),('UNSIGNED_SHORT',2),('UNSIGNED_INT',4),('UNSIGNED_LONG',4),('UNSIGNED_LONG_LONG',8),('INT_P',8)]:
    text=text.replace('@SIZEOF_'+key+'@',str(size))
Path('sdk2_unix.h').write_text(text)
PY
sources='grguDepth.cpp grguMisc.cpp grgu3df.cpp grguDraw.cpp grguSstGlide.cpp grguFog.cpp grguTex.cpp grguLfb.cpp GLRender.cpp OGLFogTables.cpp OGLTextureTables.cpp OGLColorAlphaTables.cpp TexDB.cpp PGUTexture.cpp Glide.cpp GLExtensions.cpp PGTexture.cpp FormatConversion.cpp grguBuffer.cpp grguColorAlpha.cpp GLutil.cpp gsplash.cpp g3wrap.cpp platform/windows/clock.cpp platform/windows/error.cpp platform/windows/openglext.cpp platform/windows/library.cpp platform/windows/window.cpp'
x86_64-w64-mingw32-g++ -std=gnu++11 -O2 -DWIN32 -DCPPDLL -I. -Iplatform/windows -shared $sources -static-libgcc -static-libstdc++ -luser32 -lgdi32 -lopengl32 -lglu32 -Wl,--no-insert-timestamp,--out-implib,/work/libredguard-glide.a -o /work/redguard-glide.dll
x86_64-w64-mingw32-g++ -std=c++11 -O2 -I. /work/backend_smoke.cpp -L/work -lredguard-glide -static-libgcc -static-libstdc++ -Wl,--no-insert-timestamp -o /work/backend-smoke.exe
cp /usr/x86_64-w64-mingw32/lib/libwinpthread-1.dll /work/
cd /work
WINEDEBUG=-all WINEPREFIX=/tmp/glide-wine LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s '-screen 0 1024x768x24' /usr/lib/wine/wine64 backend-smoke.exe
