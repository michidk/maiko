"""Load each original world through the console and play it with scripted input.

Run under Xvfb with an island autosave in the scenario. For every world in WORLD.INI the probe
issues the original `load N,0,0`, waits for a living, initialized player, then sends a
reproducible mix of walking, turning, jumping and action-key presses for a fixed time while
sampling the player. A world fails if the process exits, the native fault reporter prints
BLOCKED, or the world never becomes ready. The original `drevil` cheat is typed first so that
hostile actors and hazards at marker-zero spawns cannot end the run; deaths, if any still occur,
and frozen players are recorded, not failed. A frozen player in a world loaded straight from the
console usually means the game's own lockout flag is still set; entering through the real door
clears it (probe_island_doors.py checks movement after arrival).

Linux software-GL and dummy audio only; not Windows, story progress or full-game acceptance.
"""
import argparse, hashlib, json, os, random, re, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, '/tests')
from book_menu import BookMenu
from combat_trace import CombatTrace

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, default=Path('/work'))
p.add_argument('--game-dir', default='/work/island-reload-scenario')
p.add_argument('--user-dir', required=True, type=Path)
p.add_argument('--label', default='world-wander')
p.add_argument('--seconds', type=int, default=20, help='Scripted play time per world')
p.add_argument('--worlds', help='Comma-separated world numbers; default all in WORLD.INI')
a = p.parse_args()
if a.user_dir.exists():
    p.error('Use a fresh isolated user directory')
root = a.root
symbols = {x[2]: int(x[0], 16) for line in (root / 'linux-symbols.txt').read_text().splitlines() if len((x := line.split())) == 3}
ini = Path(a.game_dir, 'WORLD.INI')
ini_text = (ini if ini.exists() else Path('/game/WORLD.INI')).read_text(errors='replace')
all_worlds = [(int(n), m.replace('\\', '/').split('/')[-1].lower()) for n, m in re.findall(r'world_map\[(\d+)\]=(\S+)', ini_text)]
wanted = None if not a.worlds else {int(x) for x in a.worlds.split(',')}
# World 99 is not loadable: the engine stops with its own "Bad world number in opening map" error.
worlds = [(n, m) for n, m in all_worlds if n < 32 and (wanted is None or n in wanted)]
log_path = root / (a.label + '.log')
KEYS = {' ': 'space', ',': 'comma'}
report = {'worlds': [], 'seconds_per_world': a.seconds, 'full_game_verified': False, 'native_windows_verified': False,
          'wine_used': False, 'driver_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          'exe_sha256': hashlib.sha256((root / 'redguard-native').read_bytes()).hexdigest()}


def log_text():
    return log_path.read_text(errors='replace')


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
        time.sleep(0.15)

    def state():
        for _ in range(10):
            try:
                snap = trace.snapshot()
                return snap, next(x for x in snap['actors'] if x['selected'])
            except (OSError, RuntimeError, StopIteration):
                time.sleep(0.05)
        raise RuntimeError('No player snapshot')

    def wait_world(timeout=60, require_alive=True):
        end, since = time.monotonic() + timeout, None
        while time.monotonic() < end:
            assert proc.poll() is None, 'Game exited'
            try:
                snap, pl = state()
            except RuntimeError:
                time.sleep(0.2)
                continue
            if (pl['health'] > 0 or not require_alive) and pl['animation'] is not None and snap['ui_mode'] == 31 and snap['startup_remaining'] == 0:
                since = since or time.monotonic()
                if time.monotonic() - since > 1:
                    return snap, pl
            else:
                since = None
            time.sleep(0.1)
        raise TimeoutError('Initialized world')

    def frame_to(name):
        src, requested = root / 'game-frame-latest.ppm', time.time_ns()
        for _ in range(100):
            if src.exists() and src.stat().st_mtime_ns > requested:
                time.sleep(0.1)
                (root / f'{a.label}-{name}.ppm').write_bytes(src.read_bytes())
                return
            time.sleep(0.05)

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
        console('drevil')  # the original cheat that prevents player death, so a hazard cannot end the run
        report['death_prevented'] = True
        for number, name in worlds:
            entry = {'world': number, 'map': name}
            report['worlds'].append(entry)
            blocked_before = log_text().count('BLOCKED')
            try:
                opens = log_text().lower().count(name)
                console(f'load {number},0,0')
                deadline = time.monotonic() + 45
                while time.monotonic() < deadline and proc.poll() is None and log_text().lower().count(name) <= opens:
                    time.sleep(0.5)
                assert proc.poll() is None, 'Game exited while loading'
                assert log_text().lower().count(name) > opens, 'Map never opened'
                snap, pl = wait_world(60)
                entry.update(start=pl['position'], actors=len(snap['actors']))
                rng = random.Random(number)
                moves = [('Up', 1.0), ('Up', 1.0), ('Left', 0.4), ('Right', 0.4), ('space', 0.15), ('Control_L', 0.2)]
                samples, died = [], False
                end = time.monotonic() + a.seconds
                while time.monotonic() < end:
                    assert proc.poll() is None, 'Game exited during play'
                    k, hold = rng.choice(moves)
                    key(k, hold)
                    try:
                        _, pl2 = state()
                        samples.append((pl2['health'], pl2['position']))
                        if pl2['health'] <= 0:
                            died = True
                            break
                    except RuntimeError:
                        pass
                frame_to(f'w{number}')
                health = [h for h, _ in samples]
                moved = len({tuple(pos) for _, pos in samples})
                entry.update(samples=len(samples), min_health=min(health) if health else None, died=died, positions_seen=moved,
                             frozen=bool(samples) and moved <= 1 and not died)
                assert log_text().count('BLOCKED') == blocked_before, 'Native fault reported'
                entry['ok'] = True
            except Exception as error:
                entry.update(ok=False, error=repr(error), exited=proc.poll() is not None)
                if proc.poll() is not None:
                    break
            print(json.dumps(entry), flush=True)
            if entry.get('died'):
                # A dead player blocks the next ready check; reload the island first.
                try:
                    console('load 1,0,0')
                    wait_world(60)
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
        report['all_ok'] = bool(report['worlds']) and all(w.get('ok') for w in report['worlds'])
        Path(str(root / a.label) + '.json').write_text(json.dumps(report, indent=1))
print(json.dumps({'all_ok': report.get('all_ok'), 'failed': [w['world'] for w in report['worlds'] if not w.get('ok')]}))
