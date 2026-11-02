use std::process::Command;

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_samsung-camera-windows"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_identifies_diagnostics_inspection_and_guarded_upload() {
    let out = cli(&["--help"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for required in [
        "inventory",
        "probe",
        "inspect FIRMWARE",
        "load FIRMWARE",
        "read-only",
        "signed WinUSB driver package",
        "does not install a driver",
    ] {
        assert!(text.contains(required), "missing {required}");
    }
}

#[test]
fn upload_arguments_are_exact_and_invalid_firmware_is_rejected_before_usb() {
    for args in [vec!["load"], vec!["load", "FalconFW.bin", "--force"]] {
        let out = cli(&args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("Usage"));
    }
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let out = cli(&["load", manifest]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("682712"));
}

#[test]
fn inspect_rejects_invalid_firmware_without_usb_access() {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let out = cli(&["inspect", manifest]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("682712"));
}

#[test]
fn unknown_or_extra_arguments_are_not_ignored() {
    for args in [
        vec!["inventory", "--force"],
        vec!["probe", "other-camera"],
        vec!["--unknown"],
    ] {
        let out = cli(&args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
}

#[cfg(not(windows))]
#[test]
fn windows_commands_refuse_non_windows_host_without_device_access() {
    for command in ["inventory", "probe"] {
        let out = cli(&[command]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("Windows only"));
    }
}
