// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use std::time::Duration;

fn phase_present(wanted: &str) -> Result<bool, String> {
    let set = Set::new(None)?;
    for index in 0..4096 {
        let mut info = DeviceData::new();
        // SAFETY: live set and correctly sized output structure.
        if unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut info) } == 0 {
            let code = unsafe { GetLastError() };
            if code == 259 {
                return Ok(false);
            }
            return Err(format!("Enumerate camera state: Windows error {code}"));
        }
        if set.identity(&mut info)? == Some(wanted) {
            return Ok(true);
        }
    }
    Err("Present-device enumeration exceeded bound".into())
}

fn detail(set: &Set, interface: &InterfaceData) -> Result<(Vec<u16>, DeviceData), String> {
    let mut needed = 0;
    // SAFETY: first call queries the detail buffer size only.
    let ok = unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            set.0,
            interface,
            ptr::null_mut(),
            0,
            &mut needed,
            ptr::null_mut(),
        )
    };
    if ok == 0 && unsafe { GetLastError() } != 122 {
        return Err(format!(
            "WinUSB interface detail size: Windows error {}",
            unsafe { GetLastError() }
        ));
    }
    if !(6..=65536).contains(&needed) {
        return Err("WinUSB interface detail size exceeds bound".into());
    }
    let size = needed;
    let mut storage = vec![0u64; (size as usize).div_ceil(8)];
    // SAFETY: storage is DWORD-aligned; SDK cbSize differs by pointer width.
    unsafe {
        ptr::write(
            storage.as_mut_ptr().cast::<u32>(),
            if size_of::<usize>() == 8 { 8 } else { 6 },
        );
    }
    let mut info = DeviceData::new();
    // SAFETY: correctly sized detail buffer and associated devinfo output.
    if unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            set.0,
            interface,
            storage.as_mut_ptr().cast(),
            size,
            &mut needed,
            &mut info,
        )
    } == 0
    {
        return Err(format!(
            "Read WinUSB interface detail: Windows error {}",
            unsafe { GetLastError() }
        ));
    }
    if needed > size {
        return Err("WinUSB interface detail changed length".into());
    }
    // SAFETY: successful detail call returned no more than the allocated length.
    let bytes =
        unsafe { std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), needed as usize) };
    Ok((crate::interface_path(bytes)?, info))
}

fn boot_interface() -> Result<Vec<u16>, String> {
    let set = Set::new(Some(&WINUSB_INTERFACE))?;
    let mut matches = Vec::new();
    for index in 0..4096 {
        let mut interface = InterfaceData::new();
        // SAFETY: live interface set and matching device-interface GUID.
        if unsafe {
            SetupDiEnumDeviceInterfaces(
                set.0,
                ptr::null(),
                &WINUSB_INTERFACE,
                index,
                &mut interface,
            )
        } == 0
        {
            let code = unsafe { GetLastError() };
            if code == 259 {
                break;
            }
            return Err(format!("Enumerate WinUSB interface: Windows error {code}"));
        }
        let (path, mut info) = detail(&set, &interface)?;
        if set.identity(&mut info)? != Some("boot") {
            continue;
        }
        if !set.string(&mut info, 4)?.eq_ignore_ascii_case("winusb") {
            return Err("Samsung boot interface is not bound to WinUSB; no device opened".into());
        }
        matches.push((path, info));
        if matches.len() > 1 {
            return Err(
                "More than one Samsung boot interface found; refusing ambiguous upload".into(),
            );
        }
    }
    let (path, mut info) = matches.into_iter().next().ok_or(
        "No Samsung 04e8:205c WinUSB interface found. Connect the cold camera and install the signed boot-driver package.",
    )?;
    if set.identity(&mut info)? != Some("boot")
        || !set.string(&mut info, 4)?.eq_ignore_ascii_case("winusb")
    {
        return Err("Device association changed; upload cancelled".into());
    }
    Ok(path)
}

struct Camera {
    interface: Handle,
    _device: Device,
}

