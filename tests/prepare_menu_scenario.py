"""Prepare a disposable Wine scenario; /game remains a read-only Docker mount."""
import argparse
import hashlib
from pathlib import Path
import re
import shutil

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--sound',action='store_true',help='Enable native sample audio; CD audio remains separately unsupported')
parser.add_argument('--redbook',action='store_true',help='Exercise the original CD-open absence path')
args=parser.parse_args()
root = Path('/work/menu-scenario')
shutil.copytree('/game', root, copy_function=lambda source, target: Path(target).symlink_to(source))
system = root/'SYSTEM.INI'
contents = system.read_text()
for key in ['sound', 'redbook']:
    contents, count = re.subn(rf'^{key}\s*=.*$', f'{key}={int(args.sound if key=="sound" else args.redbook)}', contents, flags=re.MULTILINE)
    if count != 1:
        raise ValueError(f'Expected exactly one {key} configuration entry')
system.unlink()
system.write_text(contents)
slot = root/'SAVEGAME/SAVEGAME.001'
slot.mkdir(parents=True, exist_ok=True)
save = slot/'SAVEGAME.SAV'
save.unlink(missing_ok=True)
shutil.copyfile('/work/menu-autosave.sav', save)
assert hashlib.sha256(save.read_bytes()).digest() == hashlib.sha256(Path('/work/menu-autosave.sav').read_bytes()).digest()
print(f'Prepared isolated startup autosave scenario sound={args.sound} redbook={args.redbook}')
