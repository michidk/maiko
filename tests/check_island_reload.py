"""Reload a real conversation save and exercise its original journal/inventory.

Run under Xvfb. The scenario must contain the autosave and LOGBOOK.TXT from
probe_island_conversation.py. Use a fresh user directory and its matching report.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

from book_menu import BookMenu
from combat_trace import CombatTrace


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/work'))
    parser.add_argument('--game-dir', type=Path, required=True)
    parser.add_argument('--user-dir', type=Path, required=True)
    parser.add_argument('--conversation-report', type=Path, required=True)
    args = parser.parse_args()
    if args.user_dir.exists():
        parser.error('Use a fresh isolated user directory')
    slot = args.game_dir / 'SAVEGAME/SAVEGAME.001'
    source = {name: digest(slot / name) for name in ('SAVEGAME.SAV', 'LOGBOOK.TXT')}
    previous = json.loads(args.conversation_report.read_text())
    if not previous.get('conversation_control_restored') or previous.get('quit_exit') != 0:
        parser.error('The conversation report must record restored control and clean Quit')
    expected = next(actor for actor in previous['snapshots'][-1]['actors'] if actor['selected'])
    journal = (slot / 'LOGBOOK.TXT').read_bytes()
    if b'Captain Brennan' not in journal or b'Draggin Tale' not in journal:
        parser.error('Source journal lacks the original Brennan directions')
    root = args.root
    symbols = {fields[2]: int(fields[0], 16)
               for line in (root / 'linux-symbols.txt').read_text().splitlines()
               if len(fields := line.split()) == 3}
    report = dict(exe_sha256=digest(root / 'redguard-native'),
                  source_files_sha256=source, conversation_report_sha256=digest(args.conversation_report),
                  source_sha256={name: digest(Path(__file__).with_name(name)) for name in
                                 ('check_island_reload.py', 'combat_trace.py', 'book_menu.py')},
                  snapshots=[], captures={}, full_game_verified=False,
                  native_windows_verified=False, wine_used=False)
    assert report['exe_sha256'] == previous['exe_sha256'], 'Use the conversation executable'
    began = time.monotonic()
    deadline = began + 80
    log_path = root / 'island-reload-check.log'
    with log_path.open('w') as log:
        process = subprocess.Popen(
            [str(root / 'redguard-native'), '--game-dir', str(args.game_dir),
             '--user-dir', str(args.user_dir)], stdout=log, stderr=subprocess.STDOUT,
            env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1', RG_CAPTURE_DIR=str(root)))
        trace = CombatTrace(process, symbols)
        book = BookMenu(process, root / 'linux-symbols.txt')

        def snapshot(label):
            if time.monotonic() > deadline:
                raise TimeoutError('Island reload deadline')
            for attempt in range(10):
                try:
                    current = trace.snapshot()
                    break
                except (OSError, RuntimeError):
                    if attempt == 9:
                        raise
                    time.sleep(.02)
            current.update(step=label, seconds=round(time.monotonic() - began, 3))
            report['snapshots'].append(current)
            return current

        def key(name, duration=.15):
            subprocess.run(['xdotool', 'keydown', name], check=True, timeout=5)
            try:
                time.sleep(duration)
            finally:
                subprocess.run(['xdotool', 'keyup', name], check=True, timeout=5)
            time.sleep(.15)

        def capture(label):
            requested = time.time_ns()
            until = time.monotonic() + 5
            source_frame = root / 'game-frame-latest.ppm'
            while time.monotonic() < until:
                try:
                    stamp = source_frame.stat().st_mtime_ns
                    data = source_frame.read_bytes()
                    magic, dimensions, maximum, pixels = data.split(b'\n', 3)
                    width, height = map(int, dimensions.split())
                    if (stamp > requested and stamp == source_frame.stat().st_mtime_ns
                            and magic == b'P6' and maximum == b'255'
                            and len(pixels) == width * height * 3):
                        path = root / ('island-reload-check-' + label + '.ppm')
                        path.write_bytes(data)
                        report['captures'][label] = dict(sha256=digest(path), mtime_ns=stamp)
                        return
                except (OSError, ValueError):
                    pass
                time.sleep(.04)
            raise TimeoutError('Fresh complete frame: ' + label)

        try:
            book.wait_ready(timeout=40)
            result = subprocess.run(['xdotool', 'search', '--name', '^Redguard native experiment$'],
                                    capture_output=True, text=True, check=True, timeout=5)
            subprocess.run(['xdotool', 'windowfocus', '--sync', result.stdout.splitlines()[0]],
                           check=True, timeout=5)
            book.select_main_action(-1, key)
            key('Return')
            ready_since = None
            while time.monotonic() < deadline:
                current = snapshot('load')
                player = next((actor for actor in current['actors'] if actor['selected']), None)
                if (player and player['health'] > 0 and player['animation'] is not None
                        and current['ui_mode'] == 31 and current['startup_remaining'] == 0):
                    if ready_since is None:
                        ready_since = time.monotonic()
                    elif time.monotonic() - ready_since >= 1:
                        break
                else:
                    ready_since = None
                time.sleep(.1)
            else:
                raise TimeoutError('Initialized island after Continue')
            assert player['position'] == expected['position'], (player, expected)
            assert player['health'] == expected['health'], (player, expected)
            assert (args.user_dir / 'LOGBOOK.TXT').read_bytes() == journal
            report['original_journal_reloaded'] = True
            for label, button, mode in [('journal', 'l', 30), ('journal-close', 'Escape', 31),
                                        ('inventory', 'i', 28), ('inventory-close', 'Escape', 31)]:
                key(button, .3 if button == 'Escape' else .15)
                time.sleep(2)
                assert snapshot(label)['ui_mode'] == mode, label
                capture(label)
            key('Escape', .3)
            book.wait_ready()
            book.select_main_action(-2, key)
            key('Return')
            report['quit_exit'] = process.wait(timeout=15)
            assert report['quit_exit'] == 0
            assert (args.user_dir / 'LOGBOOK.TXT').read_bytes() == journal
            report['journal_preserved'] = True
        except Exception as error:
            report['error'] = repr(error)
            raise
        finally:
            report['natural_exit'] = process.poll()
            if process.poll() is None:
                process.kill()
                process.wait()
            report['seconds'] = round(time.monotonic() - began, 3)
            report['source_unchanged'] = all(digest(slot / name) == value for name, value in source.items())
            report['blocked'] = 'BLOCKED' in log_path.read_text()
            (root / 'island-reload-check.json').write_text(json.dumps(report, indent=2) + '\n')
    assert report['source_unchanged'] and not report['blocked']
    print('PASS separate-process island position/health, original journal, inventory and Quit')


if __name__ == '__main__':
    main()
