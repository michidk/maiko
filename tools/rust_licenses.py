"""Collect the installed license files for the locked Windows Rust dependency graph."""
import json
from pathlib import Path
import shutil
import subprocess
from .toolchain import ROOT


MIT = """Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."""


def generated_mit(package):
    """MIT notice for a crate whose published package carries no license file.

    The crate's manifest offers MIT (alone or as one option), so the MIT text is given with the
    copyright holders named in the manifest; the notice says where it came from.
    """
    holders = ', '.join(a.split('<')[0].strip() for a in package['authors']) or 'the authors'
    return ('MIT License (generated from the crate manifest: the published package has no license file; '
            + str(package['repository']) + ')\n\nCopyright (c) ' + holders + '\n\n' + MIT + '\n')


def notices(target='x86_64-pc-windows-gnu'):
    cargo = shutil.which('cargo') or str(Path.home()/'.cargo/bin/cargo')
    metadata = json.loads(subprocess.check_output([
        cargo, 'metadata', '--format-version=1', '--locked', '--offline',
        '--filter-platform', target], cwd=ROOT, text=True))
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
        # Crates that embed fonts ship their licences beside the fonts.
        files |= {path for path in (directory/'fonts').glob('*.txt')} if (directory/'fonts').is_dir() else set()
        generated = None
        if not files and 'MIT' in str(package['license']):
            generated = generated_mit(package)
        elif not files:
            raise ValueError('No installed license notice for '+package['name'])
        label = package['name']+' '+package['version']+' ('+str(package['license'])+')'
        result.append('===== '+label+' =====\n')
        if generated:
            result.append('--- generated MIT notice ---\n'+generated+'\n')
        for path in sorted(files):
            result.append('--- '+path.name+' ---\n'+path.read_text()+'\n')
    return '\n'.join(result)
