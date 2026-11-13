#!/usr/bin/env python3
"""Check SR LLVM/Mach-O diagnostics and optionally cross-link an emitted game."""
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
           'openglide-sdl-window.patch', 'openglide-macos.patch',
           'sr-heap-c-api.patch', 'sr-heap-macos.patch', 'sr-audio.patch']
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

FIXTURE_TARGETS = {
    'linux': 'x86_64-unknown-linux-gnu',
    'linux-arm64': 'aarch64-unknown-linux-gnu',
    'x86_64': 'x86_64-apple-darwin',
    'arm64': 'aarch64-apple-darwin',
}


def stage_rust(container, game=False):
    """Link diagnostics to the same Rust instruction and Darwin implementations.

    The instruction-only archives avoid pulling SDL into the CPU fixture. The
    memory diagnostic uses the real runtime with its normal, non-offset feature
    set; it checks Darwin queries, not guest arena initialization.
    """
    from .rust_runtime import build
    cargo = shutil.which('cargo') or str(Path.home() / '.cargo/bin/cargo')
    rustc = shutil.which('rustc') or str(Path.home() / '.cargo/bin/rustc')
    metadata = json.loads(subprocess.check_output(
        [cargo, 'metadata', '--format-version=1', '--no-deps', '--locked'],
        cwd=ROOT, text=True))
    directory = Path(metadata['target_directory'])
    reports = {}
    for name, target in FIXTURE_TARGETS.items():
        destination = directory / target / 'release'
        destination.mkdir(parents=True, exist_ok=True)
        archive = destination / 'libmaiko_llasm_fixture.a'
        run(rustc, '--crate-type', 'staticlib', '--edition', '2024',
            '-C', 'opt-level=2', '-C', 'panic=abort', '--target', target,
            str(ROOT / 'runtime/rust/llasm.rs'), '-o', str(archive))
        run('docker', 'cp', str(archive),
            container + f':/results/libmaiko-llasm-{name}.a')
        reports[name] = {'target': target, 'helper_sha256': digest(archive)}
        if target.endswith('apple-darwin'):
            runtime, report = build(target)
            run('docker', 'cp', str(runtime),
                container + f':/results/libmaiko-runtime-{name}.a')
            reports[name]['runtime'] = report
            if game:
                host, report = build(target, component='host', llasm=True)
                run('docker', 'cp', str(host),
                    container + f':/results/libmaiko-host-{name}.a')
                reports[name]['host'] = report
    return reports


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def run(*args, **kwargs):
    return subprocess.run(list(args), check=True, **kwargs)


