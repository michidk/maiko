"""Walk-in activation of the original island entrances.

Run under Xvfb with an island autosave in the scenario. For each entrance the probe loads
the island at the authored marker beside that door with the original console (`load 1,N,0`),
turns until the original activation prompt is on screen, then presses the ordinary action
key and requires the destination map to open with a living, initialized player. The catacomb door
needs its story flag, set through the console. A failed door is recorded and the run continues.

Linux software-GL and dummy audio only; not Windows, story progress or full-game acceptance.
"""
import argparse, hashlib, json, os, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, '/tests')
from book_menu import BookMenu
from combat_trace import CombatTrace

# name, island marker beside the door, destination map, console setup. ENTCAVE (caverns, world 4)
# is left out: its script is a proximity trigger with no prompt, and its object does not exist at
# the marker (`object search entcave` finds nothing), so the trigger could not be exercised.
DOORS = [
    ('ENTTAVE1', 8, 'tavern.rgm', []),
    ('ENTGERR1', 21, 'gerricks.rgm', []),
    ('ENTROLL1', 16, 'rollos.rgm', []),
    ('ENTBELL1', 0, 'belltowr.rgm', []),
    ('ENTHARB1', 22, 'harbtowr.rgm', []),
    ('ENTJFFE1', 14, 'jffers.rgm', []),
    ('ENTOBSRV', 6, 'observe.rgm', []),
    ('ENTCATA1', 45, 'catacomb.rgm', ['setflag after_catacombs,1']),
]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, default=Path('/work'))
p.add_argument('--game-dir', default='/work/island-reload-scenario')
p.add_argument('--user-dir', required=True, type=Path)
p.add_argument('--label', default='island-doors')
p.add_argument('--only', help='Comma-separated entrance names')
a = p.parse_args()
if a.user_dir.exists():
    p.error('Use a fresh isolated user directory')
