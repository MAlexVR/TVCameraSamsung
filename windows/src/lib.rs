// SPDX-License-Identifier: GPL-3.0-only
use std::fmt;

#[cfg(any(windows, feature = "check-windows-ffi"))]
#[cfg_attr(not(windows), allow(dead_code))]
pub mod native;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}

impl Guid {
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 16 {
            return Err("Truncated GUID".into());
        }
        Ok(Self {
            a: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            b: u16::from_le_bytes(data[4..6].try_into().unwrap()),
            c: u16::from_le_bytes(data[6..8].try_into().unwrap()),
            d: data[8..16].try_into().unwrap(),
        })
    }
    pub fn bytes(self) -> [u8; 16] {
        let mut out = [0; 16];
        out[..4].copy_from_slice(&self.a.to_le_bytes());
        out[4..6].copy_from_slice(&self.b.to_le_bytes());
        out[6..8].copy_from_slice(&self.c.to_le_bytes());
        out[8..].copy_from_slice(&self.d);
        out
    }
}
impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-",
            self.a, self.b, self.c, self.d[0], self.d[1]
        )?;
        for byte in &self.d[2..] {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

pub const TOPOLOGY: Guid = Guid {
    a: 0x720d4ac0,
    b: 0x7533,
    c: 0x11d0,
    d: [0xa5, 0xd6, 0x28, 0xdb, 4, 0xc1, 0, 0],
};
pub const DEV_SPECIFIC: Guid = Guid {
    a: 0x941c7ac0,
    b: 0xc559,
    c: 0x11d0,
    d: [0x8a, 0x2b, 0, 0xa0, 0xc9, 0x25, 0x5a, 0xc1],
};
// Observed on runtime 205d only; boot 205c compatibility must be measured.
pub const UNIT6_CANDIDATE: Guid = Guid {
    a: 0x23e49ed0,
    b: 0x1178,
    c: 0x4f31,
    d: [0xae, 0x52, 0xd2, 0xfb, 0x8a, 0x8d, 0x3b, 0x48],
};

#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct Property {
    pub set: Guid,
    pub id: u32,
    pub flags: u32,
}
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct NodeProperty {
    pub property: Property,
    pub node_id: u32,
    pub reserved: u32,
}
impl Property {
    pub fn nodes() -> Self {
        Self {
            set: TOPOLOGY,
            id: 1,
            flags: 1,
        }
    }
}
impl NodeProperty {
    pub fn info(node_id: u32) -> Self {
        Self {
            property: Property {
                set: UNIT6_CANDIDATE,
                id: 0,
                flags: 0x10000001,
            },
            node_id,
            reserved: 0,
        }
    }
    pub fn support(node_id: u32, selector: u32) -> Result<Self, String> {
        if ![3, 4].contains(&selector) {
            return Err("Only firmware-control metadata selectors 3 and 4 are permitted".into());
        }
        Ok(Self {
            property: Property {
                set: UNIT6_CANDIDATE,
                id: selector,
                flags: 0x10000200,
            },
            node_id,
            reserved: 0,
        })
    }
}