impl Camera {
    fn open(path: &[u16]) -> Result<Self, String> {
        let device = Device::open_winusb(path)?;
        let mut interface = 0;
        // SAFETY: live device handle and valid output pointer for WinUsb_Initialize.
        if unsafe { WinUsb_Initialize(device.0, &mut interface) } == 0 {
            return Err(format!("Initialize WinUSB: Windows error {}", unsafe {
                GetLastError()
            }));
        }
        if interface == 0 || interface == -1 {
            return Err("WinUSB returned an empty interface handle".into());
        }
        let camera = Self {
            interface,
            _device: device,
        };
        let mut descriptor = [0u8; 18];
        let mut transferred = 0;
        // SAFETY: live interface and an 18-byte writable descriptor buffer.
        if unsafe {
            WinUsb_GetDescriptor(
                camera.interface,
                1,
                0,
                0,
                descriptor.as_mut_ptr(),
                descriptor.len() as u32,
                &mut transferred,
            )
        } == 0
        {
            return Err(format!(
                "Read USB device descriptor: Windows error {}",
                unsafe { GetLastError() }
            ));
        }
        validate_descriptor(&descriptor, transferred as usize)?;
        Ok(camera)
    }
}

fn validate_descriptor(data: &[u8; 18], transferred: usize) -> Result<(), String> {
    let vendor = u16::from_le_bytes([data[8], data[9]]);
    let product = u16::from_le_bytes([data[10], data[11]]);
    if transferred != data.len()
        || data[0] != 18
        || data[1] != 1
        || vendor != 0x04e8
        || product != 0x205c
    {
        return Err(format!(
            "USB identity check failed: expected Samsung 04e8:205c boot descriptor, got {vendor:04x}:{product:04x}"
        ));
    }
    Ok(())
}

impl Transport for Camera {
    fn send(&mut self, value: u16, payload: &[u8]) -> Result<usize, String> {
        if payload.len() > 4096 {
            return Err("WinUSB control payload exceeds 4096 bytes".into());
        }
        let mut bytes = payload.to_vec();
        let setup = SetupPacket {
            request_type: 0x20,
            request: 1,
            value,
            index: INDEX,
            length: u16::try_from(payload.len()).map_err(|_| "USB payload too large")?,
        };
        let mut transferred = 0;
        // SAFETY: live interface, initialized 8-byte setup packet and writable buffer.
        // A null OVERLAPPED pointer makes this a synchronous device-recipient request.
        let ok = unsafe {
            WinUsb_ControlTransfer(
                self.interface,
                setup,
                bytes.as_mut_ptr(),
                payload.len() as u32,
                &mut transferred,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(format!(
                "AIT WinUSB control transfer: Windows error {}",
                unsafe { GetLastError() }
            ));
        }
        Ok(transferred as usize)
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        // SAFETY: uniquely owned live WinUSB interface; device closes afterwards.
        unsafe {
            WinUsb_Free(self.interface);
        }
    }
}

pub fn load(firmware: &[u8]) -> Result<usize, String> {
    if phase_present("runtime")? {
        return Err("Camera is already initialized as 04e8:205d; no firmware sent".into());
    }
    let path = boot_interface()?;
    let mut camera = Camera::open(&path)?;
    let blocks = samsung_camera::upload(&mut camera, firmware)?;
    drop(camera);
    for _ in 0..40 {
        if phase_present("runtime")? {
            return Ok(blocks);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err("RAM boot command was sent, but runtime 04e8:205d was not detected within 20 seconds; no upload retry was made".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_packet_and_descriptor_guards_are_exact() {
        assert_eq!(size_of::<SetupPacket>(), 8);
        let mut descriptor = [0u8; 18];
        descriptor[0] = 18;
        descriptor[1] = 1;
        descriptor[8..10].copy_from_slice(&0x04e8u16.to_le_bytes());
        descriptor[10..12].copy_from_slice(&0x205cu16.to_le_bytes());
        assert!(validate_descriptor(&descriptor, 18).is_ok());
        assert!(validate_descriptor(&descriptor, 17).is_err());
        descriptor[10] ^= 1;
        assert!(validate_descriptor(&descriptor, 18).is_err());
    }
}
