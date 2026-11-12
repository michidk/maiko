#!/usr/bin/env python3
"""Package an observed native test with separate Wine and Windows acceptance."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile

ROOT=Path(__file__).resolve().parent
BACKEND='9f8a2cb73af3b6d26e9ec67bb9e1873d3e211726'

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('work',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    work=args.work.resolve()
    report=json.loads((work/'report.json').read_text())
    if not all(report.get(k) for k in ['native_crt_verified','native_config_verified','native_glide_window_verified']):
        parser.error('Requires an observed original CRT, configuration and Glide window run')
    if not report.get('image_variants'):
        parser.error('Requires a build with checked dual-image support and folder selection')
    if not report.get('wine_original_save_reload_verified'):
        parser.error('Requires original separate-process autosave/reload')
    exercise=report.get('wine_gameplay_probes',{}).get('new',{})
    steps={x['step']:x.get('state',{}) for x in exercise.get('exercise_snapshots',[])}
    if exercise.get('blocked',True) or not exercise.get('exercise_inputs_delivered') or steps.get('inventory',{}).get('ui_mode')!=28 or steps.get('inventory-close',{}).get('ui_mode')!=31:
        parser.error('Requires observed original inventory open/close through game input')
    if not report.get('movie_video_skip_to_ship_verified') or not report.get('cd_image_files_verified') or not report.get('movie_audio_mixed_verified'):
        parser.error('Requires original movie PCM/video/skip/ship evidence through the CD image adapter')
    executable=work/'redguard-native.exe'
    if hashlib.sha256(executable.read_bytes()).hexdigest()!=report.get('native_exe_sha256'):
        parser.error('Executable does not match the observed build')
    if hashlib.sha256((work/'redguard-glide.dll').read_bytes()).hexdigest()!=report['glide_backend']['dll_sha256']:
        parser.error('Renderer does not match the observed build')
    if not report.get('wine_original_audio_tests_verified'):
        parser.error('Requires the native sample-audio regression')
    for project,prefix in [('SDL','SDL2'),('SDL_mixer','SDL2_mixer')]:
        if hashlib.sha256((work/(prefix+'.dll')).read_bytes()).hexdigest()!=report['sdl_audio'][project]['dll_sha256']:
            parser.error('SDL runtime differs from the observed build')
    source=ROOT/'.cache/openglide-ng'
    archive=subprocess.check_output(['git','archive',BACKEND],cwd=source)
    args.output.mkdir(parents=True,exist_ok=True)
    destination=args.output/'redguard-native-startup-test.zip'
    launcher=(ROOT/'runtime/RUN.cmd').read_text().replace('\n','\r\n')
    notices=sorted((ROOT/'artifacts/launcher-check/notices').glob('*.txt'))
    if len(notices)!=6:
        parser.error('Expected the complete SR, GCC/MinGW and LGPL notices')
    licence='Redguard native experiment. Personal test; requires original game assets.\n'
    licence+='OpenGLide-ng '+BACKEND+' is a replaceable LGPL library.\n'
    licence+='Its corresponding source and build files are provided alongside this ZIP in redguard-development.zip.\n\n'
    for notice in notices:
        licence+='===== '+notice.name+' =====\n'+notice.read_text()+'\n\n'
    for project in ['SDL2','SDL2_mixer']:
        notice=ROOT/'.cache/sdl-windows'/(project+'-LICENSE.txt')
        licence+='===== '+project+' =====\n'+notice.read_text()+'\n\n'
    with zipfile.ZipFile(destination,'w',zipfile.ZIP_DEFLATED) as package:
        for filename in ['redguard-native.exe','redguard-glide.dll','libwinpthread-1.dll','SDL2.dll','SDL2_mixer.dll']:
            package.write(work/filename,filename)
        package.writestr('RUN.cmd',launcher)
        package.writestr('LICENSES.txt',licence)
    development=args.output/'redguard-development.zip'
    with zipfile.ZipFile(development,'w',zipfile.ZIP_DEFLATED) as package:
        package.write(work/'native-wine.log','wine-reference.log')
        package.writestr('build-report.json',json.dumps(report,indent=2)+'\n')
        package.writestr('LICENSES.txt',licence)
        package.writestr('sources/openglide-ng-'+BACKEND+'.tar.gz',gzip.compress(archive,mtime=0))
        for filename in ['patches/openglide-x64.patch','glide/build_backend.sh','glide/backend_smoke.cpp','Dockerfile']:
            package.write(ROOT/filename,'sources/'+filename)
        package.writestr('sources/README.txt','OpenGLide-ng upstream: https://github.com/eugenenuke/openglide-ng\n'
                         'Extract the source archive to /src, apply patches/openglide-x64.patch with patch -p1,\n'
                         'place glide/*.sh and glide/*.cpp in /work, and run sh /work/build_backend.sh\n'
                         'inside the supplied Dockerfile toolchain. The build script creates the SDK header.\n')
    digest=hashlib.sha256(destination.read_bytes()).hexdigest()
    source_digest=hashlib.sha256(development.read_bytes()).hexdigest()
    (args.output/'RUN.cmd').write_bytes(launcher.encode('utf-8'))
    (args.output/'SHA256SUMS.txt').write_text(f'{digest}  {destination.name}\n{source_digest}  {development.name}\n')
    (args.output/'index.html').write_text(r'''<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Redguard native gameplay test</title><style>body{font:18px/1.6 system-ui;max-width:720px;margin:50px auto;padding:0 24px}</style>
<h1>Redguard native gameplay experiment</h1><p><strong>The original book menu and bounded island gameplay run without DOSBox on Linux and under Wine.</strong> This is an incomplete experimental build. Real Windows acceptance and full-game completion remain unverified.</p>
<p><a href="redguard-native-startup-test.zip" download>Download Windows x64 test ZIP</a></p>
<p><a href="RUN.cmd" download>Download the editable RUN.cmd separately</a></p>
<p>Extract into a separate writable folder and run RUN.cmd. GAME_DIR is preset to D:\Games\GOG Galaxy\Redguard\Redguard, the folder containing RGFX.EXE. Keep all extracted runtime files together. Edit GAME_DIR only if your installation moves. Both inspected executable variants are supported. USER_DIR controls the separate save/config location.</p>
<p>Send back native-test.log after one short jump on the opening ship. This update adds bounded original jump and frame-timing samples and fixes native window-close cleanup. Endless rising reported on Windows, startup acceleration, combat completion and distant fog artifacts remain under investigation. The original startup can go straight into a fresh game when no usable autosave is found; the book-menu tests use a copied autosave.</p>
<p>RUN.cmd imports saves and settings once into its own user-data folder and writes there. Installed menu artwork and textures load directly from GAME_DIR. CD_IMAGE is empty by default. On Windows, root-level D: movie requests also search for loose SMK files in GAME_DIR and its MOVIES, ANIMS and VIDEO folders. The log reports missing files. This does not establish which cinematic files exist in your installation, and the package supplies no movie assets. The optional image adapter remains available for owners of compatible media. Original movie PCM mixes through SDL audio in the Linux and Wine tests. Audible hardware quality and synchronization remain unverified. CD music remains unsupported. Twelve diagnostic CPU traps remain in untranslated mixed-mode system code; reaching one stops the test. The graphics wrapper may write OpenGLid.ini/OpenGLid.log in the selected game folder.</p>
<p><a href="redguard-development.zip">Optional renderer source and build evidence</a></p>
<p><a href="SHA256SUMS.txt">ZIP SHA-256 checksum</a></p>''')
    print(destination)
    print(digest)

if __name__=='__main__':
    main()
