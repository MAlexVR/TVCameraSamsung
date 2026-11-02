# Install Linux automatic camera initialization

This installer adds the existing guarded VG-STC3000 initializer and opt-in
systemd/udev integration. **Do not reinstall on the already configured host.**
It does not change video, audio, LED behavior, device permissions or security policy.
Firmware is not included or downloaded: supply your own known-working,
SHA-256-pinned `FalconFW.bin`.

## Quick path

Use a trusted release extracted into a directory only you control. The archive
must contain `bin/samsung-camera`, this installer directory, and the unchanged
`packaging/systemd/` and `packaging/udev/` assets. Run as an ordinary user:

```sh
./packaging/linux/install.sh --help
./packaging/linux/install.sh --check /absolute/path/to/FalconFW.bin
# Only after reviewing the paths and automatic-upload behavior below:
./packaging/linux/install.sh --install /absolute/path/to/FalconFW.bin
```

`--help` requires only `/bin/sh`. Check/install require `/usr/bin/python3`,
polkit (`pkexec`), systemd tools, udev tools and the packaged executable's native
runtime dependencies, including libusb. Nothing installs missing dependencies.
The executable's existing `inspect` command validates firmware before installation.
`--check` neither installs files nor runs camera commands; native systemd unit
verification is deferred until the executable and unit occupy their real paths.

## Privileged effects

`--install` is explicit consent to these four root-owned files:

| Destination | Purpose |
| --- | --- |
| `/usr/local/libexec/samsung-camera` | Existing initializer executable |
| `/usr/local/share/samsung-camera/FalconFW.bin` | Your pinned firmware copy |
| `/etc/systemd/system/samsung-camera-load@.service` | Existing restricted oneshot unit |
| `/etc/udev/rules.d/70-samsung-vg-stc3000.rules` | Existing boot-device-only activation rule |

Each copy invokes the shipped `install.py` with `pkexec /usr/bin/python3 -I -B`.
Review this helper as privileged release code. It accepts only fixed destination
roles, checks root-owned/non-writable ancestors, rejects symlinks, and creates
files exclusively rather than overwriting an existing destination. It checks
firmware bytes again before its privileged copy. New parent directories can be
created. Do not modify or replace the release while installing it.

Before any privileged copy, installation refuses destination conflicts, unsafe
parents, missing tools and matching potential competing rules in the standard
udev rule directories. That bounded rule scan cannot detect every possible
external initializer; do not run another loader concurrently.

The rule is copied **only after** the installed executable's firmware inspection
and `systemd-analyze verify` both succeed. Finally the installer reloads systemd
and udev rules. It does not enable a service, start a loader, trigger udev, capture
media, or unplug a camera. Once installed, future matching USB-add events can
upload firmware to camera RAM automatically as root.

## Verify and handle failures

Close camera applications before a deliberate unplug/replug of only this camera.
Check the actual initialization event using the existing instructions in
[`docs/AUTOLOAD.md`](../../docs/AUTOLOAD.md). A reload alone does not initialize
an already-connected camera. Linux reconnect evidence does not establish Windows
support, startup-with-camera-attached, or suspend/resume behavior.

Installation is **not transactional**. An error can leave earlier files or parent
directories, including a partial newly created file. Rule reload can fail after
the rule was copied. Stop and inspect the exact destinations before retrying;
never delete another installation or force an overwrite. There is no automatic
rollback/uninstall. Follow the existing autoload removal guidance only after
identifying your files and ensuring no upload is active. Do not disable SELinux,
polkit or other security protections to bypass an error.
