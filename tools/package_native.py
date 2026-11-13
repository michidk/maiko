#!/usr/bin/env python3
"""Package an observed native test with separate Wine and Windows acceptance."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile
import shutil
from .package_media import movie_evidence
from .rust_licenses import notices as rust_dependency_notices

ROOT=Path(__file__).resolve().parents[1]
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
    try:
        media=movie_evidence(report)
    except ValueError as error:
        parser.error(str(error))
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
    notices=sorted((work/'notices').glob('*.txt'))
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
    if report.get('rust_runtime'):
        rustc=shutil.which('rustc') or str(Path.home()/'.cargo/bin/rustc')
        sysroot=Path(subprocess.check_output([rustc,'--print','sysroot'],cwd=ROOT,text=True).strip())
        library_notice=sysroot/'share/doc/rust/COPYRIGHT-library.html'
        licence+='===== Rust standard library and bundled dependencies =====\n'+library_notice.read_text()+'\n\n'
        licence+=rust_dependency_notices()+'\n'
    # Ship the smallest files that run: symbol tables and debug sections are build-machine
    # information that the game does not read (the unstripped build stays in the work folder for the
    # symbol map the tests use).
    strip=shutil.which('x86_64-w64-mingw32-strip') or shutil.which('strip')
    shipped=args.output/'.stripped'
    shipped.mkdir(exist_ok=True)
    with zipfile.ZipFile(destination,'w',zipfile.ZIP_DEFLATED,compresslevel=9) as package:
        for filename in ['redguard-native.exe','redguard-glide.dll','libwinpthread-1.dll','SDL2.dll','SDL2_mixer.dll']:
            source=work/filename
            if strip:
                subprocess.run([strip,'-s','-o',str(shipped/filename),str(source)],check=True)
                source=shipped/filename
            package.write(source,filename)
        package.writestr('LICENSES.txt',licence)
    shutil.rmtree(shipped)
    development=args.output/'redguard-development.zip'
    with zipfile.ZipFile(development,'w',zipfile.ZIP_DEFLATED) as package:
        package.write(work/'native-wine.log','wine-reference.log')
        package.writestr('build-report.json',json.dumps(report,indent=2)+'\n')
        package.writestr('LICENSES.txt',licence)
        package.writestr('sources/openglide-ng-'+BACKEND+'.tar.gz',gzip.compress(archive,mtime=0))
        for filename in ['patches/openglide-x64.patch','glide/build_backend.sh','tests/backend_smoke.cpp','tools/docker/Dockerfile']:
            package.write(ROOT/filename,'sources/'+filename)
        package.writestr('sources/README.txt','OpenGLide-ng upstream: https://github.com/eugenenuke/openglide-ng\n'
                         'Extract the source archive to /src, apply patches/openglide-x64.patch with patch -p1,\n'
                         'copy glide/build_backend.sh and tests/backend_smoke.cpp into /work, then run\n'
                         'sh /work/build_backend.sh\n'
                         'inside the supplied tools/docker/Dockerfile toolchain. The build script creates the SDK header.\n')
    digest=hashlib.sha256(destination.read_bytes()).hexdigest()
    source_digest=hashlib.sha256(development.read_bytes()).hexdigest()
    (args.output/'SHA256SUMS.txt').write_text(f'{digest}  {destination.name}\n{source_digest}  {development.name}\n')
    (args.output/'index.html').write_text(r'''<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Maiko for Windows</title><style>body{font:18px/1.6 system-ui;max-width:720px;margin:50px auto;padding:0 24px}</style>
<h1>Maiko for Windows</h1><p><a href="redguard-native-startup-test.zip" download>Download the Windows x64 ZIP</a> (<a href="SHA256SUMS.txt">SHA-256</a>)</p>
<p>Extract it into its own writable folder and double-click <code>redguard-native.exe</code>. It looks for your Redguard folder itself (its own folder, the folders around it, the standard GOG folders) and asks only if it finds nothing; it remembers the answer. Saves and settings go into a <code>user-data</code> folder beside the program. You need your own copy of Redguard; the ZIP contains no game files.</p>
<p>For fullscreen, a window size, skipping the intro or loading a save, add options to a shortcut to <code>redguard-native.exe</code>, for example <code>--fullscreen</code> or <code>--skip-intro --load-save 1</code>.</p>
<p><a href="redguard-development.zip">Renderer source and build evidence</a></p>''')
    (args.output/'movie-evidence.json').write_text(json.dumps(media,indent=2)+'\n')
    print(destination)
    print(digest)

if __name__=='__main__':
    main()
