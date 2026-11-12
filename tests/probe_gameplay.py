"""Bounded original-menu gameplay probe. A timeout is evidence, never a pass."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import struct
import time

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--runtime-dir',type=Path,default=Path('/work'),help='Run the actual extracted runtime without replacing its executable or libraries')
p.add_argument('--game-dir',default='/work/menu-scenario',help='Disposable development fixture directory')
p.add_argument('--cd-image',help='Owned ISO or MODE1/2352 image mounted read-only as DOS D:')
p.add_argument('--action',choices=['continue','new'],default='continue')
p.add_argument('--fresh-start',action='store_true',help='Observe original startup without an autosave or a book selection')
p.add_argument('--seconds',type=float,default=30)
p.add_argument('--wine-game-dir',help='Optional Windows drive path for the selected Linux game fixture')
p.add_argument('--wine',action='store_true',help='Run the Windows host under Wine with posted window messages')
p.add_argument('--controls',action='store_true',help='Measure original movement, return to book and Quit after Continue')
p.add_argument('--uncaptured',action='store_true',help='Avoid framebuffer readback overhead while measuring timing and physics')
p.add_argument('--jump-cycle',action='store_true',help='Observe original jump ascent, descent and landing after releasing the jump key')
p.add_argument('--window-close',action='store_true',help='Close the native window after the jump cycle and require clean process exit')
p.add_argument('--movie-skip',action='store_true',help='Inspect original Smacker playback, skip configured points and verify entry into the ship')
p.add_argument('--movie-lead-seconds',type=float,default=2,help='Uninterrupted initial movie playback before skip input, from0 to10 seconds')
p.add_argument('--modal-ui',action='store_true',help='Verify original logbook open/close and map input followed by return to the book')
p.add_argument('--exercise',action='store_true',help='Record inventory, sword, attack and jump input with original state snapshots; visual review required')
p.add_argument('--user-dir',help='Opt-in isolated save/config directory, outside the game installation')
a=p.parse_args()
if not 0<=a.movie_lead_seconds<=10:p.error('--movie-lead-seconds must be between0 and10')
if sum((a.controls,a.exercise,a.modal_ui,a.movie_skip,a.jump_cycle))>1:p.error('Use either --controls or --exercise')
if a.controls and a.action!='continue':p.error('--controls requires --action continue')
if a.window_close and (not a.jump_cycle or not a.wine):p.error('--window-close requires --jump-cycle --wine')
if a.fresh_start and a.action!='new':p.error('--fresh-start requires --action new')
# Every probe writes shared captures; fail before touching them if another is active.
probe_lock=open('/tmp/redguard-game-probe.lock','w')
fcntl.flock(probe_lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
runtime=a.runtime_dir.resolve()
root=Path('/work');prefix=root/f'gameplay-{a.action}'
report={'action':a.action,'full_game_verified':False,'wine_used':a.wine,'native_windows_verified':False,
        'exe_sha256':hashlib.sha256((runtime/('redguard-native.exe' if a.wine else 'redguard-native')).read_bytes()).hexdigest()}
environment=dict(os.environ,LIBGL_ALWAYS_SOFTWARE='1',RG_CAPTURE_DIR='Z:\\work' if a.wine else '/work')
if a.uncaptured:environment.pop('RG_CAPTURE_DIR',None)
command=[str(runtime/'redguard-native'),'--game-dir',a.game_dir]
if a.user_dir:
    command+=['--user-dir',a.user_dir]
    report['writable_user_storage']=True
if a.wine:
    environment.update(WINEDEBUG='-all',WINEPREFIX=os.environ.get('WINEPREFIX','/tmp/rg-wine'))
    command=['/usr/lib/wine/wine64',str(runtime/'redguard-native.exe'),'--game-dir',a.wine_game_dir or 'Z:'+a.game_dir.replace('/','\\')]
    if a.user_dir:command+=['--user-dir','Z:'+a.user_dir.replace('/','\\')]
if a.cd_image:
    command+=['--cd-image',('Z:'+a.cd_image.replace('/','\\')) if a.wine else a.cd_image]
    report['cd_image_requested']=True
for file in root.glob('game-frame-*.ppm'):file.unlink()
began=time.monotonic();peak=0
with prefix.with_suffix('.log').open('w') as log:
    process=subprocess.Popen(command,env=environment,stdout=log,stderr=subprocess.STDOUT)
    try:
        if not a.fresh_start:
            deadline=time.monotonic()+40
            while 'CLIP entry=7' not in prefix.with_suffix('.log').read_text():
                if process.poll() is not None:raise RuntimeError('Original game stopped before menu')
                if time.monotonic()>deadline:raise TimeoutError('Original menu initialization')
                time.sleep(.1)
            time.sleep(3)
        if not a.wine:
            window=subprocess.check_output(['xdotool','search','--name','^Redguard native experiment$'],text=True,timeout=5).splitlines()[0]
            subprocess.run(['xdotool','windowfocus','--sync',window],check=True,timeout=5)
        if not a.uncaptured and not a.fresh_start:shutil.copyfile(root/'game-frame-latest.ppm',str(prefix)+'-menu.ppm')
        def key(name,hold=.15,observations=None):
            def observe_for(seconds):
                until=time.monotonic()+seconds
                while time.monotonic()<until:
                    if observations is not None:observations.append(state())
                    time.sleep(.04)
            if a.wine:
                helper=subprocess.Popen(['/usr/lib/wine/wine64','/work/menu-keys.exe',('enter' if name=='Return' else name.lower()),str(round(hold*1000))],env=environment)
                try:
                    until=time.monotonic()+10
                    while helper.poll() is None:
                        if time.monotonic()>until:raise TimeoutError('Input helper')
                        observe_for(.04)
                    if helper.returncode:raise RuntimeError('Input helper failed')
                finally:
                    if helper.poll() is None:helper.kill();helper.wait()
            else:
                subprocess.run(['xdotool','keydown',name],check=True,timeout=5)
                try:observe_for(hold)
                finally:subprocess.run(['xdotool','keyup',name],check=True,timeout=5)
            observe_for(.5)
        if not a.fresh_start:
            if a.action=='new':
                for _ in range(3):key('Up')
            key('Return');report['selection_sent']=True
        else:report['fresh_start_without_selection']=True
        deadline=time.monotonic()+a.seconds
        world_at=None;state_address=None;ui_address=None;extra_symbols={}
        if a.controls or a.exercise or a.modal_ui or a.movie_skip or a.jump_cycle:
            for line in (root/('native-symbols.txt' if a.wine else 'linux-symbols.txt')).read_text().splitlines():
                parts=line.split()
                if len(parts)==3:extra_symbols[parts[2]]=int(parts[0],16)
                if len(parts)==3 and parts[2]=='loc_1A3865':state_address=int(parts[0],16)
                if len(parts)==3 and parts[2]=='loc_1A824C':ui_address=int(parts[0],16)
            if state_address is None:raise RuntimeError('Missing original selected-player symbol')
        def state():
            with open(f'/proc/{process.pid}/mem','rb',buffering=0) as memory:
                memory.seek(state_address);actor=struct.unpack('<I',memory.read(4))[0]
                if not actor:return None
                memory.seek(actor+0xcb);position=list(struct.unpack('<iii',memory.read(12)))
                memory.seek(actor+0x247);anim=struct.unpack('<I',memory.read(4))[0]
                group=None
                if anim:memory.seek(anim);group=struct.unpack('<H',memory.read(2))[0]
                memory.seek(actor+0x272);attrs=struct.unpack('<I',memory.read(4))[0]
                health=None
                if attrs:memory.seek(attrs+3);health=memory.read(1)[0]
                memory.seek(actor+0x1b1);motion=list(memory.read(3))
                memory.seek(actor+0x1c2);origin,timer=struct.unpack('<iB',memory.read(5))
                timing={}
                for label in ['loc_1A4620','rg_tick_delta','loc_1EEC48','loc_1EEC54','loc_1A4624']:
                    if label in extra_symbols:
                        memory.seek(extra_symbols[label]);timing[label]=int.from_bytes(memory.read(4),'little')
                ui_mode=None
                if ui_address:memory.seek(ui_address);ui_mode=struct.unpack('<I',memory.read(4))[0]
            return {'actor':hex(actor),'position':position,'animation_group':group,'health':health,'ui_mode':ui_mode,'motion':motion,'fall_origin':origin,'fall_timer':timer,'timing':timing}
        def movie_state():
            with open(f'/proc/{process.pid}/mem','rb',buffering=0) as memory:
                def read(address,n=4):memory.seek(address);return int.from_bytes(memory.read(n),'little')
                pointer=read(extra_symbols['loc_1FA7B4'])
                result={'handle':pointer,'sample_starts':read(extra_symbols['starts'],8),'nonzero_frames':read(extra_symbols['nonzero'],8),'stream_buffers':read(extra_symbols['stream_buffers'],8),'stream_nonzero':read(extra_symbols['stream_nonzero'],8),'stream_mixed':read(extra_symbols['stream_mixed'],8),'stream_underrun':read(extra_symbols['stream_underrun'],8)}
                if pointer:
                    result.update(width=read(pointer+4),height=read(pointer+8),frames=read(pointer+12),frame=read(pointer+0x374),audio_track=read(pointer+0x444))
                return result
        while process.poll() is None and time.monotonic()<deadline:
            if a.movie_skip:
                movie=movie_state();report['movie_last_seen']=movie
                if movie['handle'] and movie.get('width')==640 and movie.get('height')==240:
                    report['movie_snapshots']=[movie];report['movie_open_observed']=True
                    time.sleep(a.movie_lead_seconds)
                    report['movie_uninterrupted_lead_seconds']=a.movie_lead_seconds
                    for index in range(4):
                        key('Escape',.6);time.sleep(2)
                        if process.poll() is not None:raise RuntimeError('Original movie stopped during skip')
                        report['movie_snapshots'].append(movie_state())
                        shutil.copyfile(root/'game-frame-latest.ppm',str(prefix)+f'-movie-skip-{index}.ppm')
                    snapshots=report['movie_snapshots']
                    for index,point in enumerate([1508,4502,5801],1):
                        assert snapshots[index]['handle'] and snapshots[index].get('frame',0)>=point,'Original movie skip did not reach configured point'
                        assert snapshots[index]['frame']>snapshots[index-1]['frame'],'Original movie frame counter did not advance'
                    assert snapshots[4]['handle']==0,'Original movie handle remained open after final skip'
                    report['movie_audio_mixed_verified']=any(x.get('audio_track',4294967295)!=4294967295 for x in snapshots[:4]) and snapshots[3]['stream_buffers']>snapshots[0]['stream_buffers'] and snapshots[3]['stream_nonzero']>snapshots[0]['stream_nonzero']
                    until=time.monotonic()+25
                    while process.poll() is None and state() is None and time.monotonic()<until:time.sleep(.2)
                    report['world_after_movie']=state()
                    assert report['world_after_movie'],'Original movie did not return to world'
                    key('Escape');time.sleep(3);key('Up');key('Return')
                    assert process.wait(timeout=15)==0
                    report['movie_skip_to_world_verified']=True
                    break
            status=Path(f'/proc/{process.pid}/status').read_text()
            for line in status.splitlines():
                if line.startswith('VmRSS:'):peak=max(peak,int(line.split()[1]))
            if a.jump_cycle:
                current=state()
                if current and world_at is None:world_at=time.monotonic()
                if current and time.monotonic()-world_at>4:
                    samples=[state()];report['jump_samples']=samples
                    key('space',.15,samples)
                    end=time.monotonic()+8
                    while process.poll() is None and time.monotonic()<end:
                        sample=state()
                        if sample:samples.append(sample)
                        time.sleep(.04)
                    assert process.poll() is None,'Original game stopped during jump'
                    initial=samples[0]['position'][1];lowest=min(x['position'][1] for x in samples)
                    report['jump_ascent_observed']=lowest<initial-256
                    apex=next(i for i,x in enumerate(samples) if x['position'][1]==lowest)
                    report['jump_descent_observed']=any(x['position'][1]>lowest+256 for x in samples[apex+1:])
                    report['jump_landed_observed']=any(abs(x['position'][1]-initial)<=512 and x['motion'][1:]==[0,0] for x in samples[apex+1:])
                    assert report['jump_ascent_observed'] and report['jump_descent_observed'] and report['jump_landed_observed'],'Jump must rise, descend and land after key release'
                    if a.window_close:
                        key('close');code=process.wait(timeout=15)
                        report['window_close_exit']=code
                        assert code==0,('Native window close must exit cleanly',code)
                        report['window_close_verified']=True
                    else:
                        key('Escape');time.sleep(3);key('Up');key('Return')
                        assert process.wait(timeout=15)==0
                    report['jump_cycle_verified']=True
                    break
            if a.modal_ui:
                current=state()
                if current and world_at is None:world_at=time.monotonic()
                if current and time.monotonic()-world_at>4:
                    if not a.user_dir:raise RuntimeError('Modal fixture requires isolated user-dir')
                    (Path(a.user_dir)/'LOGBOOK.TXT').write_text('7,Native compatibility interface test\n')
                    report['synthetic_logbook_entry']=True
                    report['modal_snapshots']=[{'step':'initial','state':state()}]
                    for label,name,delay in [('logbook','l',3),('logbook-close','Escape',3),('map','m',3),('map-close','Escape',3)]:
                        if label=='map-close' and not report.get('map_open_observed'):continue
                        key(name,.6 if name in ['Escape','m'] else .15);time.sleep(delay)
                        if process.poll() is not None:raise RuntimeError('Original game stopped during '+label)
                        snapshot=state();report['modal_snapshots'].append({'step':label,'state':snapshot})
                        shutil.copyfile(root/'game-frame-latest.ppm',str(prefix)+'-'+label+'.ppm')
                        if label=='map':report['map_open_observed']='OPEN SYSTEM\\MAPMAP.GXA' in prefix.with_suffix('.log').read_text()
                        if label=='logbook':assert snapshot['ui_mode']==30,'Original logbook did not open'
                        if label=='logbook-close':assert snapshot['ui_mode']==31,'Original logbook did not close'
                    report['map_open_observed']='OPEN SYSTEM\\MAPMAP.GXA' in prefix.with_suffix('.log').read_text()
                    key('Escape');time.sleep(3)
                    key('Up');key('Return')
                    code=process.wait(timeout=15);assert code==0,code
                    report['modal_ui_verified']=True
                    break
            if a.exercise:
                current=state()
                if current and world_at is None:world_at=time.monotonic()
                if current and time.monotonic()-world_at>4:
                    report['exercise_snapshots']=[{'step':'initial','state':state()}]
                    for label,name,hold,delay in [('inventory','i',.15,2),('inventory-close','i',.15,2),
                        ('sword','s',.15,2),('attack','Control_L',.4,1),('attack-2','Control_L',.4,1),
                        ('attack-3','Control_L',.4,1),('jump','space',.2,2)]:
                        observed=[];key(name,hold,observed);time.sleep(delay)
                        report.setdefault('action_observations',{})[label]=observed
                        if process.poll() is not None:raise RuntimeError('Original game stopped during '+label)
                        snapshot=state()
                        report['exercise_snapshots'].append({'step':label,'state':snapshot})
                        if label=='inventory':assert snapshot['ui_mode']==28,'Inventory did not open'
                        if label=='inventory-close':assert snapshot['ui_mode']==31,'Inventory key-release wait did not finish'
                        shutil.copyfile(root/'game-frame-latest.ppm',str(prefix)+'-'+label+'.ppm')
                    jump=[x for x in report.get('action_observations',{}).get('jump',[]) if x]
                    report['jump_vertical_motion_observed']=bool(jump) and len({x['position'][1] for x in jump})>1
                    report['exercise_inputs_delivered']=True
                    report['visual_review_required']=True
                    break
            if a.controls:
                current=state()
                if current and world_at is None:world_at=time.monotonic()
                if current and time.monotonic()-world_at>4:
                    report['world_before']=state()
                    shutil.copyfile(root/'game-frame-latest.ppm',str(prefix)+'-world-before.ppm')
                    key('Up',2);time.sleep(2)
                    report['world_after_walk']=state()
                    before=report['world_before']['position'];after=report['world_after_walk']['position']
                    assert (before[0],before[2])!=(after[0],after[2]),'Original player did not move horizontally'
                    report['world_movement_verified']=True
                    shutil.copyfile(root/'game-frame-latest.ppm',str(prefix)+'-world-after.ppm')
                    key('Right',1);time.sleep(2)
                    report['world_after_turn']=state()
                    key('Escape');time.sleep(3)
                    shutil.copyfile(root/'game-frame-latest.ppm',str(prefix)+'-world-book.ppm')
                    key('Up');key('Return')
                    code=process.wait(timeout=15)
                    before=report['world_before']['position'];after=report['world_after_walk']['position']
                    assert (before[0],before[2])!=(after[0],after[2]),'Original player did not move horizontally'
                    assert code==0,code
                    report['world_controls_verified']=True
                    break
            time.sleep(.2)
        if a.jump_cycle and not report.get('jump_cycle_verified'):raise RuntimeError('Jump cycle sequence was not reached')
        if a.movie_skip and not report.get('movie_skip_to_world_verified'):raise RuntimeError('Movie skip sequence was not reached')
        if a.modal_ui and not report.get('modal_ui_verified'):raise RuntimeError('Modal UI sequence was not reached')
        if a.controls and not report.get('world_controls_verified'):raise RuntimeError('World controls were not reached')
        report['natural_exit']=process.poll()
        report['observation_timeout']=process.poll() is None and not report.get('exercise_inputs_delivered')
        if process.poll() is None:report['intentional_termination_reason']='exercise_complete' if report.get('exercise_inputs_delivered') else 'observation_deadline'
    except Exception as error:
        report['harness_error']=repr(error)
        raise
    finally:
        if process.poll() is None:process.kill();process.wait()
        latest=root/'game-frame-latest.ppm'
        if latest.exists():shutil.copyfile(latest,str(prefix)+'-latest.ppm')
        report['elapsed_seconds']=round(time.monotonic()-began,3)
        report['peak_rss_kib']=peak
        report['blocked']='BLOCKED' in prefix.with_suffix('.log').read_text()
        prefix.with_suffix('.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
