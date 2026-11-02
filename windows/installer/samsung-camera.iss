; SPDX-License-Identifier: GPL-3.0-only
; Per-user installer for the unsigned Samsung VG-STC3000 camera utilities.
; Copies files and creates shortcuts only: no driver, registry policy, service,
; certificate or camera access. Build: python tools/build_windows_installer.py
#define AppVersion "1.0.0"

[Setup]
AppId={{6F0B2C54-6C1E-4B1B-9A57-3E0D5C1A7F10}
AppName=Samsung VG-STC3000 Camera Utilities
AppVersion={#AppVersion}
AppPublisher=TVCameraSamsung contributors
DefaultDirName={localappdata}\SamsungCamera\{#AppVersion}
DefaultGroupName=Samsung Camera
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..\dist
OutputBaseFilename=samsung-camera-{#AppVersion}-windows-setup
Compression=lzma2
SolidCompression=yes
UninstallDisplayName=Samsung VG-STC3000 Camera Utilities
WizardStyle=modern

[Files]
Source: "..\samsung-camera-windows.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\PACKAGE_STATUS.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\RUN-DIAGNOSTICS.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\TEST-CAMERA.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\test-camera.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\autoload.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\autoload-setup.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\usb-suspend.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\AUTOLOAD-INSTALL.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\AUTOLOAD-REMOVE.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\driver\CONFIGURE-INBOX.cmd"; DestDir: "{app}\driver"; Flags: ignoreversion
Source: "..\driver\configure-inbox.ps1"; DestDir: "{app}\driver"; Flags: ignoreversion
Source: "..\driver\README.md"; DestDir: "{app}\driver"; Flags: ignoreversion
Source: "..\driver\samsung-vg-stc3000.inf"; DestDir: "{app}\driver"; Flags: ignoreversion
Source: "..\driver\local-driver.ps1"; DestDir: "{app}\driver"; Flags: ignoreversion
Source: "..\driver\LOCAL-DRIVER.cmd"; DestDir: "{app}\driver"; Flags: ignoreversion
Source: "..\driver\LOCAL-DRIVER-REMOVE.cmd"; DestDir: "{app}\driver"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Test camera"; Filename: "{app}\TEST-CAMERA.cmd"; WorkingDir: "{app}"
Name: "{group}\Camera diagnostics (read-only)"; Filename: "{app}\RUN-DIAGNOSTICS.cmd"; WorkingDir: "{app}"
Name: "{group}\Configure WinUSB loader (asks for administrator)"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -Command ""Start-Process -Verb RunAs -FilePath cmd.exe -ArgumentList '/c','""""{app}\driver\CONFIGURE-INBOX.cmd""""'"""; WorkingDir: "{app}\driver"
Name: "{group}\Install local WinUSB driver (administrator)"; Filename: "{app}\driver\LOCAL-DRIVER.cmd"; WorkingDir: "{app}\driver"
Name: "{group}\Remove local WinUSB driver"; Filename: "{app}\driver\LOCAL-DRIVER-REMOVE.cmd"; WorkingDir: "{app}\driver"
Name: "{group}\Enable automatic firmware loading"; Filename: "{app}\AUTOLOAD-INSTALL.cmd"; WorkingDir: "{app}"
Name: "{group}\Disable automatic firmware loading"; Filename: "{app}\AUTOLOAD-REMOVE.cmd"; WorkingDir: "{app}"
Name: "{group}\Driver instructions"; Filename: "{app}\driver\README.md"
Name: "{group}\Uninstall"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\driver\README.md"; Description: "Read the WinUSB driver steps"; Flags: postinstall shellexec skipifsilent unchecked
