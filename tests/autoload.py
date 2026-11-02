"""Offline checks for opt-in autoload assets; never trigger udev or start services."""
import configparser
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
RULE = ROOT / "packaging/udev/70-samsung-vg-stc3000.rules"
UNIT = ROOT / "packaging/systemd/samsung-camera-load@.service"
BINARY = "/usr/local/libexec/samsung-camera"
FIRMWARE = "/usr/local/share/samsung-camera/FalconFW.bin"


class AutoloadTests(unittest.TestCase):
    def test_rule_is_boot_device_add_only_and_never_changes_permissions(self):
        lines = [line for line in RULE.read_text().splitlines()
                 if line and not line.startswith("#")]
        self.assertEqual(len(lines), 1)
        expected = [
            'ACTION=="add"', 'SUBSYSTEM=="usb"', 'ENV{DEVTYPE}=="usb_device"',
            'ATTR{idVendor}=="04e8"', 'ATTR{idProduct}=="205c"',
            'ENV{BUSNUM}=="[0-9][0-9][0-9]"',
            'ENV{DEVNUM}=="[0-9][0-9][0-9]"', 'TAG+="systemd"',
            'ENV{SYSTEMD_WANTS}+="samsung-camera-load@$env{BUSNUM}-$env{DEVNUM}.service"',
        ]
        self.assertEqual(lines[0].split(", "), expected)

    def test_service_reuses_guarded_loader_without_shell_or_retries(self):
        unit = configparser.ConfigParser(interpolation=None)
        unit.optionxform = str
        unit.read_string(UNIT.read_text())
        service = unit["Service"]
        self.assertEqual(service["ExecStart"],
                         f"{BINARY} load {FIRMWARE} /dev/bus/usb/%I")
        self.assertEqual(service["Type"], "oneshot")
        self.assertEqual(service["Restart"], "no")
        self.assertEqual(service["TimeoutStartSec"], "5min")
        self.assertNotIn("RemainAfterExit", service)
        self.assertNotIn("Install", unit)
        self.assertNotIn("BindsTo", unit["Unit"])
        self.assertNotIn("ExecStartPre", service)
        self.assertNotIn("ExecStop", service)

    def test_service_limits_privilege_and_device_access(self):
        unit = configparser.ConfigParser(interpolation=None)
        unit.optionxform = str
        unit.read_string(UNIT.read_text())
        service = unit["Service"]
        self.assertEqual(service["User"], "root")
        self.assertEqual(service["ProtectSystem"], "strict")
        self.assertEqual(service["ProtectHome"], "yes")
        self.assertEqual(service["NoNewPrivileges"], "yes")
        self.assertEqual(service["CapabilityBoundingSet"], "")
        self.assertEqual(service["DevicePolicy"], "closed")
        self.assertEqual(service["DeviceAllow"], "/dev/bus/usb/%I rw")
        self.assertNotEqual(service.get("PrivateDevices"), "yes")

    def test_bus_address_unescapes_to_loader_path(self):
        for bus, device in [("001", "011"), ("002", "127"), ("123", "001")]:
            result = subprocess.run(
                ["systemd-escape", "--unescape", f"{bus}-{device}"],
                check=True, capture_output=True, text=True)
            self.assertEqual(result.stdout.strip(), f"{bus}/{device}")

    def test_native_udev_syntax(self):
        subprocess.run(["udevadm", "verify", "--no-summary", str(RULE)],
                       check=True, capture_output=True, text=True)

    def test_native_systemd_syntax_in_isolated_root(self):
        # Verification parses but NEVER executes the placeholder or the service.
        with tempfile.TemporaryDirectory(prefix="samsung-autoload-") as directory:
            root = Path(directory)
            units = root / "etc/systemd/system"
            units.mkdir(parents=True)
            shutil.copyfile(UNIT, units / UNIT.name)
            executable = root / BINARY.lstrip("/")
            executable.parent.mkdir(parents=True)
            executable.write_text("#!/bin/sh\nexit 99\n")
            executable.chmod(0o755)
            for target in ["sysinit.target", "basic.target", "shutdown.target"]:
                (units / target).write_text("[Unit]\nDefaultDependencies=no\n")
            result = subprocess.run([
                "systemd-analyze", "verify", f"--root={root}", "--man=no",
                "--instance=001-011", "/etc/systemd/system/samsung-camera-load@.service",
            ], capture_output=True, text=True)
            # Some constrained runners deny manager socket setup before parsing.
            blocked = {
                "Failed to turn off SO_PASSRIGHTS on user lookup socket, ignoring: Operation not permitted",
                "Failed to enable SO_PASSCRED on handoff timestamp socket: Operation not permitted",
            }
            errors = set(result.stderr.strip().splitlines())
            if result.returncode and errors and errors <= blocked:
                self.skipTest("Sandbox denies systemd manager sockets; run native verify on host")
            self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
