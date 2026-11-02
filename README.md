# Samsung VG-STC3000 camera tools 1.0.0

Rust tools for the Samsung VG-STC3000 USB television camera (boot `04e8:205c`,
initialized `04e8:205d`). **Linux initialization works; Windows now includes an
experimental WinUSB initializer, but its required signed driver catalog and
hardware validation are still pending.**

| Platform | Included | Verification boundary |
| --- | --- | --- |
| Linux x86-64 | RAM firmware initializer, direct preview launcher, optional udev/systemd installer | Owner confirmed initialization, video/audio and automatic loading |
| Windows 11 x64 | Diagnostics, guarded WinUSB RAM loader, boot-device INF and installers | Native build/tests; signed catalog and physical-camera testing pending |

The existing OS video/audio drivers remain in use. WinUSB is matched only to the
cold boot device, never to runtime UVC `04e8:205d`. No custom kernel driver,
firmware flashing, security-policy workaround or 60 fps support is provided.
LED and image behavior were accepted by the owner and are not modified here.

## Download and install

Get the platform archive and `SHA256SUMS` from
[Releases](https://github.com/malexvr/TVCameraSamsung/releases).
Checksums detect accidental corruption; they are not publisher signatures.
The Windows EXE is **unsigned**. If Windows blocks it, preserve the warning;
do not disable Secure Boot, signature enforcement or antivirus protection.

### Linux

Extract `samsung-camera-1.0.0-linux-x86_64.tar.gz` on a systemd/udev Linux host.
The binary is built on Fedora 44; see `BUILD-INFO.txt` for linked-library/GLIBC
requirements. It is not a universal Linux binary. Install your distribution's
libusb runtime and the required polkit/systemd/Python tools separately.

Supply your own lawfully obtained `FalconFW.bin`; vendor firmware is **not included**.
Only the known 682712-byte image with SHA-256
`df71a001ec9045d9115b482f76cffe5e30dfac5fe26e7ef8a2bd25854af20b84`
is accepted. This pin is not a vendor authenticity signature.
Follow `packaging/linux/README.md` in the archive for check/explicit installation.
**Do not reinstall on an already working host.** Existing destinations are never
silently replaced. Installation may leave earlier files in place after a failure.

Manual use without installing automatic loading:

```sh
./bin/samsung-camera inspect /path/to/FalconFW.bin
lsusb -d 04e8:205c
pkexec "$PWD/bin/samsung-camera" load /path/to/FalconFW.bin /dev/bus/usb/BBB/DDD
```

Replace `BBB/DDD` with the current address. Close capture applications first.
Firmware is uploaded to RAM and the device re-enumerates; do not repeat uploads
against an active camera. See [automatic loading](docs/AUTOLOAD.md).

### Windows

Build with `windows/BUILD.cmd`. For a local installation, follow
`windows/driver/README.md` to select Microsoft's already signed **WinUsb Device**
manually for parent boot `04e8:205c`, then run `driver/CONFIGURE-INBOX.cmd` as
administrator to register the loader interface. This does not require a custom
catalog. For distribution, the alternative custom-INF route requires a trusted
signed `samsung-vg-stc3000.cat`; no catalog or signing bypass is included.

Supply the same pinned `FalconFW.bin` as Linux, then run:

```bat
windows\samsung-camera-windows.exe inspect C:\path\to\FalconFW.bin
windows\samsung-camera-windows.exe load C:\path\to\FalconFW.bin
```

`inspect` performs no USB access. `load` validates firmware and device identity,
uploads to RAM and waits for runtime `04e8:205d`. Actual Windows driver/firmware
and media operation have not been hardware-tested. `RUN-DIAGNOSTICS.cmd` remains
read-only. `installer/INSTALL.cmd` optionally copies utilities per user but never
installs a driver. See [Windows status](docs/WINDOWS.md).

## Build from source

Linux needs Rust 1.88+, GCC and libusb development headers. Windows cross-build
needs Rust 1.88's `x86_64-pc-windows-gnu` target and MinGW.

```sh
cargo +1.88.0 build --offline --locked --release
bash windows/build-linux.sh
```

Portable Windows tests do not access devices:

```sh
cargo +1.88.0 test --offline --locked --manifest-path windows/Cargo.toml
```

The Linux integration suite additionally needs your separately supplied pinned
firmware at `FalconFW.bin`; it is intentionally absent from the public source archive.

## Scope and license

New Rust/tools code is GPL-3.0-only; see [LICENSE](LICENSE). Vendor firmware,
third-party archives, private captures, credentials and internal work notes are
not distributed. The protocol follows the documented behavior of
[samsung-tvcam-fwloader](https://github.com/ondrej-zary/samsung-tvcam-fwloader).
The [VG-STC-Driver reference](https://github.com/Psypher9/VG-STC-Driver) targets
VG-STC5000 and is not a compatibility guarantee for this camera. No NVIDIA
code, model weights or Blucast implementation is included.
