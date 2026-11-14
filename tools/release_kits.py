#!/usr/bin/env python3
"""Build the Windows and Linux runtime kits from an owned RGFX.EXE.

The release workflow runs this. It translates the executable, builds the Windows
and Linux runtimes (without playing the game), exports both kits and proves that
the real patcher turns each kit plus the executable into the accepted native
program. Neither the executable nor any generated game code leaves the work
folders; only the two kit ZIPs and a kits.json are written to --output.
"""
import argparse
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

from .toolchain import IMAGE, ROOT, ensure_tool_image

SEED = ROOT / 'platforms/kit-seed.json'
# Release assets are named by platform so a tag explains what it holds.
ASSET = 'maiko-runtime-kit-{}.zip'
REPOSITORY = 'https://github.com/michidk/maiko/releases/download'


def sh(command, **options):
    print('+', ' '.join(str(part) for part in command), flush=True)
    return subprocess.run([str(part) for part in command], cwd=ROOT, check=True, **options)


def last_line(command):
    """Run a build tool and return the folder it prints last."""
    result = sh(command, stdout=subprocess.PIPE, text=True)
    sys.stdout.write(result.stdout)
    return ROOT / result.stdout.strip().splitlines()[-1]


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def fetch_seed(target):
    """The curated SDL, GL and notice files have no source here; they come from a pinned kit."""
    pin = json.loads(SEED.read_text())[target]
    with urllib.request.urlopen(pin['url'], timeout=300) as response:
        data = response.read()
    if sha256(data) != pin['sha256']:
        raise SystemExit(f'{target} seed kit does not match its pinned SHA-256')
    return zipfile.ZipFile(io.BytesIO(data))


def runtime_directory(target, glide, source_tarball, destination):
    destination.mkdir(parents=True)
    with fetch_seed(target) as seed:
        for name in seed.namelist():
            if name.startswith('runtime/') and not name.endswith('/'):
                leaf = name.removeprefix('runtime/')
                if leaf.startswith('RUN'):
                    continue
                (destination / leaf).write_bytes(seed.read(name))
    # The renderer and its corresponding source are the parts that change per release.
    shutil.copyfile(glide, destination / glide.name)
    shutil.copyfile(source_tarball, destination / 'openglide-source.tar.gz')


def source_tarball(destination):
    """OpenGLide as built (pinned tree plus Maiko's patches) and the Maiko files it was built with."""
    with tempfile.TemporaryDirectory(dir=ROOT / '.cache') as directory:
        root = Path(directory)
        (root / 'upstream').mkdir()
        (root / 'maiko').mkdir()
        commit = json.loads((ROOT / '.cache/glide/report.json').read_text())['commit']
        archive = subprocess.check_output(['git', 'archive', commit], cwd=ROOT / '.cache/openglide-ng')
        subprocess.run(['tar', '-x', '-C', root / 'upstream'], input=archive, check=True)
        for patch in ['openglide-x64', 'openglide-sdl-window']:
            subprocess.run(['patch', '-s', '-p1', '-d', root / 'upstream'],
                           input=(ROOT / f'patches/{patch}.patch').read_bytes(), check=True)
        archive = subprocess.check_output(['git', 'archive', 'HEAD', 'Cargo.toml', 'Cargo.lock', 'LICENSE', 'patches',
                                           'glide', 'platforms/linux', 'platforms/windows', 'runtime', 'tools'], cwd=ROOT)
        subprocess.run(['tar', '-x', '-C', root / 'maiko'], input=archive, check=True)
        sh(['tar', '--sort=name', '--mtime=@0', '--owner=0', '--group=0', '--numeric-owner',
            '-czf', destination, '-C', root, 'upstream', 'maiko'])


