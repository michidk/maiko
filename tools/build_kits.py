#!/usr/bin/env python3
"""Build the Linux, Windows and macOS runtime kits. No game file is needed.

Each kit holds the compilers (SR, NASM, LLVM for macOS, LLD), game-free link inputs (start
files, system libraries, the Rust host, SR's heap and audio objects) and the packaged runtime.
A visitor's browser or the CLI translates their own RGFX.EXE and links it with these inputs.

With --exe, the script also builds a package from every kit through the website's real worker
(and the CLI for Linux and Windows), to check the kits end to end before publishing.
"""
import argparse
import hashlib
import io
import json
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

from .toolchain import ROOT
from .build_link_inputs import OPENGLIDE

MACOS_TARGETS = ['aarch64-unknown-linux-gnu', 'aarch64-apple-darwin', 'x86_64-apple-darwin', 'wasm32-unknown-unknown']
TARGETS = ['linux', 'windows', 'macos']
MACOS_TOOLCHAIN = ROOT / '.cache/client-macos-toolchain'
# Release assets are named by platform so a tag explains what it holds.
ASSET = 'maiko-runtime-kit-{}.zip'
RELEASE = 'https://github.com/michidk/maiko/releases/latest/download'


def sh(command, **options):
    print('+', ' '.join(str(part) for part in command), flush=True)
    return subprocess.run([str(part) for part in command], cwd=ROOT, check=True, **options)


def last_folder(command):
    """Run a tool that prints its output folder among its last lines and return that folder."""
    result = sh(command, stdout=subprocess.PIPE, text=True)
    sys.stdout.write(result.stdout)
    folders = [ROOT / line for line in result.stdout.splitlines() if line and (ROOT / line).is_dir()]
    return folders[-1]


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def fetch_seed(target):
    """The curated SDL, GL and notice files have no source here; they come from the latest release's kit."""
    def download(name):
        with urllib.request.urlopen(f'{RELEASE}/{name}', timeout=300) as response:
            return response.read()
    name = ASSET.format(target)
    expected = {line.split()[1].lstrip('*'): line.split()[0] for line in download('SHA256SUMS').decode().splitlines() if line.strip()}
    data = download(name)
    if sha256(data) != expected.get(name):
        raise SystemExit(f'{target} seed kit does not match the release SHA256SUMS')
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
        archive = subprocess.check_output(['git', 'archive', OPENGLIDE], cwd=ROOT / '.cache/openglide-ng')
        subprocess.run(['tar', '-x', '-C', root / 'upstream'], input=archive, check=True)
        for patch in ['openglide-x64', 'openglide-sdl-window']:
            subprocess.run(['patch', '-s', '-p1', '-d', root / 'upstream'],
                           input=(ROOT / f'patches/{patch}.patch').read_bytes(), check=True)
        archive = subprocess.check_output(['git', 'archive', 'HEAD', 'Cargo.toml', 'Cargo.lock', 'LICENSE', 'patches',
                                           'glide', 'platforms/linux', 'platforms/windows', 'runtime', 'tools'], cwd=ROOT)
        subprocess.run(['tar', '-x', '-C', root / 'maiko'], input=archive, check=True)
        sh(['tar', '--sort=name', '--mtime=@0', '--owner=0', '--group=0', '--numeric-owner',
            '-czf', destination, '-C', root, 'upstream', 'maiko'])


def sources():
    """The pinned SR and OpenGLide checkouts the tools, link inputs and source tarball are built from."""
    from .pipeline import SR_COMMIT
    for repository, url, commit in [('sr', 'https://github.com/M-HT/SR.git', SR_COMMIT),
                                    ('openglide-ng', 'https://github.com/eugenenuke/openglide-ng', OPENGLIDE)]:
        directory = ROOT / '.cache' / repository
        if not directory.exists():
            sh(['git', 'clone', '--no-checkout', url, directory])
            sh(['git', '-C', directory, 'checkout', '--detach', commit])


def browser_build(target, kit, exe, output, scratch, kit_tool):
    """Build a package from `kit` with web/worker.js under Bun, as a visitor's browser does."""
    work = scratch / f'worker-{target}'
    work.mkdir()
    for name in ['worker.js', 'wasi.js']:
        shutil.copyfile(ROOT / 'web' / name, work / name)
    shutil.copytree(scratch / 'pkg', work / 'pkg')
    if target == 'macos':
        sh([kit_tool, 'web-tools', kit, work])
    sh(['bun', ROOT / 'tools/browser_build.mjs', work, target, kit, exe, output])


