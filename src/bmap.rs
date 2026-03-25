//! Bose BMAP (Bose Multi-device Application Protocol) implementation.
//! Reverse-engineered from the Bose Music Android application.
//!
//! NC 700 wire format for `SettingsCnc`:
//!   Send: `[0x01, 0x05, 0x02, 0x02, (10 - level), enabled]`
//!   Response: `[0x01, 0x05, 0x03, 0x03, 0x0B, (10 - level), enabled]`
//!   Level is inverted: `wire_value = 10 - user_level`

use anyhow::{Result, bail};

pub(crate) const HEADER_SIZE: usize = 4;
const MAX_NC_LEVEL: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum FunctionBlock {
    Settings = 0x01,
}

impl TryFrom<u8> for FunctionBlock {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(Self::Settings),
            _ => bail!("unknown function block: {value:#04x}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Function {
    SettingsCnc = 0x05,
    SettingsAnr = 0x06,
}

impl TryFrom<u8> for Function {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x05 => Ok(Self::SettingsCnc),
            0x06 => Ok(Self::SettingsAnr),
            _ => bail!("unknown function: {value:#04x}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Operator {
    Set = 0x00,
    Get = 0x01,
    SetGet = 0x02,
    Status = 0x03,
    Error = 0x04,
}

impl TryFrom<u8> for Operator {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(Self::Set),
            0x01 => Ok(Self::Get),
            0x02 => Ok(Self::SetGet),
            0x03 => Ok(Self::Status),
            0x04 => Ok(Self::Error),
            _ => bail!("unknown operator: {value:#04x}"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct BmapPacket {
    pub(crate) function_block: FunctionBlock,
    pub(crate) function: Function,
    pub(crate) device_id: u8,
    pub(crate) port_num: u8,
    pub(crate) operator: Operator,
    pub(crate) payload: Vec<u8>,
}

impl BmapPacket {
    pub(crate) fn new(
        function_block: FunctionBlock,
        function: Function,
        operator: Operator,
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

    fn device_port_operator_byte(&self) -> u8 {
        debug_assert!(self.device_id <= 0x03, "device_id exceeds 2 bits");
        debug_assert!(self.port_num <= 0x03, "port_num exceeds 2 bits");
        (self.device_id << 6) | (self.port_num << 4) | (self.operator as u8)
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        buf.push(self.function_block as u8);
        buf.push(self.function as u8);
        buf.push(self.device_port_operator_byte());
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
            function_block: FunctionBlock::try_from(data[0])?,
            function: Function::try_from(data[1])?,
            device_id: (data[2] >> 6) & 0x03,
            port_num: (data[2] >> 4) & 0x03,
            operator: Operator::try_from(data[2] & 0x0F)?,
            payload: data[HEADER_SIZE..HEADER_SIZE + payload_len].to_vec(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct CncStatus {
    pub(crate) level: u8,
    pub(crate) max_level: u8,
    pub(crate) enabled: bool,
}

impl CncStatus {
    /// Parse CNC response payload: `[num_steps, 10-level, enabled]`.
    pub(crate) fn parse(payload: &[u8]) -> Result<Self> {
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
        Ok(Self {
            level,
            max_level,
            enabled,
        })
    }
}

/// Build a CNC Get packet (query current noise cancellation state).
pub(crate) fn cnc_get_packet() -> Vec<u8> {
    BmapPacket::new(
        FunctionBlock::Settings,
        Function::SettingsCnc,
        Operator::Get,
        vec![],
    )
    .to_bytes()
}

/// Build a CNC `SetGet` packet for NC 700.
/// `level`: user-facing level 0-10 (10 = max NC, 0 = full transparency).
/// `enabled`: whether NC is on at all.
pub(crate) fn cnc_set_packet(level: u8, enabled: bool) -> Vec<u8> {
    let wire_level = MAX_NC_LEVEL.saturating_sub(level);
    BmapPacket::new(
        FunctionBlock::Settings,
        Function::SettingsCnc,
        Operator::SetGet,
        vec![wire_level, u8::from(enabled)],
    )
    .to_bytes()
}

#[allow(dead_code)]
pub(crate) fn anr_set_packet(level: u8) -> Vec<u8> {
    BmapPacket::new(
        FunctionBlock::Settings,
        Function::SettingsAnr,
        Operator::SetGet,
        vec![level],
    )
    .to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_packet() {
        let original = BmapPacket::new(
            FunctionBlock::Settings,
            Function::SettingsCnc,
            Operator::Get,
            vec![],
        );
        let bytes = original.to_bytes();
        let parsed = BmapPacket::parse(&bytes).unwrap();
        assert_eq!(parsed.function_block, FunctionBlock::Settings);
        assert_eq!(parsed.function, Function::SettingsCnc);
        assert_eq!(parsed.operator, Operator::Get);
        assert!(parsed.payload.is_empty());
    }

    #[test]
    fn roundtrip_cnc_set() {
        let bytes = cnc_set_packet(7, true);
        let parsed = BmapPacket::parse(&bytes).unwrap();
        assert_eq!(parsed.function_block, FunctionBlock::Settings);
        assert_eq!(parsed.function, Function::SettingsCnc);
        assert_eq!(parsed.operator, Operator::SetGet);
        // wire_level = 10 - 7 = 3, enabled = 1
        assert_eq!(parsed.payload, vec![3, 1]);
    }

    #[test]
    fn cnc_get_packet_format() {
        let bytes = cnc_get_packet();
        assert_eq!(bytes[0], FunctionBlock::Settings as u8);
        assert_eq!(bytes[1], Function::SettingsCnc as u8);
        assert_eq!(bytes[2] & 0x0F, Operator::Get as u8);
        assert_eq!(bytes[3], 0x00);
    }

    #[test]
    fn parse_cnc_status() {
        // num_steps=11 (0x0B), wire_level=5, enabled=1
        let status = CncStatus::parse(&[0x0B, 0x05, 0x01]).unwrap();
        assert_eq!(status.max_level, 10);
        assert_eq!(status.level, 5);
        assert!(status.enabled);
    }

    #[test]
    fn parse_cnc_status_disabled() {
        let status = CncStatus::parse(&[0x0B, 0x00, 0x00]).unwrap();
        assert_eq!(status.level, 10);
        assert!(!status.enabled);
    }

    #[test]
    fn parse_packet_too_short() {
        assert!(BmapPacket::parse(&[0x01, 0x05]).is_err());
    }

    #[test]
    fn parse_packet_truncated_payload() {
        assert!(BmapPacket::parse(&[0x01, 0x05, 0x02, 0x03, 0xFF]).is_err());
    }

    #[test]
    fn parse_cnc_status_too_short() {
        assert!(CncStatus::parse(&[0x0B, 0x05]).is_err());
    }
}
