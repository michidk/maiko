"""Fight the original ship encounter using keyboard input and observe island arrival.

Run inside the SDL test container under Xvfb, with a disposable game scenario
without an autosave and a separate writable user directory. No guest memory is
written. This is a bounded encounter check, not full-game acceptance.
"""
import argparse
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import time

from book_menu import BookMenu
from combat_trace import CombatTrace


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-dir', type=Path, default=Path('/work'))
    parser.add_argument('--game-dir', default='/work/menu-scenario')
    parser.add_argument('--user-dir', required=True)
    parser.add_argument('--seconds', type=float, default=220)
    args = parser.parse_args()
    if not 30 <= args.seconds <= 240:
        parser.error('--seconds must be between 30 and 240')
    for directory in (Path(args.game_dir), Path(args.user_dir)):
        if (directory / 'SAVEGAME/SAVEGAME.001/SAVEGAME.SAV').exists():
            parser.error('Ship encounter requires a fresh scenario and user directory without an autosave')
    lock = open('/tmp/redguard-game-probe.lock', 'w')
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    root = Path('/work')
    runtime = args.runtime_dir.resolve()
    symbols = {}
    for line in (root / 'linux-symbols.txt').read_text().splitlines():
        fields = line.split()
        if len(fields) == 3:
            symbols[fields[2]] = int(fields[0], 16)
    report = dict(
        exe_sha256=hashlib.sha256((runtime / 'redguard-native').read_bytes()).hexdigest(),
        wine_used=False, native_windows_verified=False, full_game_verified=False,
        snapshots=[], decisions=[], peak_rss_kib=0,
        rss_measurement='maximum sampled VmRSS, not kernel high-water mark',
        source_sha256={name: hashlib.sha256(Path(__file__).with_name(name).read_bytes()).hexdigest()
                       for name in ('probe_ship_combat.py', 'combat_trace.py', 'book_menu.py')},
    )
    began = time.monotonic()
    launched_ns = time.time_ns()
    deadline = began + args.seconds
    environment = dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1', RG_CAPTURE_DIR=str(root))
    with (root / 'ship-combat.log').open('w') as log:
        process = subprocess.Popen(
            [str(runtime / 'redguard-native'), '--game-dir', args.game_dir,
             '--user-dir', args.user_dir], env=environment,
            stdout=log, stderr=subprocess.STDOUT,
        )
        trace = CombatTrace(process, symbols)

        def require(condition, message):
            if not condition:
                raise RuntimeError(message)

        def sample(step):
            require(time.monotonic() < deadline, 'Ship encounter observation deadline')
            status = Path(f'/proc/{process.pid}/status').read_text()
            for line in status.splitlines():
                if line.startswith('VmRSS:'):
                    report['peak_rss_kib'] = max(report['peak_rss_kib'], int(line.split()[1]))
            snapshot = trace.snapshot()
            snapshot.update(seconds=round(time.monotonic() - began, 3), step=step)
            report['snapshots'].append(snapshot)
            return snapshot

        def hold(key, duration, step, observe=True):
            subprocess.run(['xdotool', 'keydown', key], check=True, timeout=5)
            try:
                end = time.monotonic() + duration
                while time.monotonic() < end and process.poll() is None:
                    if observe:
                        sample(step)
                    time.sleep(.04)
            finally:
                subprocess.run(['xdotool', 'keyup', key], check=True, timeout=5)

        def capture(name, after_ns, settled=False):
            source = root / 'game-frame-latest.ppm'
            end = time.monotonic() + (18 if settled else 3)
            previous = None
            previous_stamp = None
            while time.monotonic() < end:
                try:
                    before = source.stat()
                    data = source.read_bytes()
                    after = source.stat()
                    magic, dimensions, maximum, pixels = data.split(b'\n', 3)
                    width, height = map(int, dimensions.split())
                    if (before.st_mtime_ns == after.st_mtime_ns and after.st_mtime_ns > after_ns
                            and magic == b'P6' and maximum == b'255' and width > 0 and height > 0
                            and len(pixels) == width * height * 3):
                        digest = hashlib.sha256(data).hexdigest()
                        if settled:
                            if len(set(pixels)) <= 16:
                                previous = None
                                time.sleep(.05)
                                continue
                            if digest != previous or after.st_mtime_ns == previous_stamp:
                                previous = digest
                                previous_stamp = after.st_mtime_ns
                                time.sleep(.05)
                                continue
                        (root / f'ship-combat-{name}.ppm').write_bytes(data)
                        report.setdefault('captures', {})[name] = dict(
                            sha256=digest, mtime_ns=after.st_mtime_ns,
                            required_after_ns=after_ns, width=width, height=height,
                            settled_nonblank=settled)
                        return
                except (FileNotFoundError, ValueError):
                    pass
                # OpenGLide writes the shared file in place. Retry if the read
                # overlaps its truncation or a partially written frame.
                time.sleep(.05)
            raise RuntimeError('Original renderer did not export a complete frame')

        try:
            window_deadline = time.monotonic() + 30
            while time.monotonic() < window_deadline:
                require(process.poll() is None, 'Original game stopped before its SDL window')
                found = subprocess.run(
                    ['xdotool', 'search', '--name', '^Redguard native experiment$'],
                    capture_output=True, text=True, timeout=5,
                )
                if found.returncode == 0:
                    subprocess.run(['xdotool', 'windowfocus', '--sync', found.stdout.splitlines()[0]],
                                   check=True, timeout=5)
                    break
                time.sleep(.1)
            else:
                raise TimeoutError('Original SDL window')
            book = BookMenu(process, root / 'linux-symbols.txt')
            book.wait_ready(timeout=40)
            report['startup_book_before_input_verified'] = True
            require(not book.continue_available(), 'Continue available in a fresh scenario')
            report['startup_continue_unavailable_verified'] = True
            require(book.main_entry() == {'action': 10, 'grayed': False},
                    'Fresh book must initially select New Game')
            report['startup_new_selected_verified'] = True
            capture('startup-book', launched_ns, settled=True)
            def menu_key(name):
                hold(name, .15, 'menu-select', observe=False)
                time.sleep(.5)
            book.select_main_action(10, menu_key)
            hold('Return', .15, 'new-game-confirm', observe=False)
            time.sleep(2)
            # Respect the original movie's configured skip points. Installed-GOG
            # discovery supplies media; the probe passes no media argument.
            for index in range(6):
                current = sample('intro')
                if current['selected'] != '0x0':
                    break
                hold('Escape', .6, 'intro-skip')
                time.sleep(2)
            time.sleep(3)
            first = sample('fight-start')
            player = next((x for x in first['actors'] if x['selected']), None)
            require(player is not None and first['ui_mode'] == 31, 'Original ship player not ready')
            spawn = player['position']
            enemies = {x['actor'] for x in first['actors']
                       if not x['selected'] and x['lifecycle'] == 6 and x['health'] > 0}
            require(len(enemies) == 2, f'Expected two original pirates, got {enemies}')
            report['initial_enemy_ids'] = sorted(enemies)
            capture('ship', launched_ns)
            while True:
                current = sample('fight')
                player = next((x for x in current['actors'] if x['selected']), None)
                require(player is not None and player['health'] > 0, 'Player lost during encounter')
                observed = {x['actor']: x for x in current['actors'] if x['actor'] in enemies}
                require(len(observed) == 2, 'Pirate disappeared before observed defeat')
                alive = [x for x in observed.values() if x['health'] > 0]
                if not alive:
                    # Health reaches zero before the death animation completes.
                    if all(x['lifecycle'] == 8 for x in observed.values()):
                        report['both_pirates_defeated_verified'] = True
                        report['defeated_enemies'] = list(observed.values())
                        break
                    time.sleep(.1)
                    continue
                target = min(alive, key=lambda x: math.dist(player['position'], x['position']))
                dx = target['position'][0] - player['position'][0]
                dz = target['position'][2] - player['position'][2]
                bearing = round(math.atan2(-dx, dz) * 2048 / (2 * math.pi)) % 2048
                error = (bearing - player['heading'] + 1024) % 2048 - 1024
                distance = math.dist(player['position'], target['position'])
                report['decisions'].append(dict(target=target['actor'], distance=round(distance, 1),
                                                heading=player['heading'], bearing=bearing, error=error))
                if abs(error) > 64:
                    hold('Left' if error > 0 else 'Right', .12, 'turn')
                elif distance > 30000:
                    hold('Up', .15, 'approach')
                else:
                    hold('Control_L', .16, 'attack')
                time.sleep(.12)
            arrived_at = None
            while True:
                # Loading frees and rebuilds the original actor list. Do not
                # interpret a transient unreadable allocation as a game fault.
                try:
                    current = sample('after-defeat')
                except OSError:
                    report['transient_loading_reads'] = report.get('transient_loading_reads', 0) + 1
                    require(process.poll() is None, 'Original game stopped during island loading')
                    time.sleep(.2)
                    continue
                text = (root / 'ship-combat.log').read_text().lower()
                player = next((x for x in current['actors'] if x['selected']), None)
                arrived = ('open maps\\island.rgm' in text and player is not None
                           and current['ui_mode'] == 31 and player['health'] > 0
                           and current['startup_remaining'] == 0
                           and math.dist(player['position'], spawn) > 1000000)
                if arrived:
                    if arrived_at is None:
                        arrived_at = time.monotonic()
                        arrived_ns = time.time_ns()
                    elif time.monotonic() - arrived_at > 3:
                        report['island_arrival_verified'] = True
                        report['island_clock_warmup_finished_verified'] = True
                        report['island_player'] = player
                        capture('island', arrived_ns)
                        break
                else:
                    arrived_at = None
                time.sleep(.2)
            hold('Escape', .6, 'book')
            BookMenu(process, root / 'linux-symbols.txt').wait_ready()
            book.select_main_action(-2, menu_key)
            hold('Return', .15, 'quit-confirm', observe=False)
            code = process.wait(timeout=20)
            report['quit_exit'] = code
            require(code == 0, f'Original Quit exited {code}')
            report['ship_to_island_verified'] = True
        except Exception as error:
            report['harness_error'] = repr(error)
            raise
        finally:
            report['natural_exit'] = process.poll()
            if process.poll() is None:
                process.kill()
                process.wait()
                report['intentional_termination'] = 'incomplete_encounter_observation'
            report['elapsed_seconds'] = round(time.monotonic() - began, 3)
            report['blocked'] = 'BLOCKED' in (root / 'ship-combat.log').read_text()
            (root / 'ship-combat.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k not in ('snapshots', 'decisions')}, indent=2))


if __name__ == '__main__':
    main()
