"""Check the packaged CMD with an argument-reporting Windows test executable.

Run in the tool container with the ZIP, prior CMD and compiled stub in /work.
The disposable extraction replaces only its executable; it is a CMD test,
not evidence of original game execution.
"""
import os
import re
from pathlib import Path
import shutil
import subprocess
import zipfile

root=Path('/work/launcher test')
with zipfile.ZipFile('/work/runtime.zip') as archive:
    archive.extractall(root)
canonical=(root/'RUN.cmd').read_text()
shutil.copyfile('/work/launcher-stub.exe',root/'redguard-native.exe')
environment=dict(os.environ,WINEDEBUG='-all',WINEPREFIX='/tmp/rg-launcher-wine')
def launch(source,directory,expected_code):
    configured,count=re.subn(r'^set "GAME_DIR=[^"\r\n]*"$',lambda _:f'set "GAME_DIR={directory}"',source,flags=re.MULTILINE)
    assert count==1,count
    (root/'RUN.cmd').write_bytes(configured.replace('\r\n','\n').replace('\n','\r\n').encode())
    result=subprocess.run(['/usr/lib/wine/wine64','cmd','/c','RUN.cmd'],cwd=root,
                          env=dict(environment,RG_TEST_EXPECTED_DIR=directory),
                          input='\n',text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=50)
    log=(root/'native-test.log').read_text()
    print(result.stdout,flush=True)
    assert result.returncode==expected_code,(directory,result.returncode)
    assert f'Exit code: {expected_code}' in log,log
    return log
old=Path('/work/old-RUN.cmd').read_text()
launch(old,'Z:\\work\\fake game\\',99)
print('RED old CMD misquotes a trailing backslash',flush=True)
for directory in ['Z:\\work\\fake game','Z:\\work\\fake game\\','Z:\\']:
    launch(canonical,directory,7)
print('GREEN packaged CMD handles spaces, trailing backslash and drive root; exit code retained',flush=True)
