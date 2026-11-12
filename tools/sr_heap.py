"""Stage the pinned SR heap sources without trusting cached working-tree edits."""
import hashlib
from pathlib import Path
import subprocess
import tempfile
from .toolchain import ROOT
from .pipeline import SR_COMMIT


def stage_sr_heap(container):
    repo = ROOT / '.cache/sr'
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip()
    if head != SR_COMMIT:
        raise ValueError('SR heap checkout revision differs from translator pin')
    patch = ROOT / 'patches/sr-heap-c-api.patch'
    report = {'commit': SR_COMMIT, 'files': {},
              'c_api_patch_sha256': hashlib.sha256(patch.read_bytes()).hexdigest(),
              'staged_sha256': {}}
    with tempfile.TemporaryDirectory() as directory:
        for source, target in [('Game_memory.c', 'sr_Game_memory.c'),
                               ('Game_memory.h', 'Game_memory.h')]:
            path = 'games/Albion/SR-Main/' + source
            data = subprocess.check_output(['git', 'show', SR_COMMIT + ':' + path], cwd=repo)
            report['files'][path] = hashlib.sha256(data).hexdigest()
            Path(directory, target).write_bytes(data)
        subprocess.run(['patch', '-p1', '-d', directory], input=patch.read_bytes(),
                       stdout=subprocess.DEVNULL, check=True)
        for target in ['sr_Game_memory.c', 'Game_memory.h']:
            data = Path(directory, target).read_bytes()
            report['staged_sha256'][target] = hashlib.sha256(data).hexdigest()
            subprocess.run(['docker', 'exec', '-i', container, 'tee', '/work/' + target],
                           input=data, stdout=subprocess.DEVNULL, check=True)
    return report