pub fn allowed_property(p: &Property) -> bool {
    (p.set == TOPOLOGY && p.id == 1 && p.flags == 1)
        || (p.set == UNIT6_CANDIDATE
            && ((p.id == 0 && p.flags == 0x10000001)
                || ([3, 4].contains(&p.id) && p.flags == 0x10000200)))
}
pub fn single_sz(data: &[u8]) -> Result<String, String> {
    if data.len() < 2 || !data.len().is_multiple_of(2) {
        return Err("Invalid registry string length".into());
    }
    let words: Vec<u16> = data
        .chunks_exact(2)
        .map(|x| u16::from_le_bytes([x[0], x[1]]))
        .collect();
    let end = words
        .iter()
        .position(|x| *x == 0)
        .ok_or("Unterminated registry string")?;
    if words[end..].iter().any(|x| *x != 0) {
        return Err("Trailing registry string data".into());
    }
    String::from_utf16(&words[..end]).map_err(|_| "Invalid registry UTF-16".into())
}
pub fn interface_path(data: &[u8]) -> Result<Vec<u16>, String> {
    if data.len() < 6 || data.len() > 65536 || !data.len().is_multiple_of(2) {
        return Err("Invalid interface detail length".into());
    }
    if ![6, 8].contains(&u32::from_le_bytes(data[..4].try_into().unwrap())) {
        return Err("Invalid interface detail header".into());
    }
    let words: Vec<u16> = data[4..]
        .chunks_exact(2)
        .map(|x| u16::from_le_bytes([x[0], x[1]]))
        .collect();
    let end = words
        .iter()
        .position(|x| *x == 0)
        .ok_or("Unterminated interface path")?;
    if end == 0 {
        return Err("Empty interface path".into());
    }
    Ok(words[..=end].to_vec())
}

pub fn phase(ids: &[String]) -> Option<&'static str> {
    let mut found = None;
    for id in ids {
        let upper = id.to_ascii_uppercase();
        for (prefix, name) in [
            ("USB\\VID_04E8&PID_205C", "boot"),
            ("USB\\VID_04E8&PID_205D", "runtime"),
        ] {
            if upper
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('&'))
            {
                if found.is_some_and(|prior| prior != name) {
                    return None;
                }
                found = Some(name);
            }
        }
    }
    found
}

pub fn multi_sz(data: &[u8]) -> Result<Vec<String>, String> {
    if data.len() < 4 || !data.len().is_multiple_of(2) {
        return Err("Invalid UTF-16 registry length".into());
    }
    let words: Vec<u16> = data
        .chunks_exact(2)
        .map(|x| u16::from_le_bytes([x[0], x[1]]))
        .collect();
    let mut result = Vec::new();
    let mut start = 0;
    for i in 0..words.len() {
        if words[i] == 0 {
            if i == start {
                if words[i..].iter().any(|x| *x != 0) {
                    return Err("Trailing registry data".into());
                }
                return Ok(result);
            }
            result.push(
                String::from_utf16(&words[start..i])
                    .map_err(|_| "Invalid UTF-16 registry value")?,
            );
            start = i + 1;
        }
    }
    Err("Registry list lacks double terminator".into())
}

