"""Stage pinned SR sample conversion/mixing support with a Redguard integration shim."""
import hashlib
import re
import subprocess
from .build_windows import ROOT
from .pipeline import SR_COMMIT

def stage_sr_audio(container,patched=True):
    repo=ROOT/'.cache/sr'
    if subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()!=SR_COMMIT:
        raise ValueError('SR audio revision differs from translator pin')
    subprocess.run(['docker','exec',container,'mkdir','-p','/audio'],check=True)
    report={'commit':SR_COMMIT,'files':{}}
    for name in ['Albion-sound.c','Albion-sound.h','Albion-AIL.h','Game_defs.h']:
        source='games/Albion/SR-Main/'+name
        data=subprocess.check_output(['git','show',SR_COMMIT+':'+source],cwd=repo)
        report['files'][source]=hashlib.sha256(data).hexdigest()
        if name=='Game_defs.h':
            text=data.decode();notice=text[:text.index('#if')]
            sample=re.search(r'typedef struct _Game_sample \{.*?\} Game_sample;',text,re.S).group()
            data=(notice+'#pragma once\n#include <stdint.h>\n#include <SDL.h>\n#include <stddef.h>\n#define CCALL\n#define GAME_MIXER_CHANNELS_INITIAL 16\n#define GAME_SAMPLE_CACHE_SIZE 32\n'+sample+'\n').encode()
        subprocess.run(['docker','exec','-i',container,'tee','/audio/'+name],input=data,stdout=subprocess.DEVNULL,check=True)
    for name in ['Game_vars.h','Game_memory.h','stream_pcm.c','stream_pcm.h']:
        subprocess.run(['docker','cp',str(ROOT/'audio'/name),container+':/audio/'+name],check=True)
    patch=ROOT/'patches/sr-audio.patch'
    if patched and patch.exists():
        # Apply on the host so the offline runtime image needs no patch utility.
        import tempfile
        with tempfile.TemporaryDirectory(dir=ROOT/'.cache') as directory:
            subprocess.run(['docker','cp',container+':/audio/.',directory],check=True)
            subprocess.run(['patch','-p1','-d',directory],input=patch.read_bytes(),check=True)
            subprocess.run(['docker','cp',directory+'/.',container+':/audio'],check=True)
        report['patch_sha256']=hashlib.sha256(patch.read_bytes()).hexdigest()
    return report
