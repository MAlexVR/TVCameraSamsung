# Windows VG-STC3000: implementation and verification status

The `windows/` crate now contains read-only diagnostics and a WinUSB RAM firmware
initializer. It reuses the Linux-tested AIT sequence and firmware identity,
length and SHA-256 guards. The Windows transport uses the same class/device
request type `0x20`; it is not substituted with UVC class/interface requests.

## WinUSB binding options

For local use, Microsoft's already signed inbox **WinUsb Device** can be selected
manually in Device Manager for the parent cold boot `USB\VID_04E8&PID_205C`.
`windows/driver/configure-inbox.ps1` then verifies the exact binding and registers
only the project-specific loader interface GUID. It preserves existing GUIDs,
rejects multiple boot devices and refuses any WinUSB binding on runtime
`04e8:205d`. Administrator confirmation and a physical reconnect are required.
The documented rollback restores the USB Composite Device binding.

For a distributable custom package,
`windows/driver/samsung-vg-stc3000.inf` selects the same inbox WinUSB only for
boot `04e8:205c`. **Its required signed catalog is not present.** The separate
administrative INF installer verifies a trusted Authenticode signature before
PnPUtil and refuses missing/untrusted packages.

No test-signing, certificate installation, Secure Boot or signature-enforcement
workaround is provided. Runtime `04e8:205d` must always remain on Microsoft's
UVC camera/audio drivers. See `windows/driver/README.md` and Microsoft's
[WinUSB installation documentation](https://learn.microsoft.com/windows-hardware/drivers/usbcon/winusb-installation).

## Implemented upload safeguards

- The exact 682712-byte `AIT842SF` image and pinned SHA-256 are checked before
  opening a USB device.
- An existing runtime camera, missing or multiple boot interfaces and non-WinUSB
  bindings are rejected.
- Hardware IDs/service are checked through SetupAPI and the USB device descriptor
  is checked again after `WinUsb_Initialize`.
- The known AIT start/chunks/boot sequence is sent synchronously through
  `WinUsb_ControlTransfer`; any error or short transfer stops without retry.
- Success requires runtime `04e8:205d` to appear within 20 seconds. No claim of
  video/audio capture is inferred from re-enumeration.

## Verification boundary

Native Windows compilation and portable tests can verify CLI behavior, SHA-256,
protocol and ABI layouts. They cannot prove signed-driver installation, actual
USB transfer acceptance, runtime re-enumeration or media capture. Those hardware
steps remain unverified. The unsigned utility EXE and signed driver catalog are
separate artifacts with separate trust requirements.

The original `inventory`/`probe` diagnostics remain read-only. The per-user
installer copies utilities only and never installs a driver. The driver installer
never uploads firmware. Build helpers perform neither operation.