root = a.root
symbols = {x[2]: int(x[0], 16) for line in (root / 'linux-symbols.txt').read_text().splitlines() if len((x := line.split())) == 3}
log_path = root / (a.label + '.log')
W, H = 1024, 768
KEYS = {' ': 'space', ',': 'comma', '*': 'asterisk', '_': 'underscore'}
report = {'doors': [], 'full_game_verified': False, 'native_windows_verified': False, 'wine_used': False,
          'driver_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          'exe_sha256': hashlib.sha256((root / 'redguard-native').read_bytes()).hexdigest()}


def log_count(name):
    return log_path.read_text(errors='replace').lower().count(name)


def prompt_pixels(data):
    # The activation prompt is orange text along the bottom-left; nothing else is drawn there.
    px = data[-W * H * 3:]
    n = 0
    for y in range(715, H):
        row = y * W * 3
        for x in range(0, 420):
            o = row + x * 3
            if px[o] > 200 and 120 < px[o + 1] < 230 and px[o + 2] < 90:
                n += 1
    return n


def mean_luma(data):
    px = data[-W * H * 3:][::97 * 3]
    return round(sum(px[i] * 0.3 + px[i + 1] * 0.59 + px[i + 2] * 0.11 for i in range(0, len(px) - 2, 3)) / (len(px) // 3), 1)


with log_path.open('w') as log:
    proc = subprocess.Popen([str(root / 'redguard-native'), '--game-dir', a.game_dir, '--user-dir', str(a.user_dir)],
                            stdout=log, stderr=subprocess.STDOUT,
                            env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1', RG_CAPTURE_DIR=str(root)))
    trace = CombatTrace(proc, symbols)
    book = BookMenu(proc, root / 'linux-symbols.txt')

    def key(name, hold=0.15):
        subprocess.run(['xdotool', 'keydown', name], check=True, timeout=5)
        time.sleep(hold)
        subprocess.run(['xdotool', 'keyup', name], check=True, timeout=5)
        time.sleep(0.2)

    def state():
        for _ in range(10):
            try:
                snap = trace.snapshot()
                return snap, next(x for x in snap['actors'] if x['selected'])
            except (OSError, RuntimeError, StopIteration):
                time.sleep(0.05)
        raise RuntimeError('No player snapshot')

    def wait_world(timeout=60):
        end, since = time.monotonic() + timeout, None
        while time.monotonic() < end:
            assert proc.poll() is None, 'Game exited'
            try:
                snap, pl = state()
            except RuntimeError:
                time.sleep(0.2)
                continue
            if pl['health'] > 0 and pl['animation'] is not None and snap['ui_mode'] == 31 and snap['startup_remaining'] == 0:
                since = since or time.monotonic()
                if time.monotonic() - since > 1:
                    return snap, pl
            else:
                since = None
            time.sleep(0.1)
        raise TimeoutError('Initialized world')

    def frame():
        src, requested = root / 'game-frame-latest.ppm', time.time_ns()
        for _ in range(100):
            if src.exists() and src.stat().st_mtime_ns > requested:
                time.sleep(0.1)
                return src.read_bytes()
            time.sleep(0.05)
        raise TimeoutError('Frame')

    def save(name, data):
        (root / f'{a.label}-{name}.ppm').write_bytes(data)

    def console(command):
        # xdotool type drops characters; single presses at this pace do not.
        key('F12', 0.2)
        time.sleep(1.5)
        for ch in command:
            subprocess.run(['xdotool', 'keydown', KEYS.get(ch, ch)], check=True, timeout=5)
            time.sleep(0.09)
            subprocess.run(['xdotool', 'keyup', KEYS.get(ch, ch)], check=True, timeout=5)
            time.sleep(0.22)
        time.sleep(0.4)
        key('Return')
        time.sleep(1.2)
        key('Escape', 0.2)
        time.sleep(1.0)

    try:
        for _ in range(300):
            r = subprocess.run(['xdotool', 'search', '--name', '^Redguard native experiment$'], capture_output=True, text=True, timeout=5)
            if r.returncode == 0:
                subprocess.run(['xdotool', 'windowfocus', '--sync', r.stdout.splitlines()[0]], check=True, timeout=5)
                break
            time.sleep(0.1)
        book.wait_ready(timeout=40)
        book.select_main_action(-1, key)
        key('Return')
        wait_world()
        only = set(a.only.split(',')) if a.only else None
        for name, marker, dest, setup in DOORS:
            if only and name not in only:
                continue
            entry = {'door': name, 'marker': marker, 'destination': dest}
            report['doors'].append(entry)
            try:
                islands = log_count('island.rgm')
                console(f'load 1,{marker},0')
                for _ in range(40):
                    if log_count('island.rgm') > islands:
                        break
                    time.sleep(0.5)
                _, entry['start'] = wait_world()
                for command in setup:
                    console(command)
                before = log_count(dest)
                for turn in range(14):
                    data = frame()
                    if prompt_pixels(data) > 30:
                        entry['prompt_after_turns'] = turn
                        save(name + '-prompt', data)
                        break
                    key('Right', 0.3)
                    time.sleep(0.5)
                else:
                    raise AssertionError('Activation prompt never appeared')
                key('Control_L', 0.25)
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline and log_count(dest) == before:
                    time.sleep(0.5)
                entry['opened'] = log_count(dest) > before
                assert entry['opened'], dest + ' did not open'
                snap, pl = wait_world(60)
                time.sleep(3)
                data = frame()
                save(name + '-arrival', data)
                # The game's own lockout flag must clear and the player must then respond to input.
                def lockout():
                    with open(f'/proc/{proc.pid}/mem', 'rb', buffering=0) as memory:
                        memory.seek(symbols['loc_1A391E'])
                        return int.from_bytes(memory.read(2), 'little')
                began = time.monotonic()
                cleared = None
                while time.monotonic() - began < 20:
                    if lockout() == 0:
                        cleared = round(time.monotonic() - began, 1)
                        break
                    time.sleep(0.5)
                _, here = state()
                key('Up', 1.5)
                time.sleep(0.5)
                _, there = state()
                entry.update(lockout_cleared_after_s=cleared, moved_after_up=here['position'] != there['position'])
                entry.update(arrival=pl, actors=len(snap['actors']), arrival_mean_luma=mean_luma(data), ok=True)
            except Exception as error:
                entry.update(ok=False, error=repr(error))
                if proc.poll() is not None:
                    break
                try:
                    save(name + '-failed', frame())
                except Exception:
                    pass
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(10)
            except subprocess.TimeoutExpired:
                proc.kill()
        report['exit'] = proc.returncode
        report['all_ok'] = bool(report['doors']) and all(d.get('ok') for d in report['doors'])
        Path(str(root / a.label) + '.json').write_text(json.dumps(report, indent=1))
print(json.dumps([{k: d.get(k) for k in ('door', 'ok', 'prompt_after_turns', 'lockout_cleared_after_s', 'moved_after_up', 'error')} for d in report['doors']]))
