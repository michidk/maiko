"""Shared helpers for the disposable Docker toolchain used by the build and test tools."""
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
IMAGE = 'redguard-recomp-tools:bookworm'
# Docker bind mounts must resolve below this folder: the repository's parent unless overridden.
MOUNT_ROOT = Path(os.environ.get('MAIKO_MOUNT_ROOT', ROOT.parent)).resolve()


def is_mountable(path):
    return Path(path).resolve().is_relative_to(MOUNT_ROOT)


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
            run(['docker','build','-t',IMAGE,'-f',str(ROOT/'tools/docker/Dockerfile'),str(ROOT/'tools/docker')],
                stdout=log,stderr=subprocess.STDOUT,timeout=600)
        inspected=run(['docker','image','inspect','--format','{{.Id}}',IMAGE],capture_output=True)
    return dict(image_id=inspected.stdout.strip(),rebuilt=rebuilt,
                seconds=round(time.monotonic()-began,3),log=str(logfile) if logfile else None)
