#!/usr/bin/env python3
"""Build/test the native CRT target, with bounded offline containers."""
import argparse
import json
import hashlib
import bisect
import re
from pathlib import Path
import subprocess
import time
import sys
from .toolchain import (IMAGE, ROOT, run, ensure_tool_image, collect_tool_notices, is_mountable, MOUNT_ROOT,
                        container_name, start_container, remove_container, copy_in, copy_out, game_build_header)
from . import probes
from .rust_runtime import build as build_rust_runtime, stage as stage_rust_runtime, CRATE_IMPORT_LIBRARIES, crate_import_libraries
from .gog_install import movie_bundle

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('work', type=Path)
    p.add_argument('--install', type=Path, required=True)
    p.add_argument('--link-only',action='store_true',help='Stop after the tested build; skip all game runs (needs only RGFX.EXE in --install)')
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
    p.add_argument('--redbook',action='store_true',help="Enable redbook CD music in the scenario's SYSTEM.INI")
    p.add_argument('--sound',action='store_true',help='Enable sample audio (silent without a sound device)')
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
    name = container_name('native')
    report.update(native_build_status='started', native_crt_verified=False,
                  native_config_verified=False, native_glide_window_verified=False, menu_verified=False, native_windows_verified=False,full_game_verified=False)
    for key in ['native_wine_exit','native_exe_sha256','native_boundary_symbols']:
        report.pop(key,None)
    variants=json.loads((ROOT/'metadata/image/image_variants.json').read_text())
    if report['image']['sha256']!=variants['translated_image_sha256']:
        raise ValueError('Image variant guards require the inspected local translation')
    report['image_variants']=variants
    game_build_header(work/'game_build.h',variants)
    began = time.monotonic()
    try:
        report['toolchain']=ensure_tool_image()
        # Cargo can wait for another project; do this before the container lease starts.
        rust_runtime=build_rust_runtime('x86_64-pc-windows-gnu')
        rust_host=build_rust_runtime('x86_64-pc-windows-gnu',component='host')
        start_container(name,IMAGE,'4g',3600 if a.link_only else 600,
                        [(install,'/game')]+([(media,'/media/game.gog')] if media else []))
        copy_in(name,[(file,'/work/'+file.name) for file in work.glob('*') if file.suffix in ['.asm','.inc','.h']])
        copy_in(name,[(str(ROOT/directory)+'/.',destination) for directory,destination in
                      [('crates/runtime','/work'),('platforms/windows','/work'),('tests','/tests'),('.cache/sr/SR/x64-support','/support')]])
        copy_in(name,[(work/include,'/work/'+include) for include in ['extern.inc','macros.inc']])
        collect_tool_notices(name,ROOT/'.cache/sr/SR/x64-support',work/'notices')
        report['rust_runtime']=stage_rust_runtime(name,rust_runtime)
        report['rust_host']=stage_rust_runtime(name,rust_host)
        # The runtime's crates link MinGW import libraries their -sys crates ship.
        for library in crate_import_libraries():
            run(['docker','cp',str(library),name+':/work/'])
        commands = ('MAIKO_LINK_ONLY=1 ' if a.link_only else '')+"CRATE_LIBRARIES='"+' '.join('-l'+n for n in CRATE_IMPORT_LIBRARIES)+"' sh /work/build_runtime.sh"
        with (work/'native-build.log').open('w') as log:
            run(['docker','exec',name,'timeout','1800' if a.link_only else '360','sh','-c',commands],stdout=log,stderr=subprocess.STDOUT)
        report['native_build_status']='linked'
        report['native_setup_build_test_seconds']=round(time.monotonic()-began,3)
        for filename in ['redguard-native.exe','native.map','native-symbols.txt']:
            run(['docker','cp',name+':/work/'+filename,str(work/filename)])
        report['native_exe_sha256']=hashlib.sha256((work/'redguard-native.exe').read_bytes()).hexdigest()
        if a.link_only:
            print(work)
            return 0
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
        copy_out(name,[('/work/'+capture,work/capture) for capture in ['game-frame-first.ppm','game-frame-latest.ppm']])
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
            report['wine_gameplay_probes']=probes.gameplay(name,work,a,wine=True)
        if a.save_roundtrip:
            probes.save_roundtrip(name,work,wine=True)
            report['wine_original_save_reload_verified']=True
        if a.movie_skip:
            if a.installed_gog:run(['docker','exec',name,'ln','-s','/media/game.gog','/work/game.gog'])
            probes.movie_skip(name,work,a,True,report)

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
        remove_container(name)

if __name__ == '__main__': sys.exit(main())
