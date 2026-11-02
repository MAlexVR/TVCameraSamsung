# Opt-in Linux automatic initialization

The supplied udev rule asks systemd to run the existing guarded Rust loader when
an uninitialized VG-STC3000 (`04e8:205c`) is connected. It does **not** install
itself. Installation and automatic USB-add initialization were verified on the
owner's host on 2026-11-02: the service logged21335uploaded blocks/RAMboot after
reconnect, and the device changed from boot `04e8:205c` to runtime `04e8:205d`.
Do not rerun the installation on that working host. The instructions below are
setup reference, not a request to reinstall. Startup with an already-connected
camera and suspend/resume were not separately tested. LED/image behavior is
owner-accepted and out of scope; Windows support remains separate.

## Before installing

Close camera/microphone applications. Use the known working firmware only.
These instructions target a systemd/udev Linux host with polkit and libusb.
They install four root-owned files under `/usr/local` and `/etc`; subsequent
matching camera connections upload firmware to RAM automatically as root.
No USB mode/group changes, background capture or LED commands are installed.

From the project directory, as your ordinary user:

```sh
cargo build --offline --release &&
./target/release/samsung-camera inspect "$PWD/FalconFW.bin" &&
python3 -B tests/autoload.py
```

The Python checks require Python 3, `udevadm`, `systemd-escape` and
`systemd-analyze`. A sandbox may skip native systemd verification when manager
socket setup is denied; a skip is NOT a successful native validation.

Check for existing files before proceeding. Do not overwrite another install,
symlink or local configuration; these commands intentionally stop on a conflict:

```sh
(
  for path in \
    /usr/local/libexec/samsung-camera \
    /usr/local/share/samsung-camera/FalconFW.bin \
    /etc/systemd/system/samsung-camera-load@.service \
    /etc/udev/rules.d/70-samsung-vg-stc3000.rules
  do
    if [ -e "$path" ] || [ -L "$path" ]; then
      printf 'Existing path: %s; stop and inspect it first.\n' "$path" >&2
      exit 1
    fi
  done
)
```

Only continue if the check succeeds. The parent directories must be owned by
root and not writable by ordinary users. Do not execute a privileged service
from this user-writable checkout or substitute workspace paths in the unit.

## Check for another initializer

Before installation, inspect existing USB rules for another Samsung firmware
loader. Do not install a second initializer over an existing mechanism: two
loaders could address the same boot device concurrently. This is a bounded
reference check, not proof that no other userspace automation exists.

```sh
for directory in /etc/udev/rules.d /usr/lib/udev/rules.d \
  /usr/local/lib/udev/rules.d /run/udev/rules.d
 do
  if [ -d "$directory" ]; then
    grep -rHnEi --include='*.rules' \
      '205c|FalconFW|samsung.*(tvcam|fwload)' "$directory"
  fi
 done
```

No matching output is normal. Review any matches or read errors before proceeding;
do not disable or overwrite an existing rule automatically.

## Install explicitly with polkit

Run this block only after reviewing the four destination paths. Each `pkexec`
uses the normal system authentication dialog; no credential file is needed.
The rule is installed last, after checking the installed unit. A failure can
leave earlier files in place; installation is not transactional. Stop on errors.

```sh
pkexec /usr/bin/install -D -o root -g root -m 0755 \
  "$PWD/target/release/samsung-camera" /usr/local/libexec/samsung-camera &&
pkexec /usr/bin/install -D -o root -g root -m 0644 \
  "$PWD/FalconFW.bin" /usr/local/share/samsung-camera/FalconFW.bin &&
pkexec /usr/bin/install -D -o root -g root -m 0644 \
  "$PWD/packaging/systemd/samsung-camera-load@.service" \
  /etc/systemd/system/samsung-camera-load@.service &&
/usr/local/libexec/samsung-camera inspect /usr/local/share/samsung-camera/FalconFW.bin &&
systemd-analyze verify --man=no --instance=001-011 /etc/systemd/system/samsung-camera-load@.service &&
pkexec /usr/bin/install -D -o root -g root -m 0644 \
  "$PWD/packaging/udev/70-samsung-vg-stc3000.rules" \
  /etc/udev/rules.d/70-samsung-vg-stc3000.rules &&
pkexec /usr/bin/systemctl daemon-reload &&
pkexec /usr/bin/udevadm control --reload-rules
```

No `systemctl enable` or broad `udevadm trigger` is needed. Reloading rules does
not initialize an already-active device. Close applications, then unplug/replug
only this camera. Wait for initialization before opening preview. Do not run the
manual loader at the same time as automatic initialization.

## Verify the actual event

```sh
journalctl -b -u 'samsung-camera-load@*.service' --no-pager -n 60
lsusb -d 04e8:205d
```

If journal access is denied, request access through the normal system policy;
do not broaden device permissions. Verify that the event log reports one upload
and that the camera becomes initialized `04e8:205d` without a manual load command.
The owner has already accepted image/Snapshot and LED behavior: do not change or
retest those components here. This step verifies automatic initialization only;
it does not claim new video, audio or LED evidence. Startup with the camera
attached and suspend/resume are separate initialization checks, not completed
by the single reconnect test. Do not run usbmon as part of this procedure.

If SELinux denies the installed executable or USB access, preserve the denial
and diagnose its label/policy. Do not disable SELinux or add broad allow rules.
On errors/timeouts, do not repeatedly restart the service: a partial RAM upload
requires a clean unplug/replug, with applications closed.

## Disable or remove

First prevent future USB-add activation by renaming only this rule, then reload:

```sh
pkexec /usr/bin/mv /etc/udev/rules.d/70-samsung-vg-stc3000.rules \
  /etc/udev/rules.d/70-samsung-vg-stc3000.rules.disabled &&
pkexec /usr/bin/udevadm control --reload-rules
```

This does not stop an upload already in progress. Wait for any active
`samsung-camera-load@...` job to finish before unplugging or removing files;
do not kill the loader mid-transfer. The original manual loader stays available.
For full removal, after confirming these are the files you installed and no
upload is active, remove the disabled rule, the service template, and the two
`/usr/local` files listed above, then run `systemctl daemon-reload` through
polkit. Do not remove shared parent directories or use wildcard removal.

## Design and limits

- The rule matches USB-device **add**, exact vendor/product, and numeric bus/device
  properties. It does not match interfaces or initialized `04e8:205d`.
- Instance `001-011` becomes `001/011` via systemd `%I`. `load` independently
  validates the address, buffered firmware SHA-256 and opened USB boot identity.
- The oneshot has no retries, no persistent active state and a five-minute cap.
  Its device allowlist is restricted to the selected USB node. Root capabilities
  are dropped; the filesystem is read-only and home directories inaccessible.
- No `BindsTo` ties the job to the boot node: successful RAM boot intentionally
  removes that node. Keeping a job active after success would also obstruct
  later connections that reuse the same address.
- Root-owned installation is a trust boundary, not a vendor authenticity
  guarantee. The firmware pin and identity checks remain the existing loader's.

References: [systemd device activation](https://www.freedesktop.org/software/systemd/man/latest/systemd.device.html),
[instance escaping](https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html),
[udev substitutions](https://www.freedesktop.org/software/systemd/man/latest/udev.html).
