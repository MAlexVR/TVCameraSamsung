# Install the Windows diagnostic

This installer copies the **unsigned read-only diagnostic**, version 1.0.0,
for the current user. It **does not install a camera driver**, initialize
firmware or enable camera capture. Native installer and hardware execution have
**not been validated** on Windows; Linux structural checks are not a replacement.

## Install

1. Extract the complete Windows release ZIP into an ordinary local directory.
2. Open `installer\INSTALL.cmd`, without administrator privileges.
3. Read the notice and type `INSTALL` only if you accept the diagnostic install.

Files are installed under `%LocalAppData%\SamsungCamera\1.0.0`. An existing
installation is never overwritten. The installer checks the EXE SHA256 against
`PACKAGE_STATUS.json`; this detects inconsistency, **not publisher authenticity**,
because the package and manifest are unsigned.

To check the package without writing anything, open a terminal in the extracted
release directory:

```bat
powershell.exe -NoProfile -File installer\install.ps1 -Check
```

For an explicit command-line install without the interactive wrapper:

```bat
powershell.exe -NoProfile -File installer\install.ps1 -Install
```

No registry, PATH, services, startup tasks, driver bindings, certificates or
security settings are changed. No downloads or camera access occur. If PowerShell
policy, SmartScreen or antivirus blocks execution, preserve the warning and stop;
do **not** change execution policy or disable protections. You can report the
warning to the maintainer. The installed files are an EXE, its diagnostic runner,
package status and this README; the original extracted source package is unchanged.

## Run separately

Open `%LocalAppData%\SamsungCamera\1.0.0` and run `RUN-DIAGNOSTICS.cmd` manually.
It creates `samsung-camera-windows-report-*.txt` there. Share that report, not
private camera images or microphone recordings. A failed probe is useful evidence;
it is not an instruction to replace the camera's Windows driver.

The installer stages new files and publishes the directory only after checking
the copied EXE. A failure attempts to remove only its staging files, without
recursive deletion. An empty `SamsungCamera` parent or an incomplete staging
folder may remain after interruption. Do not delete another installation to
force a retry. No automatic uninstall or upgrade is provided in this version.
