// SPDX-License-Identifier: GPL-3.0-only
//! Windows-only metadata API. No driver installation, SET, capture or upload path.
use crate::{Guid, NodeProperty, Property, DEV_SPECIFIC, UNIT6_CANDIDATE};
use std::collections::HashSet;
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr;

type Handle = isize;
const VIDEO_CAMERA: Guid = Guid {
    a: 0xe5323777,
    b: 0xf976,
    c: 0x4f5b,
    d: [0x9b, 0x55, 0xb9, 0x46, 0x99, 0xc4, 0x6e, 0x44],
};
const CAPTURE: Guid = Guid {
    a: 0x65e8773d,
    b: 0x8f56,
    c: 0x11d0,
    d: [0xa3, 0xb9, 0, 0xa0, 0xc9, 0x22, 0x31, 0x96],
};
const PROPERTY_IOCTL: u32 = 0x002f0003;

#[repr(C)]
#[derive(Clone, Copy)]
struct DeviceData {
    size: u32,
    class: Guid,
    instance: u32,
    reserved: usize,
}
#[repr(C)]
struct InterfaceData {
    size: u32,
    class: Guid,
    flags: u32,
    reserved: usize,
}
impl DeviceData {
    fn new() -> Self {
        // SAFETY: all-zero integers/GUID are valid; API requires cbSize initialized.
        let mut d: Self = unsafe { zeroed() };
        d.size = size_of::<Self>() as u32;
        d
    }
}
impl InterfaceData {
    fn new() -> Self {
        // SAFETY: all fields are plain integers/GUID.
        let mut d: Self = unsafe { zeroed() };
        d.size = size_of::<Self>() as u32;
        d
    }
}

#[cfg_attr(windows, link(name = "setupapi"))]
unsafe extern "system" {
    fn SetupDiGetClassDevsW(
        class: *const Guid,
        enumerator: *const u16,
        parent: Handle,
        flags: u32,
    ) -> Handle;
    fn SetupDiDestroyDeviceInfoList(set: Handle) -> i32;
    fn SetupDiEnumDeviceInfo(set: Handle, index: u32, info: *mut DeviceData) -> i32;
    fn SetupDiGetDeviceRegistryPropertyW(
        set: Handle,
        info: *mut DeviceData,
        property: u32,
        kind: *mut u32,
        data: *mut u8,
        size: u32,
        needed: *mut u32,
    ) -> i32;
    fn SetupDiEnumDeviceInterfaces(
        set: Handle,
        info: *const DeviceData,
        class: *const Guid,
        index: u32,
        interface: *mut InterfaceData,
    ) -> i32;
    fn SetupDiGetDeviceInterfaceDetailW(
        set: Handle,
        interface: *const InterfaceData,
        detail: *mut c_void,
        size: u32,
        needed: *mut u32,
        info: *mut DeviceData,
    ) -> i32;
}
#[cfg_attr(windows, link(name = "kernel32"))]
unsafe extern "system" {
    fn GetLastError() -> u32;
    fn CloseHandle(handle: Handle) -> i32;
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *const c_void,
        creation: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    fn DeviceIoControl(
        handle: Handle,
        code: u32,
        input: *const c_void,
        input_size: u32,
        output: *mut c_void,
        output_size: u32,
        returned: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
}
#[cfg_attr(windows, link(name = "cfgmgr32"))]
unsafe extern "system" {
    fn CM_Get_DevNode_Status(status: *mut u32, problem: *mut u32, instance: u32, flags: u32)
        -> u32;
}

fn error(operation: &str, code: u32) -> String {
    format!("{operation}: Windows error {code}; no device writes sent")
}
struct Set(Handle);
impl Drop for Set {
    fn drop(&mut self) {
        // SAFETY: unique live HDEVINFO from SetupDiGetClassDevsW.
        unsafe {
            SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}
impl Set {
    fn new(class: Option<&Guid>) -> Result<Self, String> {
        // SAFETY: optional GUID and null optional parameters are valid for this call.
        let h = unsafe {
            SetupDiGetClassDevsW(
                class.map_or(ptr::null(), |x| x),
                ptr::null(),
                0,
                2 | if class.is_some() { 16 } else { 4 },
            )
        };
        if h == -1 || h == 0 {
            return Err(error("Enumerate devices", unsafe { GetLastError() }));
        }
        Ok(Self(h))
    }
    fn registry(
        &self,
        info: &mut DeviceData,
        property: u32,
    ) -> Result<Option<(u32, Vec<u8>)>, String> {
        let mut kind = 0;
        let mut needed = 0;
        // SAFETY: first call asks size only, retaining a live device set/info.
        let first = unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                self.0,
                info,
                property,
                &mut kind,
                ptr::null_mut(),
                0,
                &mut needed,
            )
        };
        if first == 0 {
            let e = unsafe { GetLastError() };
            if e == 13 || e == 1168 {
                return Ok(None);
            }
            if e != 122 {
                return Err(error("Registry metadata size", e));
            }
        }
        if needed == 0 {
            return Ok(None);
        }
        if needed > 65536 {
            return Err("Registry metadata exceeds bound".into());
        }
        let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
        let size = needed;
        // SAFETY: u64 storage supplies sufficient alignment/capacity for the byte count.
        let ok = unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                self.0,
                info,
                property,
                &mut kind,
                buffer.as_mut_ptr().cast(),
                size,
                &mut needed,
            )
        };
        if ok == 0 {
            return Err(error("Read registry metadata", unsafe { GetLastError() }));
        }
        if needed > size {
            return Err("Registry metadata length changed".into());
        }
        // SAFETY: successful API returned at most the allocated byte count.
        let bytes =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), needed as usize) }
                .to_vec();
        Ok(Some((kind, bytes)))
    }
    fn string(&self, info: &mut DeviceData, property: u32) -> Result<String, String> {
        match self.registry(info, property)? {
            Some((1, b)) => crate::single_sz(&b),
            None => Ok(String::new()),
            _ => Err("Unexpected registry string type".into()),
        }
    }
    fn identity(&self, info: &mut DeviceData) -> Result<Option<&'static str>, String> {
        match self.registry(info, 1)? {
            Some((7, b)) => Ok(crate::phase(&crate::multi_sz(&b)?)),
            None => Ok(None),
            _ => Err("Unexpected hardware-ID registry type".into()),
        }
    }
}

