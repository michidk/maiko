"""Run in the tool container after compiling crash-reporting-test.exe."""
import os
import subprocess
environment=dict(os.environ,WINEDEBUG='-all',WINEPREFIX='/tmp/rg-wine')
for scenario,expected in [('--handled',0),('--fatal-busy',3)]:
    result=subprocess.run(['/usr/lib/wine/wine64','crash-reporting-test.exe',scenario],
                          env=environment,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=50)
    print(result.stdout,flush=True)
    assert result.returncode==expected,(scenario,result.returncode)
    if scenario=='--handled':
        assert 'PASS component handles' in result.stdout
        assert 'BLOCKED exception=' not in result.stdout
    else:
        assert result.stdout.count('BLOCKED exception=')==1
        assert 'exception=c000001d' in result.stdout
        assert result.stdout.count('CALLSITE ')==32
        assert 'UNEXPECTED crash cleanup ran' not in result.stdout
print('PASS first-chance dispatch, bounded concurrent dump and immediate fatal shutdown',flush=True)
