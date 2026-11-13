"""Assert original DOS exit and window-close callbacks on each host."""
import argparse
import os
import subprocess
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--wine',action='store_true')
a=p.parse_args()
command=['/usr/lib/wine/wine64','original-close-test.exe'] if a.wine else ['./original-close']
for mode,status,line in [('--close',0,'PASS native close ran exit cleanup'),
                         ('--exit',7,'PASS native DOS exit runs host cleanup')]:
    result=subprocess.run(command+[mode],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,
                          env=dict(os.environ,WINEDEBUG='-all',WINEPREFIX='/tmp/rg-wine',SDL_AUDIODRIVER='dummy'),timeout=15)
    print(result.stdout,flush=True)
    assert result.returncode==status,(mode,result.returncode)
    assert line in result.stdout
    assert 'PASS native shutdown preserves SDL video through exit callbacks' in result.stdout
print('PASS original close and DOS exit preserve callbacks and SDL lifetime',flush=True)
