"""Stage checksum-pinned official SDL development packages for the Windows audio adapter."""
import hashlib
from pathlib import Path
import subprocess
import tarfile
import urllib.request
from .build_windows import ROOT

PACKAGES=[
 ('SDL','2.30.11','SDL2','0590db0a47b564aab92499ed03eaa75db9aee17c371080caa3b05df9d49d9ff9'),
 ('SDL_mixer','2.8.1','SDL2_mixer','b6c98eecd308aa9922ede0aac466bb5dea6c3e9b9b59dd496009f778c90d83d7'),
]
def stage_windows_sdl(container):
    destination=ROOT/'.cache/sdl-windows';destination.mkdir(exist_ok=True)
    report={}
    for project,version,prefix,expected in PACKAGES:
        archive=ROOT/'.cache'/f'{prefix}-devel-{version}-mingw.tar.gz'
        url=f'https://github.com/libsdl-org/{project}/releases/download/release-{version}/{archive.name}'
        if not archive.exists():urllib.request.urlretrieve(url,archive)
        if hashlib.sha256(archive.read_bytes()).hexdigest()!=expected:raise ValueError('SDL archive checksum differs: '+archive.name)
        with tarfile.open(archive) as source:
            base=f'{prefix}-{version}/x86_64-w64-mingw32/'
            for member in source.getmembers():
                if not member.isfile() or not member.name.startswith(base):continue
                relative=Path(member.name.removeprefix(base))
                if '..' in relative.parts or relative.is_absolute():raise ValueError('Unsafe SDL archive path')
                if relative.suffix not in ['.h','.a','.dll']:continue
                target=destination/relative;target.parent.mkdir(parents=True,exist_ok=True)
                target.write_bytes(source.extractfile(member).read())
            for member in source.getmembers():
                if member.isfile() and member.name.endswith('/LICENSE.txt'):
                    (destination/(prefix+'-LICENSE.txt')).write_bytes(source.extractfile(member).read())
        runtime=destination/'bin'/f'{prefix}.dll'
        report[project]=dict(version=version,url=url,archive_sha256=expected,
                             dll_sha256=hashlib.sha256(runtime.read_bytes()).hexdigest())
    subprocess.run(['docker','cp',str(destination)+'/.',container+':/sdl'],check=True)
    for dll in ['SDL2.dll','SDL2_mixer.dll']:
        subprocess.run(['docker','cp',str(destination/'bin'/dll),container+':/work/'+dll],check=True)
    return report
