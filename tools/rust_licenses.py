"""Collect the installed license files for the locked Windows Rust dependency graph."""
import json
from pathlib import Path
import shutil
import subprocess
from .toolchain import ROOT


def notices():
    cargo = shutil.which('cargo') or str(Path.home()/'.cargo/bin/cargo')
    metadata = json.loads(subprocess.check_output([
        cargo, 'metadata', '--format-version=1', '--locked', '--offline',
        '--filter-platform', 'x86_64-pc-windows-gnu'], cwd=ROOT, text=True))
    packages = {package['id']: package for package in metadata['packages']}
    nodes = {node['id']: node for node in metadata['resolve']['nodes']}
    included = set()
    pending = list(metadata['workspace_members'])
    while pending:
        package_id = pending.pop()
        if package_id in included:
            continue
        included.add(package_id)
        pending.extend(dep['pkg'] for dep in nodes[package_id]['deps']
                       if any(kind['kind'] != 'dev' for kind in dep['dep_kinds']))
    result = []
    for package in sorted((packages[key] for key in included), key=lambda p: (p['name'], p['version'])):
        if not package['source']:
            continue
        directory = Path(package['manifest_path']).parent
        files = {path for path in directory.iterdir()
                 if path.is_file() and path.name.upper().startswith(('LICENSE', 'COPYING', 'COPYRIGHT', 'NOTICE'))}
        if package.get('license_file'):
            files.add(directory/package['license_file'])
        if package['name'] == 'sdl2-sys':
            files.add(directory/'SDL/LICENSE.txt')
            # The sys crate comes from rust-sdl2, whose root MIT notice is
            # distributed in the sibling sdl2 crate rather than this package.
            sdl = next(p for p in packages.values() if p['name'] == 'sdl2')
            files.add(Path(sdl['manifest_path']).parent/'LICENSE')
        if not files:
            raise ValueError('No installed license notice for '+package['name'])
        label = package['name']+' '+package['version']+' ('+str(package['license'])+')'
        result.append('===== '+label+' =====\n')
        for path in sorted(files):
            result.append('--- '+path.name+' ---\n'+path.read_text()+'\n')
    return '\n'.join(result)
