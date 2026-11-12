#!/usr/bin/env python3
"""Assemble/link the diagnostic Windows probe in a disposable offline container."""
import argparse
import json
import hashlib
from pathlib import Path
import shutil
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
IMAGE = 'redguard-recomp-tools:bookworm'


def run(args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def collect_tool_notices(container, support, destination):
    destination.mkdir(parents=True,exist_ok=True)
    (destination/'SR-MIT.txt').write_text((support/'asm_call.inc').read_text().split('%macro')[0])
    for package in ['mingw-w64-common','gcc-mingw-w64-x86-64-posix-runtime']:
        run(['docker','cp',container+':/usr/share/doc/'+package+'/copyright',str(destination/(package+'.txt'))])
    for license in ['GPL-3','LGPL-2','LGPL-2.1']:
        run(['docker','cp',container+':/usr/share/common-licenses/'+license,str(destination/(license+'.txt'))])


def ensure_tool_image():
    """Restore the disposable toolchain after external Docker cache cleanup."""
    began=time.monotonic()
    inspected=subprocess.run(['docker','image','inspect','--format','{{.Id}}',IMAGE],
                             stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True)
    rebuilt=inspected.returncode!=0
    if not rebuilt:
        labels=subprocess.check_output(['docker','image','inspect','--format','{{json .Config.Labels}}',IMAGE],text=True)
        rebuilt=(json.loads(labels) or {}).get('org.maiko.wine-version')!='10.0'
    logfile=None
    if rebuilt:
        logfile=ROOT/'.cache'/f'toolchain-build-{time.time_ns()}.log'
        logfile.parent.mkdir(exist_ok=True)
        print(f'Restoring {IMAGE}; log: {logfile}',flush=True)
        with logfile.open('w') as log:
            run(['docker','build','-t',IMAGE,'-f',str(ROOT/'Dockerfile'),str(ROOT)],
                stdout=log,stderr=subprocess.STDOUT,timeout=600)
        inspected=run(['docker','image','inspect','--format','{{.Id}}',IMAGE],capture_output=True)
    return dict(image_id=inspected.stdout.strip(),rebuilt=rebuilt,
                seconds=round(time.monotonic()-began,3),log=str(logfile) if logfile else None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('work', type=Path)
    parser.add_argument('--support', type=Path, default=ROOT / '.cache/sr/SR/x64-support')
    parser.add_argument('--wine', action='store_true')
    args = parser.parse_args()
    work = args.work.resolve()
    report = json.loads((work / 'report.json').read_text())
    if report['status'] != 'assembly_emitted_with_fatal_traps':
        parser.error('This host is only for diagnostic --probe-traps output')
    if not (args.support / 'x64inc.inc').is_file():
        parser.error('Build pinned SR first, or supply --support')
    ensure_tool_image()
    name = 'redguard-probe-' + uuid.uuid4().hex[:12]
    began = time.monotonic()
    report['windows_probe'] = 'started'
    report['wine_entry_boundary_verified'] = False
    report.pop('wine_exit_code', None)
    report.pop('exe_sha256', None)
    report['toolchain_image'] = subprocess.check_output(['docker', 'image', 'inspect', '--format', '{{.Id}}', IMAGE], text=True).strip()
    try:
        run(['docker', 'run', '-d', '--network', 'none', '--memory', '4g', '--memory-swap', '4g', '--cpus', '2', '--name', name, IMAGE, 'sleep', 'infinity'], stdout=subprocess.DEVNULL)
        # Only generated assembly and host sources enter the container. No install is mounted.
        for pattern in ['*.asm', 'seg*.inc']:
            for file in work.glob(pattern):
                run(['docker', 'cp', str(file), name + ':/work/' + file.name])
        run(['docker', 'cp', str(args.support.resolve()), name + ':/support'])
        run(['docker', 'cp', str(ROOT / 'runtime') + '/.', name + ':/work/'])
        run(['docker', 'cp', str(ROOT / 'diagnostic') + '/.', name + ':/work/'])
        shell = '''set -e
nasm -O1 -f win64 -I/support/ rgfx.asm -o rgfx.obj
nasm -I/support/ -f win64 probe.asm -o probe.obj
x86_64-w64-mingw32-gcc -Wall -Wextra -Werror -O2 probe.c probe.obj rgfx.obj -Wl,--image-base,0x10000000,--disable-dynamicbase,--no-insert-timestamp -o redguard-probe.exe
x86_64-w64-mingw32-objdump -p redguard-probe.exe
'''
        with (work / 'windows-build.log').open('w') as log:
            run(['docker', 'exec', name, 'timeout', '120', 'sh', '-c', shell], stdout=log, stderr=subprocess.STDOUT)
        run(['docker', 'cp', name + ':/work/redguard-probe.exe', str(work / 'redguard-probe.exe')])
        report['windows_probe'] = 'PE32+_linked'
        report['exe_sha256'] = hashlib.sha256((work / 'redguard-probe.exe').read_bytes()).hexdigest()
        report['tool_versions'] = subprocess.check_output(['docker', 'exec', name, 'dpkg-query', '-W', 'nasm', 'gcc-mingw-w64-x86-64-posix', 'mingw-w64-common', 'wine-stable-amd64'], text=True).splitlines()
        collect_tool_notices(name,args.support,work/'notices')
        if args.wine:
            result = subprocess.run(['docker', 'exec', name, 'timeout', '60', 'env', 'WINEDEBUG=-all', 'WINEPREFIX=/tmp/rg-wine', '/usr/lib/wine/wine64', 'redguard-probe.exe'], capture_output=True, text=True)
            output = result.stdout + result.stderr
            (work / 'wine.log').write_text(output)
            report['wine_exit_code'] = result.returncode
            expected = result.returncode == 2 and 'entering statically translated RGFX entry point' in output and 'INT 21 EAX=00003000' in output
            report['wine_entry_boundary_verified'] = expected
            if not expected:
                raise RuntimeError('Unexpected execution result; see wine.log')
            print('Verified translated entry reaches DOS version request under Wine. Menu is NOT implemented.')
        print(work / 'redguard-probe.exe')
    finally:
        report['windows_probe_seconds'] = round(time.monotonic() - began, 3)
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        subprocess.run(['docker', 'rm', '-f', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

if __name__ == '__main__':
    main()
