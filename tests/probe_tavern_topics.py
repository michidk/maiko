"""Walk up to the nearest characters, talk to them and ask every topic a character offers.

By default the probe enters the Draggin Tale; with --island it stays on the docked island.

Run under Xvfb with a docked-island autosave already in the scenario. The tavern is entered
through its real door (the island is loaded at the marker beside it with the original console and
the door is activated with the action key); the console then places the player beside Tobias.
This probe sends keyboard input and reads guest memory without modifying it.
"""
import argparse, hashlib, json, math, os, subprocess, time
from pathlib import Path
from book_menu import BookMenu
from combat_trace import CombatTrace
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--root', type=Path, default=Path('/work'))
parser.add_argument('--game-dir', default='/work/menu-scenario')
parser.add_argument('--user-dir', required=True, type=Path)
parser.add_argument('--label', default='tavern-topics')
parser.add_argument('--island', action='store_true', help='Talk to characters on the docked island instead of entering the tavern')
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
report['source_sha256'] = {name: hashlib.sha256(Path(__file__).with_name(name).read_bytes()).hexdigest() for name in ['probe_tavern_topics.py', 'combat_trace.py', 'book_menu.py']}
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

    def snap_retry(label, tries=80):
        # The actor list changes while an NPC answers; wait it out instead of failing.
        for _ in range(tries):
            try:
                return snap(label)
            except (RuntimeError, OSError):
                time.sleep(0.1)
        return snap(label)

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
            try:
                current = trace.snapshot()
            except (RuntimeError, OSError):
                time.sleep(0.1)
                continue
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
        if current['controls']['lockout'] or current['controls']['dialogue'] or current['controls']['topics']:
            key('Control_L', 0.2)
            time.sleep(2)

        names = {' ': 'space', ',': 'comma', '*': 'asterisk'}

        def console(command):
            # xdotool type drops characters; single presses at this pace do not.
            key('F12', 0.2)
            time.sleep(1.5)
            for ch in command:
                subprocess.run(['xdotool', 'keydown', names.get(ch, ch)], check=True, timeout=5)
                time.sleep(0.09)
                subprocess.run(['xdotool', 'keyup', names.get(ch, ch)], check=True, timeout=5)
                time.sleep(0.22)
            time.sleep(0.4)
            key('Return')
            time.sleep(1.2)
            key('Escape', 0.2)
            time.sleep(1.0)

        def log_count(name):
            return out.with_suffix('.log').read_text(errors='replace').lower().count(name)

        def prompt_visible():
            requested = time.time_ns()
            for _ in range(100):
                if source_frame.exists() and source_frame.stat().st_mtime_ns > requested:
                    time.sleep(0.1)
                    px = source_frame.read_bytes()[-1024 * 768 * 3:]
                    hits = 0
                    for y in range(715, 768):
                        row = y * 1024 * 3
                        for x in range(0, 420):
                            o = row + x * 3
                            if px[o] > 200 and 120 < px[o + 1] < 230 and px[o + 2] < 90:
                                hits += 1
                    return hits > 30
                time.sleep(0.05)
            raise TimeoutError('Frame')

        def face_prompt():
            for turn in range(14):
                if prompt_visible():
                    return turn
                key('Right', 0.3)
                time.sleep(0.5)
            raise AssertionError('Activation prompt never appeared')

        source_frame = root / 'game-frame-latest.ppm'
        if not args.island:
            # Enter the tavern through its real door: load the island at the marker beside it.
            islands = log_count('island.rgm')
            console('load 1,8,0')
            for _ in range(40):
                if log_count('island.rgm') > islands:
                    break
                time.sleep(0.5)
            report['door_prompt_after_turns'] = face_prompt()
            taverns = log_count('tavern.rgm')
            # Activation fires only while the key is held at the moment the door script checks it, so a
            # press can miss; the prompt stays on screen, so press again.
            for press in range(3):
                key('Control_L', 0.4)
                for _ in range(30):
                    if log_count('tavern.rgm') > taverns:
                        break
                    time.sleep(0.5)
                if log_count('tavern.rgm') > taverns:
                    break
            report['door_presses'] = press + 1
            for _ in range(120):
                if log_count('tavern.rgm') > taverns:
                    break
                time.sleep(0.5)
            assert log_count('tavern.rgm') > taverns, 'Tavern did not open'
            deadline = time.monotonic() + 60
            while time.monotonic() < deadline:
                try:
                    current = snap_retry('tavern-loaded')
                    if current['ui_mode'] == 31 and current['startup_remaining'] == 0 and current['controls']['lockout'] == 0:
                        break
                except Exception:
                    pass
                time.sleep(0.5)
            capture('tavern')
            # The console only gets the player near the bar. Each living character, nearest first, is then
            # approached and spoken to: some answer with one line, those with a topic menu are asked every
            # topic they offer and then dismissed with Bye.
            console('object search tobias')
            console('object goto')
        spoken = set()
        report['conversations'] = []

        def living_targets():
            cur = snap_retry('characters')
            pl = next(x for x in cur['actors'] if x['selected'])
            ts = [x for x in cur['actors'] if not x['selected'] and x['protected'] and x['health'] > 0
                  and x['lifecycle'] != 8 and x['actor'] not in spoken]
            return sorted(ts, key=lambda x: math.dist(pl['position'], x['position']))

        def approach(target):
            stuck, last = 0, None
            for _ in range(160):
                cur = snap_retry('approach')
                pl = next(x for x in cur['actors'] if x['selected'])
                tg = next((x for x in cur['actors'] if x['actor'] == target['actor']), None)
                if tg is None:
                    return False
                dx, dz = tg['position'][0] - pl['position'][0], tg['position'][2] - pl['position'][2]
                error = (round(math.atan2(-dx, dz) * 2048 / (2 * math.pi)) % 2048 - pl['heading'] + 1024) % 2048 - 1024
                if abs(error) > 40:
                    key('Left' if error > 0 else 'Right', 0.12)
                elif math.dist(pl['position'], tg['position']) > 18000:
                    key('Up', 0.15)
                    stuck = stuck + 1 if last == pl['position'] else 0
                    if stuck >= 8:
                        return False
                else:
                    return True
                last = pl['position']
            return False

        def wait_free(timeout=40):
            end = time.monotonic() + timeout
            while time.monotonic() < end:
                cur = snap_retry('free')
                if all(cur['controls'][x] == 0 for x in ['lockout', 'dialogue', 'topics', 'sound_inhibited']):
                    return True
                time.sleep(0.3)
            return False

        def choose_topic(name):
            end = time.monotonic() + 10
            while time.monotonic() < end:
                menu = snap_retry('select-' + name)['controls']['menu']
                wanted = next(x for x in menu['items'] if name in x['text'].upper())
                if menu['highlight'] == wanted['index']:
                    key('Control_L', 0.15)
                    return
                key('Down', 0.1)
            raise TimeoutError('Select ' + name)

        def menu_texts():
            return [x['text'].upper() for x in snap_retry('menu')['controls']['menu']['items']]

        def wait_menu_back():
            end = time.monotonic() + 60
            seen_closed = False
            while time.monotonic() < end:
                cur = snap_retry('topic-response')
                seen_closed |= cur['controls']['topics'] == 0
                if seen_closed and cur['controls']['topics'] == 1:
                    return
                time.sleep(0.2)
            raise TimeoutError('Topic response and menu return')

        for attempt in range(8):
            candidates = living_targets()
            if not candidates:
                break
            target = candidates[0]
            spoken.add(target['actor'])
            entry = {'actor': target['actor'], 'approached': approach(target)}
            report['conversations'].append(entry)
            if not entry['approached']:
                continue
            capture(f'before-{attempt}')
            key('Control_L', 0.3)
            entry['has_topic_menu'] = False
            for _ in range(40):
                if snap_retry('talk')['controls']['topics'] == 1:
                    entry['has_topic_menu'] = True
                    break
                time.sleep(0.3)
            if entry['has_topic_menu']:
                entry['menus'] = [menu_texts()]
                entry['asked'] = []
                for _ in range(14):
                    choices = [t for t in entry['menus'][-1] if 'BYE' not in t and t not in entry['asked']]
                    if not choices:
                        break
                    entry['asked'].append(choices[0])
                    choose_topic(choices[0])
                    wait_menu_back()
                    entry['menus'].append(menu_texts())
                choose_topic('BYE')
            entry['control_restored'] = wait_free()
            capture(f'after-{attempt}')
        conversations = report['conversations']
        assert any(c['approached'] for c in conversations), 'Reached no character'
        assert all(c['control_restored'] for c in conversations if c['approached']), 'Control not restored after a conversation'
        report['topic_menu_observed'] = any(c.get('has_topic_menu') for c in conversations)
        assert report['topic_menu_observed'], 'No character offered topics'
        report['topic_response_observed'] = True
        current = snap_retry('after-conversations')
        before = next((x for x in current['actors'] if x['selected']))['heading']
        key('Right', 0.25)
        current = snap('restored-turn')
        assert next((x for x in current['actors'] if x['selected']))['heading'] != before
        report['conversation_control_restored'] = True
        capture('conversation-exit')
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
assert report['source_save_unchanged'], 'Source save changed'
print(json.dumps({k: v for k, v in report.items() if k not in ['snapshots', 'decisions']}, indent=2))
