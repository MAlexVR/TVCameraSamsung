# Copy the Windows camera utilities per user

This installer copies the unsigned camera utilities for the current user. The
executable includes diagnostics and firmware upload support, but **this step
does not install a camera driver**. The separate `driver/INSTALL.cmd` requires
an administrator and a valid signed catalog; see `driver/README.md`.

Native driver installation and hardware operation have **not been validated**.
Native Rust tests and installer checks are not proof of camera compatibility.

## Install

1. Extract the complete Windows release ZIP into an ordinary local directory.
2. Open `installer\INSTALL.cmd` without administrator privileges.
3. Read the notice and type `INSTALL` only if you accept the per-user copy.

Files are installed under `%LocalAppData%\SamsungCamera\1.0.0`. An existing
installation is never overwritten. The installer checks the EXE SHA256 against
`PACKAGE_STATUS.json`; this detects inconsistency, **not publisher authenticity**,
because the utility executable and manifest are unsigned.

Check without writing anything:

```bat
powershell.exe -NoProfile -File installer\install.ps1 -Check
```

Explicit command-line install:

```bat
powershell.exe -NoProfile -File installer\install.ps1 -Install
```

No registry, PATH, services, startup tasks, driver bindings, certificates or
security settings are changed. No downloads or camera access occur. If PowerShell
policy, SmartScreen or antivirus blocks execution, preserve the warning and stop;
do not change execution policy or disable protections.

## Run

Open `%LocalAppData%\SamsungCamera\1.0.0` and run `RUN-DIAGNOSTICS.cmd` for a
read-only report. After separately installing the signed WinUSB boot driver,
initialize a cold camera with:

```bat
samsung-camera-windows.exe load C:\path\to\FalconFW.bin
```

The pinned vendor firmware is not included. Firmware upload writes to device RAM;
do not run it against an already initialized camera. Video/audio operation needs
separate verification.

The installer stages new files and publishes the directory only after checking
the copied EXE. A failure attempts to remove only its staging files, without
recursive deletion. An empty parent or incomplete staging folder may remain after
interruption. No automatic uninstall or upgrade is provided.