struct Device(Handle);
impl Drop for Device {
    fn drop(&mut self) {
        // SAFETY: uniquely owned live kernel handle.
        unsafe {
            CloseHandle(self.0);
        }
    }
}
impl Device {
    fn open(path: &[u16]) -> Result<Self, String> {
        // SAFETY: path is a verified bounded, NUL-terminated UTF-16 buffer.
        // GENERIC_READ only; FILE_SHARE_READ|WRITE does not grant write access.
        let h = unsafe { CreateFileW(path.as_ptr(), 0x80000000, 3, ptr::null(), 3, 0x80, 0) };
        if h == -1 || h == 0 {
            return Err(error("Open camera metadata filter", unsafe {
                GetLastError()
            }));
        }
        Ok(Self(h))
    }
    fn query(
        &self,
        property: &Property,
        node: Option<&NodeProperty>,
        capacity: usize,
    ) -> Result<Vec<u8>, String> {
        let actual = node.map_or(property, |n| &n.property);
        if !crate::allowed_property(actual) || capacity == 0 || capacity > 65536 {
            return Err("Refusing non-metadata property or invalid buffer size".into());
        }
        let (input, size) = if let Some(n) = node {
            if n.reserved != 0 {
                return Err("Nonzero reserved node field".into());
            }
            (
                n as *const NodeProperty as *const c_void,
                size_of::<NodeProperty>(),
            )
        } else {
            (
                property as *const Property as *const c_void,
                size_of::<Property>(),
            )
        };
        let mut storage = vec![0u64; capacity.div_ceil(8)];
        let mut returned = 0;
        // SAFETY: fixed initialized SDK layouts, valid aligned buffers and live handle.
        // allowed_property excludes SET and limits all IDs/property sets.
        let ok = unsafe {
            DeviceIoControl(
                self.0,
                PROPERTY_IOCTL,
                input,
                size as u32,
                storage.as_mut_ptr().cast(),
                capacity as u32,
                &mut returned,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(error("KS metadata query", unsafe { GetLastError() }));
        }
        if returned as usize > capacity {
            return Err("KS returned length exceeds buffer".into());
        }
        // SAFETY: bounded successful returned length from allocated storage.
        Ok(
            unsafe { std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), returned as usize) }
                .to_vec(),
        )
    }
}

