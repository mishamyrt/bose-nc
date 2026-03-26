use anyhow::{Result, bail};

use crate::bmap::{self, Packet};

use super::{Capability, DeviceProfile, NcStatus};

pub(crate) const PRODUCT_ID: u16 = 0x4024;

const FB: u8 = 0x01; // Settings
const FN: u8 = 0x05; // SettingsCnc
const MAX_LEVEL: u8 = 10;

pub(crate) struct Nc700;

impl DeviceProfile for Nc700 {
    fn product_name(&self) -> &'static str {
        "Bose NC 700"
    }

    fn max_nc_level(&self) -> u8 {
        MAX_LEVEL
    }

    fn capabilities(&self) -> &'static [Capability] {
        &[Capability::NcGet, Capability::NcSet, Capability::NcOff]
    }

    fn build_nc_get(&self) -> Vec<u8> {
        Packet::new(FB, FN, bmap::OP_GET, vec![]).to_bytes()
    }

    fn build_nc_set(&self, level: u8) -> Vec<u8> {
        let wire_level = MAX_LEVEL.saturating_sub(level);
        Packet::new(FB, FN, bmap::OP_SET_GET, vec![wire_level, 0x01]).to_bytes()
    }

    fn build_nc_off(&self) -> Option<Vec<u8>> {
        Some(Packet::new(FB, FN, bmap::OP_SET_GET, vec![0x00, 0x00]).to_bytes())
    }

    fn parse_nc_status(&self, payload: &[u8]) -> Result<NcStatus> {
        if payload.len() < 3 {
            bail!(
                "CNC payload too short: {} bytes, need at least 3",
                payload.len()
            );
        }
        let num_steps = payload[0];
        let wire_level = payload[1];
        let enabled = payload[2] != 0;
        let max_level = num_steps.saturating_sub(1);
        let level = max_level.saturating_sub(wire_level);
        Ok(NcStatus {
            level,
            max_level,
            enabled,
        })
    }

    fn nc_set_repeat_count(&self) -> i32 {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Nc700 {
        Nc700
    }

    #[test]
    fn get_packet_format() {
        let bytes = profile().build_nc_get();
        assert_eq!(bytes[0], FB);
        assert_eq!(bytes[1], FN);
        assert_eq!(bytes[2] & 0x0F, bmap::OP_GET);
        assert_eq!(bytes[3], 0x00);
    }

    #[test]
    fn set_packet_roundtrip() {
        let bytes = profile().build_nc_set(7);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.function_block, FB);
        assert_eq!(parsed.function, FN);
        assert_eq!(parsed.operator, bmap::OP_SET_GET);
        assert_eq!(parsed.payload, vec![3, 1]); // wire_level = 10 - 7
    }

    #[test]
    fn off_packet_format() {
        let bytes = profile().build_nc_off().unwrap();
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![0x00, 0x00]);
    }

    #[test]
    fn parse_status() {
        // num_steps=11 (0x0B), wire_level=5, enabled=1
        let status = profile().parse_nc_status(&[0x0B, 0x05, 0x01]).unwrap();
        assert_eq!(status.max_level, 10);
        assert_eq!(status.level, 5);
        assert!(status.enabled);
    }

    #[test]
    fn parse_status_disabled() {
        let status = profile().parse_nc_status(&[0x0B, 0x00, 0x00]).unwrap();
        assert_eq!(status.level, 10);
        assert!(!status.enabled);
    }

    #[test]
    fn parse_status_too_short() {
        assert!(profile().parse_nc_status(&[0x0B, 0x05]).is_err());
    }
}
