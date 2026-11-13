"""Walk across the island on foot and enter a building by its door.

Run under Xvfb with an island autosave in the scenario. The original console is used twice to learn
where two authored markers are (`load 1,N,0`) and to start at the first one; the probe then steers the
player with ordinary keyboard input toward the second marker, beside the Draggin Tale door, sidestepping
when blocked, and presses the action key at the door. Success is the walk (distance to the goal falls
below the door range), the on-screen prompt, and the tavern opening with a living player. The `drevil`
cheat keeps hazards from ending the walk. Linux software-GL and dummy audio only; not Windows, story
progress or full-game acceptance.
"""
import argparse, hashlib, json, math, os, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, '/tests')
from book_menu import BookMenu
from combat_trace import CombatTrace

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, default=Path('/work'))
p.add_argument('--game-dir', default='/work/island-reload-scenario')
p.add_argument('--user-dir', required=True, type=Path)
p.add_argument('--label', default='island-walk')
p.add_argument('--start', type=int, default=0, help='Island marker to start from')
p.add_argument('--goal', type=int, default=8, help='Island marker beside the door to reach')
a = p.parse_args()
if a.user_dir.exists():
    p.error('Use a fresh isolated user directory')
root = a.root
symbols = {x[2]: int(x[0], 16) for line in (root / 'linux-symbols.txt').read_text().splitlines() if len((x := line.split())) == 3}
log_path = root / (a.label + '.log')
W, H = 1024, 768
KEYS = {' ': 'space', ',': 'comma', '*': 'asterisk', '_': 'underscore'}
report = {'path': [], 'sidesteps': 0, 'full_game_verified': False, 'native_windows_verified': False, 'wine_used': False,
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

    def lockout():
        with open(f'/proc/{proc.pid}/mem', 'rb', buffering=0) as memory:
            memory.seek(symbols['loc_1A391E'])
            return int.from_bytes(memory.read(2), 'little')

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
        console('drevil')

        def load_marker(n):
            islands = log_count('island.rgm')
            console(f'load 1,{n},0')
            for _ in range(40):
                if log_count('island.rgm') > islands:
                    break
                time.sleep(0.5)
            return wait_world()[1]

        goal = load_marker(a.goal)['position']
        report['goal'] = goal
        load_marker(a.start)
        began = time.monotonic()
        while time.monotonic() - began < 30 and lockout() != 0:
            time.sleep(0.5)
        report['lockout_cleared'] = lockout() == 0
        _, here = state()
        report['start'] = here['position']
        report['initial_distance'] = round(math.dist(here['position'], goal))
        save('start', frame())
        stuck, last, turns, log_every = 0, None, 0, 0
        for step in range(1400):
            assert proc.poll() is None, 'Game exited'
            _, pl = state()
            dist = math.dist(pl['position'], goal)
            if step % 25 == 0:
                report['path'].append([round(pl['position'][0]), round(pl['position'][2]), round(dist)])
            if dist < 5000:
                break
            dx, dz = goal[0] - pl['position'][0], goal[2] - pl['position'][2]
            error = (round(math.atan2(-dx, dz) * 2048 / (2 * math.pi)) % 2048 - pl['heading'] + 1024) % 2048 - 1024
            if abs(error) > 90:
                key('Left' if error > 0 else 'Right', 0.12)
                continue
            key('Up', 0.25)
            _, after = state()
            stuck = stuck + 1 if math.dist(after['position'], pl['position']) < 300 else 0
            if stuck >= 4:
                # Blocked by scenery: turn away, walk along it, then steer back toward the goal.
                report['sidesteps'] += 1
                side = 'Left' if report['sidesteps'] % 2 else 'Right'
                key(side, 0.5)
                key('Up', 1.2)
                stuck = 0
        _, pl = state()
        report['final_distance'] = round(math.dist(pl['position'], goal))
        report['reached_goal'] = report["final_distance"] < 8000
        save('arrived', frame())
        assert report['reached_goal'], 'Did not reach the door on foot'
        taverns = log_count('tavern.rgm')
        for turn in range(14):
            if prompt_pixels(frame()) > 30:
                report['prompt_after_turns'] = turn
                break
            key('Right', 0.3)
            time.sleep(0.5)
        else:
            raise AssertionError('Activation prompt never appeared at the door')
        for press in range(3):
            key('Control_L', 0.4)
            for _ in range(30):
                if log_count('tavern.rgm') > taverns:
                    break
                time.sleep(0.5)
            if log_count('tavern.rgm') > taverns:
                break
        assert log_count('tavern.rgm') > taverns, 'Tavern did not open'
        snap, pl = wait_world(60)
        report['entered_tavern'] = True
        report['tavern_player'] = pl
        report['ok'] = True
    except Exception as error:
        report['ok'] = False
        report['error'] = repr(error)
        try:
            save('failed', frame())
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
        Path(str(root / a.label) + '.json').write_text(json.dumps(report, indent=1))
print(json.dumps({k: v for k, v in report.items() if k != 'path'}))