def source_manifest(root=ROOT):
    paths = [root / name for name in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml']]
    paths += [root / 'tools' / name for name in
              ('check_macos_backend.py', 'rust_runtime.py', 'sr_heap.py', 'sr_audio.py')]
    paths += [root / 'patches' / name for name in PATCHES]
    extensions = {'.c', '.cpp', '.h', '.py', '.sh', '.asm', '.inc', '.cmd', '.command',
                  '.rs', '.toml', '.sci', '.json'}
    for directory in ['macos', 'tests', 'runtime', 'glide', 'linux', 'audio', 'llasm-metadata']:
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
    parser.add_argument('--reuse-toolchain', action='store_true',
                        help='Use the existing local compiler image instead of rebuilding it')
    parser.add_argument('--translation', type=Path,
                        help='Also cross-link an original-game tools.translate_llasm output for both Macs')
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
    translation = args.translation.resolve() if args.translation else None
    if translation:
        original = json.loads((translation / 'report.json').read_text())
        if original['status'] != 'llasm_emitted_not_runtime_verified':
            parser.error('Requires successfully emitted tools.translate_llasm output')
        report['translation_image_sha256'] = original['image']['sha256']
        report['translation_sha256'] = {p.name: digest(p) for p in translation.iterdir()
                                        if p.suffix in ('.llasm', '.llinc')}
    report['source_control'] = 'Uncommitted working-tree inputs; identify them by the source manifest'
    report['source_sha256'] = source_manifest()
    (output / 'source-manifest.json').write_text(json.dumps(report['source_sha256'], indent=2) + '\n')
    report['llasm_flags'] = ['-m64', '-inline-idiv', '-pic', '-ptrofs', '-O']
    report['llvm_driver'] = 'opt -O3 then llc -O=3 -filetype=obj --relocation-model=pic, with target-specific -mtriple'
    if translation:
        report['game_llvm_driver'] = 'opt -O2 then llc -O=2 -filetype=obj --relocation-model=pic, with target-specific -mtriple'
    began = time.monotonic()
    container = 'rg-macos-' + uuid.uuid4().hex[:10]
    exported = False
    try:
        downloads = {name: download(cache, name, *values) for name, values in DOWNLOADS.items()}
        report['downloads_sha256'] = {name: digest(path) for name, path in downloads.items()}
        report['patch_sha256'] = {name: digest(ROOT / 'patches' / name) for name in PATCHES}
        if not args.reuse_toolchain:
            with (output / 'toolchain-build.log').open('w') as log:
                run('docker', 'build', '-t', IMAGE, '-f', str(ROOT / 'macos/Dockerfile'),
                    str(ROOT / 'macos'), stdout=log, stderr=subprocess.STDOUT,
                    timeout=600)
        report['toolchain_image_reused'] = args.reuse_toolchain
        report['toolchain_image'] = subprocess.check_output(
            ['docker', 'image', 'inspect', IMAGE, '--format', '{{.Id}}'], text=True).strip()
        memory = '6g' if translation else '3g'
        run('docker', 'run', '-d', '--network', 'none', '--memory', memory, '--memory-swap', memory,
            '--cpus', '2', '--name', container, IMAGE, 'sleep', '3600' if translation or args.keep_container else '900',
            stdout=subprocess.DEVNULL)
        run('docker', 'exec', container, 'mkdir', '-p', '/results')
        report['rust_archives'] = stage_rust(container, game=bool(translation))
        if translation:
            from .sr_heap import stage_sr_heap
            from .sr_audio import stage_sr_audio
            report['heap'] = stage_sr_heap(container)
            report['audio'] = stage_sr_audio(container)
            run('docker', 'exec', container, 'mkdir', '-p', '/game')
            for source in translation.iterdir():
                if source.suffix in ('.llasm', '.llinc'):
                    run('docker', 'cp', str(source), container + ':/game/' + source.name)
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
            run('docker', 'cp', str(path.resolve()), container + ':/work/' + name)
        for directory in ['macos', 'tests', 'runtime', 'glide', 'linux', 'llasm-metadata']:
            run('docker', 'cp', str(ROOT / directory), container + ':/work/' + directory)
        for name in PATCHES:
            run('docker', 'cp', str(ROOT / 'patches' / name), container + ':/work/' + name)
        with (output / 'build-test.log').open('w') as log:
            run('docker', 'exec', container, 'timeout', '1200' if translation else '600', 'sh', '/work/macos/check_backend.sh',
                stdout=log, stderr=subprocess.STDOUT)
        run('docker', 'cp', container + ':/results/.', str(output))
        exported = True
        verify_source_manifest(report['source_sha256'])
        report['status'] = 'synthetic_linux_execution_and_macos_cross_link_passed'
        report['linux_translated_fixture_verified'] = True
        report['linux_arm64_emulated_fixture_verified'] = True
        report['darwin_rust_runtime_cross_linked'] = True
        if translation:
            current = {p.name: digest(p) for p in translation.iterdir()
                       if p.suffix in ('.llasm', '.llinc')}
            if current != report['translation_sha256']:
                raise RuntimeError('Translation changed during the build')
            report['macos_game_built'] = True
            report['status'] = 'synthetic_execution_and_macos_game_cross_link_passed'
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
