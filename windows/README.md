# Samsung VG-STC3000 Windows camera utilities 1.0.0

This independent Rust project provides read-only diagnostics and a guarded
WinUSB RAM firmware initializer for the Samsung VG-STC3000. **Native compilation
and portable tests do not establish hardware compatibility.** Signed-driver
installation, firmware upload and video/audio operation still require testing
with the physical camera.

## Components

- `inventory`: present Samsung `04e8:205c`/`205d` PnP bindings and status.
- `probe`: read-only Microsoft UVC metadata queries; never sends UVC SET.
- `inspect FIRMWARE`: validates the pinned firmware without USB access.
- `load FIRMWARE`: uploads to cold boot `04e8:205c` through WinUSB, sends the RAM
  boot command and waits for runtime `04e8:205d` to appear.
- `driver/`: INF and administrative installer for Microsoft's inbox WinUSB.

The crate uses the parent project's tested AIT upload sequence. It has no
third-party Cargo dependencies. No firmware, kernel driver, certificate or
signing key is redistributed.

## WinUSB binding options

The fastest local-test route uses Microsoft's already signed inbox WinUSB. In
Device Manager, manually select **WinUsb Device** only for the parent whose
hardware ID is exactly `USB\VID_04E8&PID_205C`, then run
`driver\CONFIGURE-INBOX.cmd` as administrator. The configuration script checks
the binding and registers the loader interface GUID; it preserves existing GUIDs
and rejects runtime `04e8:205d`. Follow the exact selection and rollback steps
in `driver/README.md`.

```bat
powershell.exe -NoProfile -File driver\configure-inbox.ps1 -Check
```

For a distributable custom-INF package,
`driver/samsung-vg-stc3000.inf` also requires a trusted signed
`samsung-vg-stc3000.cat`. The catalog is currently absent. `driver\INSTALL.cmd`
is only for that signed-package route and refuses missing/untrusted catalogs
before calling PnPUtil.

Neither route may match runtime `04e8:205d`, whose camera/audio functions remain
on Microsoft's UVC drivers. Never enable test signing, install a test root
certificate or disable Secure Boot/signature enforcement. No driver command is
run by `BUILD.cmd`, the diagnostic runner or the per-user utility installer.

## Quick test on Windows 11

