# Samsung VG-STC3000 Windows diagnostic 1.0.0

**Read-only diagnostic, NOT a functional firmware loader or signed driver.**
This first test identifies the Windows driver binding and queries metadata that
Microsoft's inbox UVC driver exposes. It does not initialize an unbooted camera,
start video/audio, install a driver, modify the LED, or change Windows security.

## What is included

- Independent Rust project with **zero Cargo dependencies**, Rust1.88+.
- `inventory`: only present Samsung `04e8:205c`/`205d` PnP bindings/status.
- `probe`: matching camera/capture interfaces, KS topology, candidate XU descriptor
  and BASIC SUPPORT metadata for selectors3/4. It never follows SET support with
  a write. It queries only interfaces bound to Microsoft's `usbvideo` service.
- Parser/ABI/CLI tests and explicit refusal of `load`, even with extra arguments.
- Build/run helpers that install nothing and do not request administrator access.

`samsung-camera-windows-test-win64.zip` includes the Windows x64 EXE and source;
no Rust installation is needed to run that package. The separate `*-source.zip`
contains source only. The EXE was cross-compiled and linked with Rust1.88/MinGW;
its SDK layouts/constants and system-DLL imports were checked. It has not been
executed on Windows or tested with the hardware. Cross-compilation is not native
execution proof. It is unsigned and is still NOT a firmware loader or driver.

## Optional per-user installation

The 1.0.0 diagnostic archive includes `installer/INSTALL.cmd`. Follow
`installer/README.md`; it installs only the diagnostic, not a camera driver.
It respects PowerShell policy and needs no administrator access.

## Build on Windows

With Rust and its matching Windows linker/SDK already installed, open a normal
terminal in this directory and run:

```bat
BUILD.cmd
```

The helper runs offline tests/build and creates `samsung-camera-windows.exe`.
It does not install Rust, an SDK, drivers, certificates or system components.
If prerequisites are missing, stop and follow the official
[Rust Windows installation documentation](https://rust-lang.github.io/rustup/installation/windows.html).
Do not change execution policy, Secure Boot, signature enforcement, S mode or
antivirus protection to make this test run. If security software blocks it,
report the warning rather than disabling the protection. The resulting user-mode
EXE is unsigned; no INF/SYS/CAT or kernel-driver signing claim is made.

## Run the diagnostic

Connect the camera directly to the Windows machine. Close applications using it.
Prefer testing its normal cold-connection boot state; do not initialize it on
Linux just to hide a Windows boot-device failure. If it already identifies as
runtime205d, report that distinction.

```bat
RUN-DIAGNOSTICS.cmd
```

Or run the individual commands:

```bat
samsung-camera-windows.exe inventory
samsung-camera-windows.exe probe
```

The helper creates a new `samsung-camera-windows-report-*.txt` without overwriting
an existing report. It contains JSON lines plus command exit summaries. Send that
report back. No serial numbers, device-interface/instance paths, user/computer
names, raw property buffers, video or microphone data are deliberately output.
Only the target camera's service/class/status and capability metadata are saved.
The inventory API must enumerate present PnP metadata to identify the target;
other devices' entries are not reported.

A nonzero probe exit can be a useful result: Windows may not expose the candidate
controls. Preserve the report. Do not install WinUSB/Zadig, replace the camera or
audio driver, unregister anything, flash firmware or guess another GUID/selector.

## Optional Linux cross-build prerequisites

For the maintainer's Linux build, the documented target is Windows x64 GNU.
`build-linux.sh` checks prerequisites but never installs them. Once the MinGW
linker and Rust1.88 Windows standard library exist:

```sh
bash windows/build-linux.sh
```

Run that command from the parent project. The script produces
`windows/samsung-camera-windows.exe`. This is still a diagnostic; building an
executable does not establish native hardware support. Native Windows build can
use its installed default target; only actually built/tested targets may be claimed.

## Why firmware loading is deliberately unavailable

Linux's verified uploader uses class/device request type0x20. A UVC extension
property is routed by Windows through class/interface control semantics. That
transport compatibility is not established for the boot device. Microsoft also
requires several control queries during initialization and may hide a control
when they fail; advertising SET does not prove the upload interface is usable.

There is no universal `PROPSETID_VIDCAP_EXTENSION_UNIT` discovery GUID: the sample
name stands for the actual descriptor GUID. Candidate unit6 GUID
`23e49ed0-1178-4f31-ae52-d2fb8a8d3b48` was observed only on Linux runtime205d,
not independently on boot205c. The probe checks the returned cached INFO GUID;
no match or exposed metadata is a legitimate diagnostic failure.

KS BASIC SUPPORT is **not raw USB GET_INFO/GET_LEN**. It is queried only as
metadata; even successful output is not an instruction to send a write.

## Development checks

```sh
cargo test --offline --manifest-path windows/Cargo.toml
cargo check --offline --manifest-path windows/Cargo.toml --features check-windows-ffi
```

The feature permits type-checking declarations on Linux. It does not turn Linux
into Windows, link a Windows executable, run Win32 APIs, or verify native ABI.
The independent crate does not modify Linux's Cargo manifest, firmware, loader,
autoload service, image settings or LED behavior.

## References and license

- [Microsoft UVC extension requirements](https://learn.microsoft.com/windows-hardware/drivers/stream/device-requirements-for-usb-video-class-extension-units)
- [Extension architecture](https://learn.microsoft.com/windows-hardware/drivers/stream/extension-unit-plug-in-architecture)
- [Cached INFO metadata](https://learn.microsoft.com/windows-hardware/drivers/stream/ksproperty-extension-unit-info)
- [Kernel-streaming properties](https://learn.microsoft.com/windows-hardware/drivers/stream/ks-properties)
- [Microsoft SDK declarations](https://github.com/microsoft/win32metadata/tree/main/generation/WinSDK/RecompiledIdlHeaders/shared)
- [Psypher9 guide](https://github.com/Psypher9/VG-STC-Driver): targets VG-STC5000,
  not this3000; its implementation was not copied.

New source: SPDX `GPL-3.0-only`; [license text](https://www.gnu.org/licenses/gpl-3.0.html).
No Samsung firmware, third-party driver or certificate is redistributed here.
