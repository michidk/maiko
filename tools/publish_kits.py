#!/usr/bin/env python3
"""Replace the runtime kits on the latest release and redeploy the site.

Run it on the folder `tools.build_kits` wrote. It uploads the three kit ZIPs, rewrites their
lines in the release's SHA256SUMS and starts "Deploy browser builder", which serves the kits
from the latest release. Needs an authenticated `gh`.
"""
import argparse
import hashlib
import subprocess
import tempfile
from pathlib import Path

REPOSITORY = 'michidk/maiko'
KITS = [f'maiko-runtime-kit-{target}.zip' for target in ('linux', 'windows', 'macos')]


def gh(*args):
    return subprocess.run(['gh', *args, '--repo', REPOSITORY], check=True, text=True, capture_output=True).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('kits', type=Path, help='Folder written by tools.build_kits')
    args = parser.parse_args()
    missing = [name for name in KITS if not (args.kits / name).is_file()]
    if missing:
        parser.error(f'missing {", ".join(missing)}')
    tag = gh('release', 'view', '--json', 'tagName', '--jq', '.tagName').strip()
    with tempfile.TemporaryDirectory() as directory:
        sums = Path(directory) / 'SHA256SUMS'
        gh('release', 'download', tag, '--pattern', 'SHA256SUMS', '--dir', directory, '--clobber')
        lines = [line for line in sums.read_text().splitlines() if line.split()[-1:] and line.split()[-1] not in KITS]
        lines += [f'{hashlib.sha256((args.kits / name).read_bytes()).hexdigest()}  {name}' for name in KITS]
        sums.write_text('\n'.join(lines) + '\n')
        gh('release', 'upload', tag, '--clobber', sums.as_posix(), *(str(args.kits / name) for name in KITS))
    gh('workflow', 'run', 'pages.yml', '--ref', 'main')
    print(f'Kits replaced on {tag}; the site redeploys from them.')


if __name__ == '__main__':
    main()
