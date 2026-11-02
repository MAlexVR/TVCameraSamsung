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
        for term in ('ConvertFrom-Json', 'Get-Sha256', 'SHA256',
                     'firmware_loader_implemented', 'read_only',
                     "'1.0.0'", 'binary_sha256'):
            self.assertIn(term, self.script)
        self.assertLess(self.script.index('Get-Sha256'),
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


class WindowsDriverStructure(unittest.TestCase):
    def setUp(self):
        self.inf = (ROOT / 'windows/driver/samsung-vg-stc3000.inf').read_text()
        self.script = (ROOT / 'windows/driver/install.ps1').read_text()
        self.wrapper = (ROOT / 'windows/driver/INSTALL.cmd').read_text()
        self.readme = (ROOT / 'windows/driver/README.md').read_text()

    def test_inf_binds_only_cold_boot_device_to_inbox_winusb(self):
        self.assertIn('USB\\VID_04E8&PID_205C', self.inf)
        self.assertNotIn('USB\\VID_04E8&PID_205D', self.inf)
        self.assertIn('CatalogFile = samsung-vg-stc3000.cat', self.inf)
        self.assertIn('Include = winusb.inf', self.inf)
        self.assertIn('Needs   = WINUSB.NT', self.inf)
        self.assertIn('{889992a2-9243-4ba5-beb7-7d0b029e11a0}', self.inf)
        native = (ROOT / 'windows/src/native.rs').read_text()
        self.assertIn('a: 0x889992a2', native)

    def test_valid_catalog_and_admin_precede_driver_installation(self):
        for term in ('Get-AuthenticodeSignature', 'SignatureStatus]::Valid',
                     'SignerCertificate', 'WindowsBuiltInRole]::Administrator',
                     'Read-Host', 'INSTALL DRIVER', '/add-driver $inf /install'):
            self.assertIn(term, self.script)
        self.assertLess(self.script.index('Get-AuthenticodeSignature'),
                        self.script.index('/add-driver $inf /install'))
        self.assertLess(self.script.index('Read-Host'),
                        self.script.index('/add-driver $inf /install'))
        self.assertIn('-Install -ConfirmInstall', self.wrapper)
        self.assertNotIn('-ExecutionPolicy', self.wrapper)

    def test_no_signing_or_security_bypass(self):
        code = '\n'.join(line for line in self.script.splitlines()
                         if not line.strip().startswith('#'))
        for term in ('Set-ExecutionPolicy', 'bcdedit', 'testsigning',
                     'Set-MpPreference', 'New-SelfSignedCertificate',
                     'CertUtil', 'Invoke-WebRequest', 'Zadig'):
            self.assertNotIn(term.lower(), code.lower())
        self.assertIn('valid trusted signature', self.readme)
        self.assertIn('not yet been validated', self.readme)

    def test_inbox_configuration_checks_exact_binding_before_registry_write(self):
        script = (ROOT / 'windows/driver/configure-inbox.ps1').read_text()
        wrapper = (ROOT / 'windows/driver/CONFIGURE-INBOX.cmd').read_text()
        for term in ('VID_04E8&PID_205C', 'VID_04E8&PID_205D',
                     'Get-WinUsbInstances', 'DeviceInterfaceGUIDs',
                     'WindowsBuiltInRole]::Administrator', 'Read-Host',
                     'New-ItemProperty', 'Set-ItemProperty'):
            self.assertIn(term, script)
        self.assertLess(script.index('Read-Host'),
                        script.index('New-ItemProperty -LiteralPath'))
        self.assertIn('-Configure -ConfirmConfigure', wrapper)
        self.assertNotIn('-ExecutionPolicy', wrapper)

    def test_inbox_method_preserves_uvc_and_documents_rollback(self):
        script = (ROOT / 'windows/driver/configure-inbox.ps1').read_text()
        self.assertIn('Existing interface GUIDs are preserved', script)
        self.assertIn('Runtime 04e8:205d is not modified', script)
        self.assertIn('USB Composite', self.readme)
        self.assertIn('WinUsb Device', self.readme)
        for term in ('Set-ExecutionPolicy', 'bcdedit', 'testsigning',
                     'New-SelfSignedCertificate', 'Disable-PnpDevice'):
            self.assertNotIn(term.lower(), script.lower())


class WindowsLocalDriverStructure(unittest.TestCase):
    def test_local_trust_is_confirmed_scoped_and_removable(self):
        script = (ROOT / 'windows/driver/local-driver.ps1').read_text()
        for term in ('INSTALL LOCAL', '-Remove', 'Remove-LocalCerts', 'PID_205D',
                     'Test-Admin', 'USB\VID_04E8&PID_205C'):
            self.assertIn(term, script)
        self.assertLess(script.index("-cne 'INSTALL LOCAL'"), script.index('/add-driver'))
        self.assertLess(script.index("-cne 'INSTALL LOCAL'"), script.index('Import-Certificate'))
        code = script.lower()
        for term in ('bcdedit', 'testsigning', 'set-executionpolicy', 'set-mppreference',
                     'invoke-webrequest', 'secureboot'):
            self.assertNotIn(term, code)
        for name in ('LOCAL-DRIVER.cmd', 'LOCAL-DRIVER-REMOVE.cmd'):
            wrapper = (ROOT / 'windows/driver' / name).read_text().lower()
            self.assertIn('-verb runas', wrapper)
            self.assertNotIn('-executionpolicy', wrapper)


class WindowsAutoloadStructure(unittest.TestCase):
    def test_autoload_is_per_user_pinned_and_never_touches_runtime_camera(self):
        watcher = (ROOT / 'windows/autoload.ps1').read_text()
        setup = (ROOT / 'windows/autoload-setup.ps1').read_text()
        for term in ('inspect $Firmware', "'boot-ready'", 'runtime', 'Mutex', 'FalconFW.bin'):
            self.assertIn(term, watcher)
        self.assertLess(watcher.index("if ($state -eq 'boot-ready')"), watcher.index('& $exe load'))
        for text in (watcher, setup):
            code = text.lower()
            for term in ('pnputil', 'bcdedit', 'schtasks', 'register-scheduledtask',
                         'new-service', 'set-itemproperty', 'new-itemproperty',
                         'set-executionpolicy', 'invoke-webrequest'):
                self.assertNotIn(term, code)
        self.assertIn("GetFolderPath('Startup')", setup)
        suspend = (ROOT / 'windows/usb-suspend.ps1').read_text()
        for term in ('-Restore', 'usb-suspend-backup.json', 'SCHEME_CURRENT'):
            self.assertIn(term, suspend)
        self.assertIn('usb-suspend.ps1', setup)


class WindowsGuidedTest(unittest.TestCase):
    def test_guided_test_is_read_only_until_confirmed_load(self):
        script = (ROOT / 'windows/test-camera.ps1').read_text()
        wrapper = (ROOT / 'windows/TEST-CAMERA.cmd').read_text()
        for term in ('inventory', "'WINUSB'", 'inspect', 'Read-Host'):
            self.assertIn(term, script)
        self.assertLess(script.index('Read-Host'), script.index('& $exe load'))
        code = '\n'.join(l for l in script.splitlines() if not l.strip().startswith('#')).lower()
        for term in ('set-executionpolicy', 'pnputil', 'bcdedit', 'invoke-webrequest',
                     'new-itemproperty', 'set-itemproperty', 'start-process'):
            self.assertNotIn(term, code)
        self.assertNotIn('-executionpolicy', wrapper.lower())
        self.assertIn('-NoProfile -File', wrapper)


if __name__ == '__main__':
    unittest.main()
