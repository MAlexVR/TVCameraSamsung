// SPDX-License-Identifier: GPL-3.0-only
//! AIT RAM-upload protocol; no USB access is performed by this module.

pub mod led;

pub const CHUNK_SIZE: usize = 32;
pub const INDEX: u16 = 0x0600;

pub trait Transport {
    fn send(&mut self, value: u16, payload: &[u8]) -> Result<usize, String>;
}

pub fn upload<T: Transport>(transport: &mut T, firmware: &[u8]) -> Result<usize, String> {
    if firmware.is_empty() {
        return Err("Empty firmware is not allowed".into());
    }
    fn send<T: Transport>(transport: &mut T, value: u16, data: &[u8]) -> Result<(), String> {
        let actual = transport.send(value, data)?;
        if actual != data.len() {
            return Err(format!(
                "Short USB transfer: expected {}, received {actual}",
                data.len()
            ));
        }
        Ok(())
    }
    send(transport, 0x0400, &[1, 0, 3, 0, 0, 0, 0, 0])?;
    // Preserve the reference fread/feof protocol, including its final zero block
    // for exact multiples. The pinned Falcon image ends with a partial block.
    let blocks = firmware.len() / CHUNK_SIZE + 1;
    for index in 0..blocks {
        let start = index * CHUNK_SIZE;
        let remaining = (firmware.len() - start).min(CHUNK_SIZE);
        let mut block = [0; CHUNK_SIZE];
        block[..remaining].copy_from_slice(&firmware[start..start + remaining]);
        send(transport, 0x0300, &block)?;
    }
    send(transport, 0x0400, &[1, 1, 3, 0, 0, 0, 0, 0])?;
    Ok(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        calls: Vec<(u16, Vec<u8>)>,
        fail_at: Option<usize>,
        short_at: Option<usize>,
    }

    impl Transport for Fake {
        fn send(&mut self, value: u16, payload: &[u8]) -> Result<usize, String> {
            self.calls.push((value, payload.to_vec()));
            if self.fail_at == Some(self.calls.len()) {
                return Err("simulated timeout".into());
            }
            Ok(payload.len() - usize::from(self.short_at == Some(self.calls.len())))
        }
    }

    #[test]
    fn start_chunks_padding_and_boot_match_the_protocol() {
        let mut fake = Fake::default();
        assert_eq!(upload(&mut fake, &[0x55; 35]).unwrap(), 2);
        assert_eq!(fake.calls[0], (0x0400, vec![1, 0, 3, 0, 0, 0, 0, 0]));
        assert_eq!(fake.calls[1], (0x0300, vec![0x55; 32]));
        let mut last = vec![0; 32];
        last[..3].fill(0x55);
        assert_eq!(fake.calls[2], (0x0300, last));
        assert_eq!(fake.calls[3], (0x0400, vec![1, 1, 3, 0, 0, 0, 0, 0]));
        assert_eq!(fake.calls.len(), 4);
    }

    #[test]
    fn exact_chunk_multiple_matches_upstream_trailing_zero_block() {
        // The reference fread/feof loop emits a zero block at exact multiples.
        let mut fake = Fake::default();
        assert_eq!(upload(&mut fake, &[1; 32]).unwrap(), 2);
        assert_eq!(fake.calls[2], (0x0300, vec![0; 32]));
        assert_eq!(fake.calls.len(), 4);
    }

    #[test]
    fn rejected_empty_input_never_contacts_usb() {
        let mut fake = Fake::default();
        assert!(upload(&mut fake, &[]).is_err());
        assert!(fake.calls.is_empty());
    }

    #[test]
    fn failures_stop_at_each_stage_without_retry_or_later_boot() {
        for step in 1..=4 {
            let mut fake = Fake {
                fail_at: Some(step),
                ..Fake::default()
            };
            assert!(upload(&mut fake, &[1; 35]).is_err());
            assert_eq!(fake.calls.len(), step);
        }
    }

    #[test]
    fn short_transfers_are_errors_at_every_stage() {
        for step in 1..=4 {
            let mut fake = Fake {
                short_at: Some(step),
                ..Fake::default()
            };
            assert!(upload(&mut fake, &[1; 35]).is_err());
            assert_eq!(fake.calls.len(), step);
        }
    }
}
