//! Bose QC45
//!
//! Mode-based NC: `function_block`=0x1f, `function`=0x03, `operator`=0x05
//!   Set: `[0x1f, 0x03, 0x05, 0x02, mode, 0x01]`
//!
//! Modes: Quiet=0x00 (max NC), Aware=0x01 (transparency)
//! User levels: 0=Off/Aware, 1=Quiet

use crate::device::bmap::{self, Packet, PacketError};

use super::{Capability, DeviceProfile, NcStatus, Result};

pub(crate) const PRODUCT_ID: u16 = 0x4039;

const FB: u8 = 0x1f; // NoiseControl
const FN: u8 = 0x03;
const OP_MODE_SET: u8 = 0x05;
const MAX_LEVEL: u8 = 1;

const MODE_QUIET: u8 = 0x00;
const MODE_AWARE: u8 = 0x01;

pub(crate) struct Qc45;

impl DeviceProfile for Qc45 {
    fn product_name(&self) -> &'static str {
        "Bose QC45"
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
        let mode = if level > 0 { MODE_QUIET } else { MODE_AWARE };
        Packet::new(FB, FN, OP_MODE_SET, vec![mode, 0x01]).to_bytes()
    }

    fn parse_nc_status(&self, payload: &[u8]) -> Result<NcStatus> {
        if payload.is_empty() {
            return Err(PacketError::EmptyPayload.into());
        }
        let mode = payload[0];
        let (level, enabled) = if mode == MODE_QUIET {
            (1, true)
        } else {
            (0, false)
        };
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

    fn profile() -> Qc45 {
        Qc45
    }

    #[test]
    fn set_quiet() {
        let bytes = profile().build_nc_set(1);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.function_block, FB);
        assert_eq!(parsed.function, FN);
        assert_eq!(parsed.operator, OP_MODE_SET);
        assert_eq!(parsed.payload, vec![MODE_QUIET, 0x01]);
    }

    #[test]
    fn set_aware() {
        let bytes = profile().build_nc_set(0);
        let parsed = Packet::parse(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![MODE_AWARE, 0x01]);
    }

    #[test]
    fn off_not_supported() {
        assert!(profile().build_nc_off().is_none());
    }
}