def macos_kit(scratch, revision, kit_tool, output):
    """Mac link inputs, renderer and frameworks from the backend check; no game translation."""
    backend = last_folder([sys.executable, '-m', 'tools.check_macos_backend', '--kit'])
    runtime = backend / 'runtime'
    with fetch_seed('macos') as seed:
        (runtime / 'LICENSES.txt').write_bytes(seed.read('runtime/LICENSES.txt'))
    sh([kit_tool, 'export-macos', backend, runtime, revision, output])
    return backend


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='New folder for the three kit ZIPs')
    parser.add_argument('--exe', type=Path, help='Owned RGFX.EXE: also build a package from each kit (local check)')
    parser.add_argument('--wasmtime', default=shutil.which('wasmtime'), help='Wasmtime for the CLI check with --exe')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('--output must not exist')
    if args.exe and not args.wasmtime:
        parser.error('--exe needs Wasmtime on PATH or --wasmtime')
    revision = subprocess.check_output(['git', 'rev-parse', '--short=12', 'HEAD'], cwd=ROOT, text=True).strip()
    (ROOT / '.cache').mkdir(exist_ok=True)
    scratch = Path(tempfile.mkdtemp(prefix='kit-build-', dir=ROOT / '.cache'))
    sources()
    sh(['rustup', 'target', 'add', *MACOS_TARGETS, 'x86_64-pc-windows-gnu'])
    sh(['cargo', 'build', '--locked', '--release', '-p', 'maiko-patcher', '--bin', 'maiko-kit', '--bin', 'maiko-patcher'])
    target_dir = Path(json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1'],
                                                          cwd=ROOT))['target_directory'])
    kit_tool, patcher = target_dir / 'release/maiko-kit', target_dir / 'release/maiko-patcher'
    if not (ROOT / '.cache/client-toolchain/sr.wasm').exists():
        sh([kit_tool, 'tools'])
    if not (MACOS_TOOLCHAIN / 'ld64.wasm').exists():
        sh([kit_tool, 'tools-macos'])
    tarball = scratch / 'openglide-source.tar.gz'
    source_tarball(tarball)
    args.output.mkdir(parents=True)
    for target, renderer in [('windows', 'redguard-glide.dll'), ('linux', 'libredguard-glide.so')]:
        inputs = scratch / f'inputs-{target}'
        sh([sys.executable, '-m', 'tools.build_link_inputs', '--target', target, '--output', inputs])
        runtime = scratch / f'runtime-{target}'
        runtime_directory(target, inputs / 'runtime' / renderer, tarball, runtime)
        sh([kit_tool, 'export', target, inputs / 'link', runtime, revision, args.output / ASSET.format(target)])
    backend = macos_kit(scratch, revision, kit_tool, args.output / ASSET.format('macos'))
    shutil.rmtree(backend, ignore_errors=True)
    if args.exe:
        sh(['cargo', 'build', '--locked', '--release', '-p', 'maiko-patcher', '--lib', '--target', 'wasm32-unknown-unknown'])
        sh(['wasm-bindgen', '--target', 'web', '--out-dir', scratch / 'pkg',
            target_dir / 'wasm32-unknown-unknown/release/maiko_patcher.wasm'])
        for target in TARGETS:
            kit = args.output / ASSET.format(target)
            browser_build(target, kit, args.exe, scratch / f'browser-{target}.zip', scratch, kit_tool)
            if target != 'macos':
                sh([patcher, 'build', '--input', args.exe, '--target', target, '--kit', kit,
                    '--wasmtime', args.wasmtime, '--output', scratch / f'cli-{target}.zip'])
    shutil.rmtree(scratch, ignore_errors=True)
    # The site build checks served kits against this file, as it does a release's SHA256SUMS.
    (args.output / 'SHA256SUMS').write_text(''.join(
        f'{sha256((args.output / ASSET.format(target)).read_bytes())}  {ASSET.format(target)}\n' for target in TARGETS))
    print('Kits:', *sorted(path.name for path in args.output.iterdir()))


if __name__ == '__main__':
    main()