def support_directory(destination):
    """SR's pristine x64 support files, selected the way the kit exporter selects them."""
    from .pipeline import SR_COMMIT
    names = subprocess.check_output(['git', '-C', '.cache/sr', 'ls-tree', '-r', '--name-only', SR_COMMIT,
                                     'SR/x64-support'], cwd=ROOT, text=True).splitlines()
    destination.mkdir(parents=True)
    for name in names:
        if name.endswith(('.inc', '.asm')):
            data = subprocess.check_output(['git', '-C', '.cache/sr', 'show', f'{SR_COMMIT}:{name}'], cwd=ROOT)
            (destination / Path(name).name).write_bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exe', type=Path, required=True, help='Owned RGFX.EXE (the accepted original)')
    parser.add_argument('--output', type=Path, required=True, help='New folder for linux.zip, windows.zip, kits.json')
    parser.add_argument('--tag', required=True, help='Release tag the kits will be published under')
    parser.add_argument('--wasmtime', type=Path, default=shutil.which('wasmtime'),
                        help='Wasmtime used to prove each kit with the real patcher')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('--output must not exist')
    if not args.wasmtime:
        parser.error('Install Wasmtime or pass --wasmtime')
    variants = json.loads((ROOT / 'metadata/image_variants.json').read_text())
    exe = args.exe.read_bytes()
    if sha256(exe) != variants['translated_image_sha256']:
        raise SystemExit('RGFX.EXE is not the accepted original executable')
    revision = subprocess.check_output(['git', 'rev-parse', '--short', 'HEAD'], cwd=ROOT, text=True).strip()
    cache = ROOT / '.cache'
    cache.mkdir(exist_ok=True)
    install = cache / 'kit-install'
    shutil.rmtree(install, ignore_errors=True)
    install.mkdir()
    (install / 'RGFX.EXE').write_bytes(exe)
    scratch = cache / 'kit-build'
    shutil.rmtree(scratch, ignore_errors=True)
    scratch.mkdir()
    try:
        ensure_tool_image()
        sh(['cargo', 'build', '--locked', '--release', '-p', 'maiko-patcher', '--bin', 'maiko-kit', '--bin', 'maiko-patcher'])
        target_dir = Path(json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1'],
                                                              cwd=ROOT))['target_directory'])
        kit_tool, patcher = target_dir / 'release/maiko-kit', target_dir / 'release/maiko-patcher'
        if not (cache / 'client-toolchain/sr.wasm').exists():
            sh([kit_tool, 'tools'])
        translation = last_line([sys.executable, '-m', 'tools.pipeline', '--install', install, 'translate'])
        sh([sys.executable, '-m', 'tools.build_glide', '--install', install])
        windows = last_line([sys.executable, '-m', 'tools.build_native', translation, '--install', install, '--link-only'])
        linux = last_line([sys.executable, '-m', 'tools.build_linux', translation, '--install', install, '--link-only'])
        support = scratch / 'support'
        support_directory(support)
        objects = scratch / 'objects'
        objects.mkdir()
        # Both objects come from the same translation the patcher will rebuild from the user's executable.
        relative = lambda path: path.relative_to(ROOT)
        sh(['docker', 'run', '--rm', '--network', 'none', '-v', f'{ROOT}:/w', IMAGE, 'sh', '-c',
            f'cd /w/{relative(translation)} && '
            f'nasm -O1 -f elf64 -I/w/{relative(support)}/ rgfx.asm -o /w/{relative(objects)}/rgfx.o && '
            f'nasm -f win64 -I/w/{relative(support)}/ -I./ -O1 rgfx.asm -o /w/{relative(objects)}/rgfx.obj && '
            f'strip -o /w/{relative(objects)}/rg-linux /w/{relative(linux)}/redguard-native && '
            f'chown -R {os.getuid()}:{os.getgid()} /w/{relative(objects)}'])
        tarball = scratch / 'openglide-source.tar.gz'
        source_tarball(tarball)
        runtimes = {'linux': scratch / 'runtime-linux', 'windows': scratch / 'runtime-windows'}
        runtime_directory('linux', linux / 'libredguard-glide.so', tarball, runtimes['linux'])
        runtime_directory('windows', windows / 'redguard-glide.dll', tarball, runtimes['windows'])
        args.output.mkdir(parents=True)
        sh([kit_tool, 'export', 'linux', objects / 'rg-linux', objects / 'rgfx.o', runtimes['linux'], revision,
            args.output / 'linux.zip'])
        sh([kit_tool, 'export', 'windows', windows / 'redguard-native.exe', objects / 'rgfx.obj', runtimes['windows'],
            revision, args.output / 'windows.zip'])
        for target in ['linux', 'windows']:
            package = scratch / f'verify-{target}'
            sh([patcher, 'build', '--input', args.exe, '--target', target, '--kit', args.output / f'{target}.zip',
                '--wasmtime', args.wasmtime, '--output', package])
            package.unlink()
        pins = json.loads((ROOT / 'web/kits.json').read_text())
        for target in ['linux', 'windows']:
            pins[target] = dict(url=f'{REPOSITORY}/{args.tag}/{ASSET.format(target)}',
                                sha256=sha256((args.output / f'{target}.zip').read_bytes()))
        (args.output / 'kits.json').write_text(json.dumps(pins, indent=2) + '\n')
        print(json.dumps(pins, indent=2))
    finally:
        # Translated game code and the executable copy never stay behind.
        shutil.rmtree(install, ignore_errors=True)
        shutil.rmtree(scratch, ignore_errors=True)
        # Keep failed builds' folders for diagnosis; hosted runners are discarded anyway.
        if sys.exc_info()[0] is None:
            for directory in [locals().get('translation'), locals().get('windows'), locals().get('linux')]:
                if directory:
                    shutil.rmtree(directory, ignore_errors=True)


if __name__ == '__main__':
    main()
