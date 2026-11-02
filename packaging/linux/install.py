# SPDX-License-Identifier: GPL-3.0-only
"""Opt-in, fixed-destination installer; no USB operations or firmware download."""
import hashlib
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys

PACKAGE = Path(__file__).resolve().parents[2]
PIN = 'df71a001ec9045d9115b482f76cffe5e30dfac5fe26e7ef8a2bd25854af20b84'
DEST = {
    'binary': Path('/usr/local/libexec/samsung-camera'),
    'firmware': Path('/usr/local/share/samsung-camera/FalconFW.bin'),
    'unit': Path('/etc/systemd/system/samsung-camera-load@.service'),
    'rule': Path('/etc/udev/rules.d/70-samsung-vg-stc3000.rules'),
}
SOURCE = {'binary': 'bin/samsung-camera',
          'unit': 'packaging/systemd/samsung-camera-load@.service',
          'rule': 'packaging/udev/70-samsung-vg-stc3000.rules'}
RULE_DIRS = tuple(Path(p) for p in ('/etc/udev/rules.d', '/usr/lib/udev/rules.d',
                                   '/usr/local/lib/udev/rules.d', '/run/udev/rules.d'))
COMPETING = re.compile(r'205c|FalconFW|samsung.*(?:tvcam|fwload)', re.I)


def run(args):
    subprocess.run([str(a) for a in args], check=True)


def regular(path):
    if not stat.S_ISREG(path.lstat().st_mode):
        raise ValueError(f'Expected a regular, non-symlink file: {path}')


def trusted_parents(path, create=False):
    """Refuse symlinks and writable/non-root ancestors, including existing parents."""
    for parent in reversed(path.parents):
        if not parent.exists() and not parent.is_symlink():
            if not create:
                continue
            parent.mkdir(mode=0o755)
        info = parent.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
            raise ValueError(f'Unsafe destination parent: {parent}')


def conflict(path):
    if path.exists() or path.is_symlink():
        raise ValueError(f'Existing destination; do not overwrite: {path}')
    trusted_parents(path)


def sources(package, firmware):
    result = {role: package / name for role, name in SOURCE.items()}
    result['firmware'] = firmware
    for path in result.values():
        regular(path)
    return result


def check(package, firmware, destinations=None, rule_dirs=None):
    destinations = DEST if destinations is None else destinations
    rule_dirs = RULE_DIRS if rule_dirs is None else rule_dirs
    if os.geteuid() == 0:
        raise ValueError('Run the installer as an ordinary user, not root.')
    src = sources(package, firmware)
    for tool in ('pkexec', 'systemd-analyze', 'systemctl', 'udevadm'):
        if not shutil.which(tool):
            raise ValueError(f'Missing required tool: {tool}')
    for path in destinations.values():
        conflict(path)
    def scan_error(error):
        raise error
    for directory in rule_dirs:
        if directory.exists():
            for parent, _, names in os.walk(directory, onerror=scan_error):
                for name in names:
                    if name.endswith('.rules'):
                        path = Path(parent) / name
                        if COMPETING.search(path.read_text()):
                            raise ValueError(f'Possible competing initializer rule: {path}')
    # The packaged executable validates the firmware pin and loads its libusb dependency.
    run([src['binary'], 'inspect', firmware])
    print('Preflight passed. No files installed. Native unit verification occurs before rule installation.')
    return src


def copy_role(role, firmware):
    """Privileged entry: fixed destinations only, no replace, no USB or subprocesses."""
    if os.geteuid() != 0 or role not in DEST:
        raise ValueError('Privileged copy requires root and a known installation role.')
    source = firmware if role == 'firmware' else PACKAGE / SOURCE[role]
    regular(source)
    # Buffer once: the bytes checked are the bytes written, even if the source changes.
    fd = os.open(source, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, 'rb') as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError('Installation source is not a regular file.')
        data = stream.read(64 * 1024 * 1024 + 1)
    if len(data) > 64 * 1024 * 1024:
        raise ValueError('Installation source exceeds the 64 MiB bound.')
    if role == 'firmware' and hashlib.sha256(data).hexdigest() != PIN:
        raise ValueError('Firmware SHA-256 does not match the pinned image.')
    destination = DEST[role]
    trusted_parents(destination, create=True)
    fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                 0o755 if role == 'binary' else 0o644)
    with os.fdopen(fd, 'wb') as stream:
        stream.write(data)
        stream.flush()
        os.fchmod(stream.fileno(), 0o755 if role == 'binary' else 0o644)
    print(f'Installed {destination}')


def main(argv):
    if len(argv) == 3 and argv[0] == '--copy-role':
        copy_role(argv[1], Path(argv[2]))
        return
    if len(argv) != 2 or argv[0] not in ('--check', '--install'):
        raise ValueError('Usage: install.sh --check FIRMWARE | --install FIRMWARE')
    firmware = Path(argv[1]).absolute()
    check(PACKAGE, firmware)
    if argv[0] == '--check':
        return
    pkexec = shutil.which('pkexec')
    helper = Path(__file__).resolve()
    for role in ('binary', 'firmware', 'unit'):
        run([pkexec, '/usr/bin/python3', '-I', '-B', helper, '--copy-role', role, firmware])
    run([DEST['binary'], 'inspect', DEST['firmware']])
    run([shutil.which('systemd-analyze'), 'verify', '--man=no', '--instance=001-011', DEST['unit']])
    run([pkexec, '/usr/bin/python3', '-I', '-B', helper, '--copy-role', 'rule', firmware])
    run([pkexec, '/usr/bin/systemctl', 'daemon-reload'])
    run([pkexec, '/usr/bin/udevadm', 'control', '--reload-rules'])
    print('Installed. No device was triggered. See packaging/linux/README.md for reconnect verification.')


if __name__ == '__main__':
    try:
        main(sys.argv[1:])
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f'Installation stopped: {error}. Earlier files may remain; inspect before retrying.', file=sys.stderr)
        sys.exit(1)
