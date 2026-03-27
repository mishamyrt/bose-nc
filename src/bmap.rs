//! BMAP (Bose Multi-device Application Protocol) packet framing.
//!
//! Device-agnostic: handles only serialization and deserialization of the
//! fixed 4-byte header + variable-length payload.
//!
//! Header layout:
//!   byte 0 — function block
//!   byte 1 — function
//!   byte 2 — device id (2 bit) | port num (2 bit) | operator (4 bit)
//!   byte 3 — payload length

use anyhow::{Result, bail};

pub(crate) const OP_GET: u8 = 0x01;
pub(crate) const OP_SET_GET: u8 = 0x02;
#[allow(dead_code)]
pub(crate) const OP_STATUS: u8 = 0x03;
pub(crate) const OP_ERROR: u8 = 0x04;

const HEADER_SIZE: usize = 4;

#[derive(Debug)]
pub(crate) struct Packet {
    pub(crate) function_block: u8,
    pub(crate) function: u8,
    pub(crate) device_id: u8,
    pub(crate) port_num: u8,
    pub(crate) operator: u8,
    pub(crate) payload: Vec<u8>,
}

impl Packet {
    pub(crate) fn new(
        function_block: u8,
        function: u8,
        operator: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            function_block,
            function,
            device_id: 0,
            port_num: 0,
            operator,
            payload,
        }
    }

    fn dpo_byte(&self) -> u8 {
        (self.device_id << 6) | (self.port_num << 4) | (self.operator & 0x0F)
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        buf.push(self.function_block);
        buf.push(self.function);
        buf.push(self.dpo_byte());
        buf.push(self.payload.len() as u8);
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_SIZE {
            bail!(
                "packet too short: {} bytes, need at least {HEADER_SIZE}",
                data.len()
            );
        }
        let payload_len = data[3] as usize;
        if data.len() < HEADER_SIZE + payload_len {
            bail!(
                "payload truncated: have {} bytes, need {payload_len}",
                data.len() - HEADER_SIZE
            );
        }
        Ok(Self {
            function_block: data[0],
            function: data[1],
            device_id: (data[2] >> 6) & 0x03,
            port_num: (data[2] >> 4) & 0x03,
            operator: data[2] & 0x0F,
            payload: data[HEADER_SIZE..HEADER_SIZE + payload_len].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_packet() {
        let original = Packet::new(0x01, 0x05, OP_GET, vec![]);
        let bytes = original.to_bytes();
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.function_block, 0x01);
        assert_eq!(parsed.function, 0x05);
        assert_eq!(parsed.operator, OP_GET);
        assert!(parsed.payload.is_empty());
    }

    #[test]
    fn parse_too_short() {
        assert!(Packet::parse(&[0x01, 0x05]).is_err());
    }

    #[test]
    fn parse_truncated_payload() {
        assert!(Packet::parse(&[0x01, 0x05, 0x02, 0x03, 0xFF]).is_err());
    }

    #[test]
    fn dpo_byte_encodes_correctly() {
        let mut pkt = Packet::new(0x00, 0x00, OP_SET_GET, vec![]);
        pkt.device_id = 0x02;
        pkt.port_num = 0x01;
        let bytes = pkt.to_bytes();
        assert_eq!(bytes[2], 0b1001_0010);
    }
}
