"""Inventory owned C/C++ fixtures and reject production C/C++ outside dependencies."""
import argparse
import json
from .pipeline import ROOT

PRODUCTION = ('runtime', 'linux', 'audio', 'glide')


def inventory():
    groups = {}
    for directory in [*PRODUCTION, 'tests']:
        groups[directory] = [dict(path=str(path.relative_to(ROOT)),
                                  lines=len(path.read_text().splitlines()))
                             for path in sorted((ROOT / directory).rglob('*'))
                             if path.suffix in ('.c', '.cpp')]
    return dict(owned_c_cpp=groups,
                production_files=sum(len(groups[directory]) for directory in PRODUCTION),
                fixture_files=len(groups['tests']),
                total_files=sum(map(len, groups.values())),
                total_lines=sum(file['lines'] for files in groups.values() for file in files),
                fixture_policy='Independent C ABI and renderer contract programs; never shipped in the game executable',
                excluded=['external SR/SDL/OpenGLide sources', 'translated/handwritten assembly',
                          'separate Mac branch'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='Fail if owned production C/C++ remains')
    args = parser.parse_args()
    report = inventory()
    print(json.dumps(report, indent=2))
    if args.check and report['production_files']:
        raise SystemExit('Owned production C/C++ remains')
