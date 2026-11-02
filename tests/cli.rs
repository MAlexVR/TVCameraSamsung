// SPDX-License-Identifier: GPL-3.0-only
#![cfg(target_os = "linux")]

use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_samsung-camera"))
        .args(args)
        .output()
        .unwrap()
}

fn firmware() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("FalconFW.bin")
}

#[test]
fn help_lists_explicit_commands_without_hardware_access() {
    let output = cli(&["--help"]);
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for command in [
        "inspect",
        "load",
        "preview",
        "led-info",
        "--confirm-experimental",
    ] {
        assert!(help.contains(command));
    }
}

#[test]
fn inspect_verifies_known_firmware_without_hardware_access() {
    let output = cli(&["inspect", firmware().to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in ["682712", "21335", "8", "No USB writes"] {
        assert!(text.contains(expected));
    }
}

#[test]
fn malformed_device_is_rejected_before_usb_access() {
    let output = cli(&["load", firmware().to_str().unwrap(), "/dev/null"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("/dev/bus/usb/BBB/DDD"));
}

#[test]
fn unknown_options_fail_instead_of_being_ignored() {
    let output = cli(&["inspect", firmware().to_str().unwrap(), "--force"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Usage"));
}

#[test]
fn invalid_firmware_is_rejected_without_modification() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let before = std::fs::read(&path).unwrap();
    let output = cli(&["inspect", path.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("682712"));
    assert_eq!(before, std::fs::read(&path).unwrap());
}

#[test]
fn wrong_hash_is_rejected_without_modifying_input() {
    let mut bytes = std::fs::read(firmware()).unwrap();
    bytes[100] ^= 1;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("samsung-fw-test-{}-{nonce}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .unwrap();
    let path = directory.join("firmware.bin");
    std::fs::write(&path, &bytes).unwrap();
    let output = cli(&["inspect", path.to_str().unwrap()]);
    let after = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(&directory).unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("SHA-256"));
    assert_eq!(bytes, after);
}

#[test]
fn preview_requires_an_explicit_video_node() {
    let output = cli(&["preview", "/dev/null"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("/dev/videoN"));
}

#[test]
fn led_info_rejects_non_video_paths_before_queries() {
    let output = cli(&["led-info", "/dev/null"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Expected /dev/videoN"));
}

#[test]
fn led_write_requires_confirmation_and_known_state_before_device_access() {
    let missing = cli(&["led", "/dev/null", "on"]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("--confirm-experimental"));
    let invalid = cli(&["led", "/dev/null", "blink", "--confirm-experimental"]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("LED state must be on or off"));
    let invalid_node = cli(&["led", "/dev/null", "on", "--confirm-experimental"]);
    assert!(!invalid_node.status.success());
    assert!(String::from_utf8_lossy(&invalid_node.stderr).contains("Expected /dev/videoN"));
}
