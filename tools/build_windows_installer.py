#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Compile windows/installer/samsung-camera.iss with a local Inno Setup 6."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
CANDIDATES = (
    os.environ.get('ISCC', ''),
    r'C:\Program Files (x86)\Inno Setup 6\ISCC.exe',
    r'C:\Program Files\Inno Setup 6\ISCC.exe',
)


def main():
    exe = ROOT / 'windows/samsung-camera-windows.exe'
    status = json.loads((ROOT / 'windows/PACKAGE_STATUS.json').read_text())
    if not exe.is_file() or status['binary_sha256'] != hashlib.sha256(exe.read_bytes()).hexdigest():
        raise SystemExit('Windows EXE is missing or stale versus PACKAGE_STATUS.json.')
    iscc = shutil.which('iscc') or next((c for c in CANDIDATES if c and Path(c).is_file()), None)
    if not iscc:
        raise SystemExit('Inno Setup 6 (ISCC.exe) not found; set ISCC to its path.')
    target = ROOT / 'dist/samsung-camera-1.0.0-windows-setup.exe'
    if target.exists():
        raise SystemExit('Installer already exists; inspect rather than overwrite.')
    subprocess.run([iscc, '/Q', str(ROOT / 'windows/installer/samsung-camera.iss')], check=True)
    print(f'{target.name}: {target.stat().st_size} bytes, sha256 {hashlib.sha256(target.read_bytes()).hexdigest()}')


if __name__ == '__main__':
    main()
