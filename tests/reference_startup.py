"""Read DOSBox-X child memory for comparison only; never part of native runtime.

Run inside the existing Cyrus original-game image with /source mounted read-only.
Uses the retained original-state.py signature and relocation model. Results are
observations of this reference run, not a general DOS memory guarantee.
"""
import hashlib
import argparse
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import time

SOURCE = Path('/source')
GAME = Path('/tmp/reference-game')
options=argparse.ArgumentParser(description=__doc__)
options.add_argument('--autosave',type=Path)
options.add_argument('--observe-book-fonts',action='store_true',help='After combat startup, wait up to 120 seconds for original book font initialization')
options.add_argument('--observe-world',action='store_true',help='Select Continue after book startup and record original static-object state')
options.add_argument('--world-action',choices=['continue','new'],default='continue')
arguments=options.parse_args()
signature = bytes.fromhex('535156575589e581ec0c0000008945f48955f88b45f83b45f4742f8b45f883b83b020000')
assert hashlib.sha256((SOURCE/'Redguard/RGFX.EXE').read_bytes()).hexdigest() == '844a316b5f71e3c7fadf4b012ab63c5c13aa3606a4f1504f852ac1607ee4dfdf'
for root, dirs, files in os.walk(SOURCE):
    destination = GAME / Path(root).relative_to(SOURCE)
    destination.mkdir(parents=True, exist_ok=True)
    for name in files:
        original = Path(root)/name
        target = destination/name
        if original.suffix.lower() == '.ini':
            shutil.copyfile(original, target)
        else:
            target.symlink_to(original)
(GAME/'game.cue').write_bytes((SOURCE/'game.ins').read_bytes())
(GAME/'Redguard/glide2x.ovl').unlink(missing_ok=True)
(GAME/'Redguard/glide2x.ovl').symlink_to(SOURCE/'DOSBOX/glide2x_emu.ovl')
if arguments.autosave:
    slot=GAME/'Redguard/SAVEGAME/SAVEGAME.001'
    slot.mkdir(parents=True,exist_ok=True)
    save=slot/'SAVEGAME.SAV'
    save.unlink(missing_ok=True)
    shutil.copyfile(arguments.autosave,save)
    system=GAME/'Redguard/SYSTEM.INI'
    import re
    contents=system.read_text()
    for key in ['sound','redbook']:
        contents,count=re.subn(rf'^{key}\s*=.*$',f'{key}=0',contents,flags=re.MULTILINE)
        assert count==1
    system.write_text(contents)
environment = dict(os.environ, DISPLAY=':8', SDL_AUDIODRIVER='dummy')
configuration = Path('/tmp/reference.conf')
configuration.write_text(Path('/etc/cyrus/dosbox-x.conf').read_text().replace('/game', str(GAME)))
x = subprocess.Popen(['Xvfb', ':8', '-screen', '0', '1024x768x24', '-nolisten', 'tcp'])
time.sleep(1)
process = subprocess.Popen(['dosbox-x', '-conf', str(configuration)],
                           cwd=GAME, env=environment,
                           stdout=open('/tmp/reference-dosbox.log', 'w'), stderr=subprocess.STDOUT)
