"""Run the actual extracted runtime ZIP in an isolated game scenario.

Container prerequisites: read-only /game, /work/runtime.zip, actual uploaded
/work/user-RGFX.EXE, /work/menu-autosave.sav, and tests in /tests.
No executable or DLL in the ZIP is replaced. Outputs stay under /work.
"""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import time
import shutil
import zipfile

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--game-on-drive-d',action='store_true',help='Expose the selected install on real host D: while original guest paths remain C:')
p.add_argument('--cd-image',help='Exercise the optional explicit movie CLI after the ordinary launcher test')
p.add_argument('--installed-gog',action='store_true',help='Reproduce the installed GOG movie layout and test automatic discovery')
p.add_argument('--loose-movie',help='Place an owned INTRO.SMK fixture in the disposable install and test without a CD mount')
p.add_argument('--jump-cycle',action='store_true')
p.add_argument('--jump-action',choices=['continue','new'],default='continue')
p.add_argument('--window-close',action='store_true')
p.add_argument('--fresh-start',action='store_true')
p.add_argument('--expected-exit',type=int,required=True)
p.add_argument('--movie-pcm-sha256',help='Independent reference SHA256 for the first32768 source PCM bytes')
a=p.parse_args()
for disposable in ['/work/menu-scenario','/work/menu scenario','/work/extracted native test']:
    if Path(disposable).exists(): shutil.rmtree(disposable)
for capture in Path('/work').glob('game-frame-*.ppm'): capture.unlink()
subprocess.run(['python3','/tests/prepare_menu_scenario.py','--sound','--redbook'],check=True)
scenario=Path('/work/menu-scenario').rename('/work/menu scenario')
if a.installed_gog and not a.game_on_drive_d:
    Path('/work/game.gog').symlink_to('/media/owned CD image.gog')
image=scenario/'RGFX.EXE'
image.unlink()
image.write_bytes(Path('/work/user-RGFX.EXE').read_bytes())
if a.loose_movie:shutil.copyfile(a.loose_movie,scenario/'INTRO.SMK')
root=Path('/work/extracted native test')
with zipfile.ZipFile('/work/runtime.zip') as archive:
    assert set(archive.namelist())=={'redguard-native.exe','redguard-glide.dll','libwinpthread-1.dll','SDL2.dll','SDL2_mixer.dll','RUN.cmd','LICENSES.txt'}
    archive.extractall(root)
cmd=root/'RUN.cmd'
canonical_bytes=cmd.read_bytes()
canonical=cmd.read_text()
def setting(source,name,value):
    updated,count=re.subn(r'^set "'+re.escape(name)+r'=[^"\r\n]*"$',
                          lambda _:f'set "{name}={value}"',source,flags=re.MULTILINE)
    assert count==1,(name,count)
    return updated

def write_cmd(source):
    cmd.write_bytes(source.replace('\r\n','\n').replace('\n','\r\n').encode())

# Check the shipped preset, then exercise it unchanged at a real Wine D: path.
default_game=r'D:\Games\GOG Galaxy\Redguard\Redguard'
assert f'set "GAME_DIR={default_game}"' in canonical
assert 'CD_IMAGE' not in canonical and '--cd-image' not in canonical
configured=canonical
if a.game_on_drive_d:
    prefix=Path('/tmp/rg-package-wine')
    # Initialize before mapping D:. Wine10's GUI wineboot update hangs in the
    # amd64-only tool image; a headless command creates the same prefix without
    # starting its desktop setup. The real game still runs on the Xvfb display.
    setup_environment=dict(os.environ,WINEDEBUG='-all',WINEPREFIX=str(prefix))
    setup_environment.pop('DISPLAY',None)
    subprocess.run(['/usr/lib/wine/wine64','cmd','/c','exit'],env=setup_environment,check=True,timeout=30)
    # Restart the server so its desktop uses the game display, not the headless
    # initialization environment. Otherwise CreateWindow fails at mouse reset.
    subprocess.run(['/usr/lib/wine/wineserver64','-k'],env=setup_environment,check=True,timeout=10)
    subprocess.run(['/usr/lib/wine/wineserver64','-w'],env=setup_environment,check=True,timeout=10)
    drive=prefix/'dosdevices/d:'
    if drive.is_symlink(): drive.unlink()
    drive_root=Path('/work/host-drive-d')
    selected=drive_root/'Games/GOG Galaxy/Redguard/Redguard'
    selected.parent.mkdir(parents=True,exist_ok=True)
    scenario=scenario.rename(selected)
    if a.installed_gog:
        (selected.parent/'game.gog').symlink_to('/media/owned CD image.gog')
        assert not selected.is_symlink() and selected.parent.joinpath('game.gog').is_file()
    drive.symlink_to(drive_root)
