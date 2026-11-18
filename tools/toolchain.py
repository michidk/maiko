"""Shared helpers for the disposable Docker toolchain used by the build and test tools."""
import json
import os
from pathlib import Path
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
IMAGE = 'redguard-recomp-tools:bookworm'
# Docker bind mounts must resolve below this folder: the repository's parent unless overridden.
MOUNT_ROOT = Path(os.environ.get('MAIKO_MOUNT_ROOT', ROOT.parent)).resolve()


def is_mountable(path):
    return Path(path).resolve().is_relative_to(MOUNT_ROOT)


def run(args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def container_name(purpose):
    return f'rg-{purpose}-'+uuid.uuid4().hex[:10]


def start_container(name, image, memory, seconds, mounts=()):
    """Start an offline, memory- and CPU-capped container that lives at most `seconds`.
    `mounts` are (host path, container path) pairs, mounted read-only."""
    run(['docker','run','-d','--network','none','--memory',memory,'--memory-swap',memory,'--cpus','2','--name',name,
         *[arg for source,destination in mounts for arg in ('--mount',f'type=bind,src={source},dst={destination},readonly')],
         image,'sleep',str(seconds)],stdout=subprocess.DEVNULL)


def remove_container(name):
    subprocess.run(['docker','rm','-f',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)


def copy_in(name, pairs):
    """Copy (host path, container path) pairs; a trailing '/.' on a folder copies its contents."""
    for source,destination in pairs:
        run(['docker','cp',str(source),f'{name}:{destination}'])


def copy_out(name, pairs):
    """Copy (container path, host path) pairs that exist; evidence a failed run did not write is skipped."""
    for source,destination in pairs:
        subprocess.run(['docker','cp',f'{name}:{source}',str(destination)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)


def game_build_header(path, variants):
    """The RGFX.EXE hashes the native build checks the user's image against."""
    path.write_text('#define RGFX_SHA256 "'+variants['translated_image_sha256']+'"\n'
                    '#define RGFX_ALTERNATE_SHA256 "'+variants['alternate_image_sha256']+'"\n')


def collect_tool_notices(container, support, destination):
    destination.mkdir(parents=True,exist_ok=True)
    (destination/'SR-MIT.txt').write_text((support/'asm_call.inc').read_text().split('%macro')[0])
    for package in ['mingw-w64-common','gcc-mingw-w64-x86-64-posix-runtime']:
        run(['docker','cp',container+':/usr/share/doc/'+package+'/copyright',str(destination/(package+'.txt'))])
    for license in ['GPL-3','LGPL-2','LGPL-2.1']:
        run(['docker','cp',container+':/usr/share/common-licenses/'+license,str(destination/(license+'.txt'))])


LINUX_IMAGE='redguard-recomp-linux:bookworm'


def ensure_linux_image(**options):
    """Build the Linux toolchain image; Docker's cache makes an unchanged Dockerfile cheap."""
    run(['docker','build','-t',LINUX_IMAGE,'-f',str(ROOT/'platforms/linux/Dockerfile'),str(ROOT/'platforms/linux')],**options)
    return subprocess.check_output(['docker','image','inspect',LINUX_IMAGE,'--format','{{.Id}}'],text=True).strip()


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
            run(['docker','build','-t',IMAGE,'-f',str(ROOT/'platforms/windows/Dockerfile'),str(ROOT/'platforms/windows')],
                stdout=log,stderr=subprocess.STDOUT,timeout=2400)
        inspected=run(['docker','image','inspect','--format','{{.Id}}',IMAGE],capture_output=True)
    return dict(image_id=inspected.stdout.strip(),rebuilt=rebuilt,
                seconds=round(time.monotonic()-began,3),log=str(logfile) if logfile else None)
