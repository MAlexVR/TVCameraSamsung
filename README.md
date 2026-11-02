# Samsung VG-STC3000 camera tools 1.0.0

Rust tools for the Samsung VG-STC3000 USB television camera (boot `04e8:205c`,
initialized `04e8:205d`). **Linux initialization works; Windows support is an
experimental read-only diagnostic, NOT a functional camera driver.**

| Platform | Included | Verification boundary |
| --- | --- | --- |
| Linux x86-64 | RAM firmware initializer, direct preview launcher, optional udev/systemd installer | Owner confirmed initialization, video/audio and automatic loading |
| Windows 11 x64 | Inventory/UVC metadata diagnostic and per-user installer | Cross-compiled; native execution and hardware testing pending |

The existing OS video/audio drivers remain in use. No replacement kernel driver,
firmware flashing, security-policy workaround or 60 fps support is provided.
LED and image behavior were accepted by the owner and are not modified by this release.

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

Extract `samsung-camera-1.0.0-windows-x86_64-diagnostic.zip`.
Run `installer/INSTALL.cmd` and confirm installation of the **diagnostic only**.
It uses a versioned per-user directory, no administrator access, driver replacement,
registry/startup changes or security-policy changes. Existing installations cause
a refusal, not an overwrite. PowerShell policy is respected, never bypassed.

Connect the camera and run `RUN-DIAGNOSTICS.cmd` from the installed directory,
or directly from the extracted archive without installation. Share the resulting
`samsung-camera-windows-report-*.txt`. No Rust installation is needed to run it.
The report is deliberately limited to target-camera metadata, not media or serials.
The `load` command is unavailable. See [Windows status](docs/WINDOWS.md).

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
