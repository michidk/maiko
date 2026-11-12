"""Build the shared Rust runtime without overriding the managed Cargo target directory."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
from .build_windows import ROOT

TARGETS = ('x86_64-unknown-linux-gnu', 'x86_64-pc-windows-gnu')

def build(target):
    if target not in TARGETS:
        raise ValueError('Unsupported runtime target: '+target)
    cargo = shutil.which('cargo') or str(Path.home()/'.cargo/bin/cargo')
    output = ROOT/'.cache/rust-runtime'
    output.mkdir(parents=True, exist_ok=True)
    with (output/(target+'.log')).open('w') as log:
        subprocess.run([cargo, 'build', '--locked', '--release', '--target', target],
                       cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    metadata = json.loads(subprocess.check_output(
        [cargo, 'metadata', '--format-version=1', '--no-deps', '--locked'], cwd=ROOT, text=True))
    archive = Path(metadata['target_directory'])/target/'release/libmaiko_runtime.a'
    sources = [ROOT/'Cargo.toml', ROOT/'Cargo.lock', ROOT/'rust-toolchain.toml',
               *sorted((ROOT/'runtime/rust').glob('*.rs'))]
    report = dict(target=target, archive_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                  source_sha256={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources})
    (output/(target+'.json')).write_text(json.dumps(report, indent=2)+'\n')
    return archive, report

def stage(container, target):
    archive, report = build(target)
    subprocess.run(['docker','cp',str(archive),container+':/work/libmaiko_runtime.a'],check=True)
    return report

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', choices=TARGETS, required=True)
    args = parser.parse_args()
    print(build(args.target)[0])
