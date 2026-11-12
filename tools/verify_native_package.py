#!/usr/bin/env python3
"""Exercise an actual seven-file Windows ZIP before publishing it."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time
import uuid
from .build_windows import ROOT,IMAGE,ensure_tool_image,run

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('package',type=Path)
    p.add_argument('--build',type=Path,required=True)
    p.add_argument('--install',type=Path,required=True)
    p.add_argument('--menu-autosave',type=Path,required=True)
    p.add_argument('--test-image',type=Path,required=True)
    p.add_argument('--jump-cycle',action='store_true')
    p.add_argument('--jump-action',choices=['continue','new'],default='continue')
    p.add_argument('--window-close',action='store_true')
    p.add_argument('--fresh-start',action='store_true')
    p.add_argument('--cd-image',type=Path,help='Optional owned image for the additional movie test')
    p.add_argument('--loose-movie',type=Path,help='Owned INTRO.SMK fixture to test installed-file playback without an image')
    p.add_argument('--pcm-reference',type=Path,help='Independent PCM report, required with --cd-image')
    a=p.parse_args()
    began=time.monotonic()
    if a.cd_image and a.loose_movie:p.error('Choose image or loose-file media, not both')
    if bool(a.cd_image or a.loose_movie)!=bool(a.pcm_reference):
        p.error('Supply media and --pcm-reference together, or omit both for the no-media menu test')
    if a.window_close and not a.jump_cycle:p.error('--window-close requires --jump-cycle')
    if a.fresh_start and (not a.jump_cycle or a.jump_action!='new' or a.cd_image or a.loose_movie):p.error('--fresh-start requires --jump-cycle --jump-action new without movie media')
    for path in [a.install]+([a.cd_image] if a.cd_image else []):
        if not path.resolve().is_relative_to('/home/vibepod/workspaces'):
            p.error('Docker mounts must resolve under /home/vibepod/workspaces')
    reference=json.loads(a.pcm_reference.read_text()) if a.pcm_reference else None
    if reference and (reference.get('original_pcm_reference_verified') is not True or reference.get('bytes_compared')!=32768):
        p.error('Requires an independent32768-byte PCM comparison report')
    directory=a.package.resolve();archive=directory/'redguard-native-startup-test.zip'
    (directory/'package-verification.json').unlink(missing_ok=True)
    name='rg-package-'+uuid.uuid4().hex[:10]
    invocation=dict(container=name,game_mount_readonly=True,cd_mount_readonly=bool(a.cd_image),
                    zip_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                    test_image_sha256=hashlib.sha256(a.test_image.read_bytes()).hexdigest(),
                    pcm_reference_report_sha256=hashlib.sha256(a.pcm_reference.read_bytes()).hexdigest() if reference else None,
                    pcm_reference_sha256=reference['sha256'] if reference else None)
    try:
        invocation['toolchain']=ensure_tool_image()
        run(['docker','run','-d','--network','none','--memory','3g','--memory-swap','3g','--cpus','2',
             '--env','SDL_AUDIODRIVER=dummy','--name',name,
             '--mount',f'type=bind,src={a.install.resolve()},dst=/game,readonly',
             *(['--mount',f'type=bind,src={a.cd_image.resolve()},dst=/media/owned CD image.gog,readonly'] if a.cd_image else []),
             IMAGE,'sleep','240'])
        run(['docker','cp',str(ROOT/'tests')+'/.',name+':/tests'])
        for source,destination in [(archive,'runtime.zip'),(a.test_image,'user-RGFX.EXE'),
                                   (a.menu_autosave,'menu-autosave.sav'),
                                   (a.build/'native-symbols.txt','native-symbols.txt')]:
            run(['docker','cp',str(source),name+':/work/'+destination])
        if a.loose_movie:run(['docker','cp',str(a.loose_movie.resolve()),name+':/work/INTRO.SMK'])
        run(['docker','exec',name,'x86_64-w64-mingw32-gcc','-Wall','-Wextra','-Werror','-O2',
             '/tests/menu_keys.c','-o','/work/menu-keys.exe'])
        command=['docker','exec',name,'timeout','-k','5','150','xvfb-run','-a','python3',
                 '/tests/check_native_package.py','--expected-exit','0','--game-on-drive-d']
        if a.jump_cycle:command+=['--jump-cycle','--jump-action',a.jump_action]
        if a.window_close:command+=['--window-close']
        if a.fresh_start:command+=['--fresh-start']
        if a.cd_image:
            command+=['--cd-image','/media/owned CD image.gog','--movie-pcm-sha256',reference['sha256']]
        if a.loose_movie:
            command+=['--loose-movie','/work/INTRO.SMK','--movie-pcm-sha256',reference['sha256']]
        invocation['command']=command
        with (directory/'package-test.log').open('w') as log:
            run(command,stdout=log,stderr=subprocess.STDOUT)
    finally:
        invocation['seconds']=round(time.monotonic()-began,3)
        (directory/'package-invocation.json').write_text(json.dumps(invocation,indent=2)+'\n')
        for file in ['package-verification.json','package-movie.json','package-movie.log',
                     'package-native-test.log','package-invalid-cd.log','gameplay-continue.json','gameplay-continue.log','gameplay-new.json','gameplay-new.log']:
            subprocess.run(['docker','cp',name+':/work/'+file,str(directory/file)],
                           stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        subprocess.run(['docker','rm','-f',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    print((directory/'package-verification.json').read_text())

if __name__=='__main__':main()
