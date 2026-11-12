"""Walk from Brennan’s boat across the original gangway and onto the harbour dock.

Run under Xvfb with the post-Brennan-conversation autosave in the scenario.
This probe sends keyboard input and reads guest memory without modifying it.
"""
import argparse, hashlib, json, math, os, subprocess, time
from pathlib import Path
import sys
sys.path.insert(0, '/tests')
from book_menu import BookMenu
from combat_trace import CombatTrace
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--root', type=Path, default=Path('/work'))
parser.add_argument('--game-dir', default='/work/menu-scenario')
parser.add_argument('--user-dir', required=True, type=Path)
parser.add_argument('--label', default='island-gangway')
args = parser.parse_args()
if args.user_dir.exists():
    parser.error('Use a fresh isolated user directory')
if Path(args.label).name != args.label:
    parser.error('Label must be a filename')
source_save = Path(args.game_dir) / 'SAVEGAME/SAVEGAME.001/SAVEGAME.SAV'
if not source_save.is_file():
    parser.error('A docked-island source save is required')
source_hash = hashlib.sha256(source_save.read_bytes()).hexdigest()
root = args.root
out = root / args.label
began = time.monotonic()
report = {'snapshots': [], 'decisions': [], 'full_game_verified': False, 'native_windows_verified': False, 'wine_used': False}
symbols = {p[2]: int(p[0], 16) for line in (root / 'linux-symbols.txt').read_text().splitlines() if len((p := line.split())) == 3}
report['source_save_sha256'] = source_hash
report['driver_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
report['source_sha256'] = {name: hashlib.sha256((Path('/tests') / name).read_bytes()).hexdigest() for name in ['check_island_gangway.py', 'combat_trace.py', 'book_menu.py']}
report['exe_sha256'] = hashlib.sha256((root / 'redguard-native').read_bytes()).hexdigest()
with out.with_suffix('.log').open('w') as log:
    process = subprocess.Popen([str(root / 'redguard-native'), '--game-dir', args.game_dir, '--user-dir', str(args.user_dir)], stdout=log, stderr=subprocess.STDOUT, env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1', RG_CAPTURE_DIR=str(root)))
    trace = CombatTrace(process, symbols)
    book = BookMenu(process, root / 'linux-symbols.txt')

    def snap(label):
        for attempt in range(10):
            try:
                r = trace.snapshot()
                break
            except (OSError, RuntimeError):
                if attempt == 9:
                    raise
                time.sleep(0.02)
        with open(f'/proc/{process.pid}/mem', 'rb', buffering=0) as memory:
            controls = {}
            for label0, symbol, size in [('lockout', 'loc_1A391E', 2), ('dialogue', 'loc_1A6209', 2), ('topics', 'loc_1A620B', 2), ('sound_inhibited', 'loc_1DEA10', 4)]:
                memory.seek(symbols[symbol])
                controls[label0] = int.from_bytes(memory.read(size), 'little')
        if controls['topics'] == 1:
            with open(f'/proc/{process.pid}/mem', 'rb', buffering=0) as memory:

                def u32(address):
                    memory.seek(address)
                    data = memory.read(4)
                    assert len(data) == 4
                    return int.from_bytes(data, 'little')
                count = u32(symbols['loc_1F65E8'])
                base = u32(symbols['loc_1F65EC'])
                assert 0 < count <= 300
                items = []
                for index in range(count):
                    pointer = u32(base + index * 30)
                    assert 536870912 <= pointer < 3758096384
                    memory.seek(pointer)
                    text = memory.read(256).split(b'\x00', 1)[0].decode('cp437')
                    items.append(dict(index=index, text=text, value=u32(base + index * 30 + 20)))
                controls['menu'] = dict(items=items, highlight=u32(symbols['loc_1F6628']), committed=u32(symbols['loc_1F6608']))
        r.update(controls=controls, step=label, seconds=round(time.monotonic() - began, 3))
        report['snapshots'].append(r)
        return r

    def key(name, duration=0.15):
        subprocess.run(['xdotool', 'keydown', name], check=True, timeout=5)
        try:
            time.sleep(duration)
        finally:
            subprocess.run(['xdotool', 'keyup', name], check=True, timeout=5)
        time.sleep(0.15)

    def capture(name):
        source = root / 'game-frame-latest.ppm'
        deadline = time.monotonic() + 5
        requested_ns = time.time_ns()
        while time.monotonic() < deadline:
            try:
                stamp = source.stat().st_mtime_ns
                data = source.read_bytes()
                magic, dim, maxval, pixels = data.split(b'\n', 3)
                w, h = map(int, dim.split())
                if stamp > requested_ns and magic == b'P6' and (maxval == b'255') and (len(pixels) == w * h * 3) and (stamp == source.stat().st_mtime_ns):
                    path = root / (args.label + '-' + name + '.ppm')
                    path.write_bytes(data)
                    report.setdefault('captures', {})[name] = {'sha256': hashlib.sha256(data).hexdigest(), 'mtime_ns': stamp}
                    return
            except (OSError, ValueError):
                pass
            time.sleep(0.04)
        raise TimeoutError('Complete capture ' + name)
    try:
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            result = subprocess.run(['xdotool', 'search', '--name', '^Redguard native experiment$'], capture_output=True, text=True, timeout=5)
            if result.returncode == 0:
                subprocess.run(['xdotool', 'windowfocus', '--sync', result.stdout.splitlines()[0]], check=True, timeout=5)
                break
            assert process.poll() is None
            time.sleep(0.1)
        book.wait_ready(timeout=40)
        book.select_main_action(-1, key)
        key('Return')
        deadline = time.monotonic() + 35
        ready_since = None
        while time.monotonic() < deadline:
            current = snap('initializing')
            player = next((x for x in current['actors'] if x['selected']), None)
            if player and player['health'] > 0 and (player['animation'] is not None) and (current['ui_mode'] == 31) and (current['startup_remaining'] == 0):
                if ready_since is None:
                    ready_since = time.monotonic()
                elif time.monotonic() - ready_since > 1:
                    break
            else:
                ready_since = None
            time.sleep(0.1)
        else:
            raise TimeoutError('Initialized island')
        assert 'maps\\island.rgm' in out.with_suffix('.log').read_text().lower(), 'Island map was not opened'
        current = snap('island-loaded')
        capture('loaded')
        player = next((x for x in current['actors'] if x['selected']))
        report['reloaded_player'] = player
        initial_health = player['health']
        report['waypoints'] = []
        for wx, wz in [(36900, 38800), (36700, 38800), (36700, 38656), (36451, 38656), (36188, 38656), (36188, 39725)]:
            destination = [wx * 256, wz * 256]
            last = None
            stuck = 0
            result = {'destination': destination}
            for attempt in range(100):
                current = snap('walk')
                player = next((x for x in current['actors'] if x['selected']))
                dx = destination[0] - player['position'][0]
                dz = destination[1] - player['position'][2]
                distance = math.hypot(dx, dz)
                bearing = round(math.atan2(-dx, dz) * 2048 / (2 * math.pi)) % 2048
                error = (bearing - player['heading'] + 1024) % 2048 - 1024
                report['decisions'].append(dict(destination=destination, distance=distance, error=error, position=player['position']))
                if abs(error) > 35:
                    key('Left' if error > 0 else 'Right', 0.1)
                elif distance > 10000:
                    key('Up', 0.15)
                    stuck = stuck + 1 if last == player['position'] else 0
                    if stuck >= 6:
                        result['stuck'] = True
                        break
                else:
                    result['reached'] = True
                    break
                last = player['position']
            result['player'] = player
            report['waypoints'].append(result)
            capture('waypoint-' + str(len(report['waypoints'])))
            assert result.get('reached'), ('Waypoint not reached', result)
            assert player['health'] == initial_health, ('Health changed while crossing gangway', player)
        report['boat_to_dock_verified'] = True
        key('Escape', 0.3)
        book.wait_ready()
        book.select_main_action(-2, key)
        key('Return')
        report['quit_exit'] = process.wait(timeout=15)
        assert report['quit_exit'] == 0
    except Exception as error:
        report['error'] = repr(error)
        raise
    finally:
        report['source_save_unchanged'] = hashlib.sha256(source_save.read_bytes()).hexdigest() == source_hash
        report['natural_exit'] = process.poll()
        if process.poll() is None:
            process.kill()
            process.wait()
        report['seconds'] = round(time.monotonic() - began, 3)
        report['blocked'] = 'BLOCKED' in out.with_suffix('.log').read_text()
        out.with_suffix('.json').write_text(json.dumps(report, indent=2) + '\n')
assert report['source_save_unchanged'] and (not report['blocked']), 'Source changed or blocked service'
print(json.dumps({k: v for k, v in report.items() if k not in ['snapshots', 'decisions']}, indent=2))