fn inventory() -> Result<usize, String> {
    let set = Set::new(None)?;
    let mut found = 0;
    for index in 0..4096 {
        let mut info = DeviceData::new();
        // SAFETY: live set and correctly sized output structure.
        if unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut info) } == 0 {
            let code = unsafe { GetLastError() };
            if code == 259 {
                return Ok(found);
            }
            return Err(error("Enumerate present device", code));
        }
        let Some(phase) = set.identity(&mut info)? else {
            continue;
        };
        let service = set.string(&mut info, 4)?;
        let class = set.string(&mut info, 7)?;
        let mut status = 0;
        let mut problem = 0;
        // SAFETY: DevInst belongs to this returned present-device record.
        let cm = unsafe { CM_Get_DevNode_Status(&mut status, &mut problem, info.instance, 0) };
        println!("{{\"kind\":\"device\",\"index\":{found},\"phase\":{},\"service\":{},\"class\":{},\"cm_result\":{cm},\"status\":{status},\"problem\":{problem}}}",crate::quote(phase),crate::quote(&service),crate::quote(&class));
        found += 1;
    }
    Err("Present-device enumeration exceeded bound".into())
}

fn probe_filter(device: &Device, interface: usize) -> Result<usize, String> {
    let p = Property::nodes();
    let topology = crate::nodes(&device.query(&p, None, 65536)?)?;
    let mut matched = 0;
    for (node, guid) in topology.iter().enumerate() {
        if *guid != DEV_SPECIFIC {
            continue;
        }
        let p = NodeProperty::info(node as u32);
        let info = match device.query(&p.property, Some(&p), 1024) {
            Ok(data) => data,
            Err(e) => {
                println!("{{\"kind\":\"candidate_unavailable\",\"interface\":{interface},\"node\":{node},\"error\":{}}}",crate::quote(&e));
                continue;
            }
        };
        let controls = match crate::extension_info(&info) {
            Ok(count) => count,
            Err(_) => {
                println!(
                    "{{\"kind\":\"candidate_rejected\",\"interface\":{interface},\"node\":{node}}}"
                );
                continue;
            }
        };
        println!("{{\"kind\":\"extension\",\"interface\":{interface},\"node\":{node},\"guid\":{},\"controls\":{controls},\"candidate_origin\":\"Linux runtime 205d, boot not assumed\"}}",crate::quote(&UNIT6_CANDIDATE.to_string()));
        for selector in [3, 4] {
            if selector > u32::from(controls) {
                continue;
            }
            let p = NodeProperty::support(node as u32, selector)?;
            match device.query(&p.property,Some(&p),4).and_then(|b|crate::access(&b)) {
                Ok(flags)=>{println!("{{\"kind\":\"control_support\",\"interface\":{interface},\"node\":{node},\"selector\":{selector},\"flags\":{flags},\"get\":{},\"set\":{}}}",flags&1!=0,flags&2!=0);matched+=1;},
                Err(e)=>println!("{{\"kind\":\"control_unavailable\",\"interface\":{interface},\"node\":{node},\"selector\":{selector},\"error\":{}}}",crate::quote(&e))
            }
        }
    }
    Ok(matched)
}

