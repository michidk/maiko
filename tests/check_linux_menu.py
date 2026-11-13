"""Exercise original Linux menu with X11 keyboard events, without Wine."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
from book_menu import BookMenu

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--fresh-start',action='store_true')
args=parser.parse_args()
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
        frame_hashes={}
        def frame(name):
            # Original input is disabled during book camera/page turns. Software
            # GL can take much longer than three seconds to finish those turns.
            deadline=time.monotonic()+18
            previous=None;stamp=None
            while time.monotonic()<deadline:
                if process.poll() is not None:raise RuntimeError('Game ended while waiting for '+name)
                latest=root/'game-frame-latest.ppm'
                current=latest.stat().st_mtime_ns
                if current!=stamp:
                    data=latest.read_bytes();parts=data.split(b'\n',3)
                    if len(parts)==4 and parts[:3]==[b'P6',b'1024 768',b'255'] and len(parts[3])==1024*768*3:
                        digest=hashlib.sha256(data).hexdigest()
                        print('Observed book capture',name,digest,flush=True)
                        if digest==previous:
                            (root/f'linux-menu-{name}.ppm').write_bytes(data)
                            frame_hashes[name]=digest
                            return
                        previous=digest;stamp=current
                time.sleep(.05)
            raise TimeoutError('Book did not settle before '+name)
        def key(name):
            subprocess.run(['xdotool','keydown',name],check=True,timeout=5)
            time.sleep(.15)
            subprocess.run(['xdotool','keyup',name],check=True,timeout=5)
            time.sleep(.5)
        book=BookMenu(process,root/'linux-symbols.txt')
        book.wait_ready()
        continue_available=book.continue_available()
        if args.fresh_start:
            assert not continue_available,'Continue available without a valid autosave'
            assert book.main_entry()=={'action':10,'grayed':False},'Fresh book must select New Game'
            key('Escape')
            book.wait_ready()
            assert book.main_entry()=={'action':10,'grayed':False},'Fresh Escape must remain in book'
        frame('before')
        book.select_main_action(4,key);key('Return');time.sleep(3);frame('options')
        key('Escape');time.sleep(3);frame('return')
        assert frame_hashes['before']!=frame_hashes['options']
        assert frame_hashes['options']!=frame_hashes['return']
        book.select_main_action(-2,key);key('Return')
        code=process.wait(timeout=15)
        assert code==0,code
    finally:
        if process.poll() is None: process.kill();process.wait()
text=(root/'linux-menu.log').read_text()
assert 'BLOCKED' not in text
assert 'reason=original-dos-termination status=0' in text
assert 'Native Glide window 1024x768 opened=1' in text
result=dict(exe_sha256=hashlib.sha256((root/'redguard-native').read_bytes()).hexdigest(),
            wine_used=False,original_quit_exit=code,keyboard_events='X11 XTest through SDL',
            menu_navigation_exercised=True,captured_frame_sha256=frame_hashes,
            startup_continue_available=continue_available,
            fresh_start_escape_verified=args.fresh_start,
            fresh_start_new_selected_verified=args.fresh_start,
            capture_settle='two successive complete matching back buffers',visual_review_required=True,native_windows_verified=False)
(root/'linux-menu-verification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
print('PASS native Linux original menu key sequence and clean original Quit without Wine')
