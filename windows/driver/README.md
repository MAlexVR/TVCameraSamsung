# Samsung VG-STC3000 Windows boot driver

The cold firmware device `04e8:205c` needs Microsoft's WinUSB so the user-mode
loader can send the AIT firmware sequence. Initialized `04e8:205d` must remain
on Microsoft's UVC camera/audio drivers. Firmware resides in RAM and must be
loaded again after a cold device boot.

There are two supported installation routes. Do not disable Secure Boot, driver
signature enforcement, antivirus or other Windows protections for either route.

## Option A: use the signed WinUSB driver included in Windows

This route does not need the project INF, a custom catalog or a signing
certificate. It requires a manual driver selection and an administrative script
that registers only the loader's device-interface GUID.

### 1. Select the inbox driver

1. Close camera/audio applications and connect the cold camera.
2. Open Device Manager (`devmgmt.msc`).
3. Locate the Samsung **parent USB device**, normally shown as a USB Composite
   Device or an unknown device. In **Properties > Details > Hardware IDs**, it
   must include exactly `USB\VID_04E8&PID_205C`.
4. Do not select an interface whose ID contains `&MI_...`, anything identified as
   `04e8:205d`, or an interface bound to `usbvideo`.
5. Choose **Update driver > Browse my computer for drivers > Let me pick from a
   list of available drivers > Universal Serial Bus devices > WinUsb Device**.
6. Continue only after checking the hardware ID again. The boot device's UVC
   interfaces may disappear while its parent is bound to WinUSB; this is expected
   before firmware initialization.

Check the binding without changing the system:

```bat
powershell.exe -NoProfile -File driver\configure-inbox.ps1 -Check
```

The script rejects zero or multiple matching WinUSB devices and refuses to
continue if runtime `04e8:205d` is bound to WinUSB.

### 2. Register the loader interface

Right-click `driver\CONFIGURE-INBOX.cmd`, choose **Run as administrator**, read
the notice and type `CONFIGURE`. The script adds
`{889992a2-9243-4ba5-beb7-7d0b029e11a0}` to the selected boot device's
`Device Parameters\DeviceInterfaceGUIDs` multi-string. Existing GUIDs are
preserved and the service is rechecked immediately before writing.

Disconnect and reconnect the cold camera, then repeat `-Check`. It should report
that the inbox WinUSB binding and loader interface are both present.

To roll back before initializing, use Device Manager to select **USB Composite
Device** (`usbccgp`) for the same parent. Alternatively, uninstall only that
device instance without deleting Microsoft driver files, then reconnect it. Do
not remove or replace Windows' WinUSB/UVC driver files.

## Option B: distribute a signed custom INF package

`samsung-vg-stc3000.inf` matches only `USB\VID_04E8&PID_205C` and references the
inbox WinUSB sections. A complete package also requires
`samsung-vg-stc3000.cat` with a valid trusted signature. Microsoft requires this
catalog even though no custom kernel driver is included.

A maintainer must install the Windows Driver Kit, create the catalog and sign it
through the normal Microsoft/organizational driver-signing process. No catalog,
certificate, signing key, test certificate or signing bypass is included.

```bat
powershell.exe -NoProfile -File driver\install.ps1 -Check
```

After a valid signature is reported, right-click `driver\INSTALL.cmd`, choose
**Run as administrator** and type `INSTALL DRIVER`. This route runs only
`pnputil.exe /add-driver ... /install`. Do not run it for option A.

## Option C: local-only driver (this computer)

Use this when Device Manager does not offer **WinUsb Device** for the composite
device. Run `LOCAL-DRIVER.cmd` (UAC prompt) and type `INSTALL LOCAL`. It signs the
project INF catalog with a certificate generated on this computer, trusts it
machine-wide and runs `pnputil /add-driver`. It needs the Windows SDK
(`makecat.exe`, `signtool.exe`). Secure Boot and signature enforcement stay on and
`04e8:205d` is never matched. Undo everything with `LOCAL-DRIVER-REMOVE.cmd`, which
deletes the driver package and removes the certificate and its private key.
Reconnect the camera after either command.

## Initialize the camera

Supply your own lawfully obtained pinned `FalconFW.bin`; it is not included.
From the directory containing the built executable:

```bat
samsung-camera-windows.exe inspect C:\path\to\FalconFW.bin
samsung-camera-windows.exe load C:\path\to\FalconFW.bin
```

`inspect` performs no USB access. `load` validates the exact 682712-byte image,
its `AIT842SF` header and SHA-256
`df71a001ec9045d9115b482f76cffe5e30dfac5fe26e7ef8a2bd25854af20b84`,
then refuses an already initialized camera or an ambiguous/multiple boot device.
It validates the USB descriptor again through WinUSB and sends the tested AIT
sequence with device-recipient request type `0x20`, request `1`, index `0x0600`.
Any error or short transfer stops the sequence without retry or a later boot
command. Success requires runtime `04e8:205d` to appear within 20 seconds; video
and audio still need separate application-level verification.

Native tests and registry guards are verified, but the manual binding, firmware
upload and camera hardware operation on Windows have not yet been validated.
