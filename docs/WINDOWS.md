# Windows VG-STC3000: test-stage status

A read-only Rust diagnostic project is now available in [`windows/`](../windows/README.md).
**A working Windows firmware initializer, signed driver package and native
hardware success are still NOT delivered.** The first package intentionally
collects binding/control metadata before permitting any firmware writes.

The independent zero-dependency crate provides `inventory` and `probe`; `load`
refuses before opening a device. It preserves Microsoft's camera/audio drivers
and makes no signature-policy, Secure Boot or system configuration changes.
Build/run instructions and exact proof limits are in the package README.

## Current build boundary

The Windows x64 diagnostic EXE has now been cross-compiled and linked using
Rust1.88 and the installed MinGW toolchain. Windows header size/alignment/offset
and constant assertions passed; the PE imports only Windows system DLLs and has
no Authenticode signature. The binary test ZIP includes the EXE, source, run
helper and integrity manifest; a separate source-only ZIP is also available.
**Execution on Windows and actual boot/runtime camera behavior remain unverified.**
This build is not a functional firmware initializer or signed kernel driver.

## Evidence needed before implementing upload

- Identify boot `04e8:205c` and runtime `04e8:205d`, their hardware-ID association,
  driver service and problem status on Windows.
- Establish whether `usbvideo` exposes the relevant extension-unit properties.
  Windows may hide controls whose initialization queries fail. BASIC SUPPORT is
  not raw USB GET_INFO/GET_LEN.
- Validate the unit6 GUID on the boot device; its currently known value came
  from Linux runtime205d only.
- Validate an actual upload transport: Linux class/device0x20 must not silently
  be replaced with class/interface0x21 and called equivalent without evidence.
- Only then add a narrowly confirmed transport using the already-tested AIT
  sequence and firmware/device guards. Never send a boot command after an error.

The optional 1.0.0 per-user installer copies the diagnostic only.
No WinUSB rebind, device removal, driver installer, INF/SYS/CAT or signing
workaround is included. If a custom driver eventually proves necessary, its
binding, rollback and applicable Microsoft signing process need separate design
and Windows validation; test-signing or disabling security is not a solution.

The owner's reference [`Psypher9/VG-STC-Driver`](https://github.com/Psypher9/VG-STC-Driver)
is a guide only and targets VG-STC5000, not the attached VG-STC3000. No copied
implementation or compatibility claim is derived from its README.

## Primary references

- https://learn.microsoft.com/windows-hardware/drivers/stream/device-requirements-for-usb-video-class-extension-units
- https://learn.microsoft.com/windows-hardware/drivers/stream/ksproperty-extension-unit-info
- https://learn.microsoft.com/windows-hardware/drivers/stream/ks-properties
- https://learn.microsoft.com/windows-hardware/drivers/stream/uvc-camera-implementation-guide