else:
    configured=setting(configured,'GAME_DIR','Z:\\work\\menu scenario\\')
if configured!=canonical:
    write_cmd(configured)
assert not a.game_on_drive_d or cmd.read_bytes()==canonical_bytes

environment=dict(os.environ,WINEDEBUG='-all',WINEPREFIX='/tmp/rg-package-wine',
                 LIBGL_ALWAYS_SOFTWARE='1',SDL_AUDIODRIVER='dummy',RG_CAPTURE_DIR='Z:\\work')
with Path('/work/package-cmd-output.log').open('w') as output:
    process=subprocess.Popen(['/usr/lib/wine/wine64','cmd','/c','RUN.cmd'],cwd=root,
                             env=environment,stdin=subprocess.PIPE,stdout=output,stderr=subprocess.STDOUT,text=True)
    def key(name):
        subprocess.run(['/usr/lib/wine/wine64','/work/menu-keys.exe',name],env=environment,check=True,timeout=10)
        time.sleep(0.5)
    def frame(name):
        shutil.copyfile('/work/game-frame-latest.ppm',f'/work/menu-{name}.ppm')
    try:
        deadline=time.monotonic()+50
        while True:
            logpath=root/'native-test.log'
            if logpath.exists() and 'CLIP entry=7' in logpath.read_text(): break
            if process.poll() is not None: raise RuntimeError('Game ended before book initialization')
            if time.monotonic()>deadline: raise TimeoutError('No book initialization trace')
            time.sleep(0.1)
        time.sleep(4)
        frame('before')
        key('up');key('up');key('enter')
        time.sleep(4);frame('options')
        key('escape');time.sleep(4);frame('return')
        key('down');key('down');key('down');key('enter')
        process.communicate('\n',timeout=25)
    finally:
        if (root/'native-test.log').exists():
            shutil.copyfile(root/'native-test.log','/work/package-native-test.log')
        if process.poll() is None:
            process.kill();process.wait()
            subprocess.run(['/usr/lib/wine/wineserver64','-k'],env=environment,check=False)
    result=process

log=(root/'native-test.log').read_text()
Path('/work/package-native-test.log').write_text(log)
assert result.returncode==a.expected_exit,(result.returncode,a.expected_exit)
assert f'Exit code: {a.expected_exit}' in log
assert 'Recognized RGFX.EXE variant=0' in log or 'Recognized RGFX.EXE variant=1' in log
assert 'Native Glide window 1024x768 opened=1' in log
assert 'Isolated native user storage:' in log
if a.game_on_drive_d:assert 'getcwd result=C:\\' in log
assert 'Native audio device=dummy' in log
assert 'Native CD audio unavailable:' in log
assert ('GOG movies: using bundled installed data' in log)==a.installed_gog
assert ('Native CD image mounted' in log)==a.installed_gog
assert (root/'user-data/.redguard-storage-v1').is_file()
if a.expected_exit==0:
    assert 'BLOCKED' not in log
    assert 'Original DOS termination status=0; exiting native process' in log
report=dict(zip_sha256=hashlib.sha256(Path('/work/runtime.zip').read_bytes()).hexdigest(),
            exe_sha256=hashlib.sha256((root/'redguard-native.exe').read_bytes()).hexdigest(),
            host_game_drive_d=a.game_on_drive_d,shipped_default_game_dir=default_game,unmodified_cmd_verified=cmd.read_bytes()==canonical_bytes,requested_cd_image=a.cd_image,launcher_cd_image_mounted='Native CD image mounted' in log,cd_image_mounted=False,uploaded_image_recognized=True,cmd_exit_code=result.returncode,
            frame_exported=Path('/work/game-frame-latest.ppm').exists(),
            wine_menu_sequence_verified=True,menu_verified=False,native_windows_verified=False)
if a.game_on_drive_d:
    report['physical_d_layout_verified']=not scenario.is_symlink() and scenario==selected
