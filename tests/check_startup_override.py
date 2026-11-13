"""Check ignore_autosave=1 in an isolated native Linux game scenario."""
import configparser
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

from book_menu import BookMenu
from combat_trace import CombatTrace


def main():
    root = Path('/work')
    scenario = root / 'override-scenario'
    config = configparser.ConfigParser(strict=False)
    config.read(scenario / 'MENU.INI')
    assert config.getint('general', 'ignore_autosave') == 1
    assert not (scenario / 'SAVEGAME/SAVEGAME.001/SAVEGAME.SAV').exists()
    symbols = {}
    for line in (root / 'linux-symbols.txt').read_text().splitlines():
        fields = line.split()
        if len(fields) == 3:
            symbols[fields[2]] = int(fields[0], 16)
    began = time.monotonic()
    with (root / 'startup-override.log').open('w') as log:
        process = subprocess.Popen(
            [str(root / 'redguard-native'), '--game-dir', str(scenario),
             '--user-dir', str(root / 'override-user')],
            env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1'),
            stdout=log, stderr=subprocess.STDOUT,
        )
        try:
            trace = CombatTrace(process, symbols)
            deadline = began + 45
            while time.monotonic() < deadline:
                assert process.poll() is None, 'Game stopped before direct startup'
                try:
                    snapshot = trace.snapshot()
                except (OSError, RuntimeError):
                    time.sleep(.05)
                    continue
                assert snapshot['ui_mode'] != 30, 'Override unexpectedly entered startup book'
                if snapshot['ui_mode'] == 31 and snapshot['selected'] != '0x0':
                    break
                time.sleep(.05)
            else:
                raise TimeoutError('Original direct-world startup')
            window = subprocess.check_output(
                ['xdotool', 'search', '--name', '^Redguard native experiment$'],
                text=True, timeout=5,
            ).splitlines()[0]
            subprocess.run(['xdotool', 'windowfocus', '--sync', window], check=True, timeout=5)

            def key(name):
                subprocess.run(['xdotool', 'keydown', name], check=True, timeout=5)
                time.sleep(.15)
                subprocess.run(['xdotool', 'keyup', name], check=True, timeout=5)
                time.sleep(.5)

            time.sleep(1)
            key('Escape')
            book = BookMenu(process, root / 'linux-symbols.txt')
            book.wait_ready()
            book.select_main_action(-2, key)
            key('Return')
            code = process.wait(timeout=15)
            assert code == 0, code
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
    text = (root / 'startup-override.log').read_text()
    assert 'OPEN D:\\INTRO.SMK' not in text.upper(), 'Override unexpectedly opened intro'
    assert 'BLOCKED' not in text
    report = dict(
        exe_sha256=hashlib.sha256((root / 'redguard-native').read_bytes()).hexdigest(),
        ignore_autosave_direct_start_verified=True, original_quit_exit=code,
        elapsed_seconds=round(time.monotonic() - began, 3), wine_used=False,
        native_windows_verified=False,
    )
    (root / 'startup-override.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