pub fn nodes(data: &[u8]) -> Result<Vec<Guid>, String> {
    if data.len() < 8 {
        return Err("Truncated topology header".into());
    }
    let size = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    if count > 256 || size > data.len() || size < 8 + count * 16 {
        return Err("Invalid topology size/count".into());
    }
    data[8..8 + count * 16]
        .chunks_exact(16)
        .map(Guid::from_bytes)
        .collect()
}
pub fn extension_info(data: &[u8]) -> Result<u8, String> {
    if data.len() < 18 || Guid::from_bytes(data)? != UNIT6_CANDIDATE {
        return Err("Extension GUID mismatch or short metadata".into());
    }
    let pins = usize::from(data[17]);
    if pins == 0 || data.len() < 18 + pins {
        return Err("Truncated extension input-pin metadata".into());
    }
    Ok(data[16])
}
pub fn access(data: &[u8]) -> Result<u32, String> {
    if data.len() != 4 {
        return Err("Expected four-byte KS access flags".into());
    }
    Ok(u32::from_le_bytes(data.try_into().unwrap()))
}
pub fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_only_property_gate_rejects_set_and_unknown_requests() {
        assert!(allowed_property(&Property::nodes()));
        for x in [
            NodeProperty::info(0),
            NodeProperty::support(0, 3).unwrap(),
            NodeProperty::support(0, 4).unwrap(),
        ] {
            assert!(allowed_property(&x.property));
        }
        let mut p = Property::nodes();
        p.flags = 3;
        assert!(!allowed_property(&p));
        let mut p = NodeProperty::info(0).property;
        p.id = 7;
        assert!(!allowed_property(&p));
    }
    #[test]
    fn single_registry_string_and_interface_path_are_bounded() {
        let b: Vec<u8> = "usbvideo\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(single_sz(&b).unwrap(), "usbvideo");
        assert!(single_sz(&b[..b.len() - 2]).is_err());
        let mut path = 8u32.to_le_bytes().to_vec();
        path.extend("device\0".encode_utf16().flat_map(u16::to_le_bytes));
        let decoded = interface_path(&path).unwrap();
        assert_eq!(*decoded.last().unwrap(), 0);
        assert!(interface_path(&path[..path.len() - 2]).is_err());
    }
    #[test]
    fn conflicting_identity_and_registry_suffix_fail_closed() {
        assert_eq!(
            phase(&[
                "USB\\VID_04E8&PID_205C".into(),
                "USB\\VID_04E8&PID_205D".into()
            ]),
            None
        );
        let b: Vec<u8> = "usbvideo\0suffix\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(single_sz(&b).is_err());
    }
    #[test]
    fn exact_usb_identity_and_boundaries() {
        assert_eq!(
            phase(&["usb\\vid_04e8&pid_205c&mi_00".into()]),
            Some("boot")
        );
        assert_eq!(phase(&["USB\\VID_04E8&PID_205D".into()]), Some("runtime"));
        for id in [
            "USB\\VID_04E8&PID_205C_FAKE",
            "USB\\VID_04E8&PID_2065",
            "ROOT\\VID_04E8&PID_205C",
        ] {
            assert_eq!(phase(&[id.into()]), None);
        }
    }
    #[test]
    fn metadata_queries_have_sdk_layout_and_never_set() {
        assert_eq!((size_of::<Property>(), align_of::<Property>()), (24, 8));
        assert_eq!(
            (size_of::<NodeProperty>(), align_of::<NodeProperty>()),
            (32, 8)
        );
        assert_eq!(std::mem::offset_of!(NodeProperty, node_id), 24);
        assert_eq!(Property::nodes().flags, 1);
        let info = NodeProperty::info(7);
        assert_eq!(
            (info.property.flags, info.property.id, info.reserved),
            (0x10000001, 0, 0)
        );
        for selector in [3, 4] {
            let x = NodeProperty::support(7, selector).unwrap();
            assert_eq!(x.property.flags, 0x10000200);
            assert_eq!(x.property.flags & 2, 0);
        }
        assert!(NodeProperty::support(0, 5).is_err());
    }
    #[test]
    fn bounded_topology_and_extension_metadata() {
        let mut b = Vec::new();
        b.extend(24u32.to_le_bytes());
        b.extend(1u32.to_le_bytes());
        b.extend(DEV_SPECIFIC.bytes());
        assert_eq!(nodes(&b).unwrap(), vec![DEV_SPECIFIC]);
        b[4..8].copy_from_slice(&257u32.to_le_bytes());
        assert!(nodes(&b).is_err());
        assert!(nodes(&b[..7]).is_err());
        let mut info = UNIT6_CANDIDATE.bytes().to_vec();
        info.extend([5, 1, 3]);
        assert_eq!(extension_info(&info).unwrap(), 5);
        assert!(extension_info(&info[..18]).is_err());
        info[0] ^= 1;
        assert!(extension_info(&info).is_err());
        assert_eq!(access(&2u32.to_le_bytes()).unwrap(), 2);
        assert!(access(&[2]).is_err());
    }
    #[test]
    fn registry_and_json_do_not_accept_truncation_or_control_injection() {
        let b: Vec<u8> = "USB\\VID_04E8&PID_205C\0\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(multi_sz(&b).unwrap(), vec!["USB\\VID_04E8&PID_205C"]);
        assert!(multi_sz(&b[..b.len() - 2]).is_err());
        assert_eq!(quote("a\n\"\\"), "\"a\\u000a\\\"\\\\\"");
    }
}
