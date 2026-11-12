#!/usr/bin/env python3
"""Build/test the experimental native CRT target, with bounded offline containers."""
import argparse
import json
import hashlib
import bisect
import re
from pathlib import Path
import subprocess
import time
import uuid
import sys
from .build_windows import IMAGE, ROOT, run, ensure_tool_image, collect_tool_notices
from .sr_heap import stage_sr_heap
from .sr_audio import stage_sr_audio
from .windows_sdl import stage_windows_sdl
from .rust_runtime import stage as stage_rust_runtime
from .gog_install import movie_bundle

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('work', type=Path)
    p.add_argument('--install', type=Path, required=True)
    p.add_argument('--menu-autosave', type=Path, help='Use a copied save as slot001 in a disposable scenario with sound/CD audio disabled')
    p.add_argument('--run-seconds',type=int,default=45,choices=range(1,181),metavar='1..180')
    p.add_argument('--cd-image',type=Path,help='Owned ISO or MODE1/2352 image for the original movie regression')
    p.add_argument('--installed-gog',action='store_true',help='Test automatic movies from the selected installed GOG layout, without a runtime media argument')
    p.add_argument('--movie-skip',action='store_true',help='Verify original intro playback/skips and ship entry using --cd-image')
    p.add_argument('--gameplay',choices=['continue','new','both'])
    p.add_argument('--modal-ui',action='store_true',help='Check original logbook and map input in the Continue probe')
    p.add_argument('--exercise',action='store_true',help='Check inventory close and original sword/attack/jump input under Wine')
    p.add_argument('--gameplay-seconds',type=int,default=30,choices=range(1,181),metavar='1..180')
    p.add_argument('--save-roundtrip',action='store_true',help='Verify original autosave/reload with Wine and isolated user files')
    p.add_argument('--redbook',action='store_true',help='Exercise the original CD-open absence path; CD playback is not implemented')
    p.add_argument('--sound',action='store_true',help='Enable sample audio through SDL dummy test device')
    p.add_argument('--writable-gameplay',action='store_true',help='Use isolated writable user files in gameplay probes')
    p.add_argument('--world-controls',action='store_true',help='Check original movement/book/Quit on Continue under Wine')
    a = p.parse_args()
    if a.installed_gog and a.cd_image:p.error('Choose installed GOG or explicit media')
    if a.installed_gog and not a.movie_skip:p.error('--installed-gog requires --movie-skip')
    if a.movie_skip and not (a.cd_image or a.installed_gog):p.error('--movie-skip requires --installed-gog or --cd-image')
    if a.cd_image and not str(a.cd_image.resolve()).startswith('/home/vibepod/workspaces/'):
        p.error('CD image mount must be under /home/vibepod/workspaces')
    if a.gameplay and not a.menu_autosave:p.error('--gameplay requires --menu-autosave')
    work = a.work.resolve()
    report = json.loads((work / 'report.json').read_text())
    if report['status'] != 'native_assembly_emitted': p.error('Requires --native translation')
    install = a.install.resolve()
    media=movie_bundle(install) if a.installed_gog else a.cd_image.resolve() if a.cd_image else None
    if not str(install).startswith('/home/vibepod/workspaces/'): p.error('Docker game mount must be under /home/vibepod/workspaces')
    name = 'rg-native-' + uuid.uuid4().hex[:10]
    report.update(native_build_status='started', native_crt_verified=False,
                  native_config_verified=False, native_glide_window_verified=False, menu_verified=False, native_windows_verified=False,full_game_verified=False)
    for key in ['native_wine_exit','native_exe_sha256','native_boundary_symbols']:
        report.pop(key,None)
    variants=json.loads((ROOT/'metadata/image_variants.json').read_text())
    if report['image']['sha256']!=variants['translated_image_sha256']:
        raise ValueError('Image variant guards require the inspected local translation')
    report['image_variants']=variants
    (work/'game_build.h').write_text('#define RGFX_SHA256 "'+report['image']['sha256']+'"\n'
        '#define RGFX_ALTERNATE_SHA256 "'+variants['alternate_image_sha256']+'"\n')
    began = time.monotonic()
    try:
        report['toolchain']=ensure_tool_image()
        run(['docker','run','-d','--network','none','--memory','4g','--memory-swap','4g','--cpus','2','--env','SDL_AUDIODRIVER=dummy','--name',name,
             '--mount',f'type=bind,src={install},dst=/game,readonly']+(['--mount',f'type=bind,src={media},dst=/media/game.gog,readonly'] if media else [])+[IMAGE,'sleep','600'])
        for file in work.glob('*'):
            if file.suffix in ['.asm','.inc','.h']: run(['docker','cp',str(file),name+':/work/'+file.name])
        for directory,destination in [(ROOT/'runtime','/work'),(ROOT/'tests','/tests'),(ROOT/'.cache/sr/SR/x64-support','/support'),(ROOT/'glide','/work'),(ROOT/'.cache/glide/include','/glide-sdk')]:
            run(['docker','cp',str(directory)+'/.',name+':'+destination])
        for include in ['extern.inc','macros.inc']:
            run(['docker','cp',str(work/include),name+':/work/'+include])
        backend=ROOT/'.cache/glide'
        backend_report=json.loads((backend/'report.json').read_text())
        if backend_report['status']!='smoke_and_guest_bridge_passed':
            raise ValueError('Run python3 -m tools.build_glide before linking the game')
        for file in ['redguard-glide.dll','libredguard-glide.a','libwinpthread-1.dll']:
            run(['docker','cp',str(backend/file),name+':/work/'+file])
        report['glide_backend']=backend_report
        collect_tool_notices(name,ROOT/'.cache/sr/SR/x64-support',work/'notices')
        report['rust_runtime']=stage_rust_runtime(name,'x86_64-pc-windows-gnu')
        report['sr_heap']=stage_sr_heap(name)
        report['sr_audio']=stage_sr_audio(name)
        report['sdl_audio']=stage_windows_sdl(name)
        run(['docker','cp',str(ROOT/'audio/audio_sdl.c'),name+':/audio/audio_sdl.c'])
        commands = '''set -e
x86_64-w64-mingw32-g++ -Wall -Wextra -Werror -O2 -I/work -c guest_memory.cpp -o guest_memory.obj
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 /tests/menu_keys.c -o menu-keys.exe
mkdir -p 'native game files'
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/crash_reporting.c crash.c -o crash-reporting-test.exe
python3 /tests/check_crash_reporting.py
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/game_install.c game_install.c -lbcrypt -Wl,--image-base,0x10000000,--disable-dynamicbase -o 'native game files/install-test.exe'
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 'native game files/install-test.exe'
nasm -f win64 -I/support/ -DABI_TEST native_entry.asm -o abi.obj
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 /tests/game_directory_dialog.c game_directory_dialog.c -lshell32 -lole32 -luser32 -o dialog-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine xvfb-run -a /usr/lib/wine/wine64 dialog-test.exe
nasm -f win64 -I/support/ /tests/abi_harness.asm -o abi_test.obj
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/abi_harness.c guest_memory.obj abi.obj abi_test.obj -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -o abi-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 abi-test.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/runtime_io.c guest_memory.obj game_files.c -Wl,--image-base,0x10000000,--disable-dynamicbase -o io-test.exe
(cd /game && WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 /work/io-test.exe)
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/platform_services.c guest_memory.obj platform_services.c input.c -luser32 -Wl,--image-base,0x10000000,--disable-dynamicbase -o platform-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 platform-test.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/native_input.c platform_services.c guest_memory.obj input.c -luser32 -o input-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine xvfb-run -a /usr/lib/wine/wine64 input-test.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/native_timing.c timing.c -o timing-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 timing-test.exe
nasm -f win64 /tests/image_variant.asm -o variant_test.obj
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 /tests/image_variant.c variant_test.obj -o variant-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 variant-test.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/guest_alloc.c guest_memory.obj -o alloc-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 alloc-test.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/heap_lifetime.c guest_memory.obj -o heap-lifetime.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 heap-lifetime.exe
nasm -f win64 -I/work/ /tests/font_metric.asm -o font_metric.obj
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/font_metric.c guest_memory.obj font_metric.obj -o font-metric-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 font-metric-test.exe
nasm -O1 -f win64 -I/support/ rgfx.asm -o rgfx.obj
nasm -f win64 -I/support/ native_entry.asm -o native_entry.obj
x86_64-w64-mingw32-g++ -std=c++11 -Wall -Wextra -Werror -O2 -I/work -I/glide-sdk -c bridge.cpp -o bridge.obj
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 native_main.c crash.c guest_memory.obj game_files.c platform_services.c input.c timing.c game_install.c game_directory_dialog.c native_entry.obj rgfx.obj bridge.obj -L/work -lredguard-glide -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp,-Map,native.map -lbcrypt -luser32 -lshell32 -lole32 -o redguard-native.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -Dmain=rg_unused_main -c native_main.c -o test_host.obj
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/original_input.c test_host.obj crash.c guest_memory.obj game_files.c platform_services.c input.c timing.c game_install.c game_directory_dialog.c native_entry.obj rgfx.obj bridge.obj -L/work -lredguard-glide -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -lbcrypt -luser32 -lshell32 -lole32 -o original-input-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine xvfb-run -a /usr/lib/wine/wine64 original-input-test.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/original_numeric.c test_host.obj crash.c guest_memory.obj game_files.c platform_services.c input.c timing.c game_install.c game_directory_dialog.c native_entry.obj rgfx.obj bridge.obj -L/work -lredguard-glide -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -lbcrypt -luser32 -lshell32 -lole32 -o original-numeric-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 original-numeric-test.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/original_heap.c test_host.obj crash.c guest_memory.obj game_files.c platform_services.c input.c timing.c game_install.c game_directory_dialog.c native_entry.obj rgfx.obj bridge.obj -L/work -lredguard-glide -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -lbcrypt -luser32 -lshell32 -lole32 -o original-heap-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 original-heap-test.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/original_directory.c test_host.obj crash.c guest_memory.obj game_files.c platform_services.c input.c timing.c game_install.c game_directory_dialog.c native_entry.obj rgfx.obj bridge.obj -L/work -lredguard-glide -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -lbcrypt -luser32 -lshell32 -lole32 -o original-directory-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 original-directory-test.exe
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/original_clip.c test_host.obj crash.c guest_memory.obj game_files.c platform_services.c input.c timing.c game_install.c game_directory_dialog.c native_entry.obj rgfx.obj bridge.obj -L/work -lredguard-glide -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -lbcrypt -luser32 -lshell32 -lole32 -o original-clip-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 original-clip-test.exe
for name in rob_vertex file_read animation_flags attribute animation_advance critical movie_clock audio close; do
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/original_$name.c test_host.obj crash.c guest_memory.obj game_files.c platform_services.c input.c timing.c game_install.c game_directory_dialog.c native_entry.obj rgfx.obj bridge.obj -L/work -lredguard-glide -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -lbcrypt -luser32 -lshell32 -lole32 -o original-$name-test.exe
WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 original-$name-test.exe
done
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -I/work /tests/original_fault.c test_host.obj crash.c guest_memory.obj game_files.c platform_services.c input.c timing.c game_install.c game_directory_dialog.c native_entry.obj rgfx.obj bridge.obj -L/work -lredguard-glide -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -lbcrypt -luser32 -lshell32 -lole32 -o original-fault-test.exe
python3 /tests/check_original_fault.py
x86_64-w64-mingw32-nm -n redguard-native.exe > native-symbols.txt
'''
        # The original-function tests all use the same host flags. Compile each
        # host source once so adding a function regression costs a link/run.
        common=['movie_files','cd_image','crash','game_files','platform_services','input','timing','game_install','game_directory_dialog','windows_storage']
        for source in common:
            commands=commands.replace(' '+source+'.c ',' '+source+'.obj ')
        flags='-Wall -Wextra -Werror -O2 -I/work -I/audio -I/sdl/include/SDL2 -DRG_USE_SDL_AUDIO -DSDL_MAIN_HANDLED'
        prefix='set -e\n'+''.join('x86_64-w64-mingw32-gcc '+flags+' -c '+source+'.c -o '+source+'.obj\n' for source in common if source not in ['movie_files','timing'])
        prefix+='x86_64-w64-mingw32-gcc '+flags+' -c /audio/audio_sdl.c -o audio_sdl.obj\n'
        prefix+='x86_64-w64-mingw32-gcc '+flags+' -c /audio/stream_pcm.c -o stream_pcm.obj\n'
        prefix+='x86_64-w64-mingw32-gcc '+flags+' -Wno-unused-parameter -c /audio/Albion-sound.c -o sr_audio.obj\n'
        commands=commands.replace('x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 ',
                                  'x86_64-w64-mingw32-gcc '+flags+' ')
        commands=prefix+commands.removeprefix('set -e\n')
        commands=commands.replace(' bridge.obj -L/work',' bridge.obj audio_sdl.obj stream_pcm.obj sr_audio.obj -L/sdl/lib -lSDL2_mixer -lSDL2 -L/work')
        commands='\n'.join(line.replace(' -o ', ' windows_storage.obj cd_image.obj movie_files.obj -o ') if line.startswith('x86_64-w64-mingw32-gcc ') and (' game_files.obj ' in line or ' game_install.obj ' in line) else line for line in commands.split('\n'))
        commands=commands.replace('WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 original-$name-test.exe',
                                  'SDL_AUDIODRIVER=dummy WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine xvfb-run -a /usr/lib/wine/wine64 original-$name-test.exe')
        sample_tests='x86_64-w64-mingw32-gcc '+flags+' /tests/audio_samples.c guest_memory.obj sr_audio.obj -L/sdl/lib -lSDL2_mixer -lSDL2 -o audio-samples.exe\n'
        sample_tests+='for sample_case in pan loop volume cache; do SDL_AUDIODRIVER=dummy WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 audio-samples.exe "$sample_case"; done\n'
        sample_tests+='x86_64-w64-mingw32-gcc '+flags+' /tests/audio_stream.c audio_sdl.obj stream_pcm.obj sr_audio.obj guest_memory.obj -L/sdl/lib -lSDL2_mixer -lSDL2 -o audio-stream.exe\n'
        sample_tests+='SDL_AUDIODRIVER=dummy WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 audio-stream.exe\n'
        sample_tests+='x86_64-w64-mingw32-gcc '+flags+' /tests/stream_pcm.c stream_pcm.obj -L/sdl/lib -lSDL2 -o stream-pcm.exe\n'
        sample_tests+='WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 stream-pcm.exe\n'
        commands=commands.replace('nasm -O1 -f win64',sample_tests+'nasm -O1 -f win64')
        cd_tests='x86_64-w64-mingw32-gcc '+flags+' /tests/cd_image.c cd_image.obj -Wl,--image-base,0x10000000,--disable-dynamicbase -o cd-image-test.exe\n'
        cd_tests+='WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine python3 /tests/check_cd_image.py /work/cd-image-test.exe --wine\n'
        commands=commands.replace('nasm -O1 -f win64',cd_tests+'nasm -O1 -f win64')
        storage_test='x86_64-w64-mingw32-gcc '+flags+' /tests/windows_storage.c game_files.obj windows_storage.obj cd_image.obj movie_files.obj -Wl,--image-base,0x10000000,--disable-dynamicbase -o windows-storage.exe\n'
        storage_test+='WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 windows-storage.exe\n'
        storage_test+='x86_64-w64-mingw32-gcc '+flags+' /tests/installed_movies.c game_files.obj windows_storage.obj cd_image.obj movie_files.obj -Wl,--image-base,0x10000000,--disable-dynamicbase -o installed-movies.exe\n'
        storage_test+='mkdir movie-contract; (cd movie-contract && WINEDEBUG=-all WINEPREFIX=/tmp/rg-wine /usr/lib/wine/wine64 ../installed-movies.exe)\n'
        commands=commands.replace('nasm -O1 -f win64',storage_test+'nasm -O1 -f win64')
        commands=commands.replace('movie_files.obj','/work/libmaiko_runtime.a').replace('timing.obj','/work/libmaiko_runtime.a')
        commands='\n'.join(line+' -lws2_32 -luserenv -lntdll -ladvapi32 -ldbghelp -lgcc_eh -lwinpthread' if line.startswith('x86_64-w64-mingw32-gcc ') and ' -c ' not in line else line for line in commands.split('\n'))
        with (work/'native-build.log').open('w') as log:
            run(['docker','exec',name,'timeout','360','sh','-c',commands],stdout=log,stderr=subprocess.STDOUT)
        report['native_build_status']='linked'
        report['wine_crash_dispatch_tests_verified']=True
        report['wine_translated_fault_reports_verified']=True
        report['wine_original_clip_tests_verified']=True
        report['wine_original_gameplay_function_tests_verified']=True
        report['wine_original_audio_tests_verified']=True
        report['native_setup_build_test_seconds']=round(time.monotonic()-began,3)
        for filename in ['redguard-native.exe','native.map','native-symbols.txt','redguard-glide.dll','libwinpthread-1.dll','SDL2.dll','SDL2_mixer.dll']:
            run(['docker','cp',name+':/work/'+filename,str(work/filename)])
        report['native_exe_sha256']=hashlib.sha256((work/'redguard-native.exe').read_bytes()).hexdigest()
        game_directory='Z:\\game'
        if a.menu_autosave:
            autosave=a.menu_autosave.resolve()
            run(['docker','cp',str(autosave),name+':/work/menu-autosave.sav'])
            scenario=run(['docker','exec',name,'python3','/tests/prepare_menu_scenario.py']+(['--sound'] if a.sound else [])+(['--redbook'] if a.redbook else []),capture_output=True)
            (work/'scenario.log').write_text(scenario.stdout)
            report['scenario']=dict(name='startup-autosave',sound=a.sound,redbook=a.redbook,
                                    autosave_sha256=hashlib.sha256(autosave.read_bytes()).hexdigest())
            game_directory='Z:\\work\\menu-scenario'
        run_began=time.monotonic()
        report['native_run_limit_seconds']=a.run_seconds
        with (work/'native-wine.log').open('w') as log:
            result = subprocess.run(['docker','exec',name,'timeout','-k','5',str(a.run_seconds),'env','WINEDEBUG=-all','WINEPREFIX=/tmp/rg-wine','LIBGL_ALWAYS_SOFTWARE=1','RG_CAPTURE_DIR=Z:\\work','xvfb-run','-a','/usr/lib/wine/wine64','redguard-native.exe','--game-dir',game_directory],stdout=log,stderr=subprocess.STDOUT,text=True)
        report['native_run_seconds']=round(time.monotonic()-run_began,3)
        for capture in ['game-frame-first.ppm','game-frame-latest.ppm']:
            subprocess.run(['docker','cp',name+':/work/'+capture,str(work/capture)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        output = (work/'native-wine.log').read_text()
        report['native_wine_exit'] = result.returncode
        report['native_crt_verified'] = 'CRT complete' in output
        report['native_glide_window_verified'] = bool(re.search(r'Native Glide window \d+x\d+ opened=1',output))
        report['native_config_verified'] = 'Original SYSTEM.INI parsed final_version=1 memory_manager=0' in output
        report['native_build_status']=('run_limit_reached' if result.returncode==124 else 'blocked') if result.returncode else 'original_main_returned'
        symbols=[]
        for line in (work/'native-symbols.txt').read_text().splitlines():
            match=re.fullmatch(r'([0-9a-fA-F]+) [tTdD] loc_([0-9A-F]+)',line)
            if match: symbols.append((int(match[1],16),int(match[2],16)))
        symbols.sort()
        addresses=[item[0] for item in symbols]
        boundaries=[]
        for item in re.findall(r'(?:host_pc=|return=|CALLSITE )([0-9a-fA-F]+)',output):
            pc=int(item,16); index=bisect.bisect_right(addresses,pc)-1
            if index>=0:
                host,original=symbols[index]
                if pc-host < 0x10000:
                    boundaries.append(dict(host_pc=hex(pc),original_label=hex(original),translated_byte_offset=pc-host))
        report['native_boundary_symbols']=boundaries
        if a.gameplay:
            report['wine_gameplay_probes']={}
            for action in (['continue','new'] if a.gameplay=='both' else [a.gameplay]):
                with (work/f'gameplay-{action}-runner.log').open('w') as log:
                    probe=subprocess.run(['docker','exec',name,'timeout','-k','5',str(a.gameplay_seconds+60),
                        'xvfb-run','-a','python3','/tests/probe_gameplay.py','--wine','--action',action,
                        '--seconds',str(a.gameplay_seconds)]+(['--user-dir',f'/work/gameplay-{action}-user'] if a.writable_gameplay else [])+(['--controls'] if a.world_controls and action=='continue' else [])+(['--exercise'] if a.exercise and (a.gameplay!='both' or action=='new') else [])+(['--modal-ui'] if a.modal_ui and action=='continue' else []),stdout=log,stderr=subprocess.STDOUT)
                for suffix in ['.json','.log','-menu.ppm','-latest.ppm','-inventory.ppm','-inventory-close.ppm','-sword.ppm','-attack.ppm','-attack-2.ppm','-attack-3.ppm','-jump.ppm','-logbook.ppm','-logbook-close.ppm','-map.ppm','-map-close.ppm']:
                    file='gameplay-'+action+suffix
                    subprocess.run(['docker','cp',name+':/work/'+file,str(work/file)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
                evidence=work/f'gameplay-{action}.json'
                report['wine_gameplay_probes'][action]=json.loads(evidence.read_text()) if evidence.exists() else {'harness_error':'No probe report'}
                if probe.returncode:raise RuntimeError(f'Wine {action} probe failed; inspect exported evidence')
        if a.save_roundtrip:
            with (work/'save-roundtrip-runner.log').open('w') as log:
                saved=subprocess.run(['docker','exec',name,'timeout','-k','5','220','xvfb-run','-a',
                                      'python3','/tests/check_save_roundtrip.py','--wine'],stdout=log,stderr=subprocess.STDOUT)
            for file in ['save-roundtrip.json','save-roundtrip-1.log','save-roundtrip-2.log','save-roundtrip-1.json','save-roundtrip-2.json']:
                subprocess.run(['docker','cp',name+':/work/'+file,str(work/file)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            if saved.returncode:raise RuntimeError('Wine original autosave/reload regression failed')
            report['wine_original_save_reload_verified']=True
        if a.movie_skip:
            if a.installed_gog:run(['docker','exec',name,'ln','-s','/media/game.gog','/work/game.gog'])
            with (work/'movie-runner.log').open('w') as log:
                movie=subprocess.run(['docker','exec',name,'timeout','-k','5','100','xvfb-run','-a',
                    'python3','/tests/probe_gameplay.py','--action','new','--movie-skip','--seconds','50',
                    '--user-dir','/work/movie-user']+(['--cd-image','/media/game.gog'] if a.cd_image else [])+['--wine'],stdout=log,stderr=subprocess.STDOUT)
            for suffix in ['.json','.log','-movie-skip-0.ppm','-movie-skip-1.ppm','-movie-skip-2.ppm','-movie-skip-3.ppm']:
                subprocess.run(['docker','cp',name+':/work/gameplay-new'+suffix,str(work/('movie'+suffix))],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            evidence=work/'movie.json'
            report['movie_probe']=json.loads(evidence.read_text()) if evidence.exists() else {'harness_error':'No movie report'}
            if movie.returncode:raise RuntimeError('Original movie/CD-image probe failed; inspect exported evidence')
            report['movie_video_skip_to_ship_verified']=True
            report['movie_audio_mixed_verified']=report['movie_probe'].get('movie_audio_mixed_verified',False)
            if a.sound and not report['movie_audio_mixed_verified']:raise RuntimeError('Movie audio did not mix nonzero PCM while video advanced')
            report['movie_audio_verified']=False  # Audible hardware playback remains separate.
            report['cd_image_files_verified']=bool(a.cd_image)
            report['installed_gog_movies_verified']=a.installed_gog
            if a.installed_gog and 'GOG movies: using bundled installed data' not in (work/'movie.log').read_text():
                raise RuntimeError('Movie probe did not discover the installed GOG data')

        report['tool_versions'] = subprocess.check_output(['docker','exec',name,'dpkg-query','-W','nasm','gcc-mingw-w64-x86-64-posix','wine-stable-amd64'],text=True).splitlines()
        print(output)
        print(work)
        return 0 if result.returncode==0 else 2
    except Exception:
        report['native_build_status']='failed'
        raise
    finally:
        report['native_build_run_seconds'] = round(time.monotonic()-began,3)
        (work/'report.json').write_text(json.dumps(report,indent=2)+'\n')
        subprocess.run(['docker','rm','-f',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

if __name__ == '__main__': sys.exit(main())
