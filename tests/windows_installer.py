#!/usr/bin/env python3
"""Portable structural checks only; these do not execute PowerShell or Windows."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class WindowsInstallerStructure(unittest.TestCase):
    def setUp(self):
        self.script = (ROOT / 'windows/installer/install.ps1').read_text()
        self.wrapper = (ROOT / 'windows/installer/INSTALL.cmd').read_text()
        self.readme = (ROOT / 'windows/installer/README.md').read_text()

    def test_explicit_consent_precedes_writes(self):
        self.assertIn('[switch]$Install', self.script)
        self.assertIn('[switch]$Check', self.script)
        self.assertLess(self.script.index('if (-not $Install)'),
                        self.script.index('[System.IO.Directory]::CreateDirectory'))
        self.assertIn('Read-Host', self.script)

    def test_checksum_and_diagnostic_metadata_precede_writes(self):
        for term in ('ConvertFrom-Json', 'Get-FileHash', 'SHA256',
                     'firmware_loader_implemented', 'read_only',
                     "'1.0.0'", 'binary_sha256'):
            self.assertIn(term, self.script)
        self.assertLess(self.script.index('Get-FileHash'),
                        self.script.index('[System.IO.Directory]::CreateDirectory'))
        self.assertIn('-isnot [bool]', self.script)

    def test_bounded_copy_and_no_overwrite(self):
        for filename in ('samsung-camera-windows.exe', 'RUN-DIAGNOSTICS.cmd',
                         'PACKAGE_STATUS.json', 'README.md'):
            self.assertIn(filename, self.script)
        self.assertIn('[System.IO.File]::Copy($entry.Source, $destination, $false)', self.script)
        self.assertIn('[System.IO.Directory]::Move($staging, $target)', self.script)
        self.assertNotIn('Copy-Item', self.script)
        self.assertNotIn('-Recurse', self.script)

    def test_reparse_guards_and_no_policy_or_device_mutations(self):
        self.assertIn('[System.IO.FileAttributes]::ReparsePoint', self.script)
        self.assertIn('Test-Path -LiteralPath $target', self.script)
        code = '\n'.join(line for line in self.script.splitlines()
                         if not line.strip().startswith('#'))
        forbidden = ('Set-ExecutionPolicy', '-ExecutionPolicy', 'pnputil',
                     'bcdedit', 'Start-Process', 'Set-MpPreference',
                     'New-Service', 'schtasks', 'Invoke-WebRequest',
                     'Set-ItemProperty', 'reg.exe', '& $')
        for term in forbidden:
            self.assertNotIn(term.lower(), code.lower())
        self.assertNotIn('-executionpolicy', self.wrapper.lower())
        self.assertRegex(self.wrapper, r'(?i)powershell(?:\.exe)? -NoProfile -File')
        self.assertIn('-Install -ConfirmInstall', self.wrapper)

    def test_truthful_documentation(self):
        for term in ('diagnostic', 'unsigned', 'not been validated',
                     'LocalAppData', 'does not install a camera driver'):
            self.assertIn(term, self.readme)


if __name__ == '__main__':
    unittest.main()
