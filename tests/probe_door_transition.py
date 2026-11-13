"""Walk-in door activation and scene change on the original island.

Run under Xvfb with an island autosave in the scenario. The probe uses the original F12
console only to place the player at the cartographer's cellar door (`ENTSMUG1`) and to set
the `password` flag; the door itself is activated with the ordinary action key.

Steps: without the password the original script must refuse and leave the island loaded;
with it the script must call `LoadWorld(24, 0, 0)`, open SMDEN.RGM and leave a living,
initialized player. Linux software-GL and dummy audio only; not Windows or full-game
acceptance.
"""
import argparse, hashlib, json, os, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, '/tests')
from book_menu import BookMenu
from combat_trace import CombatTrace

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, default=Path('/work'))
p.add_argument('--game-dir', default='/work/island-reload-scenario')
p.add_argument('--user-dir', required=True, type=Path)
p.add_argument('--label', default='door-transition')
a = p.parse_args()
if a.user_dir.exists():
    p.error('Use a fresh isolated user directory')
root = a.root
symbols = {x[2]: int(x[0], 16) for line in (root / 'linux-symbols.txt').read_text().splitlines() if len((x := line.split())) == 3}
log_path = root / (a.label + '.log')
report = {'steps': [], 'full_game_verified': False, 'native_windows_verified': False, 'wine_used': False,
          'driver_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          'exe_sha256': hashlib.sha256((root / 'redguard-native').read_bytes()).hexdigest()}
KEYS = {' ': 'space', ',': 'comma', '*': 'asterisk'}


def log_count(name):
    return log_path.read_text(errors='replace').lower().count(name)


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

    def capture(name):
        src, requested = root / 'game-frame-latest.ppm', time.time_ns()
        for _ in range(100):
            if src.exists() and src.stat().st_mtime_ns > requested:
                time.sleep(0.1)
                (root / f'{a.label}-{name}.ppm').write_bytes(src.read_bytes())
                return
            time.sleep(0.05)
        raise TimeoutError('Frame ' + name)

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
        _, start = wait_world()
        report['start'] = start
        console('object search entsmug1')
        console('object goto')
        # The door prompt needs the player to face it: one short turn, as the prompt is visible only then.
        key('Right', 0.3)
        time.sleep(0.6)
        _, at_door = state()
        report['at_door'] = at_door
        assert at_door['position'] != start['position'], 'Console placement did not move the player'
        capture('prompt')

        # 1. No password: the script refuses; the island stays loaded.
        before = log_count('smden.rgm')
        key('Control_L', 0.25)
        deadline = time.monotonic() + 25
        while time.monotonic() < deadline and log_count('smden.rgm') == before:
            time.sleep(0.5)
        report['refused_without_password'] = log_count('smden.rgm') == before
        assert report['refused_without_password'], 'Door opened without the password'
        snap, pl = wait_world()
        report['after_refusal'] = pl
        capture('refused')

        # 2. Password set: the same door loads world 24.
        console('setflag password,1')
        key('Control_L', 0.25)
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline and log_count('smden.rgm') == before:
            time.sleep(0.5)
        report['opened_with_password'] = log_count('smden.rgm') > before
        assert report['opened_with_password'], 'SMDEN.RGM did not open'
        snap, pl = wait_world(60)
        report['after'] = pl
        report['actors_after'] = len(snap['actors'])
        capture('after')
        report['door_transition_verified'] = pl['position'][0] != at_door['position'][0] and len(snap['actors']) > 1
        assert report['door_transition_verified']
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(10)
            except subprocess.TimeoutExpired:
                proc.kill()
        report['exit'] = proc.returncode
        Path(str(root / a.label) + '.json').write_text(json.dumps(report, indent=1))
print(json.dumps({k: v for k, v in report.items() if k in ('refused_without_password', 'opened_with_password', 'door_transition_verified', 'exit')}))
