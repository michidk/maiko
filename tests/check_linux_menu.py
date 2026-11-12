"""Exercise original Linux menu with X11 keyboard events, without Wine."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

root=Path('/work')
environment=dict(os.environ,LIBGL_ALWAYS_SOFTWARE='1',RG_CAPTURE_DIR='/work')
for file in root.glob('game-frame-*.ppm'): file.unlink()
with (root/'linux-menu.log').open('w') as log:
    process=subprocess.Popen(['/work/redguard-native','--game-dir','/work/menu-scenario'],env=environment,stdout=log,stderr=subprocess.STDOUT)
    try:
        deadline=time.monotonic()+40
        while 'CLIP entry=7' not in (root/'linux-menu.log').read_text():
            if process.poll() is not None: raise RuntimeError('Original game stopped before book rendering')
            if time.monotonic()>deadline: raise TimeoutError('Original book initialization')
            time.sleep(.1)
        time.sleep(3)
        window=subprocess.check_output(['xdotool','search','--name','^Redguard native experiment$'],text=True,timeout=5).strip().splitlines()[0]
        subprocess.run(['xdotool','windowfocus','--sync',window],check=True,timeout=5)
        def frame(name): shutil.copyfile(root/'game-frame-latest.ppm',root/f'linux-menu-{name}.ppm')
        def key(name):
            subprocess.run(['xdotool','keydown',name],check=True,timeout=5)
            time.sleep(.15)
            subprocess.run(['xdotool','keyup',name],check=True,timeout=5)
            time.sleep(.5)
        frame('before')
        key('Up');key('Up');key('Return');time.sleep(3);frame('options')
        key('Escape');time.sleep(3);frame('return')
        key('Down');key('Down');key('Down');key('Return')
        code=process.wait(timeout=15)
        assert code==0,code
    finally:
        if process.poll() is None: process.kill();process.wait()
text=(root/'linux-menu.log').read_text()
assert 'BLOCKED' not in text
assert 'Original DOS termination status=0; exiting native process' in text
assert 'Native Glide window 1024x768 opened=1' in text
result=dict(exe_sha256=hashlib.sha256((root/'redguard-native').read_bytes()).hexdigest(),
            wine_used=False,original_quit_exit=code,keyboard_events='X11 XTest through SDL',
            menu_navigation_exercised=True,visual_review_required=True,native_windows_verified=False)
(root/'linux-menu-verification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
print('PASS native Linux original menu key sequence and clean original Quit without Wine')
