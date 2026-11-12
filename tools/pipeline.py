#!/usr/bin/env python3
"""Reproduce the SR translation experiment. Generated game code stays local."""
import argparse
import datetime
import hashlib
import json
from itertools import groupby
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import struct
import tempfile

from .le_image import Image
from glide.imports import emit as emit_glide

ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / '.cache'
SR_COMMIT = 'ac690ddf3010bc3d2c875cb1b5e8da4f53a8d5b3'


def command(args, *, cwd=None, log=None, timeout=180):
    start = time.monotonic()
    result = subprocess.run([str(a) for a in args], cwd=cwd, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True, timeout=timeout)
    if log:
        Path(log).write_text(result.stdout)
    else:
        print(result.stdout, end='')
    if result.returncode:
        if log:
            print(result.stdout[-3000:], file=sys.stderr)
        raise RuntimeError(f'{args[0]} exited {result.returncode}; log: {log}')
    return round(time.monotonic() - start, 3)


def patch_body(path):
    """The diff itself: a patch's descriptive header is documentation and must not invalidate caches."""
    data = Path(path).read_bytes()
    match = re.search(rb'^(diff --git |--- )', data, re.M)
    return data[match.start():] if match else data


def build():
    """Build the pinned SR translator with the repository's x64 patch.

    Patch: patches/sr-x64.patch plus the OUTPUT_TYPE switch below (documented in the patch header).
    Effect: SR emits x86-64 NASM source instead of 32-bit.
    Why: the native Windows and Linux builds are 64-bit; stock SR targets 32-bit output.
    Evidence: docs/development/PLATFORM.md. The stamp file refuses a changed patch on a cached checkout.
    """
    CACHE.mkdir(exist_ok=True)
    venv = CACHE / 'venv'
    if not (venv / 'bin/scons').exists():
        command([sys.executable, '-m', 'venv', venv])
        command([venv / 'bin/pip', 'install', 'scons==4.10.1'])
    repo = CACHE / 'sr'
    if not repo.exists():
        command(['git', 'clone', '--no-checkout', '--filter=blob:none', 'https://github.com/M-HT/SR.git', repo])
        command(['git', 'checkout', '--detach', SR_COMMIT], cwd=repo)
    actual = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip()
    if actual != SR_COMMIT:
        raise RuntimeError('Cached SR checkout is at a different revision')
    patch = ROOT / 'patches/sr-x64.patch'
    signature = hashlib.sha256(patch_body(patch)).hexdigest()
    stamp = repo / '.redguard-patch'
    if not stamp.exists():
        command(['git', 'apply', '--check', patch], cwd=repo)
        command(['git', 'apply', patch], cwd=repo)
        defs = repo / 'SR/SR_defs.h'
        text = defs.read_text()
        if text.count('#define OUTPUT_TYPE  OUT_X86') != 1:
            raise RuntimeError('SR output configuration changed')
        defs.write_text(text.replace('#define OUTPUT_TYPE  OUT_X86', '#define OUTPUT_TYPE  OUT_X64'))
        stamp.write_text(signature)
    if stamp.read_text() != signature:
        raise RuntimeError('Patch changed: remove the disposable .cache/sr checkout, then rebuild')
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    elapsed = command([venv / 'bin/scons', '-j2'], cwd=repo / 'SR', log=CACHE / f'build-{stamp}.log')
    return repo / 'SR/SR.exe', elapsed


REQUIRED_DOC = ('Original:', 'Replacement:', 'Effect:', 'Why:', 'Evidence:')


def read_replacements(path, *, require_hash=True):
    """Parse an annotated replacement table into (label, length, replacement, sha256, doc) entries.

    The comment block above an entry documents it and a `# sha256:` line pins the original
    bytes. A blank line ends a block that several entries share. The translator never sees
    the comments: use `replacement_text` for what it reads.
    """
    entries, doc, digest = [], [], None
    for number, line in enumerate(Path(path).read_text().splitlines(), 1):
        if not line.strip():
            doc = []
        elif line.startswith('#'):
            body = line[1:].strip()
            if body.startswith('sha256:'):
                digest = body.split(':', 1)[1].strip()
            elif body:
                doc.append(body)
        else:
            label, length, replacement = line.split(',', 2)
            missing = [key for key in REQUIRED_DOC if not any(d.startswith(key) for d in doc)]
            if missing:
                raise ValueError(f'{path}:{number} {label} lacks documentation: {", ".join(missing)}')
            if require_hash and not digest:
                raise ValueError(f'{path}:{number} {label} lacks a sha256 line')
            entries.append((label, int(length), replacement, digest, list(doc)))
            digest = None
    return entries


