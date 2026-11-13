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
from .toolchain import IMAGE, ROOT, run, ensure_tool_image, collect_tool_notices, is_mountable, MOUNT_ROOT
from .sr_heap import stage_sr_heap
from .sr_audio import stage_sr_audio
from .windows_sdl import stage_windows_sdl
from .rust_runtime import build as build_rust_runtime, stage as stage_rust_runtime
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
    if a.cd_image and not is_mountable(a.cd_image):
        p.error(f'CD image mount must be under {MOUNT_ROOT}')
    if a.gameplay and not a.menu_autosave:p.error('--gameplay requires --menu-autosave')
    work = a.work.resolve()
    report = json.loads((work / 'report.json').read_text())
    if report['status'] != 'native_assembly_emitted': p.error('Requires output of python3 -m tools.pipeline translate')
    install = a.install.resolve()
    media=movie_bundle(install) if a.installed_gog else a.cd_image.resolve() if a.cd_image else None
    if not is_mountable(install): p.error(f'Docker game mount must be under {MOUNT_ROOT}')
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
        # Cargo can wait for another project; do this before the container lease starts.
        rust_runtime=build_rust_runtime('x86_64-pc-windows-gnu')
        rust_host=build_rust_runtime('x86_64-pc-windows-gnu',component='host')
        run(['docker','run','-d','--network','none','--memory','4g','--memory-swap','4g','--cpus','2','--env','SDL_AUDIODRIVER=dummy','--name',name,
             '--mount',f'type=bind,src={install},dst=/game,readonly']+(['--mount',f'type=bind,src={media},dst=/media/game.gog,readonly'] if media else [])+[IMAGE,'sleep','600'])
        for file in work.glob('*'):
            if file.suffix in ['.asm','.inc','.h']: run(['docker','cp',str(file),name+':/work/'+file.name])
        for directory,destination in [(ROOT/'runtime','/work'),(ROOT/'platforms/windows','/work'),(ROOT/'tests','/tests'),(ROOT/'.cache/sr/SR/x64-support','/support'),(ROOT/'glide','/work'),(ROOT/'.cache/glide/include','/glide-sdk')]:
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
        report['rust_runtime']=stage_rust_runtime(name,rust_runtime)
        report['rust_host']=stage_rust_runtime(name,rust_host)
        report['sr_heap']=stage_sr_heap(name)
        report['sr_audio']=stage_sr_audio(name)
        report['sdl_audio']=stage_windows_sdl(name)
        commands = 'sh /work/build_runtime.sh'
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
