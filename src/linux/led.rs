// SPDX-License-Identifier: GPL-3.0-only
//! UVC extension queries through the existing kernel driver, not raw USB.
use samsung_camera::led::{self as protocol, Info, Query, GET_INFO, GET_LEN, SET_CUR};
use std::ffi::{c_int, c_ulong};
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt};

#[repr(C)]
struct XuQuery {
    unit: u8,
    selector: u8,
    query: u8,
    padding: u8,
    size: u16,
    pointer_padding: [u8; 2],
    data: *mut u8,
}

// asm-generic _IOWR('u', 0x21, struct uvc_xu_control_query).
const UVCIOC_CTRL_QUERY: c_ulong = (3 << 30)
    | ((std::mem::size_of::<XuQuery>() as c_ulong) << 16)
    | ((b'u' as c_ulong) << 8)
    | 0x21;

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

fn initialized_identity(identity: Option<(String, String)>) -> bool {
    identity.is_some_and(|(vendor, product)| vendor.trim() == "04e8" && product.trim() == "205d")
}

fn device_numbers(device: u64) -> (u64, u64) {
    // Linux dev_t encoding, including high major/minor bits.
    (
        ((device >> 8) & 0xfff) | ((device >> 32) & 0xfffff000),
        (device & 0xff) | ((device >> 12) & 0xffffff00),
    )
}

struct Device(File);

impl Device {
    fn open(path: &str, writable: bool) -> Result<Self, String> {
        if !cfg!(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "arm",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )) {
            return Err("LED queries are unavailable for this platform's ioctl ABI".into());
        }
        let suffix = path.strip_prefix("/dev/video").unwrap_or("");
        if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Expected /dev/videoN".into());
        }
        let expected =
            std::fs::canonicalize(format!("/sys/class/video4linux/video{suffix}/device"))
                .map_err(|e| format!("Cannot locate camera in sysfs: {e}"))?;
        if !initialized_identity(super::usb_identity(&expected)) {
            return Err(
                "LED queries require initialized Samsung 04e8:205d; no control sent".into(),
            );
        }
        let file = OpenOptions::new()
            .read(true)
            .write(writable)
            .open(path)
            .map_err(|e| format!("Cannot open host video node for LED query: {e}"))?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.file_type().is_char_device() {
            return Err("LED query target is not a character device".into());
        }
        // Validate the opened device, not only a pathname checked before open.
        let (major, minor) = device_numbers(metadata.rdev());
        let actual = std::fs::canonicalize(format!("/sys/dev/char/{major}:{minor}/device"))
            .map_err(|e| format!("Cannot verify opened video device: {e}"))?;
        if actual != expected || !initialized_identity(super::usb_identity(&actual)) {
            return Err(
                "Opened video identity changed or is not the initialized Samsung; no control sent"
                    .into(),
            );
        }
        Ok(Self(file))
    }
}

impl Query for Device {
    fn query(&mut self, request: u8, data: &mut [u8]) -> Result<(), String> {
        if !matches!(
            (request, data.len()),
            (GET_INFO, 1) | (GET_LEN, 2) | (SET_CUR, 8)
        ) {
            return Err("Unsupported LED query or buffer size".into());
        }
        let mut query = XuQuery {
            unit: 6,
            selector: 4,
            query: request,
            padding: 0,
            size: data.len() as u16,
            pointer_padding: [0; 2],
            data: data.as_mut_ptr(),
        };
        // SAFETY: fd is held open and its character-device identity was verified.
        // XuQuery matches the kernel C ABI; the synchronous ioctl receives a
        // live, writable buffer of its declared size. No interface is detached.
        let result = unsafe {
            ioctl(
                self.0.as_raw_fd(),
                UVCIOC_CTRL_QUERY,
                &mut query as *mut XuQuery,
            )
        };
        if result < 0 {
            return Err(format!(
                "UVC query 0x{request:02x} failed: {}; no retry performed",
                std::io::Error::last_os_error()
            ));
        }
        if result != 0 {
            return Err(format!(
                "Unexpected UVC ioctl result {result}; no retry performed"
            ));
        }
        Ok(())
    }
}

pub fn info(device: &str) -> Result<Info, String> {
    protocol::info(&mut Device::open(device, false)?)
}

pub fn set(device: &str, on: bool) -> Result<(), String> {
    protocol::set(&mut Device::open(device, true)?, on)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn led_requires_runtime_identity_not_boot_or_other_camera() {
        assert!(initialized_identity(Some((
            "04e8\n".into(),
            "205d\n".into()
        ))));
        for pair in [("04e8", "205c"), ("04e8", "2065"), ("1234", "205d")] {
            assert!(!initialized_identity(Some((pair.0.into(), pair.1.into()))));
        }
        assert!(!initialized_identity(None));
    }
    #[test]
    fn query_layout_matches_generic_linux_uvc_abi() {
        assert_eq!(std::mem::offset_of!(XuQuery, size), 4);
        assert_eq!(std::mem::offset_of!(XuQuery, data), 8);
        assert_eq!(
            std::mem::size_of::<XuQuery>(),
            8 + std::mem::size_of::<*mut u8>()
        );
        #[cfg(target_pointer_width = "64")]
        assert_eq!(UVCIOC_CTRL_QUERY, 0xc0107521);
    }
    #[test]
    fn opened_character_device_numbers_preserve_high_bits() {
        assert_eq!(device_numbers(0x5102), (81, 2));
        let (major, minor) = (0x12345678_u64, 0xabcdef12_u64);
        let encoded = ((major & 0xfff) << 8)
            | ((major & 0xfffff000) << 32)
            | (minor & 0xff)
            | ((minor & 0xffffff00) << 12);
        assert_eq!(device_numbers(encoded), (major, minor));
    }
}