fn probe() -> Result<usize, String> {
    let mut found = 0;
    let mut seen = HashSet::new();
    for category in [VIDEO_CAMERA, CAPTURE] {
        let set = Set::new(Some(&category))?;
        for index in 0..4096 {
            let mut interface = InterfaceData::new();
            // SAFETY: live interface set and matching category GUID.
            if unsafe {
                SetupDiEnumDeviceInterfaces(set.0, ptr::null(), &category, index, &mut interface)
            } == 0
            {
                let code = unsafe { GetLastError() };
                if code == 259 {
                    break;
                }
                return Err(error("Enumerate camera interface", code));
            }
            let mut needed = 0;
            let mut info = DeviceData::new();
            // SAFETY: first call queries the detail buffer size only.
            let ok = unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    set.0,
                    &interface,
                    ptr::null_mut(),
                    0,
                    &mut needed,
                    ptr::null_mut(),
                )
            };
            if ok == 0 && unsafe { GetLastError() } != 122 {
                return Err("Interface detail size query failed".into());
            }
            if !(6..=65536).contains(&needed) {
                return Err("Interface detail size exceeds bound".into());
            }
            let size = needed;
            let mut detail = vec![0u64; (size as usize).div_ceil(8)];
            // SAFETY: storage is DWORD-aligned; SDK cbSize differs by pointer width.
            unsafe {
                ptr::write(
                    detail.as_mut_ptr().cast::<u32>(),
                    if size_of::<usize>() == 8 { 8 } else { 6 },
                );
            }
            // SAFETY: correctly sized/aligned detail buffer and associated devinfo output.
            if unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    set.0,
                    &interface,
                    detail.as_mut_ptr().cast(),
                    size,
                    &mut needed,
                    &mut info,
                )
            } == 0
            {
                return Err(error("Read camera interface details", unsafe {
                    GetLastError()
                }));
            }
            if needed > size {
                return Err("Interface detail changed length".into());
            }
            let Some(phase) = set.identity(&mut info)? else {
                continue;
            };
            let service = set.string(&mut info, 4)?;
            // SAFETY: successful bounded detail byte length; path is never printed.
            let bytes = unsafe {
                std::slice::from_raw_parts(detail.as_ptr().cast::<u8>(), needed as usize)
            };
            let path = crate::interface_path(bytes)?;
            if !seen.insert(path.clone()) {
                continue;
            }
            let id = seen.len() - 1;
            println!(
                "{{\"kind\":\"interface\",\"index\":{id},\"phase\":{},\"service\":{}}}",
                crate::quote(phase),
                crate::quote(&service)
            );
            if !service.eq_ignore_ascii_case("usbvideo") {
                println!("{{\"kind\":\"probe_skipped\",\"interface\":{id},\"reason\":\"Not bound to inbox usbvideo; no driver change attempted\"}}");
                continue;
            }
            let result = Device::open(&path).and_then(|device| {
                if set.identity(&mut info)? != Some(phase)
                    || !set.string(&mut info, 4)?.eq_ignore_ascii_case("usbvideo")
                {
                    return Err("Device association changed; query cancelled".into());
                }
                probe_filter(&device, id)
            });
            match result {
                Ok(n) => found += n,
                Err(e) => println!(
                    "{{\"kind\":\"probe_unavailable\",\"interface\":{id},\"error\":{}}}",
                    crate::quote(&e)
                ),
            }
            if index == 4095 {
                return Err("Camera interface enumeration exceeded bound".into());
            }
        }
    }
    Ok(found)
}

pub fn run(with_probe: bool) -> Result<(), String> {
    println!("{{\"kind\":\"scope\",\"version\":\"0.1.0\",\"architecture\":{},\"read_only\":true,\"firmware_upload\":false}}",crate::quote(std::env::consts::ARCH));
    if inventory()? == 0 {
        return Err(
            "No present Samsung 04e8:205c/205d hardware IDs found; no driver changes made".into(),
        );
    }
    if with_probe && probe()? == 0 {
        return Err("Probe incomplete: Windows exposed no usable candidate control metadata. This is not a firmware-loader success. Preserve the report; do not replace drivers.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setupapi_layouts_match_64_bit_sdk() {
        if size_of::<usize>() == 8 {
            assert_eq!(size_of::<DeviceData>(), 32);
            assert_eq!(size_of::<InterfaceData>(), 32);
            assert_eq!(std::mem::offset_of!(DeviceData, instance), 20);
            assert_eq!(std::mem::offset_of!(DeviceData, reserved), 24);
        }
        assert_eq!(PROPERTY_IOCTL, 0x002f0003);
    }
}
