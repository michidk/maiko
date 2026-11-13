"""Build the shared Rust runtime without overriding the managed Cargo target directory."""
import argparse
import hashlib
import json
import os
import uuid
from pathlib import Path
import shutil
import subprocess
from .toolchain import ROOT, IMAGE, ensure_tool_image

TARGETS = ('x86_64-unknown-linux-gnu', 'x86_64-pc-windows-gnu',
           'aarch64-apple-darwin', 'x86_64-apple-darwin')

def cross_tools(target):
    env = os.environ.copy()
    if target != 'x86_64-pc-windows-gnu':
        return env
    required = ['x86_64-w64-mingw32-dlltool', 'x86_64-w64-mingw32-as']
    if all(shutil.which(tool) for tool in required):
        return env
    ensure_tool_image()
    directory = ROOT / '.cache/toolchain/bin'
    directory.mkdir(parents=True, exist_ok=True)
    missing = [tool for tool in required if not (directory / tool).exists()]
    if missing:
        name = 'maiko-rust-tools-' + uuid.uuid4().hex[:10]
        subprocess.run(['docker', 'create', '--network', 'none', '--name', name,
                        IMAGE, 'true'], check=True, stdout=subprocess.DEVNULL)
        try:
            for tool in missing:
                subprocess.run(['docker', 'cp', name + ':/usr/bin/' + tool,
                                str(directory / tool)], check=True)
        finally:
            subprocess.run(['docker', 'rm', name], check=True, stdout=subprocess.DEVNULL)
    env['PATH'] = str(directory) + os.pathsep + env.get('PATH', '')
    return env


def build(target, *, component="runtime", llasm=False):
    if component not in ("runtime", "host", "probe", "glide"):
        raise ValueError("Unknown Rust component: "+component)
    if target not in TARGETS:
        raise ValueError('Unsupported runtime target: '+target)
    env = cross_tools(target)
    # Panic messages embed source paths; do not ship the build machine's folder names.
    cargo_home = os.environ.get('CARGO_HOME') or str(Path.home()/'.cargo')
    env['RUSTFLAGS'] = ' '.join(filter(None, [env.get('RUSTFLAGS', ''),
        f'--remap-path-prefix={cargo_home}=/cargo', f'--remap-path-prefix={ROOT}=/maiko']))
    cargo = shutil.which('cargo') or str(Path.home()/'.cargo/bin/cargo')
    output = ROOT/'.cache/rust-runtime'
    output.mkdir(parents=True, exist_ok=True)
    tag=target+('' if component=='runtime' else '-'+component)+('-llasm' if llasm else '')
    with (output/(tag+'.log')).open('w') as log:
        subprocess.run([cargo, 'build', '--locked', '--profile', 'runtime', '--target', target, '-p', 'maiko-'+component,
                        *(['--features', 'maiko-'+component+('/llasm-entry' if component=='host' else '/llasm')] if llasm else [])],
                       cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    metadata = json.loads(subprocess.check_output(
        [cargo, 'metadata', '--format-version=1', '--no-deps', '--locked'], cwd=ROOT, text=True))
    archive = Path(metadata['target_directory'])/target/('runtime/libmaiko_'+component+'.a')
    sources = [ROOT/'build.rs', ROOT/'Cargo.toml', ROOT/'Cargo.lock', ROOT/'rust-toolchain.toml',
               *sorted((ROOT/'runtime').rglob('*.rs')),
               *sorted((ROOT/'runtime').rglob('Cargo.toml'))]
    report = dict(target=target, component=component, llasm=llasm, archive_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                  source_sha256={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources})
    (output/(tag+'.json')).write_text(json.dumps(report, indent=2)+'\n')
    return archive, report

def stage(container, prepared):
    """Copy a previously built archive after starting the bounded container."""
    archive, report = prepared
    component = report["component"]
    subprocess.run(['docker','cp',str(archive),container+':/work/libmaiko_'+component+'.a'],check=True)
    return report

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', choices=TARGETS, required=True)
    parser.add_argument('--component', choices=['runtime','host','probe','glide'], default='runtime')
    parser.add_argument('--llasm', action='store_true')
    args = parser.parse_args()
    print(build(args.target, component=args.component, llasm=args.llasm)[0])
