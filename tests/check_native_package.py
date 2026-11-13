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
    assert set(archive.namelist())=={'redguard-native.exe','redguard-glide.dll','libwinpthread-1.dll','SDL2.dll','SDL2_mixer.dll','LICENSES.txt'}
    archive.extractall(root)
# The program is started directly: there is no launcher script. It asks for the game folder when
# none is given; here the folder is given the way a shortcut or terminal would, at a real Wine D: path
# for the drive layout and at a spaced Z: path otherwise.
example_game=r'D:\Games\GOG Galaxy\Redguard\Redguard'
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
    game_argument=example_game+'\\.'
else:
    game_argument='Z:\\work\\menu scenario\\.'
user_argument=r'Z:\work\extracted native test\user-data'
program=['/usr/lib/wine/wine64',str(root/'redguard-native.exe')]
explicit=program+['--game-dir',game_argument,'--user-dir',user_argument]
environment=dict(os.environ,WINEDEBUG='-all',WINEPREFIX='/tmp/rg-package-wine',
                 LIBGL_ALWAYS_SOFTWARE='1',SDL_AUDIODRIVER='dummy',RG_CAPTURE_DIR='Z:\\work')
with (root/'native-test.log').open('w') as output:
    process=subprocess.Popen(explicit,cwd=root,env=environment,stdin=subprocess.DEVNULL,
                             stdout=output,stderr=subprocess.STDOUT,text=True)
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
        process.wait(timeout=25)
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
assert 'Recognized RGFX.EXE variant=0' in log or 'Recognized RGFX.EXE variant=1' in log
assert 'Native Glide window 1024x768 opened=1' in log
assert 'Isolated native user storage:' in log
if a.game_on_drive_d:assert 'getcwd result=C:\\' in log
assert 'Native audio device=dummy' in log
assert 'CD music unavailable:' in log
assert ('GOG movies: using bundled installed data' in log)==a.installed_gog
assert ('Native CD image mounted' in log)==a.installed_gog
assert (root/'user-data/.redguard-storage-v1').is_file()
if a.expected_exit==0:
    assert 'BLOCKED' not in log
    assert 'reason=original-dos-termination status=0' in log
report=dict(zip_sha256=hashlib.sha256(Path('/work/runtime.zip').read_bytes()).hexdigest(),
            exe_sha256=hashlib.sha256((root/'redguard-native.exe').read_bytes()).hexdigest(),
            host_game_drive_d=a.game_on_drive_d,no_launcher_script=True,requested_cd_image=a.cd_image,launcher_cd_image_mounted='Native CD image mounted' in log,cd_image_mounted=False,uploaded_image_recognized=True,cmd_exit_code=result.returncode,
            frame_exported=Path('/work/game-frame-latest.ppm').exists(),
            wine_menu_sequence_verified=True,menu_verified=False,native_windows_verified=False)
if a.game_on_drive_d:
    report['physical_d_layout_verified']=not scenario.is_symlink() and scenario==selected
if a.jump_cycle:
    if a.fresh_start:shutil.rmtree(scenario/'SAVEGAME')
    jump=subprocess.run(['python3','/tests/probe_gameplay.py','--wine','--runtime-dir',str(root),
        '--game-dir',str(scenario),'--user-dir',str(root/'jump-user'),'--action',a.jump_action,
        '--seconds','40','--jump-cycle','--uncaptured']+(['--fresh-start'] if a.fresh_start else [])+(['--window-close'] if a.window_close else [])+(['--wine-game-dir',example_game] if a.game_on_drive_d else []),env=environment,timeout=80)
    assert jump.returncode==0,f'Original jump/close cycle failed; see gameplay-{a.jump_action}.json/log'
    report['jump_cycle_verified']=True
    if a.window_close:report['window_close_verified']=True
    if a.fresh_start:report['fresh_start_jump_cycle_verified']=True
if a.cd_image or a.loose_movie or a.installed_gog:
    if a.movie_pcm_sha256:
        environment['RG_MOVIE_PCM_CAPTURE']='Z:\\work\\package-movie-source.pcm'
    movie=subprocess.run(['python3','/tests/probe_gameplay.py','--wine','--runtime-dir',str(root),
        '--game-dir',str(scenario),'--user-dir',str(root/'movie-user'),'--action','new',
        '--seconds','50','--movie-skip']+(['--cd-image',a.cd_image] if a.cd_image else [])+(['--wine-game-dir',example_game] if a.game_on_drive_d else []),env=environment,timeout=100)
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
# Started with no arguments, the program looks for the game itself. First launch: REDGUARD_DIR names
# it (the next steps of the search are covered by unit tests); the program then stores the folder. Second
# launch: nothing is given and the stored folder is used.
def start_without_arguments(name,extra_environment):
    log_file=root/(name+'.log')
    with log_file.open('w') as output:
        running=subprocess.Popen(program,cwd=root,env=dict(environment,**extra_environment),
                                 stdin=subprocess.DEVNULL,stdout=output,stderr=subprocess.STDOUT,text=True)
        try:
            deadline=time.monotonic()+40
            while 'Verified original game directory' not in log_file.read_text():
                assert running.poll() is None,name+': the program did not start'
                assert time.monotonic()<deadline,name+': no start'
                time.sleep(0.2)
        finally:
            running.kill();running.wait()
            subprocess.run(['/usr/lib/wine/wineserver64','-k'],env=environment,check=False)
    return log_file.read_text()
game_for_environment=example_game if a.game_on_drive_d else 'Z:\\work\\menu scenario'
first=start_without_arguments('game-folder-environment',{'REDGUARD_DIR':game_for_environment})
assert 'Game folder found without asking' in first
stored=list(Path('/tmp/rg-package-wine/drive_c/users').glob('*/AppData/Roaming/maiko/**/install.json'))
assert len(stored)==1,stored
second=start_without_arguments('game-folder-remembered',{})
assert 'Game folder found without asking' in second
report['stored_game_folder_start_verified']=True
# A bad explicit folder must end with exit code 1 and a message, not hang.
missing=subprocess.run(program+['--game-dir',r'Z:\work\missing game'],cwd=root,
                       env=environment,stdin=subprocess.DEVNULL,text=True,stdout=subprocess.PIPE,
                       stderr=subprocess.STDOUT,timeout=20)
assert missing.returncode==1 and 'selection failed' in missing.stdout
report['missing_game_path_saved_exit']=1
print('PASS actual extracted ZIP started directly, uploaded RGFX, remembered game folder, spaced runtime path and missing-path exit',flush=True)

if a.cd_image:
    missing=subprocess.run(['/usr/lib/wine/wine64',str(root/'redguard-native.exe'),
        '--game-dir',example_game if a.game_on_drive_d else 'Z:'+str(scenario).replace('/','\\'),'--user-dir',r'Z:\work\invalid-media-user',
        '--cd-image',r'Z:\media\missing image.gog'],cwd=root,env=environment,text=True,
        stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=20)
    assert missing.returncode==1 and 'CD image unavailable or invalid:' in missing.stdout
    Path('/work/package-invalid-cd.log').write_text(missing.stdout)
    report['missing_cd_image_cli_exit']=1
    print('PASS optional CLI rejects missing media')

Path('/work/package-verification.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2),flush=True)