def replacement_text(entries):
    return ''.join(f'{label},{length},{replacement}\n' for label, length, replacement, _, _ in entries)


def check_replacements(entries, image):
    for label, length, _, digest, _ in entries:
        address = int(label.removeprefix('loc_'), 16)
        if digest and hashlib.sha256(image.at(address, length)).hexdigest() != digest:
            raise ValueError(f'Original bytes changed at {label} ({length} bytes)')


def metadata(image, work, probe):
    for file in (ROOT / 'metadata').glob('*.sci'):
        if file.name == 'instruction_replacements.sci':
            entries = read_replacements(file)
            check_replacements(entries, image)
            (work / file.name).write_text(replacement_text(entries))
        else:
            shutil.copyfile(file, work / file.name)
    # Verify the two-entry Watcom idioms before applying their equivalent expansion.
    for address in [0xf70b6, 0xf70eb, 0x119cde, 0x13b3d7, 0x13b461]:
        if image.at(address, 4) != bytes.fromhex('66a989cf'):
            raise ValueError(f'Overlapping instruction changed at {address:#x}')
    # These strings are names passed to the Glide dynamic loader, not function bodies.
    code = image.objects[1]
    names = list(re.finditer(rb'_[A-Z][A-Z0-9_]+@[0-9]+\x00', code.data))
    if len(names) != 130:
        raise ValueError('Unexpected Glide import-name inventory')
    (work / 'fixup_do_not_interpret_as_code.sci').write_text(''.join(f'loc_{code.base+m.start():X}\n' for m in names))
    if probe:
        expected = {
            0x135eec: '66ea04000000', 0x136137: '66ea04000000',
            0x13619a: '66ea04000000', 0x1361f2: '66ea04000000',
            0x127c9a: '0f03c0', 0x127eb1: '0f03c0', 0x128ceb: 'f4',
            0x135543: '0f03c0', 0x1356b5: '0f03c0', 0x135999: '0f02c0',
            0x144ca7: '0f03c2', 0x13bd6b: '0f010c24',
        }
        seen = set()
        probe_entries = read_replacements(ROOT / 'probe-only/instruction_replacements.sci', require_hash=False)
        for label, length, replacement, _, _ in probe_entries:
            if not replacement.startswith('ud2'):
                continue
            address = int(label.removeprefix('loc_'), 16)
            original = bytes.fromhex(expected[address])
            if address in seen or int(length) != len(original) or image.at(address, len(original)) != original:
                raise ValueError(f'Invalid diagnostic substitution at {address:#x}')
            seen.add(address)
        if seen != expected.keys():
            raise ValueError('Diagnostic substitution inventory changed')
        (work / 'instruction_replacements.sci').write_text(replacement_text(probe_entries))
    return len(names)


def relax_branches(work):
    """Rewrite generated assembly so it assembles: short jumps become near and data runs compact.

    Original: SR copies the original's rel8 (`jcc short`) forms.
    Effect: no behavior change. Branches may now reach further and long `db` runs use `times` or
    128-byte lines so NASM builds the large segments in reasonable time.
    Why: translation expands instructions, so original rel8 distances no longer fit.
    Evidence: none needed beyond a successful assembly; the translated output is compared byte-for-byte
    when this function changes.
    """
    for file in work.glob('seg*.inc'):
        text = re.sub(r'^(j\w+) short ', r'\1 ', file.read_text(), flags=re.M)
        compact = []
        for line, group in groupby(text.splitlines(True)):
            count = sum(1 for _ in group)
            compact.append(f'times {count} {line}' if count > 2 and line.startswith('db ') else line * count)
        output, pending = [], []
        def flush():
            if pending:
                output.append('db ' + ','.join(pending) + '\n')
                pending.clear()
        for line in ''.join(compact).splitlines(True):
            if line.startswith('db '):
                pending.append(line[3:].strip())
                if len(pending) == 128:
                    flush()
            else:
                flush()
                output.append(line)
        flush()
        file.write_text(''.join(output))


