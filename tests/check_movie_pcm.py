"""Compare original decoded movie PCM with FFmpeg, before resampling or gain.

Run in the bounded Linux development container after its original movie probe.
No captured game media belongs in a distributed runtime package.
"""
import hashlib
import json
from pathlib import Path
import subprocess

source=Path('/work/movie-source.pcm')
probe=json.loads(Path('/work/gameplay-new.json').read_text())
assert probe.get('movie_uninterrupted_lead_seconds',0)>=2
log=Path('/work/gameplay-new.log').read_text()
assert 'Native movie PCM source_rate=22050 format=2 flags=0 ' in log
movie=Path('/work/reference-intro.smk')
reference=Path('/work/reference-intro.pcm')
try:
    subprocess.run(['python3','/tests/extract_cd_fixture.py','/media/game.gog','INTRO.SMK',str(movie)],check=True,timeout=30)
    subprocess.run(['ffmpeg','-v','error','-i',str(movie),'-map','0:a:0','-t','1',
                    '-acodec','pcm_u8','-f','u8','-y',str(reference)],check=True,timeout=30)
    actual=source.read_bytes()[:32768]
    expected=reference.read_bytes()[:32768]
    assert len(actual)==len(expected)==32768,(len(actual),len(expected))
    assert actual==expected,'Original Smacker PCM differs from independent FFmpeg decode'
    report=dict(bytes_compared=len(actual),source_rate=22050,channels=2,format='unsigned8',
                sha256=hashlib.sha256(actual).hexdigest(),
                ffmpeg_version=subprocess.check_output(['ffmpeg','-version'],text=True).splitlines()[0],
                original_pcm_reference_verified=True,hardware_audio_verified=False,synchronization_verified=False)
    Path('/work/movie-pcm-reference.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))
finally:
    movie.unlink(missing_ok=True)
    reference.unlink(missing_ok=True)
