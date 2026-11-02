// SPDX-License-Identifier: GPL-3.0-only
#[cfg(target_os = "linux")]
mod linux;

const HELP: &str = "Usage:
  samsung-camera inspect FIRMWARE
  samsung-camera load FIRMWARE /dev/bus/usb/BBB/DDD
  samsung-camera preview /dev/videoN
  samsung-camera led-info /dev/videoN
  samsung-camera led /dev/videoN on|off --confirm-experimental

Linux only. inspect performs no USB access. load explicitly uploads the pinned
VG-STC3000 firmware to RAM. Close camera/audio apps before load. Never run preview
as root. preview uses ffplay at MJPEG 1280x720/30, bypassing Snapshot; it is not
a proven repair for Snapshot's split preview. Preview recognizes the known boot
and initialized USB identities; firmware upload remains boot-only.
led-info sends read-only UVC capability/length queries, never SET_CUR. led is an
explicit historical LED-command test; confirmation and compatible replies are
required. It does not implement an automatic activity/privacy indicator.
Windows transport is not implemented.";

fn main() {
    let args = std::env::args_os()
        .skip(1)
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "Arguments must be UTF-8".to_owned())
        })
        .collect::<Result<Vec<_>, _>>();
    if let Err(error) = args.and_then(run) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    if args.as_slice() == ["--help"] || args.as_slice() == ["-h"] {
        println!("{HELP}");
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    match args.as_slice() {
        [command, path] if command == "inspect" => {
            let bytes = firmware(path)?;
            println!("VG-STC3000: {} bytes; SHA-256 verified", bytes.len());
            println!(
                "{} blocks; {} padding bytes in final block",
                bytes.len() / 32 + 1,
                32 - bytes.len() % 32
            );
            println!("No USB writes; firmware inspection only.");
            return Ok(());
        }
        [command, path, device] if command == "load" => {
            linux::validate_usb_path(device)?;
            let bytes = firmware(path)?;
            let mut camera = linux::Camera::open(device)?;
            let blocks = samsung_camera::upload(&mut camera, &bytes)?;
            println!("Uploaded {blocks} blocks and sent RAM boot command.");
            println!("Verify video/audio separately; this is not a capture success report.");
            return Ok(());
        }
        [command, device] if command == "preview" => return linux::preview(device),
        [command, device] if command == "led-info" => {
            let info = linux::led::info(device)?;
            println!(
                "LED unit 6 selector 4: GET_INFO=0x{:02x}, GET_LEN={}, manual_test_allowed={}",
                info.capabilities,
                info.length,
                info.can_test()
            );
            println!("No SET_CUR sent. This is not an LED-state or streaming report.");
            return Ok(());
        }
        [command, device, state, confirmation]
            if command == "led" && confirmation == "--confirm-experimental" =>
        {
            let on = match state.as_str() {
                "on" => true,
                "off" => false,
                _ => return Err("LED state must be on or off".into()),
            };
            linux::led::set(device, on)?;
            println!("Historical LED command submitted; verify the physical LED yourself.");
            println!("Manual state is not tied to capture; turn it off after the test.");
            return Ok(());
        }
        [command, ..] if command == "led" => {
            return Err(format!(
                "LED manual test requires on|off and --confirm-experimental.\n{HELP}"
            ))
        }
        _ => {}
    }
    Err(HELP.to_owned())
}

#[cfg(target_os = "linux")]
fn firmware(path: &str) -> Result<Vec<u8>, String> {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    const SIZE: usize = 682_712;
    const HASH: &str = "df71a001ec9045d9115b482f76cffe5e30dfac5fe26e7ef8a2bd25854af20b84";
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open firmware: {e}"))?;
    let mut bytes = Vec::with_capacity(SIZE + 1);
    file.take((SIZE + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Cannot read firmware: {e}"))?;
    if bytes.len() != SIZE || !bytes.starts_with(b"AIT842SF") {
        return Err("Expected the known 682712-byte AIT842SF FalconFW.bin".into());
    }
    // Hash the already-read bytes, not the pathname, to avoid verifying one
    // version and uploading another. No shell and no filename interpolation.
    let mut child = Command::new("/usr/bin/sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Cannot start /usr/bin/sha256sum: {e}"))?;
    let write_result = child
        .stdin
        .take()
        .ok_or("Hash input pipe unavailable")?
        .write_all(&bytes);
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Cannot collect SHA-256: {e}"))?;
    write_result.map_err(|e| format!("Cannot hash firmware: {e}"))?;
    if !output.status.success()
        || String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .next()
            != Some(HASH)
    {
        return Err("Firmware SHA-256 mismatch or hash command failure; no USB writes".into());
    }
    Ok(bytes)
}
