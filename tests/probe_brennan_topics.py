"""Walk Brennan's whole topic sequence on the original island and restore ordinary player control.

Run under Xvfb with a docked-island autosave already in the scenario.
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
parser.add_argument('--label', default='brennan-topics')
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
report['source_sha256'] = {name: hashlib.sha256(Path(__file__).with_name(name).read_bytes()).hexdigest() for name in ['probe_brennan_topics.py', 'combat_trace.py', 'book_menu.py']}
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
            current = trace.snapshot()
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
            current = snap('after-initial-action')
            capture('initial-action')
        before = next((x for x in current['actors'] if x['selected']))['heading']
        key('Right', 0.35)
        current = snap('turn-control')
        after = next((x for x in current['actors'] if x['selected']))['heading']
        assert len(current['actors']) > 2, 'Island actor population missing'
        report['ordinary_turn_verified'] = before != after
        assert report['ordinary_turn_verified'], ('Player still has no turn control', current['controls'])
        player = next((x for x in current['actors'] if x['selected']))
        targets = [x for x in current['actors'] if not x['selected'] and x['protected'] and (x['health'] > 0) and (x['lifecycle'] != 8)]
        target = min(targets, key=lambda x: math.dist(player['position'], x['position']))
        report['target'] = target
        stuck = 0
        last = None
        for attempt in range(80):
            current = snap('approach')
            player = next((x for x in current['actors'] if x['selected']))
            target = next((x for x in current['actors'] if x['actor'] == target['actor']))
            dx = target['position'][0] - player['position'][0]
            dz = target['position'][2] - player['position'][2]
            distance = math.dist(player['position'], target['position'])
            bearing = round(math.atan2(-dx, dz) * 2048 / (2 * math.pi)) % 2048
            error = (bearing - player['heading'] + 1024) % 2048 - 1024
            report['decisions'].append(dict(distance=round(distance, 1), error=error, position=player['position']))
            if abs(error) > 40:
                key('Left' if error > 0 else 'Right', 0.12)
            elif distance > 18000:
                key('Up', 0.15)
                stuck = stuck + 1 if last == player['position'] else 0
                if stuck >= 8:
                    report['approach_stuck'] = True
                    break
            else:
                report['approached_target'] = True
                break
            last = player['position']
        assert report.get('approached_target'), 'Failed to reach island NPC'
        capture('before-action')
        key('Control_L', 0.2)
        for delay in [1, 3, 8]:
            time.sleep(delay)
            snap('after-action-' + str(delay))
            capture('after-action-' + str(delay))
        report['topic_menu_observed'] = any((x['controls']['topics'] == 1 for x in report['snapshots']))

        def choose_topic(name):
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                current = snap_retry('select-' + name)
                menu = current['controls']['menu']
                wanted = next((x for x in menu['items'] if name in x['text'].upper()))
                if menu['highlight'] == wanted['index']:
                    report.setdefault('selected_topics', []).append(wanted)
                    key('Control_L', 0.15)
                    return
                key('Down', 0.1)
            raise TimeoutError('Select ' + name)
        assert report['topic_menu_observed']

        def menu_texts():
            return [x['text'].upper() for x in snap_retry('menu')['controls']['menu']['items']]

        def wait_menu_back():
            deadline = time.monotonic() + 60
            seen_closed = False
            while time.monotonic() < deadline:
                current = snap_retry('topic-response')
                seen_closed |= current['controls']['topics'] == 0
                if seen_closed and current['controls']['topics'] == 1:
                    return
                time.sleep(0.2)
            raise TimeoutError('Topic response and menu return')

        # Brennan's script unlocks EMPIRE and PIRATE HAVEN when ISZARA is asked; ISZARA and LEAGUE leave
        # the menu only once the story flag after_iszara is set, which happens later. The menu after
        # each topic is recorded.
        report['topic_sequence'] = [{'asked': None, 'menu': menu_texts()}]
        assert any('ISZARA' in x for x in report['topic_sequence'][0]['menu']), report['topic_sequence']
        for name in ['ISZARA', 'EMPIRE', 'PIRATE', 'LEAGUE', 'STROS', 'DRAGGIN']:
            before_menu = report['topic_sequence'][-1]['menu']
            assert any(name in x for x in before_menu), (name, before_menu)
            choose_topic(name)
            wait_menu_back()
            after_menu = menu_texts()
            report['topic_sequence'].append({'asked': name, 'menu': after_menu})
            capture('after-' + name.lower())
        first, after_iszara = report['topic_sequence'][0]['menu'], report['topic_sequence'][1]['menu']
        assert not any('EMPIRE' in x for x in first) and any('EMPIRE' in x for x in after_iszara), 'ISZARA did not unlock EMPIRE'
        assert not any('PIRATE' in x for x in first) and any('PIRATE' in x for x in after_iszara), 'ISZARA did not unlock PIRATE HAVEN'
        report['topic_response_observed'] = True
        choose_topic('BYE')
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            current = snap('conversation-exit')
            if all((current['controls'][x] == 0 for x in ['lockout', 'dialogue', 'topics', 'sound_inhibited'])):
                break
            time.sleep(0.2)
        else:
            raise TimeoutError('Conversation control restoration')
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
