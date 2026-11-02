#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Package only explicitly reviewed project files; no network or installation."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
VERSION = '1.0.0'
SOURCE_FILES = (
    'Cargo.toml', 'Cargo.lock', 'LICENSE', 'README.md', '.gitignore',
    'src/lib.rs', 'src/main.rs', 'src/linux.rs', 'src/led.rs', 'src/linux/led.rs',
    'tests/cli.rs', 'tests/autoload.py', 'tests/linux_installer.py',
    'tests/windows_installer.py', 'tests/release_package.py',
    'packaging/systemd/samsung-camera-load@.service',
    'packaging/udev/70-samsung-vg-stc3000.rules',
    'packaging/linux/install.sh', 'packaging/linux/install.py', 'packaging/linux/README.md',
    'docs/AUTOLOAD.md', 'docs/WINDOWS.md', 'docs/RELEASE-1.0.0.md',
    'windows/Cargo.toml', 'windows/Cargo.lock', 'windows/src/lib.rs',
    'windows/src/main.rs', 'windows/src/native.rs', 'windows/tests/cli.rs',
    'windows/README.md', 'windows/BUILD.cmd', 'windows/RUN-DIAGNOSTICS.cmd',
    'windows/build-linux.sh', 'windows/installer/INSTALL.cmd',
    'windows/installer/install.ps1', 'windows/installer/README.md',
    'tools/package_release.py',
)

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def copy(source, destination):
    if source.is_symlink() or not source.is_file():
        raise ValueError(f'Missing or symlinked release input: {source.name}')
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    destination.chmod(0o755 if source.suffix == '.sh' or source.name in ('samsung-camera',) else 0o644)


def manifest(directory):
    lines = [f'{digest(p)}  {p.relative_to(directory).as_posix()}\n'
             for p in sorted(directory.rglob('*')) if p.is_file() and p.name != 'SHA256SUMS']
    (directory / 'SHA256SUMS').write_text(''.join(lines))


def archive(directory, output, kind):
    if kind == 'zip':
        with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as z:
            for p in sorted(directory.rglob('*')):
                if p.is_file():
                    z.write(p, (Path(directory.name) / p.relative_to(directory)).as_posix())
    else:
        with tarfile.open(output, 'w:gz') as t:
            t.add(directory, arcname=directory.name)


def main():
    linux = ROOT / 'target/release/samsung-camera'
    windows = ROOT / 'windows/samsung-camera-windows.exe'
    for binary in (linux, windows):
        if not binary.is_file():
            raise SystemExit(f'Build required: {binary.name}')
    status = json.loads((ROOT / 'windows/PACKAGE_STATUS.json').read_text())
    if status['version'] != VERSION or status['binary_sha256'] != digest(windows):
        raise SystemExit('Windows release metadata is stale; rebuild/update before packaging.')
    if not status['read_only'] or status['firmware_loader_implemented'] or status['driver_package_included']:
        raise SystemExit('Unexpected Windows artifact scope.')
    out = ROOT / 'dist'
    out.mkdir(exist_ok=True)
    # Refuse to replace published/previously prepared versioned assets.
    names = [f'samsung-camera-{VERSION}-{suffix}' for suffix in
             ('linux-x86_64.tar.gz', 'windows-x86_64-diagnostic.zip', 'source.tar.gz')]
    if any((out / n).exists() for n in names):
        raise SystemExit('Versioned release assets already exist; inspect rather than overwrite.')
    with tempfile.TemporaryDirectory(prefix='samsung-release-') as scratch:
        work = Path(scratch)
        source = work / f'TVCameraSamsung-{VERSION}-source'
        source.mkdir()
        for name in SOURCE_FILES:
            copy(ROOT / name, source / name)
        source_status = dict(status, artifact_kind='source-only diagnostic', windows_executable_included=False)
        source_status.pop('binary_sha256', None)
        (source / 'windows/PACKAGE_STATUS.json').write_text(json.dumps(source_status, indent=2) + '\n')
        manifest(source)
        lin = work / f'samsung-camera-{VERSION}-linux-x86_64'
        lin.mkdir()
        copy(linux, lin / 'bin/samsung-camera')
        for name in SOURCE_FILES:
            if name.startswith('packaging/') or name in ('LICENSE', 'README.md', 'docs/AUTOLOAD.md'):
                copy(ROOT / name, lin / name)
        build_info = subprocess.check_output(['ldd', str(linux)], text=True)
        # ldd virtual addresses are runtime details, not meaningful dependency versions.
        build_info = '\n'.join(line.split(' (0x')[0] for line in build_info.splitlines())
        (lin / 'BUILD-INFO.txt').write_text(
            'Version 1.0.0; built on Fedora 44 x86-64 using Rust 1.88.0.\n'
            'Requires compatible glibc/libusb runtime, systemd/udev, Python3 and polkit.\n'
            'Not tested on other distributions. No firmware included.\n' + build_info + '\n')
        manifest(lin)
        win = work / f'samsung-camera-{VERSION}-windows-x86_64-diagnostic'
        win.mkdir()
        copy(windows, win / windows.name)
        for name in ('README.md', 'RUN-DIAGNOSTICS.cmd', 'PACKAGE_STATUS.json'):
            copy(ROOT / 'windows' / name, win / name)
        copy(ROOT / 'LICENSE', win / 'LICENSE')
        for name in SOURCE_FILES:
            if name.startswith('windows/installer/'):
                copy(ROOT / name, win / Path(name).relative_to('windows'))
        manifest(win)
        for directory, name, kind in ((lin, names[0], 'tar'), (win, names[1], 'zip'), (source, names[2], 'tar')):
            archive(directory, out / name, kind)
        (out / 'SHA256SUMS-1.0.0').write_text(''.join(f'{digest(out / name)}  {name}\n' for name in names))
    for name in names:
        print(f'{name}: {(out / name).stat().st_size} bytes')

if __name__ == '__main__':
    main()
