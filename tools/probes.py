"""Gameplay, save and movie probes shared by the Linux and Windows (Wine) builds.

Each runs a probe from tests/ in the build container, copies its evidence to `output` (also when it
fails) and raises when the probe failed. `wine` runs the Windows build under Wine.
"""
import json
import subprocess

from .toolchain import copy_out

GAMEPLAY_EVIDENCE = ['.log','.json','-menu.ppm','-latest.ppm','-world-before.ppm','-world-after.ppm','-world-book.ppm',
                     '-inventory.ppm','-inventory-close.ppm','-sword.ppm','-attack.ppm','-attack-2.ppm','-attack-3.ppm',
                     '-jump.ppm','-logbook.ppm','-logbook-close.ppm','-map.ppm','-map-close.ppm']


def _evidence(path, missing):
    return json.loads(path.read_text()) if path.exists() else {'harness_error':missing}


def gameplay(name, output, a, wine):
    """Continue and/or New Game probes; returns each probe's report."""
    reports={}
    for action in (['continue','new'] if a.gameplay=='both' else [a.gameplay]):
        command=['docker','exec',name,'timeout','-k','5',str(int(a.gameplay_seconds)+60),
                 'xvfb-run','-a','python3','/tests/probe_gameplay.py','--action',action,'--seconds',str(a.gameplay_seconds)]
        command+=(['--wine'] if wine else [])
        command+=(['--controls'] if a.world_controls and action=='continue' else [])
        command+=(['--exercise'] if a.exercise and (a.gameplay!='both' or action=='new') else [])
        command+=(['--modal-ui'] if a.modal_ui and action=='continue' else [])
        command+=(['--user-dir',f'/work/gameplay-{action}-user'] if a.writable_gameplay else [])
        with (output/f'gameplay-{action}-runner.log').open('w') as log:
            result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
        copy_out(name,[(f'/work/gameplay-{action}{suffix}',output/f'gameplay-{action}{suffix}') for suffix in GAMEPLAY_EVIDENCE])
        reports[action]=_evidence(output/f'gameplay-{action}.json','No probe report')
        if result.returncode:raise RuntimeError(f'{action} gameplay probe failed; inspect exported evidence')
    return reports


def save_roundtrip(name, output, wine):
    """Autosave, quit and reload through the original save code."""
    with (output/'save-roundtrip-runner.log').open('w') as log:
        result=subprocess.run(['docker','exec',name,'timeout','-k','5','220','xvfb-run','-a','python3',
                               '/tests/check_save_roundtrip.py']+(['--wine'] if wine else []),stdout=log,stderr=subprocess.STDOUT)
    copy_out(name,[(f'/work/{file}',output/file) for file in
                   ['save-roundtrip.json','save-roundtrip-1.log','save-roundtrip-2.log','save-roundtrip-1.json','save-roundtrip-2.json']])
    if result.returncode:raise RuntimeError('Original autosave/reload regression failed; inspect exported evidence')


def movie_skip(name, output, a, wine, report, environment=()):
    """New Game through the intro movie, skipping it to the ship; fills the movie report keys."""
    with (output/'movie-runner.log').open('w') as log:
        result=subprocess.run(['docker','exec',*environment,name,'timeout','-k','5','100','xvfb-run','-a',
            'python3','/tests/probe_gameplay.py','--action','new','--movie-skip','--seconds','50','--user-dir','/work/movie-user']
            +(['--cd-image','/media/game.gog'] if a.cd_image else [])+(['--wine'] if wine else []),stdout=log,stderr=subprocess.STDOUT)
    copy_out(name,[(f'/work/gameplay-new{suffix}',output/f'movie{suffix}') for suffix in
                   ['.json','.log','-movie-skip-0.ppm','-movie-skip-1.ppm','-movie-skip-2.ppm','-movie-skip-3.ppm']])
    report['movie_probe']=_evidence(output/'movie.json','No movie report')
    if result.returncode:raise RuntimeError('Original movie/CD-image probe failed; inspect exported evidence')
    report['movie_video_skip_to_ship_verified']=True
    report['movie_audio_mixed_verified']=report['movie_probe'].get('movie_audio_mixed_verified',False)
    if a.sound and not report['movie_audio_mixed_verified']:raise RuntimeError('Movie audio did not mix nonzero PCM while video advanced')
    report['movie_audio_verified']=False  # Audible hardware playback remains separate.
    report['cd_image_files_verified']=bool(a.cd_image)
    report['installed_gog_movies_verified']=a.installed_gog
    if a.installed_gog and 'GOG movies: using bundled installed data' not in (output/'movie.log').read_text():
        raise RuntimeError('Movie probe did not discover the installed GOG data')
