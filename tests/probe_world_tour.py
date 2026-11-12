"""Load worlds through the original F12 console and record per-world readiness.

Run under Xvfb with an island autosave in the scenario. Input is keyboard only: each
world is requested with the original `load N,0,0` command, then the probe waits for
the map file to open and for a living, initialized player. Not a Windows or full-game
acceptance; entry markers other than zero are not exercised."""
import argparse, json, os, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, '/tests')
from book_menu import BookMenu
from combat_trace import CombatTrace

p = argparse.ArgumentParser()
p.add_argument('--game-dir', default='/work/island-reload-scenario')
p.add_argument('--user-dir', required=True, type=Path)
p.add_argument('--label', default='world-load')
p.add_argument('--worlds', default='8:drint.rgm')
a = p.parse_args()
root = Path('/work'); out = root / a.label
symbols = {x[2]: int(x[0], 16) for line in (root / 'linux-symbols.txt').read_text().splitlines() if len((x := line.split())) == 3}
report = {'worlds': [], 'full_game_verified': False, 'native_windows_verified': False, 'wine_used': False}
logpath = out.with_suffix('.log')
with logpath.open('w') as log:
    proc = subprocess.Popen([str(root / 'redguard-native'), '--game-dir', a.game_dir, '--user-dir', str(a.user_dir)], stdout=log, stderr=subprocess.STDOUT, env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1', RG_CAPTURE_DIR=str(root)))
    trace = CombatTrace(proc, symbols); book = BookMenu(proc, root / 'linux-symbols.txt')

    def key(name, d=0.15):
        subprocess.run(['xdotool', 'keydown', name], check=True, timeout=5)
        time.sleep(d)
        subprocess.run(['xdotool', 'keyup', name], check=True, timeout=5)
        time.sleep(0.2)

    def state():
        for i in range(10):
            try:
                s = trace.snapshot(); break
            except (OSError, RuntimeError):
                time.sleep(0.05)
        pl = next((x for x in s['actors'] if x['selected']), None)
        return s, pl

    def wait_world(timeout=40):
        end = time.monotonic() + timeout; since = None
        while time.monotonic() < end:
            if proc.poll() is not None: raise RuntimeError('game exited')
            try:
                s, pl = state()
            except Exception:
                time.sleep(0.2); continue
            if pl and pl['health'] > 0 and pl['animation'] is not None and s['ui_mode'] == 31 and s['startup_remaining'] == 0:
                since = since or time.monotonic()
                if time.monotonic() - since > 1: return s, pl
            else: since = None
            time.sleep(0.1)
        raise TimeoutError('world ready')

    def shot(name):
        t = time.time_ns(); src = root / 'game-frame-latest.ppm'
        for _ in range(100):
            if src.exists() and src.stat().st_mtime_ns > t:
                time.sleep(0.1); (root / f'{a.label}-{name}.ppm').write_bytes(src.read_bytes()); return
            time.sleep(0.05)
    try:
        for _ in range(300):
            r = subprocess.run(['xdotool', 'search', '--name', '^Redguard native experiment$'], capture_output=True, text=True, timeout=5)
            if r.returncode == 0:
                subprocess.run(['xdotool', 'windowfocus', '--sync', r.stdout.splitlines()[0]], check=True, timeout=5); break
            time.sleep(0.1)
        book.wait_ready(timeout=40); book.select_main_action(-1, key); key('Return')
        s, pl = wait_world(); report['before'] = pl; shot('before')
        def rss():
            return next(int(l.split()[1]) for l in open(f'/proc/{proc.pid}/status') if l.startswith('VmRSS:'))
        for item in a.worlds.split(','):
            n, expect = item.split(':'); entry = {'world': int(n), 'map': expect}
            before_log = logpath.read_text().lower().count(expect)
            key('F12', 0.2); time.sleep(1.5)
            subprocess.run(['xdotool', 'type', '--delay', '150', f'load {n},0,0'], check=True, timeout=30)
            time.sleep(0.3); key('Return'); time.sleep(1.0); key('Escape', 0.2)
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline and logpath.read_text().lower().count(expect) <= before_log and proc.poll() is None:
                time.sleep(0.3)
            entry['opened'] = logpath.read_text().lower().count(expect) > before_log
            time.sleep(2)
            try:
                s, pl = wait_world(60); entry.update(ready=True, health=pl['health'], position=pl['position'], actors=len(s['actors']), rss_kib=rss())
                shot(f'w{n}')
            except Exception as e:
                entry.update(ready=False, error=repr(e)); report['worlds'].append(entry); break
            report['worlds'].append(entry)
            print(json.dumps(entry), flush=True)
    finally:
        if proc.poll() is None:
            proc.terminate()
            try: proc.wait(10)
            except subprocess.TimeoutExpired: proc.kill()
        report['exit'] = proc.returncode
(out.with_suffix('.json')).write_text(json.dumps(report, indent=1))
print(json.dumps(report["worlds"][-1] if report["worlds"] else {}))
