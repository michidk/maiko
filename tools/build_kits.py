#!/usr/bin/env python3
"""Build the Linux, Windows and macOS runtime kits from an owned RGFX.EXE.

It translates the executable, builds the three runtimes (without playing the game),
exports the kits and proves each one by building a package from it: the real patcher
for Linux and Windows, the website's real worker for macOS. Neither the executable nor
any generated game code leaves the work folders; only the three kit ZIPs, named like the release assets, are
written to --output.
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

MACOS_TARGETS = ['aarch64-unknown-linux-gnu', 'aarch64-apple-darwin', 'x86_64-apple-darwin', 'wasm32-unknown-unknown']
TARGETS = ['linux', 'windows', 'macos']
MACOS_TOOLCHAIN = ROOT / '.cache/client-macos-toolchain'
# Release assets are named by platform so a tag explains what it holds.
ASSET = 'maiko-runtime-kit-{}.zip'
RELEASE = 'https://github.com/michidk/maiko/releases/latest/download'


def sh(command, **options):
    print('+', ' '.join(str(part) for part in command), flush=True)
    return subprocess.run([str(part) for part in command], cwd=ROOT, check=True, **options)


def last_line(command):
    """Run a build tool and return the folder it prints last."""
    result = sh(command, stdout=subprocess.PIPE, text=True)
    sys.stdout.write(result.stdout)
    return ROOT / result.stdout.strip().splitlines()[-1]


def last_folder(command):
    """Run a tool that prints its output folder among its last lines and return that folder."""
    result = sh(command, stdout=subprocess.PIPE, text=True)
    sys.stdout.write(result.stdout)
    folders = [ROOT / line for line in result.stdout.splitlines() if (ROOT / line).is_dir()]
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


def llasm_translator(destination):
    """SR with the LLasm output, built from the pinned commit and the repository's patches."""
    from .pipeline import SR_COMMIT
    venv = ROOT / '.cache/venv'
    if not (venv / 'bin/scons').exists():
        sh([sys.executable, '-m', 'venv', venv])
        sh([venv / 'bin/pip', 'install', 'scons==4.10.1'])
    destination.mkdir(parents=True)
    archive = subprocess.check_output(['git', '-C', '.cache/sr', 'archive', SR_COMMIT, 'SR', 'llasm'], cwd=ROOT)
    subprocess.run(['tar', '-x', '-C', destination], input=archive, check=True)
    for patch in ['sr-x64', 'sr-llasm']:
        subprocess.run(['patch', '-s', '-p1', '-d', destination], input=(ROOT / f'patches/{patch}.patch').read_bytes(), check=True)
    defs = destination / 'SR/SR_defs.h'
    text = defs.read_text()
    assert text.count('#define OUTPUT_TYPE  OUT_X86') == 1, 'SR output configuration changed'
    defs.write_text(text.replace('#define OUTPUT_TYPE  OUT_X86', '#define OUTPUT_TYPE  OUT_LLASM'))
    print('+ scons -j2 (SR, LLasm output)', flush=True)
    subprocess.run([str(venv / 'bin/scons'), '-j2'], cwd=destination / 'SR', check=True)
    return destination / 'SR/SR.exe'


def browser_build(kit, exe, output, scratch, kit_tool, allow_mismatch=False):
    """Run web/worker.js under Bun with `kit`. Returns the universal executable the worker linked."""
    work = scratch / f'worker-{output.stem}'
    work.mkdir()
    for name in ['worker.js', 'wasi.js']:
        shutil.copyfile(ROOT / 'web' / name, work / name)
    shutil.copytree(scratch / 'pkg', work / 'pkg')
    sh([kit_tool, 'web-tools', kit, work])
    sh(['bun', ROOT / 'tools/browser_build.mjs', work, kit, exe, output] + (['--allow-mismatch'] if allow_mismatch else []))
    return Path(f'{output}.universal')