if a.jump_cycle:
    if a.fresh_start:shutil.rmtree(scenario/'SAVEGAME')
    jump=subprocess.run(['python3','/tests/probe_gameplay.py','--wine','--runtime-dir',str(root),
        '--game-dir',str(scenario),'--user-dir',str(root/'jump-user'),'--action',a.jump_action,
        '--seconds','40','--jump-cycle','--uncaptured']+(['--fresh-start'] if a.fresh_start else [])+(['--window-close'] if a.window_close else [])+(['--wine-game-dir',default_game] if a.game_on_drive_d else []),env=environment,timeout=80)
    assert jump.returncode==0,f'Original jump/close cycle failed; see gameplay-{a.jump_action}.json/log'
    report['jump_cycle_verified']=True
    if a.window_close:report['window_close_verified']=True
    if a.fresh_start:report['fresh_start_jump_cycle_verified']=True
if a.cd_image or a.loose_movie or a.installed_gog:
    if a.movie_pcm_sha256:
        environment['RG_MOVIE_PCM_CAPTURE']='Z:\\work\\package-movie-source.pcm'
    movie=subprocess.run(['python3','/tests/probe_gameplay.py','--wine','--runtime-dir',str(root),
        '--game-dir',str(scenario),'--user-dir',str(root/'movie-user'),'--action','new',
        '--seconds','50','--movie-skip']+(['--cd-image',a.cd_image] if a.cd_image else [])+(['--wine-game-dir',default_game] if a.game_on_drive_d else []),env=environment,timeout=100)
    evidence=json.loads(Path('/work/gameplay-new.json').read_text())
    shutil.copyfile('/work/gameplay-new.json','/work/package-movie.json')
    shutil.copyfile('/work/gameplay-new.log','/work/package-movie.log')
    assert movie.returncode==0 and evidence.get('movie_skip_to_world_verified') and evidence.get('movie_audio_mixed_verified') and evidence.get('natural_exit')==0
    assert evidence['exe_sha256']==report['exe_sha256']
    report['actual_zip_movie_skip_to_ship_verified']=True
    movie_log=Path('/work/package-movie.log').read_text()
    report['cd_image_mounted']='Native CD image mounted' in movie_log
    if a.installed_gog:
        assert 'GOG movies: using bundled installed data' in movie_log
        report['installed_gog_movies_verified']=True
    if a.loose_movie:
        assert 'Native CD image mounted' not in Path('/work/package-movie.log').read_text()
        assert 'movie loose d:\\intro.smk' in Path('/work/package-movie.log').read_text().lower()
        report['actual_zip_loose_movie_without_image_verified']=True
        report['loose_movie_files_verified']=True
    report['movie_audio_mixed_verified']=True
    report['movie_audio_verified']=False
    if a.movie_pcm_sha256:
        pcm=Path('/work/package-movie-source.pcm').read_bytes()[:32768]
        assert len(pcm)==32768 and hashlib.sha256(pcm).hexdigest()==a.movie_pcm_sha256
        report['original_movie_pcm_reference_verified']=True
        report['movie_pcm_reference_bytes']=len(pcm)
        report['movie_pcm_reference_sha256']=a.movie_pcm_sha256
# The actual launcher also needs a useful saved log when the path is invalid.
write_cmd(setting(canonical,'GAME_DIR',r'Z:\work\missing game'))
missing=subprocess.run(['/usr/lib/wine/wine64','cmd','/c','RUN.cmd'],cwd=root,
                       env=environment,input='\n',text=True,stdout=subprocess.PIPE,
                       stderr=subprocess.STDOUT,timeout=20)
assert missing.returncode==1
assert 'Exit code: 1' in (root/'native-test.log').read_text()
report['missing_game_path_saved_exit']=1
print('PASS actual extracted ZIP, uploaded RGFX, shipped default game path, spaced runtime path and missing-path saved exit',flush=True)

if a.cd_image:
    missing=subprocess.run(['/usr/lib/wine/wine64',str(root/'redguard-native.exe'),
        '--game-dir',default_game if a.game_on_drive_d else 'Z:'+str(scenario).replace('/','\\'),'--user-dir',r'Z:\work\invalid-media-user',
        '--cd-image',r'Z:\media\missing image.gog'],cwd=root,env=environment,text=True,
        stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=20)
    assert missing.returncode==1 and 'CD image unavailable or invalid:' in missing.stdout
    Path('/work/package-invalid-cd.log').write_text(missing.stdout)
    report['missing_cd_image_cli_exit']=1
    print('PASS optional CLI rejects missing media; normal CMD has no CD setting')

Path('/work/package-verification.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2),flush=True)
