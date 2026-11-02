// SPDX-License-Identifier: GPL-3.0-only
//! Historical LED command, gated by current-device UVC capabilities.

pub const GET_INFO: u8 = 0x86;
pub const GET_LEN: u8 = 0x85;
pub const SET_CUR: u8 = 0x01;

pub trait Query {
    fn query(&mut self, request: u8, data: &mut [u8]) -> Result<(), String>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Info {
    pub capabilities: u8,
    pub length: u16,
}

impl Info {
    pub fn can_test(&self) -> bool {
        self.length == 8 && self.capabilities & 2 != 0 && self.capabilities & 4 == 0
    }
}

pub fn info<Q: Query>(device: &mut Q) -> Result<Info, String> {
    let mut capabilities = [0];
    device.query(GET_INFO, &mut capabilities)?;
    let mut length = [0; 2];
    device.query(GET_LEN, &mut length)?;
    Ok(Info {
        capabilities: capabilities[0],
        length: u16::from_le_bytes(length),
    })
}

pub fn set<Q: Query>(device: &mut Q, on: bool) -> Result<(), String> {
    let capabilities = info(device)?;
    if !capabilities.can_test() {
        return Err(format!(
            "Refusing LED write: GET_INFO=0x{:02x}, GET_LEN={}; need enabled SET support and length 8; no SET_CUR sent",
            capabilities.capabilities, capabilities.length
        ));
    }
    device.query(SET_CUR, &mut [0x0e, 0, u8::from(on), 0, 0, 0, 0, 0])
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        capabilities: u8,
        length: u16,
        calls: Vec<(u8, Vec<u8>)>,
        fail_at: Option<usize>,
    }

    impl Default for Fake {
        fn default() -> Self {
            Self {
                capabilities: 3,
                length: 8,
                calls: vec![],
                fail_at: None,
            }
        }
    }

    impl Query for Fake {
        fn query(&mut self, request: u8, data: &mut [u8]) -> Result<(), String> {
            self.calls.push((request, data.to_vec()));
            if self.fail_at == Some(self.calls.len()) {
                return Err("simulated UVC error".into());
            }
            match request {
                GET_INFO => data.copy_from_slice(&[self.capabilities]),
                GET_LEN => data.copy_from_slice(&self.length.to_le_bytes()),
                SET_CUR => {}
                _ => panic!("unexpected request"),
            }
            Ok(())
        }
    }

    #[test]
    fn information_queries_never_write_control_state() {
        let mut device = Fake::default();
        assert_eq!(
            info(&mut device).unwrap(),
            Info {
                capabilities: 3,
                length: 8
            }
        );
        assert_eq!(device.calls, [(GET_INFO, vec![0]), (GET_LEN, vec![0, 0])]);
    }

    #[test]
    fn manual_on_and_off_read_capabilities_before_exact_historical_payload() {
        for on in [false, true] {
            let mut device = Fake::default();
            set(&mut device, on).unwrap();
            assert_eq!(
                device.calls,
                [
                    (GET_INFO, vec![0]),
                    (GET_LEN, vec![0, 0]),
                    (SET_CUR, vec![0x0e, 0, u8::from(on), 0, 0, 0, 0, 0]),
                ]
            );
        }
    }

    #[test]
    fn incompatible_or_disabled_controls_never_receive_set() {
        for (capabilities, length) in [(1, 8), (7, 8), (3, 0), (3, 32), (3, 65535)] {
            let mut device = Fake {
                capabilities,
                length,
                ..Fake::default()
            };
            assert!(set(&mut device, true).is_err());
            assert_eq!(device.calls.len(), 2);
            assert!(device.calls.iter().all(|(request, _)| *request != SET_CUR));
        }
    }

    #[test]
    fn failed_queries_or_writes_stop_without_retry() {
        for step in 1..=3 {
            let mut device = Fake {
                fail_at: Some(step),
                ..Fake::default()
            };
            assert!(set(&mut device, true).is_err());
            assert_eq!(device.calls.len(), step);
        }
    }

    #[test]
    fn info_is_useful_even_when_manual_test_is_unsupported() {
        let mut device = Fake {
            capabilities: 1,
            length: 32,
            ..Fake::default()
        };
        let result = info(&mut device).unwrap();
        assert_eq!(
            result,
            Info {
                capabilities: 1,
                length: 32
            }
        );
        assert!(!result.can_test());
        assert_eq!(device.calls.len(), 2);
    }
}
