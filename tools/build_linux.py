#!/usr/bin/env python3
"""Build/test the native Linux x86-64 host using an existing native SR translation."""
import argparse
from datetime import datetime,timezone
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import time
import uuid
from .toolchain import ROOT,run,ensure_tool_image,is_mountable,MOUNT_ROOT
from .sr_heap import stage_sr_heap
from .sr_audio import stage_sr_audio
from .rust_runtime import build as build_rust_runtime, stage as stage_rust_runtime
from .gog_install import movie_bundle

IMAGE='redguard-recomp-linux:bookworm'
BACKEND='9f8a2cb73af3b6d26e9ec67bb9e1873d3e211726'

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('translation',type=Path)
    p.add_argument('--install',type=Path,required=True)
    p.add_argument('--menu-autosave',type=Path,required=True)
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
    p.add_argument('--redbook',action='store_true',help='Exercise the original CD-open absence path; CD playback is not implemented')
    p.add_argument('--sound',action='store_true',help='Enable native sample audio with the dummy test device; CD music remains disabled')
    p.add_argument('--keep-container',action='store_true',help='Retain the bounded development container for follow-up diagnosis')
    p.add_argument('--save-roundtrip',action='store_true',help='Verify original autosave/reload with the supplied stable noncombat save')
    a=p.parse_args()
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
    variants=json.loads((ROOT/'metadata/image_variants.json').read_text())
    if original['image']['sha256']!=variants['translated_image_sha256']:p.error('Translation image hash mismatch')
    sr_head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT/'.cache/sr',text=True).strip()
    if sr_head!=original['sr_commit']:p.error('Mounted SR checkout differs from translation report')
    output=ROOT/'artifacts'/(datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')+'-linux')
    output.mkdir(parents=True)
    report=dict(target='linux-x86_64',status='started',sr_commit=original['sr_commit'],
                translation_image_sha256=original['image']['sha256'],image_variants=variants,full_game_verified=False,
                backend_commit=BACKEND,wine_used=False,menu_verified=False,native_windows_verified=False)
    report['patch_sha256']={file.name:hashlib.sha256(file.read_bytes()).hexdigest()
                            for file in sorted((ROOT/'patches').glob('*.patch'))}
    began=time.monotonic();name='rg-linux-'+uuid.uuid4().hex[:10]
    try:
        report['base_toolchain']=ensure_tool_image()
        if subprocess.run(['docker','image','inspect',IMAGE],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
            with (output/'toolchain-build.log').open('w') as log:
                run(['docker','build','-t',IMAGE,'-f',str(ROOT/'tools/docker/linux.Dockerfile'),str(ROOT/'tools/docker')],stdout=log,stderr=subprocess.STDOUT)
        report['toolchain_image']=subprocess.check_output(['docker','image','inspect',IMAGE,'--format','{{.Id}}'],text=True).strip()
        backend=ROOT/'.cache/openglide-ng'
        archive=subprocess.check_output(['git','archive',BACKEND],cwd=backend)
        with tempfile.TemporaryDirectory(dir=ROOT/'.cache') as directory:
            subprocess.run(['tar','-x','-C',directory],input=archive,check=True)
            for patch in ['openglide-x64.patch','openglide-sdl-window.patch']:
                subprocess.run(['patch','-p1','-d',directory],input=(ROOT/'patches'/patch).read_bytes(),check=True)
            archive=subprocess.check_output(['tar','-c','-C',directory,'.'])
        # Cargo can wait for another project; do this before the container lease starts.
        rust_runtime=build_rust_runtime('x86_64-unknown-linux-gnu')
        rust_host=build_rust_runtime('x86_64-unknown-linux-gnu',component='host')
        run(['docker','run','-d','--network','none','--memory','3g','--memory-swap','3g','--cpus','2','--env','SDL_AUDIODRIVER=dummy','--name',name,
             '--mount',f'type=bind,src={install},dst=/game,readonly']+(['--mount',f'type=bind,src={media},dst=/media/game.gog,readonly'] if media else [])+[IMAGE,'sleep','1800' if a.keep_container else '480'])
        if a.installed_gog:run(['docker','exec',name,'ln','-s','/media/game.gog','/work/game.gog'])
        run(['docker','exec',name,'mkdir','/src'])
        subprocess.run(['docker','exec','-i',name,'tar','-x','-C','/src'],input=archive,check=True)
        for directory,destination in [('runtime','/work'),('glide','/work'),('linux','/linux'),('tests','/tests'),('.cache/sr/SR/x64-support','/support')]:
            run(['docker','cp',str(ROOT/directory)+'/.',name+':'+destination])
        # Generated macros include checked game fixes and must overwrite template includes.
        for file in source.iterdir():
            if file.suffix in ['.asm','.inc','.h']:run(['docker','cp',str(file),name+':/work/'+file.name])
        header=output/'game_build.h'
        header.write_text('#define RGFX_SHA256 "'+variants['translated_image_sha256']+'"\n#define RGFX_ALTERNATE_SHA256 "'+variants['alternate_image_sha256']+'"\n')
        run(['docker','cp',str(header),name+':/work/game_build.h'])
        report['rust_runtime']=stage_rust_runtime(name,rust_runtime)
        report['rust_host']=stage_rust_runtime(name,rust_host)
        report["sr_heap"]=stage_sr_heap(name)
        report['sr_audio']=stage_sr_audio(name)
        setup=time.monotonic()
        with (output/'build.log').open('w') as log:
            run(['docker','exec',name,'timeout','240','sh','-c','sh /linux/build_backend.sh && sh /linux/build_runtime.sh'],stdout=log,stderr=subprocess.STDOUT)
        report['build_test_seconds']=round(time.monotonic()-setup,3)
        report['original_function_tests_verified']=True
        for file in ['redguard-native','libredguard-glide.so','linux.map','linux-symbols.txt','linux-dependencies.txt']:
            run(['docker','cp',name+':/work/'+file,str(output/file)])
        report['exe_sha256']=hashlib.sha256((output/'redguard-native').read_bytes()).hexdigest()
        report['renderer_sha256']=hashlib.sha256((output/'libredguard-glide.so').read_bytes()).hexdigest()
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
            report['gameplay_probes']={}
            actions=['continue','new'] if a.gameplay=='both' else [a.gameplay]
            for action in actions:
                with (output/f'gameplay-{action}-runner.log').open('w') as log:
                    result=subprocess.run(['docker','exec',name,'timeout','-k','5',str(int(a.gameplay_seconds)+60),
                        'xvfb-run','-a','python3','/tests/probe_gameplay.py','--action',action,
                        '--seconds',str(a.gameplay_seconds)]+(['--controls'] if a.world_controls and action=='continue' else [])+(['--exercise'] if a.exercise and (a.gameplay!='both' or action=='new') else [])+(['--modal-ui'] if a.modal_ui and action=='continue' else [])+
                        (['--user-dir',f'/work/gameplay-{action}-user'] if a.writable_gameplay else []),stdout=log,stderr=subprocess.STDOUT)
                for suffix in ['.log','.json','-menu.ppm','-latest.ppm','-world-before.ppm','-world-after.ppm','-world-book.ppm','-inventory.ppm','-inventory-close.ppm','-sword.ppm','-attack.ppm','-attack-2.ppm','-attack-3.ppm','-jump.ppm','-logbook.ppm','-logbook-close.ppm','-map.ppm','-map-close.ppm']:
                    file=f'gameplay-{action}'+suffix
                    subprocess.run(['docker','cp',name+':/work/'+file,str(output/file)],
                                   stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
                evidence=output/f'gameplay-{action}.json'
                report['gameplay_probes'][action]=json.loads(evidence.read_text()) if evidence.exists() else {'harness_error':'No probe report'}
                if result.returncode:raise RuntimeError(f'{action} probe harness failed; inspect exported evidence')
            report['status']='linux_menu_passed_gameplay_probed'

        if a.save_roundtrip:
            with (output/'save-roundtrip-runner.log').open('w') as log:
                saved=subprocess.run(['docker','exec',name,'timeout','-k','5','220','xvfb-run','-a',
                                      'python3','/tests/check_save_roundtrip.py'],stdout=log,stderr=subprocess.STDOUT)
            for file in ['save-roundtrip.json','save-roundtrip-1.log','save-roundtrip-2.log',
                         'save-roundtrip-1.json','save-roundtrip-2.json']:
                subprocess.run(['docker','cp',name+':/work/'+file,str(output/file)],
                               stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            if saved.returncode:raise RuntimeError('Original autosave/reload regression failed; inspect exported evidence')
            report['linux_original_save_reload_verified']=True

        if a.movie_skip:
            with (output/'movie-runner.log').open('w') as log:
                movie=subprocess.run(['docker','exec','-e','RG_MOVIE_PCM_CAPTURE=/work/movie-source.pcm',name,'timeout','-k','5','100','xvfb-run','-a',
                    'python3','/tests/probe_gameplay.py','--action','new','--movie-skip','--seconds','50',
                    '--user-dir','/work/movie-user']+(['--cd-image','/media/game.gog'] if a.cd_image else []),stdout=log,stderr=subprocess.STDOUT)
            for suffix in ['.json','.log','-movie-skip-0.ppm','-movie-skip-1.ppm','-movie-skip-2.ppm','-movie-skip-3.ppm']:
                subprocess.run(['docker','cp',name+':/work/gameplay-new'+suffix,str(output/('movie'+suffix))],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            evidence=output/'movie.json'
            report['movie_probe']=json.loads(evidence.read_text()) if evidence.exists() else {'harness_error':'No movie report'}
            if movie.returncode:raise RuntimeError('Original movie/CD-image probe failed; inspect exported evidence')
            report['movie_video_skip_to_ship_verified']=True
            report['movie_audio_mixed_verified']=report['movie_probe'].get('movie_audio_mixed_verified',False)
            if a.sound and not report['movie_audio_mixed_verified']:raise RuntimeError('Movie audio did not mix nonzero PCM while video advanced')
            report['movie_audio_verified']=False  # Audible hardware playback remains separate.
            report['cd_image_files_verified']=bool(a.cd_image)
            report['installed_gog_movies_verified']=a.installed_gog
            if a.installed_gog and 'GOG movies: using bundled installed data' not in (output/'movie.log').read_text():
                raise RuntimeError('Movie probe did not discover the installed GOG data')
            if a.sound:
                with (output/'movie-pcm-reference.log').open('w') as log:
                    run(['docker','exec',name,'python3','/tests/check_movie_pcm.py'],stdout=log,stderr=subprocess.STDOUT)
                run(['docker','cp',name+':/work/movie-pcm-reference.json',str(output/'movie-pcm-reference.json')])
                report['original_movie_pcm_reference_verified']=True

        report['tool_versions']=subprocess.check_output(['docker','exec',name,'dpkg-query','-W','gcc','g++','nasm','libsdl2-dev','libsdl2-mixer-dev','libssl-dev','libgl1-mesa-dri'],text=True).splitlines()
    except Exception:
        report['status']='failed'
        # Preserve the real game boundary even when the menu assertion fails.
        for file in ['linux-menu.log','linux-menu-before.ppm','linux-menu-options.ppm','linux-menu-return.ppm']:
            subprocess.run(['docker','cp',name+':/work/'+file,str(output/file)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        raise
    finally:
        report['elapsed_seconds']=round(time.monotonic()-began,3)
        if a.keep_container:report['development_container']=name
        else:subprocess.run(['docker','rm','-f',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
        print(output,flush=True)

if __name__=='__main__':main()
