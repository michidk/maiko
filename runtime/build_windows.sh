#!/bin/sh
set -eu
ulimit -c 0
cd /work
sh /tests/init_wine.sh /tmp/rg-wine
cc='x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work -I/audio -I/sdl/include/SDL2 -DRG_USE_SDL_AUDIO -DSDL_MAIN_HANDLED'
x86_64-w64-mingw32-g++ -Wall -Wextra -Werror -O2 -I/work -x c++ -c sr_Game_memory.c -o guest_memory.obj
$cc -Wno-unused-parameter -c /audio/Albion-sound.c -o sr_audio.obj
link() {
    $cc "$@" -Wl,--gc-sections -Wl,--start-group ${rust_library:-/work/libmaiko_runtime.a} guest_memory.obj sr_audio.obj -Wl,--end-group \
      -L/sdl/lib -lSDL2_mixer -lSDL2 -L/work -lredguard-glide -lstdc++ \
      -lws2_32 -luserenv -lntdll -ladvapi32 -ldbghelp -lgcc_eh -lwinpthread -luser32 -lshell32 -lole32 \
      -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp
}
wine_run() { env SDL_AUDIODRIVER=dummy WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 "$@"; }
$cc /tests/menu_keys.c -o menu-keys.exe
mkdir -p 'native game files'
link /tests/crash_reporting.c -o crash-reporting-test.exe
python3 /tests/check_crash_reporting.py
link /tests/game_install.c -o 'native game files/install-test.exe'
wine_run 'native game files/install-test.exe'
link /tests/game_directory_dialog.c -o dialog-test.exe
env WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine xvfb-run -a /usr/lib/wine/wine64 dialog-test.exe
nasm -f win64 -I/support/ -DABI_TEST native_entry.asm -o abi.obj
nasm -f win64 -I/support/ /tests/abi_harness.asm -o abi_test.obj
link /tests/abi_harness.c abi.obj abi_test.obj -o abi-test.exe
wine_run abi-test.exe
link /tests/runtime_io.c -o io-test.exe
(cd /game && wine_run /work/io-test.exe)
link /tests/platform_services.c -o platform-test.exe
wine_run platform-test.exe
link /tests/sdl_input.c -o input-test.exe
env WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine xvfb-run -a /usr/lib/wine/wine64 input-test.exe
link /tests/native_timing.c -o timing-test.exe
wine_run timing-test.exe
nasm -f win64 /tests/image_variant.asm -o variant_test.obj
link /tests/image_variant.c variant_test.obj -o variant-test.exe
wine_run variant-test.exe
for spec in 'guest_alloc alloc-test' 'heap_lifetime heap-lifetime' 'audio_stream audio-stream' 'stream_pcm stream-pcm' 'windows_storage windows-storage'; do
    set -- $spec
    link /tests/"$1".c -o "$2".exe
    wine_run "$2".exe
done
link /tests/audio_samples.c -o audio-samples.exe
for sample_case in pan loop volume cache; do wine_run audio-samples.exe "$sample_case"; done
link /tests/cd_image.c -o cd-image-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine python3 /tests/check_cd_image.py /work/cd-image-test.exe --wine
link /tests/installed_movies.c -o installed-movies.exe
mkdir movie-contract
(cd movie-contract && wine_run ../installed-movies.exe)
nasm -f win64 -I/work/ /tests/font_metric.asm -o font_metric.obj
link /tests/font_metric.c font_metric.obj -o font-metric-test.exe
wine_run font-metric-test.exe
nasm -O1 -f win64 -I/support/ rgfx.asm -o rgfx.obj
nasm -f win64 -I/support/ native_entry.asm -o native_entry.obj
nasm -f win64 main.asm -o main.obj
nasm -f win64 service.asm -o service.obj
rust_library=/work/libmaiko_host.a link main.obj service.obj native_entry.obj rgfx.obj -Wl,-Map,native.map -o redguard-native.exe
for name in input numeric heap directory clip rob_vertex file_read animation_flags attribute animation_advance critical movie_clock audio close; do
    rust_library=/work/libmaiko_host.a link /tests/original_$name.c service.obj native_entry.obj rgfx.obj -o original-$name-test.exe
    case "$name" in
        input) env SDL_AUDIODRIVER=dummy WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine xvfb-run -a /usr/lib/wine/wine64 original-$name-test.exe ;;
        close) xvfb-run -a python3 /tests/check_original_shutdown.py --wine ;;
        *) wine_run original-$name-test.exe ;;
    esac
done
rust_library=/work/libmaiko_host.a link /tests/original_fault.c service.obj native_entry.obj rgfx.obj -o original-fault-test.exe
python3 /tests/check_original_fault.py
x86_64-w64-mingw32-nm -n redguard-native.exe > native-symbols.txt
