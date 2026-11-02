// SPDX-License-Identifier: GPL-3.0-only
//! Narrow libusb ABI adapter. No driver detach, module reload, or permission edits.
use samsung_camera::{Transport, INDEX};
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::FileTypeExt;
use std::ptr;

pub mod led;

#[repr(C)]
#[derive(Default)]
struct Descriptor {
    length: u8,
    kind: u8,
    usb: u16,
    class: u8,
    subclass: u8,
    protocol: u8,
    packet_size: u8,
    vendor: u16,
    product: u16,
    release: u16,
    manufacturer: u8,
    product_string: u8,
    serial: u8,
    configurations: u8,
}

#[link(name = "usb-1.0")]
unsafe extern "C" {
    fn libusb_set_option(context: *mut c_void, option: i32, ...) -> i32;
    fn libusb_init(context: *mut *mut c_void) -> i32;
    fn libusb_exit(context: *mut c_void);
    fn libusb_wrap_sys_device(context: *mut c_void, fd: isize, handle: *mut *mut c_void) -> i32;
    fn libusb_get_device(handle: *mut c_void) -> *mut c_void;
    fn libusb_get_device_descriptor(device: *mut c_void, descriptor: *mut Descriptor) -> i32;
    fn libusb_close(handle: *mut c_void);
    fn libusb_control_transfer(
        handle: *mut c_void,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: *mut u8,
        length: u16,
        timeout: u32,
    ) -> i32;
}

struct Context(*mut c_void);
impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: this context came from a successful libusb_init and is owned here.
        unsafe { libusb_exit(self.0) };
    }
}

pub struct Camera {
    handle: *mut c_void,
    _context: Context,
    _file: File,
}

fn checked(code: i32, operation: &str) -> Result<(), String> {
    if code < 0 {
        Err(format!("{operation}: libusb error {code}"))
    } else {
        Ok(())
    }
}

fn validate_identity(descriptor: &Descriptor) -> Result<(), String> {
    if descriptor.length != 18
        || descriptor.kind != 1
        || descriptor.vendor != 0x04e8
        || descriptor.product != 0x205c
    {
        return Err(format!(
            "Refusing device {:04x}:{:04x}; expected VG-STC3000 04e8:205c",
            descriptor.vendor, descriptor.product
        ));
    }
    Ok(())
}

pub fn validate_usb_path(path: &str) -> Result<(), String> {
    let valid = path
        .strip_prefix("/dev/bus/usb/")
        .and_then(|s| s.split_once('/'))
        .is_some_and(|(bus, device)| {
            bus.len() == 3
                && device.len() == 3
                && bus
                    .bytes()
                    .chain(device.bytes())
                    .all(|b| b.is_ascii_digit())
                && bus.parse::<u16>().is_ok_and(|n| n > 0)
                && device.parse::<u16>().is_ok_and(|n| (1..=127).contains(&n))
        });
    if valid {
        Ok(())
    } else {
        Err("Expected /dev/bus/usb/BBB/DDD with a valid USB address".into())
    }
}

impl Camera {
    pub fn open(path: &str) -> Result<Self, String> {
        validate_usb_path(path)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| format!("Cannot open USB device: {e}; use host-side polkit if needed"))?;
        if !file
            .metadata()
            .map_err(|e| e.to_string())?
            .file_type()
            .is_char_device()
        {
            return Err("USB path is not a character device".into());
        }
        let mut context = ptr::null_mut();
        // SAFETY: option 2 takes no varargs; it disables discovery only in this
        // process. The context output pointer is valid for the initialization.
        unsafe {
            checked(libusb_set_option(ptr::null_mut(), 2), "Disable discovery")?;
            checked(libusb_init(&mut context), "Initialize libusb")?;
        }
        let context = Context(context);
        let mut handle = ptr::null_mut();
        // SAFETY: fd stays open throughout Camera's lifetime; context is live.
        unsafe {
            checked(
                libusb_wrap_sys_device(context.0, file.as_raw_fd() as isize, &mut handle),
                "Wrap USB device",
            )?;
        }
        if handle.is_null() {
            return Err("libusb returned an empty handle".into());
        }
        let camera = Self {
            handle,
            _context: context,
            _file: file,
        };
        let mut descriptor = Descriptor::default();
        // SAFETY: handle is valid and Descriptor matches libusb's C ABI.
        unsafe {
            let device = libusb_get_device(camera.handle);
            if device.is_null() {
                return Err("libusb returned an empty device".into());
            }
            checked(
                libusb_get_device_descriptor(device, &mut descriptor),
                "Read USB identity",
            )?;
        }
        validate_identity(&descriptor)?;
        Ok(camera)
    }
}

