#!/usr/bin/env python3
"""Reproduce original-game LLasm translation analysis, not a runnable Mac build."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time
from .le_image import Image
from .pipeline import ROOT, SR_COMMIT, metadata
from glide.imports import inventory


def sha(data):
    return hashlib.sha256(data).hexdigest()


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
        metadata(image, work, True)
        for source in (ROOT / 'native-metadata').glob('*.sci'):
            if source.name == 'instruction_replacements.sci':
                continue
            if source.name in ['global_aliases.sci', 'fixup_do_not_interpret_as_code.sci']:
                with (work / source.name).open('a') as out:
                    out.write(source.read_text())
            else:
                shutil.copyfile(source, work / source.name)
        for source in (ROOT / 'llasm-metadata').glob('*.sci'):
            shutil.copyfile(source, work / source.name)
        replacements = [line.split(',', 2) for line in (work / 'instruction_replacements.sci').read_text().splitlines()]
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
