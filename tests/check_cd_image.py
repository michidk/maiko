"""Exercise the actual DOS CD adapter with small synthetic ISO and raw images."""
from pathlib import Path
import argparse
import struct
import subprocess
import tempfile
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('executable');p.add_argument('--wine',action='store_true');a=p.parse_args()

def both(value,n):return value.to_bytes(n,'little')+value.to_bytes(n,'big')
def record(name,extent,size,flags=0):
    r=bytearray(33+len(name)+(not len(name)%2));r[0]=len(r)
    r[2:10]=both(extent,4);r[10:18]=both(size,4);r[25]=flags;r[28:32]=both(1,2);r[32]=len(name);r[33:33+len(name)]=name
    return r
base=bytearray(24*2048)
pvd=memoryview(base)[16*2048:17*2048];pvd[:7]=b'\1CD001\1';pvd[80:88]=both(24,4);pvd[128:132]=both(2048,2)
pvd[156:190]=record(b'\0',20,2048,2)
r=record(b'INTRO.SMK;1',21,5000);base[20*2048:20*2048+len(r)]=r
base[21*2048:21*2048+5000]=bytes((i*13+7)&255 for i in range(5000))
exe=str(Path(a.executable).resolve());command=(['/usr/lib/wine/wine64',exe] if a.wine else [exe])
with tempfile.TemporaryDirectory(prefix='rg-cd-',dir='.') as folder:
    root=Path(folder).resolve()
    def run(name,data,mode='valid'):
        path=root/name;path.write_bytes(data)
        argument='Z:'+str(path).replace('/','\\') if a.wine else str(path)
        subprocess.run(command+[argument,mode],check=True,timeout=20,stdout=subprocess.DEVNULL)
    run('plain.iso',base)
    raw=b''.join(b'\xaa'*16+base[n:n+2048]+b'\xbb'*288 for n in range(0,len(base),2048))
    run('raw.bin',raw)
    run('truncated.iso',base[:-2048],'invalid')
    malformed=bytearray(base);malformed[20*2048+2:20*2048+10]=both(23,4)
    run('extent.iso',malformed,'invalid')
    malformed=bytearray(base);malformed[20*2048+6]^=1
    run('endian.iso',malformed,'invalid')
    malformed=bytearray(base);malformed[20*2048+len(r):20*2048+2*len(r)]=r
    run('duplicate.iso',malformed,'invalid')
print('PASS ISO2048/raw2352, truncated/endian/extent/duplicate rejection and actual DOS file operations')