impl Transport for Camera {
    fn send(&mut self, value: u16, payload: &[u8]) -> Result<usize, String> {
        let mut bytes = payload.to_vec();
        let length = u16::try_from(bytes.len()).map_err(|_| "USB payload too large")?;
        // SAFETY: handle is owned/live, bytes is writable for exactly length,
        // and this is a synchronous 1000ms class OUT transfer on endpoint zero.
        let count = unsafe {
            libusb_control_transfer(
                self.handle,
                0x20,
                1,
                value,
                INDEX,
                bytes.as_mut_ptr(),
                length,
                1000,
            )
        };
        checked(count, "AIT control transfer")?;
        Ok(count as usize)
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        // SAFETY: this uniquely owned handle is closed before context/file drop.
        unsafe { libusb_close(self.handle) };
    }
}

fn is_preview_camera(vendor: &str, product: &str) -> bool {
    vendor.trim() == "04e8" && matches!(product.trim(), "205c" | "205d")
}

fn usb_identity(sysfs: &std::path::Path) -> Option<(String, String)> {
    sysfs.ancestors().find_map(|parent| {
        Some((
            std::fs::read_to_string(parent.join("idVendor")).ok()?,
            std::fs::read_to_string(parent.join("idProduct")).ok()?,
        ))
    })
}

pub fn preview(device: &str) -> Result<(), String> {
    let suffix = device.strip_prefix("/dev/video").unwrap_or("");
    if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Expected /dev/videoN".into());
    }
    let sysfs = std::fs::canonicalize(format!("/sys/class/video4linux/video{suffix}/device"))
        .map_err(|e| format!("Cannot locate camera in sysfs: {e}"))?;
    let identity = usb_identity(&sysfs);
    if !identity.is_some_and(|(v, p)| is_preview_camera(&v, &p)) {
        return Err(
            "Refusing preview: expected Samsung 04e8:205c (boot) or 04e8:205d (initialized)".into(),
        );
    }
    if !std::fs::metadata(device)
        .map_err(|e| format!("Cannot access host video node: {e}"))?
        .file_type()
        .is_char_device()
    {
        return Err("Video path is not a character device".into());
    }
    let status = std::process::Command::new("ffplay")
        .args([
            "-hide_banner",
            "-f",
            "v4l2",
            "-input_format",
            "mjpeg",
            "-video_size",
            "1280x720",
            "-framerate",
            "30",
            "-i",
            device,
        ])
        .status()
        .map_err(|e| format!("Cannot start ffplay: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ffplay exited with {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_layout_matches_libusb_header() {
        assert_eq!(std::mem::size_of::<Descriptor>(), 18);
        assert_eq!(std::mem::offset_of!(Descriptor, vendor), 8);
        assert_eq!(std::mem::offset_of!(Descriptor, product), 10);
    }
    #[test]
    fn usb_paths_reject_traversal_and_out_of_range_addresses() {
        assert!(validate_usb_path("/dev/bus/usb/001/009").is_ok());
        for path in [
            "/dev/null",
            "/dev/bus/usb/001/../009",
            "/dev/bus/usb/001/128",
            "/dev/bus/usb/000/009",
            "/dev/bus/usb/001/000",
            "/dev/bus/usb/001/009/extra",
        ] {
            assert!(validate_usb_path(path).is_err(), "{path}");
        }
    }

    #[test]
    fn identity_guard_rejects_other_models_and_malformed_descriptors() {
        let valid = Descriptor {
            length: 18,
            kind: 1,
            vendor: 0x04e8,
            product: 0x205c,
            ..Descriptor::default()
        };
        assert!(validate_identity(&valid).is_ok());
        for wrong in [
            Descriptor {
                product: 0x2065,
                ..valid
            },
            Descriptor {
                product: 0x205d,
                ..valid
            },
            Descriptor {
                vendor: 0x1234,
                ..valid
            },
            Descriptor { length: 0, ..valid },
            Descriptor { kind: 0, ..valid },
        ] {
            assert!(validate_identity(&wrong).is_err());
        }
    }

    #[test]
    fn preview_accepts_boot_and_initialized_samsung_but_not_other_devices() {
        assert!(is_preview_camera("04e8\n", "205c\n"));
        assert!(is_preview_camera("04e8\n", "205d\n"));
        for (vendor, product) in [
            ("5986", "115f"),
            ("04e8", "2065"),
            ("1234", "205d"),
            ("04e8", "205e"),
        ] {
            assert!(!is_preview_camera(vendor, product));
        }
    }
}