Extract the Windows ZIP to a normal local folder (not the per-user copy; the
guide needs `driver\`) and run `TEST-CAMERA.cmd`. It verifies the EXE hash, runs
the read-only inventory and reports the next step: select inbox WinUSB (see
`driver/README.md`), run `driver\CONFIGURE-INBOX.cmd` as administrator, then:

```bat
TEST-CAMERA.cmd -Firmware C:\path	o\FalconFW.bin -Load
```

`-Load` asks you to type `LOAD` before sending firmware to RAM. If the camera is
already initialized (`04e8:205d`) the guide stops and tells you to open the
Windows Camera app. Build the ZIP with `python tools/package_release.py
--windows-only` (needs the built EXE; output in `dist/`).

## Setup installer

`python tools/build_windows_installer.py` compiles `installer/samsung-camera.iss`
with Inno Setup 6 into `dist/samsung-camera-1.0.0-windows-setup.exe`. It installs
per user under `%LocalAppData%\SamsungCamera.0.0` without administrator rights,
adds Start Menu shortcuts (test, diagnostics, WinUSB configuration via UAC) and an
uninstaller. It installs no driver. The setup EXE is unsigned, so SmartScreen may warn.

## Keeping the camera initialized

Firmware lives in the camera's RAM, so it must be loaded after every cold boot,
and Windows' USB selective suspend makes the camera reset a few minutes after it
goes idle (observed: 3 to 4 minutes with no open stream, none in 8 minutes with
one open and none in 6 minutes of idle with suspend disabled).

- `AUTOLOAD-INSTALL.cmd` starts a per-user watcher at sign-in (a Startup-folder
  shortcut, no admin, service or scheduled task). It loads the pinned firmware
  whenever the cold camera, bound to WinUSB, appears, and it also turns USB
  selective suspend off in the active power plan. `AUTOLOAD-REMOVE.cmd` stops it
  and restores the saved power values. The log is
  `%LocalAppData%\SamsungCamera\autoload.log`.
- `usb-suspend.ps1 -Check | -Disable | -Restore` does only the power-plan part.

## Hardware verification

Verified once on Windows 11 with a VG-STC3000 on a direct USB port: local WinUSB
binding, firmware upload (21335 blocks), re-enumeration as `04e8:205d`, MJPEG
1280x720 at 30 fps (150 distinct frames in 5 s) and per-user installer
install/uninstall. A tearing artifact seen in the Windows Camera app did not
appear in raw MJPEG frames read directly. Not verified: other machines, USB 2
hubs (the camera was unstable behind one), YUYV modes above 640x480, which can
make the camera drop off the bus.

## Build on Windows

Install Rust with its matching Windows linker/SDK, then run in this directory:

```bat
BUILD.cmd
```

The helper runs offline locked tests and a release build, producing
`samsung-camera-windows.exe`. It does not install Rust, drivers, certificates or
system components. The resulting EXE is unsigned; that is separate from the
mandatory signed driver catalog.

Optional Linux cross-build prerequisites are Rust 1.88's
`x86_64-pc-windows-gnu` target and MinGW. From the parent project:

```sh
bash windows/build-linux.sh
```

## Initialize the camera

Supply your own lawfully obtained `FalconFW.bin`. Only the known 682712-byte
`AIT842SF` image with SHA-256
`df71a001ec9045d9115b482f76cffe5e30dfac5fe26e7ef8a2bd25854af20b84`
is accepted. Close camera/audio applications and connect the cold camera.

```bat
samsung-camera-windows.exe inspect C:\path\to\FalconFW.bin
samsung-camera-windows.exe load C:\path\to\FalconFW.bin
```

`load` refuses runtime `04e8:205d`, missing/ambiguous WinUSB interfaces and any
USB descriptor other than boot `04e8:205c`. It uses class/device request `0x20`,
request `1`, index `0x0600`, the tested 32-byte AIT chunks and final RAM boot
command. Errors and short transfers stop immediately without retry. A reported
success means runtime re-enumeration was detected, not that video/audio capture
was tested. Firmware resides in RAM and must be loaded again after a cold boot.

## Diagnostics and optional per-user copy

```bat
RUN-DIAGNOSTICS.cmd
```

This creates a new `samsung-camera-windows-report-*.txt` without overwriting an
existing report. Reports omit device paths, serials, user/computer names and
media. `inventory` and `probe` do not upload firmware or install a driver.

`installer/INSTALL.cmd` copies the utilities for the current user under
`%LocalAppData%\SamsungCamera\1.0.0`; it does not install a driver. See
`installer/README.md`. Existing installations are never overwritten and
PowerShell execution policy is respected.

## Development checks

```sh
cargo test --offline --locked --manifest-path windows/Cargo.toml
cargo check --offline --locked --manifest-path windows/Cargo.toml --features check-windows-ffi
```

The FFI feature permits type-checking declarations on another OS; it does not
run Win32 APIs or verify hardware. Native Windows tests cover the CLI, firmware
hash/guards and ABI layouts, but cannot simulate a successful camera upload.

## References and license

- [Microsoft WinUSB installation and signed catalog requirement](https://learn.microsoft.com/windows-hardware/drivers/usbcon/winusb-installation)
- [WinUsb_ControlTransfer](https://learn.microsoft.com/windows/win32/api/winusb/nf-winusb-winusb_controltransfer)
- [Microsoft UVC extension requirements](https://learn.microsoft.com/windows-hardware/drivers/stream/device-requirements-for-usb-video-class-extension-units)

New source: SPDX `GPL-3.0-only`. See the parent project's `LICENSE`.
