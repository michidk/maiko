#!/usr/bin/env python3
"""Package the verified diagnostic for the owner's Windows test."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parent

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('work', type=Path)
    parser.add_argument('--output', type=Path, default=ROOT / 'artifacts/download')
    args = parser.parse_args()
    report = json.loads((args.work / 'report.json').read_text())
    if not report.get('wine_entry_boundary_verified'):
        parser.error('Package only after the diagnostic Wine entry check passes')
    exe = args.work / 'redguard-probe.exe'
    if report.get('exe_sha256') != hashlib.sha256(exe.read_bytes()).hexdigest():
        parser.error('Executable does not match the verified build report')
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    archive = out / 'redguard-recomp-diagnostic.zip'
    readme = '''REDGUARD RECOMPILATION DIAGNOSTIC - NOT A MENU BUILD

Extract the ZIP to a writable folder and double-click RUN.cmd.
No installation, administrator rights, or game-file changes are needed.
The test prints a translated-startup message, then:
  BLOCKED original program requested INT 21 EAX=00003000
The expected exit code is 2. There is no game window or menu yet.
Send diagnostic.log back to compare native Windows against the Wine result.

This executable was generated from the owner's Redguard copy for this test.
It contains translated original code. It is not a general public release.
It does not read a game installation, modify saves, or contact the network.
'''
    cmd = '@echo off\r\ncd /d "%~dp0"\r\nredguard-probe.exe > diagnostic.log 2>&1\r\nset "probe_exit=%errorlevel%"\r\n>>diagnostic.log echo Exit code: %probe_exit%\r\ntype diagnostic.log\r\necho.\r\necho Expected: INT 21 EAX=00003000 and exit code 2. This is NOT a menu build.\r\npause\r\n'
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as z:
        z.write(exe, 'redguard-probe.exe')
        for notice in sorted((args.work / 'notices').glob('*.txt')):
            z.write(notice, 'notices/' + notice.name)
        z.writestr('README.txt', readme)
        z.writestr('RUN.cmd', cmd)
        z.writestr('build-report.json', json.dumps(report, indent=2))
    sha = hashlib.sha256(archive.read_bytes()).hexdigest()
    (out / 'SHA256SUMS.txt').write_text(f'{sha}  {archive.name}\n')
    (out / 'index.html').write_text('''<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Redguard recompilation diagnostic</title>
<style>body{font:18px/1.6 system-ui;max-width:760px;margin:60px auto;padding:0 24px;color:#29231e;background:#f5f0e7}a{color:#75430f}h1{line-height:1.2}</style>
<h1>Redguard recompilation diagnostic</h1>
<p><strong>This is not the menu PoC yet.</strong> The Windows x64 executable runs translated original startup code and stops at the first unimplemented DOS service.</p>
<p><a href="redguard-recomp-diagnostic.zip" download>Download the Windows diagnostic ZIP</a></p>
<p>Extract it and double-click <code>RUN.cmd</code>. Expected: <code>INT 21 EAX=00003000</code> and exit code <code>2</code>. The launcher writes <code>diagnostic.log</code> beside the executable. It does not modify a game installation.</p>
<p>Wine has passed this boundary check. Native Windows testing is pending. Menu rendering, input and Glide support remain unfinished.</p>
<p><a href="SHA256SUMS.txt">ZIP checksum</a></p>''')
    print(archive)
    print(sha)

if __name__ == '__main__':
    main()