def macos_kit(exe, install, scratch, revision, kit_tool, output):
    """The macOS kit needs its own compilers and a universal executable that the browser itself produced.

    The kit holds the expected hash of that executable, so it is exported twice: a bootstrap kit lets
    the real worker build the executable, then the final kit pins it and the worker must reproduce it.
    """
    sh(['rustup', 'target', 'add', *MACOS_TARGETS])
    if not (MACOS_TOOLCHAIN / 'ld64.wasm').exists():
        sh([kit_tool, 'tools-macos'])
    translator = llasm_translator(scratch / 'sr-llasm')
    translation = last_folder([sys.executable, '-m', 'tools.translate_llasm', '--install', install, '--sr', translator])
    backend = last_folder([sys.executable, '-m', 'tools.check_macos_backend', '--translation', translation])
    runtime = backend / 'runtime'
    with fetch_seed('macos') as seed:
        (runtime / 'LICENSES.txt').write_bytes(seed.read('runtime/LICENSES.txt'))
    bootstrap = scratch / 'bootstrap.zip'
    sh([kit_tool, 'export-macos', backend, runtime, runtime / 'redguard-native', revision, bootstrap])
    sh(['cargo', 'build', '--locked', '--release', '-p', 'maiko-patcher', '--lib', '--target', 'wasm32-unknown-unknown'])
    target_dir = Path(json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1'],
                                                          cwd=ROOT))['target_directory'])
    sh(['wasm-bindgen', '--target', 'web', '--out-dir', scratch / 'pkg', target_dir / 'wasm32-unknown-unknown/release/maiko_patcher.wasm'])
    accepted = browser_build(bootstrap, exe, scratch / 'bootstrap-package.zip', scratch, kit_tool, allow_mismatch=True)
    executable = scratch / 'accepted-universal'
    shutil.copyfile(accepted, executable)
    sh([kit_tool, 'export-macos', backend, runtime, executable, revision, output])
    final = browser_build(output, exe, scratch / 'final-package.zip', scratch, kit_tool)
    if final.read_bytes() != executable.read_bytes():
        raise SystemExit('The final macOS kit did not reproduce the accepted universal executable')
    return [translation, backend]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exe', type=Path, required=True, help='Owned RGFX.EXE (the accepted original)')
    parser.add_argument('--output', type=Path, required=True, help='New folder for the three kit ZIPs')
    parser.add_argument('--wasmtime', type=Path, default=shutil.which('wasmtime'),
                        help='Wasmtime used to prove each kit with the real patcher')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('--output must not exist')
    if not args.wasmtime:
        parser.error('Install Wasmtime or pass --wasmtime')
    variants = json.loads((ROOT / 'metadata/image_variants.json').read_text())
    exe = args.exe.read_bytes()
    exe_path = args.exe.resolve()
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
            args.output / ASSET.format('linux')])
        sh([kit_tool, 'export', 'windows', windows / 'redguard-native.exe', objects / 'rgfx.obj', runtimes['windows'],
            revision, args.output / ASSET.format('windows')])
        for target in ['linux', 'windows']:
            package = scratch / f'verify-{target}'
            sh([patcher, 'build', '--input', args.exe, '--target', target, '--kit', args.output / ASSET.format(target),
                '--wasmtime', args.wasmtime, '--output', package])
            package.unlink()
        for directory in {translation, windows, linux}:
            shutil.rmtree(directory, ignore_errors=True)
        # The macOS stage needs most of the disk, so the first stage's output is gone before it starts.
        macos_folders = macos_kit(exe_path, install, scratch, revision, kit_tool, args.output / ASSET.format('macos'))
        print('Kits:', *sorted(p.name for p in args.output.iterdir()))
    finally:
        # The executable copy never stays behind. A failed build keeps its other folders
        # (translations, build output, scratch) for diagnosis; a successful one removes them.
        shutil.rmtree(install, ignore_errors=True)
        if sys.exc_info()[0] is None:
            shutil.rmtree(scratch, ignore_errors=True)
            for directory in locals().get('macos_folders', []):
                shutil.rmtree(directory, ignore_errors=True)


if __name__ == '__main__':
    main()
