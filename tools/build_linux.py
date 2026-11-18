#!/usr/bin/env python3
"""Build/test the native Linux x86-64 host using an existing native SR translation."""
import argparse
from datetime import datetime,timezone
import hashlib
import json
from pathlib import Path
import subprocess
import time
from .toolchain import (ROOT,run,ensure_tool_image,ensure_linux_image,is_mountable,MOUNT_ROOT,LINUX_IMAGE as IMAGE,
                        container_name,start_container,remove_container,copy_in,copy_out,game_build_header)
from . import probes
from .rust_runtime import build as build_rust_runtime, stage as stage_rust_runtime
from .gog_install import movie_bundle


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('translation',type=Path)
    p.add_argument('--install',type=Path,required=True)
    p.add_argument('--menu-autosave',type=Path)
    p.add_argument('--link-only',action='store_true',help='Stop after the tested build; skip the game scenarios (needs only RGFX.EXE in --install)')
    p.add_argument('--test-image',type=Path,help='Use either recognized original image in the disposable scenario')
    p.add_argument('--cd-image',type=Path,help='Owned ISO or MODE1/2352 image for the original movie regression')
    p.add_argument('--installed-gog',action='store_true',help='Test automatic movies from the installed GOG layout without a runtime media argument')
    p.add_argument('--movie-skip',action='store_true',help='Verify original intro playback/skips and ship entry using --cd-image')
    p.add_argument('--gameplay',choices=['continue','new','both'],help='Record bounded original gameplay probes after the menu regression')
    p.add_argument('--modal-ui',action='store_true',help='Check original logbook and map input in the Continue probe')
    p.add_argument('--exercise',action='store_true',help='Exercise original inventory/sword/attack/jump inputs in gameplay probes')
    p.add_argument('--gameplay-seconds',type=float,default=30,help='Observation time per probe, from1 to180 seconds; a timeout is not a pass')
    p.add_argument('--world-controls',action='store_true',help='Measure player movement and book/Quit in the Continue probe')
    p.add_argument('--writable-gameplay',action='store_true',help='Use disposable isolated save/config storage for gameplay probes')
    p.add_argument('--redbook',action='store_true',help="Enable redbook CD music in the scenario's SYSTEM.INI")
    p.add_argument('--sound',action='store_true',help='Enable native sample audio with the dummy test device; CD music remains disabled')
    p.add_argument('--keep-container',action='store_true',help='Retain the bounded development container for follow-up diagnosis')
    p.add_argument('--save-roundtrip',action='store_true',help='Verify original autosave/reload with the supplied stable noncombat save')
    a=p.parse_args()
    if not a.link_only and not a.menu_autosave:p.error('--menu-autosave is required unless --link-only')
    if a.installed_gog and a.cd_image:p.error('Choose installed GOG or explicit media')
    if a.installed_gog and not a.movie_skip:p.error('--installed-gog requires --movie-skip')
    if a.movie_skip and not (a.cd_image or a.installed_gog):p.error('--movie-skip requires --installed-gog or --cd-image')
    if a.cd_image and not is_mountable(a.cd_image):
        p.error(f'CD image mount must be under {MOUNT_ROOT}')
    if a.exercise and (not a.gameplay or a.world_controls):p.error('--exercise requires --gameplay without --world-controls')
    if a.world_controls and a.gameplay not in ['continue','both']:p.error('--world-controls requires --gameplay continue or both')
    if not 1<=a.gameplay_seconds<=180:p.error('--gameplay-seconds must be between1 and180')
    source=a.translation.resolve();install=a.install.resolve()
    media=movie_bundle(install) if a.installed_gog else a.cd_image.resolve() if a.cd_image else None
    if not is_mountable(install):
        p.error(f'Game mount must be under {MOUNT_ROOT}')
    original=json.loads((source/'report.json').read_text())
    if original.get('status')!='native_assembly_emitted':p.error('Requires python3 -m tools.pipeline translate output')
    variants=json.loads((ROOT/'metadata/image/image_variants.json').read_text())
    if original['image']['sha256']!=variants['translated_image_sha256']:p.error('Translation image hash mismatch')
    sr_head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT/'.cache/sr',text=True).strip()
    if sr_head!=original['sr_commit']:p.error('Mounted SR checkout differs from translation report')
    output=ROOT/'artifacts'/(datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')+'-linux')
    output.mkdir(parents=True)
    report=dict(target='linux-x86_64',status='started',sr_commit=original['sr_commit'],
                translation_image_sha256=original['image']['sha256'],image_variants=variants,full_game_verified=False,
                wine_used=False,menu_verified=False,native_windows_verified=False)
    report['patch_sha256']={file.name:hashlib.sha256(file.read_bytes()).hexdigest()
                            for file in sorted((ROOT/'patches').glob('*.patch'))}
    began=time.monotonic();name=container_name('linux')
    try:
        report['base_toolchain']=ensure_tool_image()
        with (output/'toolchain-build.log').open('w') as log:
            report['toolchain_image']=ensure_linux_image(stdout=log,stderr=subprocess.STDOUT)
        # Cargo can wait for another project; do this before the container lease starts.
        rust_runtime=build_rust_runtime('x86_64-unknown-linux-gnu')
        rust_host=build_rust_runtime('x86_64-unknown-linux-gnu',component='host')
        start_container(name,IMAGE,'3g',3600 if a.link_only else 1800 if a.keep_container else 480,
                        [(install,'/game')]+([(media,'/media/game.gog')] if media else []))
        if a.installed_gog:run(['docker','exec',name,'ln','-s','/media/game.gog','/work/game.gog'])
        copy_in(name,[(str(ROOT/directory)+'/.',destination) for directory,destination in
                      [('crates/runtime','/work'),('platforms/linux','/work'),('platforms/linux','/linux'),('tests','/tests'),('.cache/sr/SR/x64-support','/support')]])
        # Generated macros include checked game fixes and must overwrite template includes.
        for file in source.iterdir():
            if file.suffix in ['.asm','.inc','.h']:run(['docker','cp',str(file),name+':/work/'+file.name])
        header=output/'game_build.h'
        game_build_header(header,variants)
        run(['docker','cp',str(header),name+':/work/game_build.h'])
        report['rust_runtime']=stage_rust_runtime(name,rust_runtime)
        report['rust_host']=stage_rust_runtime(name,rust_host)
        setup=time.monotonic()
        with (output/'build.log').open('w') as log:
            run(['docker','exec',name,'timeout','1800' if a.link_only else '240','sh','-c',('export MAIKO_LINK_ONLY=1; ' if a.link_only else '')+'sh /linux/build_runtime.sh'],stdout=log,stderr=subprocess.STDOUT)
        report['build_test_seconds']=round(time.monotonic()-setup,3)
        report['original_function_tests_verified']=True
        for file in ['redguard-native','linux.map','linux-symbols.txt','linux-dependencies.txt']:
            run(['docker','cp',name+':/work/'+file,str(output/file)])
        report['exe_sha256']=hashlib.sha256((output/'redguard-native').read_bytes()).hexdigest()
        if a.link_only:
            report['status']='linked'
            return
        run(['docker','cp',str(a.menu_autosave.resolve()),name+':/work/menu-autosave.sav'])
        run(['docker','exec',name,'python3','/tests/prepare_menu_scenario.py']+(['--sound'] if a.sound else [])+(['--redbook'] if a.redbook else []))
        report['scenario_sound_enabled']=a.sound
        report['scenario_redbook_enabled']=a.redbook
        report['audio_test_device']='dummy'
        report['cd_playback_verified']=False
        report['movie_playback_verified']=False
        if a.test_image:
            digest=hashlib.sha256(a.test_image.read_bytes()).hexdigest()
            if digest not in [variants['translated_image_sha256'],variants['alternate_image_sha256']]:
                raise ValueError('Test image is not a recognized RGFX variant')
            run(['docker','exec',name,'rm','/work/menu-scenario/RGFX.EXE'])
            run(['docker','cp',str(a.test_image.resolve()),name+':/work/menu-scenario/RGFX.EXE'])
            report['scenario_image_sha256']=digest
        run_began=time.monotonic()
        with (output/'menu-test.log').open('w') as log:
            run(['docker','exec',name,'timeout','75','xvfb-run','-a','python3','/tests/check_linux_menu.py'],stdout=log,stderr=subprocess.STDOUT)
        report['menu_test_seconds']=round(time.monotonic()-run_began,3)
        for file in ['linux-menu.log','linux-menu-verification.json','linux-menu-before.ppm','linux-menu-options.ppm','linux-menu-return.ppm']:
            run(['docker','cp',name+':/work/'+file,str(output/file)])
        report['linux_menu_sequence_verified']=True
        report['status']='linux_menu_sequence_passed'
        if a.gameplay:
            report['gameplay_probes']=probes.gameplay(name,output,a,wine=False)
            report['status']='linux_menu_passed_gameplay_probed'

        if a.save_roundtrip:
            probes.save_roundtrip(name,output,wine=False)
            report['linux_original_save_reload_verified']=True

        if a.movie_skip:
            probes.movie_skip(name,output,a,False,report,environment=['-e','RG_MOVIE_PCM_CAPTURE=/work/movie-source.pcm'])
            if a.sound:
                with (output/'movie-pcm-reference.log').open('w') as log:
                    run(['docker','exec',name,'python3','/tests/check_movie_pcm.py'],stdout=log,stderr=subprocess.STDOUT)
                run(['docker','cp',name+':/work/movie-pcm-reference.json',str(output/'movie-pcm-reference.json')])
                report['original_movie_pcm_reference_verified']=True

        report['tool_versions']=subprocess.check_output(['docker','exec',name,'dpkg-query','-W','gcc','g++','nasm','libasound2-dev','libudev-dev','libssl-dev','mesa-vulkan-drivers'],text=True).splitlines()
    except Exception:
        report['status']='failed'
        # Preserve the real game boundary even when the menu assertion fails.
        copy_out(name,[('/work/'+file,output/file) for file in ['linux-menu.log','linux-menu-before.ppm','linux-menu-options.ppm','linux-menu-return.ppm']])
        raise
    finally:
        report['elapsed_seconds']=round(time.monotonic()-began,3)
        if a.keep_container:report['development_container']=name
        else:remove_container(name)
        (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
        print(output,flush=True)

if __name__=='__main__':main()
