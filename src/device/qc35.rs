//! Bose QC35 / QC35 II
//!
//! NC protocol: `function_block`=0x01, `function`=0x06
//!   Set: `[0x01, 0x06, 0x02, 0x01, wire_level]`
//!   Response: `[0x01, 0x06, 0x03, 0x02, wire_level, 0x0b]`
//!
//! Wire values: Off=0x00, High=0x01, Medium=0x02, Low=0x03
//! User levels: 0=Off, 1=Low, 2=Medium, 3=High

use crate::device::bmap::{self, Packet, PacketError};

use super::{Capability, DeviceProfile, NcStatus, Result};

pub(crate) const PRODUCT_ID: u16 = 0x400C;

const FB: u8 = 0x01; // Settings
const FN: u8 = 0x06;
const MAX_LEVEL: u8 = 3;

pub(crate) struct Qc35;

fn user_to_wire(level: u8) -> u8 {
    match level {
        0 => 0x00,
        l => (MAX_LEVEL + 1) - l,
    }
}

fn wire_to_user(wire: u8) -> u8 {
    match wire {
        0x00 => 0,
        w => (MAX_LEVEL + 1) - w,
    }
}

impl DeviceProfile for Qc35 {
    fn product_name(&self) -> &'static str {
        "Bose QC35"
    }

    fn max_nc_level(&self) -> u8 {
        MAX_LEVEL
    }

    fn capabilities(&self) -> &'static [Capability] {
        &[Capability::NcGet, Capability::NcSet]
    }

    fn build_nc_get(&self) -> Vec<u8> {
        Packet::new(FB, FN, bmap::OP_GET, vec![]).to_bytes()
    }

    fn build_nc_set(&self, level: u8) -> Vec<u8> {
        Packet::new(FB, FN, bmap::OP_SET_GET, vec![user_to_wire(level)]).to_bytes()
    }

    fn parse_nc_status(&self, payload: &[u8]) -> Result<NcStatus> {
        if payload.len() < 2 {
            return Err(PacketError::TooShort(payload.len(), 2).into());
        }
        let wire = payload[0];
        let level = wire_to_user(wire);
        let enabled = wire != 0;
        Ok(NcStatus {
            level,
            max_level: MAX_LEVEL,
            enabled,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Qc35 {
        Qc35
    }

    #[test]
    fn set_high() {
        let bytes = profile().build_nc_set(3);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![0x01]); // High
    }

    #[test]
    fn set_medium() {
        let bytes = profile().build_nc_set(2);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![0x02]); // Medium
    }

    #[test]
    fn set_low() {
        let bytes = profile().build_nc_set(1);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![0x03]); // Low
    }

    #[test]
    fn set_off() {
        let bytes = profile().build_nc_set(0);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![0x00]); // Off
    }

    #[test]
    fn parse_status_high() {
        let status = profile().parse_nc_status(&[0x01, 0x0b]).unwrap();
        assert_eq!(status.level, 3);
        assert!(status.enabled);
    }

    #[test]
    fn parse_status_off() {
        let status = profile().parse_nc_status(&[0x00, 0x0b]).unwrap();
        assert_eq!(status.level, 0);
        assert!(!status.enabled);
    }

    #[test]
    fn wire_roundtrip() {
        for level in 0..=MAX_LEVEL {
            assert_eq!(wire_to_user(user_to_wire(level)), level);
        }
    }
}