def instrument_native(work):
    """Add a trace call at the entry of 21 original functions (diagnostic only).

    Original: the listed function entry labels, each required to occur exactly once.
    Effect: each entry first calls `trace_<address>`, a host service that records the call; game
    logic and registers are untouched.
    Why: lets the probes follow startup, movie and audio flow without a debugger.
    Evidence: runtime/call_trace.inc; runtime/host/lib.rs (trace services, service numbers with the top bit set).
    """
    addresses = [0x1115c2, 0x113c04, 0x113c63, 0x113cad, 0xf1cde, 0x129dc4,
                 0x119b86, 0x133707, 0x11fd96, 0x1303d4, 0x130c8b, 0x110826,
                 0xc7f20, 0xf976c, 0xc80f8, 0x20000, 0x20095, 0x98673, 0x138b82,
                 0x2bd78, 0xd92d0]
    source = work / 'seg02.inc'
    text = source.read_text()
    declarations = []
    services = []
    for address in addresses:
        label = f'loc_{address:X}:'
        if text.count(label) != 1:
            raise ValueError(f'Missing native trace root {label}')
        name = f'trace_{address:X}'
        text = text.replace(label, label + f'\nCALL {name}')
        declarations.append(f'extern {name}\n')
        services.append(f'HOST_SERVICE {name}, {0x80000000 | address}\n')
    source.write_text(text)
    with (work / 'extern.inc').open('a') as out:
        out.writelines(declarations)
    (work / 'native_traces.inc').write_text(''.join(services))


def split_mixed_section(work, number):
    """Retain writable embedded data without making translated code writable.

    Original: translated sections that mix executable code and data in one object.
    Effect: no behavior change. Data runs go to non-executable writable sections and code to executable
    read-only ones; the original's own writes to its embedded data still work.
    Why: modern platforms refuse writable-and-executable memory and the translator emits one section.
    Evidence: docs/development/PLATFORM.md (operating-system bindings).
    """
    file=work/f'seg{number:02}.inc'
    output=[];pending=[];active='code'
    for line in file.read_text().splitlines(True):
        if not line.strip() or line.startswith((';','align ')) or line.strip().endswith(':'):
            pending.append(line);continue
        target='data' if re.match(r'^(?:(?:times \d+ )?(?:db|dw|dd|dq)|resb)\b',line) else 'code'
        if target!=active:
            name=f'{"dseg" if target=="data" else "cseg"}{number:02}'
            elf='noexec write align=4' if target=='data' else 'exec nowrite align=16'
            output.append(f'%ifidn __OUTPUT_FORMAT__, elf64\nsection {name} progbits alloc {elf}\n%else\nsection {name} {target} align=4\n%endif\n')
            active=target
        output.extend(pending);pending=[];output.append(line)
    output.extend(pending)
    # The outer generated section_end label must remain in its code section.
    if active!='code':
        output.append(f'%ifidn __OUTPUT_FORMAT__, elf64\nsection cseg{number:02} progbits alloc exec nowrite align=16\n%else\nsection cseg{number:02} code align=16\n%endif\n')
    file.write_text(''.join(output))


