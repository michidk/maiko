"""Check the annotated byte-replacement tables without the game.

Every entry must parse and carry its documentation fields and a sha256 line; malformed tables must be rejected. Run: python3 tests/check_patch_tables.py
"""
import sys, tempfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.pipeline import read_replacements, replacement_text

TABLES = {'native-metadata/instruction_replacements.sci': True,
          'metadata/instruction_replacements.sci': True,
          'llasm-metadata/instruction_replacements.sci': True}
for name, hashed in TABLES.items():
    entries = read_replacements(ROOT / name, require_hash=hashed)
    assert entries, name
    plain = replacement_text(entries)
    assert '#' not in plain.split(',', 2)[0] and all(line.count(',') >= 2 for line in plain.splitlines()), name
    labels = [e[0] for e in entries]
    assert len(labels) == len(set(labels)), f'{name}: duplicate label'
    print(f'{name}: {len(entries)} entries')

GOOD = ('# Original: a\n# Replacement: b\n# Effect: c\n# Why: d\n# Evidence: e\n# sha256: ' + '0' * 64 + '\nloc_1,2,RET\n')
BAD = {'missing documentation': GOOD.replace('# Why: d\n', ''),
       'missing hash': GOOD.replace('# sha256: ' + '0' * 64 + '\n', ''),
       'documentation lost after a blank line': GOOD.replace('# sha256', '\n# sha256')}
with tempfile.TemporaryDirectory() as directory:
    path = Path(directory) / 't.sci'
    path.write_text(GOOD)
    assert read_replacements(path)[0][:3] == ('loc_1', 2, 'RET')
    for reason, text in BAD.items():
        path.write_text(text)
        try:
            read_replacements(path)
        except ValueError:
            continue
        raise AssertionError('accepted: ' + reason)
print('patch table checks passed')
