#!/usr/bin/env python3
"""Build synthetic SR LLVM/Mach-O diagnostics. This does not build Redguard."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[1]
SR = 'ac690ddf3010bc3d2c875cb1b5e8da4f53a8d5b3'
IMAGE = 'redguard-recomp-macos:bookworm'
OPENGLIDE = '9f8a2cb73af3b6d26e9ec67bb9e1873d3e211726'
PATCHES = ['sr-x64.patch', 'sr-llasm.patch', 'openglide-x64.patch',
           'openglide-sdl-window.patch', 'openglide-macos.patch']
DOWNLOADS = {
    'MacOSX11.3.sdk.tar.xz': (
        'https://github.com/joseluisq/macosx-sdks/releases/download/11.3/MacOSX11.3.sdk.tar.xz',
        '9adc1373d3879e1973d28ad9f17c9051b02931674a3ec2a2498128989ece2cb1'),
    'SDL2-2.30.11.dmg': (
        'https://github.com/libsdl-org/SDL/releases/download/release-2.30.11/SDL2-2.30.11.dmg',
        '7b9daa7b8440e5a15f1a596df5a1ba326e3f6bf805a0307ce68ba2876192235f'),
    'SDL2_mixer-2.8.1.dmg': (
        'https://github.com/libsdl-org/SDL_mixer/releases/download/release-2.8.1/SDL2_mixer-2.8.1.dmg',
        'd74052391ee4d91836bf1072a060f1d821710f3498a54996c66b9a17c79a72d1'),
}


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def run(*args, **kwargs):
    return subprocess.run(list(args), check=True, **kwargs)


def source_manifest(root=ROOT):
    paths = [root / 'tools/check_macos_backend.py', *(root / 'patches' / name for name in PATCHES)]
    extensions = {'.c', '.cpp', '.h', '.py', '.sh', '.asm', '.inc', '.cmd'}
    for directory in ['macos', 'tests', 'runtime', 'glide', 'linux']:
        paths.extend(path for path in (root / directory).rglob('*')
                     if path.is_file() and '__pycache__' not in path.parts
                     and (path.suffix in extensions or path.name == 'Dockerfile'))
    return {path.relative_to(root).as_posix(): digest(path) for path in sorted(set(paths))}


def verify_source_manifest(expected, root=ROOT):
    if source_manifest(root) != expected:
        raise RuntimeError('Source inputs changed during the build; rerun from stable inputs')


def download(cache, name, url, expected):
    path = cache / name
    if not path.exists():
        temporary = path.with_suffix(path.suffix + '.partial')
        try:
            with urllib.request.urlopen(url, timeout=90) as response, temporary.open('wb') as destination:
                shutil.copyfileobj(response, destination)
            if digest(temporary) != expected:
                raise ValueError('Download checksum mismatch: ' + name)
            temporary.replace(path)
        finally:
            temporary.unlink(missing_ok=True)
    if digest(path) != expected:
        raise ValueError('Cached checksum mismatch: ' + name)
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--keep-container', action='store_true', help='Retain bounded tools for follow-up compilation')
    args = parser.parse_args()
    if sys.version_info < (3, 11):
        parser.error('Python 3.11 or newer is required')
    if shutil.disk_usage(ROOT).free < 60_000_000_000:
        parser.error('Less than 60 GB free; clean disposable output before building')
    cache = ROOT / '.cache/macos'
    cache.mkdir(parents=True, exist_ok=True)
    output = ROOT / 'artifacts' / (datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ') + '-macos-backend')
    output.mkdir(parents=True)
    report = dict(status='started', sr_commit=SR, macos_runtime_verified=False,
                  macos_game_built=False, full_game_verified=False, openglide_commit=OPENGLIDE)
    report['source_control'] = 'Uncommitted working-tree inputs; identify them by the source manifest'
    report['source_sha256'] = source_manifest()
    (output / 'source-manifest.json').write_text(json.dumps(report['source_sha256'], indent=2) + '\n')
    report['llasm_flags'] = ['-m64', '-inline-idiv', '-pic', '-ptrofs', '-O']
    report['llvm_driver'] = 'opt -O3 then llc -O=3 -filetype=obj --relocation-model=pic, with target-specific -mtriple'
    began = time.monotonic()
    container = 'rg-macos-' + uuid.uuid4().hex[:10]
    exported = False
    try:
        downloads = {name: download(cache, name, *values) for name, values in DOWNLOADS.items()}
        report['downloads_sha256'] = {name: digest(path) for name, path in downloads.items()}
        report['patch_sha256'] = {name: digest(ROOT / 'patches' / name) for name in PATCHES}
        with (output / 'toolchain-build.log').open('w') as log:
            run('docker', 'build', '-t', IMAGE, '-f', str(ROOT / 'macos/Dockerfile'),
                str(ROOT / 'macos'), stdout=log, stderr=subprocess.STDOUT)
        report['toolchain_image'] = subprocess.check_output(
            ['docker', 'image', 'inspect', IMAGE, '--format', '{{.Id}}'], text=True).strip()
        run('docker', 'run', '-d', '--network', 'none', '--memory', '3g', '--memory-swap', '3g',
            '--cpus', '2', '--name', container, IMAGE, 'sleep', '1800' if args.keep_container else '900',
            stdout=subprocess.DEVNULL)
        archive = cache / 'sr-backend.tar'
        with archive.open('wb') as destination:
            run('git', '-C', str(ROOT / '.cache/sr'), 'archive', SR, 'SR', 'llasm', stdout=destination)
        run('docker', 'cp', str(archive), container + ':/work/sr.tar')
        report['sr_source_archive_sha256'] = digest(archive)
        archive = cache / 'openglide-backend.tar'
        with archive.open('wb') as destination:
            run('git', '-C', str(ROOT / '.cache/openglide-ng'), 'archive', OPENGLIDE, stdout=destination)
        run('docker', 'cp', str(archive), container + ':/work/openglide.tar')
        report['openglide_source_archive_sha256'] = digest(archive)
        for name, path in downloads.items():
            run('docker', 'cp', str(path), container + ':/work/' + name)
        for directory in ['macos', 'tests', 'runtime', 'glide', 'linux']:
            run('docker', 'cp', str(ROOT / directory), container + ':/work/' + directory)
        for name in PATCHES:
            run('docker', 'cp', str(ROOT / 'patches' / name), container + ':/work/' + name)
        with (output / 'build-test.log').open('w') as log:
            run('docker', 'exec', container, 'timeout', '600', 'sh', '/work/macos/check_backend.sh',
                stdout=log, stderr=subprocess.STDOUT)
        run('docker', 'cp', container + ':/results/.', str(output))
        exported = True
        verify_source_manifest(report['source_sha256'])
        report['status'] = 'synthetic_linux_execution_and_macos_cross_link_passed'
        report['linux_translated_fixture_verified'] = True
        report['linux_arm64_emulated_fixture_verified'] = True
        report['darwin_posix_objects_compiled'] = True
    except Exception as error:
        report['status'] = 'failed'
        report['error'] = str(error)
        raise
    finally:
        if not exported:
            subprocess.run(['docker', 'cp', container + ':/results/.', str(output)],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        report['elapsed_seconds'] = round(time.monotonic() - began, 3)
        report['artifact_sha256'] = {p.relative_to(output).as_posix(): digest(p)
                                     for p in sorted(output.rglob('*')) if p.is_file()}
        if args.keep_container:
            report['development_container'] = container
        else:
            subprocess.run(['docker', 'rm', '-f', container], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(output)


if __name__ == '__main__':
    main()
