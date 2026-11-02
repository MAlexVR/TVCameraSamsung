"""Offline distribution integrity checks; never access hardware or GitHub."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class ReleaseTests(unittest.TestCase):
    def test_explicit_public_source_allowlist(self):
        spec = importlib.util.spec_from_file_location('package_release', ROOT / 'tools/package_release.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.assertIn('LICENSE', module.SOURCE_FILES)
        self.assertIn('packaging/linux/install.sh', module.SOURCE_FILES)
        self.assertIn('windows/installer/install.ps1', module.SOURCE_FILES)
        self.assertIn('windows/driver/samsung-vg-stc3000.inf', module.SOURCE_FILES)
        self.assertIn('windows/driver/install.ps1', module.SOURCE_FILES)
        self.assertIn('windows/driver/configure-inbox.ps1', module.SOURCE_FILES)
        self.assertIn('windows/driver/CONFIGURE-INBOX.cmd', module.SOURCE_FILES)
        self.assertIn('windows/src/native/winusb.rs', module.SOURCE_FILES)
        self.assertIn('windows/test-camera.ps1', module.SOURCE_FILES)
        self.assertIn('windows/TEST-CAMERA.cmd', module.SOURCE_FILES)
        self.assertNotIn('FalconFW.bin', module.SOURCE_FILES)
        self.assertFalse(any(name.endswith(('.cat', '.cer', '.pfx', '.p12'))
                             for name in module.SOURCE_FILES))
        for name in module.SOURCE_FILES:
            self.assertFalse(any(x in name.lower() for x in ('secure', 'credential', 'blucast', 'usb-control.', 'odd/')))
            self.assertFalse(Path(name).is_absolute())
        self.assertEqual(len(module.SOURCE_FILES), len(set(module.SOURCE_FILES)))

if __name__ == '__main__':
    unittest.main()
