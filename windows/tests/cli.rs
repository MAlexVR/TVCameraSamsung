use std::process::Command;

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_samsung-camera-windows"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_identifies_read_only_scope_and_unimplemented_upload() {
    let out = cli(&["--help"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for required in [
        "inventory",
        "probe",
        "read-only",
        "not implemented",
        "No driver installation",
    ] {
        assert!(text.contains(required), "missing {required}");
    }
}

#[test]
fn upload_is_refused_before_access_even_with_force_like_arguments() {
    for args in [vec!["load"], vec!["load", "FalconFW.bin", "--force"]] {
        let out = cli(&args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("No device opened"));
    }
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
