use anyhow::{Result, bail};

use crate::bmap::{self, Packet};

use super::{Capability, DeviceProfile, NcStatus};

pub(crate) const PRODUCT_ID: u16 = 0x4066;

const FB: u8 = 0x1f; // NoiseControl
const FN: u8 = 0x06; // SettingsAnr
const MAX_LEVEL: u8 = 10;
const MODE_NAME_LEN: usize = 32;
const MODE_NAME: &[u8; 4] = b"None";

pub(crate) struct QcUltra;

impl DeviceProfile for QcUltra {
    fn product_name(&self) -> &'static str {
        "Bose QC Ultra"
    }

    fn max_nc_level(&self) -> u8 {
        MAX_LEVEL
    }

    fn capabilities(&self) -> &'static [Capability] {
        &[Capability::NcGet, Capability::NcSet]
    }

    fn build_nc_get(&self) -> Vec<u8> {
        Packet::new(FB, FN, bmap::OP_GET, vec![0x03]).to_bytes()
    }

    fn build_nc_set(&self, level: u8) -> Vec<u8> {
        let wire_level = MAX_LEVEL.saturating_sub(level);
        let mut payload = Vec::with_capacity(39);
        payload.extend_from_slice(&[0x03, 0x00, MAX_LEVEL + 1]);
        payload.extend_from_slice(MODE_NAME);
        payload.resize(3 + MODE_NAME_LEN, 0x00);
        payload.push(wire_level);
        payload.extend_from_slice(&[0x00, 0x00, 0x00]);
        Packet::new(FB, FN, bmap::OP_SET_GET, payload).to_bytes()
    }

    fn parse_nc_status(&self, payload: &[u8]) -> Result<NcStatus> {
        const MIN_PAYLOAD: usize = 47;
        if payload.len() < MIN_PAYLOAD {
            bail!(
                "ANR payload too short: {} bytes, need at least {MIN_PAYLOAD}",
                payload.len()
            );
        }
        let num_steps = payload[2];
        let max_level = num_steps.saturating_sub(1);
        let wire_level = payload[42];
        let level = max_level.saturating_sub(wire_level);
        let enabled = payload[45] != 0;
        Ok(NcStatus {
            level,
            max_level,
            enabled,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> QcUltra {
        QcUltra
    }

    #[test]
    fn get_packet_format() {
        let bytes = profile().build_nc_get();
        assert_eq!(bytes, vec![0x1f, 0x06, 0x01, 0x01, 0x03]);
    }

    #[test]
    fn set_packet_format() {
        let bytes = profile().build_nc_set(2);
        assert_eq!(bytes[0], FB);
        assert_eq!(bytes[1], FN);
        assert_eq!(bytes[2] & 0x0F, bmap::OP_SET_GET);
        assert_eq!(bytes[3], 39); // payload length
        assert_eq!(&bytes[4..7], &[0x03, 0x00, 0x0b]);
        assert_eq!(&bytes[7..11], b"None");
        assert_eq!(bytes[39], 8); // wire_level = 10 - 2
    }

    #[test]
    fn set_packet_matches_wire_dump() {
        let bytes = profile().build_nc_set(10);
        assert_eq!(bytes[0], 0x1f);
        assert_eq!(bytes[1], 0x06);
        assert_eq!(bytes[2], 0x02);
        assert_eq!(bytes[3], 0x27);
        assert_eq!(bytes.len(), 4 + 39);
        assert_eq!(bytes[39], 0); // wire_level = 10 - 10
    }

    #[test]
    fn parse_status() {
        // wire_level=8 at offset 42 → user_level = 10 - 8 = 2
        let mut payload = vec![0x03, 0x00, 0x0b, 0x01, 0x01, 0x01];
        payload.extend_from_slice(b"None");
        payload.resize(6 + 32, 0x00);
        payload.extend_from_slice(&[0x00, 0x00, 0x00, 0x0d, 0x08, 0x00, 0x00, 0x01, 0x00]);
        assert_eq!(payload.len(), 47);

        let status = profile().parse_nc_status(&payload).unwrap();
        assert_eq!(status.max_level, 10);
        assert_eq!(status.level, 2);
        assert!(status.enabled);
    }

    #[test]
    fn parse_status_too_short() {
        assert!(profile().parse_nc_status(&[0x03, 0x00, 0x0b]).is_err());
    }
}
