//! Bose QC Earbuds
//!
//! Mode-based NC: `function_block`=0x1f, `function`=0x03, `operator`=0x05
//!   Set: `[0x1f, 0x03, 0x05, 0x02, mode, 0x01]`
//!
//! Modes:
//!   0x00 = Quiet  (max NC)
//!   0x01 = Aware  (transparency)
//!   0x02 = User-1 (medium NC)
//!   0x03 = User-2 (NC off)
//!
//! User levels: 0=Aware, 1=Medium(User-1), 2=Quiet(max)
//! Off command → User-2 (hardware off, distinct from Aware/transparency)

use crate::device::bmap::{self, Packet, PacketError};

use super::{Capability, DeviceProfile, NcStatus, Result};

pub(crate) const PRODUCT_ID: u16 = 0x402F;

const FB: u8 = 0x1f; // NoiseControl
const FN: u8 = 0x03;
const OP_MODE_SET: u8 = 0x05;
const MAX_LEVEL: u8 = 2;

const MODE_QUIET: u8 = 0x00;
const MODE_AWARE: u8 = 0x01;
const MODE_USER1: u8 = 0x02;
const MODE_USER2: u8 = 0x03;

pub(crate) struct QcEarbuds;

fn level_to_mode(level: u8) -> u8 {
    match level {
        0 => MODE_AWARE,
        1 => MODE_USER1,
        _ => MODE_QUIET,
    }
}

fn mode_to_level(mode: u8) -> (u8, bool) {
    match mode {
        MODE_QUIET => (2, true),
        MODE_USER1 => (1, true),
        MODE_AWARE => (0, true),
        _ => (0, false), // User-2 / off
    }
}

impl DeviceProfile for QcEarbuds {
    fn product_name(&self) -> &'static str {
        "Bose QC Earbuds"
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
        Packet::new(FB, FN, OP_MODE_SET, vec![level_to_mode(level), 0x01]).to_bytes()
    }

    fn build_nc_off(&self) -> Option<Vec<u8>> {
        Some(Packet::new(FB, FN, OP_MODE_SET, vec![MODE_USER2, 0x01]).to_bytes())
    }

    fn parse_nc_status(&self, payload: &[u8]) -> Result<NcStatus> {
        if payload.is_empty() {
            return Err(PacketError::EmptyPayload.into());
        }
        let (level, enabled) = mode_to_level(payload[0]);
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

    fn profile() -> QcEarbuds {
        QcEarbuds
    }

    #[test]
    fn set_quiet() {
        let bytes = profile().build_nc_set(2);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![MODE_QUIET, 0x01]);
    }

    #[test]
    fn set_medium() {
        let bytes = profile().build_nc_set(1);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![MODE_USER1, 0x01]);
    }

    #[test]
    fn set_aware() {
        let bytes = profile().build_nc_set(0);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![MODE_AWARE, 0x01]);
    }

    #[test]
    fn off_sends_user2() {
        let bytes = profile().build_nc_off().unwrap();
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![MODE_USER2, 0x01]);
    }

    #[test]
    fn off_differs_from_set_0() {
        assert_ne!(profile().build_nc_set(0), profile().build_nc_off().unwrap());
    }

    #[test]
    fn mode_roundtrip() {
        for level in 0..=MAX_LEVEL {
            let mode = level_to_mode(level);
            let (back, enabled) = mode_to_level(mode);
            assert_eq!(back, level);
            assert!(enabled);
        }
    }
}
