// SPDX-License-Identifier: GPL-3.0-only
const HELP: &str = "Usage:
  samsung-camera-windows inventory
  samsung-camera-windows probe
  samsung-camera-windows inspect FIRMWARE
  samsung-camera-windows load FIRMWARE

inventory and probe are read-only diagnostics. inspect validates the pinned
682712-byte FalconFW.bin without USB access. load uploads that firmware to RAM
through WinUSB and sends the boot command; it does not install a driver.
The boot device 04e8:205c requires the separately signed WinUSB driver package.
The initialized UVC device 04e8:205d continues using Microsoft's drivers.
Close camera/audio applications before load. Never disable Windows security.";

fn run(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [x] if x == "--help" || x == "-h" => {
            println!("{HELP}");
            Ok(())
        }
        [command, path] if command == "inspect" => {
            let bytes = samsung_camera_windows::firmware::read(path)?;
            println!(
                "VG-STC3000: {} bytes; SHA-256 verified; {} RAM blocks; no USB writes.",
                bytes.len(),
                bytes.len() / 32 + 1
            );
            Ok(())
        }
        [command, path] if command == "load" => {
            let bytes = samsung_camera_windows::firmware::read(path)?;
            #[cfg(windows)]
            {
                let blocks = samsung_camera_windows::native::load(&bytes)?;
                println!("Uploaded {blocks} blocks, sent RAM boot command and detected runtime 04e8:205d.");
                println!("Firmware is in RAM only; repeat after a cold device boot. Verify video/audio separately.");
                Ok(())
            }
            #[cfg(not(windows))]
            {
                Err("Windows only; firmware validated but no device opened.".into())
            }
        }
        [x] if x == "inventory" || x == "probe" => {
            #[cfg(windows)]
            {
                samsung_camera_windows::native::run(x == "probe")
            }
            #[cfg(not(windows))]
            {
                Err("Windows only; no device opened. Use --help.".into())
            }
        }
        _ => Err(HELP.into()),
    }
}
fn main() {
    let args = std::env::args_os()
        .skip(1)
        .map(|x| {
            x.into_string()
                .map_err(|_| "Arguments must be Unicode".to_owned())
        })
        .collect::<Result<Vec<_>, _>>();
    if let Err(error) = args.and_then(run) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