try:
    began = time.monotonic()
    result = None
    while time.monotonic()-began < 90:
        time.sleep(2)
        if process.poll() is not None:
            raise RuntimeError('DOSBox exited before combat table observation')
        with open(f'/proc/{process.pid}/mem', 'rb', buffering=0) as memory:
            for line in Path(f'/proc/{process.pid}/maps').read_text().splitlines():
                parts = line.split()
                start, end = (int(v,16) for v in parts[0].split('-'))
                if 'rw' not in parts[1] or end-start < 60 << 20 or len(parts)>5:
                    continue
                memory.seek(start)
                ram = memory.read(end-start)
                offset = ram.find(signature)
                if offset < 0:
                    continue
                delta = offset-0x76626-0x200010
                u32 = lambda address: struct.unpack_from('<I',ram,address)[0]
                pointer = u32(0x1a4e31+0x1d3010+delta)
                if not 0x10000 < pointer < len(ram)-0x100:
                    continue
                motion = u32(pointer+0x10+0x18)
                if not 0x10000 < motion < len(ram)-0x100:
                    continue
                record = [u32(motion+0x10+i*4) for i in range(18)]
                if record[0]!=1 or record[1]!=12001 or record[2]!=20000:
                    continue
                result = dict(elapsed_seconds=round(time.monotonic()-began,3),
                              guest_code_delta=delta, table_pointer=hex(pointer),
                              motion_pointer=hex(motion), motion_words=record,
                              physical_zero_bytes=ram[:32].hex(),
                              dos_ds_null_word=struct.unpack_from('<H',ram,0x10)[0],
                              dos_ds_null_attribute_pointer=u32(0x10+0x272),
                              original_attack00_arc_center=record[14])
                break
        if result:
            break
    if result is None:
        raise RuntimeError('No completed attack00 record observed within 90 seconds')
    if arguments.observe_book_fonts:
        ram_start,ram_size=start,end-start
        checkpoint_began=time.monotonic()
        while time.monotonic()-checkpoint_began<120:
            time.sleep(1)
            if process.poll() is not None:
                raise RuntimeError('DOSBox exited during original font observation')
            with open(f'/proc/{process.pid}/mem','rb',buffering=0) as memory:
                memory.seek(ram_start); ram=memory.read(ram_size)
            fonts={}
            for style in [3,4,5,7,8,9,10]:
                pointer=u32(0x1a2dc9+style*4+0x1d3010+delta)
                metric=None
                if 0x10000<pointer<len(ram)-0xe60:
                    metric=list(struct.unpack_from('<HH',ram,pointer+0x10+0x46))
                fonts[str(style)]={'pointer':hex(pointer),'metrics':metric}
            result['font_observation']={
                'elapsed_seconds':round(time.monotonic()-began,3),
                'styles':fonts,
                'dos_ds_null_offset_46_word':struct.unpack_from('<H',ram,0x10+0x46)[0],
                'completed':fonts['9']['metrics'] is not None,
            }
            if result['font_observation']['completed']: break
    elif arguments.autosave:
        time.sleep(20)
    if arguments.observe_world:
        if not arguments.observe_book_fonts:
            raise ValueError('World reference requires --observe-book-fonts')
        time.sleep(5)
        subprocess.run(['xdotool','search','--name','DOSBox','windowfocus','--sync'],env=environment,check=True,timeout=5)
        if arguments.world_action=='new':
            for _ in range(3):
                subprocess.run(['xdotool','keydown','Up'],env=environment,check=True,timeout=5)
                time.sleep(1)
                subprocess.run(['xdotool','keyup','Up'],env=environment,check=True,timeout=5)
                time.sleep(1)
        subprocess.run(['xdotool','keydown','Return'],env=environment,check=True,timeout=5)
        time.sleep(1)
        subprocess.run(['xdotool','keyup','Return'],env=environment,check=True,timeout=5)
        snapshots=[]
        for _ in range(30):
            time.sleep(2)
            if process.poll() is not None: break
            with open(f'/proc/{process.pid}/mem','rb',buffering=0) as memory:
                memory.seek(ram_start);ram=memory.read(ram_size)
            pointer=u32(0x1a37ae+0x1d3010+delta)
            actors=[];visited=set()
            while 0x10000<pointer<len(ram)-0x310 and pointer not in visited and len(actors)<1500:
                visited.add(pointer);base=pointer+0x10
                actors.append(dict(pointer=hex(pointer),kind=ram[base+0xc1],animation=hex(u32(base+0x247)),pose=hex(u32(base+0x24b))))
                pointer=u32(base+4)
            selected=u32(0x1a3865+0x1d3010+delta)
            snapshot=dict(elapsed_seconds=round(time.monotonic()-began,3),actor_count=len(actors),static_null_animation=sum(v['kind']==1 and v['animation']=='0x0' for v in actors),selected=hex(selected),dos_ds_null_word=struct.unpack_from('<H',ram,0x10)[0])
            if 0x10000<selected<len(ram)-0x310:
                snapshot['position']=list(struct.unpack_from('<iii',ram,selected+0x10+0xcb))
            snapshots.append(snapshot)
            if len(actors)>100 and snapshot['static_null_animation'] and len(snapshots)>5: break
        result['world_snapshots']=snapshots

    if arguments.autosave:
        result['autosave_sha256']=hashlib.sha256(arguments.autosave.read_bytes()).hexdigest()
    subprocess.run(['scrot','/tmp/reference-startup.png'],env=environment,check=True)
    print(json.dumps(result,indent=2),flush=True)
finally:
    process.terminate()
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=3)
    finally:
        x.terminate()
        x.wait(timeout=3)
