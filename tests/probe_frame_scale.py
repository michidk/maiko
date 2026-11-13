"""Watch the game's frame scale around a level load and a menu close.

The original derives how far everything moves in an update from a measured frame rate. A load or a
menu stall used to leave that measurement far too low, so everything ran several times too fast for
about a second afterwards. This samples the scale (guest word loc_1EEC50, 256 = one nominal update)
about every 50 ms in steady play, across a console load of an island marker, and across opening and
closing the book menu, along with the update counter (loc_1AF228), and fails when the scale exceeds what
the real frame rate calls for (3072 / fps) by more than `--limit` times. Linux software-GL and dummy audio only; not Windows.
"""
import argparse, json, os, signal, statistics, subprocess, sys, threading, time
from pathlib import Path
sys.path.insert(0, '/tests')
from book_menu import BookMenu

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, default=Path('/work'))
p.add_argument('--game-dir', default='/work/island-reload-scenario')
p.add_argument('--user-dir', required=True, type=Path)
p.add_argument('--label', default='frame-scale')
p.add_argument('--limit', type=float, default=1.6, help='Largest allowed scale relative to what the real frame rate calls for')
p.add_argument('--marker', type=int, default=8, help='Island marker to load')
a = p.parse_args()
if a.user_dir.exists():
    p.error('Use a fresh isolated user directory')
signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
root = a.root
symbols = {x[2]: int(x[0], 16) for line in (root / 'linux-symbols.txt').read_text().splitlines() if len((x := line.split())) == 3}
KEYS = {' ': 'space', ',': 'comma'}
report = {'full_game_verified': False, 'native_windows_verified': False, 'wine_used': False, 'phases': {}}
events, samples, stop = [], [], threading.Event()


def word(memory, name):
    memory.seek(symbols[name])
    return int.from_bytes(memory.read(4), 'little', signed=True)


with (root / (a.label + '.log')).open('w') as log:
    proc = subprocess.Popen([str(root / 'redguard-native'), '--game-dir', a.game_dir, '--user-dir', str(a.user_dir)],
                            stdout=log, stderr=subprocess.STDOUT, env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1'))
    book = BookMenu(proc, root / 'linux-symbols.txt')

    def key(name, hold=0.15):
        subprocess.run(['xdotool', 'keydown', name], check=True, timeout=5)
        time.sleep(hold)
        subprocess.run(['xdotool', 'keyup', name], check=True, timeout=5)
        time.sleep(0.2)

    def sampler():
        with open(f'/proc/{proc.pid}/mem', 'rb', buffering=0) as memory:
            while not stop.is_set():
                try:
                    samples.append((time.monotonic(), word(memory, 'loc_1EEC50'), word(memory, 'loc_1EEC58'), word(memory, 'loc_1EEC5C'), word(memory, 'loc_1AF228')))
                except OSError:
                    pass
                time.sleep(0.05)

    def mark(name):
        events.append((name, time.monotonic()))

    def console(command):
        key('F12', 0.2)
        time.sleep(1.5)
        for ch in command:
            subprocess.run(['xdotool', 'keydown', KEYS.get(ch, ch)], check=True, timeout=5)
            time.sleep(0.09)
            subprocess.run(['xdotool', 'keyup', KEYS.get(ch, ch)], check=True, timeout=5)
            time.sleep(0.22)
        time.sleep(0.4)
        mark('load-issued')
        key('Return')
        time.sleep(1.2)
        key('Escape', 0.2)

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
        time.sleep(15)  # into the world, past the warm-up, and some steady play
        threading.Thread(target=sampler, daemon=True).start()
        mark('steady')
        time.sleep(8)
        console(f'load 1,{a.marker},0')
        mark('after-load')
        time.sleep(10)
        mark('menu-open')
        key('Escape', 0.2)
        time.sleep(4)
        mark('menu-close')
        key('Escape', 0.2)
        time.sleep(8)
        mark('end')
        stop.set()
        marks = dict(events)
        def ratio(i):
            """Scale relative to what the real frame rate calls for: 3072 / fps (normal rate 12 times 256)."""
            now = samples[i]
            before = next((s for s in samples[:i] if now[0] - s[0] <= 0.6), None)
            if not before or now[4] == before[4] or now[0] == before[0]:
                return None
            fps = (now[4] - before[4]) / (now[0] - before[0])
            return now[1] / (3072 / fps)

        for name, begin, end in (('steady', marks['steady'] + 2, marks['load-issued']), ('load', marks['load-issued'], marks['after-load'] + 6),
                                 ('menu', marks['menu-open'], marks['end'])):
            window = [(i, ratio(i)) for i, s in enumerate(samples) if begin <= s[0] <= end]
            window = [(i, r) for i, r in window if r is not None]
            # A one-off slow frame on a busy machine is not a burst: judge the median over about a second.
            smooth = [(i, statistics.median(r for _, r in window[max(0, n - 10):n + 10])) for n, (i, _) in enumerate(window)]
            worst_i, worst = max(smooth, key=lambda x: x[1])
            report['phases'][name] = {'samples': len(window), 'worst_ratio': round(worst, 2), 'at_s': round(samples[worst_i][0] - begin, 1),
                                      'lowest_ratio': round(min(r for _, r in smooth), 2), 'scale': samples[worst_i][1],
                                      'measured': samples[worst_i][3], 'smoothed': samples[worst_i][2]}
        report['ok'] = all(v['worst_ratio'] <= a.limit for v in report['phases'].values())
    except Exception as error:
        report['ok'] = False
        report['error'] = repr(error)
    finally:
        stop.set()
        proc.terminate()
        report['timing_resets'] = [l.split('event=timing-reset ')[1] for l in (root / (a.label + '.log')).read_text(errors='replace').splitlines() if 'event=timing-reset' in l]
        (root / (a.label + '.json')).write_text(json.dumps(report, indent=1))
        print(json.dumps(report))
