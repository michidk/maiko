"""Verify final exception dispatch from actual translated code on both threads."""
import os
import subprocess
for mode in ['--main','--worker']:
    result=subprocess.run(['/usr/lib/wine/wine64','original-fault-test.exe',mode],
                          env=dict(os.environ,WINEDEBUG='-all',WINEPREFIX='/tmp/rg-wine'),
                          stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=20)
    print(result.stdout,flush=True)
    assert result.returncode==3,(mode,result.returncode)
    assert 'PASS original translated trap has covering Windows unwind entry' in result.stdout
    assert result.stdout.count('BLOCKED exception=c000001d')==1
print('PASS final crash reports through actual translated section on main and worker threads',flush=True)

result=subprocess.run(['/usr/lib/wine/wine64','original-fault-test.exe','--exit'],
                      env=dict(os.environ,WINEDEBUG='-all',WINEPREFIX='/tmp/rg-wine'),
                      stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=20)
print(result.stdout,flush=True)
assert result.returncode==7
assert 'PASS native DOS exit runs host cleanup' in result.stdout
assert 'Original DOS termination status=7' in result.stdout
print('PASS original DOS exit status maps to native process and cleanup',flush=True)
