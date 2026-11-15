#!/usr/bin/env python3
"""Build the Linux, Windows and macOS runtime kits. No game file is needed.

Each kit holds the compilers (SR, NASM, LLVM for macOS, LLD), game-free link inputs (start
files, system libraries, the Rust host, SR's heap and audio objects) and the packaged runtime.
A visitor's browser or the CLI translates their own RGFX.EXE and links it with these inputs.

With --exe, the script also builds a package from every kit through the website's real worker
(and the CLI for Linux and Windows), to check the kits end to end before publishing.
"""
import argparse
import concurrent.futures
import hashlib
import io
import json
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

from .toolchain import ROOT

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
    """The curated SDL and notice files have no source here; they come from the latest release's kit."""
    def download(name):
        with urllib.request.urlopen(f'{RELEASE}/{name}', timeout=300) as response:
            return response.read()
    name = ASSET.format(target)
    expected = {line.split()[1].lstrip('*'): line.split()[0] for line in download('SHA256SUMS').decode().splitlines() if line.strip()}
    data = download(name)
    if sha256(data) != expected.get(name):
        raise SystemExit(f'{target} seed kit does not match the release SHA256SUMS')
    return zipfile.ZipFile(io.BytesIO(data))


RUST_NOTICES = '\n===== Rust standard library and bundled dependencies =====\n'
RUST_TARGETS = {'windows': 'x86_64-pc-windows-gnu', 'linux': 'x86_64-unknown-linux-gnu', 'macos': 'aarch64-apple-darwin'}


def notices(text, target):
    """The seed's notices without OpenGLide, GLU and GL dispatch (the renderer is now built into the
    program), followed by fresh notices for the Rust standard library and the target's crates."""
    from .rust_licenses import notices as crate_notices
    text = text.split(RUST_NOTICES)[0]
    # shortcut: edits the notices carried forward from the release; drop once a release ships without OpenGLide.
    text = re.sub(r'OpenGLide-ng[^\n]*\n', '', text)
    text = re.sub(r'===== OpenGLide LGPL 2\.1 =====\n.*?(?====== )', '', text, flags=re.S)
    # GLU and libglvnd are the last two Linux notices.
    text = text.split('\n\ncopyright\nFormat: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n'
                      'Upstream-Name: glu\n')[0]
    sysroot = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], cwd=ROOT, text=True).strip())
    library = (sysroot / 'share/doc/rust/COPYRIGHT-library.html').read_text()
    return text.rstrip('\n') + '\n' + RUST_NOTICES + library + '\n\n' + crate_notices(RUST_TARGETS[target]) + '\n'


def runtime_directory(target, built, destination):
    destination.mkdir(parents=True)
    with fetch_seed(target) as seed:
        for name in seed.namelist():
            if name.startswith('runtime/') and not name.endswith('/'):
                leaf = name.removeprefix('runtime/')
                # shortcut: skips the launcher's old name, the OpenGLide renderer and its GL libraries in the seed;
                # drop once a release ships without them.
                if (leaf.startswith(('RUN', 'libGL', 'libOpenGL', 'libredguard-', 'redguard-', 'libSDL2_mixer', 'SDL2_mixer'))
                        or leaf.endswith('.tar.gz')
                        or leaf in ('README.txt', 'Maiko.exe')):
                    continue
                data = seed.read(name)
                (destination / leaf).write_bytes(notices(data.decode(), target).encode() if leaf == 'LICENSES.txt' else data)
    # The launcher is the part that changes per release.
    shutil.copyfile(ROOT / 'platforms' / target / 'README.txt', destination / 'README.txt')
    launcher = 'maiko.exe' if target == 'windows' else 'maiko'
    shutil.copyfile(built / launcher, destination / launcher)


def sources():
    """The pinned SR checkout the tools and link inputs are built from."""
    from .pipeline import SR_COMMIT
    for repository, url, commit in [('sr', 'https://github.com/M-HT/SR.git', SR_COMMIT)]:
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
    """Mac link inputs and frameworks from the backend check; no game translation."""
    backend = last_folder([sys.executable, '-m', 'tools.check_macos_backend', '--kit'])
    runtime = backend / 'runtime'
    with fetch_seed('macos') as seed:
        (runtime / 'LICENSES.txt').write_text(notices(seed.read('runtime/LICENSES.txt').decode(), 'macos'))
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
    # The licence notices read every target's crates from the local registry.
    sh(['cargo', 'fetch', '--locked'])
    target_dir = Path(json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1'],
                                                          cwd=ROOT))['target_directory'])
    kit_tool, patcher = target_dir / 'release/maiko-kit', target_dir / 'release/maiko-patcher'
    if not (ROOT / '.cache/client-toolchain/sr.wasm').exists():
        sh([kit_tool, 'tools'])
    if not (MACOS_TOOLCHAIN / 'ld64.wasm').exists():
        sh([kit_tool, 'tools-macos'])
    args.output.mkdir(parents=True)

    def kit(target):
        inputs = scratch / f'inputs-{target}'
        sh([sys.executable, '-m', 'tools.build_link_inputs', '--target', target, '--output', inputs])
        runtime = scratch / f'runtime-{target}'
        runtime_directory(target, inputs / 'runtime', runtime)
        sh([kit_tool, 'export', target, inputs / 'link', runtime, revision, args.output / ASSET.format(target)])

    def macos():
        shutil.rmtree(macos_kit(scratch, revision, kit_tool, args.output / ASSET.format('macos')), ignore_errors=True)

    # The three kits use separate containers and output files; Cargo's own lock orders their Rust builds.
    with concurrent.futures.ThreadPoolExecutor(3) as pool:
        for job in [pool.submit(kit, 'windows'), pool.submit(kit, 'linux'), pool.submit(macos)]:
            job.result()
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