def native_startup_book(image, text):
    """Show the native startup book, preserving the original direct-start override.

    Original: a fresh install with no usable autosave skips the book and starts the intro immediately,
    and the book only fades in when an autosave was admitted.
    Replacement: the default fresh start enters the book with New Game selected and Continue disabled,
    through the original disable-entry path; startup Escape with no admitted save takes the original
    no-op path. MENU.INI `ignore_autosave=1` keeps its original direct-world path.
    Effect: a new player sees the book menu first. This deliberately differs from the original.
    Why: reported on real Windows: starting straight into the game bypassed the book.
    Evidence: docs/development/VALIDATION.md (fresh start, Continue disabled); tests/check_startup_override.py.
    Guards: the original bytes at 0x205F6 and the exact translation text are checked; any change stops the build.
    """
    original = bytes.fromhex('e840ed02006685c0741cc705a31d030001000000')
    if image.at(0x205f6, len(original)) != original:
        raise ValueError('Original startup autosave branch bytes changed')
    before = ('CALL loc_4F33B\n'
              'test ax, ax\n'
              'ACTION_CONDITIONAL_JUMP_FORWARD\n'
              'jz loc_2061C\n'
              'mov dword [loc_1A1DA3], 0x1\n')
    after = before + 'native_startup_book:\n'
    if text.count(before) != 1:
        raise ValueError('Native startup autosave branch inventory changed')
    text = text.replace(before, after)
    # Preserve ignore_autosave=1's original direct-world path. Only replace
    # the default fresh-install auto-intro path with the requested book.
    if image.at(0x2063a, 9) != bytes.fromhex('85c07559b8ffffffff'):
        raise ValueError('Original ignore_autosave branch bytes changed')
    before = ('jnz loc_20697\n'
              'mov eax, 0xffffffff\n')
    after = ('jnz loc_20697\n'
             'mov dword [loc_1A1DA3], 0x0\n'
             'mov dword [loc_1A3F74], 0x0\n'
             'jmp native_startup_book\n'
             'mov eax, 0xffffffff\n')
    if text.count(before) != 1:
        raise ValueError('Native ignore_autosave branch inventory changed')
    text = text.replace(before, after)
    # The original only enters the startup book with a valid autosave. Its
    # presentation uses that same flag to request and permit the book fade-in.
    # Allow the startup book to fade in independently of Continue availability.
    if image.at(0xabb3f, 10) != bytes.fromhex('a0a31d0300a272a30800'):
        raise ValueError('Original main-book fade request bytes changed')
    request = 'mov al, [loc_1A1DA3]\nmov [loc_1FA372], al\n'
    if text.count(request) != 1:
        raise ValueError('Native main-book fade request inventory changed')
    # This is the one-time context-1 initialization, before input begins.
    # Select the enabled New Game action for a fresh start. Doing this in the
    # per-frame gray-entry code would reset the player's selection every frame.
    selection = ('mov byte [loc_1FA372], 0x1\n'
                 'cmp dword [loc_1A1DA3], 0x0\n'
                 'jnz native_startup_selection_done\n'
                 'mov eax, [loc_1A667B]\n'
                 'mov ecx, 0x40\n'
                 'native_startup_select_new:\n'
                 'test eax, eax\n'
                 'jz native_startup_selection_done\n'
                 'cmp dword [eax+0x4], 0xa\n'
                 'jnz native_startup_select_next\n'
                 'test byte [eax+0xc], 0x8\n'
                 'jnz native_startup_select_next\n'
                 'test byte [eax+0xc], 0x1\n'
                 'jz native_startup_select_next\n'
                 'mov [loc_1A6677], eax\n'
                 'jmp native_startup_selection_done\n'
                 'native_startup_select_next:\n'
                 'test byte [eax+0xc], 0x40\n'
                 'jnz native_startup_selection_done\n'
                 'mov eax, [eax+0x31]\n'
                 'dec ecx\n'
                 'jnz native_startup_select_new\n'
                 'native_startup_selection_done:\n')
    text = text.replace(request, selection)
    if image.at(0xab81a, 13) != bytes.fromhex('833da31d0300010f85f4000000'):
        raise ValueError('Original book fade admission bytes changed')
    before = ('cmp dword [loc_1A1DA3], 0x1\n'
              'ACTION_CONDITIONAL_JUMP_FORWARD\n'
              'jnz near loc_AB91B\n')
    after = ('cmp dword [loc_1A1DA3], 0x1\n'
             'ACTION_CONDITIONAL_JUMP_FORWARD\n'
             'jz native_startup_book_fade\n'
             'cmp dword [loc_1F9BD0], 0x1\n'
             'ACTION_CONDITIONAL_JUMP_FORWARD\n'
             'jnz near loc_AB91B\n'
             'native_startup_book_fade:\n')
    if text.count(before) != 1:
        raise ValueError('Native book fade admission inventory changed')
    text = text.replace(before, after)
    # Context 1 originally implied an admitted autosave. Reuse the original
    # gray/disable-entry path when that implication no longer holds.
    if image.at(0xaf874, 9) != bytes.fromhex('833dd09b080002750c'):
        raise ValueError('Original Continue context check bytes changed')
    before = ('loc_AF874:\n'
              'cmp dword [loc_1F9BD0], 0x2\n'
              'ACTION_CONDITIONAL_JUMP_FORWARD\n'
              'jnz loc_AF889\n')
    after = ('loc_AF874:\n'
             'cmp dword [loc_1F9BD0], 0x2\n'
             'ACTION_CONDITIONAL_JUMP_FORWARD\n'
             'jz native_startup_disable_continue\n'
             'cmp dword [loc_1F9BD0], 0x1\n'
             'ACTION_CONDITIONAL_JUMP_FORWARD\n'
             'jnz loc_AF889\n'
             'cmp dword [loc_1A1DA3], 0x0\n'
             'ACTION_CONDITIONAL_JUMP_FORWARD\n'
             'jnz loc_AF889\n'
             'native_startup_disable_continue:\n')
    if text.count(before) != 1:
        raise ValueError('Native Continue context check inventory changed')
    text = text.replace(before, after)
    # Startup Escape originally means "resume slot 1". With no admitted save,
    # use the same no-op path as original context 2 instead.
    if image.at(0xac8a4, 11) != bytes.fromhex('833dd09b0800027502eb42'):
        raise ValueError('Original book Escape context bytes changed')
    before = ('loc_AC8A4:\n'
              'cmp dword [loc_1F9BD0], 0x2\n'
              'ACTION_CONDITIONAL_JUMP_FORWARD\n'
              'jnz loc_AC8AF\n')
    after = ('loc_AC8A4:\n'
             'cmp dword [loc_1F9BD0], 0x2\n'
             'jz loc_AC8AD\n'
             'cmp dword [loc_1F9BD0], 0x1\n'
             'jnz loc_AC8AF\n'
             'cmp dword [loc_1A1DA3], 0x0\n'
             'jnz loc_AC8AF\n')
    if text.count(before) != 1:
        raise ValueError('Native book Escape context inventory changed')
    return text.replace(before, after)


