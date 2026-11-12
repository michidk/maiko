"""Verify original autosave persists player state across separate native processes."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

root=Path('/work')
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--user-dir',type=Path,default=root/'save-roundtrip-user')
p.add_argument('--wine',action='store_true',help='Exercise Windows executable with Wine; native Windows remains unverified')
a=p.parse_args()
user=a.user_dir
if user.exists():raise RuntimeError('Use a fresh isolated save-roundtrip directory')
source=root/'menu-scenario/SAVEGAME/SAVEGAME.001/SAVEGAME.SAV'
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
original=digest(source)
runs=[]
for number in [1,2]:
    result=subprocess.run(['python3','/tests/probe_gameplay.py','--controls','--user-dir',str(user),'--seconds','45']+(['--wine'] if a.wine else []),timeout=100)
    evidence=json.loads((root/'gameplay-continue.json').read_text())
    for suffix in ['.log','.json','-world-before.ppm','-world-after.ppm','-world-book.ppm']:
        shutil.copyfile(root/('gameplay-continue'+suffix),root/(f'save-roundtrip-{number}'+suffix))
    assert result.returncode==0 and evidence['natural_exit']==0 and evidence['world_controls_verified'],evidence
    assert not evidence['blocked'] and digest(source)==original
    evidence['saved_sha256']=digest(user/'SAVEGAME/SAVEGAME.001/SAVEGAME.SAV')
    assert evidence['saved_sha256']!=original,'Autosave unchanged after original player movement'
    runs.append(evidence)
assert runs[1]['world_before']['position']==runs[0]['world_after_turn']['position'],runs
assert runs[0]['saved_sha256']!=runs[1]['saved_sha256'],'Second original autosave did not change'
report=dict(original_save_sha256=original,runs=runs,original_player_state_reload_verified=True,
            source_save_unchanged=True,wine_used=a.wine,full_game_verified=False,native_windows_verified=False)
(root/'save-roundtrip.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS original player movement, autosave, clean Quit, separate-process Continue and exact position reload twice; source save unchanged')
