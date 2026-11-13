"""Check that the island's CD music comes out of the game's audio output.

Run under Xvfb with an island autosave in the scenario, SYSTEM.INI with `sound=1` and `redbook=on`,
and `game.gog` with `game.ins` beside the game files. The game runs with SDL's disk audio driver, which
records the mixed output. The probe then reads the log for the track the game chose, finds that track
on the disc image, and requires the recording to contain it: at a few moments the recorded left channel
is correlated with the disc's left channel around the expected position, and the best match must
correlate strongly (other game sounds are mixed in, so not perfectly) with a gain near the game's
music volume. Linux software-GL and a recording audio device only; not Windows or hardware playback.
"""
import argparse, array, json, math, os, re, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, '/tests')
from book_menu import BookMenu

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, default=Path('/work'))
p.add_argument('--game-dir', default='/work/music-scenario')
p.add_argument('--user-dir', required=True, type=Path)
p.add_argument('--disc', type=Path, default=Path('/gog/game.gog'))
p.add_argument('--cue', type=Path, default=Path('/gog/game.ins'))
p.add_argument('--seconds', type=int, default=50)
p.add_argument('--label', default='cd-music')
a = p.parse_args()
if a.user_dir.exists():
    p.error('Use a fresh isolated user directory')
root = a.root
log_path, raw_path = root / (a.label + '.log'), root / (a.label + '.raw')
report = {'full_game_verified': False, 'native_windows_verified': False, 'wine_used': False}


def start_sector(track):
    sector, current = None, 0
    for line in a.cue.read_text().splitlines():
        words = line.split()
        if len(words) >= 2 and words[0].upper() == 'TRACK':
            current = int(words[1])
        elif len(words) >= 3 and words[0].upper() == 'INDEX' and words[1] == '01' and current == track:
            m, s, f = map(int, words[2].split(':'))
            return (m * 60 + s) * 75 + f


def correlate(recorded, disc, at, lag_center, window=4000, span=24000, step=4):
    """Best normalised correlation of recorded[at:at+window] with the disc near lag_center."""
    w = recorded[at:at + window:step]
    wn = math.sqrt(sum(x * x for x in w)) or 1.0
    best = (0.0, 0, 1.0)
    for lag in range(max(0, lag_center - span), lag_center + span, step):
        seg = disc[lag:lag + window:step]
        if len(seg) != len(w):
            continue
        sn = math.sqrt(sum(x * x for x in seg)) or 1.0
        c = sum(x * y for x, y in zip(w, seg)) / (wn * sn)
        if c > best[0]:
            best = (c, lag, sum(x * y for x, y in zip(w, seg)) / (sum(y * y for y in seg) or 1.0))
    return best


with log_path.open('w') as log:
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1', SDL_AUDIODRIVER='disk', SDL_DISKAUDIOFILE=str(raw_path))
    proc = subprocess.Popen([str(root / 'redguard-native'), '--game-dir', a.game_dir, '--user-dir', str(a.user_dir)],
                            stdout=log, stderr=subprocess.STDOUT, env=env)
    book = BookMenu(proc, root / 'linux-symbols.txt')

    def key(name, hold=0.15):
        subprocess.run(['xdotool', 'keydown', name], check=True, timeout=5)
        time.sleep(hold)
        subprocess.run(['xdotool', 'keyup', name], check=True, timeout=5)
        time.sleep(0.2)

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
        time.sleep(a.seconds)
    finally:
        proc.terminate()
        try:
            proc.wait(10)
        except subprocess.TimeoutExpired:
            proc.kill()

text = log_path.read_text(errors='replace')
report['disc_found'] = bool(re.search(r'CD music: \d+ (audio )?tracks', text))
m = re.search(r'CD music: playing track (\d+) \((\d+) ms to (\d+) ms\)', text)
report['track'] = int(m.group(1)) if m else None
try:
    assert report['disc_found'], 'The game did not find the disc image'
    assert m, 'The game never started a music track'
    track = report['track']
    first = start_sector(track)
    recorded = array.array('h')
    recorded.frombytes(raw_path.read_bytes())
    left = recorded[0::2]
    with a.disc.open('rb') as image:
        image.seek(first * 2352)
        disc = array.array('h')
        disc.frombytes(image.read(2352 * 75 * (a.seconds + 5)))
    disc_left = disc[0::2]
    results = []
    for t in (25, 35):
        # Playback starts a little after the game starts; the recording is continuous from there.
        corr, lag, gain = correlate(left, disc_left, 44100 * t, 44100 * t - 22000)
        results.append(dict(second=t, correlation=round(corr, 3), disc_offset_s=round(lag / 44100, 3), gain=round(gain, 3)))
    report['matches'] = results
    report['music_in_output_verified'] = all(r['correlation'] > 0.8 and 0.5 < r['gain'] < 1.0 for r in results)
    offsets = [r['second'] - r['disc_offset_s'] for r in results]
    report['playback_continuous'] = abs(offsets[0] - offsets[1]) < 0.05
    assert report['music_in_output_verified'], 'The recording does not contain the track'
    assert report['playback_continuous'], 'The track is not continuous'
finally:
    Path(str(root / a.label) + '.json').write_text(json.dumps(report, indent=1))
    print(json.dumps(report))
