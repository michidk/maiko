#!/usr/bin/env python3
"""Reproduce original-game LLasm translation analysis, not a runnable Mac build."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import re
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time
from .le_image import Image
from .pipeline import ROOT, SR_COMMIT, metadata, read_replacements, check_replacements, replacement_text
from glide.imports import inventory


def sha(data):
    return hashlib.sha256(data).hexdigest()


def prepare_host(work, imports):
    """Emit CPU-state boundaries from the same service inventory as NASM.

    Original: native services marshal SR registers through the platform ABI.
    Replacement: LLasm writes back its CPU state before the Rust C ABI call;
    wrappers pop the guest return and Glide's documented stdcall arguments.
    Effect: the dispatcher and all service implementations stay shared.
    Why: fastcc guest procedures cannot be called through Rust's C ABI.
    Evidence: tests/llasm_execution.c exercises SR's offset C/guest wrappers;
    runtime/host/llasm.rs owns register conversion and reentrant host calls.
    """
    declarations = ('funcv rg_llasm_service $, service\n'
                    'func rg_llasm_target $\nfunc rg_llasm_variant\n'
                    'funcv rg_translation_trap address\nextern rg_bios_ticks\n'
                    'extern rg_skip_intro\nextern rg_load_save\n')
    (work / 'extern.llinc').write_text(declarations)
    wrappers = ['proc rg_dispatch global c_rg_dispatch',
                'call rg_llasm_target $', 'tcall tmp0', 'endp']
    services = [(name, int(number), 0) for name, number in re.findall(
        r'^HOST_SERVICE (\w+), (\d+)', (ROOT / 'runtime/native_services.inc').read_text(), re.M)]
    services += [(f'host_glide_{entry["index"]}', 0x2000 + entry['index'], entry['bytes'])
                 for entry in imports]
    for name, service, pop in services:
        wrappers += [f'proc {name} public', f'call rg_llasm_service $, {service}', 'POP tmp1']
        if pop:
            wrappers.append(f'add esp, esp, {pop}')
        wrappers += ['tcall tmp1', 'endp']
    (work / 'host.llinc').write_text('\n'.join(wrappers) + '\n')
    source = work / 'rgfx.llasm'
    text = source.read_text().removesuffix('\ninclude host.llinc\n')
    # SR emits LLVM functions separately from data. Writable embedded data in
    # the mixed objects must remain writable without making text executable+writable.
    for number in (5, 7):
        text = text.replace(f'datasegment cseg{number:02} constant', f'datasegment cseg{number:02}')
    source.write_text(text + '\ninclude host.llinc\n')
    # Original: 27 instructions load the DOS BIOS clock address 0x46c.
    # Replacement: use the offset-encoded address of the shared atomic clock.
    # Effect: the engine reads the same host clock as the native backend.
    # Why: Darwin cannot expose DOS low memory. LLasm's extern address operand
    # applies pointer_offset before storing a guest pointer.
    # Evidence: pipeline.py's BIOS tick hook enforces the same 27-site inventory.
    # Each matched source comment must still identify the original literal.
    actions = set()
    clock_sites = 0
    for path in work.glob('seg*_code.llinc'):
        code = path.read_text()
        # Original: the native backend traces the clip-code entry at 0xD92D0.
        # Replacement: use the same register-preserving diagnostic service.
        # Effect: tests/check_linux_menu.py can observe original book rendering.
        # Why: the NASM trace wrapper cannot run through the CPU-state ABI.
        # Evidence: pipeline.py instrument_native and host/lib.rs service
        # 0x800D92D0 implement the existing clip trace, without changing logic.
        entry = 'proc rg_clip_code global c_rg_clip_code\n'
        if entry in code:
            if code.count(entry) != 1:
                raise ValueError('LLasm clip trace entry inventory changed')
            code = code.replace(entry, entry + 'PUSH loc_20000\ncall rg_llasm_service $, 0x800d92d0\nadd esp, esp, 4\n')
        pattern = r'^(;mov (?:dword \[ebp-0x[0-9a-f]+\]|eax|esi|edi), 0x46c\n(?:add tmpadr, ebp, -[0-9]+\n)?)(mov (?:eax|esi|edi|tmp\d), )0x46c$'
        code, count = re.subn(pattern, r'\1\2rg_bios_ticks', code, flags=re.M)
        clock_sites += count
        path.write_text(code)
        actions.update(re.findall(r'^(ACTION_\w+)\s*$', code, re.M))
    if clock_sites != 27:
        raise ValueError(f'LLasm BIOS tick pointer inventory changed: {clock_sites}')
    (work / 'macros.llinc').write_text(''.join(f'macro {name}\nendm\n' for name in sorted(actions)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--install', type=Path, required=True)
    parser.add_argument('--sr', type=Path, required=True, help='Patched OUT_LLASM SR executable')
    args = parser.parse_args()
    image = Image(args.install / 'RGFX.EXE')
    work = ROOT / 'artifacts' / (datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ') + '-llasm-analysis')
    work.mkdir(parents=True)
    report = dict(status='started', image=image.report(), sr_commit_reference=SR_COMMIT,
                  translator_source_verified=False,
                  translator_sha256=sha(args.sr.read_bytes()), mode='translation_analysis',
                  macos_game_built=False, macos_runtime_verified=False, full_game_verified=False)
    began = time.monotonic()
    try:
        metadata(image, work)
        for source in (ROOT / 'native-metadata').glob('*.sci'):
            if source.name == 'instruction_replacements.sci':
                continue
            if source.name in ['global_aliases.sci', 'fixup_do_not_interpret_as_code.sci']:
                with (work / source.name).open('a') as out:
                    out.write(source.read_text())
            else:
                shutil.copyfile(source, work / source.name)
        for source in (ROOT / 'llasm-metadata').glob('*.sci'):
            if source.name == 'global_aliases.sci':
                with (work / source.name).open('a') as out:
                    out.write(source.read_text())
            elif source.name == 'instruction_replacements.sci':
                entries = read_replacements(source)
                check_replacements(entries, image)
                (work / source.name).write_text(replacement_text(entries))
            else:
                shutil.copyfile(source, work / source.name)
        replacements = [line.split(',', 2) for line in (work / 'instruction_replacements.sci').read_text().splitlines()
                        if line.strip() and not line.lstrip().startswith('#')]
        native_sites = {int(entry[0][4:], 16) for entry in read_replacements(
            ROOT / 'native-metadata/instruction_replacements.sci')}
        covered_sites = {int(label[4:], 16) for label, _, _ in replacements}
        if native_sites - covered_sites:
            raise ValueError('LLasm is missing native replacements: ' +
                             ', '.join(hex(site) for site in sorted(native_sites - covered_sites)))
        checks = json.loads((ROOT / 'llasm-metadata/instruction_checks.json').read_text())
        if [(int(label[4:], 16), int(length)) for label, length, _ in replacements] != [
                (int(check['address'], 16), check['length']) for check in checks]:
            raise ValueError('LLasm replacement inventory differs from byte checks')
        for check in checks:
            if sha(image.at(int(check['address'], 16), check['length'])) != check['sha256']:
                raise ValueError('LLasm replacement bytes changed at ' + check['address'])
        imports = inventory(image)
        with (work / 'external_procedures.sci').open('a') as out:
            out.writelines(f'loc_{entry["address"]:X},host_glide_{entry["index"]}\n' for entry in imports)
        with (work / 'fixup_do_not_interpret_as_code.sci').open('a') as out:
            out.writelines(f'loc_{address:X}\n' for address in range(0x160000, 0x163000))
        report['metadata_sha256'] = {p.name: sha(p.read_bytes()) for p in sorted(work.glob('*.sci'))}
        report['source_sha256'] = {str(p.relative_to(ROOT)): sha(p.read_bytes()) for p in [
            Path(__file__), ROOT / 'tools/pipeline.py', ROOT / 'tools/le_image.py', ROOT / 'glide/imports.py',
            *(ROOT / 'llasm-metadata').glob('*')] if p.is_file()}
        raw = bytearray((args.install / 'RGFX.EXE').read_bytes())
        le = struct.unpack_from('<I', raw, 0x3c)[0]
        table = struct.unpack_from('<I', raw, le + 64)[0]
        for number in [5, 7]:
            offset = le + table + (number - 1) * 24 + 8
            flags = struct.unpack_from('<I', raw, offset)[0]
            if flags != 0x2043:
                raise ValueError('Unexpected mixed code/data object flags')
            struct.pack_into('<I', raw, offset, flags | 4)
        report['translator_input_sha256'] = sha(raw)
        with tempfile.TemporaryDirectory(dir=ROOT / '.cache') as directory:
            source = Path(directory) / 'RGFX.EXE'
            source.write_bytes(raw)
            with (work / 'translation.log').open('w') as log:
                result = subprocess.run([str(args.sr.resolve()), str(source), 'rgfx.llasm'],
                                        cwd=work, stdout=log, stderr=subprocess.STDOUT, timeout=60)
        report['translator_exit'] = result.returncode
        log = (work / 'translation.log').read_text()
        if 'Output type: LLasm version' not in log:
            raise ValueError('Translator is not the LLasm backend')
        if result.returncode == 0:
            prepare_host(work, imports)
        report['status'] = 'llasm_emitted_not_runtime_verified' if result.returncode == 0 else 'translation_blocked'
        report['translation_tail'] = log.splitlines()[-4:]
    except Exception as error:
        report['status'] = 'failed'
        report['error'] = str(error)
        raise
    finally:
        report['elapsed_seconds'] = round(time.monotonic() - began, 3)
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(work)
        print(report['status'])
    if result.returncode:
        raise SystemExit(2)


if __name__ == '__main__':
    main()