def native_startup_clock(image, text):
    """Pace the original fixed-step clock before it advances the simulation.

    Original: at 0x5F6BC the original calls its clock update (loc_5F761) and then tests the step counter.
    Replacement: call host_startup_clock_pace first (runtime/rust/timing.rs).
    Effect: the 12 Hz loading warm-up advances at its fixed rate instead of as fast as the host runs.
    Simulation arithmetic is untouched.
    Why: reported on real Windows: a burst of fast movement at the start.
    Evidence: docs/development/VALIDATION.md (startup pacing); the Linux and Wine runs record the countdowns.
    Guards: the 12 original bytes are checked and the call must occur exactly once and not already be present.
    """
    if image.at(0x5f6bc, 12) != bytes.fromhex('e8a0000000833d2446030000'):
        raise ValueError('Original startup clock call bytes changed')
    before = 'CALL loc_5F761\ncmp dword [loc_1A4624], 0x0\n'
    if text.count(before) != 1 or 'CALL host_startup_clock_pace\n' in text:
        raise ValueError('Native startup clock call inventory changed')
    return text.replace(before, 'CALL host_startup_clock_pace\n' + before)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['inspect', 'build', 'translate'])
    parser.add_argument('--install', type=Path, default=os.environ.get('REDGUARD_DIR'))
    parser.add_argument('--probe-traps', action='store_true', help='Diagnostic only: unresolved DOS boundaries become fatal UD2 traps')
    parser.add_argument('--native', action='store_true', help='Experimental native CRT/main target with explicit runtime replacements')
    parser.add_argument('--sr', type=Path, help='Use an already built patched SR executable')
    args = parser.parse_args()
    if args.action == 'build':
        print(build()); return
    if not args.install:
        parser.error('Set REDGUARD_DIR or pass --install')
    install = Path(args.install).resolve()
    image = Image(install / 'RGFX.EXE')
    if args.action == 'inspect':
        print(json.dumps(image.report(), indent=2)); return
    if args.native and args.probe_traps:
        parser.error('--native and --probe-traps are separate targets')
    mode = 'native' if args.native else 'probe' if args.probe_traps else 'strict'
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    work = ROOT / 'artifacts' / f'{stamp}-{mode}'
    work.mkdir(parents=True)
    sr, build_seconds = (args.sr.resolve(), None) if args.sr else build()
    report = dict(image=image.report(), sr_commit=SR_COMMIT, mode=mode, build_seconds=build_seconds,
                  menu_verified=False, native_windows_verified=False, status='started')
    report['import_names'] = metadata(image, work, args.probe_traps or args.native)
    if args.native:
        report['unresolved_instruction_sites'] = 12
        report['native_runtime_complete'] = False
        for file in (ROOT / 'native-metadata').glob('*.sci'):
            if file.name in ['global_aliases.sci','fixup_do_not_interpret_as_code.sci']:
                with (work / file.name).open('a') as out:
                    out.write(file.read_text())
            elif file.name == 'instruction_replacements.sci':
                entries = read_replacements(file)
                check_replacements(entries, image)
                with (work / file.name).open('a') as out:
                    out.write(replacement_text(entries))
            else:
                shutil.copyfile(file, work / file.name)
    if args.native:
        emit_glide(image,work)
        # Object5 begins with lookup tables. SR's no-code SCI accepts exact
        # fixup targets only, so expand this byte range in generated metadata.
        # Excluding just named labels misses references to table interiors.
        with (work/'fixup_do_not_interpret_as_code.sci').open('a') as out:
            for address in range(0x160000,0x163000):
                out.write(f'loc_{address:X}\n')
    began = time.monotonic()
    try:
        with tempfile.TemporaryDirectory(prefix='translation-',dir=CACHE) as directory:
            translator_input=install/'RGFX.EXE'
            if args.native:
                # Objects5/7 contain original 32-bit assembly helpers despite
                # lacking LE's executable bit. Object5 includes the ROB decoder.
                raw=bytearray(translator_input.read_bytes())
                le=struct.unpack_from('<I',raw,0x3c)[0]
                table=struct.unpack_from('<I',raw,le+64)[0]
                overrides=[]
                for number in [5,7]:
                    flag_offset=le+table+(number-1)*24+8
                    flags=struct.unpack_from('<I',raw,flag_offset)[0]
                    if flags!=0x2043:
                        raise ValueError(f'Object{number} classification changed')
                    struct.pack_into('<I',raw,flag_offset,flags|4)
                    overrides.append(dict(object=number,original_flags=hex(flags),translator_flags=hex(flags|4)))
                translator_input=Path(directory)/'RGFX.EXE'
                translator_input.write_bytes(raw)
                report['translation_header_overrides']=overrides
                report['translator_input_sha256']=hashlib.sha256(raw).hexdigest()
            command([sr, translator_input, 'rgfx.asm'], cwd=work, log=work / 'translation.log', timeout=60)
        relax_branches(work)
        for file in (ROOT / 'runtime').glob('*.inc'):
            shutil.copyfile(file, work / file.name)
        if args.native:
            shutil.copyfile(ROOT / 'native-metadata/extern.inc', work / 'extern.inc')
            report['glide_imports']=emit_glide(image,work,write_mappings=False)
            source=work/'seg02.inc'
            source.write_text(native_startup_book(image, source.read_text()))
            source.write_text(native_startup_clock(image, source.read_text()))
            report['native_startup_clock_pacing'] = dict(
                original_fixed_step_hz=12,
                active='original warm-up counter nonzero, including loading fade resets',
                simulation='original scale, deltas and movement arithmetic unchanged')
            report['native_startup_book'] = dict(
                autosave_validation='original loc_4F33B',
                continue_available='only with a valid original autosave',
                presentation='startup book fade-in independent of autosave availability',
                no_autosave='native book, then original New Game return path',
                ignore_autosave='original direct-world override preserved',
                fresh_selection='New Game', no_save_escape='remain in book')
            pattern=r'^mov (dword \[ebp-0x[0-9a-f]+\]|eax|esi|edi), 0x46c$'
            text,count=re.subn(pattern,r'mov \1, rg_bios_ticks',source.read_text(),flags=re.M)
            if count!=27:
                raise ValueError(f'BIOS tick pointer inventory changed: {count}')
            source.write_text(text)
            report['native_bios_tick_pointer_rewrites']=count
            with (work/'extern.inc').open('a') as out:
                out.write('extern rg_bios_ticks\nextern host_startup_clock_pace\n')
            # Address-taken handwritten clipping helpers were previously emitted as
            # raw x86 bytes. Each must use the translated stack and call target.
            roots=[0xd933c,0xd93cc,0xd945c,0xd94ec,0xd9578,0xd9604]
            translated=source.read_text()
            for address in roots:
                label=f'loc_{address:X}:\n'
                if translated.count(label)!=1:
                    raise ValueError(f'Missing clipping helper: {address:#x}')
                body=translated.split(label,1)[1].split('\nRET',1)[0]
                if (re.search(r'^(?:db|dw|dd) ',body,re.M) or
                    'sub r11d, 0x4' not in body or 'CALL loc_D92D0' not in body):
                    raise ValueError(f'Clipping helper was not translated: {address:#x}')
            report['native_checked_clip_roots']=[hex(address) for address in roots]
            movie_roots=[(0xf1f80,0xf1ff0,'host_audio_sample_pan'),
                         (0xf2010,0xf2090,'host_audio_stream_release'),
                         (0xf2090,0xf20c0,'host_audio_sample_end'),
                         (0xf20c0,0xf22b0,'host_audio_stream_allocate'),
                         (0xf22b0,0xf2340,'host_audio_sample_type'),
                         (0xf2340,0xf2390,'loc_F2390'),
                         (0xf2390,0xf24a0,'host_audio_stream_load')]
            for address,end,target in movie_roots:
                label=f'loc_{address:X}:\n'
                if translated.count(label)!=1:
                    raise ValueError(f'Missing Smacker callback root: {address:#x}')
                end_label=f'loc_{end:X}:\n'
                if translated.count(end_label)!=1:
                    raise ValueError(f'Missing Smacker callback end: {end:#x}')
                # Cover every return path; padding after the last return is data.
                body=translated.split(label,1)[1].split(end_label,1)[0].rsplit('\nRET',1)[0]
                if (re.search(r'^(?:db|dw|dd) ',body,re.M) or 'r11d' not in body or
                    f'CALL {target}' not in body):
                    raise ValueError(f'Smacker callback translation contract failed: {address:#x}')
            report['native_checked_movie_roots']=[hex(address) for address,_,_ in movie_roots]
            for number in [5,7]:
                split_mixed_section(work,number)
            report['native_mixed_sections_split']=[5,7]
            instrument_native(work)
            with (work/'macros.inc').open('a') as out:
                out.write((ROOT/'runtime/call_trace.inc').read_text())
                out.write((ROOT/'runtime/image_variant.inc').read_text())
                out.write((ROOT/'runtime/font_metric.inc').read_text())
        report['status'] = 'native_assembly_emitted' if args.native else 'assembly_emitted_with_fatal_traps' if args.probe_traps else 'assembly_emitted'
    except (ValueError, RuntimeError, subprocess.TimeoutExpired) as exc:
        report['status'] = 'blocked'; report['error'] = str(exc)
        raise
    finally:
        report['translation_seconds'] = round(time.monotonic() - began, 3)
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(work)

if __name__ == '__main__':
    try:
        main()
    except (ValueError, RuntimeError, subprocess.TimeoutExpired) as exc:
        print(str(exc), file=sys.stderr)
        sys.exit(1)
