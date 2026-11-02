// SPDX-License-Identifier: GPL-3.0-only
const HELP: &str = "Usage: samsung-camera-windows inventory|probe|--help
Windows read-only diagnostic; this is NOT a working firmware loader.
No driver installation, camera capture, firmware/LED writes or security changes.
Firmware upload is not implemented; the boot-device transport needs native evidence.
The runtime XU GUID is only a candidate for boot-state probing.
Reports omit instance paths/serial numbers. Close camera applications before probe.";
fn run(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [x] if x=="--help" || x=="-h" => {println!("{HELP}");Ok(())},
        [x,..] if x=="load" => Err("Firmware upload is not implemented. No device opened. Do not replace drivers or disable Windows security.".into()),
        [x] if x=="inventory" || x=="probe" => {
            #[cfg(windows)] { samsung_camera_windows::native::run(x=="probe") }
            #[cfg(not(windows))] { Err("Windows only; no device opened. Use --help.".into()) }
        },
        _=>Err(HELP.into())
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
