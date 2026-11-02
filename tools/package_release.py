#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Package only explicitly reviewed project files; no network or installation."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
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
    'windows/src/main.rs', 'windows/src/native.rs', 'windows/src/firmware.rs',
    'windows/src/native/winusb.rs', 'windows/tests/cli.rs',
    'windows/README.md', 'windows/BUILD.cmd', 'windows/RUN-DIAGNOSTICS.cmd',
    'windows/build-linux.sh', 'windows/TEST-CAMERA.cmd', 'windows/test-camera.ps1',
    'windows/autoload.ps1', 'windows/autoload-setup.ps1', 'windows/usb-suspend.ps1', 'windows/AUTOLOAD-INSTALL.cmd',
    'windows/AUTOLOAD-REMOVE.cmd',
    'windows/installer/INSTALL.cmd',
    'windows/installer/install.ps1', 'windows/installer/README.md',
    'windows/driver/samsung-vg-stc3000.inf', 'windows/driver/INSTALL.cmd',
    'windows/driver/install.ps1', 'windows/driver/CONFIGURE-INBOX.cmd',
    'windows/driver/configure-inbox.ps1', 'windows/driver/README.md',
    'windows/driver/local-driver.ps1', 'windows/driver/LOCAL-DRIVER.cmd',
    'windows/driver/LOCAL-DRIVER-REMOVE.cmd',
    'windows/installer/samsung-camera.iss', 'tools/build_windows_installer.py',
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
    windows_only = '--windows-only' in sys.argv[1:]
    linux = ROOT / 'target/release/samsung-camera'
    windows = ROOT / 'windows/samsung-camera-windows.exe'
    catalog = ROOT / 'windows/driver/samsung-vg-stc3000.cat'
    for binary in ((windows,) if windows_only else (linux, windows)):
        if not binary.is_file():
            raise SystemExit(f'Build required: {binary.name}')
    status = json.loads((ROOT / 'windows/PACKAGE_STATUS.json').read_text())
    if status['version'] != VERSION or status['binary_sha256'] != digest(windows):
        raise SystemExit('Windows release metadata is stale; rebuild/update before packaging.')
    if (status['read_only'] or not status['firmware_loader_implemented']
            or not status['driver_inf_included']
            or not status['inbox_winusb_configuration_included']):
        raise SystemExit('Unexpected Windows artifact scope.')
    signed_package = (catalog.is_file() or status['driver_package_included']
                      or status['signed_driver_catalog_included']
                      or status['driver_catalog_signature_verified'])
    if signed_package and (not catalog.is_file() or not status['driver_package_included']
                           or not status['signed_driver_catalog_included']
                           or not status['driver_catalog_signature_verified']):
        raise SystemExit('Custom-INF releases require a complete verified signed catalog.')
    if signed_package and os.name == 'nt':
        subprocess.run([
            'powershell.exe', '-NoProfile', '-File',
            str(ROOT / 'windows/driver/install.ps1'), '-Check',
        ], cwd=ROOT, check=True)
    out = ROOT / 'dist'
    out.mkdir(exist_ok=True)
    # Refuse to replace published/previously prepared versioned assets.
    names = [f'samsung-camera-{VERSION}-{suffix}' for suffix in
             ('linux-x86_64.tar.gz', 'windows-x86_64.zip', 'source.tar.gz')]
    if windows_only:
        names = [names[1]]
    if any((out / n).exists() for n in names):
        raise SystemExit('Versioned release assets already exist; inspect rather than overwrite.')
    with tempfile.TemporaryDirectory(prefix='samsung-release-') as scratch:
        work = Path(scratch)
        source = work / f'TVCameraSamsung-{VERSION}-source'
        source.mkdir()
        for name in () if windows_only else SOURCE_FILES:
            copy(ROOT / name, source / name)
        source_status = dict(
            status,
            artifact_kind='source-only camera utilities and unsigned driver INF',
            windows_executable_included=False,
            driver_package_included=False,
            signed_driver_catalog_included=False,
            driver_catalog_signature_verified=False,
        )
        source_status.pop('binary_sha256', None)
        lin = work / f'samsung-camera-{VERSION}-linux-x86_64'
        if not windows_only:
            (source / 'windows/PACKAGE_STATUS.json').write_text(json.dumps(source_status, indent=2) + '\n')
            manifest(source)
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
        win = work / f'samsung-camera-{VERSION}-windows-x86_64'
        win.mkdir()
        copy(windows, win / windows.name)
        for name in ('README.md', 'RUN-DIAGNOSTICS.cmd', 'PACKAGE_STATUS.json',
                     'TEST-CAMERA.cmd', 'test-camera.ps1', 'autoload.ps1', 'autoload-setup.ps1', 'usb-suspend.ps1',
                     'AUTOLOAD-INSTALL.cmd', 'AUTOLOAD-REMOVE.cmd'):
            copy(ROOT / 'windows' / name, win / name)
        copy(ROOT / 'LICENSE', win / 'LICENSE')
        for name in SOURCE_FILES:
            if name.startswith(('windows/installer/', 'windows/driver/')):
                copy(ROOT / name, win / Path(name).relative_to('windows'))
        if signed_package:
            copy(catalog, win / 'driver/samsung-vg-stc3000.cat')
        manifest(win)
        jobs = ((win, names[0], 'zip'),) if windows_only else (
            (lin, names[0], 'tar'), (win, names[1], 'zip'), (source, names[2], 'tar'))
        for directory, name, kind in jobs:
            archive(directory, out / name, kind)
        (out / ('SHA256SUMS-1.0.0-windows' if windows_only else 'SHA256SUMS-1.0.0')).write_text(''.join(f'{digest(out / name)}  {name}\n' for name in names))
    for name in names:
        print(f'{name}: {(out / name).stat().st_size} bytes')

if __name__ == '__main__':
    main()
