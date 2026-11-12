"""Stage the pinned SR heap sources without trusting cached working-tree edits."""
import hashlib
import subprocess
from build_windows import ROOT
from pipeline import SR_COMMIT

def stage_sr_heap(container):
    repo=ROOT/'.cache/sr'
    head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()
    if head!=SR_COMMIT:raise ValueError('SR heap checkout revision differs from translator pin')
    report={'commit':SR_COMMIT,'files':{}}
    for source,target in [('Game_memory.c','sr_Game_memory.c'),('Game_memory.h','Game_memory.h')]:
        path='games/Albion/SR-Main/'+source
        data=subprocess.check_output(['git','show',SR_COMMIT+':'+path],cwd=repo)
        subprocess.run(['docker','exec','-i',container,'tee','/work/'+target],input=data,
                       stdout=subprocess.DEVNULL,check=True)
        report['files'][path]=hashlib.sha256(data).hexdigest()
    return report
