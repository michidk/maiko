"""Measure how fast the player turns, in degrees per real second, at different frame rates.

Turning should not depend on the frame rate: the same key held for the same time must turn the
player by the same angle. The probe holds Left for a few seconds at several island markers (each
renders at a different speed under software GL), reading the heading from the original player
actor and the update counter (loc_1AF228) from guest memory. Linux software-GL and dummy audio only.
"""
import argparse, json, os, signal, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, '/tests')
from book_menu import BookMenu
from combat_trace import CombatTrace

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, default=Path('/work'))
p.add_argument('--game-dir', default='/work/island-reload-scenario')
p.add_argument('--user-dir', required=True, type=Path)
p.add_argument('--label', default='turn-rate')
p.add_argument('--markers', default='8,1,30', help='Island markers to turn at')
p.add_argument('--seconds', type=float, default=3.0)
a = p.parse_args()
if a.user_dir.exists():
    p.error('Use a fresh isolated user directory')
signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
root = a.root
symbols = {x[2]: int(x[0], 16) for line in (root / 'linux-symbols.txt').read_text().splitlines() if len((x := line.split())) == 3}
KEYS = {' ': 'space', ',': 'comma'}
report = {'full_game_verified': False, 'native_windows_verified': False, 'wine_used': False, 'turns': []}
with (root / (a.label + '.log')).open('w') as log:
    proc = subprocess.Popen([str(root / 'redguard-native'), '--game-dir', a.game_dir, '--user-dir', str(a.user_dir)],
                            stdout=log, stderr=subprocess.STDOUT, env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1'))
    trace = CombatTrace(proc, symbols)
    book = BookMenu(proc, root / 'linux-symbols.txt')

    def key(name, hold=0.15):
        subprocess.run(['xdotool', 'keydown', name], check=True, timeout=5)
        time.sleep(hold)
        subprocess.run(['xdotool', 'keyup', name], check=True, timeout=5)
        time.sleep(0.2)

    def counter():
        with open(f'/proc/{proc.pid}/mem', 'rb', buffering=0) as memory:
            memory.seek(symbols['loc_1AF228'])
            return int.from_bytes(memory.read(4), 'little')

    def heading():
        for _ in range(10):
            try:
                return next(x for x in trace.snapshot()['actors'] if x['selected'])['heading']
            except (OSError, RuntimeError, StopIteration):
                time.sleep(0.05)
        raise RuntimeError('No player snapshot')

    def console(command):
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
        time.sleep(3.0)

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
        time.sleep(15)
        console('drevil')
        for marker in map(int, a.markers.split(',')):
            console(f'load 1,{marker},0')
            time.sleep(4)
            h0, c0, t0 = heading(), counter(), time.monotonic()
            subprocess.run(['xdotool', 'keydown', 'Left'], check=True, timeout=5)
            time.sleep(a.seconds)
            h1, c1, t1 = heading(), counter(), time.monotonic()
            subprocess.run(['xdotool', 'keyup', 'Left'], check=True, timeout=5)
            turned = (h0 - h1) % 2048 or (h1 - h0) % 2048  # 2048 units are a full turn; direction does not matter here
            turned = min((h1 - h0) % 2048, (h0 - h1) % 2048)
            seconds, updates = t1 - t0, c1 - c0
            report['turns'].append({'marker': marker, 'seconds': round(seconds, 2), 'updates': updates, 'fps': round(updates / seconds, 1),
                                    'degrees_per_second': round(turned * 360 / 2048 / seconds, 1), 'units_per_update': round(turned / max(updates, 1), 2),
                                    'note': 'a turn above half a circle in the interval is folded and under-reported'})
        report['ok'] = True
    except Exception as error:
        report['ok'] = False
        report['error'] = repr(error)
    finally:
        proc.terminate()
        (root / (a.label + '.json')).write_text(json.dumps(report, indent=1))
        print(json.dumps(report))
