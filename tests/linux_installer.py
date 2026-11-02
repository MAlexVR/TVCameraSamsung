"""Offline installer checks. Never invoke polkit or access camera devices."""
from contextlib import ExitStack, redirect_stdout
import hashlib
import importlib.util
import io
from pathlib import Path
import os
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('installer', ROOT / 'packaging/linux/install.py')
installer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(installer)


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.scratch = Path(tempfile.mkdtemp(prefix='samsung-linux-installer-'))
        print(f'Retained scratch: {self.scratch}')
        self.package = self.scratch / 'package'
        for role, name in installer.SOURCE.items():
            p = self.package / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text('asset')
        self.binary = self.package / installer.SOURCE['binary']
        self.binary.write_text('#!/bin/sh\n[ "$1" = inspect ] && [ "$(cat "$2")" = firmware ]\n')
        self.binary.chmod(0o755)
        self.firmware = self.scratch / 'firmware.bin'
        self.firmware.write_bytes(b'firmware')
        self.dest = {r: self.scratch / 'system' / p.name for r, p in installer.DEST.items()}
        (self.scratch / 'system').mkdir()
        self.rules = self.scratch / 'rules'
        self.rules.mkdir()

    def environment(self):
        stack = ExitStack()
        # Fixture-only trust/tool simulation; never changes the production CLI.
        stack.enter_context(patch.object(installer, 'trusted_parents'))
        stack.enter_context(patch.object(installer.os, 'geteuid', return_value=1000))
        stack.enter_context(patch.object(installer.shutil, 'which', side_effect=lambda n: '/usr/bin/' + n))
        stack.enter_context(redirect_stdout(io.StringIO()))
        return stack

    def check(self):
        return installer.check(self.package, self.firmware, self.dest, (self.rules,))

    def test_release_installer_exists(self):
        self.assertTrue((ROOT / 'packaging/linux/install.sh').is_file())

    def test_help_without_external_tools(self):
        result = subprocess.run(['/bin/sh', str(ROOT / 'packaging/linux/install.sh'), '--help'],
                                env={'PATH': '/nonexistent'}, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0)
        self.assertIn('--install', result.stdout)

    def test_check_writes_nothing_and_never_invokes_pkexec(self):
        before = sorted(str(p) for p in self.scratch.rglob('*'))
        with self.environment(), patch.object(installer, 'run', wraps=installer.run) as run:
            self.check()
        self.assertEqual(before, sorted(str(p) for p in self.scratch.rglob('*')))
        self.assertEqual(len(run.call_args_list), 1)
        self.assertEqual(run.call_args.args[0][1], 'inspect')

    def test_existing_file_or_symlink_refused_before_commands(self):
        for symlink in (False, True):
            destination = self.dest['rule']
            if symlink:
                destination.symlink_to(self.scratch / 'missing')
            else:
                destination.write_text('existing')
            with self.environment(), patch.object(installer, 'run') as run:
                with self.assertRaisesRegex(ValueError, 'Existing destination'):
                    self.check()
                run.assert_not_called()
            destination.unlink()  # Only this test-owned fixture, never installed paths.

    def test_invalid_firmware_stops_without_privileged_operations(self):
        self.firmware.write_bytes(b'invalid')
        with self.environment(), patch.object(installer, 'run', wraps=installer.run) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                self.check()
        self.assertEqual(len(run.call_args_list), 1)
        self.assertEqual(run.call_args.args[0][0], self.binary)

    def test_competing_rule_refused(self):
        (self.rules / 'other.rules').write_text('ATTR{idProduct}=="205c"')
        with self.environment(), patch.object(installer, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'competing'):
                self.check()
            run.assert_not_called()

    def test_missing_tool_refused(self):
        with self.environment(), patch.object(installer.shutil, 'which', return_value=None):
            with self.assertRaisesRegex(ValueError, 'Missing required tool'):
                self.check()

    def test_source_symlink_refused(self):
        self.firmware.unlink()
        self.firmware.symlink_to(self.binary)
        with self.environment():
            with self.assertRaisesRegex(ValueError, 'non-symlink'):
                self.check()

    def test_real_parent_check_refuses_user_owned_or_writable_directory(self):
        with self.assertRaisesRegex(ValueError, 'Unsafe destination parent'):
            installer.trusted_parents(self.scratch / 'new')

    def test_privileged_copy_is_exclusive_and_checks_firmware_pin(self):
        with patch.object(installer, 'DEST', self.dest), \
             patch.object(installer, 'trusted_parents'), \
             patch.object(installer.os, 'geteuid', return_value=0), \
             redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(ValueError, 'SHA-256'):
                installer.copy_role('firmware', self.firmware)
            self.assertFalse(self.dest['firmware'].exists())
            with patch.object(installer, 'PIN', hashlib.sha256(b'firmware').hexdigest()):
                installer.copy_role('firmware', self.firmware)
                self.assertEqual(self.dest['firmware'].read_bytes(), b'firmware')
                with self.assertRaises(FileExistsError):
                    installer.copy_role('firmware', self.firmware)

    def test_install_verifies_before_rule_and_never_triggers_devices(self):
        with self.environment(), patch.object(installer, 'check'), \
             patch.object(installer, 'run') as run:
            installer.main(['--install', str(self.firmware)])
        commands = [[str(a) for a in call.args[0]] for call in run.call_args_list]
        rule = next(i for i, c in enumerate(commands) if '--copy-role' in c and 'rule' in c)
        verify = next(i for i, c in enumerate(commands) if 'verify' in c)
        self.assertLess(verify, rule)
        self.assertTrue(all('trigger' not in c and 'load' not in c for c in commands))
        self.assertIn('-I', commands[0])
        self.assertIn('-B', commands[0])

    def test_failed_native_verification_never_installs_rule(self):
        def fake_run(args):
            if 'verify' in args:
                raise subprocess.CalledProcessError(1, args)
        with self.environment(), patch.object(installer, 'check'), \
             patch.object(installer, 'run', side_effect=fake_run) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                installer.main(['--install', str(self.firmware)])
        self.assertFalse(any('rule' in call.args[0] for call in run.call_args_list))


if __name__ == '__main__':
    unittest.main()
